//! Windows service validation and one confirmed protected save.
//!
//! This boundary trusts a direct TLS response from the fixed authentication
//! service. It does not independently verify JWT signatures or content access.
//! The cookie is persistent material; the service bearer is transient evidence.
//! Browser cookie attributes and consent must be checked before this boundary.

use std::{
    fmt,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    time::Duration,
};

use reqwest::Client;
use serde::Serialize;
use tokio::{sync::Notify, time::Instant};

use super::{
    AuthError, AuthErrorKind, SessionSecret, SessionStore,
    handoff::{ErrorCode, pairing::check_pending_deadline, slot::PendingLoginSlot},
    service::{Cause, SERVICE_URL, exchange},
};
#[cfg(any(test, feature = "windows-login-fixture"))]
use crate::http::direct_client_builder;

pub use super::service::{BrowserSessionCookie, EstablishmentError};

#[cfg(test)]
use super::service::{
    MAX_COOKIE_BYTES, MAX_RESPONSE_BYTES, MAX_STORED_BYTES, MAX_TOKEN_BYTES, validate_response,
};
#[cfg(test)]
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
#[cfg(test)]
use std::{
    error::Error,
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(test)]
use zeroize::Zeroizing;

/// Runs only compiled invented material through isolated development processes.
#[cfg(feature = "windows-login-fixture")]
#[path = "windows_login_fixture.rs"]
pub mod fixture;

/// Separates a write not started, a started write, and a permanently stopped write.
const READY: u8 = 0;
/// Marks the linearization point immediately before the native write.
const WRITING: u8 = 1;
/// Records that native storage confirmed success.
const SAVED: u8 = 2;
/// Prevents a queued worker from starting a write after cancellation or timeout.
const STOPPED: u8 = 3;

/// Owns the sole accepted service profile's cookie after browser-side checks.
#[derive(Debug)]
pub struct SessionEstablishmentInput {
    /// Cannot be copied or exposed through a credential accessor.
    cookie: BrowserSessionCookie,
}

impl SessionEstablishmentInput {
    /// Transfers one bounded cookie into the shared establishment operation.
    pub fn new(cookie: BrowserSessionCookie) -> Self {
        Self { cookie }
    }

    /// Consumes a profile-bound receipt without copying its credential allocation.
    ///
    /// Receipt proves neither browser metadata nor service validation. The trusted
    /// browser component must check consent and cookie attributes before submission.
    ///
    /// # Errors
    ///
    /// Returns `UnsupportedProfile` for synthetic material and `InvalidMaterial`
    /// if the transferred value does not meet the Windows cookie contract.
    pub fn from_handoff(
        receipt: super::handoff::flow::ReceivedSubmission,
    ) -> Result<Self, EstablishmentError> {
        if receipt.profile() != super::handoff::Profile::CobaltSessionV1 {
            return Err(EstablishmentError::new(ErrorCode::UnsupportedProfile));
        }
        let mut bytes = receipt.into_value().0;
        BrowserSessionCookie::new(std::mem::take(&mut *bytes)).map(Self::new)
    }
}

/// Describes confirmed validation and save, not permanent upstream usability.
#[derive(Debug, Serialize)]
pub struct SavedSession {
    /// Fixed evidence from the service response, not a local-presence check.
    validation: &'static str,
    /// Bearer expiry does not establish reusable-cookie expiry.
    expiry: &'static str,
}

impl SavedSession {
    /// Returns the fixed evidence name without exposing credentials.
    pub fn validation_evidence(&self) -> &'static str {
        self.validation
    }

    /// Returns unknown cookie expiry; never substitutes the bearer expiry.
    pub fn expiry_evidence(&self) -> &'static str {
        self.expiry
    }
}

/// Shares cancellation and write admission while retaining the pending reservation.
struct ControlState {
    /// Keeps the OS-wide pending slot alive through blocking store work.
    _reservation: Arc<PendingLoginSlot>,
    /// Never extends the original pending-login deadline.
    deadline: Instant,
    /// Stops validation as well as pending write admission.
    cancelled: AtomicBool,
    /// Stores one wake permit for the single establishment operation.
    notification: Notify,
    /// Arbitrates cancellation against the start of a native write.
    stage: AtomicU8,
}

impl ControlState {
    /// Prevents future writes and reports uncertainty if a write already started.
    fn stop(&self, before_write: ErrorCode) -> EstablishmentError {
        let code =
            match self
                .stage
                .compare_exchange(READY, STOPPED, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) | Err(STOPPED) => before_write,
                Err(_) => ErrorCode::SaveIndeterminate,
            };
        EstablishmentError::new(code)
    }

    /// Rejects cancellation and the earlier of the phase and original login deadlines.
    fn check_before(&self, deadline: Instant) -> Result<(), EstablishmentError> {
        if self.cancelled.load(Ordering::SeqCst) {
            return Err(self.stop(ErrorCode::Cancelled));
        }
        if Instant::now() >= self.deadline.min(deadline) {
            return Err(self.stop(ErrorCode::TimedOut));
        }
        Ok(())
    }

    /// Signals the sole waiter and atomically closes write admission when possible.
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let _ = self.stop(ErrorCode::Cancelled);
        self.notification.notify_one();
    }
}

/// Owns one immutable deadline and the reservation required for establishment.
pub struct LoginControl(Arc<ControlState>);

impl LoginControl {
    /// Binds a future deadline of at most five minutes to an owned pending slot.
    ///
    /// # Errors
    ///
    /// Returns the existing pending-deadline code for an expired or excessive bound.
    pub fn new(
        reservation: Arc<PendingLoginSlot>,
        deadline: Instant,
    ) -> Result<Self, EstablishmentError> {
        check_pending_deadline(deadline).map_err(|error| EstablishmentError::new(error.code()))?;
        Ok(Self(Arc::new(ControlState {
            _reservation: reservation,
            deadline,
            cancelled: AtomicBool::new(false),
            notification: Notify::new(),
            stage: AtomicU8::new(READY),
        })))
    }

    /// Provides a non-secret cancellation handle, not a second operation owner.
    pub fn cancellation(&self) -> LoginCancellation {
        LoginCancellation(Arc::downgrade(&self.0))
    }
}

impl Drop for LoginControl {
    /// Stops a queued write when the owning async operation is dropped.
    fn drop(&mut self) {
        self.0.cancel();
    }
}

impl fmt::Debug for LoginControl {
    /// Hides reservation metadata and internal state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LoginControl { state: [REDACTED] }")
    }
}

/// Lets an adapter stop its own login without gaining credential access.
#[derive(Clone)]
pub struct LoginCancellation(Weak<ControlState>);

impl LoginCancellation {
    /// Cancels an active owner and reports whether its write may already have started.
    ///
    /// A missing owner cannot establish unchanged storage, so it is conservative.
    pub fn cancel_outcome(&self) -> ErrorCode {
        if let Some(state) = self.0.upgrade() {
            let code = state.stop(ErrorCode::Cancelled).code();
            state.cancel();
            code
        } else {
            ErrorCode::SaveIndeterminate
        }
    }

    /// Requests cancellation; an already started write cannot be rolled back.
    /// A completed operation is unaffected; this handle does not retain its slot.
    pub fn cancel(&self) {
        if let Some(state) = self.0.upgrade() {
            state.cancel();
        }
    }
}

impl fmt::Debug for LoginCancellation {
    /// Omits pending-slot and task details.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LoginCancellation { state: [REDACTED] }")
    }
}

/// Maps failures before write admission without changing the prior credential.
fn setup_error(error: AuthError) -> EstablishmentError {
    let code = match error.kind() {
        AuthErrorKind::StorageAccess => ErrorCode::StorageAccess,
        AuthErrorKind::StorageUnavailable | AuthErrorKind::UnsupportedPlatform => {
            ErrorCode::StorageUnavailable
        }
        _ => ErrorCode::StorageFailure,
    };
    EstablishmentError::caused(code, Cause::Storage(error))
}

/// Opens and writes off the executor, retaining reservation ownership even after caller drop.
async fn save<F>(
    secret: SessionSecret,
    state: &Arc<ControlState>,
    open: F,
) -> Result<SavedSession, EstablishmentError>
where
    F: FnOnce() -> Result<Arc<SessionStore>, AuthError> + Send + 'static,
{
    state.check_before(state.deadline)?;
    let save_deadline = state.deadline.min(Instant::now() + Duration::from_secs(5));
    let worker_state = Arc::clone(state);
    let worker = tokio::task::spawn_blocking(move || {
        worker_state.check_before(save_deadline)?;
        let store = open().map_err(setup_error)?;
        worker_state.check_before(save_deadline)?;
        if worker_state
            .stage
            .compare_exchange(READY, WRITING, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            worker_state.check_before(save_deadline)?;
            return Err(EstablishmentError::new(ErrorCode::Cancelled));
        }
        store.save(&secret).map_err(|error| {
            EstablishmentError::caused(ErrorCode::SaveIndeterminate, Cause::Storage(error))
        })?;
        worker_state.stage.store(SAVED, Ordering::SeqCst);
        Ok(SavedSession {
            validation: "service_validated",
            expiry: "unknown",
        })
    });
    tokio::select! {
        biased;
        _ = state.notification.notified() => Err(state.stop(ErrorCode::Cancelled)),
        _ = tokio::time::sleep_until(save_deadline) => Err(state.stop(ErrorCode::TimedOut)),
        result = worker => {
            let saved = result.map_err(|error| {
                let code = state.stop(ErrorCode::StorageFailure).code();
                EstablishmentError::caused(code, Cause::Worker(error))
            })??;
            state.check_before(save_deadline)?;
            Ok(saved)
        },
    }
}

/// Runs the same lifecycle with private synthetic service and store seams in tests.
pub(super) async fn establish_with<F>(
    input: SessionEstablishmentInput,
    control: LoginControl,
    client: Client,
    url: &str,
    open: F,
) -> Result<SavedSession, EstablishmentError>
where
    F: FnOnce() -> Result<Arc<SessionStore>, AuthError> + Send + 'static,
{
    control.0.check_before(control.0.deadline)?;
    let secret = input.cookie.encode();
    tokio::select! {
        biased;
        _ = control.0.notification.notified() => return Err(control.0.stop(ErrorCode::Cancelled)),
        _ = tokio::time::sleep_until(control.0.deadline) => return Err(control.0.stop(ErrorCode::TimedOut)),
        result = exchange(&input.cookie, &client, url) => { result?; },
    }
    save(secret, &control.0, open).await
}

/// Validates the accepted Windows service session and confirms one protected save.
///
/// Native store setup starts only after validation. Cancellation before write
/// admission preserves the prior entry. An unconfirmed started write returns
/// `SaveIndeterminate`; it is never deleted or retried. Dropping this future
/// closes write admission and keeps the reservation in any blocking worker.
///
/// # Errors
///
/// Returns fixed rejected, expired-bearer, bounded-validation, pre-write storage,
/// cancellation, timeout, or indeterminate-save categories with safe typed causes.
/// A future bearer expiry does not establish cookie expiry or content access.
pub async fn establish_session(
    input: SessionEstablishmentInput,
    control: LoginControl,
) -> Result<SavedSession, EstablishmentError> {
    let client = super::service::client()?;
    establish_with(input, control, client, SERVICE_URL, || {
        SessionStore::open().map(Arc::new)
    })
    .await
}

#[cfg(test)]
#[path = "establishment_tests.rs"]
mod tests;
