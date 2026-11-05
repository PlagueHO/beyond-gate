//! Compiled invented library services for core and actual adapter process checks.
//!
//! Product commands cannot select these fixtures or supply an origin or credential.

use crate::{
    auth::session::{Session, fixture as auth},
    http::HttpClient,
};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// Selects original metadata, account evidence, and explicit failure cases.
#[derive(Clone, Copy)]
pub enum Scenario {
    /// Purchase, sharing, both, catalog-only, and empty ownership in one collection.
    Normal,
    /// A subsequent observation retains purchase but no longer establishes sharing.
    SharingLost,
    /// A complete empty library, not an error-to-empty fallback.
    Empty,
    /// Authentication fails before either resource read.
    Absent,
    /// Authentication rejects the invented credential before metadata.
    Rejected,
    /// Authentication returns expired evidence before metadata.
    Expired,
    /// Authentication waits for cancellation after local public classification.
    Waiting,
    /// Metadata is malformed and must prevent the ownership request.
    Malformed,
    /// Metadata lacks complete count equality.
    Incomplete,
    /// Metadata exceeds the separate raw catalog cap before deduplication.
    Oversized,
    /// Ownership is denied after successful metadata.
    Denied,
    /// Ownership rate-limits the request and must not be retried.
    RateLimited,
    /// Public classification is invalid before any session use.
    InvalidConfiguration,
    /// Whitespace cleanup cannot repair a non-whitespace control character.
    InvalidTitle,
    /// Hidden-source evidence still rejects unknown ownership classifications.
    InvalidHiddenEvidence,
    /// Ownership names a source absent from validated public configuration.
    UnmatchedOwnership,
    /// Valid configured account evidence outside the Web catalog creates no book.
    OutsideWebCatalog,
    /// Account evidence outside the Web catalog must still have a supported type.
    InvalidOutsideEvidence,
}

/// Holds private fixed loopback routes and their owned bounded worker.
pub(crate) struct Source {
    /// Uses the real one-attempt transport.
    pub(crate) client: HttpClient,
    /// Selects only the invented public configuration.
    pub(crate) config: String,
    /// Selects only the invented catalog.
    pub(crate) sources: String,
    /// Selects only the invented account observations.
    pub(crate) ownership: String,
    /// Signals only the owned worker.
    stop: Arc<AtomicBool>,
    /// Joins only the owned worker.
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Source {
    /// Stops this listener after bounded socket work and reports unexpected worker failure.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            eprintln!("Synthetic library service worker failed.");
        }
    }
}

/// Builds fixed original catalog and ownership bodies; no account snapshots are used.
fn replies(scenario: Scenario) -> (String, u16, String) {
    let record = json!({"sourceId":1,"name":"Invented volume 1","slug":"invented-1"});
    let mut items = match scenario {
        Scenario::Empty => vec![],
        Scenario::Oversized => vec![record; super::discovery::MAX_CATALOG_RECORDS + 1],
        _ => (1..=5).map(|id| json!({"sourceId":id,"name":format!("Invented volume {id}"),"slug":format!("invented-{id}")})).collect(),
    };
    if !matches!(scenario, Scenario::Empty | Scenario::Oversized) {
        items[0]["name"] = json!(if matches!(scenario, Scenario::InvalidTitle) {
            "Invented\u{1b} volume 1"
        } else {
            " \tInvented\n volume 1 "
        });
        items.push(json!({"sourceId":6,"name":"","slug":null}));
    }
    let total = items.len() + usize::from(matches!(scenario, Scenario::Incomplete));
    let catalog = if matches!(scenario, Scenario::Malformed) {
        "{invalid-invented-private-body".to_owned()
    } else {
        json!({"items":items,"totalCount":total}).to_string()
    };
    let mut evidence = match scenario {
        Scenario::Empty => vec![],
        Scenario::SharingLost => vec![json!({"sourceId":1,"ownership":[{"type":"owned"}]})],
        _ => vec![
            json!({"sourceId":1,"ownership":[{"type":"owned"}]}),
            json!({"sourceId":2,"ownership":[{"type":"shared"}]}),
            json!({"sourceId":3,"ownership":[{"type":"owned"},{"type":"shared"}]}),
            json!({"sourceId":5,"ownership":[]}),
        ],
    };
    if !matches!(scenario, Scenario::Empty) {
        evidence.push(json!({"sourceId":6,"ownership":[{"type":
            if matches!(scenario, Scenario::InvalidHiddenEvidence) {"unexpected"} else {"owned"}
        }]}));
    }
    if matches!(scenario, Scenario::UnmatchedOwnership) {
        evidence.push(json!({"sourceId":9,"ownership":[{"type":"owned"}]}));
    }
    if matches!(
        scenario,
        Scenario::OutsideWebCatalog | Scenario::InvalidOutsideEvidence
    ) {
        evidence.push(json!({"sourceId":7,"ownership":[{"type":
            if matches!(scenario, Scenario::InvalidOutsideEvidence) {"unexpected"} else {"owned"}
        },{"type":"shared"}]}));
        evidence.push(json!({"sourceId":8,"ownership":[{"type":"owned"}]}));
    }
    let status = match scenario {
        Scenario::Denied => 403,
        Scenario::RateLimited => 429,
        _ => 200,
    };
    (
        catalog,
        status,
        json!({"items":evidence,"totalCount":evidence.len()}).to_string(),
    )
}

impl Source {
    /// Starts only a bounded loopback metadata service with fixed synthetic replies.
    fn start(scenario: Scenario) -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let origin = format!("http://{}", listener.local_addr()?);
        listener.set_nonblocking(true)?;
        let client = HttpClient::for_fixture(&origin).map_err(std::io::Error::other)?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let (catalog, status, ownership) = replies(scenario);
        let config = json!({
            "sources": (1..=8).map(|id| json!({"id":id,"sourceCategoryId":if matches!(id, 6 | 8) {41} else {73}})).collect::<Vec<_>>(),
            "sourceCategories": if matches!(scenario, Scenario::InvalidConfiguration) {
                vec![]
            } else {
                vec![json!({"id":41,"name":"Hidden Sources"}),json!({"id":73,"name":"Invented books"})]
            }
        }).to_string();
        let worker = std::thread::spawn(move || {
            while !flag.load(Ordering::SeqCst) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("Synthetic listener failed: {error}"),
                };
                stream
                    .set_nonblocking(false)
                    .expect("bounded blocking stream");
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .expect("read bound");
                stream
                    .set_write_timeout(Some(Duration::from_secs(1)))
                    .expect("write bound");
                let mut header = Vec::new();
                while !header.ends_with(b"\r\n\r\n") && header.len() < 16 * 1024 {
                    let mut byte = [0];
                    stream
                        .read_exact(&mut byte)
                        .expect("bounded synthetic request");
                    header.push(byte[0]);
                }
                let header = String::from_utf8(header)
                    .expect("synthetic header")
                    .to_ascii_lowercase();
                assert!(!header.contains("\r\ncookie:"));
                let public = header.starts_with("get /api/config/json ");
                assert_eq!(header.contains("\r\nauthorization:"), !public);
                let (status, body) = if public {
                    (200, &config)
                } else if header.starts_with("get /library/sources?platform=web ") {
                    (200, &catalog)
                } else {
                    assert!(header.starts_with("get /library/sources/me "));
                    (status, &ownership)
                };
                let response = format!(
                    "HTTP/1.1 {status} Synthetic\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                if let Err(error) = stream.write_all(response.as_bytes()) {
                    assert!(matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::BrokenPipe
                    ));
                }
            }
        });
        Ok(Self {
            client,
            config: format!("{origin}/api/config/json"),
            sources: format!("{origin}/library/sources?platform=Web"),
            ownership: format!("{origin}/library/sources/me"),
            stop,
            worker: Some(worker),
        })
    }
}

/// Supplies the real shared lifecycle and HTTP discovery with fixed invented services.
///
/// # Errors
///
/// Preserves local socket setup or synthetic session setup errors. Native storage
/// and live services are never accessed, including in failure scenarios.
pub fn session(scenario: Scenario) -> std::io::Result<Session> {
    let mut session = auth::session(match scenario {
        Scenario::Absent => auth::Scenario::Absent,
        Scenario::Rejected => auth::Scenario::Rejected,
        Scenario::Expired => auth::Scenario::Expired,
        Scenario::Waiting => auth::Scenario::Waiting,
        _ => auth::Scenario::Valid,
    })
    .map_err(std::io::Error::other)?;
    session.library_fixture = Some(Source::start(scenario)?);
    Ok(session)
}
