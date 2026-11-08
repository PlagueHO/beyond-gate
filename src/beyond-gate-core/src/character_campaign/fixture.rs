//! Compiled invented character and campaign services for offline contract checks.

use crate::{
    auth::session::{Session, fixture as auth},
    http::HttpClient,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

/// Selects one fixed synthetic source behavior.
#[derive(Clone, Copy)]
pub enum Scenario {
    /// Supplies complete character and campaign responses.
    Normal,
    /// Denies the selected campaign roster without retry.
    DeniedRoster,
    /// Returns an incomplete character snapshot.
    IncompleteCharacters,
    /// Returns malformed campaign summaries.
    MalformedCampaigns,
    /// Returns a successful campaign list without the selected campaign.
    MissingCampaign,
    /// Returns conflicting campaign summaries with the same identifier.
    ConflictingCampaigns,
    /// Returns a selected character with a different source identifier.
    ChangedCharacter,
    /// Returns not found for the selected character route.
    MissingCharacter,
}

/// Owns one loopback source and the shared bounded HTTP client.
pub(crate) struct Source {
    /// Restricts all resource requests to this synthetic loopback origin.
    pub(crate) client: HttpClient,
    /// Replaces only the fixed production hostname in compiled fixture builds.
    pub(crate) origin: String,
    /// Stops only this local service.
    stop: Arc<AtomicBool>,
    /// Joins the service worker when its owning session is dropped.
    worker: Option<thread::JoinHandle<()>>,
    /// Counts source requests, not protected-session exchanges.
    requests: Arc<AtomicUsize>,
    /// Keeps only synthetic route paths for exact request assertions.
    routes: Arc<Mutex<Vec<String>>>,
}

impl Drop for Source {
    /// Stops and joins this fixture's only worker.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Source {
    /// Starts a local service with fixed invented JSON responses.
    fn start(scenario: Scenario) -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let origin = format!("http://{}", listener.local_addr()?);
        listener.set_nonblocking(true)?;
        let client = HttpClient::for_fixture(&origin).map_err(std::io::Error::other)?;
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(AtomicUsize::new(0));
        let routes = Arc::new(Mutex::new(Vec::new()));
        let worker_stop = Arc::clone(&stop);
        let worker_requests = Arc::clone(&requests);
        let worker_routes = Arc::clone(&routes);
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::SeqCst) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock
                                | std::io::ErrorKind::Interrupted
                                | std::io::ErrorKind::ConnectionAborted
                                | std::io::ErrorKind::ConnectionReset
                        ) =>
                    {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(_) => break,
                };
                if stream.set_nonblocking(false).is_err()
                    || stream
                        .set_read_timeout(Some(Duration::from_secs(10)))
                        .is_err()
                    || stream
                        .set_write_timeout(Some(Duration::from_secs(10)))
                        .is_err()
                {
                    break;
                }
                let mut header = Vec::new();
                let mut buffer = [0; 1024];
                while !header.ends_with(b"\r\n\r\n") && header.len() < 16 * 1024 {
                    let read = match stream.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(read) => read,
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    };
                    header.extend_from_slice(&buffer[..read]);
                }
                if !header.ends_with(b"\r\n\r\n") {
                    continue;
                }
                worker_requests.fetch_add(1, Ordering::SeqCst);
                let request = String::from_utf8_lossy(&header).to_ascii_lowercase();
                let route = request.split_whitespace().nth(1).unwrap_or_default();
                let authorized = request.contains("\r\nauthorization:");
                worker_routes
                    .lock()
                    .expect("synthetic route log")
                    .push(route.to_owned());
                let (status, body) = if !authorized || request.contains("\r\ncookie:") {
                    (401, "{}".to_owned())
                } else {
                    reply(scenario, route)
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
            requests,
            routes,
        })
    }
}

/// Returns one synthetic response for a fixed route.
fn reply(scenario: Scenario, route: &str) -> (u16, String) {
    if route.starts_with("/character/v5/characters/list?userid=") {
        let total = if matches!(scenario, Scenario::IncompleteCharacters) {
            3
        } else {
            2
        };
        return (
            200,
            format!(
                r#"{{"success":true,"data":{{"characters":[{{"id":42,"name":"Invented Hero","campaignId":8,"unknown":{{"preserved":true}}}},{{"id":43,"name":"Invented Ally"}}]}},"pagination":{{"skip":0,"take":100,"total":{total}}}}}"#
            ),
        );
    }
    if route == "/character/v5/character/42?includecustomitems=true" {
        if matches!(scenario, Scenario::MissingCharacter) {
            return (404, "{}".to_owned());
        }
        let id = if matches!(scenario, Scenario::ChangedCharacter) {
            43
        } else {
            42
        };
        return (
            200,
            format!(
                r#"{{"success":true,"data":{{"id":{id},"name":"Invented Hero","userId":42,"campaign":{{"id":8}},"unknown":true}},"pagination":null}}"#
            ),
        );
    }
    if route == "/api/campaign/stt/user-campaigns" {
        if matches!(scenario, Scenario::MissingCampaign) {
            return (200, r#"{"status":"success","data":[]}"#.to_owned());
        }
        if matches!(scenario, Scenario::MalformedCampaigns) {
            return (200, r#"{"status":"unknown","data":[]}"#.to_owned());
        }
        if matches!(scenario, Scenario::ConflictingCampaigns) {
            return (
                200,
                r#"{"status":"success","data":[{"id":8,"name":"Invented Campaign","playerCount":2},{"id":8,"name":"Different Campaign","playerCount":2}]}"#
                    .to_owned(),
            );
        }
        return (
            200,
            r#"{"status":"success","data":[{"id":8,"name":"Invented Campaign","playerCount":2,"unknown":{"kept":true}}]}"#.to_owned(),
        );
    }
    if route == "/api/campaign/stt/active-campaigns" {
        return (
            200,
            r#"{"status":"success","data":[{"id":8,"name":"Invented Campaign","playerCount":2}]}"#
                .to_owned(),
        );
    }
    if route == "/api/campaign/stt/active-short-characters/8" {
        if matches!(scenario, Scenario::DeniedRoster) {
            return (403, "{}".to_owned());
        }
        return (
            200,
            r#"{"data":[{"id":42,"name":"Invented Hero","userId":42,"unknown":{"kept":true}}]}"#
                .to_owned(),
        );
    }
    (404, "{}".to_owned())
}

/// Returns the number of source requests observed by the synthetic service.
pub fn request_count(session: &Session) -> usize {
    session
        .character_campaign_fixture
        .as_ref()
        .map_or(0, |source| source.requests.load(Ordering::SeqCst))
}

/// Returns only fixed synthetic paths, never headers or credential values.
pub fn requested_routes(session: &Session) -> Vec<String> {
    session
        .character_campaign_fixture
        .as_ref()
        .and_then(|source| source.routes.lock().ok().map(|routes| routes.clone()))
        .unwrap_or_default()
}

/// Builds an isolated protected session and synthetic resource service.
///
/// # Errors
///
/// Returns local socket or synthetic session setup failures. It never opens
/// native credential storage or contacts a live service.
pub fn session(scenario: Scenario) -> std::io::Result<Session> {
    let mut session = auth::session(auth::Scenario::Valid).map_err(std::io::Error::other)?;
    session.character_campaign_fixture = Some(Arc::new(Source::start(scenario)?));
    Ok(session)
}
