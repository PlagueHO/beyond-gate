//! Explicit development fixtures with invented credentials and a loopback service.
//!
//! Product commands have no fixture selector. Callers cannot supply a cookie,
//! store identity, response, or service URL through this interface.

use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use keyring_core::mock;
use serde_json::json;

use super::*;
use crate::http::direct_client_builder;

/// Selects fixed invented success and failure cases, never live input.
#[derive(Clone, Copy)]
pub enum Scenario {
    /// A valid session with future service-issued bearer evidence.
    Valid,
    /// The fixed entry does not exist.
    Absent,
    /// The service rejects the invented cookie.
    Rejected,
    /// The response has verified expired bearer evidence.
    Expired,
    /// The stored bytes fail the envelope contract.
    Invalid,
    /// The stored profile is unknown.
    UnknownProfile,
    /// Protected storage is locked or denied.
    Locked,
    /// The protected store cannot be used.
    Unavailable,
    /// Multiple entries match.
    Ambiguous,
    /// The service response is not valid JSON.
    Malformed,
    /// The service rate-limits the request without a retry.
    RateLimited,
    /// Leaves validation pending until cancellation or its bounded client timeout.
    Waiting,
}

/// Owns the exact loopback listener's bounded worker and shutdown signal.
pub(super) struct Service {
    /// Signals only this fixture's worker.
    stop: Arc<AtomicBool>,
    /// Joins the owned thread after bounded socket work.
    worker: Option<std::thread::JoinHandle<()>>,
    /// Counts actual accepted mock requests.
    pub(super) requests: Arc<AtomicUsize>,
    /// Supplies mutable responses for core policy checks.
    #[cfg(test)]
    pub(super) response: Arc<Mutex<(u16, Vec<u8>)>>,
}

impl Drop for Service {
    /// Stops only this fixture's worker; no native entry or external process exists.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Builds a bounded invented token response without a real account identifier.
pub(super) fn response(expiry: u64) -> Vec<u8> {
    let claims = json!({
        "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier": 42,
        "exp": expiry
    });
    let payload = URL_SAFE_NO_PAD.encode(claims.to_string());
    format!(r#"{{"token":"e30.{payload}.c3ludGhldGlj"}}"#).into_bytes()
}

/// Starts an isolated loopback service; setup errors remain typed and redacted.
fn start(status: u16, body: Vec<u8>, waiting: bool) -> Result<(Service, String), SessionError> {
    let setup = |_| SessionError::new(SessionErrorKind::ValidationUnavailable);
    let listener = TcpListener::bind("127.0.0.1:0").map_err(setup)?;
    let address = listener.local_addr().map_err(setup)?;
    listener.set_nonblocking(true).map_err(setup)?;
    let stop = Arc::new(AtomicBool::new(false));
    let requests = Arc::new(AtomicUsize::new(0));
    let response = Arc::new(Mutex::new((status, body)));
    let flag = Arc::clone(&stop);
    let count = Arc::clone(&requests);
    let replies = Arc::clone(&response);
    let worker = std::thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(_) => break,
            };
            count.fetch_add(1, Ordering::SeqCst);
            if stream.set_nonblocking(false).is_err()
                || stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .is_err()
                || stream
                    .set_write_timeout(Some(Duration::from_secs(1)))
                    .is_err()
            {
                break;
            }
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") && header.len() <= 16 * 1024 {
                let mut byte = [0];
                if stream.read_exact(&mut byte).is_err() {
                    break;
                }
                if waiting {
                    while !flag.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    break;
                }
                header.push(byte[0]);
            }
            let Ok(reply) = replies.lock() else { break };
            let head = format!(
                "HTTP/1.1 {} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                reply.0,
                reply.1.len()
            );
            if stream
                .write_all(head.as_bytes())
                .and_then(|()| stream.write_all(&reply.1))
                .is_err()
            {
                break;
            }
        }
    });
    Ok((
        Service {
            stop,
            worker: Some(worker),
            requests,
            #[cfg(test)]
            response,
        },
        format!("http://{address}/v1/cobalt-token"),
    ))
}

/// Counts actual loopback requests made by the selected synthetic session.
pub fn request_count(session: &Session) -> usize {
    session
        .fixture
        .as_ref()
        .map_or(0, |fixture| fixture.requests.load(Ordering::SeqCst))
}

/// Builds the actual shared lifecycle with fixed invented store and service cases.
///
/// # Errors
///
/// Returns safe local setup failures. This never opens native credential storage.
pub fn session(scenario: Scenario) -> Result<Session, SessionError> {
    let backend = mock::Store::new()
        .map_err(AuthError::storage)
        .map_err(SessionError::storage)?;
    let store = Arc::new(
        SessionStore::from_identity(
            backend.as_ref(),
            None,
            "beyond-gate-synthetic",
            "session-fixture",
        )
        .map_err(SessionError::storage)?,
    );
    if !matches!(scenario, Scenario::Absent) {
        let cookie = BrowserSessionCookie::new(b"invented-session".to_vec())
            .map_err(SessionError::service)?;
        let mut secret = cookie.encode();
        if matches!(scenario, Scenario::Invalid) {
            secret.0[6] ^= 1;
        }
        if matches!(scenario, Scenario::UnknownProfile) {
            secret.0[5] = 2;
        }
        store.save(&secret).map_err(SessionError::storage)?;
    }
    if let Some(error) = match scenario {
        Scenario::Locked => Some(keyring_core::Error::NoStorageAccess(Box::new(
            std::io::Error::other("invented-private-cause"),
        ))),
        Scenario::Unavailable => Some(keyring_core::Error::NoDefaultStore),
        Scenario::Ambiguous => Some(keyring_core::Error::Ambiguous(vec![])),
        _ => None,
    } {
        let credential = store
            .entry
            .as_any()
            .downcast_ref::<mock::Cred>()
            .ok_or_else(|| SessionError::new(SessionErrorKind::StorageFailure))?;
        credential.set_error(error);
    }
    let now = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| SessionError::new(SessionErrorKind::ValidationUnavailable))?
        .as_secs();
    let body = if matches!(scenario, Scenario::Malformed) {
        b"{invalid invented-private-body".to_vec()
    } else {
        response(if matches!(scenario, Scenario::Expired) {
            now
        } else {
            now + 3600
        })
    };
    let status = match scenario {
        Scenario::Rejected => 401,
        Scenario::RateLimited => 429,
        _ => 200,
    };
    let (fixture, url) = start(status, body, matches!(scenario, Scenario::Waiting))?;
    let client = direct_client_builder()
        .connect_timeout(Duration::from_secs(1))
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|_| SessionError::new(SessionErrorKind::ValidationUnavailable))?;
    Ok(Session {
        state: Arc::new(tokio::sync::Mutex::new(State::default())),
        backend: Backend::Synthetic(store),
        binding: None,
        exchange: Some((client, url)),
        fixture: Some(Arc::new(fixture)),
        library_fixture: None,
        character_campaign_fixture: None,
        class_catalog_fixture: None,
        monster_fixture: None,
    })
}
