//! Compiled synthetic catalog services; no live accounts, rules text, or runtime overrides.

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

/// Selects a fixed invented success, failure, or limit case.
#[derive(Clone, Copy)]
pub enum Scenario {
    /// Two same-name classes and one subclass per parent.
    Normal,
    /// The same subclass ID occurs under two parents.
    Ambiguous,
    /// A subclass request receives HTTP 403.
    Denied,
    /// A protected class request receives HTTP 401.
    Rejected,
    /// The exchange returns expired bearer evidence.
    Expired,
    /// No class records exist in the successful terminal response.
    Empty,
    /// One parent has no subclass records.
    EmptySubclasses,
    /// The class response is malformed.
    Malformed,
    /// The class response carries nonterminal pagination.
    Incomplete,
    /// Duplicate class IDs occur in the raw catalog.
    Conflict,
    /// A raw source ID is not configured.
    InvalidSource,
    /// Public configuration contains an invalid category reference.
    InvalidConfiguration,
    /// Subclass metadata names a different parent.
    ChangedParent,
    /// Exactly 32 root definitions support bounded global discovery.
    ParentLimit,
    /// 33 roots remain valid for scoped, but not global, operations.
    TooManyParents,
    /// 1,001 raw class definitions exceed the record limit.
    TooManyRecords,
    /// One response exceeds the 1 MiB transport limit.
    Oversized,
    /// Valid compact metadata expands beyond 8 MiB when table rows are padded.
    WideTable,
    /// Valid JSON with whitespace exceeds the aggregate response-byte budget.
    AggregateBytes,
    /// Leaves the first protected resource waiting until the owner stops it.
    Waiting,
}

/// Owns only this loopback service and its single-attempt transport.
pub(crate) struct Source {
    /// Loopback transport exists only in compiled fixture builds.
    pub(crate) client: HttpClient,
    /// Replaces fixed origins only for this synthetic owner.
    pub(crate) origin: String,
    /// Stops the exact worker when its final owner is dropped.
    stop: Arc<AtomicBool>,
    /// Joins the owned service worker.
    worker: Option<thread::JoinHandle<()>>,
    /// Records paths only, never headers or credentials.
    routes: Arc<Mutex<Vec<String>>>,
}

impl Drop for Source {
    /// Stops and joins the only fixture worker.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Source {
    /// Starts a local source using only invented fixed response scenarios.
    fn start(scenario: Scenario) -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let origin = format!("http://{}", listener.local_addr()?);
        let client = HttpClient::for_fixture(&origin).map_err(std::io::Error::other)?;
        let stop = Arc::new(AtomicBool::new(false));
        let routes = Arc::new(Mutex::new(Vec::new()));
        let worker_stop = Arc::clone(&stop);
        let worker_routes = Arc::clone(&routes);
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::SeqCst) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock
                                | std::io::ErrorKind::ConnectionAborted
                                | std::io::ErrorKind::ConnectionReset
                        ) =>
                    {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
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
                let mut byte = [0; 1];
                while !header.ends_with(b"\r\n\r\n") && header.len() < 16 * 1024 {
                    match stream.read(&mut byte) {
                        Ok(1) => header.push(byte[0]),
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        _ => break,
                    }
                }
                if !header.ends_with(b"\r\n\r\n") {
                    continue;
                }
                let request = String::from_utf8_lossy(&header).to_ascii_lowercase();
                let route = request.split_whitespace().nth(1).unwrap_or_default();
                worker_routes
                    .lock()
                    .expect("synthetic paths")
                    .push(route.to_owned());
                let public = route == "/api/config/json";
                let authorized = request.contains("\r\nauthorization:");
                let valid_headers = !request.contains("\r\ncookie:")
                    && request.starts_with("get ")
                    && (public != authorized);
                if matches!(scenario, Scenario::Waiting) && !public {
                    while !worker_stop.load(Ordering::SeqCst) {
                        thread::sleep(Duration::from_millis(2));
                    }
                    break;
                }
                let (status, body) = if valid_headers {
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
        Ok(Self {
            client,
            origin,
            stop,
            worker: Some(worker),
            routes,
        })
    }
}

/// Supplies a synthetic raw definition containing discarded sentinel fields.
pub(super) fn record(id: u64, parent: Option<u64>, name: &str) -> Value {
    let source_id = if id == 2_190_879 || parent == Some(2_190_879) {
        8
    } else {
        7
    };
    json!({
        "id": id, "name": name, "parentClassId": parent,
        "sources": [{"sourceId":source_id,"future_reference_flag":true}],
        "isHomebrew": false, "slug": "invented",
        "moreDetailsUrl": "/classes/invented#Example",
        "description": "DISCARDED_DEFINITION_TEXT",
        "classFeatures": [{"description":"DISCARDED_FEATURE_TEXT"}],
        "equipmentDescription": "DISCARDED_EQUIPMENT_TEXT",
        "spellRules": {"text":"DISCARDED_SPELL_TEXT"},
        "unknownDefinition": {"text":"DISCARDED_UNKNOWN_TEXT"}
    })
}

/// Wraps invented definitions in the verified terminal envelope shape.
pub(super) fn envelope(records: Vec<Value>) -> Value {
    json!({"id":0,"success":true,"message":"","data":records,"pagination":null})
}

/// Chooses a fixed invented response for a known catalog/configuration path.
fn reply(scenario: Scenario, route: &str) -> (u16, String) {
    if route == "/api/config/json" {
        let category = if matches!(scenario, Scenario::InvalidConfiguration) {
            99
        } else {
            1
        };
        return (
            200,
            json!({
                "sources":[{"id":7,"sourceCategoryId":category},{"id":8,"sourceCategoryId":2}],
                "sourceCategories":[{"id":1,"name":"Invented"},{"id":2,"name":"Hidden Sources"}]
            })
            .to_string(),
        );
    }
    if route == "/character/v5/game-data/classes?sharingsetting=2" {
        if matches!(scenario, Scenario::Rejected) {
            return (401, "{}".to_owned());
        }
        if matches!(scenario, Scenario::Malformed) {
            return (200, "{}".to_owned());
        }
        if matches!(scenario, Scenario::Oversized) {
            return (200, " ".repeat(1024 * 1024 + 1));
        }
        let records = match scenario {
            Scenario::Empty => vec![],
            Scenario::WideTable => (1..=1_000)
                .map(|id| {
                    record(
                        id,
                        None,
                        &if id == 1 {
                            "x".repeat(9_000)
                        } else {
                            "Invented".to_owned()
                        },
                    )
                })
                .collect(),
            Scenario::ParentLimit
            | Scenario::TooManyParents
            | Scenario::TooManyRecords
            | Scenario::AggregateBytes => {
                let count = match scenario {
                    Scenario::ParentLimit => 32,
                    Scenario::TooManyParents => 33,
                    Scenario::TooManyRecords => 1_001,
                    _ => 9,
                };
                (1..=count)
                    .map(|id| record(id, None, "Invented Fighter"))
                    .collect()
            }
            Scenario::Conflict => vec![
                record(10, None, "Invented Fighter"),
                record(10, None, "Invented Fighter"),
            ],
            _ => vec![
                record(10, None, "Invented Fighter"),
                record(2_190_879, None, "Invented Fighter"),
            ],
        };
        let mut response = envelope(records);
        if matches!(scenario, Scenario::Incomplete) {
            response["pagination"] = json!({"next":1});
        }
        if matches!(scenario, Scenario::InvalidSource) {
            response["data"][0]["sources"][0]["sourceId"] = json!(999);
        }
        return (200, response.to_string());
    }
    if let Some(parent) = route
        .strip_prefix("/character/v5/game-data/subclasses?sharingsetting=2&baseclassid=")
        .and_then(|id| id.parse::<u64>().ok())
    {
        if matches!(scenario, Scenario::Denied) {
            return (403, "{}".to_owned());
        }
        let id = if parent == 2_190_879 || matches!(scenario, Scenario::Ambiguous) {
            7001
        } else {
            7002
        };
        let actual_parent = if matches!(scenario, Scenario::ChangedParent) {
            parent + 1
        } else {
            parent
        };
        let entries = if matches!(scenario, Scenario::EmptySubclasses) {
            vec![]
        } else {
            vec![record(id, Some(actual_parent), "Invented Champion")]
        };
        let mut body = envelope(entries).to_string();
        if matches!(scenario, Scenario::AggregateBytes) {
            body.push_str(&" ".repeat(1024 * 1024 - 1024 - body.len()));
        }
        return (200, body);
    }
    (404, "{}".to_owned())
}

/// Builds isolated invented storage, exchange, and source services.
///
/// # Errors
///
/// Returns local fixture setup errors. No native store or live account is used.
pub fn session(scenario: Scenario) -> std::io::Result<Session> {
    let auth_scenario = if matches!(scenario, Scenario::Expired) {
        auth::Scenario::Expired
    } else {
        auth::Scenario::Valid
    };
    let mut session = auth::session(auth_scenario).map_err(std::io::Error::other)?;
    session.class_catalog_fixture = Some(Arc::new(Source::start(scenario)?));
    Ok(session)
}

/// Returns only the requested synthetic route paths.
pub fn requested_routes(session: &Session) -> Vec<String> {
    session
        .class_catalog_fixture
        .as_ref()
        .map(|source| source.routes.lock().expect("synthetic routes").clone())
        .unwrap_or_default()
}
