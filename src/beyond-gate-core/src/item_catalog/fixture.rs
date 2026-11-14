//! Owned synthetic item pages with no production endpoint or fixture selector.

use crate::{
    auth::session::{Session, fixture as auth},
    http::HttpClient,
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

/// Fixed invented responses for core and adapter process tests.
#[derive(Clone, Copy)]
pub enum Scenario {
    /// Mixed full, terminal, and empty source pages with unknown raw fields.
    Normal,
    /// A complete empty source, not an unknown listing shape.
    Empty,
    /// A nonempty page consisting only of magic items.
    MagicOnly,
    /// A complete page consisting only of equipment.
    EquipmentOnly,
    /// A magic row lacks required local-filter metadata.
    BadFilterMetadata,
    /// The service denies the protected request with HTTP 403.
    Denied,
    /// The service rejects the protected request with HTTP 401.
    Rejected,
    /// The synthetic session exchange establishes an expired bearer.
    Expired,
    /// A required response field is malformed.
    Malformed,
    /// The source ignores the requested page and returns page zero.
    IgnoredPage,
    /// A page is short without matching terminal pagination.
    Incomplete,
    /// Two raw rows have the same complete definition key.
    Duplicate,
    /// An unselected magic row has an unconfigured source reference.
    InvalidSource,
    /// An unselected row has no explicit boolean classification.
    BadMagic,
    /// The source introduces an unverified access map.
    Unsupported,
    /// The HTTP body exceeds the unchanged 1 MiB cap.
    Oversized,
    /// Holds the protected resource read until its owner is dropped.
    Waiting,
}

/// Keeps this fixture's server and safe route log alive.
pub(crate) struct Source {
    /// Single-attempt transport restricted to this loopback origin.
    pub(crate) client: HttpClient,
    /// Compiled-only replacement for both fixed source origins.
    pub(crate) origin: String,
    /// Stops only this server worker.
    stop: Arc<AtomicBool>,
    /// Joins only the worker started by this owner.
    worker: Option<thread::JoinHandle<()>>,
    /// Contains paths only, never headers or raw definitions.
    routes: Arc<Mutex<Vec<String>>>,
}

impl Drop for Source {
    /// Stops and joins this owned worker without affecting other processes.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Builds an invented definition with source and nested raw extensions.
pub(super) fn record(id: u64, magic: bool, entity: u64) -> Value {
    json!({
        "id":id, "entityTypeId":entity, "definitionKey":format!("{entity}:{id}"),
        "name":"Invented Item", "magic":magic, "isHomebrew":false,
        "filterType":match entity {8=>"Weapon",9=>"Armor",_=>"Other Gear"},
        "cost":if magic {None} else {Some(2)}, "weight":1,
        "rarity":if id.is_multiple_of(3) {"Rare"} else {"Common"},
        "canAttune":id.is_multiple_of(3), "tags":["Utility","Combat"],
        "sources":[{"sourceId":7,"sourceType":1,"pageNumber":null,"future":{"ref":true}}],
        "description":"Invented item text.", "future":{"nested":[true,null,3]},
        "terminal_controls":"tab\tline\n\u{1b}[31m"
    })
}

/// Builds the observed numeric pagination shape for one synthetic raw page.
pub(super) fn envelope(rows: Vec<Value>, page: u32, total: u64) -> Value {
    json!({"id":0,"success":true,"message":"","data":rows,
        "pagination":{"skip":u64::from(page)*100,"take":100,"total":total}})
}

/// Supplies checked public configuration or one exact synthetic item page.
fn reply(scenario: Scenario, route: &str) -> (u16, String) {
    if route == "/api/config/json" {
        return (
            200,
            json!({
                "sourceCategories":[{"id":1,"name":"Invented Category"}],
                "sources":[{"id":7,"sourceCategoryId":1}]
            })
            .to_string(),
        );
    }
    if matches!(scenario, Scenario::Denied) {
        return (403, "{}".to_owned());
    }
    if matches!(scenario, Scenario::Rejected) {
        return (401, "{}".to_owned());
    }
    if matches!(scenario, Scenario::Oversized) {
        return (200, "x".repeat(1024 * 1024 + 1));
    }
    let Some(page) = route
        .strip_prefix("/character/v5.1/game-data/items?sharingSetting=2&page=")
        .and_then(|s| s.strip_suffix("&pageSize=100"))
        .and_then(|s| s.parse::<u32>().ok())
    else {
        return (404, "{}".to_owned());
    };
    let page = if matches!(scenario, Scenario::IgnoredPage) {
        0
    } else {
        page
    };
    let total: u64 = if matches!(scenario, Scenario::Empty) {
        0
    } else {
        102
    };
    let start = u64::from(page) * 100;
    let count = total.saturating_sub(start).min(100);
    let mut rows: Vec<_> = (0..count)
        .map(|i| {
            let index = start + i;
            let entity = match index {
                2 => 9,
                3 => 8,
                _ => 7,
            };
            record(
                17 + index,
                !matches!(scenario, Scenario::EquipmentOnly)
                    && (matches!(scenario, Scenario::MagicOnly) || index % 2 == 1),
                entity,
            )
        })
        .collect();
    match scenario {
        Scenario::Incomplete => {
            rows.pop();
        }
        Scenario::Duplicate if rows.len() > 1 => rows[1] = rows[0].clone(),
        Scenario::InvalidSource if rows.len() > 1 => rows[1]["sources"][0]["sourceId"] = json!(999),
        Scenario::BadMagic if rows.len() > 1 => rows[1]["magic"] = Value::Null,
        Scenario::BadFilterMetadata if rows.len() > 1 => rows[1]["tags"] = json!([null]),
        _ => {}
    }
    let mut value = envelope(rows, page, total);
    if matches!(scenario, Scenario::Malformed) {
        value["data"] = json!({});
    }
    if matches!(scenario, Scenario::Unsupported) {
        value["accessTypes"] = json!({"7:17":1});
    }
    (200, value.to_string())
}

/// Starts one loopback service with explicit public/bearer header checks.
fn source(scenario: Scenario) -> std::io::Result<Source> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let origin = format!("http://{}", listener.local_addr()?);
    let client = HttpClient::for_fixture(&origin).map_err(std::io::Error::other)?;
    let stop = Arc::new(AtomicBool::new(false));
    let routes = Arc::new(Mutex::new(Vec::new()));
    let flag = Arc::clone(&stop);
    let paths = Arc::clone(&routes);
    let worker = thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            let (mut stream, _) = match listener.accept() {
                Ok(value) => value,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            };
            if stream.set_nonblocking(false).is_err()
                || stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .is_err()
                || stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .is_err()
            {
                break;
            }
            let mut header = Vec::new();
            let mut byte = [0];
            while !header.ends_with(b"\r\n\r\n") && header.len() < 16 * 1024 {
                if stream.read_exact(&mut byte).is_err() {
                    break;
                }
                header.push(byte[0]);
            }
            let request = String::from_utf8_lossy(&header);
            let route = request.split_whitespace().nth(1).unwrap_or_default();
            paths
                .lock()
                .expect("synthetic paths")
                .push(route.to_owned());
            if matches!(scenario, Scenario::Waiting) && route != "/api/config/json" {
                while !flag.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(2));
                }
                break;
            }
            let headers = request.to_ascii_lowercase();
            let cookie = headers.contains("\r\ncookie:");
            let bearer = headers.contains("\r\nauthorization:");
            let valid =
                request.starts_with("GET ") && !cookie && (bearer != (route == "/api/config/json"));
            let (status, body) = if valid {
                reply(scenario, route)
            } else {
                (401, "{}".to_owned())
            };
            let response = format!(
                "HTTP/1.1 {status} Synthetic\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    Ok(Source {
        client,
        origin,
        stop,
        worker: Some(worker),
        routes,
    })
}

/// Creates an invented authenticated owner without native credential access.
///
/// # Errors
///
/// Returns loopback listener or synthetic session setup failures.
pub fn session(scenario: Scenario) -> Result<Session, Box<dyn std::error::Error>> {
    let mut session = auth::session(if matches!(scenario, Scenario::Expired) {
        auth::Scenario::Expired
    } else {
        auth::Scenario::Valid
    })?;
    session.item_fixture = Some(Arc::new(source(scenario)?));
    Ok(session)
}

/// Returns safe paths to check request counts and exact selector encoding.
pub fn routes(session: &Session) -> Vec<String> {
    session
        .item_fixture
        .as_ref()
        .expect("fixture owner")
        .routes
        .lock()
        .expect("fixture paths")
        .clone()
}
