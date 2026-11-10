//! Owner-scoped loopback background responses with invented content and no runtime overrides.

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

/// Fixed offline source cases, never selected by a production argument.
#[derive(Clone, Copy)]
pub enum Scenario {
    /// Two equal names with distinct IDs and preserved raw extensions.
    Normal,
    /// Explicit empty website result and empty terminal collection.
    Empty,
    /// HTTP 403 denies the protected read.
    Denied,
    /// HTTP 401 rejects the session.
    Rejected,
    /// Exchange establishes expiry.
    Expired,
    /// Missing website structure or malformed collection.
    Malformed,
    /// Nonterminal collection or unexpected website paging.
    Incomplete,
    /// Duplicate raw identities.
    Duplicate,
    /// Source reference is not configured.
    InvalidSource,
    /// An unexpected access map requires a new contract.
    Unsupported,
    /// Body exceeds the unchanged transport cap.
    Oversized,
    /// Compact metadata fits, but padding the table would exceed 8 MiB.
    WideTable,
    /// Leaves the owned resource waiting until cancellation.
    Waiting,
}

/// Owns the fixture worker, safe path log, and compiled-only transport.
pub(crate) struct Source {
    /// Uses HTTP only at this synthetic loopback origin.
    pub(crate) client: HttpClient,
    /// Replaces fixed production origins only in this owner.
    pub(crate) origin: String,
    /// Ends the worker when the final owner drops.
    stop: Arc<AtomicBool>,
    /// Joins only the owned worker.
    worker: Option<thread::JoinHandle<()>>,
    /// Records routes, never headers or credentials.
    routes: Arc<Mutex<Vec<String>>>,
}

impl Drop for Source {
    /// Stops and joins this fixture's worker without touching other processes.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Supplies invented raw data with an unknown nested field and source extension.
pub(super) fn record(id: u64) -> Value {
    json!({"id":id,"name":"Invented Origin","entityTypeId":42,
        "moreDetailsUrl":format!("/backgrounds/{id}-invented"),
        "sources":[{"sourceId":7,"pageNumber":1,"sourceType":1,"future":true}],
        "description":"Synthetic background text.","future":{"nested":[true,null,3],"controls":"tab\tline\n\u{1b}[31m"}})
}

/// Wraps invented records in the observed terminal response shape.
pub(super) fn envelope(rows: Vec<Value>) -> Value {
    json!({"id":0,"success":true,"message":"","data":rows,"pagination":null})
}

/// Builds only the observed filter controls and synthetic metadata rows.
pub(super) fn website(scenario: Scenario, route: &str) -> String {
    if matches!(scenario, Scenario::Malformed) {
        return "<html>synthetic malformed page</html>".to_owned();
    }
    let mut body =
        "<form method=\"get\" action=\"/backgrounds\"><input type=\"text\" name=\"filter-name\">"
            .to_owned();
    for (_, _, name) in super::query::ID_FILTERS {
        body.push_str(&format!("<select name=\"{name}\" multiple>"));
        for id in [7, 8, 240, 241] {
            body.push_str(&format!("<option value=\"{id}\">Invented option</option>"));
        }
        body.push_str("</select>");
    }
    body.push_str("<select name=\"filter-partnered-content\"><option value=\"\"></option><option value=\"t\">Yes</option><option value=\"f\">No</option></select></form><ul class=\"listing listing-rpgbackground rpgbackground-listing\">");
    let empty = matches!(scenario, Scenario::Empty) || route.contains("filter-name=missing");
    if empty {
        body.push_str("<li class=\"alert no-results\">Invented empty result</li>");
    } else if matches!(scenario, Scenario::WideTable) {
        for id in 1..=1000 {
            let source = if id == 1 {
                "s".repeat(16_384)
            } else {
                "Invented".to_owned()
            };
            body.push_str(&format!("<li><div class=\"list-row-name-primary-text\"><a href=\"/backgrounds/{id}-invented\">Invented Origin</a></div><div class=\"list-row-name-secondary-text\">{source}</div></li>"));
        }
    } else {
        for id in [
            17,
            if matches!(scenario, Scenario::Duplicate) {
                17
            } else {
                18
            },
        ] {
            body.push_str(&format!("<li><div class=\"list-row-name-primary-text\"><a href=\"/backgrounds/{id}-invented\">Invented Origin</a></div><div class=\"list-row-name-secondary-text\">Invented Source</div></li>"));
        }
    }
    body.push_str("</ul><div class=\"listing-footer\"></div>");
    if matches!(scenario, Scenario::Incomplete) {
        body.push_str("<ul class=\"b-pagination-list\"><li><a href=\"/backgrounds?page=2\">Next</a></li></ul>");
    }
    body
}

/// Chooses an invented response for an exact fixed route.
fn reply(scenario: Scenario, route: &str) -> (u16, String) {
    if route == "/api/config/json" {
        return (200,json!({"sources":[{"id":7,"sourceCategoryId":1}],"sourceCategories":[{"id":1,"name":"Invented"}]}).to_string());
    }
    match scenario {
        Scenario::Denied => return (403, "{}".to_owned()),
        Scenario::Rejected => return (401, "{}".to_owned()),
        Scenario::Oversized => return (200, " ".repeat(1024 * 1024 + 1)),
        _ => {}
    }
    if route.starts_with("/backgrounds?") {
        return (200, website(scenario, route));
    }
    if route != "/character/v5/game-data/backgrounds?sharingSetting=2" {
        return (404, "{}".to_owned());
    }
    let rows = if matches!(scenario, Scenario::Empty) {
        vec![]
    } else {
        vec![
            record(17),
            record(if matches!(scenario, Scenario::Duplicate) {
                17
            } else {
                18
            }),
        ]
    };
    let mut response = envelope(rows);
    match scenario {
        Scenario::Malformed => response = json!({}),
        Scenario::Incomplete => response["pagination"] = json!({"next":"unverified"}),
        Scenario::InvalidSource => response["data"][1]["sources"][0]["sourceId"] = json!(999),
        Scenario::Unsupported => response["accessType"] = json!({"17":1,"18":1}),
        _ => {}
    }
    (200, response.to_string())
}

/// Starts a local single-attempt service that checks cookie/bearer separation.
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
            let html = route.starts_with("/backgrounds?");
            let public = route == "/api/config/json";
            let cookie = headers.contains("\r\ncookie:");
            let bearer = headers.contains("\r\nauthorization:");
            let valid = request.starts_with("GET ")
                && if public {
                    !cookie && !bearer
                } else if html {
                    cookie && !bearer
                } else {
                    bearer && !cookie
                };
            let (status, body) = if valid {
                reply(scenario, route)
            } else {
                (401, "{}".to_owned())
            };
            let media = if html {
                "text/html"
            } else {
                "application/json"
            };
            let response = format!(
                "HTTP/1.1 {status} Synthetic\r\nContent-Type: {media}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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

/// Supplies a synthetic authenticated owner without native credential access.
///
/// # Errors
///
/// Returns listener or synthetic-session setup errors.
pub fn session(scenario: Scenario) -> Result<Session, Box<dyn std::error::Error>> {
    let mut session = auth::session(if matches!(scenario, Scenario::Expired) {
        auth::Scenario::Expired
    } else {
        auth::Scenario::Valid
    })?;
    session.background_fixture = Some(Arc::new(source(scenario)?));
    Ok(session)
}

/// Returns only safe synthetic paths for request-count and encoding assertions.
pub fn routes(session: &Session) -> Vec<String> {
    session
        .background_fixture
        .as_ref()
        .expect("fixture owner")
        .routes
        .lock()
        .expect("fixture paths")
        .clone()
}
