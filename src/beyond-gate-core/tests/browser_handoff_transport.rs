//! Exercises real Noise and encrypted envelopes without accounts or credential stores.

use std::{future::Future, task::Poll, time::Duration};

use beyond_gate_core::auth::handoff::{
    ErrorCode, Message,
    pairing::{PairedChannel, PairingHandshake, PendingPairing, Role},
    transport::PairedConnection,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, DuplexStream, duplex},
    time::Instant,
};
use zeroize::Zeroizing;

mod support;

/// Returns only a compiled invented key, never user or OS credential material.
fn key() -> Zeroizing<Vec<u8>> {
    Zeroizing::new(vec![42; 32])
}

/// Loads a canonical synthetic host-to-CLI message.
fn payload(name: &str) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(
        serde_json::to_vec(&support::fixture()["messages"][name]).expect("fixture payload"),
    )
}

/// Completes an actual paired connection over two bounded in-memory byte streams.
async fn pair(
    capacity: usize,
    deadline: Instant,
) -> (
    PairedConnection<DuplexStream>,
    PairedConnection<DuplexStream>,
) {
    let (host, cli) = duplex(capacity);
    let ticket = PendingPairing::new(key(), deadline).expect("ticket");
    let (host, cli) = tokio::join!(
        PairedConnection::connect(host, key(), deadline),
        PairedConnection::accept(cli, ticket)
    );
    (host.expect("paired host"), cli.expect("paired CLI"))
}

/// Writes an explicitly synthetic raw envelope to exercise adversarial framing.
async fn raw_write(stream: &mut DuplexStream, magic: &[u8; 4], kind: u8, bytes: &[u8]) {
    stream.write_all(magic).await.expect("raw magic");
    stream.write_all(&[kind]).await.expect("raw kind");
    stream
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .await
        .expect("raw length");
    stream.write_all(bytes).await.expect("raw payload");
}

/// Completes a controlled raw CLI handshake to test authenticated bad application input.
async fn raw_cli(stream: &mut DuplexStream) -> PairedChannel {
    let mut header = [0; 9];
    stream.read_exact(&mut header).await.expect("host header");
    assert_eq!(&header[..5], b"BGP1\x01");
    let length = u32::from_le_bytes(header[5..].try_into().expect("length")) as usize;
    assert!(length <= 256);
    let mut message = vec![0; length];
    stream
        .read_exact(&mut message)
        .await
        .expect("host handshake");
    let mut state = PairingHandshake::new(Role::Cli, key()).expect("raw CLI");
    state.read(&message).expect("host proof");
    raw_write(stream, b"BGP1", 1, &state.write().expect("CLI reply")).await;
    state.finish().expect("raw encrypted channel")
}

/// Establishes a host against a raw but genuinely authenticated synthetic peer.
async fn raw_pair() -> (PairedConnection<DuplexStream>, DuplexStream, PairedChannel) {
    let (host, mut cli) = duplex(32768);
    let deadline = Instant::now() + Duration::from_secs(5);
    let (host, channel) = tokio::join!(
        PairedConnection::connect(host, key(), deadline),
        raw_cli(&mut cli)
    );
    (host.expect("paired host"), cli, channel)
}

/// Polls a started operation once so dropping its future tests actual cancellation.
async fn start_pending<F: Future>(future: std::pin::Pin<&mut F>) {
    let mut future = future;
    std::future::poll_fn(|context| {
        assert!(future.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
}

/// Checks canonical application messages in both directions after mutual PSK proof.
#[tokio::test]
async fn encrypted_messages_round_trip() {
    let deadline = Instant::now() + Duration::from_secs(5);
    let (mut host, mut cli) = pair(4096, deadline).await;
    host.write_message(payload("cli_open"), deadline)
        .await
        .expect("open");
    assert!(matches!(
        cli.read_message(deadline).await.expect("read open"),
        Message::Initiate { .. }
    ));
    cli.write_message(payload("cli_error"), deadline)
        .await
        .expect("safe error");
    assert!(matches!(
        host.read_message(deadline).await.expect("read error"),
        Message::Error {
            code: ErrorCode::SessionRejected,
            ..
        }
    ));
}

/// Checks that the wrong pairing key closes both real stream endpoints without authorization.
#[tokio::test]
async fn wrong_key_rejects_connection() {
    let deadline = Instant::now() + Duration::from_secs(5);
    let (host, cli) = duplex(4096);
    let ticket = PendingPairing::new(key(), deadline).expect("ticket");
    let (host, cli) = tokio::join!(
        PairedConnection::connect(host, Zeroizing::new(vec![43; 32]), deadline),
        PairedConnection::accept(cli, ticket)
    );
    assert!(host.is_err());
    assert_eq!(
        cli.err().expect("wrong proof").code(),
        ErrorCode::UnauthorizedPeer
    );
}

/// Checks the whole-handshake cap and closure when a peer never replies.
#[tokio::test(start_paused = true)]
async fn stalled_handshake_has_one_deadline() {
    let started = Instant::now();
    let (host, mut peer) = duplex(4096);
    let result = PairedConnection::connect(host, key(), started + Duration::from_secs(300)).await;
    assert_eq!(result.err().expect("timeout").code(), ErrorCode::TimedOut);
    assert_eq!(Instant::now() - started, Duration::from_secs(10));
    let mut bytes = Vec::new();
    peer.read_to_end(&mut bytes).await.expect("closed host");
    assert!(bytes.starts_with(b"BGP1\x01"));
}

/// Checks that application callers cannot extend an accepted ticket's pending deadline.
#[tokio::test(start_paused = true)]
async fn pending_deadline_rejects_ready_late_messages() {
    let deadline = Instant::now() + Duration::from_secs(1);
    let (mut host, mut cli) = pair(4096, deadline).await;
    cli.write_message(payload("cli_error"), deadline)
        .await
        .expect("queued before expiry");
    tokio::time::advance(Duration::from_secs(1)).await;
    let error = host
        .read_message(deadline + Duration::from_secs(100))
        .await
        .expect_err("late ready input");
    assert_eq!(error.code(), ErrorCode::TimedOut);
    assert!(
        host.write_message(payload("cli_open"), deadline + Duration::from_secs(100))
            .await
            .is_err()
    );
}

/// Checks independent eight-message budgets and rejection of the ninth operation.
#[tokio::test]
async fn frame_budgets_are_independent() {
    let deadline = Instant::now() + Duration::from_secs(5);
    let (mut host, mut cli) = pair(4096, deadline).await;
    for _ in 0..8 {
        host.write_message(payload("cli_open"), deadline)
            .await
            .expect("host frame");
        cli.read_message(deadline).await.expect("CLI frame");
        cli.write_message(payload("cli_error"), deadline)
            .await
            .expect("CLI reply");
        host.read_message(deadline).await.expect("host reply");
    }
    assert_eq!(
        host.write_message(payload("cli_open"), deadline)
            .await
            .expect_err("ninth write")
            .code(),
        ErrorCode::ProtocolError
    );
    assert_eq!(
        cli.read_message(deadline)
            .await
            .expect_err("ninth read")
            .code(),
        ErrorCode::ProtocolError
    );
}

/// Checks that shape failure sends no invalid plaintext and prevents reuse in either direction.
#[tokio::test]
async fn invalid_application_payload_closes_connection() {
    let deadline = Instant::now() + Duration::from_secs(5);
    let (mut host, mut cli) = pair(4096, deadline).await;
    assert!(
        host.write_message(
            Zeroizing::new(b"synthetic-private-invalid-json".to_vec()),
            deadline
        )
        .await
        .is_err()
    );
    assert!(cli.read_message(deadline).await.is_err());
    assert_eq!(
        host.read_message(deadline)
            .await
            .expect_err("closed host")
            .code(),
        ErrorCode::Disconnected
    );
}

/// Checks that cancellation of a started read drops the stream and blocks later writes.
#[tokio::test]
async fn cancelled_read_closes_both_directions() {
    let deadline = Instant::now() + Duration::from_secs(5);
    let (mut host, mut cli) = pair(4096, deadline).await;
    let mut future = Box::pin(host.read_message(deadline));
    start_pending(future.as_mut()).await;
    drop(future);
    assert!(cli.read_message(deadline).await.is_err());
    assert_eq!(
        host.write_message(payload("cli_open"), deadline)
            .await
            .expect_err("cancelled")
            .code(),
        ErrorCode::Disconnected
    );
}

/// Checks closure after cancellation of a partial, blocked encrypted write.
#[tokio::test]
async fn cancelled_partial_write_cannot_resume() {
    let deadline = Instant::now() + Duration::from_secs(5);
    let (mut host, mut cli) = pair(1, deadline).await;
    let mut future = Box::pin(host.write_message(payload("cli_open"), deadline));
    start_pending(future.as_mut()).await;
    drop(future);
    assert!(cli.read_message(deadline).await.is_err());
    assert!(host.read_message(deadline).await.is_err());
}

/// Checks version, kind, and declared-size rejection before payload allocation or parsing.
#[tokio::test]
async fn invalid_envelopes_are_rejected() {
    for (magic, kind, length, expected) in [
        (*b"BGP2", 2, 17_u32, ErrorCode::VersionMismatch),
        (*b"JSON", 2, 17, ErrorCode::ProtocolError),
        (*b"BGP1", 1, 48, ErrorCode::ProtocolError),
        (*b"BGP1", 2, 16, ErrorCode::ProtocolError),
        (*b"BGP1", 2, 16401, ErrorCode::ProtocolError),
    ] {
        let (mut host, mut peer, _) = raw_pair().await;
        let mut header = Vec::from(magic);
        header.push(kind);
        header.extend_from_slice(&length.to_le_bytes());
        peer.write_all(&header)
            .await
            .expect("invalid envelope header");
        let deadline = Instant::now() + Duration::from_secs(5);
        let error = host.read_message(deadline).await.expect_err("bad header");
        assert_eq!(error.code(), expected);
        assert!(host.read_message(deadline).await.is_err());
    }
}

/// Checks exact plaintext size, including JSON whitespace, with the outer authentication tag.
#[tokio::test]
async fn encrypted_payload_limit_is_exact() {
    let deadline = Instant::now() + Duration::from_secs(5);
    let (mut host, mut cli) = pair(32768, deadline).await;
    let mut bytes = payload("cli_open");
    bytes.resize(16384, b' ');
    host.write_message(bytes, deadline)
        .await
        .expect("exact limit");
    cli.read_message(deadline).await.expect("exact limit read");
    let mut bytes = payload("cli_open");
    bytes.resize(16385, b' ');
    assert!(host.write_message(bytes, deadline).await.is_err());
}

/// Checks authenticated browser-protocol input, tampering, and replay on actual stream frames.
#[tokio::test]
async fn authenticated_bad_input_and_replay_fail_closed() {
    for mode in ["browser-protocol", "tamper", "replay"] {
        let (mut host, mut peer, mut channel) = raw_pair().await;
        let bytes = payload(if mode == "browser-protocol" {
            "browser_hello"
        } else {
            "cli_error"
        });
        let mut ciphertext = channel.seal(bytes).expect("synthetic ciphertext");
        if mode == "tamper" {
            ciphertext[0] ^= 1;
        }
        raw_write(&mut peer, b"BGP1", 2, &ciphertext).await;
        let deadline = Instant::now() + Duration::from_secs(5);
        if mode == "replay" {
            host.read_message(deadline).await.expect("first receipt");
            raw_write(&mut peer, b"BGP1", 2, &ciphertext).await;
        }
        assert!(host.read_message(deadline).await.is_err());
        assert!(
            host.write_message(payload("cli_open"), deadline)
                .await
                .is_err()
        );
    }
}

/// Checks one non-resetting application deadline across partial envelope bytes.
#[tokio::test(start_paused = true)]
async fn incomplete_envelope_times_out_and_drops_stream() {
    let (mut host, mut peer, _) = raw_pair().await;
    peer.write_all(b"BGP1").await.expect("partial header");
    let deadline = Instant::now() + Duration::from_secs(1);
    let error = host
        .read_message(deadline)
        .await
        .expect_err("partial timeout");
    assert_eq!(error.code(), ErrorCode::TimedOut);
    let mut bytes = Vec::new();
    peer.read_to_end(&mut bytes)
        .await
        .expect("closed after timeout");
    assert!(bytes.is_empty());
}

/// Checks that cancelling an owned host handshake closes its stream without application data.
#[tokio::test]
async fn cancelled_host_handshake_drops_owned_stream() {
    let deadline = Instant::now() + Duration::from_secs(5);
    let (host, mut peer) = duplex(4096);
    let mut future = Box::pin(PairedConnection::connect(host, key(), deadline));
    start_pending(future.as_mut()).await;
    drop(future);
    let mut bytes = Vec::new();
    peer.read_to_end(&mut bytes)
        .await
        .expect("cancelled host EOF");
    assert_eq!(&bytes[..5], b"BGP1\x01");
    let length = u32::from_le_bytes(bytes[5..9].try_into().expect("header")) as usize;
    assert_eq!(bytes.len(), 9 + length);
}

/// Checks that cancelling CLI acceptance consumes its ticket and sends no authorization.
#[tokio::test]
async fn cancelled_cli_handshake_sends_no_authorization() {
    let deadline = Instant::now() + Duration::from_secs(5);
    let (cli, mut peer) = duplex(4096);
    let ticket = PendingPairing::new(key(), deadline).expect("ticket");
    let mut future = Box::pin(PairedConnection::accept(cli, ticket));
    start_pending(future.as_mut()).await;
    drop(future);
    let mut bytes = Vec::new();
    peer.read_to_end(&mut bytes)
        .await
        .expect("cancelled CLI EOF");
    assert!(bytes.is_empty());
}

/// Checks that an expired ticket cannot begin a handshake or emit any bytes.
#[tokio::test(start_paused = true)]
async fn expired_cli_ticket_closes_before_handshake() {
    let deadline = Instant::now() + Duration::from_secs(1);
    let (cli, mut peer) = duplex(4096);
    let ticket = PendingPairing::new(key(), deadline).expect("ticket");
    tokio::time::advance(Duration::from_secs(1)).await;
    let result = PairedConnection::accept(cli, ticket).await;
    assert_eq!(
        result.err().expect("expired ticket").code(),
        ErrorCode::TimedOut
    );
    let mut bytes = Vec::new();
    peer.read_to_end(&mut bytes).await.expect("expired CLI EOF");
    assert!(bytes.is_empty());
}

/// Checks one deadline across a valid header followed by a stalled partial ciphertext.
#[tokio::test(start_paused = true)]
async fn partial_ciphertext_cannot_reset_deadline() {
    let (mut host, mut peer, mut channel) = raw_pair().await;
    let ciphertext = channel.seal(payload("cli_error")).expect("ciphertext");
    let mut header = Vec::from(*b"BGP1");
    header.push(2);
    header.extend_from_slice(&(ciphertext.len() as u32).to_le_bytes());
    peer.write_all(&header).await.expect("complete header");
    peer.write_all(&ciphertext[..1])
        .await
        .expect("partial ciphertext");
    let started = Instant::now();
    let error = host
        .read_message(started + Duration::from_secs(1))
        .await
        .expect_err("stalled body");
    assert_eq!(error.code(), ErrorCode::TimedOut);
    assert_eq!(Instant::now() - started, Duration::from_secs(1));
}
