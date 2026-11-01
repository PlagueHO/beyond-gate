//! Encrypted channel primitive for the user-approved one-use pairing direction.
//!
//! Noise authenticates possession of a shared 256-bit pairing secret, not an
//! executable, OS logon, or browser launch. The eventual interactive controller
//! must generate the secret with an OS CSPRNG, deliver it only through explicit
//! user pairing, enforce one pending slot and one claim, and clear it on expiry.
//! This module does not implement that controller, secret delivery, IPC, or login.
//!
//! Handshakes carry no application payload. Only completed handshakes can create
//! transport objects. Any failed operation consumes the cryptographic state.
//! Owned secret/plaintext buffers are zeroizing; dependency-internal copies are
//! outside this ownership guarantee.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use snow::{Builder, HandshakeState, TransportState};
use std::{error::Error, fmt, time::Duration};
use tokio::time::Instant;
use zeroize::Zeroizing;

use super::{ErrorCode, HandoffError, MAX_FRAME_BYTES};

/// Fixes one protocol; peer input cannot select weaker algorithms or a PSK-free pattern.
const PATTERN: &str = "Noise_NNpsk0_25519_ChaChaPoly_SHA256";
/// Separates this channel from other applications and future protocol versions.
const PROLOGUE: &[u8] = b"beyond-gate:host-cli:pairing-v1";
/// Allows only the two short handshake messages used by this fixed pattern.
pub(super) const MAX_HANDSHAKE_BYTES: usize = 256;
/// Accounts for the ChaChaPoly authentication tag outside the JSON payload bound.
pub(super) const TAG_BYTES: usize = 16;
/// Bounds an issued pairing ticket to the proposed five-minute pending-login lifetime.
pub(super) const MAX_PENDING_TIME: Duration = Duration::from_secs(300);

/// Checks one authoritative future deadline without a second clock read or extension.
pub(crate) fn check_pending_deadline(deadline: Instant) -> Result<(), HandoffError> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| HandoffError::new(ErrorCode::TimedOut))?;
    if remaining > MAX_PENDING_TIME {
        return Err(HandoffError::new(ErrorCode::ProtocolError));
    }
    Ok(())
}

/// Retains a typed randomness failure without exposing dependency or OS details.
pub struct RandomnessError(getrandom::Error);

impl fmt::Display for RandomnessError {
    /// Reports only whether the failure came from the OS randomness provider.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(if self.0.raw_os_error().is_some() {
            "OS secure randomness failed"
        } else {
            "secure randomness is unavailable"
        })
    }
}

impl fmt::Debug for RandomnessError {
    /// Uses the same safe classification as Display.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for RandomnessError {}

/// Fills owned bytes from the system source without a fallback or partial success.
pub(super) fn fill_random(bytes: &mut [u8]) -> Result<(), RandomnessError> {
    getrandom::fill(bytes).map_err(RandomnessError)
}

/// Owns a generated 256-bit pairing key with redacted diagnostics and no public clone.
pub struct PairingKey(Zeroizing<Vec<u8>>);

impl PairingKey {
    /// Consumes the canonical 43-character key entered in the trusted extension.
    ///
    /// Syntax and size do not prove entropy or authorization. The waiting CLI
    /// must generate the matching secret and authenticate its possession.
    ///
    /// # Errors
    ///
    /// Returns a redacted material or encoding failure; no alternate encoding,
    /// whitespace, padding, or user-chosen password is accepted as a fallback.
    pub fn from_encoded(value: Zeroizing<String>) -> Result<Self, HandoffError> {
        if value.len() != 43 {
            return Err(HandoffError::new(ErrorCode::InvalidMaterial));
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(value.as_bytes())
            .map(Zeroizing::new)
            .map_err(HandoffError::encoding)?;
        if bytes.len() != 32 {
            return Err(HandoffError::new(ErrorCode::InvalidMaterial));
        }
        Ok(Self(bytes))
    }

    /// Generates exactly 32 bytes from the system's cryptographic randomness source.
    ///
    /// This creates material only, not user delivery or application authorization.
    /// The OS call can block; an async timer cannot interrupt it.
    ///
    /// # Errors
    ///
    /// Returns a redacted typed failure and clears partial output without fallback.
    pub fn generate() -> Result<Self, RandomnessError> {
        let mut bytes = Zeroizing::new(vec![0; 32]);
        fill_random(&mut bytes)?;
        Ok(Self(bytes))
    }

    /// Consumes the key for the existing handshake or a protected transfer surface.
    ///
    /// Do not print these bytes or place them in arguments, URLs, files, or logs.
    ///
    /// ```compile_fail
    /// use beyond_gate_core::auth::handoff::pairing::PairingKey;
    /// fn cannot_transfer_twice(key: PairingKey) {
    ///     let _first = key.into_bytes();
    ///     let _second = key.into_bytes();
    /// }
    /// ```
    pub fn into_bytes(self) -> Zeroizing<Vec<u8>> {
        self.0
    }
}

impl fmt::Debug for PairingKey {
    /// Never includes generated material in diagnostic output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PairingKey { value: [REDACTED] }")
    }
}

#[cfg(test)]
mod randomness_tests {
    use super::*;

    /// Checks that unsupported randomness is a typed failure with a safe source boundary.
    #[test]
    fn randomness_failure_is_redacted() {
        let error = RandomnessError(getrandom::Error::UNSUPPORTED);
        assert_eq!(error.to_string(), "secure randomness is unavailable");
        assert_eq!(format!("{error:?}"), error.to_string());
        assert!(error.source().is_none());
    }

    /// Checks key size and diagnostic redaction without printing generated bytes.
    #[test]
    fn generated_key_has_exact_size_and_safe_debug() {
        let key = PairingKey::generate().expect("system randomness");
        assert_eq!(format!("{key:?}"), "PairingKey { value: [REDACTED] }");
        assert_eq!(key.into_bytes().len(), 32);
    }
}

/// Owns one expiring CLI pairing ticket; claiming consumes it even on failure.
///
/// The application controller must ensure only one ticket exists per OS logon,
/// deliver the matching host secret through user action, and drop unclaimed tickets
/// when its authoritative timer expires. This object performs no process discovery.
pub struct PendingPairing {
    /// Clears the owned waiting secret when cancelled by dropping this ticket.
    secret: Zeroizing<Vec<u8>>,
    /// Uses monotonic time rather than wall-clock cookie timestamps.
    deadline: Instant,
}

impl PendingPairing {
    /// Supplies the ticket's authoritative deadline to the encrypted transport.
    pub(super) fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Owns a full-entropy pairing secret until one claim or cancellation by drop.
    ///
    /// The supplied deadline must be future and within five minutes. The caller
    /// must generate the secret securely; byte length does not prove entropy.
    ///
    /// # Errors
    ///
    /// Returns a safe material, elapsed-deadline, or excess-lifetime error.
    pub fn new(secret: Zeroizing<Vec<u8>>, deadline: Instant) -> Result<Self, HandoffError> {
        if secret.len() != 32 {
            return Err(HandoffError::new(ErrorCode::InvalidMaterial));
        }
        check_pending_deadline(deadline)?;
        Ok(Self { secret, deadline })
    }

    /// Consumes the ticket to create one CLI handshake, never a reusable claim.
    ///
    /// The returned handshake still has to authenticate its peer. Claiming does
    /// not grant cookie access. Disconnect or failure must start a new user login,
    /// not recreate a ticket with the old secret.
    ///
    /// ```compile_fail
    /// use beyond_gate_core::auth::handoff::pairing::PendingPairing;
    ///
    /// // Ownership rejects a second claim even if the first attempt failed.
    /// fn cannot_reuse(ticket: PendingPairing) {
    ///     let _first = ticket.claim();
    ///     let _second = ticket.claim();
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a safe timeout or cryptographic failure. The owned ticket cannot
    /// be reused after any result; expired owned secret input is dropped and cleared.
    pub fn claim(self) -> Result<PairingHandshake, HandoffError> {
        if Instant::now() >= self.deadline {
            return Err(HandoffError::new(ErrorCode::TimedOut));
        }
        PairingHandshake::new(Role::Cli, self.secret)
    }
}

/// Selects the fixed initiator/responder responsibilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    /// The browser-launched native host initiates pairing.
    Host,
    /// The pending CLI receives the host's first handshake.
    Cli,
}

/// Owns one handshake attempt; every failed operation permanently closes it.
pub struct PairingHandshake {
    /// Taking the state before an operation prevents reuse after failure.
    state: Option<HandshakeState>,
}

impl PairingHandshake {
    /// Consumes a 32-byte secret that the caller obtained through explicit pairing.
    ///
    /// The caller must use CSPRNG-generated bytes in production, never a short
    /// password, public transaction ID, or fixture constant. Length is not entropy
    /// validation. The constructor proves neither delivery nor one-use authorization.
    ///
    /// # Errors
    ///
    /// Returns a safe material or cryptographic error; invalid owned input is cleared.
    pub fn new(role: Role, secret: Zeroizing<Vec<u8>>) -> Result<Self, HandoffError> {
        let key_bytes: &[u8; 32] = secret
            .as_slice()
            .try_into()
            .map_err(|_| HandoffError::new(ErrorCode::InvalidMaterial))?;
        let parameters = PATTERN.parse().map_err(HandoffError::crypto)?;
        let builder = Builder::new(parameters)
            .prologue(PROLOGUE)
            .map_err(HandoffError::crypto)?
            .psk(0, key_bytes)
            .map_err(HandoffError::crypto)?;
        let state = match role {
            Role::Host => builder.build_initiator(),
            Role::Cli => builder.build_responder(),
        }
        .map_err(HandoffError::crypto)?;
        Ok(Self { state: Some(state) })
    }

    /// Writes the next empty handshake message, never an application credential.
    ///
    /// # Errors
    ///
    /// Returns a safe sequencing or cryptographic error and closes this attempt.
    pub fn write(&mut self) -> Result<Vec<u8>, HandoffError> {
        let mut state = self
            .state
            .take()
            .ok_or_else(|| HandoffError::new(ErrorCode::Disconnected))?;
        let mut output = vec![0; MAX_HANDSHAKE_BYTES];
        let length = state
            .write_message(&[], &mut output)
            .map_err(HandoffError::crypto)?;
        output.truncate(length);
        self.state = Some(state);
        Ok(output)
    }

    /// Reads a bounded handshake and rejects even authenticated application payload.
    ///
    /// # Errors
    ///
    /// Returns a safe bounds, proof, or sequencing error and closes this attempt.
    pub fn read(&mut self, message: &[u8]) -> Result<(), HandoffError> {
        let mut state = self
            .state
            .take()
            .ok_or_else(|| HandoffError::new(ErrorCode::Disconnected))?;
        if message.is_empty() || message.len() > MAX_HANDSHAKE_BYTES {
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        let mut plaintext = Zeroizing::new(vec![0; MAX_HANDSHAKE_BYTES]);
        let length = state
            .read_message(message, &mut plaintext)
            .map_err(HandoffError::crypto)?;
        if length != 0 {
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        self.state = Some(state);
        Ok(())
    }

    /// Consumes a completed handshake to enable encrypted application messages.
    ///
    /// This proves PSK possession only. The controller must separately bind the
    /// connection to its pending login before releasing browser authorization.
    ///
    /// # Errors
    ///
    /// Returns a safe failure if the attempt failed or the handshake is incomplete.
    pub fn finish(self) -> Result<PairedChannel, HandoffError> {
        let state = self
            .state
            .ok_or_else(|| HandoffError::new(ErrorCode::Disconnected))?;
        let state = state.into_transport_mode().map_err(HandoffError::crypto)?;
        Ok(PairedChannel { state: Some(state) })
    }
}

/// Encrypts one ordered channel with independent nonce counters in each direction.
///
/// The transport controller must enforce framing, frame counts, deadlines, and
/// one-use transaction state. This object performs no I/O and cannot authorize a save.
pub struct PairedChannel {
    /// A failed seal or open prevents all later reuse of this channel.
    state: Option<TransportState>,
}

impl PairedChannel {
    /// Encrypts owned plaintext within the shared frame-payload bound.
    ///
    /// # Errors
    ///
    /// Returns a safe size, closed-channel, or cryptographic error and closes the channel.
    pub fn seal(&mut self, plaintext: Zeroizing<Vec<u8>>) -> Result<Vec<u8>, HandoffError> {
        let mut state = self
            .state
            .take()
            .ok_or_else(|| HandoffError::new(ErrorCode::Disconnected))?;
        if plaintext.is_empty() || plaintext.len() > MAX_FRAME_BYTES {
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        let mut ciphertext = vec![0; plaintext.len() + TAG_BYTES];
        let length = state
            .write_message(&plaintext, &mut ciphertext)
            .map_err(HandoffError::crypto)?;
        ciphertext.truncate(length);
        self.state = Some(state);
        Ok(ciphertext)
    }

    /// Authenticates the next ciphertext before returning zeroizing plaintext.
    ///
    /// Modification, reordering, duplicate messages, or another channel's keys fail.
    ///
    /// # Errors
    ///
    /// Returns a safe size, closed-channel, or proof error and closes the channel.
    pub fn open(&mut self, ciphertext: &[u8]) -> Result<Zeroizing<Vec<u8>>, HandoffError> {
        let mut state = self
            .state
            .take()
            .ok_or_else(|| HandoffError::new(ErrorCode::Disconnected))?;
        if ciphertext.len() <= TAG_BYTES || ciphertext.len() > MAX_FRAME_BYTES + TAG_BYTES {
            return Err(HandoffError::new(ErrorCode::ProtocolError));
        }
        let mut plaintext = Zeroizing::new(vec![0; ciphertext.len() - TAG_BYTES]);
        let length = state
            .read_message(ciphertext, &mut plaintext)
            .map_err(HandoffError::crypto)?;
        plaintext.truncate(length);
        self.state = Some(state);
        Ok(plaintext)
    }
}
