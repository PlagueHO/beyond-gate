//! Bounded async frames that become unusable after failure or cancellation.

use std::time::Duration;

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    time::{Instant, timeout_at},
};
use zeroize::Zeroizing;

use super::{
    ErrorCode, HandoffError, MAX_FRAME_BYTES, MAX_FRAMES, Message, Protocol, message::parse_payload,
};

/// Caps one frame operation at the proposal's longest individual phase budget.
pub(super) const MAX_WAIT: Duration = Duration::from_secs(15);

/// Rejects late progress even when an I/O future is immediately ready.
pub(super) fn before_deadline(deadline: Instant) -> Result<(), HandoffError> {
    if Instant::now() >= deadline {
        Err(HandoffError::new(ErrorCode::TimedOut))
    } else {
        Ok(())
    }
}

/// Reads one protocol's bounded frames without authenticating the transport.
///
/// Callers must supply the authoritative phase deadline and close the transport
/// after any failure. Cancellation poisons this reader; no resynchronization,
/// reconnect, or replay occurs.
pub struct FrameReader<R> {
    reader: R,
    protocol: Protocol,
    frames: u8,
    closed: bool,
}

impl<R: AsyncRead + Unpin> FrameReader<R> {
    /// Selects a fixed protocol and an eight-frame inbound budget.
    pub fn new(reader: R, protocol: Protocol) -> Self {
        Self {
            reader,
            protocol,
            frames: 0,
            closed: false,
        }
    }

    /// Reads a prefix and payload under one non-resetting monotonic deadline.
    ///
    /// The deadline must be the smaller of the pending deadline and phase
    /// budget. This layer additionally caps a frame wait at 15 seconds.
    ///
    /// # Errors
    ///
    /// Returns a fixed error for late, partial, oversized, invalid, exhausted,
    /// or cancelled frames. A failed reader cannot be reused.
    pub async fn read_message(&mut self, deadline: Instant) -> Result<Message, HandoffError> {
        self.read_with_budget(deadline, MAX_WAIT).await
    }

    /// Monitors cancellation and final receipt across validation, save, and acknowledgement.
    ///
    /// The 25-second cap covers the fixed 15/5/5-second completion phases.
    /// Callers must keep this future alive across result delivery so cancelling
    /// a partial read cannot discard bytes and then resume the same reader.
    ///
    /// # Errors
    ///
    /// Returns the same safe, poisoning failures and frame limits as `read_message`.
    pub async fn read_completion(&mut self, deadline: Instant) -> Result<Message, HandoffError> {
        self.read_with_budget(deadline, Duration::from_secs(25))
            .await
    }

    /// Uses only the two compiled wait budgets without changing frame ownership.
    async fn read_with_budget(
        &mut self,
        deadline: Instant,
        budget: Duration,
    ) -> Result<Message, HandoffError> {
        if self.closed || self.frames >= MAX_FRAMES {
            self.closed = true;
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        self.closed = true;
        self.frames += 1;
        let deadline = deadline.min(Instant::now() + budget);
        before_deadline(deadline)?;
        let result = timeout_at(deadline, self.read_one(deadline))
            .await
            .map_err(|_| HandoffError::new(ErrorCode::TimedOut))?;
        before_deadline(deadline)?;
        if result.is_ok() {
            self.closed = false;
        }
        result
    }

    /// Checks the declared bound before allocating an owned, zeroizing payload.
    async fn read_one(&mut self, deadline: Instant) -> Result<Message, HandoffError> {
        let mut prefix = [0_u8; 4];
        self.reader
            .read_exact(&mut prefix)
            .await
            .map_err(HandoffError::io)?;
        before_deadline(deadline)?;
        let length = match self.protocol {
            Protocol::Browser => u32::from_ne_bytes(prefix),
            Protocol::HostCli => u32::from_le_bytes(prefix),
        };
        if length == 0 || length > MAX_FRAME_BYTES as u32 {
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        let mut payload = Zeroizing::new(vec![0_u8; length as usize]);
        self.reader
            .read_exact(&mut payload)
            .await
            .map_err(HandoffError::io)?;
        before_deadline(deadline)?;
        parse_payload(self.protocol, &payload)
    }
}

/// Writes only validated frames with a separate eight-frame outbound budget.
///
/// A syntactically valid result still requires real operation proof before a
/// caller can send it. This type does not supply that proof.
pub struct FrameWriter<W> {
    writer: W,
    protocol: Protocol,
    frames: u8,
    closed: bool,
}

impl<W: AsyncWrite + Unpin> FrameWriter<W> {
    /// Selects a fixed protocol and an eight-frame outbound budget.
    pub fn new(writer: W, protocol: Protocol) -> Self {
        Self {
            writer,
            protocol,
            frames: 0,
            closed: false,
        }
    }

    /// Validates, writes, and flushes one owned zeroizing payload before its deadline.
    ///
    /// Ownership is already protected if this future is dropped before polling.
    /// The caller supplies the authoritative phase deadline; the additional
    /// per-frame cap is 15 seconds. Failed or cancelled writers cannot be reused.
    ///
    /// # Errors
    ///
    /// Returns fixed errors for invalid, exhausted, partial, failed, or late
    /// writes. No retry or late-success recovery is performed.
    pub async fn write_message(
        &mut self,
        payload: Zeroizing<Vec<u8>>,
        deadline: Instant,
    ) -> Result<(), HandoffError> {
        if self.closed || self.frames >= MAX_FRAMES {
            self.closed = true;
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        self.closed = true;
        self.frames += 1;
        let deadline = deadline.min(Instant::now() + MAX_WAIT);
        before_deadline(deadline)?;
        drop(parse_payload(self.protocol, &payload)?);
        before_deadline(deadline)?;
        let length = payload.len() as u32;
        let prefix = match self.protocol {
            Protocol::Browser => length.to_ne_bytes(),
            Protocol::HostCli => length.to_le_bytes(),
        };
        timeout_at(deadline, async {
            self.writer
                .write_all(&prefix)
                .await
                .map_err(HandoffError::io)?;
            before_deadline(deadline)?;
            self.writer
                .write_all(&payload)
                .await
                .map_err(HandoffError::io)?;
            before_deadline(deadline)?;
            self.writer.flush().await.map_err(HandoffError::io)
        })
        .await
        .map_err(|_| HandoffError::new(ErrorCode::TimedOut))??;
        before_deadline(deadline)?;
        self.closed = false;
        Ok(())
    }
}
