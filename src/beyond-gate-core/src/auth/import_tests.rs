//! Portable protected-import checks with invented cookies and local services.

use super::*;
use crate::auth::session::fixture;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

/// Seeds an isolated mock entry that cannot contact a native credential service.
fn mock_store() -> Arc<SessionStore> {
    let backend = keyring_core::mock::Store::new().expect("mock");
    let store = SessionStore::from_store(backend.as_ref(), None).expect("entry");
    store
        .save(&SessionSecret::new(b"prior".to_vec()).expect("prior"))
        .expect("seed");
    Arc::new(store)
}

/// Supplies one bounded exchange response without any production host access.
fn service(status: u16) -> (Client, String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback");
    let url = format!(
        "http://{}/v1/cobalt-token",
        listener.local_addr().expect("address")
    );
    let expiry = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs()
        + 3600;
    let body = fixture::response(expiry);
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout");
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).expect("header");
            bytes.push(byte[0]);
            assert!(bytes.len() < 16_384);
        }
        write!(stream, "HTTP/1.1 {status} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).expect("response");
        stream.write_all(&body).expect("body");
    });
    (
        direct_client_builder().build().expect("client"),
        url,
        worker,
    )
}

/// Successful import stores only the versioned cookie and opens storage once.
#[tokio::test]
async fn explicit_import_validates_before_one_cookie_only_save() {
    let store = mock_store();
    let target = store.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let (client, url, worker) = service(200);
    let saved = establish_with(
        SessionEstablishmentInput::from_prompt(b"invented-import-cookie".to_vec()).expect("input"),
        LoginControl::for_import(Instant::now() + Duration::from_secs(10)).expect("control"),
        client,
        &url,
        move || {
            counted.fetch_add(1, Ordering::SeqCst);
            Ok(target)
        },
    )
    .await
    .expect("saved");
    worker.join().expect("service");
    assert_eq!(saved.validation_evidence(), "service_validated");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let cookie = BrowserSessionCookie::from_persisted(store.load().expect("load").expect("entry"))
        .expect("cookie envelope");
    assert_eq!(&*cookie.0, b"invented-import-cookie");
}

/// Rejection and pre-write cancellation cannot open or change saved input.
#[tokio::test]
async fn rejected_or_cancelled_import_never_opens_storage() {
    let (client, url, worker) = service(401);
    let error = establish_with(
        SessionEstablishmentInput::from_prompt(b"invented-rejected".to_vec()).expect("input"),
        LoginControl::for_import(Instant::now() + Duration::from_secs(10)).expect("control"),
        client,
        &url,
        || panic!("rejected input must not open storage"),
    )
    .await
    .expect_err("rejected");
    assert_eq!(error.code(), ErrorCode::SessionRejected);
    worker.join().expect("service");

    let control =
        LoginControl::for_import(Instant::now() + Duration::from_secs(10)).expect("control");
    control.cancellation().cancel();
    let error = establish_with(
        SessionEstablishmentInput::from_prompt(b"invented-cancelled".to_vec()).expect("input"),
        control,
        direct_client_builder().build().expect("client"),
        "http://127.0.0.1:1",
        || panic!("cancelled input must not open storage"),
    )
    .await
    .expect_err("cancelled");
    assert_eq!(error.code(), ErrorCode::Cancelled);
}

/// Portable import uses the same uncertain-write state transition as browser login.
#[tokio::test]
async fn import_storage_failures_and_started_writes_keep_distinct_outcomes() {
    for (failure, code) in [
        (AuthErrorKind::StorageAccess, ErrorCode::StorageAccess),
        (
            AuthErrorKind::StorageUnavailable,
            ErrorCode::StorageUnavailable,
        ),
    ] {
        let control =
            LoginControl::for_import(Instant::now() + Duration::from_secs(10)).expect("control");
        let secret = SessionEstablishmentInput::from_prompt(b"invented".to_vec())
            .expect("input")
            .cookie
            .encode();
        let error = save(secret, &control.0, move || Err(AuthError::new(failure)))
            .await
            .expect_err("store error");
        assert_eq!(error.code(), code);
    }
    let control =
        LoginControl::for_import(Instant::now() + Duration::from_secs(10)).expect("control");
    control.0.stage.store(WRITING, Ordering::SeqCst);
    assert_eq!(
        control.cancellation().cancel_outcome(),
        ErrorCode::SaveIndeterminate
    );
}

/// Explicit import does not reinterpret absence or invalid bytes as saved input.
#[test]
fn import_environment_requires_an_explicit_valid_value() {
    assert_eq!(
        SessionEstablishmentInput::from_environment_value(None)
            .expect_err("missing")
            .code(),
        ErrorCode::CookieUnavailable
    );
    assert_eq!(
        SessionEstablishmentInput::from_environment_value(Some("".into()))
            .expect_err("empty")
            .code(),
        ErrorCode::InvalidMaterial
    );
}
