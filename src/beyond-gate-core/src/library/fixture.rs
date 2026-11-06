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
        atomic::{AtomicBool, AtomicUsize, Ordering},
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
    /// Source pages redirect without a followed request.
    BookRedirect,
    /// The selected chapter rejects the cookie after metadata and outline succeed.
    BookRejected,
    /// The selected chapter denies the request without proving an ownership state.
    BookDenied,
    /// A preview has no supported article container.
    BookPreview,
    /// Fresh selected content differs from an earlier fixture's revision.
    BookChanged,
    /// Fresh navigation moves the selected heading to a different chapter URL.
    BookMoved,
    /// The article contains repeated target IDs.
    BookMalformed,
    /// The source response declares more than the HTML body cap.
    BookOversized,
    /// The selected chapter waits long enough to exercise cancellation.
    BookWaiting,
    /// The HTML media type is wrong.
    BookWrongMedia,
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
    /// Counts received resource headers, including source pages but excluding auth exchange.
    requests: Arc<AtomicUsize>,
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
        let requests = Arc::new(AtomicUsize::new(0));
        let received = Arc::clone(&requests);
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
                received.fetch_add(1, Ordering::SeqCst);
                let public = header.starts_with("get /api/config/json ");
                let page = header.starts_with("get /sources/");
                assert_eq!(header.contains("\r\ncookie:"), page);
                assert_eq!(header.contains("\r\nauthorization:"), !public && !page);
                let page_reply;
                let (status, body) = if page {
                    let path = header
                        .split_whitespace()
                        .nth(1)
                        .expect("fixed source route");
                    if matches!(scenario, Scenario::BookWaiting) && path.ends_with("/opening") {
                        std::thread::sleep(Duration::from_millis(250));
                    }
                    page_reply = book_reply(scenario, path);
                    (page_reply.0, &page_reply.1)
                } else if public {
                    (200, &config)
                } else if header.starts_with("get /library/sources?platform=web ") {
                    (200, &catalog)
                } else {
                    assert!(header.starts_with("get /library/sources/me "));
                    (status, &ownership)
                };
                let media = if page && !matches!(scenario, Scenario::BookWrongMedia) {
                    "text/html; charset=utf-8"
                } else {
                    "application/json"
                };
                let response = format!(
                    "HTTP/1.1 {status} Synthetic\r\nContent-Type: {media}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
            requests,
        })
    }
}

/// Produces original source markup; it contains no account snapshot or proprietary text.
fn book_reply(scenario: Scenario, path: &str) -> (u16, String) {
    let chapter = path.ends_with("/opening") || path.ends_with("/revised-opening");
    let status = match scenario {
        Scenario::BookRedirect => 307,
        Scenario::BookRejected if chapter => 401,
        Scenario::BookDenied if chapter => 403,
        _ => 200,
    };
    if matches!(scenario, Scenario::BookOversized) {
        return (status, "x".repeat(1024 * 1024 + 1));
    }
    let content = if chapter {
        if matches!(scenario, Scenario::BookPreview) {
            "<p>An invented preview, not a supported article.</p>".into()
        } else {
            let text = if matches!(scenario, Scenario::BookChanged) {
                "A changed invented lantern glows. "
            } else {
                "An invented lantern glows \u{754c}. "
            }
            .repeat(240);
            let second = if matches!(scenario, Scenario::BookMalformed) {
                "first"
            } else {
                "second"
            };
            format!(
                "<article class=\"p-article-content\"><h1>An invented chapter</h1>\
                <h2 id=\"first\">An invented opening</h2><p>{text}</p>\
                <aside><h3>An invented sidebar</h3><p>A useful invented note.</p></aside>\
                <h2 id=\"{second}\">A second topic</h2><p>Adjacent topic must not leak.</p></article>"
            )
        }
    } else {
        let slug = path.rsplit('/').next().unwrap_or_default();
        let chapter = if matches!(scenario, Scenario::BookMoved) {
            "revised-opening"
        } else {
            "opening"
        };
        format!(
            "<div class=\"compendium-toc-full-text\">\
            <a href=\"/sources/{slug}/{chapter}\">An invented chapter</a>\
            <a href=\"/sources/{slug}/{chapter}#first\">An invented opening</a>\
            <a href=\"/sources/{slug}/{chapter}#second\">A second topic</a></div>"
        )
    };
    (
        status,
        format!(
            "<!doctype html><html><head><link rel=\"canonical\" \
        href=\"https://www.dndbeyond.com{path}\"></head><body>{content}</body></html>"
        ),
    )
}

/// Returns resource requests observed by the compiled local service, not a live counter.
pub fn request_count(session: &Session) -> usize {
    session
        .library_fixture
        .as_ref()
        .map_or(0, |source| source.requests.load(Ordering::SeqCst))
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
    session.library_fixture = Some(Arc::new(Source::start(scenario)?));
    Ok(session)
}
