//! Offline cryptographic pairing checks with fixed invented secrets and real Noise.
//!
//! These tests prove the channel primitive, not user delivery, native IPC, pending
//! authorization, executable identity, account validation, or protected storage.

use beyond_gate_core::auth::handoff::{
    ErrorCode,
    pairing::{PairedChannel, PairingHandshake, PendingPairing, Role},
};
use std::time::Duration;
use tokio::time::Instant;
use zeroize::Zeroizing;

/// Constructs a reproducible invented key; production callers must use OS randomness.
fn key() -> Zeroizing<Vec<u8>> {
    Zeroizing::new(vec![42; 32])
}

/// Completes a real two-message handshake with independently generated ephemeral keys.
///
/// # Panics
///
/// Panics if the synthetic peers cannot complete their expected handshake.
fn channels() -> (PairedChannel, PairedChannel) {
    let mut host = PairingHandshake::new(Role::Host, key()).expect("synthetic host");
    let mut cli = PairingHandshake::new(Role::Cli, key()).expect("synthetic CLI");
    cli.read(&host.write().expect("host hello"))
        .expect("CLI accepts hello");
    host.read(&cli.write().expect("CLI reply"))
        .expect("host accepts reply");
    (
        host.finish().expect("paired host"),
        cli.finish().expect("paired CLI"),
    )
}

/// Checks real authentication and encryption independently in both directions.
#[test]
fn paired_channel_encrypts_both_directions() {
    let (mut host, mut cli) = channels();
    let sent = b"synthetic-private-submit";
    let ciphertext = host.seal(Zeroizing::new(sent.to_vec())).expect("seal");
    assert!(!ciphertext.windows(sent.len()).any(|window| window == sent));
    assert_eq!(&*cli.open(&ciphertext).expect("open"), sent);
    let reply = cli
        .seal(Zeroizing::new(b"synthetic-safe-result".to_vec()))
        .expect("seal reply");
    assert_eq!(
        &*host.open(&reply).expect("open reply"),
        b"synthetic-safe-result"
    );
}

/// Checks that the cryptographic primitive cannot turn an unfinished handshake into a channel.
#[test]
fn incomplete_handshake_cannot_release_application_channel() {
    for role in [Role::Host, Role::Cli] {
        let handshake = PairingHandshake::new(role, key()).expect("synthetic peer");
        assert!(handshake.finish().is_err());
    }
}

/// Checks that a wrong secret fails proof and permanently closes the attempted handshake.
#[test]
fn wrong_secret_closes_handshake() {
    let mut host = PairingHandshake::new(Role::Host, key()).expect("host");
    let mut cli = PairingHandshake::new(Role::Cli, Zeroizing::new(vec![43; 32])).expect("CLI");
    let message = host.write().expect("hello");
    let error = cli.read(&message).expect_err("wrong key");
    assert_eq!(error.code(), ErrorCode::UnauthorizedPeer);
    assert!(cli.write().is_err());
    assert!(cli.read(&message).is_err());
    assert!(cli.finish().is_err());
}

/// Checks that another application's prologue cannot be accepted with the same secret.
#[test]
fn different_protocol_context_fails_proof() {
    let mut peer = snow::Builder::new(
        "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
            .parse()
            .expect("fixed pattern"),
    )
    .prologue(b"synthetic-other-application")
    .expect("prologue")
    .psk(0, &[42; 32])
    .expect("PSK")
    .build_initiator()
    .expect("other peer");
    let mut message = [0; 256];
    let length = peer.write_message(&[], &mut message).expect("other hello");
    let mut cli = PairingHandshake::new(Role::Cli, key()).expect("CLI");
    assert!(cli.read(&message[..length]).is_err());
    assert!(cli.finish().is_err());
}

/// Checks that even an authenticated peer cannot send application data during pairing.
#[test]
fn handshake_payload_is_rejected() {
    let secret = Zeroizing::new([42; 32]);
    let mut peer = snow::Builder::new(
        "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
            .parse()
            .expect("fixed pattern"),
    )
    .prologue(b"beyond-gate:host-cli:pairing-v1")
    .expect("prologue")
    .psk(0, &secret)
    .expect("PSK")
    .build_initiator()
    .expect("peer");
    let mut message = [0; 256];
    let length = peer
        .write_message(b"synthetic-early-material", &mut message)
        .expect("payload");
    let mut cli = PairingHandshake::new(Role::Cli, key()).expect("CLI");
    assert_eq!(
        cli.read(&message[..length])
            .expect_err("early payload")
            .code(),
        ErrorCode::ProtocolError
    );
    assert!(cli.finish().is_err());
}

/// Checks exact secret and handshake bounds without secret-bearing diagnostics.
#[test]
fn invalid_key_and_handshake_bounds_fail() {
    for length in [0, 1, 31, 33, 8192] {
        assert!(PairingHandshake::new(Role::Host, Zeroizing::new(vec![42; length])).is_err());
    }
    for message in [vec![], vec![0; 257]] {
        let mut cli = PairingHandshake::new(Role::Cli, key()).expect("CLI");
        assert_eq!(
            cli.read(&message).expect_err("invalid bound").code(),
            ErrorCode::ProtocolError
        );
        assert!(cli.finish().is_err());
    }
}

/// Checks tampering and replay rejection with permanent channel closure.
#[test]
fn tampered_or_replayed_message_closes_channel() {
    for tamper in [false, true] {
        let (mut host, mut cli) = channels();
        let mut ciphertext = host
            .seal(Zeroizing::new(b"synthetic-material".to_vec()))
            .expect("seal");
        if tamper {
            ciphertext[0] ^= 1;
        } else {
            cli.open(&ciphertext).expect("first receipt");
        }
        assert_eq!(
            cli.open(&ciphertext).expect_err("tamper or replay").code(),
            ErrorCode::UnauthorizedPeer
        );
        assert!(cli.open(&ciphertext).is_err());
        assert!(
            cli.seal(Zeroizing::new(b"synthetic-late-reply".to_vec()))
                .is_err()
        );
    }
}

/// Checks ordered delivery and isolation from another handshake using the same synthetic PSK.
#[test]
fn reordered_and_cross_channel_messages_fail() {
    let (mut host, mut cli) = channels();
    let _first = host.seal(Zeroizing::new(vec![1])).expect("first");
    let second = host.seal(Zeroizing::new(vec![2])).expect("second");
    assert!(cli.open(&second).is_err());
    let (mut another_host, mut another_cli) = channels();
    let another = another_host.seal(Zeroizing::new(vec![3])).expect("other");
    assert!(host.open(&another).is_err());
    assert!(another_cli.open(&second).is_err());
}

/// Checks exact plaintext/ciphertext bounds and poisoning after oversized input.
#[test]
fn encrypted_payload_bounds_include_tag() {
    let (mut host, mut cli) = channels();
    let plaintext = Zeroizing::new(vec![7; 16384]);
    let ciphertext = host.seal(plaintext).expect("exact plaintext limit");
    assert_eq!(ciphertext.len(), 16400);
    assert_eq!(
        cli.open(&ciphertext).expect("exact ciphertext limit").len(),
        16384
    );
    assert!(host.seal(Zeroizing::new(vec![7; 16385])).is_err());
    assert!(host.seal(Zeroizing::new(vec![7])).is_err());
    for length in [0, 16, 16401] {
        let (_, mut receiver) = channels();
        assert!(receiver.open(&vec![0; length]).is_err());
        assert!(receiver.open(&ciphertext).is_err());
    }
}

/// Checks that cryptographic causes cannot expose test material through error chains.
#[test]
fn cryptographic_error_chain_is_redacted() {
    use std::error::Error;

    let (_, mut cli) = channels();
    let error = cli
        .open(b"synthetic-private-cookie-not-ciphertext")
        .expect_err("invalid proof");
    let mut current: Option<&dyn Error> = Some(&error);
    let mut count = 0;
    while let Some(error) = current {
        assert!(!error.to_string().contains("synthetic-private"));
        assert!(!format!("{error:?}").contains("synthetic-private"));
        count += 1;
        current = error.source();
    }
    assert_eq!(count, 2);
}

/// Checks that one owned ticket yields an authenticating handshake, not authorization by itself.
#[tokio::test(start_paused = true)]
async fn pending_ticket_claim_requires_peer_proof() {
    let ticket = PendingPairing::new(key(), Instant::now() + Duration::from_secs(300))
        .expect("pending ticket");
    let mut cli = ticket.claim().expect("single consuming claim");
    let mut host =
        PairingHandshake::new(Role::Host, Zeroizing::new(vec![43; 32])).expect("unpaired host");
    let error = cli
        .read(&host.write().expect("hello"))
        .expect_err("claim still needs proof");
    assert_eq!(error.code(), ErrorCode::UnauthorizedPeer);
    assert!(cli.finish().is_err());
}

/// Checks that a pending claim is accepted before, but not at, its monotonic deadline.
#[tokio::test(start_paused = true)]
async fn pending_ticket_deadline_is_exact() {
    let deadline = Instant::now() + Duration::from_secs(1);
    let before = PendingPairing::new(key(), deadline).expect("first ticket");
    let at = PendingPairing::new(key(), deadline).expect("second isolated test ticket");
    tokio::time::advance(Duration::from_millis(999)).await;
    assert!(before.claim().is_ok());
    tokio::time::advance(Duration::from_millis(1)).await;
    let error = at.claim().err().expect("expired claim");
    assert_eq!(error.code(), ErrorCode::TimedOut);
}

/// Checks that ticket creation rejects elapsed or excess lifetimes and malformed secret input.
#[tokio::test(start_paused = true)]
async fn pending_ticket_input_is_bounded() {
    let now = Instant::now();
    for deadline in [now, now - Duration::from_secs(1)] {
        let error = PendingPairing::new(key(), deadline)
            .err()
            .expect("elapsed deadline");
        assert_eq!(error.code(), ErrorCode::TimedOut);
    }
    assert!(PendingPairing::new(key(), now + Duration::from_secs(301)).is_err());
    assert!(
        PendingPairing::new(Zeroizing::new(vec![0; 31]), now + Duration::from_secs(1)).is_err()
    );
}
