//! Offline consent, correlation, and cancellation checks on real paired streams.

mod support;

use std::time::Duration;

use beyond_gate_core::auth::handoff::{
    ErrorCode, Message, Profile, Transaction,
    flow::PendingHandoff,
    pairing::{PairingKey, PendingPairing},
    transport::PairedConnection,
};
use serde_json::Value;
use tokio::{
    io::{DuplexStream, duplex},
    time::Instant,
};
use zeroize::Zeroizing;

/// Serializes one repository-owned synthetic message with no account data.
fn payload(name: &str) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(serde_json::to_vec(&support::fixture()["messages"][name]).expect("fixture"))
}

/// Creates actual Noise peers using a system-generated key without exposing its bytes.
async fn pair(
    deadline: Instant,
) -> (
    PairedConnection<DuplexStream>,
    PairedConnection<DuplexStream>,
) {
    let secret = PairingKey::generate()
        .expect("system randomness")
        .into_bytes();
    let ticket = PendingPairing::new(Zeroizing::new(secret.to_vec()), deadline).expect("ticket");
    let (host, cli) = duplex(65536);
    let (host, cli) = tokio::join!(
        PairedConnection::connect(host, secret, deadline),
        PairedConnection::accept(cli, ticket)
    );
    (host.expect("host pairing"), cli.expect("CLI pairing"))
}

/// Opens one consented exchange and receives its real core-generated authorization.
async fn authorize(host: &mut PairedConnection<DuplexStream>, deadline: Instant) -> Transaction {
    authorize_profile(host, deadline, Profile::SyntheticV1).await
}

/// Checks that the authorization returns the exact profile selected before opening.
async fn authorize_profile(
    host: &mut PairedConnection<DuplexStream>,
    deadline: Instant,
    profile: Profile,
) -> Transaction {
    let mut open = support::fixture()["messages"]["cli_open"].clone();
    open["profile"] = profile.as_str().into();
    host.write_message(
        Zeroizing::new(serde_json::to_vec(&open).expect("profile-bound open")),
        deadline,
    )
    .await
    .expect("open");
    match host.read_message(deadline).await.expect("authorization") {
        Message::Authorization {
            transaction,
            profile: authorized_profile,
            expires_in_ms,
        } => {
            assert_eq!(authorized_profile, profile);
            assert!((1..=300_000).contains(&expires_in_ms));
            transaction
        }
        _ => panic!("only authorization is permitted"),
    }
}

/// Produces a guaranteed different public identifier without guessing generated values.
fn different(transaction: &Transaction) -> String {
    let mut value = transaction.as_str().to_owned();
    let replacement = if value.starts_with('0') { "1" } else { "0" };
    value.replace_range(..1, replacement);
    value
}

/// Makes one bound synthetic message with an optional fixed-code error.
fn bound(name: &str, transaction: &Transaction, code: Option<&str>) -> Zeroizing<Vec<u8>> {
    let mut value = support::fixture()["messages"][name].clone();
    value["transaction"] = Value::String(transaction.as_str().to_owned());
    if let Some(code) = code {
        value["code"] = Value::String(code.to_owned());
    }
    Zeroizing::new(serde_json::to_vec(&value).expect("bound fixture"))
}

/// Checks generated public identifiers without treating format as entropy proof.
#[test]
fn generated_transaction_has_exact_public_format() {
    let transaction = Transaction::generate().expect("system randomness");
    assert_eq!(transaction.as_str().len(), 32);
    assert!(
        transaction
            .as_str()
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    assert!(!format!("{transaction:?}").contains(transaction.as_str()));
}

/// Checks exact pending lifetime limits before any authorization can be issued.
#[tokio::test(start_paused = true)]
async fn pending_deadlines_are_exact() {
    for (duration, expected) in [
        (Duration::ZERO, Some(ErrorCode::TimedOut)),
        (Duration::from_secs(300), None),
        (
            Duration::from_secs(300) + Duration::from_nanos(1),
            Some(ErrorCode::ProtocolError),
        ),
    ] {
        let transaction = Transaction::generate().expect("transaction");
        let result = PendingHandoff::new(transaction, Instant::now() + duration);
        assert_eq!(result.err().map(|error| error.code()), expected);
    }
}

/// Checks real authorization and one receipt without sending a login-success result.
#[tokio::test]
async fn bound_receipt_is_not_validation_or_save() {
    let deadline = Instant::now() + Duration::from_secs(30);
    let (mut host, cli) = pair(deadline).await;
    let transaction = Transaction::generate().expect("transaction");
    let expected = transaction.clone();
    let pending = PendingHandoff::new(transaction, deadline).expect("pending");
    let (receipt, ()) = tokio::join!(pending.receive(cli), async {
        let bound_transaction = authorize(&mut host, deadline).await;
        assert_eq!(bound_transaction, expected);
        host.write_message(bound("cli_submit", &bound_transaction, None), deadline)
            .await
            .expect("submit");
    });
    let (connection, receipt) = receipt.expect("bound receipt");
    assert_eq!(receipt.transaction(), &expected);
    assert_eq!(
        format!("{receipt:?}"),
        "ReceivedSubmission { fields: [REDACTED] }"
    );
    let material = receipt.into_value();
    assert!(!format!("{material:?}").contains("synthetic-session"));
    drop(material);
    drop(connection);
    assert!(
        host.read_message(deadline).await.is_err(),
        "receipt emits no final success"
    );
}

/// Transfers only a production-bound receipt without exchange, storage, or success.
#[cfg(windows)]
#[tokio::test]
async fn windows_receipt_transfers_to_core_without_claiming_saved_login() {
    use beyond_gate_core::auth::establishment::SessionEstablishmentInput;

    for profile in [Profile::SyntheticV1, Profile::CobaltSessionV1] {
        let deadline = Instant::now() + Duration::from_secs(30);
        let (mut host, cli) = pair(deadline).await;
        let pending = PendingHandoff::for_profile(
            profile,
            Transaction::generate().expect("random"),
            deadline,
        )
        .expect("pending");
        let (result, ()) = tokio::join!(pending.receive(cli), async {
            let transaction = authorize_profile(&mut host, deadline, profile).await;
            let mut submit: Value =
                serde_json::from_slice(&bound("cli_submit", &transaction, None)).expect("submit");
            submit["profile"] = profile.as_str().into();
            host.write_message(
                Zeroizing::new(serde_json::to_vec(&submit).expect("profile-bound submit")),
                deadline,
            )
            .await
            .expect("submit");
        });
        let (connection, receipt) = result.expect("receipt");
        assert_eq!(receipt.profile(), profile);
        let input = SessionEstablishmentInput::from_handoff(receipt);
        if profile == Profile::CobaltSessionV1 {
            let input = input.expect("production-bound input");
            assert!(!format!("{input:?}").contains("synthetic-session"));
        } else {
            assert_eq!(
                input.expect_err("synthetic input is not production").code(),
                ErrorCode::UnsupportedProfile
            );
        }
        drop(connection);
        assert!(
            host.read_message(deadline).await.is_err(),
            "receipt emits no saved-login result"
        );
    }
}

/// Rejects a peer-selected profile before issuing authorization in either direction.
#[cfg(windows)]
#[tokio::test]
async fn windows_pending_profile_cannot_be_changed_by_open() {
    for (pending_profile, peer_profile) in [
        (Profile::SyntheticV1, Profile::CobaltSessionV1),
        (Profile::CobaltSessionV1, Profile::SyntheticV1),
    ] {
        let deadline = Instant::now() + Duration::from_secs(30);
        let (mut host, cli) = pair(deadline).await;
        let pending = PendingHandoff::for_profile(
            pending_profile,
            Transaction::generate().expect("random"),
            deadline,
        )
        .expect("pending");
        let mut open = support::fixture()["messages"]["cli_open"].clone();
        open["profile"] = peer_profile.as_str().into();
        host.write_message(
            Zeroizing::new(serde_json::to_vec(&open).expect("wrong-profile open")),
            deadline,
        )
        .await
        .expect("open");
        assert_eq!(
            pending.receive(cli).await.err().expect("rejection").code(),
            ErrorCode::UnsupportedProfile
        );
        assert!(
            host.read_message(deadline).await.is_err(),
            "no authorization"
        );
    }
}

/// Rejects changing a profile after authorization even with the correct transaction.
#[cfg(windows)]
#[tokio::test]
async fn windows_bound_profile_cannot_be_changed_by_submit() {
    for (pending_profile, peer_profile) in [
        (Profile::SyntheticV1, Profile::CobaltSessionV1),
        (Profile::CobaltSessionV1, Profile::SyntheticV1),
    ] {
        let deadline = Instant::now() + Duration::from_secs(30);
        let (mut host, cli) = pair(deadline).await;
        let pending = PendingHandoff::for_profile(
            pending_profile,
            Transaction::generate().expect("random"),
            deadline,
        )
        .expect("pending");
        let (result, ()) = tokio::join!(pending.receive(cli), async {
            let transaction = authorize_profile(&mut host, deadline, pending_profile).await;
            let mut submit: Value =
                serde_json::from_slice(&bound("cli_submit", &transaction, None)).expect("submit");
            submit["profile"] = peer_profile.as_str().into();
            host.write_message(
                Zeroizing::new(serde_json::to_vec(&submit).expect("wrong-profile submit")),
                deadline,
            )
            .await
            .expect("submit");
        });
        assert_eq!(
            result.err().expect("profile rejection").code(),
            ErrorCode::UnsupportedProfile
        );
        assert!(host.read_message(deadline).await.is_err());
    }
}

/// Checks that valid but out-of-order messages cannot obtain authorization.
#[tokio::test]
async fn messages_before_open_are_rejected() {
    for name in ["cli_submit", "cli_result", "cli_authorized", "cli_error"] {
        let deadline = Instant::now() + Duration::from_secs(30);
        let (mut host, cli) = pair(deadline).await;
        let pending = PendingHandoff::new(Transaction::generate().expect("transaction"), deadline)
            .expect("pending");
        host.write_message(payload(name), deadline)
            .await
            .expect("out-of-order message");
        assert_eq!(
            pending.receive(cli).await.err().expect("rejection").code(),
            ErrorCode::ProtocolError
        );
        assert!(
            host.read_message(deadline).await.is_err(),
            "no authorization"
        );
    }
}

/// Checks that cancellation before binding never receives authorization.
#[tokio::test]
async fn cancellation_before_open_closes_without_authorization() {
    let deadline = Instant::now() + Duration::from_secs(30);
    let (mut host, cli) = pair(deadline).await;
    let pending = PendingHandoff::new(Transaction::generate().expect("transaction"), deadline)
        .expect("pending");
    let mut cancel = support::fixture()["messages"]["cli_cancel"].clone();
    cancel["transaction"] = Value::Null;
    host.write_message(
        Zeroizing::new(serde_json::to_vec(&cancel).expect("cancel")),
        deadline,
    )
    .await
    .expect("cancel");
    assert_eq!(
        pending.receive(cli).await.err().expect("cancelled").code(),
        ErrorCode::Cancelled
    );
    assert!(host.read_message(deadline).await.is_err());
}

/// Checks consent rejection before any application bytes are emitted.
#[tokio::test]
async fn missing_consent_never_obtains_authorization() {
    let deadline = Instant::now() + Duration::from_secs(30);
    let (mut host, cli) = pair(deadline).await;
    let pending = PendingHandoff::new(Transaction::generate().expect("transaction"), deadline)
        .expect("pending");
    let mut open = support::fixture()["messages"]["cli_open"].clone();
    open["consent"] = Value::Bool(false);
    let error = host
        .write_message(
            Zeroizing::new(serde_json::to_vec(&open).expect("open")),
            deadline,
        )
        .await
        .expect_err("consent");
    assert_eq!(error.code(), ErrorCode::ConsentRequired);
    assert!(pending.receive(cli).await.is_err());
}

/// Checks correlation and rejects peer claims of validation or storage failure.
#[tokio::test]
async fn claimed_exchange_rejects_invalid_followups() {
    for (name, wrong, null, code, expected) in [
        (
            "cli_submit",
            true,
            false,
            None,
            ErrorCode::TransactionMismatch,
        ),
        ("cli_cancel", false, false, None, ErrorCode::Cancelled),
        (
            "cli_cancel",
            true,
            false,
            None,
            ErrorCode::TransactionMismatch,
        ),
        (
            "cli_cancel",
            false,
            true,
            None,
            ErrorCode::TransactionMismatch,
        ),
        (
            "cli_error",
            false,
            false,
            Some("cookie_unavailable"),
            ErrorCode::CookieUnavailable,
        ),
        (
            "cli_error",
            false,
            false,
            Some("storage_failure"),
            ErrorCode::ProtocolError,
        ),
        (
            "cli_error",
            true,
            false,
            Some("cookie_unavailable"),
            ErrorCode::TransactionMismatch,
        ),
        (
            "cli_error",
            false,
            true,
            Some("cookie_unavailable"),
            ErrorCode::TransactionMismatch,
        ),
        ("cli_result", false, false, None, ErrorCode::ProtocolError),
        ("cli_open", false, false, None, ErrorCode::ProtocolError),
    ] {
        let deadline = Instant::now() + Duration::from_secs(30);
        let (mut host, cli) = pair(deadline).await;
        let pending = PendingHandoff::new(Transaction::generate().expect("transaction"), deadline)
            .expect("pending");
        let (result, ()) = tokio::join!(pending.receive(cli), async {
            let transaction = authorize(&mut host, deadline).await;
            let mut value: Value =
                serde_json::from_slice(&bound(name, &transaction, code)).expect("followup");
            if name == "cli_open" {
                value.as_object_mut().expect("open").remove("transaction");
            } else if null {
                value["transaction"] = Value::Null;
            } else if wrong {
                value["transaction"] = Value::String(different(&transaction));
            }
            host.write_message(
                Zeroizing::new(serde_json::to_vec(&value).expect("followup")),
                deadline,
            )
            .await
            .expect("followup");
        });
        assert_eq!(result.err().expect("rejection").code(), expected);
        assert!(host.read_message(deadline).await.is_err());
    }
}

/// Checks the ten-second hello cap rather than the longer pending deadline.
#[tokio::test(start_paused = true)]
async fn stalled_open_has_ten_second_limit() {
    let deadline = Instant::now() + Duration::from_secs(300);
    let (_host, cli) = pair(deadline).await;
    let pending = PendingHandoff::new(Transaction::generate().expect("transaction"), deadline)
        .expect("pending");
    let started = Instant::now();
    assert_eq!(
        pending.receive(cli).await.err().expect("timeout").code(),
        ErrorCode::TimedOut
    );
    assert_eq!(Instant::now() - started, Duration::from_secs(10));
}

/// Checks that waiting for submission cannot extend the fifteen-second operation cap.
#[tokio::test(start_paused = true)]
async fn stalled_submit_has_fifteen_second_limit() {
    let deadline = Instant::now() + Duration::from_secs(300);
    let (mut host, cli) = pair(deadline).await;
    let pending = PendingHandoff::new(Transaction::generate().expect("transaction"), deadline)
        .expect("pending");
    let started = Instant::now();
    let (result, _) = tokio::join!(pending.receive(cli), authorize(&mut host, deadline));
    assert_eq!(result.err().expect("timeout").code(), ErrorCode::TimedOut);
    assert_eq!(Instant::now() - started, Duration::from_secs(15));
}

/// Checks expired pending state even when a valid open is already buffered.
#[tokio::test(start_paused = true)]
async fn ready_open_cannot_reuse_expired_pending_state() {
    let deadline = Instant::now() + Duration::from_secs(30);
    let (mut host, cli) = pair(deadline).await;
    let expires = Instant::now() + Duration::from_secs(1);
    let pending = PendingHandoff::new(Transaction::generate().expect("transaction"), expires)
        .expect("pending");
    host.write_message(payload("cli_open"), deadline)
        .await
        .expect("ready open");
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(
        pending.receive(cli).await.err().expect("expired").code(),
        ErrorCode::TimedOut
    );
    assert!(host.read_message(deadline).await.is_err());
}

/// Checks that dropping an authorized receive future closes the owned paired connection.
#[tokio::test]
async fn cancelled_authorized_receive_closes_connection() {
    let deadline = Instant::now() + Duration::from_secs(30);
    let (mut host, cli) = pair(deadline).await;
    let pending = PendingHandoff::new(Transaction::generate().expect("transaction"), deadline)
        .expect("pending");
    host.write_message(payload("cli_open"), deadline)
        .await
        .expect("open");
    let mut receive = Box::pin(pending.receive(cli));
    let message = tokio::select! {
        _ = &mut receive => panic!("receive must await submission"),
        message = host.read_message(deadline) => message.expect("authorization"),
    };
    assert!(matches!(message, Message::Authorization { .. }));
    drop(receive);
    assert!(host.read_message(deadline).await.is_err());
}
