//! Typed auth failures with a redacted native-store cause chain.

use std::{error::Error, fmt};

use zeroize::Zeroize;

/// Separates blocked login, invalid local material, and protected-store failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthErrorKind {
    /// No permitted browser return path has been established.
    LoginBlocked,
    /// Opaque credential data is empty, oversized, or invalid for the store.
    InvalidSecret,
    /// The store denied access or is locked; the backend may not distinguish them.
    StorageAccess,
    /// The store has no configured or supported implementation.
    StorageUnavailable,
    /// A native operation failed for a reason not classified by its safe error enum.
    StorageFailure,
    /// More than one entry matches; no entry was selected or deleted.
    AmbiguousEntry,
    /// This target has no explicitly selected native credential backend.
    UnsupportedPlatform,
}

impl fmt::Display for AuthErrorKind {
    /// Gives recovery guidance without including input or native error strings.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::LoginBlocked => {
                "browser login is blocked: no permitted session handoff is established; see docs/authentication.md; do not supply a password or cookie"
            }
            Self::InvalidSecret => {
                "local session material is invalid or exceeds a storage limit; no upstream validation was performed"
            }
            Self::StorageAccess => {
                "protected credential storage is locked or access was denied; unlock it or allow access through your OS credential manager"
            }
            Self::StorageUnavailable => {
                "protected credential storage is unavailable; enable your OS credential service; no plaintext fallback is used"
            }
            Self::StorageFailure => {
                "protected credential storage failed; check your OS credential service; no plaintext fallback is used"
            }
            Self::AmbiguousEntry => {
                "multiple Beyond Gate credentials match; resolve duplicates in your OS credential manager"
            }
            Self::UnsupportedPlatform => {
                "protected session storage is supported only on Windows, macOS, and Linux"
            }
        })
    }
}

/// Retains the native typed failure without exposing its strings or raw source.
///
/// Native errors can contain credential bytes, entry metadata, or platform
/// messages. The standard error chain therefore stops at this safe wrapper.
pub struct StorageCause {
    /// Preserves the concrete native failure for internal classification.
    error: keyring_core::Error,
}

impl StorageCause {
    /// Classifies only enum variants, never native error text.
    pub fn kind(&self) -> AuthErrorKind {
        match self.error {
            keyring_core::Error::NoStorageAccess(_) => AuthErrorKind::StorageAccess,
            keyring_core::Error::NoDefaultStore | keyring_core::Error::NotSupportedByStore(_) => {
                AuthErrorKind::StorageUnavailable
            }
            keyring_core::Error::BadEncoding(_)
            | keyring_core::Error::BadDataFormat(_, _)
            | keyring_core::Error::TooLong(_, _)
            | keyring_core::Error::Invalid(_, _) => AuthErrorKind::InvalidSecret,
            keyring_core::Error::Ambiguous(_) => AuthErrorKind::AmbiguousEntry,
            _ => AuthErrorKind::StorageFailure,
        }
    }
}

impl fmt::Display for StorageCause {
    /// Reports only the safe classification.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind(), formatter)
    }
}

impl fmt::Debug for StorageCause {
    /// Prevents native error Debug from revealing data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for StorageCause {}

/// Provides a safe category and a bounded, redacted cause chain.
pub struct AuthError {
    /// Explains the application-level failure.
    kind: AuthErrorKind,
    /// Retains a native cause only when a store operation failed.
    cause: Option<StorageCause>,
}

impl AuthError {
    /// Constructs a local failure that has no native cause.
    pub(super) fn new(kind: AuthErrorKind) -> Self {
        Self { kind, cause: None }
    }

    /// Wraps a native error and clears any raw malformed secret bytes it owns.
    pub(super) fn storage(mut error: keyring_core::Error) -> Self {
        match &mut error {
            keyring_core::Error::BadEncoding(bytes)
            | keyring_core::Error::BadDataFormat(bytes, _) => bytes.zeroize(),
            _ => {}
        }
        let cause = StorageCause { error };
        Self {
            kind: cause.kind(),
            cause: Some(cause),
        }
    }

    /// Returns the safe failure category for adapter decisions.
    pub fn kind(&self) -> AuthErrorKind {
        self.kind
    }
}

impl fmt::Display for AuthError {
    /// Omits all native strings and credential material.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, formatter)
    }
}

impl fmt::Debug for AuthError {
    /// Uses the same safe diagnostic in test failures and adapter logs.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for AuthError {
    /// Exposes only the redacted wrapper, never the underlying native error.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|cause| cause as &dyn Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks malformed native bytes are cleared while the typed variant is retained.
    #[test]
    fn malformed_native_bytes_are_cleared() {
        for native in [
            keyring_core::Error::BadEncoding(b"synthetic-secret".to_vec()),
            keyring_core::Error::BadDataFormat(
                b"synthetic-secret".to_vec(),
                Box::new(std::io::Error::other("test")),
            ),
        ] {
            let error = AuthError::storage(native);
            assert_eq!(error.kind(), AuthErrorKind::InvalidSecret);
            match error.cause.expect("retained cause").error {
                keyring_core::Error::BadEncoding(bytes)
                | keyring_core::Error::BadDataFormat(bytes, _) => assert!(bytes.is_empty()),
                _ => panic!("original typed variant must be retained"),
            }
        }
    }
}
