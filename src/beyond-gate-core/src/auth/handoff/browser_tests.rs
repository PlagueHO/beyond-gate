//! Offline relay checks with actual Noise and an explicitly synthetic CLI peer.
//!
//! The peer's final-result claim is injected test data, not service-validation or
//! native-save evidence. No browser profile, account, network, or store is used.

use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use tokio::io::{DuplexStream, duplex, split};

use crate::auth::{
    SessionSecret,
    handoff::pairing::{PairingKey, PendingPairing},
};

/// Selects a fixed synthetic exchange outcome without accepting external input.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Case {
    Success,
    Rejected,
    Cancelled,
    LateCancellation,
    EarlyAck,
    WrongTransaction,
    LostAck,
    Stalled,
}

/// Constructs a known fake key without reading or printing a product key.
fn key() -> Zeroizing<Vec<u8>> {
    Zeroizing::new(vec![42; 32])
}

/// Constructs the input-only preface with invented material and explicit consent.
fn pair_payload(browser: Browser) -> Zeroizing<Vec<u8>> {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/fixtures/windows-login-v1.json"
    ))
    .expect("shared invented recipe");
    let mut recipe = fixture["messages"]["pair"].clone();
    assert_eq!(recipe["key"], URL_SAFE_NO_PAD.encode(&*key()));
    recipe["browser"] = if browser == Browser::Edge {
        "edge"
    } else {
        "chrome"
    }
    .into();
    Zeroizing::new(serde_json::to_vec(&recipe).expect("invented pair"))
}

/// Exchanges framed browser messages with the real relay, not a browser cookie API.
async fn client(stream: DuplexStream, browser: Browser, case: Case) {
    let deadline = Instant::now() + Duration::from_secs(40);
    let (read, write) = split(stream);
    let mut reader = FrameReader::new(read, Protocol::Browser);
    let mut writer = FrameWriter::new(write, Protocol::Browser);
    writer
        .write_message(pair_payload(browser), deadline)
        .await
        .expect("preface");
    let transaction = match reader.read_message(deadline).await.expect("pending") {
        Message::Authorization {
            transaction,
            profile: Profile::CobaltSessionV1,
            ..
        } => transaction,
        _ => panic!("bound production authorization"),
    };
    let submitted = if case == Case::WrongTransaction {
        Transaction::generate().expect("different invented transaction")
    } else {
        transaction.clone()
    };
    let submit = Message::Submit {
        transaction: submitted,
        profile: Profile::CobaltSessionV1,
        value: SessionSecret::new(b"invented-cookie-value".to_vec()).expect("invented cookie"),
    };
    writer
        .write_message(
            encode_message(Protocol::Browser, &submit).expect("submit"),
            deadline,
        )
        .await
        .expect("submit");
    if matches!(
        case,
        Case::Cancelled | Case::LateCancellation | Case::EarlyAck
    ) {
        let control = if matches!(case, Case::Cancelled | Case::LateCancellation) {
            Message::Cancel {
                transaction: Some(transaction.clone()),
            }
        } else {
            Message::Acknowledgement {
                transaction: transaction.clone(),
            }
        };
        writer
            .write_message(
                encode_message(Protocol::Browser, &control).expect("control"),
                deadline,
            )
            .await
            .expect("control");
    }
    let result = reader.read_completion(deadline).await.expect("final frame");
    if case == Case::Success || case == Case::LostAck {
        assert!(
            matches!(result, Message::Result { transaction: ref received } if received == &transaction)
        );
        if case == Case::LostAck {
            return;
        }
        let ack = Message::Acknowledgement {
            transaction: transaction.clone(),
        };
        writer
            .write_message(
                encode_message(Protocol::Browser, &ack).expect("ack"),
                deadline,
            )
            .await
            .expect("ack");
        assert!(matches!(
            reader.read_message(deadline).await.expect("confirmed receipt"),
            Message::Acknowledgement { transaction: received } if received == transaction
        ));
    } else {
        let expected = match case {
            Case::Rejected => ErrorCode::SessionRejected,
            Case::Cancelled => ErrorCode::Cancelled,
            Case::LateCancellation => ErrorCode::SaveIndeterminate,
            Case::EarlyAck => ErrorCode::ProtocolError,
            Case::WrongTransaction => ErrorCode::TransactionMismatch,
            Case::Stalled => ErrorCode::SaveIndeterminate,
            _ => panic!("failure case"),
        };
        assert!(
            matches!(result, Message::Error { transaction: Some(received), code }
            if received == transaction && code == expected)
        );
    }
}

/// Supplies a trusted synthetic CLI claim to test relay ordering only.
async fn cli_peer(stream: DuplexStream, browser: Browser, case: Case) {
    let deadline = Instant::now() + Duration::from_secs(40);
    let ticket = PendingPairing::new(key(), deadline).expect("invented ticket");
    let mut cli = PairedConnection::accept(stream, ticket)
        .await
        .expect("paired CLI");
    assert!(matches!(
        cli.read_message(deadline).await.expect("open"),
        Message::Initiate { browser: declared, profile: Profile::CobaltSessionV1 }
            if declared == browser
    ));
    let transaction = Transaction::generate().expect("public transaction");
    let authorization = Message::Authorization {
        transaction: transaction.clone(),
        profile: Profile::CobaltSessionV1,
        expires_in_ms: 30_000,
    };
    cli.write_message(
        encode_message(Protocol::HostCli, &authorization).expect("authorization"),
        deadline,
    )
    .await
    .expect("authorization");
    let submit = cli.read_message(deadline).await;
    if case == Case::WrongTransaction {
        assert!(submit.is_err());
        return;
    }
    assert!(matches!(submit.expect("submit"),
        Message::Submit { transaction: received, profile: Profile::CobaltSessionV1, .. }
            if received == transaction));
    if case == Case::Stalled {
        assert!(cli.read_completion(deadline).await.is_err());
        return;
    }
    if matches!(
        case,
        Case::Cancelled | Case::LateCancellation | Case::EarlyAck
    ) {
        assert!(matches!(
            cli.read_completion(deadline).await.expect("forwarded stop"),
            Message::Cancel { transaction: Some(received) } if received == transaction
        ));
        let result = if case == Case::LateCancellation {
            Message::Result { transaction }
        } else {
            Message::Error {
                transaction: Some(transaction),
                code: ErrorCode::Cancelled,
            }
        };
        cli.write_message(
            encode_message(Protocol::HostCli, &result).expect("confirmed stop"),
            deadline,
        )
        .await
        .expect("stop result");
        return;
    }
    let result = if case == Case::Rejected {
        Message::Error {
            transaction: Some(transaction.clone()),
            code: ErrorCode::SessionRejected,
        }
    } else {
        Message::Result {
            transaction: transaction.clone(),
        }
    };
    cli.write_message(
        encode_message(Protocol::HostCli, &result).expect("mock result"),
        deadline,
    )
    .await
    .expect("mock result");
    if case == Case::Rejected {
        return;
    }
    let ack = cli.read_message(deadline).await;
    if case == Case::LostAck {
        assert!(ack.is_err());
        return;
    }
    let ack = ack.expect("ack");
    assert!(
        matches!(&ack, Message::Acknowledgement { transaction: received }
        if received == &transaction)
    );
    cli.write_message(
        encode_message(Protocol::HostCli, &ack).expect("receipt confirmation"),
        deadline,
    )
    .await
    .expect("receipt confirmation");
}

/// Runs all three actual stream peers with no platform endpoint or fake saved-session type.
async fn scenario(browser: Browser, case: Case) -> Result<(), HandoffError> {
    let (client_stream, host_stream) = duplex(65_536);
    let (host_cli, peer_cli) = duplex(65_536);
    let (read, write) = split(host_stream);
    let (result, (), ()) = tokio::join!(
        run_with(read, write, |key, deadline| PairedConnection::connect(
            host_cli, key, deadline
        )),
        client(client_stream, browser, case),
        cli_peer(peer_cli, browser, case)
    );
    result
}

/// Checks the exact origin and treats the parent window as syntax, not authorization.
#[test]
fn launch_arguments_have_no_origin_fallback() {
    for arguments in [
        vec![EXTENSION_ORIGIN.into()],
        vec![EXTENSION_ORIGIN.into(), "--parent-window=0".into()],
    ] {
        assert!(check_launch(&arguments).is_ok());
    }
    for arguments in [
        vec![],
        vec!["chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/".into()],
        vec![EXTENSION_ORIGIN.trim_end_matches('/').into()],
        vec![EXTENSION_ORIGIN.into(), "--parent-window=unknown".into()],
        vec![EXTENSION_ORIGIN.into(), "--unexpected".into()],
    ] {
        assert_eq!(
            check_launch(&arguments).expect_err("origin").code(),
            ErrorCode::OriginRejected
        );
    }
}

/// Requires final CLI receipt confirmation for each declared supported browser.
#[tokio::test]
async fn paired_relay_requires_final_receipt_confirmation() {
    for browser in [Browser::Chrome, Browser::Edge] {
        scenario(browser, Case::Success).await.expect("relay");
    }
}

/// Preserves fixed rejection, cancellation, and correlation errors without false success.
#[tokio::test]
async fn paired_relay_rejects_invalid_or_cancelled_progress() {
    for (case, expected) in [
        (Case::Rejected, ErrorCode::SessionRejected),
        (Case::Cancelled, ErrorCode::Cancelled),
        (Case::LateCancellation, ErrorCode::SaveIndeterminate),
        (Case::EarlyAck, ErrorCode::ProtocolError),
        (Case::WrongTransaction, ErrorCode::TransactionMismatch),
        (Case::LostAck, ErrorCode::SaveIndeterminate),
    ] {
        assert_eq!(
            scenario(Browser::Edge, case)
                .await
                .expect_err("failure")
                .code(),
            expected
        );
    }
}

/// Caps validation/save result waiting at twenty seconds without refreshing the deadline.
#[tokio::test(start_paused = true)]
async fn paired_relay_stops_a_stalled_cli() {
    let start = Instant::now();
    assert_eq!(
        scenario(Browser::Edge, Case::Stalled)
            .await
            .expect_err("timeout")
            .code(),
        ErrorCode::SaveIndeterminate
    );
    assert_eq!(Instant::now() - start, Duration::from_secs(20));
}

/// Rejects absent consent before invoking even the injected connection factory.
#[tokio::test]
async fn missing_consent_never_connects_to_pending_cli() {
    let (client, host) = duplex(4096);
    let (read, write) = split(host);
    let mut value: serde_json::Value =
        serde_json::from_slice(&pair_payload(Browser::Edge)).expect("pair");
    value["consent"] = false.into();
    let payload = serde_json::to_vec(&value).expect("invented pair");
    let (mut output, mut input) = split(client);
    use tokio::io::AsyncWriteExt;
    input
        .write_all(&(payload.len() as u32).to_ne_bytes())
        .await
        .expect("prefix");
    input.write_all(&payload).await.expect("pair");
    let mut called = false;
    let result = run_with(read, write, |_, _| {
        called = true;
        async {
            Err::<PairedConnection<DuplexStream>, _>(HandoffError::new(ErrorCode::ProtocolError))
        }
    })
    .await;
    assert!(!called);
    assert_eq!(
        result.expect_err("consent").code(),
        ErrorCode::ConsentRequired
    );
    let mut reader = FrameReader::new(&mut output, Protocol::Browser);
    assert!(matches!(
        reader
            .read_message(Instant::now() + Duration::from_secs(1))
            .await
            .expect("error"),
        Message::Error {
            transaction: None,
            code: ErrorCode::ConsentRequired
        }
    ));
}

/// Keeps pairing syntax strict and diagnostics free of encoded or decoded key bytes.
#[test]
fn pairing_key_input_is_canonical_and_redacted() {
    let encoded = URL_SAFE_NO_PAD.encode(&*key());
    let key = PairingKey::from_encoded(Zeroizing::new(encoded.clone())).expect("canonical key");
    assert_eq!(key.into_bytes().len(), 32);
    for invalid in [
        format!("{encoded}="),
        format!(" {encoded}"),
        format!("+{}", &encoded[..42]),
        format!("{}!", &encoded[..42]),
        format!("{}p", &encoded[..42]),
    ] {
        let error = PairingKey::from_encoded(Zeroizing::new(invalid)).expect_err("noncanonical");
        assert_eq!(error.code(), ErrorCode::InvalidMaterial);
        assert!(!format!("{error:?}").contains(&encoded));
    }
}
