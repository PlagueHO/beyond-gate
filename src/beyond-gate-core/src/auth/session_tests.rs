//! Portable lifecycle conformance with synthetic stores and actual loopback exchanges.

use super::fixture::{Scenario, request_count};
use super::*;
use keyring_core::{api::CredentialStoreApi, mock};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
};

/// Runs one actual JSON read with a separately controlled synthetic resource server.
fn read_service(status: u16) -> (HttpClient, String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("local read service");
    let origin = format!("http://{}", listener.local_addr().expect("address"));
    let client = HttpClient::for_test(&origin).expect("bounded loopback client");
    let url = format!("{origin}/invented-resource");
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("one read");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read bound");
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).expect("request");
            bytes.push(byte[0]);
            assert!(bytes.len() < 16 * 1024);
        }
        stream.write_all(format!("HTTP/1.1 {status} Mock\r\nContent-Length: 14\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{{\"read\":true}}\n").as_bytes()).expect("response");
        String::from_utf8(bytes).expect("synthetic headers")
    });
    (client, url, worker)
}

/// Selects the owner's synthetic entry, never the platform credential service.
fn store(session: &Session) -> Arc<SessionStore> {
    match &session.backend {
        Backend::Synthetic(store) => Arc::clone(store),
        Backend::Native => panic!("tests must never open native storage"),
    }
}

/// Makes one reusable acquisition through the same internal path used by JSON reads.
async fn acquire(session: &Session) -> Result<(), SessionError> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut state = session.lock(deadline).await?;
    session.prepare(&mut state, deadline, false).await
}

/// Keeps one book operation tied to its first cookie even when another read changes the cache.
#[tokio::test]
async fn book_scopes_pin_identity_without_losing_process_local_reuse() {
    let session = fixture::session(Scenario::Valid).expect("fixture");
    let scoped = session.read_scope();
    assert_eq!(request_count(&session), 0);
    acquire(&scoped).await.expect("initial scoped evidence");
    acquire(&session.read_scope())
        .await
        .expect("separate scope shares bearer");
    assert_eq!(request_count(&session), 1);
    store(&session)
        .save(
            &BrowserSessionCookie::new(b"invented-other-account".to_vec())
                .expect("cookie")
                .encode(),
        )
        .expect("changed protected entry");
    acquire(&session)
        .await
        .expect("another operation changes the cached identity");
    assert_eq!(request_count(&session), 2);
    assert_eq!(
        acquire(&scoped)
            .await
            .expect_err("old metadata cannot cross accounts")
            .kind(),
        SessionErrorKind::Changed
    );
    assert_eq!(request_count(&session), 2);
    assert!(!format!("{scoped:?}").contains("other-account"));
    session.logout().await.expect("local removal");
    assert_eq!(
        acquire(&scoped).await.expect_err("actual absence").kind(),
        SessionErrorKind::Absent
    );
}

/// Keeps monster cookie reads narrow and prevents collection reads after a cookie change.
#[tokio::test]
async fn monster_cookie_profile_preserves_source_guards_and_session_identity() {
    use crate::monster::{MonsterQuery, fixture as monsters};
    let session = monsters::session(monsters::Scenario::Normal).expect("fixture");
    let source = session.monster_fixture.as_ref().expect("source");
    let scoped = session.read_scope();
    scoped.account_id().await.expect("bound session");
    for path in [
        "/sources/example",
        "/monsters/17",
        "/monsters?page=0",
        "/monsters?page=1&unknown=1",
    ] {
        assert_eq!(
            scoped
                .get_monster_html(&source.client, &format!("{}{path}", source.origin))
                .await
                .expect_err("narrow monster guard")
                .kind(),
            SessionErrorKind::UnsupportedContract
        );
    }
    let url = MonsterQuery::default()
        .website_url(&source.origin)
        .expect("URL");
    assert_eq!(
        scoped
            .get_monster_html(&source.client, &format!("{url}#fragment"))
            .await
            .expect_err("transport rejects fragments")
            .kind(),
        SessionErrorKind::ReadFailed
    );
    assert_eq!(
        scoped
            .get_book_html(&source.client, url.as_str())
            .await
            .expect_err("book guard unchanged")
            .kind(),
        SessionErrorKind::UnsupportedContract
    );
    assert!(monsters::routes(&session).is_empty());
    scoped
        .get_monster_html(&source.client, url.as_str())
        .await
        .expect("cookie page");
    store(&session)
        .save(
            &BrowserSessionCookie::new(b"invented-changed-monster-owner".to_vec())
                .expect("cookie")
                .encode(),
        )
        .expect("changed entry");
    let result: Result<serde_json::Value, _> = scoped
        .get_json(
            &source.client,
            &format!("{}/v1/Monster?ids=17", source.origin),
        )
        .await;
    assert_eq!(
        result.expect_err("identity changed").kind(),
        SessionErrorKind::Changed
    );
    assert_eq!(monsters::routes(&session).len(), 1);
}

/// Changes only this fixture's response for subsequent actual requests.
fn reply(session: &Session, status: u16, body: Vec<u8>) {
    *session
        .fixture
        .as_ref()
        .expect("fixture")
        .response
        .lock()
        .expect("response") = (status, body);
}

/// Injects one native-shaped failure into an isolated in-memory entry.
fn fail(session: &Session, error: keyring_core::Error) {
    store(session)
        .entry
        .as_any()
        .downcast_ref::<mock::Cred>()
        .expect("synthetic credential")
        .set_error(error);
}

/// Clears the cache after exact monotonic expiry and exchanges once, never in the background.
#[tokio::test]
async fn fresh_validation_reuse_and_expiry_have_exact_request_counts() {
    let session = fixture::session(Scenario::Valid).expect("fixture");
    let stored = store(&session).entry.get_secret().expect("cookie envelope");
    let first = session.validate().await.expect("fresh");
    assert_eq!(
        serde_json::to_value(first).expect("safe JSON"),
        serde_json::json!({
            "validation": "service_validated", "expiry": "unknown", "bearer_expiry": "future"
        })
    );
    acquire(&session).await.expect("reuse");
    acquire(&session).await.expect("reuse again");
    assert_eq!(request_count(&session), 1);
    session.validate().await.expect("explicitly fresh");
    assert_eq!(request_count(&session), 2);
    let deadline = session
        .state
        .lock()
        .await
        .bearer
        .as_ref()
        .expect("bearer")
        .deadline();
    tokio::time::pause();
    tokio::time::advance(deadline - Instant::now()).await;
    assert!(
        !session
            .state
            .lock()
            .await
            .bearer
            .as_ref()
            .expect("expired")
            .usable(SystemTime::now())
    );
    tokio::time::resume();
    acquire(&session).await.expect("one exchange after expiry");
    assert_eq!(request_count(&session), 3);
    assert_eq!(
        store(&session)
            .entry
            .get_secret()
            .expect("unchanged cookie"),
        stored
    );
}

/// Keeps absence, rejection, verified expiry, format, and store failures distinct.
#[tokio::test]
async fn failures_remain_distinct_and_redacted() {
    for (scenario, kind, requests) in [
        (Scenario::Absent, SessionErrorKind::Absent, 0),
        (Scenario::Rejected, SessionErrorKind::Rejected, 1),
        (Scenario::Expired, SessionErrorKind::Expired, 1),
        (Scenario::Invalid, SessionErrorKind::InvalidMaterial, 0),
        (
            Scenario::UnknownProfile,
            SessionErrorKind::UnsupportedContract,
            0,
        ),
        (Scenario::Locked, SessionErrorKind::StorageAccess, 0),
        (
            Scenario::Unavailable,
            SessionErrorKind::StorageUnavailable,
            0,
        ),
        (Scenario::Ambiguous, SessionErrorKind::AmbiguousEntry, 0),
        (
            Scenario::Malformed,
            SessionErrorKind::ValidationUnavailable,
            1,
        ),
        (
            Scenario::RateLimited,
            SessionErrorKind::ValidationUnavailable,
            1,
        ),
    ] {
        let session = fixture::session(scenario).expect("fixture");
        let error = session.validate().await.expect_err("failure");
        assert_eq!(error.kind(), kind);
        assert_eq!(request_count(&session), requests);
        assert!(session.state.lock().await.bearer.is_none());
        let mut cause: Option<&dyn Error> = Some(&error);
        while let Some(error) = cause {
            let text = format!("{error} {error:?}");
            for private in ["invented-session", "invented-private", "c3ludGhldGlj"] {
                assert!(!text.contains(private), "{text}");
            }
            cause = error.source();
        }
    }
}

/// Refuses the same rejected material until a different cookie is saved by human recovery.
#[tokio::test]
async fn rejection_stops_reuse_without_deleting_or_retrying_credentials() {
    let session = fixture::session(Scenario::Valid).expect("fixture");
    session.validate().await.expect("fresh");
    let prior = store(&session).entry.get_secret().expect("prior");
    reply(&session, 401, b"{}".to_vec());
    assert_eq!(
        session.validate().await.expect_err("rejected").kind(),
        SessionErrorKind::Rejected
    );
    assert_eq!(
        acquire(&session).await.expect_err("latched").kind(),
        SessionErrorKind::Rejected
    );
    assert_eq!(
        session
            .validate()
            .await
            .expect_err("latched explicit")
            .kind(),
        SessionErrorKind::Rejected
    );
    assert_eq!(request_count(&session), 2);
    assert_eq!(
        store(&session).entry.get_secret().expect("preserved"),
        prior
    );
    store(&session)
        .save(
            &BrowserSessionCookie::new(b"invented-replacement".to_vec())
                .expect("cookie")
                .encode(),
        )
        .expect("new human material");
    reply(&session, 200, fixture::response(u32::MAX.into()));
    acquire(&session).await.expect("recovered");
    assert_eq!(request_count(&session), 3);
}

/// An unavailable refresh cannot resurrect earlier cached evidence.
#[tokio::test]
async fn fresh_failure_and_unreadable_store_remove_prior_evidence() {
    let session = fixture::session(Scenario::Valid).expect("fixture");
    session.validate().await.expect("fresh");
    reply(&session, 500, b"{}".to_vec());
    assert_eq!(
        session.validate().await.expect_err("service down").kind(),
        SessionErrorKind::ValidationUnavailable
    );
    assert!(session.state.lock().await.bearer.is_none());
    reply(&session, 200, fixture::response(u32::MAX.into()));
    session.validate().await.expect("fresh again");
    fail(&session, keyring_core::Error::NoDefaultStore);
    assert_eq!(
        acquire(&session).await.expect_err("unreadable").kind(),
        SessionErrorKind::StorageUnavailable
    );
    assert!(session.state.lock().await.bearer.is_none());
}

/// Detects external logout and replacement and clears evidence even after failed local deletion.
#[tokio::test]
async fn logout_replacement_and_failed_deletion_do_not_keep_a_bearer() {
    let session = fixture::session(Scenario::Valid).expect("fixture");
    acquire(&session).await.expect("fresh");
    store(&session).logout().expect("another process deletes");
    assert_eq!(
        acquire(&session).await.expect_err("absent").kind(),
        SessionErrorKind::Absent
    );
    assert!(session.state.lock().await.bearer.is_none());
    assert_eq!(request_count(&session), 1);
    store(&session)
        .save(
            &BrowserSessionCookie::new(b"new-invented-cookie".to_vec())
                .expect("cookie")
                .encode(),
        )
        .expect("replacement");
    acquire(&session).await.expect("fresh replacement");
    assert_eq!(request_count(&session), 2);
    fail(
        &session,
        keyring_core::Error::NoStorageAccess(Box::new(std::io::Error::other(
            "invented-private-denial",
        ))),
    );
    assert_eq!(
        session.logout().await.expect_err("delete denied").kind(),
        SessionErrorKind::StorageAccess
    );
    assert!(session.state.lock().await.bearer.is_none());
    assert_eq!(
        store(&session).status().expect("entry unchanged"),
        super::super::SessionStatus::StoredUnverified
    );
    session.logout().await.expect("delete");
    session.logout().await.expect("absent is success");
}

/// Shares one exchange between concurrent reusable acquisitions and bounds queued waiting.
#[tokio::test]
async fn concurrent_use_and_cancelled_queue_do_not_duplicate_exchange() {
    let session = fixture::session(Scenario::Valid).expect("fixture");
    let (first, second, third) =
        tokio::join!(acquire(&session), acquire(&session), acquire(&session));
    first.expect("first");
    second.expect("second");
    third.expect("third");
    assert_eq!(request_count(&session), 1);
    let held = session.state.lock().await;
    let waiting = session.lock(Instant::now());
    assert_eq!(
        waiting.await.expect_err("queue deadline").kind(),
        SessionErrorKind::TimedOut
    );
    drop(held);
    assert_eq!(request_count(&session), 1);
}

/// Expired or malformed service evidence cannot produce a bearer; exact codec caps hold.
#[tokio::test]
async fn portable_service_contract_rejects_expiry_and_malformed_material() {
    use super::super::service::{MAX_COOKIE_BYTES, MAX_STORED_BYTES, validate_response};
    let cookie = BrowserSessionCookie::new(vec![b'x'; MAX_COOKIE_BYTES]).expect("maximum");
    assert_eq!(cookie.encode().0.len(), MAX_STORED_BYTES);
    for bad in [
        vec![],
        vec![b'x'; MAX_COOKIE_BYTES + 1],
        b"x;y".to_vec(),
        vec![0xff],
    ] {
        assert_eq!(
            BrowserSessionCookie::new(bad).expect_err("invalid").code(),
            ErrorCode::InvalidMaterial
        );
    }
    let now = std::time::UNIX_EPOCH + Duration::from_secs(100);
    assert_eq!(
        validate_response(&fixture::response(100), now)
            .expect_err("exact expiry")
            .code(),
        ErrorCode::SessionExpired
    );
    let bearer = validate_response(&fixture::response(101), now).expect("future");
    assert!(!bearer.usable(now + Duration::from_secs(1)));
    for body in [b"not-json".as_slice(), br#"{"token":"invented-session"}"#] {
        assert_eq!(
            validate_response(body, now).expect_err("malformed").code(),
            ErrorCode::ValidationUnavailable
        );
    }
}

/// Non-Windows validation and discovery stop before credential or public service access.
#[cfg(not(windows))]
#[tokio::test]
async fn production_platform_guard_precedes_native_access() {
    assert_eq!(
        Session::default()
            .validate()
            .await
            .expect_err("unsupported")
            .kind(),
        SessionErrorKind::UnsupportedPlatform
    );
    let error = crate::library::discover_library(&Session::default())
        .await
        .expect_err("unsupported discovery");
    assert!(matches!(
        error,
        crate::library::LibraryError::SessionRead(error)
            if error.kind() == SessionErrorKind::UnsupportedPlatform
    ));
}

/// Applies bearer-only headers to bounded reads, never retries denial, and distinguishes 403.
#[tokio::test]
async fn authenticated_reads_share_evidence_and_preserve_denial_semantics() {
    let session = fixture::session(Scenario::Valid).expect("fixture");
    session.validate().await.expect("fresh");
    let (client, url, worker) = read_service(200);
    let data: Value = session.get_json(&client, &url).await.expect("read");
    assert_eq!(data, json!({"read":true}));
    let header = worker.join().expect("resource worker").to_ascii_lowercase();
    assert!(header.starts_with("get /invented-resource "));
    assert!(header.contains("authorization: bearer e30."));
    assert!(!header.contains("cookie:"));
    assert!(!header.contains("invented-session"));
    assert_eq!(request_count(&session), 1);
    let (client, url, worker) = read_service(403);
    let error = session
        .get_json::<Value>(&client, &url)
        .await
        .expect_err("content denied");
    worker.join().expect("one denied read, no retry");
    assert_eq!(error.kind(), SessionErrorKind::AccessDenied);
    let cause = error
        .source()
        .expect("HTTP cause")
        .downcast_ref::<HttpError>()
        .expect("typed HTTP");
    assert_eq!(cause.kind(), ErrorKind::AccessDenied);
    let (client, url, worker) = read_service(404);
    assert_eq!(
        session
            .get_json::<Value>(&client, &url)
            .await
            .expect_err("missing record")
            .kind(),
        SessionErrorKind::NotFound
    );
    worker.join().expect("one missing read, no retry");
    let (client, url, worker) = read_service(401);
    assert_eq!(
        session
            .get_json::<Value>(&client, &url)
            .await
            .expect_err("rejected")
            .kind(),
        SessionErrorKind::Rejected
    );
    worker.join().expect("one rejected read");
    assert_eq!(
        acquire(&session).await.expect_err("requires login").kind(),
        SessionErrorKind::Rejected
    );
    assert_eq!(request_count(&session), 1);
}

/// Rejects unapproved resource origins before loading or exchanging a session.
#[tokio::test]
async fn resource_origin_policy_precedes_credential_access() {
    let session = fixture::session(Scenario::Locked).expect("fixture");
    let client = HttpClient::new(&[]).expect("no allowed origins");
    let error = session
        .get_json::<Value>(&client, "https://example.com/resource")
        .await
        .expect_err("origin");
    assert_eq!(
        error
            .source()
            .expect("cause")
            .downcast_ref::<HttpError>()
            .expect("typed")
            .kind(),
        ErrorKind::UnapprovedOrigin
    );
    assert_eq!(request_count(&session), 0);
}

/// Caller drop and the whole-operation deadline cannot install a late exchange result.
#[tokio::test]
async fn cancelled_or_timed_out_exchange_never_installs_late_evidence() {
    for timed_out in [false, true] {
        let mut session = fixture::session(Scenario::Valid).expect("fixture");
        let listener = TcpListener::bind("127.0.0.1:0").expect("stall service");
        let url = format!(
            "http://{}/v1/cobalt-token",
            listener.local_addr().expect("address")
        );
        let client = crate::http::direct_client_builder()
            .build()
            .expect("client");
        session.exchange = Some((client, url));
        let (entered, receive) = tokio::sync::oneshot::channel();
        let (release, wait) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("one exchange");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("bound");
            let mut bytes = Vec::new();
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).expect("request");
                bytes.push(byte[0]);
            }
            entered.send(()).expect("started");
            wait.recv_timeout(Duration::from_secs(2))
                .expect("test release");
            let body = fixture::response(u32::MAX.into());
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            // Cancellation can close the connection before the synthetic reply.
            let _ = stream
                .write_all(head.as_bytes())
                .and_then(|()| stream.write_all(&body));
        });
        let mut validation = Box::pin(session.validate());
        tokio::select! {
            result = &mut validation => panic!("exchange completed before release: {result:?}"),
            result = receive => result.expect("exchange started"),
        }
        if timed_out {
            tokio::time::pause();
            tokio::time::advance(Duration::from_secs(31)).await;
            assert_eq!(
                validation.await.expect_err("deadline").kind(),
                SessionErrorKind::TimedOut
            );
            tokio::time::resume();
        } else {
            drop(validation);
        }
        release.send(()).expect("release");
        worker.join().expect("worker");
        assert!(session.state.lock().await.bearer.is_none());
        assert_eq!(
            store(&session).status().expect("cookie remains"),
            super::super::SessionStatus::StoredUnverified
        );
    }
}

/// A rejected owner does not forget human recovery after a transient store failure.
#[tokio::test]
async fn rejected_material_stays_rejected_across_storage_failure() {
    let session = fixture::session(Scenario::Rejected).expect("fixture");
    session.validate().await.expect_err("rejected");
    fail(&session, keyring_core::Error::NoDefaultStore);
    assert_eq!(
        acquire(&session)
            .await
            .expect_err("store unavailable")
            .kind(),
        SessionErrorKind::StorageUnavailable
    );
    assert_eq!(
        acquire(&session).await.expect_err("still rejected").kind(),
        SessionErrorKind::Rejected
    );
    assert_eq!(request_count(&session), 1);
}

/// Wraps only a synthetic entry to delay its first protected read off the executor.
struct DelayedRead {
    /// Delegates all bytes and identity handling to the mock store.
    inner: keyring_core::Entry,
    /// Signals that the blocking read has started.
    entered: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    /// Bounds the invented storage wait until the test releases it.
    release: std::sync::Mutex<Option<mpsc::Receiver<()>>>,
}

impl keyring_core::api::CredentialApi for DelayedRead {
    /// Saves only in the synthetic underlying entry.
    fn set_secret(&self, bytes: &[u8]) -> keyring_core::Result<()> {
        self.inner.set_secret(bytes)
    }

    /// Delays only the first load; the worker never installs bearer evidence.
    fn get_secret(&self) -> keyring_core::Result<Vec<u8>> {
        if let Some(entered) = self.entered.lock().expect("signal lock").take() {
            entered.send(()).expect("entered");
            self.release
                .lock()
                .expect("release lock")
                .take()
                .expect("one wait")
                .recv_timeout(Duration::from_secs(2))
                .expect("release read");
        }
        self.inner.get_secret()
    }

    /// Deletes only the synthetic entry without loading it.
    fn delete_credential(&self) -> keyring_core::Result<()> {
        self.inner.delete_credential()
    }

    /// Keeps the same concrete wrapper instead of constructing another entry.
    fn get_credential(&self) -> keyring_core::Result<Option<Arc<keyring_core::api::Credential>>> {
        Ok(None)
    }

    /// Returns only the invented fixed identity.
    fn get_specifiers(&self) -> Option<(String, String)> {
        self.inner.get_specifiers()
    }

    /// Implements the backend's concrete-type inspection requirement.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Enforces the five-second read wait and prevents late validation after caller drop.
#[tokio::test]
async fn blocking_storage_timeout_and_cancellation_cannot_start_late_exchange() {
    for timeout in [false, true] {
        let mut session = fixture::session(Scenario::Valid).expect("fixture");
        let old = store(&session);
        let backend = mock::Store::new().expect("synthetic backend");
        let entry = backend
            .build("invented-delayed", "local", None)
            .expect("mock entry");
        entry
            .set_secret(&old.entry.get_secret().expect("synthetic cookie"))
            .expect("seed");
        let (entered, receive) = tokio::sync::oneshot::channel();
        let (release, wait) = mpsc::channel();
        session.backend = Backend::Synthetic(Arc::new(SessionStore {
            entry: keyring_core::Entry::new_with_credential(Arc::new(DelayedRead {
                inner: entry,
                entered: std::sync::Mutex::new(Some(entered)),
                release: std::sync::Mutex::new(Some(wait)),
            })),
        }));
        let mut validation = Box::pin(session.validate());
        tokio::select! {
            result = &mut validation => panic!("read finished before release: {result:?}"),
            result = receive => result.expect("native-shaped read started"),
        }
        if timeout {
            tokio::time::pause();
            tokio::time::advance(Duration::from_millis(5_001)).await;
            assert_eq!(
                validation.await.expect_err("storage deadline").kind(),
                SessionErrorKind::TimedOut
            );
            tokio::time::resume();
        } else {
            drop(validation);
        }
        release.send(()).expect("release");
        assert_eq!(request_count(&session), 0);
        assert!(session.state.lock().await.bearer.is_none());
        acquire(&session)
            .await
            .expect("next operation can validate");
        assert_eq!(request_count(&session), 1);
    }
}
