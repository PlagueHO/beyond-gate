//! Offline Windows lifecycle checks with local services and synthetic keyring entries.

use std::{
    any::Any,
    io::{Read, Write},
    net::TcpListener,
    sync::{Mutex, atomic::AtomicUsize, mpsc},
};

use keyring_core::{
    Entry,
    api::{Credential, CredentialApi, CredentialStoreApi},
    mock,
};
use serde_json::{Value, json};

use super::*;
use crate::auth::{SessionStatus, handoff::Transaction};

/// Supplies two actually paired streams with a compiled invented key.
async fn completion_peers(
    deadline: Instant,
) -> (
    crate::auth::handoff::transport::PairedConnection<tokio::io::DuplexStream>,
    crate::auth::handoff::transport::PairedConnection<tokio::io::DuplexStream>,
) {
    use crate::auth::handoff::{pairing::PendingPairing, transport::PairedConnection};
    let (host, cli) = tokio::io::duplex(65_536);
    let ticket = PendingPairing::new(Zeroizing::new(vec![42; 32]), deadline).expect("test ticket");
    let (host, cli) = tokio::join!(
        PairedConnection::connect(host, Zeroizing::new(vec![42; 32]), deadline),
        PairedConnection::accept(cli, ticket)
    );
    (host.expect("host pairing"), cli.expect("CLI pairing"))
}

/// Sends one bound invented cookie and checks real validation/save completion evidence.
#[tokio::test]
async fn paired_completion_validates_saves_and_requires_receipt() {
    use crate::auth::handoff::{
        Browser, Message, Profile, Protocol, completion::complete_with, encode_message,
        flow::PendingHandoff,
    };
    for mode in ["saved", "rejected", "lost-ack", "wrong-ack"] {
        let (client, url, server) =
            service(if mode == "rejected" { 401 } else { 200 }, positive(), "");
        let deadline = Instant::now() + Duration::from_secs(10);
        let (mut host, cli) = completion_peers(deadline).await;
        let (control, _) = login_control(Duration::from_secs(10));
        let store = store();
        let target = Arc::clone(&store);
        let pending = PendingHandoff::for_profile(
            Profile::CobaltSessionV1,
            Transaction::generate().expect("transaction"),
            deadline,
        )
        .expect("pending");
        let (result, ()) = tokio::join!(
            async {
                let (connection, receipt) = pending.receive(cli).await.expect("bound receipt");
                complete_with(connection, receipt, control, |input, control| async move {
                    establish_with(input, control, client, &url, move || Ok(target)).await
                })
                .await
            },
            async {
                let open = Message::Initiate {
                    browser: Browser::Edge,
                    profile: Profile::CobaltSessionV1,
                };
                host.write_message(
                    encode_message(Protocol::HostCli, &open).expect("open"),
                    deadline,
                )
                .await
                .expect("open");
                let transaction = match host.read_message(deadline).await.expect("authorization") {
                    Message::Authorization { transaction, .. } => transaction,
                    _ => panic!("authorization only"),
                };
                let submit = Message::Submit {
                    transaction: transaction.clone(),
                    profile: Profile::CobaltSessionV1,
                    value: SessionSecret::new(b"synthetic-session".to_vec())
                        .expect("invented cookie"),
                };
                host.write_message(
                    encode_message(Protocol::HostCli, &submit).expect("submit"),
                    deadline,
                )
                .await
                .expect("submit");
                let result = host.read_completion(deadline).await.expect("final result");
                if mode == "rejected" {
                    assert!(matches!(
                        result,
                        Message::Error {
                            code: ErrorCode::SessionRejected,
                            ..
                        }
                    ));
                } else {
                    assert!(matches!(result, Message::Result { .. }));
                    if mode == "saved" || mode == "wrong-ack" {
                        let ack = Message::Acknowledgement {
                            transaction: if mode == "wrong-ack" {
                                Transaction::generate().expect("wrong transaction")
                            } else {
                                transaction
                            },
                        };
                        host.write_message(
                            encode_message(Protocol::HostCli, &ack).expect("ack"),
                            deadline,
                        )
                        .await
                        .expect("ack");
                        if mode == "saved" {
                            assert!(matches!(
                                host.read_message(deadline)
                                    .await
                                    .expect("confirmed receipt"),
                                Message::Acknowledgement { .. }
                            ));
                        } else {
                            assert!(host.read_message(deadline).await.is_err());
                        }
                    }
                }
                drop(host);
            }
        );
        server.join().expect("one mock request");
        if mode == "saved" {
            assert_eq!(
                result.expect("confirmed save").validation_evidence(),
                "service_validated"
            );
        } else {
            assert_eq!(
                result.expect_err("safe failure").code(),
                if mode == "rejected" {
                    ErrorCode::SessionRejected
                } else {
                    ErrorCode::SaveIndeterminate
                }
            );
        }
        let bytes = store.entry.get_secret().expect("mock entry");
        if mode == "rejected" {
            assert_eq!(bytes, b"synthetic-prior");
        } else {
            assert_eq!(bytes, input().cookie.encode().0.as_slice());
        }
    }
}

/// Returns an actual paired, production-bound receipt with invented cookie bytes.
async fn completion_receipt(
    deadline: Instant,
) -> (
    crate::auth::handoff::transport::PairedConnection<tokio::io::DuplexStream>,
    crate::auth::handoff::transport::PairedConnection<tokio::io::DuplexStream>,
    crate::auth::handoff::flow::ReceivedSubmission,
    Transaction,
) {
    use crate::auth::handoff::{
        Browser, Message, Profile, Protocol, encode_message, flow::PendingHandoff,
    };
    let (mut host, cli) = completion_peers(deadline).await;
    let transaction = Transaction::generate().expect("transaction");
    let pending =
        PendingHandoff::for_profile(Profile::CobaltSessionV1, transaction.clone(), deadline)
            .expect("pending");
    let (receipt, ()) = tokio::join!(pending.receive(cli), async {
        host.write_message(
            encode_message(
                Protocol::HostCli,
                &Message::Initiate {
                    browser: Browser::Edge,
                    profile: Profile::CobaltSessionV1,
                },
            )
            .expect("open"),
            deadline,
        )
        .await
        .expect("open");
        assert!(
            matches!(host.read_message(deadline).await.expect("authorization"),
                Message::Authorization { transaction: ref received, .. } if received == &transaction)
        );
        host.write_message(
            encode_message(
                Protocol::HostCli,
                &Message::Submit {
                    transaction: transaction.clone(),
                    profile: Profile::CobaltSessionV1,
                    value: SessionSecret::new(b"synthetic-session".to_vec()).expect("invented"),
                },
            )
            .expect("submit"),
            deadline,
        )
        .await
        .expect("submit");
    });
    let (cli, receipt) = receipt.expect("receipt");
    (host, cli, receipt, transaction)
}

/// Buffered cancellation, disconnect, and early acknowledgement cannot start a request.
#[tokio::test]
async fn completion_control_before_validation_never_sends_a_request() {
    use crate::auth::handoff::{Message, Protocol, completion::complete_with, encode_message};
    for mode in ["cancel", "disconnect", "early-ack"] {
        let deadline = Instant::now() + Duration::from_secs(10);
        let (mut host, cli, receipt, transaction) = completion_receipt(deadline).await;
        let (control, _) = login_control(Duration::from_secs(10));
        if mode != "disconnect" {
            let message = if mode == "cancel" {
                Message::Cancel {
                    transaction: Some(transaction),
                }
            } else {
                Message::Acknowledgement { transaction }
            };
            host.write_message(
                encode_message(Protocol::HostCli, &message).expect("control"),
                deadline,
            )
            .await
            .expect("buffered control");
        }
        drop(host);
        let listener = TcpListener::bind("127.0.0.1:0").expect("owned listener");
        listener
            .set_nonblocking(true)
            .expect("nonblocking observation");
        let url = format!("http://{}/", listener.local_addr().expect("address"));
        let client = direct_client_builder().build().expect("client");
        let result = complete_with(cli, receipt, control, |input, control| async {
            establish_with(input, control, client, &url, || panic!("no store access")).await
        })
        .await
        .expect_err("stopped before validation");
        assert_eq!(
            result.code(),
            match mode {
                "cancel" => ErrorCode::Cancelled,
                "disconnect" => ErrorCode::Disconnected,
                _ => ErrorCode::ProtocolError,
            }
        );
        assert_eq!(
            listener.accept().expect_err("no connection").kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

/// Actual paired cancellation stops write admission or reports an already-started save.
#[tokio::test]
async fn paired_completion_cancellation_preserves_write_and_reservation_rules() {
    use crate::auth::handoff::{Message, Protocol, completion::complete_with, encode_message};
    for started_write in [false, true] {
        let deadline = Instant::now() + Duration::from_secs(10);
        let (mut host, cli, receipt, transaction) = completion_receipt(deadline).await;
        let (control, namespace) = login_control(Duration::from_secs(10));
        let (client, url, server) = service(200, positive(), "");
        let (entered, entered_rx) = tokio::sync::oneshot::channel();
        let (release, release_rx) = mpsc::channel();
        let mut setup_signal;
        let target = if started_write {
            setup_signal = None;
            let backend = mock::Store::new().expect("backend");
            let inner = backend
                .build("beyond-gate", "local-session", None)
                .expect("entry");
            inner.set_secret(b"synthetic-prior").expect("prior");
            Arc::new(SessionStore {
                entry: Entry::new_with_credential(Arc::new(BlockingCredential {
                    inner,
                    entered: Mutex::new(Some(entered)),
                    release: Mutex::new(release_rx),
                })),
            })
        } else {
            setup_signal = Some((entered, release_rx));
            store()
        };
        let retained = Arc::clone(&target);
        let mut completion = Box::pin(complete_with(
            cli,
            receipt,
            control,
            |input, control| async move {
                establish_with(input, control, client, &url, move || {
                    if let Some((entered, release)) = setup_signal.take() {
                        entered.send(()).expect("setup entered");
                        release
                            .recv_timeout(Duration::from_secs(2))
                            .expect("release setup");
                    }
                    Ok(retained)
                })
                .await
            },
        ));
        tokio::select! {
            result = &mut completion => panic!("premature completion: {result:?}"),
            result = entered_rx => result.expect("store operation entered"),
        }
        host.write_message(
            encode_message(
                Protocol::HostCli,
                &Message::Cancel {
                    transaction: Some(transaction.clone()),
                },
            )
            .expect("cancel"),
            deadline,
        )
        .await
        .expect("cancel");
        let expected = if started_write {
            ErrorCode::SaveIndeterminate
        } else {
            ErrorCode::Cancelled
        };
        let (result, reply) = tokio::join!(completion, host.read_completion(deadline));
        assert_eq!(result.expect_err("stopped completion").code(), expected);
        assert!(matches!(reply.expect("safe reply"),
            Message::Error { code, transaction: Some(received) }
                if code == expected && received == transaction));
        assert!(PendingLoginSlot::claim_namespace(Some(&namespace)).is_err());
        release.send(()).expect("release worker");
        wait_released(&namespace).await;
        let stored = target.entry.get_secret().expect("final entry");
        if started_write {
            assert_eq!(stored, input().cookie.encode().0.as_slice());
        } else {
            assert_eq!(stored, b"synthetic-prior");
        }
        server.join().expect("one request");
    }
}

/// Checks the exact five-second setup/write wait independently of the longer login deadline.
#[tokio::test(start_paused = true)]
async fn save_wait_is_five_seconds_and_cannot_admit_a_late_write() {
    let (control, namespace) = login_control(Duration::from_secs(30));
    let store = store();
    let target = Arc::clone(&store);
    let (entered, entered_rx) = tokio::sync::oneshot::channel();
    let (release, release_rx) = mpsc::channel();
    let mut future = Box::pin(save(input().cookie.encode(), &control.0, move || {
        entered.send(()).expect("setup started");
        release_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("test release");
        Ok(target)
    }));
    tokio::select! {
        result = &mut future => panic!("premature setup result: {result:?}"),
        result = entered_rx => result.expect("setup entered"),
    }
    let started = Instant::now();
    tokio::time::advance(Duration::from_secs(5)).await;
    assert_eq!(
        future.await.expect_err("fixed wait").code(),
        ErrorCode::TimedOut
    );
    assert_eq!(Instant::now() - started, Duration::from_secs(5));
    drop(control);
    release.send(()).expect("release owned worker");
    tokio::time::resume();
    wait_released(&namespace).await;
    assert_eq!(
        store.entry.get_secret().expect("prior entry"),
        b"synthetic-prior"
    );
}

/// Builds a disjoint reservation with a future deadline and no product entry access.
fn login_control(duration: Duration) -> (LoginControl, String) {
    let namespace = Transaction::generate().expect("test randomness");
    let slot = PendingLoginSlot::claim_namespace(Some(namespace.as_str())).expect("test slot");
    let control = LoginControl::new(Arc::new(slot), Instant::now() + duration).expect("control");
    (control, namespace.as_str().to_owned())
}

/// Owns a known invented RFC cookie for every offline request.
fn input() -> SessionEstablishmentInput {
    SessionEstablishmentInput::new(
        BrowserSessionCookie::new(b"synthetic-session".to_vec()).expect("cookie"),
    )
}

/// Builds an in-memory entry using the fixed product identity but no native backend.
fn store() -> Arc<SessionStore> {
    let backend = mock::Store::new().expect("mock backend");
    let store = SessionStore::from_store(backend.as_ref(), None).expect("mock entry");
    store
        .save(&SessionSecret::new(b"synthetic-prior".to_vec()).expect("prior"))
        .expect("save prior");
    Arc::new(store)
}

/// Returns a syntactically valid unsigned fixture token in the service envelope.
fn response(claims: Value) -> Vec<u8> {
    let token = format!(
        "{}.{}.{}",
        URL_SAFE_NO_PAD.encode(b"{}"),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).expect("claims")),
        URL_SAFE_NO_PAD.encode(b"synthetic-signature"),
    );
    serde_json::to_vec(&json!({ "token": token, "ttl": 3600 })).expect("response")
}

/// Provides accepted positive evidence with an expiry well inside SystemTime's range.
fn positive() -> Vec<u8> {
    response(json!({
        "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier": "42",
        "exp": SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_secs() + 3600
    }))
}

/// Checks exact codec size limits, redaction, version rejection, and malformed lengths.
#[test]
fn cookie_codec_bounds_and_versions_are_exact() {
    for value in [
        vec![],
        vec![b'x'; MAX_COOKIE_BYTES + 1],
        b"x;y".to_vec(),
        vec![0xff],
    ] {
        assert_eq!(
            BrowserSessionCookie::new(value)
                .expect_err("invalid")
                .code(),
            ErrorCode::InvalidMaterial
        );
    }
    let cookie = BrowserSessionCookie::new(vec![b'x'; MAX_COOKIE_BYTES]).expect("exact max");
    assert_eq!(cookie.encode().0.len(), MAX_STORED_BYTES);
    let decoded = BrowserSessionCookie::from_persisted(cookie.encode()).expect("round trip");
    assert_eq!(decoded.0.as_slice(), cookie.0.as_slice());
    assert!(!format!("{cookie:?}").contains(&"x".repeat(20)));
    for index in [0, 4, 5] {
        let mut encoded = cookie.encode();
        encoded.0[index] ^= 1;
        assert_eq!(
            BrowserSessionCookie::from_persisted(encoded)
                .expect_err("unknown")
                .code(),
            ErrorCode::UnsupportedContract
        );
    }
    let mut encoded = cookie.encode();
    encoded.0[6] ^= 1;
    assert_eq!(
        BrowserSessionCookie::from_persisted(encoded)
            .expect_err("length")
            .code(),
        ErrorCode::InvalidMaterial
    );
    let mut encoded = input().cookie.encode();
    encoded.0.push(b'x');
    assert_eq!(
        BrowserSessionCookie::from_persisted(encoded)
            .expect_err("trailing")
            .code(),
        ErrorCode::InvalidMaterial
    );
}

/// Requires a positive account marker and future expiry, not a token or TTL alone.
#[test]
fn service_rule_rejects_missing_zero_malformed_and_expired_claims() {
    let now = UNIX_EPOCH + Duration::from_secs(100);
    for identifier in [json!(42), json!("42")] {
        validate_response(
            &response(json!({
                "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier": identifier,
                "exp": 101
            })),
            now,
        )
        .expect("accepted evidence");
    }
    for identifier in [json!(0), json!("0"), json!("-1"), json!("x"), json!(-1)] {
        assert_eq!(validate_response(&response(json!({
            "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier": identifier,
            "exp": 101
        })), now).expect_err("rejected marker").code(), ErrorCode::SessionRejected);
    }
    for claims in [
        json!({"exp": 101}),
        json!({
            "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier": 42
        }),
    ] {
        assert_eq!(
            validate_response(&response(claims), now)
                .expect_err("missing")
                .code(),
            ErrorCode::SessionRejected
        );
    }
    for expiry in [0, 99, 100] {
        assert_eq!(
            validate_response(
                &response(json!({
                    "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier": 42,
                    "exp": expiry
                })),
                now
            )
            .expect_err("expired bearer")
            .code(),
            ErrorCode::SessionExpired
        );
    }
    for bytes in [
        b"{}".as_slice(),
        br#"{"token":null,"ttl":3600}"#,
        br#"{"token":"","ttl":3600}"#,
    ] {
        assert_eq!(
            validate_response(bytes, now).expect_err("no token").code(),
            ErrorCode::SessionRejected
        );
    }
    for bytes in [
        br#"{"token":"synthetic-secret"}"#.to_vec(),
        br#"{"token":"e30.e30.!","token":"synthetic-secret"}"#.to_vec(),
        response(json!({
            "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier": true,
            "exp": 101
        })),
        response(json!({
            "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier": 42,
            "exp": "101"
        })),
        serde_json::to_vec(&json!({"token": "x".repeat(MAX_TOKEN_BYTES + 1)})).expect("oversized"),
    ] {
        let error = validate_response(&bytes, now).expect_err("malformed");
        assert_eq!(error.code(), ErrorCode::ValidationUnavailable);
        let mut source: Option<&dyn Error> = Some(&error);
        while let Some(error) = source {
            assert!(!format!("{error} {error:?}").contains("synthetic-secret"));
            source = error.source();
        }
    }
}

/// Serves one local request and returns its actual headers for contract assertions.
fn service(
    status: u16,
    body: Vec<u8>,
    extra: &str,
) -> (Client, String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
    let url = format!(
        "http://{}/v1/cobalt-token",
        listener.local_addr().expect("address")
    );
    let extra = extra.to_owned();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read bound");
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).expect("request header");
            request.push(byte[0]);
            assert!(request.len() <= 16 * 1024);
        }
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .expect("write bound");
        let media = if extra.starts_with("Content-Type:") {
            ""
        } else {
            "Content-Type: application/json\r\n"
        };
        let chunked = extra == "Transfer-Encoding: chunked\r\n";
        let length = if chunked {
            String::new()
        } else {
            format!("Content-Length: {}\r\n", body.len())
        };
        write!(
            stream,
            "HTTP/1.1 {status} Mock\r\n{length}{media}Connection: close\r\n{extra}\r\n"
        )
        .expect("response header");
        let body = if chunked {
            [
                format!("{:x}\r\n", body.len()).as_bytes(),
                &body,
                b"\r\n0\r\n\r\n",
            ]
            .concat()
        } else {
            body
        };
        if let Err(error) = stream.write_all(&body) {
            assert!(
                matches!(
                    error.kind(),
                    std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                ),
                "unexpected mock write failure: {error}"
            );
        }
        String::from_utf8(request).expect("ASCII request")
    });
    let client = direct_client_builder()
        .https_only(false)
        .connect_timeout(Duration::from_secs(1))
        .timeout(Duration::from_secs(2))
        .build()
        .expect("mock client");
    (client, url, server)
}

/// Proves one real POST, validation-before-open, exact encoding, and safe success output.
#[tokio::test]
async fn accepted_service_response_confirms_only_one_cookie_save() {
    let (client, url, server) = service(200, positive(), "");
    let (control, namespace) = login_control(Duration::from_secs(5));
    let cancellation = control.cancellation();
    let store = store();
    let saved_store = Arc::clone(&store);
    let result = establish_with(input(), control, client, &url, move || Ok(saved_store))
        .await
        .expect("saved");
    let request = server.join().expect("mock completed").to_ascii_lowercase();
    assert!(request.starts_with("post /v1/cobalt-token http/1.1\r\n"));
    assert!(request.contains("\r\ncookie: cobaltsession=synthetic-session\r\n"));
    assert!(!request.contains("\r\nauthorization:"));
    assert!(!request.contains("\r\nreferer:"));
    assert_eq!(result.validation_evidence(), "service_validated");
    assert_eq!(result.expiry_evidence(), "unknown");
    assert_eq!(
        serde_json::to_value(&result).expect("safe output"),
        json!({"validation": "service_validated", "expiry": "unknown"})
    );
    let bytes = store.entry.get_secret().expect("saved bytes");
    assert_eq!(bytes, input().cookie.encode().0.as_slice());
    assert_eq!(
        store.status().expect("presence"),
        SessionStatus::StoredUnverified
    );
    drop(
        PendingLoginSlot::claim_namespace(Some(&namespace))
            .expect("released despite cancellation handle"),
    );
    cancellation.cancel();
}

/// Rejects denied, redirected, throttled, malformed, compressed, and oversized responses before storage.
#[tokio::test]
async fn service_failures_never_open_or_change_storage() {
    for (status, body, extra, expected) in [
        (401, b"denied".to_vec(), "", ErrorCode::SessionRejected),
        (403, b"denied".to_vec(), "", ErrorCode::SessionRejected),
        (
            302,
            Vec::new(),
            "Location: http://127.0.0.1:1/\r\n",
            ErrorCode::ValidationUnavailable,
        ),
        (
            429,
            Vec::new(),
            "Retry-After: 1\r\n",
            ErrorCode::ValidationUnavailable,
        ),
        (200, b"{}".to_vec(), "", ErrorCode::SessionRejected),
        (
            200,
            b"malformed".to_vec(),
            "",
            ErrorCode::ValidationUnavailable,
        ),
        (
            200,
            positive(),
            "Content-Encoding: gzip\r\n",
            ErrorCode::ValidationUnavailable,
        ),
        (
            200,
            positive(),
            "Content-Type: text/html\r\n",
            ErrorCode::ValidationUnavailable,
        ),
        (
            200,
            vec![b' '; MAX_RESPONSE_BYTES + 1],
            "Transfer-Encoding: chunked\r\n",
            ErrorCode::ValidationUnavailable,
        ),
        (
            200,
            vec![b' '; MAX_RESPONSE_BYTES + 1],
            "",
            ErrorCode::ValidationUnavailable,
        ),
    ] {
        let (client, url, server) = service(status, body, extra);
        let (control, _) = login_control(Duration::from_secs(5));
        let store = store();
        let open_count = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&open_count);
        let error = establish_with(input(), control, client, &url, move || {
            count.fetch_add(1, Ordering::SeqCst);
            panic!("rejected validation must not open storage")
        })
        .await
        .expect_err("failure");
        assert_eq!(error.code(), expected);
        assert_eq!(open_count.load(Ordering::SeqCst), 0);
        assert_eq!(
            store.entry.get_secret().expect("prior remains"),
            b"synthetic-prior"
        );
        server.join().expect("mock completed");
    }
}

/// Reports safe setup failures before write and uncertain native failures after admission.
#[tokio::test]
async fn store_setup_and_started_write_failures_are_distinct() {
    let (client, url, server) = service(200, positive(), "");
    let (control, _) = login_control(Duration::from_secs(5));
    let error = establish_with(input(), control, client, &url, || {
        Err(AuthError::new(AuthErrorKind::StorageAccess))
    })
    .await
    .expect_err("denied setup");
    assert_eq!(error.code(), ErrorCode::StorageAccess);
    server.join().expect("mock completed");

    let (client, url, server) = service(200, positive(), "");
    let (control, _) = login_control(Duration::from_secs(5));
    let store = store();
    store
        .entry
        .as_any()
        .downcast_ref::<mock::Cred>()
        .expect("mock")
        .set_error(keyring_core::Error::NoStorageAccess(Box::new(
            std::io::Error::other("synthetic-secret"),
        )));
    let target = Arc::clone(&store);
    let error = establish_with(input(), control, client, &url, move || Ok(target))
        .await
        .expect_err("write failed");
    assert_eq!(error.code(), ErrorCode::SaveIndeterminate);
    assert!(!format!("{error:?} {}", error.source().expect("cause")).contains("synthetic-secret"));
    assert_eq!(
        store.entry.get_secret().expect("mock did not change"),
        b"synthetic-prior"
    );
    server.join().expect("mock completed");
}

/// Cancels or drops a real queued blocking operation and prevents its later native write.
#[tokio::test]
async fn cancellation_and_drop_before_write_preserve_prior_entry() {
    for mode in ["cancel", "timeout", "drop"] {
        let (client, url, server) = service(200, positive(), "");
        let duration = if mode == "timeout" {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };
        let (control, namespace) = login_control(duration);
        let cancellation = control.cancellation();
        let store = store();
        let target = Arc::clone(&store);
        let (entered, entered_rx) = tokio::sync::oneshot::channel();
        let (release, release_rx) = mpsc::channel();
        let future = establish_with(input(), control, client, &url, move || {
            entered.send(()).expect("entered");
            release_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("release");
            Ok(target)
        });
        let mut future = Box::pin(future);
        tokio::select! {
            result = &mut future => panic!("premature completion: {result:?}"),
            result = entered_rx => result.expect("worker setup"),
        }
        if mode == "drop" {
            drop(future);
        } else {
            if mode == "cancel" {
                assert_eq!(cancellation.cancel_outcome(), ErrorCode::Cancelled);
            }
            assert_eq!(
                future.await.expect_err("cancelled").code(),
                if mode == "cancel" {
                    ErrorCode::Cancelled
                } else {
                    ErrorCode::TimedOut
                }
            );
        }
        assert!(PendingLoginSlot::claim_namespace(Some(&namespace)).is_err());
        release.send(()).expect("release worker");
        wait_released(&namespace).await;
        assert_eq!(
            store.entry.get_secret().expect("prior remains"),
            b"synthetic-prior"
        );
        server.join().expect("mock completed");
    }
}

/// Waits at most one second for the owned worker's reservation to be released.
async fn wait_released(namespace: &str) {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let Ok(slot) = PendingLoginSlot::claim_namespace(Some(namespace)) {
                drop(slot);
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("worker reservation released");
}

/// Wraps a synthetic credential so the actual set_secret call can wait for test release.
struct BlockingCredential {
    /// Delegates ordinary credential operations to the in-memory mock.
    inner: Entry,
    /// Announces entry into the already-admitted write.
    entered: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    /// Bounds the synthetic blocking native call.
    release: Mutex<mpsc::Receiver<()>>,
}

impl CredentialApi for BlockingCredential {
    /// Blocks only the synthetic write, then stores the exact supplied bytes.
    fn set_secret(&self, secret: &[u8]) -> keyring_core::Result<()> {
        self.entered
            .lock()
            .expect("entered lock")
            .take()
            .expect("one write")
            .send(())
            .expect("write entered");
        self.release
            .lock()
            .expect("release lock")
            .recv_timeout(Duration::from_secs(2))
            .expect("write release");
        self.inner.set_secret(secret)
    }

    /// Reads the underlying mock without native credential access.
    fn get_secret(&self) -> keyring_core::Result<Vec<u8>> {
        self.inner.get_secret()
    }

    /// Deletes only the underlying synthetic entry.
    fn delete_credential(&self) -> keyring_core::Result<()> {
        self.inner.delete_credential()
    }

    /// Returns this existing wrapper rather than creating a second credential.
    fn get_credential(&self) -> keyring_core::Result<Option<Arc<Credential>>> {
        Ok(None)
    }

    /// Preserves the mock's fixed entry identity.
    fn get_specifiers(&self) -> Option<(String, String)> {
        self.inner.get_specifiers()
    }

    /// Supports the keyring's required concrete-wrapper inspection.
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Treats cancellation, timeout, and caller drop during actual blocking writes as uncertain.
#[tokio::test]
async fn started_writes_retain_reservation_and_never_claim_unchanged() {
    for mode in ["cancel", "timeout", "drop"] {
        let (client, url, server) = service(200, positive(), "");
        let duration = if mode == "timeout" {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };
        let (control, namespace) = login_control(duration);
        let cancellation = control.cancellation();
        let backend = mock::Store::new().expect("backend");
        let inner = backend
            .build("beyond-gate", "local-session", None)
            .expect("entry");
        inner.set_secret(b"synthetic-prior").expect("prior");
        let (entered, entered_rx) = tokio::sync::oneshot::channel();
        let (release, release_rx) = mpsc::channel();
        let target = Arc::new(SessionStore {
            entry: Entry::new_with_credential(Arc::new(BlockingCredential {
                inner,
                entered: Mutex::new(Some(entered)),
                release: Mutex::new(release_rx),
            })),
        });
        let retained = Arc::clone(&target);
        let mut future = Box::pin(establish_with(input(), control, client, &url, move || {
            Ok(retained)
        }));
        tokio::select! {
            result = &mut future => panic!("premature completion: {result:?}"),
            result = entered_rx => result.expect("write entered"),
        }
        if mode == "drop" {
            drop(future);
        } else {
            if mode == "cancel" {
                assert_eq!(cancellation.cancel_outcome(), ErrorCode::SaveIndeterminate);
            }
            assert_eq!(
                future.await.expect_err("uncertain write").code(),
                ErrorCode::SaveIndeterminate
            );
        }
        assert!(PendingLoginSlot::claim_namespace(Some(&namespace)).is_err());
        release.send(()).expect("release write");
        wait_released(&namespace).await;
        assert_eq!(
            target.entry.get_secret().expect("completed once"),
            input().cookie.encode().0.as_slice()
        );
        server.join().expect("mock completed");
    }
}

/// Rejects pre-cancellation and expired controls before any upstream request or store open.
#[tokio::test]
async fn cancelled_control_never_starts_an_exchange() {
    let (control, _) = login_control(Duration::from_secs(1));
    control.cancellation().cancel();
    let client = direct_client_builder().build().expect("client");
    let error = establish_with(input(), control, client, "http://127.0.0.1:1/", || {
        panic!("no store")
    })
    .await
    .expect_err("cancelled");
    assert_eq!(error.code(), ErrorCode::Cancelled);
}

/// Rejects an elapsed original deadline before requesting or opening storage.
#[tokio::test]
async fn expired_control_never_starts_an_exchange() {
    let (control, _) = login_control(Duration::from_millis(10));
    tokio::time::sleep(Duration::from_millis(20)).await;
    let client = direct_client_builder().build().expect("client");
    let error = establish_with(input(), control, client, "http://127.0.0.1:1/", || {
        panic!("no store")
    })
    .await
    .expect_err("expired");
    assert_eq!(error.code(), ErrorCode::TimedOut);
}
