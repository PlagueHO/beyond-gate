//! Owned loopback feat sources with invented records and no product runtime selector.

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

/// Fixed synthetic cases; production inputs cannot select a fixture.
#[derive(Clone, Copy)]
pub enum Scenario {
    /// Duplicate names, repeatable parent/child metadata, and unknown raw fields.
    Normal,
    /// An explicit empty listing and a complete empty collection.
    Empty,
    /// HTTP 403 denies a protected read.
    Denied,
    /// HTTP 401 rejects the session.
    Rejected,
    /// The synthetic exchange reports expiry.
    Expired,
    /// Required source structure is missing.
    Malformed,
    /// Unverified paging or a nonterminal envelope.
    Incomplete,
    /// Two rows share an identity.
    Duplicate,
    /// An unselected definition refers to an unknown source.
    InvalidSource,
    /// An unexpected content access map requires another contract.
    Unsupported,
    /// The response exceeds the unchanged transport cap.
    Oversized,
    /// Leaves resource reads waiting until the owner is dropped.
    Waiting,
}

/// Owns only this fixture's worker, transport, and safe route log.
pub(crate) struct Source {
    /// Direct single-attempt transport at the owned loopback origin.
    pub(crate) client: HttpClient,
    /// Compiled-only replacement for the fixed source origins.
    pub(crate) origin: String,
    /// Stops this worker when its final owner drops.
    stop: Arc<AtomicBool>,
    /// Joins only the worker started by this source.
    worker: Option<thread::JoinHandle<()>>,
    /// Records paths, never cookies, headers, or source text.
    routes: Arc<Mutex<Vec<String>>>,
}

impl Drop for Source {
    /// Stops and joins this owned fixture without affecting other processes.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Supplies an invented repeatable feat with unknown nested source extensions.
pub(super) fn record(id: u64) -> Value {
    json!({"id":id,"name":"Invented Feat","entityTypeId":42,
        "moreDetailsUrl":format!("/feats/{id}-invented"),"isRepeatable":true,
        "repeatableParentId":if id == 18 {Some(17)} else {None},
        "prerequisites":[{"future":{"kind":"invented"}}],
        "sources":[{"sourceId":7,"pageNumber":1,"sourceType":1,"future":true}],
        "description":"Synthetic feat text.","future":{"nested":[true,null,3],"controls":"tab\tline\n\u{1b}[31m"}})
}

/// Provides a provisional terminal collection for offline contract tests only.
pub(super) fn envelope(rows: Vec<Value>) -> Value {
    json!({"id":0,"success":true,"message":"","data":rows,"pagination":null})
}

/// Escapes a literal synthetic input when writing its form echo.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Builds observed form controls and invented rows with checked selector echoes.
pub(super) fn website(scenario: Scenario, route: &str) -> String {
    if matches!(scenario, Scenario::Malformed) {
        return "<html>synthetic malformed page</html>".to_owned();
    }
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{route}")).expect("synthetic URL");
    let pairs: Vec<_> = url.query_pairs().collect();
    let name = pairs
        .iter()
        .find(|(k, _)| k == "filter-name")
        .map(|(_, v)| v.as_ref())
        .unwrap_or("");
    let mut body = format!(
        "<form method=\"get\" action=\"/feats\"><input type=\"text\" name=\"filter-name\" value=\"{}\">",
        escaped(name)
    );
    for field in [
        "filter-prereq-subtype",
        "filter-tags",
        "filter-source",
        "filter-source-category",
        "filter-partnered-content",
    ] {
        let multiple = if matches!(
            field,
            "filter-tags" | "filter-source" | "filter-source-category"
        ) {
            " multiple"
        } else {
            ""
        };
        body.push_str(&format!("<select name=\"{field}\"{multiple}>"));
        for value in ["", "7", "8", "240", "241", "f", "t"] {
            let selected = if pairs.iter().any(|(k, v)| k == field && v == value) {
                " selected"
            } else {
                ""
            };
            body.push_str(&format!(
                "<option value=\"{value}\"{selected}>Invented option</option>"
            ));
        }
        body.push_str("</select>");
    }
    body.push_str("</form><ul class=\"listing listing-rpgfeat rpgfeat-listing\">");
    if matches!(scenario, Scenario::Empty) || name == "missing" {
        body.push_str("<li class=\"alert no-results\">Invented empty result</li>");
    } else {
        for id in [
            17,
            if matches!(scenario, Scenario::Duplicate) {
                17
            } else {
                18
            },
        ] {
            body.push_str(&format!("<li><div class=\"list-row-name-primary-text\"><a href=\"/feats/{id}-invented\">Invented Feat</a></div><div class=\"list-row-name-secondary-text\">Invented Source</div></li>"));
        }
    }
    body.push_str("</ul><div class=\"listing-footer\"></div>");
    if matches!(scenario, Scenario::Incomplete) {
        body.push_str(
            "<ul class=\"b-pagination-list\"><li><a href=\"/feats?page=2\">Next</a></li></ul>",
        );
    }
    body
}

/// Chooses invented responses only for exact known source paths.
fn reply(scenario: Scenario, route: &str) -> (u16, String) {
    if route == "/api/config/json" {
        return (
            200,
            json!({"sources":[{"id":7,"sourceCategoryId":1}],
            "sourceCategories":[{"id":1,"name":"Invented"}]})
            .to_string(),
        );
    }
    match scenario {
        Scenario::Denied => return (403, "{}".to_owned()),
        Scenario::Rejected => return (401, "{}".to_owned()),
        Scenario::Oversized => return (200, " ".repeat(1024 * 1024 + 1)),
        _ => {}
    }
    if route.starts_with("/feats?") {
        return (200, website(scenario, route));
    }
    if route != "/character/v5/game-data/feats?sharingSetting=2" {
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
        Scenario::Unsupported => response["accessTypes"] = json!({"17":1}),
        _ => {}
    }
    (200, response.to_string())
}

/// Starts a local service that verifies cookie/bearer separation and records safe paths.
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
                    thread::sleep(Duration::from_millis(2))
                }
                break;
            }
            let headers = request.to_ascii_lowercase();
            let html = route.starts_with("/feats?");
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

/// Creates an authenticated synthetic owner without native credential access.
///
/// # Errors
///
/// Returns listener or synthetic-session setup failures.
pub fn session(scenario: Scenario) -> Result<Session, Box<dyn std::error::Error>> {
    let mut session = auth::session(if matches!(scenario, Scenario::Expired) {
        auth::Scenario::Expired
    } else {
        auth::Scenario::Valid
    })?;
    session.feat_fixture = Some(Arc::new(source(scenario)?));
    Ok(session)
}

/// Returns safe synthetic paths for request-count and encoding checks.
pub fn routes(session: &Session) -> Vec<String> {
    session
        .feat_fixture
        .as_ref()
        .expect("fixture owner")
        .routes
        .lock()
        .expect("fixture paths")
        .clone()
}
