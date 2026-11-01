//! Windows service validation and one confirmed protected save.
//!
//! This boundary trusts a direct TLS response from the fixed authentication
//! service. It does not independently verify JWT signatures or content access.
//! The cookie is persistent material; the service bearer is transient evidence.
//! Browser cookie attributes and consent must be checked before this boundary.

use std::{
    error::Error,
    fmt,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize, de::Visitor};
use tokio::{sync::Notify, time::Instant};
use zeroize::Zeroizing;

use super::{
    AuthError, AuthErrorKind, SessionSecret, SessionStore,
    handoff::{ErrorCode, pairing::check_pending_deadline, slot::PendingLoginSlot},
};
use crate::http::{HttpError, SessionHeaders, direct_client_builder, validate_cookie};

/// Runs only compiled invented material through isolated development processes.
#[cfg(feature = "windows-login-fixture")]
#[path = "windows_login_fixture.rs"]
pub mod fixture;

/// Fixes the only production exchange origin and path.
const SERVICE_URL: &str = "https://auth-service.dndbeyond.com/v1/cobalt-token";
/// Includes the eight-byte format header in Windows' native blob limit.
const MAX_STORED_BYTES: usize = 2_560;
/// Leaves room for magic, version, profile, and little-endian value length.
pub(crate) const MAX_COOKIE_BYTES: usize = MAX_STORED_BYTES - 8;
/// Bounds both the upstream response and every decoded allocation within it.
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
/// Caps the transient token independently of the response.
const MAX_TOKEN_BYTES: usize = 8 * 1024;
/// Identifies the portable persistent-cookie encoding.
const MAGIC: &[u8; 4] = b"BGSE";
/// Separates a write not started, a started write, and a permanently stopped write.
const READY: u8 = 0;
/// Marks the linearization point immediately before the native write.
const WRITING: u8 = 1;
/// Records that native storage confirmed success.
const SAVED: u8 = 2;
/// Prevents a queued worker from starting a write after cancellation or timeout.
const STOPPED: u8 = 3;

/// Owns one RFC 6265 value that fits the versioned Windows credential entry.
pub struct BrowserSessionCookie(Zeroizing<Vec<u8>>);

impl BrowserSessionCookie {
    /// Takes and clears owned input; no URL, cookie name, or header bag is accepted.
    ///
    /// # Errors
    ///
    /// Returns `InvalidMaterial` for invalid syntax or a value above 2,552 bytes.
    pub fn new(bytes: Vec<u8>) -> Result<Self, EstablishmentError> {
        let bytes = Zeroizing::new(bytes);
        if bytes.len() > MAX_COOKIE_BYTES {
            return Err(EstablishmentError::new(ErrorCode::InvalidMaterial));
        }
        let value = std::str::from_utf8(&bytes)
            .map_err(|_| EstablishmentError::new(ErrorCode::InvalidMaterial))?;
        validate_cookie("CobaltSession", value).map_err(|error| {
            EstablishmentError::caused(ErrorCode::InvalidMaterial, Cause::Http(error))
        })?;
        Ok(Self(bytes))
    }

    /// Encodes only the reusable cookie, never account claims or a derived bearer.
    fn encode(&self) -> SessionSecret {
        let mut bytes = Zeroizing::new(Vec::with_capacity(8 + self.0.len()));
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&[1, 1]);
        // The constructor proves this length fits both u16 and the native blob.
        bytes.extend_from_slice(&(self.0.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&self.0);
        SessionSecret(bytes)
    }

    /// Consumes a stored envelope without guessing an old opaque format.
    ///
    /// # Errors
    ///
    /// Returns `UnsupportedContract` for unknown magic, version, or profile.
    /// Returns `InvalidMaterial` for size, length, trailing data, or cookie syntax.
    pub fn from_persisted(secret: SessionSecret) -> Result<Self, EstablishmentError> {
        let bytes = &secret.0;
        if bytes.len() < 8 || bytes.len() > MAX_STORED_BYTES {
            return Err(EstablishmentError::new(ErrorCode::InvalidMaterial));
        }
        if &bytes[..4] != MAGIC || bytes[4..6] != [1, 1] {
            return Err(EstablishmentError::new(ErrorCode::UnsupportedContract));
        }
        let length = usize::from(u16::from_le_bytes([bytes[6], bytes[7]]));
        if length != bytes.len() - 8 {
            return Err(EstablishmentError::new(ErrorCode::InvalidMaterial));
        }
        Self::new(bytes[8..].to_vec())
    }
}

impl fmt::Debug for BrowserSessionCookie {
    /// Omits value, format, and length.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BrowserSessionCookie { value: [REDACTED] }")
    }
}

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

/// Preserves dependency types behind an inaccessible, redacted source boundary.
enum Cause {
    /// Retains direct TLS and bounded-body failures.
    Transport(reqwest::Error),
    /// Retains shared header validation failures.
    Http(HttpError),
    /// Retains parser failures without reflected fields.
    Json(serde_json::Error),
    /// Retains encoding failures without the token.
    Encoding(base64::DecodeError),
    /// Retains the native store's already redacted classification.
    Storage(AuthError),
    /// Retains worker failure without task panic text.
    Worker(tokio::task::JoinError),
}

impl fmt::Display for Cause {
    /// Inspects only safe categories; never formats a raw dependency error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Transport(error) if error.is_timeout() => "service request timed out",
            Self::Transport(_) => "service transport failed",
            Self::Http(error) => return fmt::Display::fmt(error, formatter),
            Self::Json(error) => match error.classify() {
                serde_json::error::Category::Data => "service fields are invalid",
                _ => "service JSON is invalid",
            },
            Self::Encoding(_error) => "service token encoding is invalid",
            Self::Storage(error) => return fmt::Display::fmt(error, formatter),
            Self::Worker(error) if error.is_cancelled() => "storage worker was cancelled",
            Self::Worker(_) => "storage worker failed",
        })
    }
}

impl fmt::Debug for Cause {
    /// Uses only the safe category description.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for Cause {}

/// Carries one fixed error code and, when needed, a redacted typed cause.
pub struct EstablishmentError {
    /// Uses the same categories as the host and CLI handoff.
    code: ErrorCode,
    /// Never exposes a raw transport, JSON, or native-store source.
    cause: Option<Cause>,
}

impl EstablishmentError {
    /// Builds a semantic failure without external text.
    fn new(code: ErrorCode) -> Self {
        Self { code, cause: None }
    }

    /// Retains a typed failure with a safe source boundary.
    fn caused(code: ErrorCode, cause: Cause) -> Self {
        Self {
            code,
            cause: Some(cause),
        }
    }

    /// Returns the fixed category for consistent adapter results.
    pub fn code(&self) -> ErrorCode {
        self.code
    }
}

impl fmt::Display for EstablishmentError {
    /// Emits only the fixed code.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code.as_str())
    }
}

impl fmt::Debug for EstablishmentError {
    /// Does not expose dependency or credential data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for EstablishmentError {
    /// Ends the standard source chain at the safe typed wrapper.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|cause| cause as &dyn Error)
    }
}

/// Borrows only the bounded service token; unknown response fields are discarded.
#[derive(Deserialize)]
struct ServiceResponse<'a> {
    /// Absence is a rejected session, not HTTP-success evidence.
    #[serde(borrow)]
    token: Option<&'a str>,
}

/// Keeps only a Boolean classification, never an account identifier allocation.
#[derive(Default)]
struct PositiveIdentifier(bool);

impl<'de> Deserialize<'de> for PositiveIdentifier {
    /// Classifies positive numeric or decimal-string identifiers without retaining values.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// Accepts only the approved numeric representations.
        struct IdentifierVisitor;
        impl<'de> Visitor<'de> for IdentifierVisitor {
            type Value = PositiveIdentifier;

            /// Gives a fixed expectation without reflecting input.
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a numeric identifier")
            }

            /// Accepts a positive unsigned numeric identifier.
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(PositiveIdentifier(value > 0))
            }

            /// Rejects negative or zero numeric identifiers.
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(PositiveIdentifier(value > 0))
            }

            /// Accepts bounded decimal digits only, with a positive representable value.
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(PositiveIdentifier(
                    !value.is_empty()
                        && value.bytes().all(|byte| byte.is_ascii_digit())
                        && value.parse::<u64>().is_ok_and(|number| number > 0),
                ))
            }
        }
        deserializer.deserialize_any(IdentifierVisitor)
    }
}

/// Selects exactly the accepted authentication and expiry evidence from a TLS response.
#[derive(Deserialize)]
struct Claims {
    /// Missing or nonpositive identifiers cannot validate an account session.
    #[serde(
        default,
        rename = "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier"
    )]
    identifier: PositiveIdentifier,
    /// Required future bearer expiry; not persisted as cookie expiry.
    exp: Option<u64>,
}

/// Applies the accepted direct-service rule without independently verifying a signature.
fn validate_response(bytes: &[u8], now: SystemTime) -> Result<(), EstablishmentError> {
    let response: ServiceResponse<'_> = serde_json::from_slice(bytes).map_err(|error| {
        EstablishmentError::caused(ErrorCode::ValidationUnavailable, Cause::Json(error))
    })?;
    let token = response
        .token
        .filter(|value| !value.is_empty())
        .ok_or_else(|| EstablishmentError::new(ErrorCode::SessionRejected))?;
    if token.len() > MAX_TOKEN_BYTES {
        return Err(EstablishmentError::new(ErrorCode::ValidationUnavailable));
    }
    let mut parts = token.split('.');
    let [Some(header), Some(encoded_payload), Some(signature), None] =
        [parts.next(), parts.next(), parts.next(), parts.next()]
    else {
        return Err(EstablishmentError::new(ErrorCode::ValidationUnavailable));
    };
    let mut payload = None;
    for (index, part) in [header, encoded_payload, signature].iter().enumerate() {
        if part.is_empty() {
            return Err(EstablishmentError::new(ErrorCode::ValidationUnavailable));
        }
        let decoded = Zeroizing::new(URL_SAFE_NO_PAD.decode(part).map_err(|error| {
            EstablishmentError::caused(ErrorCode::ValidationUnavailable, Cause::Encoding(error))
        })?);
        if index == 1 {
            payload = Some(decoded);
        }
    }
    let payload =
        payload.ok_or_else(|| EstablishmentError::new(ErrorCode::ValidationUnavailable))?;
    let claims: Claims = serde_json::from_slice(&payload).map_err(|error| {
        EstablishmentError::caused(ErrorCode::ValidationUnavailable, Cause::Json(error))
    })?;
    if !claims.identifier.0 {
        return Err(EstablishmentError::new(ErrorCode::SessionRejected));
    }
    let expiry = claims
        .exp
        .ok_or_else(|| EstablishmentError::new(ErrorCode::SessionRejected))?;
    let expiry = UNIX_EPOCH
        .checked_add(Duration::from_secs(expiry))
        .ok_or_else(|| EstablishmentError::new(ErrorCode::ValidationUnavailable))?;
    if expiry <= now {
        return Err(EstablishmentError::new(ErrorCode::SessionExpired));
    }
    Ok(())
}

/// Sends exactly one bounded direct POST and drops the transient bearer after validation.
async fn exchange(
    cookie: &BrowserSessionCookie,
    client: &Client,
    url: &str,
) -> Result<(), EstablishmentError> {
    let value = std::str::from_utf8(&cookie.0)
        .map_err(|_| EstablishmentError::new(ErrorCode::InvalidMaterial))?;
    let headers = SessionHeaders::cookie("CobaltSession", value).map_err(|error| {
        EstablishmentError::caused(ErrorCode::InvalidMaterial, Cause::Http(error))
    })?;
    let request = client
        .post(url)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("accept-encoding", "identity");
    let mut response = headers.apply(request).send().await.map_err(|error| {
        EstablishmentError::caused(ErrorCode::ValidationUnavailable, Cause::Transport(error))
    })?;
    match response.status() {
        StatusCode::OK => {}
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            return Err(EstablishmentError::new(ErrorCode::SessionRejected));
        }
        _ => return Err(EstablishmentError::new(ErrorCode::ValidationUnavailable)),
    }
    let json_content = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.split(';').next())
        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"));
    let identity_encoding = response
        .headers()
        .get(reqwest::header::CONTENT_ENCODING)
        .is_none_or(|header| header.as_bytes().eq_ignore_ascii_case(b"identity"));
    if !json_content || !identity_encoding {
        return Err(EstablishmentError::new(ErrorCode::ValidationUnavailable));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(EstablishmentError::new(ErrorCode::ValidationUnavailable));
    }
    let mut body = Zeroizing::new(Vec::new());
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        EstablishmentError::caused(ErrorCode::ValidationUnavailable, Cause::Transport(error))
    })? {
        if chunk.len() > MAX_RESPONSE_BYTES - body.len() {
            return Err(EstablishmentError::new(ErrorCode::ValidationUnavailable));
        }
        body.extend_from_slice(&chunk);
    }
    validate_response(&body, SystemTime::now())
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
        result = exchange(&input.cookie, &client, url) => result?,
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
    let client = direct_client_builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| {
            EstablishmentError::caused(ErrorCode::ValidationUnavailable, Cause::Transport(error))
        })?;
    establish_with(input, control, client, SERVICE_URL, || {
        SessionStore::open().map(Arc::new)
    })
    .await
}

#[cfg(test)]
#[path = "establishment_tests.rs"]
mod tests;
