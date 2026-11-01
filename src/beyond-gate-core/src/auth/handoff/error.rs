//! Fixed wire codes and redacted, typed parsing and transport causes.

use std::{error::Error, fmt, io};

use crate::auth::AuthError;

/// Contains only the proposed fixed wire error codes, never peer error text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    /// Explicit user consent is absent.
    ConsentRequired,
    /// Application-peer proof failed.
    UnauthorizedPeer,
    /// The configured browser or extension identity does not match.
    OriginRejected,
    /// No pending CLI login exists.
    NoPendingLogin,
    /// Another connection owns the pending slot.
    Busy,
    /// The protocol version is unsupported.
    VersionMismatch,
    /// No accepted profile matches the message.
    UnsupportedProfile,
    /// Message shape, framing, encoding, or sequencing is invalid.
    ProtocolError,
    /// The correlation identifier differs from the bound transaction.
    TransactionMismatch,
    /// The one-use authorization has already been consumed.
    Replay,
    /// The narrow cookie query failed or returned unusable candidates.
    CookieUnavailable,
    /// Cookie syntax or persistent encoding is invalid.
    InvalidMaterial,
    /// Cancellation was observed before saving.
    Cancelled,
    /// A monotonic deadline was reached.
    TimedOut,
    /// The peer disconnected without a confirmed final result.
    Disconnected,
    /// Upstream rejected the credential without verified expiry evidence.
    SessionRejected,
    /// Accepted evidence establishes expiry.
    SessionExpired,
    /// Bounded service validation could not complete.
    ValidationUnavailable,
    /// The shared validation or persistence contract is unsupported.
    UnsupportedContract,
    /// The protected store is unavailable before a write.
    StorageUnavailable,
    /// Protected-store access was denied before a write.
    StorageAccess,
    /// Store setup failed before a write.
    StorageFailure,
    /// A started write or its final acknowledgement is unconfirmed.
    SaveIndeterminate,
}

impl ErrorCode {
    /// Returns the exact, fixed version 1 code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConsentRequired => "consent_required",
            Self::UnauthorizedPeer => "unauthorized_peer",
            Self::OriginRejected => "origin_rejected",
            Self::NoPendingLogin => "no_pending_login",
            Self::Busy => "busy",
            Self::VersionMismatch => "version_mismatch",
            Self::UnsupportedProfile => "unsupported_profile",
            Self::ProtocolError => "protocol_error",
            Self::TransactionMismatch => "transaction_mismatch",
            Self::Replay => "replay",
            Self::CookieUnavailable => "cookie_unavailable",
            Self::InvalidMaterial => "invalid_material",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::Disconnected => "disconnected",
            Self::SessionRejected => "session_rejected",
            Self::SessionExpired => "session_expired",
            Self::ValidationUnavailable => "validation_unavailable",
            Self::UnsupportedContract => "unsupported_contract",
            Self::StorageUnavailable => "storage_unavailable",
            Self::StorageAccess => "storage_access",
            Self::StorageFailure => "storage_failure",
            Self::SaveIndeterminate => "save_indeterminate",
        }
    }

    /// Maps peer input to fixed codes without retaining or reflecting its text.
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` for every unknown code, including `"ok"`.
    pub fn parse(value: &str) -> Result<Self, HandoffError> {
        let codes = [
            Self::ConsentRequired,
            Self::UnauthorizedPeer,
            Self::OriginRejected,
            Self::NoPendingLogin,
            Self::Busy,
            Self::VersionMismatch,
            Self::UnsupportedProfile,
            Self::ProtocolError,
            Self::TransactionMismatch,
            Self::Replay,
            Self::CookieUnavailable,
            Self::InvalidMaterial,
            Self::Cancelled,
            Self::TimedOut,
            Self::Disconnected,
            Self::SessionRejected,
            Self::SessionExpired,
            Self::ValidationUnavailable,
            Self::UnsupportedContract,
            Self::StorageUnavailable,
            Self::StorageAccess,
            Self::StorageFailure,
            Self::SaveIndeterminate,
        ];
        codes
            .into_iter()
            .find(|code| code.as_str() == value)
            .ok_or_else(|| HandoffError::new(Self::ProtocolError))
    }
}

/// Retains a concrete dependency error behind a safe formatter and source boundary.
enum Cause {
    /// Preserves JSON syntax, data, or end-of-input classification internally.
    Json(serde_json::Error),
    /// Preserves the transport's OS error category internally.
    Io(io::Error),
    /// Preserves the shared secret constructor's typed failure.
    Secret(AuthError),
    /// Keeps the Noise library's concrete failure behind a redacted boundary.
    Crypto(snow::Error),
    /// Retains malformed pairing-key encoding without its input.
    Encoding(base64::DecodeError),
    /// Keeps both the original failure and a failed safe reply behind fixed diagnostics.
    Delivery {
        original: Box<HandoffError>,
        reply: Box<HandoffError>,
    },
}

/// Stops the standard error chain before raw JSON or OS error text.
pub struct HandoffCause(Cause);

impl fmt::Display for HandoffCause {
    /// Reports only the cause category, never reflected fields or dependency text.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Cause::Json(error) => formatter.write_str(match error.classify() {
                serde_json::error::Category::Eof => "handoff JSON is incomplete",
                serde_json::error::Category::Syntax => "handoff JSON syntax is invalid",
                serde_json::error::Category::Data => "handoff JSON fields are invalid",
                serde_json::error::Category::Io => "handoff JSON input failed",
            }),
            Cause::Io(error) => formatter.write_str(match error.kind() {
                io::ErrorKind::UnexpectedEof => "handoff frame is incomplete",
                io::ErrorKind::BrokenPipe => "handoff transport disconnected",
                _ => "handoff transport failed",
            }),
            Cause::Secret(error) => fmt::Display::fmt(error, formatter),
            Cause::Crypto(error) => formatter.write_str(match error {
                snow::Error::Decrypt => "handoff peer proof failed",
                _ => "handoff cryptographic operation failed",
            }),
            Cause::Encoding(_error) => formatter.write_str("pairing key encoding is invalid"),
            Cause::Delivery { original, reply } => write!(
                formatter,
                "handoff failed ({}); failure reply also failed ({})",
                original.code().as_str(),
                reply.code().as_str()
            ),
        }
    }
}

impl fmt::Debug for HandoffCause {
    /// Uses the same redacted representation as Display.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for HandoffCause {}

/// Carries a fixed code and, when applicable, a redacted typed cause.
pub struct HandoffError {
    /// Supports safe adapter classification.
    code: ErrorCode,
    /// Keeps raw dependency errors inaccessible to callers.
    cause: Option<HandoffCause>,
}

impl HandoffError {
    /// Distinguishes a lost completion peer without changing ordinary frame parsing.
    #[cfg(windows)]
    pub(super) fn completion_read(mut self) -> Self {
        if matches!(
            &self.cause,
            Some(HandoffCause(Cause::Io(error)))
                if matches!(error.kind(),
                    io::ErrorKind::UnexpectedEof | io::ErrorKind::BrokenPipe
                    | io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::NotConnected)
        ) {
            self.code = ErrorCode::Disconnected;
        }
        self
    }

    /// Changes only the category when confirmed-save delivery becomes uncertain.
    pub(super) fn reclassify(mut self, code: ErrorCode) -> Self {
        self.code = code;
        self
    }

    /// Preserves the original category and both typed failures after reply failure.
    pub(super) fn failed_reply(original: Self, reply: Self) -> Self {
        Self {
            code: original.code(),
            cause: Some(HandoffCause(Cause::Delivery {
                original: Box::new(original),
                reply: Box::new(reply),
            })),
        }
    }

    /// Preserves malformed key encoding behind the fixed material classification.
    pub(super) fn encoding(error: base64::DecodeError) -> Self {
        Self {
            code: ErrorCode::InvalidMaterial,
            cause: Some(HandoffCause(Cause::Encoding(error))),
        }
    }

    /// Retains a cryptographic failure without exposing library or peer details.
    pub(super) fn crypto(error: snow::Error) -> Self {
        Self {
            code: ErrorCode::UnauthorizedPeer,
            cause: Some(HandoffCause(Cause::Crypto(error))),
        }
    }
    /// Creates a semantic failure without an external cause.
    pub(super) fn new(code: ErrorCode) -> Self {
        Self { code, cause: None }
    }

    /// Retains a JSON failure without exposing its reflected input.
    pub(super) fn json(error: serde_json::Error) -> Self {
        Self {
            code: ErrorCode::ProtocolError,
            cause: Some(HandoffCause(Cause::Json(error))),
        }
    }

    /// Retains a transport failure without exposing OS strings.
    pub(super) fn io(error: io::Error) -> Self {
        Self::with_io(ErrorCode::ProtocolError, error)
    }

    /// Retains a typed local I/O failure with the operation's explicit classification.
    pub(super) fn with_io(code: ErrorCode, error: io::Error) -> Self {
        Self {
            code,
            cause: Some(HandoffCause(Cause::Io(error))),
        }
    }

    /// Retains the shared material failure behind the redacted cause boundary.
    pub(super) fn secret(error: AuthError) -> Self {
        Self {
            code: ErrorCode::InvalidMaterial,
            cause: Some(HandoffCause(Cause::Secret(error))),
        }
    }

    /// Returns a fixed classification, not a peer-supplied message.
    pub fn code(&self) -> ErrorCode {
        self.code
    }
}

impl fmt::Display for HandoffError {
    /// Emits only the fixed wire code.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code.as_str())
    }
}

impl fmt::Debug for HandoffError {
    /// Prevents diagnostic formatting from exposing dependency data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for HandoffError {
    /// Exposes only the safe wrapper, never its raw dependency cause.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|cause| cause as &dyn Error)
    }
}
