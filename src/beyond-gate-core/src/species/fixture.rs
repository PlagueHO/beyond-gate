//! Owner-scoped loopback species responses with invented data only.

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

/// Fixed offline source cases, never selected by a product invocation.
#[derive(Clone, Copy)]
pub enum Scenario {
    /// Two records with legacy and subrace metadata and an unknown field.
    Normal,
    /// A valid empty collection.
    Empty,
    /// The protected service denies the read.
    Denied,
    /// The protected service rejects the session.
    Rejected,
    /// The exchange establishes expiry.
    Expired,
    /// The source returns a response with the wrong shape or invalid identity.
    Malformed,
    /// The source returns an explicit nonterminal pagination envelope.
    Incomplete,
    /// The source returns an unexpected access map or route.
    Unsupported,
    /// The raw collection repeats an identity.
    Duplicate,
    /// The raw collection exceeds the record limit.
    TooManyRecords,
    /// The response body exceeds the shared HTTP transport limit.
    Oversized,
    /// Leaves the protected route waiting until its owner is dropped.
    Waiting,
}

/// Owns the fixture worker, safe path log, and compiled-only transport.
pub(crate) struct Source {
    /// Uses HTTP only at this synthetic loopback origin.
    pub(crate) client: HttpClient,
    /// Replaces the fixed production origin only for this owner.
    pub(crate) origin: String,
    /// Stops the worker when the final owner drops.
    stop: Arc<AtomicBool>,
    /// Joins only the worker owned by this fixture.
    worker: Option<thread::JoinHandle<()>>,
    /// Records routes only, never headers or credentials.
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

/// Returns one invented species record with an unknown nested field.
fn record(id: u64, full_name: &str, base_name: &str, legacy: bool, subrace: bool) -> Value {
    json!({
        "entityRaceId": id,
        "entityRaceTypeId": 4,
        "fullName": full_name,
        "baseName": base_name,
        "baseRaceName": if subrace { "Invented Origin" } else { "" },
        "description": "Synthetic species detail.",
        "isHomebrew": false,
        "isLegacy": legacy,
        "isSubRace": subrace,
        "size": "Medium",
        "sources": [{"sourceId": 7, "future_reference_flag": true}],
        "future_definition": {"nested": [true, null, 3]}
    })
}

/// Builds the complete synthetic array for one fixed source scenario.
fn response(scenario: Scenario) -> String {
    match scenario {
        Scenario::Empty => json!({
            "id": 0, "success": true, "message": "", "data": [], "pagination": null
        })
        .to_string(),
        Scenario::Malformed => "{}".to_owned(),
        Scenario::Incomplete => {
            json!({"data":[record(17, "Invented Origin", "Invented Origin", false, false)],
                "id":0,"success":true,"message":"","pagination":{"next":2}})
            .to_string()
        }
        Scenario::Unsupported => {
            json!({"id":0,"success":true,"message":"","pagination":null,
                "accessType":{"17":1},"data":[record(17, "Invented Origin", "Invented Origin", false, false)]})
                .to_string()
        }
        Scenario::Duplicate => json!({
            "id":0,"success":true,"message":"","pagination":null,
            "data":[record(17, "Invented Origin", "Invented Origin", false, false),
                record(17, "Invented Legacy", "Invented Legacy", true, false)]
        })
        .to_string(),
        Scenario::TooManyRecords => json!({
            "id":0,"success":true,"message":"","pagination":null,
            "data":(1..=super::MAX_RECORDS + 1)
                .map(|id| record(id as u64, "Invented Origin", "Invented Origin", false, false))
                .collect::<Vec<_>>()
        })
        .to_string(),
        Scenario::Oversized => " ".repeat(1024 * 1024 + 1),
        _ => json!({
            "id":0,"success":true,"message":"","pagination":null,
            "data":[
                record(17, "Invented Origin", "Invented Origin", false, false),
                record(18, "Invented Coastal Origin", "Invented Origin", true, true)
            ]
        })
        .to_string(),
    }
}

/// Builds invented species cards with the observed public listing structure.
pub(super) fn website(scenario: Scenario) -> String {
    if matches!(scenario, Scenario::Malformed) {
        return "<html>synthetic malformed page</html>".to_owned();
    }
    if matches!(scenario, Scenario::Unsupported) {
        return "<form id=\"challenge-form\"></form>".to_owned();
    }
    let rows = if matches!(scenario, Scenario::TooManyRecords) {
        (1..=super::MAX_RECORDS + 1).collect::<Vec<_>>()
    } else if matches!(scenario, Scenario::Duplicate) {
        vec![17, 17]
    } else if matches!(scenario, Scenario::Empty) {
        Vec::new()
    } else {
        vec![17, 18]
    };
    let mut body = "<div class=\"listing-container listing-container-ul RPGRace-listing card-listing\"><div class=\"listing-body\"><ul class=\"listing listing-rpgrace rpgrace-listing\">".to_owned();
    for id in rows {
        body.push_str(&format!(
            "<li class=\"listing-card\"><a class=\"listing-card__link\" href=\"/species/{id}-invented\"><div class=\"listing-card__body\"><h3 class=\"listing-card__title\">Invented Origin {id}</h3><div class=\"listing-card__source\">Invented Source</div>{}</div></a></li>",
            if id == 18 {
                "<span class=\"badge-label\">Legacy</span>"
            } else {
                ""
            }
        ));
    }
    body.push_str("</ul></div>");
    if !matches!(scenario, Scenario::Incomplete) {
        body.push_str("<div class=\"listing-footer\"></div>");
    }
    body.push_str("</div>");
    if matches!(scenario, Scenario::Incomplete) {
        body.push_str(
            "<ul class=\"b-pagination-list\"><li><a href=\"/species?page=2\">Next</a></li></ul>",
        );
    }
    body
}

/// Starts a local single-attempt service with cookie-only listing and bearer-only detail.
fn start(scenario: Scenario) -> std::io::Result<Source> {
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
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
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
            let mut headers = Vec::new();
            let mut byte = [0];
            while !headers.ends_with(b"\r\n\r\n") && headers.len() < 16 * 1024 {
                match stream.read(&mut byte) {
                    Ok(1) => headers.push(byte[0]),
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    _ => break,
                }
            }
            if !headers.ends_with(b"\r\n\r\n") {
                continue;
            }
            let request = String::from_utf8_lossy(&headers);
            let route = request.split_whitespace().nth(1).unwrap_or_default();
            worker_routes
                .lock()
                .expect("synthetic paths")
                .push(route.to_owned());
            if matches!(scenario, Scenario::Waiting) {
                while !worker_stop.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(2));
                }
                break;
            }
            let headers = request.to_ascii_lowercase();
            let authorized = headers.contains("\r\nauthorization:");
            let has_cookie = headers.contains("\r\ncookie:");
            let listing = route == "/species";
            let valid = request.starts_with("GET ")
                && if listing {
                    has_cookie && !authorized
                } else {
                    authorized && !has_cookie
                };
            let (status, body) = if !valid {
                (401, "{}".to_owned())
            } else {
                match (listing, scenario) {
                    (_, Scenario::Denied) => (403, "{}".to_owned()),
                    (_, Scenario::Rejected) => (401, "{}".to_owned()),
                    (true, Scenario::Oversized) => (200, " ".repeat(1024 * 1024 + 1)),
                    (true, _) => (200, website(scenario)),
                    (false, _) if route != super::COLLECTION => (404, "{}".to_owned()),
                    (false, Scenario::Unsupported) => (404, "{}".to_owned()),
                    (false, _) => (200, response(scenario)),
                }
            };
            let media = if listing {
                "text/html"
            } else {
                "application/json"
            };
            let reply = format!(
                "HTTP/1.1 {status} Synthetic\r\nContent-Type: {media}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(reply.as_bytes());
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

/// Builds an isolated invented session and loopback species service.
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
    session.species_fixture = Some(Arc::new(start(scenario)?));
    Ok(session)
}

/// Returns only safe synthetic routes for path and request-count assertions.
pub fn requested_routes(session: &Session) -> Vec<String> {
    session
        .species_fixture
        .as_ref()
        .map(|source| source.routes.lock().expect("synthetic paths").clone())
        .unwrap_or_default()
}
