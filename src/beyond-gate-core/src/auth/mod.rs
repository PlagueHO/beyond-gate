//! Local session storage without browser capture or upstream authentication.
//!
//! Both adapters must use this fixed entry, not their own credential identities.
//! Stored material is opaque: its presence does not establish usability or expiry.
//! No product command currently creates a credential or uses one for HTTP.
//! The off-by-default `e2e-isolation` feature requires an approved test context
//! and selects a separate test identity instead of the product entry.

mod error;

pub use error::{AuthError, AuthErrorKind, StorageCause};

use std::{collections::HashMap, fmt};

use keyring_core::{Entry, api::CredentialStoreApi};
use zeroize::Zeroizing;

/// Identifies this application without using an account name or other personal data.
#[cfg(any(test, not(feature = "e2e-isolation")))]
const SERVICE: &str = "beyond-gate";
/// Selects the single local session shared by the CLI and future MCP operations.
#[cfg(any(test, not(feature = "e2e-isolation")))]
const ACCOUNT: &str = "local-session";
/// Caps opaque session input; native stores can impose a smaller limit.
const MAX_SECRET_BYTES: usize = 8 * 1024;

/// Reports local presence only, never upstream validation or a guessed expiry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionStatus {
    /// No credential exists at the fixed application entry.
    Absent,
    /// A bounded, nonempty credential exists but has not been checked upstream.
    StoredUnverified,
}

/// Owns opaque session bytes and clears this allocation when dropped.
///
/// The upstream credential format is not yet established. This type validates
/// size only. It has no secret-bearing formatter or public byte accessor.
pub struct SessionSecret(Zeroizing<Vec<u8>>);

impl SessionSecret {
    /// Takes ownership of nonempty session bytes up to 8 KiB.
    ///
    /// Invalid input is also cleared on drop. This does not clear copies made by
    /// the caller or by the native credential-store implementation.
    ///
    /// # Errors
    ///
    /// Returns `InvalidSecret` for empty or oversized input.
    pub fn new(bytes: Vec<u8>) -> Result<Self, AuthError> {
        let bytes = Zeroizing::new(bytes);
        if bytes.is_empty() || bytes.len() > MAX_SECRET_BYTES {
            return Err(AuthError::new(AuthErrorKind::InvalidSecret));
        }
        Ok(Self(bytes))
    }
}

impl fmt::Debug for SessionSecret {
    /// Omits both the value and its length.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionSecret { value: [REDACTED] }")
    }
}

/// Opens one application entry in an explicitly selected native store.
///
/// This object holds an entry specifier, not a cached session or bearer token.
/// Operations are synchronous because the underlying stores use blocking APIs.
/// Future async adapters must keep these calls off their executor threads.
pub struct SessionStore {
    /// Avoids credential enumeration and caller-selected entry identities.
    entry: Entry,
}

impl SessionStore {
    /// Selects Windows Credential Manager, macOS login Keychain, or Linux Secret Service.
    ///
    /// Windows writes use local-machine persistence, not roaming persistence.
    /// No global keyring default or plaintext fallback is installed.
    ///
    /// # Errors
    ///
    /// Returns a typed, redacted failure if the platform or store is unavailable.
    /// Isolated test builds also require a valid context and explicit approval.
    /// Opening a store can contact the local OS service, but never D&D Beyond.
    pub fn open() -> Result<Self, AuthError> {
        #[cfg(feature = "e2e-isolation")]
        let account = {
            let context = e2e_context()?;
            if std::env::var("BEYOND_GATE_E2E_NATIVE_APPROVED").as_deref() != Ok("1") {
                return Err(AuthError::new(AuthErrorKind::TestContextRequired));
            }
            context
        };
        #[cfg(target_os = "windows")]
        let store = windows_native_keyring_store::Store::new();
        #[cfg(target_os = "macos")]
        let store = apple_native_keyring_store::keychain::Store::new();
        #[cfg(target_os = "linux")]
        let store = zbus_secret_service_keyring_store::Store::new();

        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        {
            let store = store.map_err(AuthError::storage)?;
            #[cfg(target_os = "windows")]
            let modifiers = Some(HashMap::from([("persistence", "Local")]));
            #[cfg(not(target_os = "windows"))]
            let modifiers = None;
            #[cfg(feature = "e2e-isolation")]
            {
                Self::from_identity(
                    store.as_ref(),
                    modifiers.as_ref(),
                    "beyond-gate-e2e",
                    &account,
                )
            }
            #[cfg(not(feature = "e2e-isolation"))]
            Self::from_store(store.as_ref(), modifiers.as_ref())
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        Err(AuthError::new(AuthErrorKind::UnsupportedPlatform))
    }

    /// Builds the same fixed entry for native backends and synthetic test stores.
    #[cfg(any(test, not(feature = "e2e-isolation")))]
    fn from_store(
        store: &dyn CredentialStoreApi,
        modifiers: Option<&HashMap<&str, &str>>,
    ) -> Result<Self, AuthError> {
        Self::from_identity(store, modifiers, SERVICE, ACCOUNT)
    }

    /// Builds one explicit identity without installing a global store.
    fn from_identity(
        store: &dyn CredentialStoreApi,
        modifiers: Option<&HashMap<&str, &str>>,
        service: &str,
        account: &str,
    ) -> Result<Self, AuthError> {
        let entry = store
            .build(service, account, modifiers)
            .map_err(AuthError::storage)?;
        Ok(Self { entry })
    }

    /// Saves opaque session material for a future approved login operation.
    ///
    /// This does not validate the session upstream. No CLI or MCP input invokes
    /// it. Native size limits can be smaller than the local 8 KiB input cap.
    ///
    /// # Errors
    ///
    /// Returns a redacted storage failure without falling back to a file.
    pub fn save(&self, secret: &SessionSecret) -> Result<(), AuthError> {
        self.entry.set_secret(&secret.0).map_err(AuthError::storage)
    }

    /// Reads and drops only this entry's secret to report its local presence.
    ///
    /// Never refreshes, changes, or deletes the stored value. A present value
    /// cannot establish usable, expired, or rejected upstream session states.
    ///
    /// # Errors
    ///
    /// Returns storage failures or `InvalidSecret` for empty or oversized data.
    /// Only an explicit native `NoEntry` result means the session is absent.
    pub fn status(&self) -> Result<SessionStatus, AuthError> {
        match self.entry.get_secret() {
            Ok(bytes) => {
                let _secret = SessionSecret::new(bytes)?;
                Ok(SessionStatus::StoredUnverified)
            }
            Err(keyring_core::Error::NoEntry) => Ok(SessionStatus::Absent),
            Err(error) => Err(AuthError::storage(error)),
        }
    }

    /// Deletes only this entry, including malformed data, without reading its secret.
    ///
    /// An already absent entry is a successful local logout. There is no token
    /// cache in this implementation. Local removal does not revoke an upstream
    /// session or affect credentials held by another process.
    ///
    /// # Errors
    ///
    /// Returns a redacted storage failure if local deletion cannot be confirmed.
    pub fn logout(&self) -> Result<(), AuthError> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(error) => Err(AuthError::storage(error)),
        }
    }
}

/// Validates the non-secret context of an isolated development build.
///
/// This performs no credential access. Normal product builds do not expose it.
///
/// # Errors
///
/// Returns a safe failure for missing or invalid context, never the input value.
#[cfg(feature = "e2e-isolation")]
pub fn e2e_context() -> Result<String, AuthError> {
    let value = std::env::var("BEYOND_GATE_E2E_CONTEXT")
        .map_err(|_| AuthError::new(AuthErrorKind::TestContextRequired))?;
    validate_e2e_context(&value)?;
    Ok(value)
}

/// Restricts test accounts to random lowercase hexadecimal submission identifiers.
#[cfg(feature = "e2e-isolation")]
fn validate_e2e_context(value: &str) -> Result<(), AuthError> {
    if value.len() != 32
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(AuthError::new(AuthErrorKind::TestContextRequired));
    }
    Ok(())
}

impl fmt::Debug for SessionStore {
    /// Does not expose the native store's credential formatter.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionStore { entry: [REDACTED] }")
    }
}

/// Explains why login cannot start, without browser, storage, or network activity.
pub fn login_blocker() -> AuthError {
    AuthError::new(AuthErrorKind::LoginBlocked)
}

#[cfg(test)]
mod tests;
