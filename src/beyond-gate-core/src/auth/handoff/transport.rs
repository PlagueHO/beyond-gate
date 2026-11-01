//! Bounded encrypted host-to-CLI messages over one owned local byte stream.
//!
//! The binary envelope is `BGP1`, a one-byte kind, and a little-endian u32 length.
//! Kind 1 carries a Noise handshake; kind 2 carries encrypted `bg-host-cli` JSON.
//! A handshake has one message in each direction and cannot carry application data.
//! Ciphertext is bounded at 16,400 bytes, including its 16-byte authentication tag.
//!
//! The caller must select a local, OS-isolated transport and deliver the pairing
//! key through user action. PSK possession is not executable or browser attestation.
//! This module performs no endpoint discovery, cookie query, validation, or save.
//! Both compiled profiles are supported on Windows; neither grants account access.
//!
//! Failure or cancellation drops the owned active stream and cryptographic state.
//! Callers must use streams whose drop closes the connection; blocking standard
//! handles do not inherit a cancellation guarantee from these generic APIs.

use std::time::Duration;

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    time::{Instant, timeout_at},
};
use zeroize::Zeroizing;

use super::{
    ErrorCode, HandoffError, MAX_FRAME_BYTES, MAX_FRAMES, Message, Protocol,
    framing::{MAX_WAIT, before_deadline},
    message::parse_payload,
    pairing::{
        MAX_HANDSHAKE_BYTES, MAX_PENDING_TIME, PairedChannel, PairingHandshake, PendingPairing,
        Role, TAG_BYTES,
    },
};

/// Fixes the outer version without allowing unencrypted JSON as a fallback.
const MAGIC: &[u8; 4] = b"BGP1";
/// Caps the entire handshake, not each header or payload separately.
const HANDSHAKE_WAIT: Duration = Duration::from_secs(10);

/// Separates handshake and encrypted messages without inspecting plaintext.
#[derive(Clone, Copy)]
pub(super) enum Kind {
    Handshake = 1,
    Encrypted = 2,
}

impl Kind {
    /// Rejects the declared size before allocating an envelope payload.
    fn validate_length(self, length: usize) -> Result<(), HandoffError> {
        let valid = match self {
            Self::Handshake => (1..=MAX_HANDSHAKE_BYTES).contains(&length),
            Self::Encrypted => (TAG_BYTES + 1..=MAX_FRAME_BYTES + TAG_BYTES).contains(&length),
        };
        if valid {
            Ok(())
        } else {
            Err(HandoffError::new(ErrorCode::ProtocolError))
        }
    }
}

/// Reads one bounded envelope under the enclosing non-resetting operation deadline.
pub(super) async fn read_envelope<S: AsyncRead + Unpin>(
    stream: &mut S,
    kind: Kind,
    deadline: Instant,
) -> Result<Zeroizing<Vec<u8>>, HandoffError> {
    let mut header = [0; 9];
    stream
        .read_exact(&mut header)
        .await
        .map_err(HandoffError::io)?;
    before_deadline(deadline)?;
    if &header[..4] != MAGIC {
        let code = if &header[..3] == b"BGP" {
            ErrorCode::VersionMismatch
        } else {
            ErrorCode::ProtocolError
        };
        return Err(HandoffError::new(code));
    }
    if header[4] != kind as u8 {
        return Err(HandoffError::new(ErrorCode::ProtocolError));
    }
    let length = u32::from_le_bytes([header[5], header[6], header[7], header[8]]) as usize;
    kind.validate_length(length)?;
    let mut payload = Zeroizing::new(vec![0; length]);
    stream
        .read_exact(&mut payload)
        .await
        .map_err(HandoffError::io)?;
    before_deadline(deadline)?;
    Ok(payload)
}

/// Writes and flushes only an already bounded handshake or encrypted payload.
pub(super) async fn write_envelope<S: AsyncWrite + Unpin>(
    stream: &mut S,
    kind: Kind,
    payload: &[u8],
    deadline: Instant,
) -> Result<(), HandoffError> {
    kind.validate_length(payload.len())?;
    before_deadline(deadline)?;
    let mut header = [0; 9];
    header[..4].copy_from_slice(MAGIC);
    header[4] = kind as u8;
    header[5..].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    stream.write_all(&header).await.map_err(HandoffError::io)?;
    before_deadline(deadline)?;
    stream.write_all(payload).await.map_err(HandoffError::io)?;
    before_deadline(deadline)?;
    stream.flush().await.map_err(HandoffError::io)?;
    before_deadline(deadline)
}

/// Completes the two empty Noise messages in their fixed role order.
async fn handshake<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    mut state: PairingHandshake,
    role: Role,
    deadline: Instant,
) -> Result<PairedChannel, HandoffError> {
    before_deadline(deadline)?;
    timeout_at(deadline, async {
        if role == Role::Host {
            let message = state.write()?;
            write_envelope(stream, Kind::Handshake, &message, deadline).await?;
        }
        let message = read_envelope(stream, Kind::Handshake, deadline).await?;
        state.read(&message)?;
        before_deadline(deadline)?;
        if role == Role::Cli {
            let message = state.write()?;
            write_envelope(stream, Kind::Handshake, &message, deadline).await?;
        }
        state.finish()
    })
    .await
    .map_err(|_| HandoffError::new(ErrorCode::TimedOut))?
    .and_then(|channel| {
        before_deadline(deadline)?;
        Ok(channel)
    })
}

/// Keeps the stream and keys together so cancellation drops both.
struct Active<S> {
    stream: S,
    channel: PairedChannel,
}

/// Exchanges at most eight ordered application messages per direction.
///
/// All operations use the smaller of the original pending deadline, supplied
/// phase deadline, and a 15-second operation cap. Started failure or cancellation
/// closes both directions. A parsed message proves neither a login transition
/// nor service validation or protected save.
pub struct PairedConnection<S> {
    active: Option<Active<S>>,
    deadline: Instant,
    reads: u8,
    writes: u8,
}

impl<S: AsyncRead + AsyncWrite + Unpin> PairedConnection<S> {
    /// Consumes a bound connection with room for two final reads and two final writes.
    #[cfg(windows)]
    pub(super) fn into_completion(self) -> Result<(S, PairedChannel, Instant), HandoffError> {
        if self.reads > MAX_FRAMES - 2 || self.writes > MAX_FRAMES - 2 {
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        let active = self
            .active
            .ok_or_else(|| HandoffError::new(ErrorCode::Disconnected))?;
        Ok((active.stream, active.channel, self.deadline))
    }

    /// Authenticates a host stream using the user-delivered pairing key.
    ///
    /// Owns the stream and key before the future is polled. There is no reconnect,
    /// protocol fallback, key discovery, or endpoint selection in this operation.
    ///
    /// # Errors
    ///
    /// Returns a safe proof, framing, or deadline error and drops the owned stream.
    pub async fn connect(
        mut stream: S,
        secret: Zeroizing<Vec<u8>>,
        deadline: Instant,
    ) -> Result<Self, HandoffError> {
        let deadline = deadline.min(Instant::now() + MAX_PENDING_TIME);
        before_deadline(deadline)?;
        let phase = deadline.min(Instant::now() + HANDSHAKE_WAIT);
        let state = PairingHandshake::new(Role::Host, secret)?;
        let channel = handshake(&mut stream, state, Role::Host, phase).await?;
        Ok(Self::new(stream, channel, deadline))
    }

    /// Consumes one pending CLI ticket and authenticates the paired host stream.
    ///
    /// The ticket's own deadline is authoritative; callers cannot extend it.
    /// The ticket is consumed on every result, including cancellation.
    ///
    /// # Errors
    ///
    /// Returns a safe proof, framing, or deadline error and drops the owned stream.
    pub async fn accept(mut stream: S, ticket: PendingPairing) -> Result<Self, HandoffError> {
        let deadline = ticket.deadline();
        let phase = deadline.min(Instant::now() + HANDSHAKE_WAIT);
        let state = ticket.claim()?;
        let channel = handshake(&mut stream, state, Role::Cli, phase).await?;
        Ok(Self::new(stream, channel, deadline))
    }

    /// Starts application counters only after the handshake completed successfully.
    fn new(stream: S, channel: PairedChannel, deadline: Instant) -> Self {
        Self {
            active: Some(Active { stream, channel }),
            deadline,
            reads: 0,
            writes: 0,
        }
    }

    /// Authenticates, decrypts, and parses one bounded host-to-CLI message.
    ///
    /// # Errors
    ///
    /// Returns fixed errors for invalid, exhausted, incomplete, modified, replayed,
    /// cancelled, or late messages. Failed connections cannot be reused.
    pub async fn read_message(&mut self, deadline: Instant) -> Result<Message, HandoffError> {
        self.read_with_budget(deadline, MAX_WAIT).await
    }

    /// Waits across the fixed validation/save/result phases while retaining frame bounds.
    ///
    /// # Errors
    ///
    /// Returns the same consuming failures as `read_message`, with a 25-second cap.
    pub async fn read_completion(&mut self, deadline: Instant) -> Result<Message, HandoffError> {
        self.read_with_budget(deadline, Duration::from_secs(25))
            .await
    }

    /// Applies a compiled phase budget without resetting the pending lifetime.
    async fn read_with_budget(
        &mut self,
        deadline: Instant,
        budget: Duration,
    ) -> Result<Message, HandoffError> {
        let mut active = self
            .active
            .take()
            .ok_or_else(|| HandoffError::new(ErrorCode::Disconnected))?;
        if self.reads >= MAX_FRAMES {
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        self.reads += 1;
        let deadline = deadline.min(self.deadline).min(Instant::now() + budget);
        before_deadline(deadline)?;
        let ciphertext = timeout_at(
            deadline,
            read_envelope(&mut active.stream, Kind::Encrypted, deadline),
        )
        .await
        .map_err(|_| HandoffError::new(ErrorCode::TimedOut))??;
        let plaintext = active.channel.open(&ciphertext)?;
        let message = parse_payload(Protocol::HostCli, &plaintext)?;
        before_deadline(deadline)?;
        self.active = Some(active);
        Ok(message)
    }

    /// Validates and encrypts an owned payload before any byte is written.
    ///
    /// A valid result message still requires operation proof supplied by the
    /// application. No parsed or encrypted result can create that proof.
    ///
    /// # Errors
    ///
    /// Returns fixed shape, size, deadline, or I/O errors. Failure or started
    /// cancellation drops the stream and keys without retry or late success.
    pub async fn write_message(
        &mut self,
        payload: Zeroizing<Vec<u8>>,
        deadline: Instant,
    ) -> Result<(), HandoffError> {
        let mut active = self
            .active
            .take()
            .ok_or_else(|| HandoffError::new(ErrorCode::Disconnected))?;
        if self.writes >= MAX_FRAMES {
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        self.writes += 1;
        let deadline = deadline.min(self.deadline).min(Instant::now() + MAX_WAIT);
        before_deadline(deadline)?;
        drop(parse_payload(Protocol::HostCli, &payload)?);
        before_deadline(deadline)?;
        let ciphertext = active.channel.seal(payload)?;
        timeout_at(
            deadline,
            write_envelope(&mut active.stream, Kind::Encrypted, &ciphertext, deadline),
        )
        .await
        .map_err(|_| HandoffError::new(ErrorCode::TimedOut))??;
        before_deadline(deadline)?;
        self.active = Some(active);
        Ok(())
    }
}
