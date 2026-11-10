//! Shared protected loading, fresh validation, and bounded process-local bearer reuse.
//!
//! Production use is Windows-only. Portable synthetic tests exercise the same
//! policy without native credential access. No operation accepts credentials from
//! an adapter. Cookie expiry and upstream revocation remain unknown.

use std::{
    error::Error,
    fmt,
    sync::Arc,
    time::{Duration, SystemTime},
};

use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    sync::{Mutex, MutexGuard},
    time::{Instant, timeout_at},
};

use super::{
    AuthError, AuthErrorKind, SessionStore,
    handoff::ErrorCode,
    service::{self, BrowserSessionCookie, EstablishmentError, ValidatedBearer},
};
use crate::http::{ErrorKind, HttpClient, HttpError};

/// Separates session evidence failures from local storage and content access.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionErrorKind {
    /// Only a native missing-entry result establishes this state.
    Absent,
    /// The upstream service rejected credentials; human login is required.
    Rejected,
    /// Accepted service evidence established an expired bearer, not an expired cookie.
    Expired,
    /// Stored material fails its bounded format or syntax checks.
    InvalidMaterial,
    /// The stored envelope has an unknown version or service profile.
    UnsupportedContract,
    /// Native storage is locked or access was denied.
    StorageAccess,
    /// No usable protected storage service exists.
    StorageUnavailable,
    /// More than one matching credential exists.
    AmbiguousEntry,
    /// Native storage failed without a more specific safe classification.
    StorageFailure,
    /// Transport, response, or service evidence could not establish usability.
    ValidationUnavailable,
    /// Queue, storage, exchange, or read waiting reached its fixed deadline.
    TimedOut,
    /// Production session use is not implemented on this platform.
    UnsupportedPlatform,
    /// An authenticated read failed; its typed HTTP cause retains access semantics.
    ReadFailed,
    /// An authenticated resource request was denied with HTTP 403.
    AccessDenied,
    /// An authenticated resource request returned HTTP 404.
    NotFound,
    /// The protected entry changed during one identity-bound book operation.
    Changed,
}

impl SessionErrorKind {
    /// Returns the fixed machine-readable category shared by both adapters.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "session_absent",
            Self::Rejected => "session_rejected",
            Self::Expired => "session_expired",
            Self::InvalidMaterial => "invalid_material",
            Self::UnsupportedContract => "unsupported_contract",
            Self::StorageAccess => "storage_access",
            Self::StorageUnavailable => "storage_unavailable",
            Self::AmbiguousEntry => "ambiguous_entry",
            Self::StorageFailure => "storage_failure",
            Self::ValidationUnavailable => "validation_unavailable",
            Self::TimedOut => "timed_out",
            Self::UnsupportedPlatform => "unsupported_platform",
            Self::ReadFailed => "authenticated_read_failed",
            Self::AccessDenied => "access_denied",
            Self::NotFound => "record_not_found",
            Self::Changed => "session_changed",
        }
    }
}

/// Retains only already-redacted shared causes, never raw store or service errors.
#[derive(Debug)]
enum SessionCause {
    /// Keeps the protected-store classification.
    Storage(AuthError),
    /// Keeps the shared service parser or transport classification.
    Service(EstablishmentError),
    /// Keeps content denial distinct from rejected credentials.
    Read(HttpError),
    /// Retains the worker type behind a redacted source boundary.
    Worker(WorkerCause),
}

/// Retains task failure without exposing panic payloads through a source chain.
struct WorkerCause(tokio::task::JoinError);

impl fmt::Display for WorkerCause {
    /// Classifies only safe task states, never task panic text.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(if self.0.is_cancelled() {
            "storage worker was cancelled"
        } else {
            "storage worker failed"
        })
    }
}

impl fmt::Debug for WorkerCause {
    /// Omits the raw task error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for WorkerCause {}

/// Carries a stable category and safe recovery text without credential values.
pub struct SessionError {
    /// Controls output consistently across CLI and MCP.
    kind: SessionErrorKind,
    /// Preserves typed causes without exposing external strings.
    cause: Option<SessionCause>,
}

impl SessionError {
    /// Creates a local policy failure without a dependency cause.
    fn new(kind: SessionErrorKind) -> Self {
        Self { kind, cause: None }
    }

    /// Classifies native failures without flattening or formatting their causes.
    fn storage(error: AuthError) -> Self {
        let kind = match error.kind() {
            AuthErrorKind::InvalidSecret => SessionErrorKind::InvalidMaterial,
            AuthErrorKind::StorageAccess => SessionErrorKind::StorageAccess,
            AuthErrorKind::StorageUnavailable => SessionErrorKind::StorageUnavailable,
            AuthErrorKind::AmbiguousEntry => SessionErrorKind::AmbiguousEntry,
            AuthErrorKind::UnsupportedPlatform => SessionErrorKind::UnsupportedPlatform,
            _ => SessionErrorKind::StorageFailure,
        };
        Self {
            kind,
            cause: Some(SessionCause::Storage(error)),
        }
    }

    /// Maps the fixed shared service contract into session-use categories.
    fn service(error: EstablishmentError) -> Self {
        let kind = match error.code() {
            ErrorCode::SessionRejected => SessionErrorKind::Rejected,
            ErrorCode::SessionExpired => SessionErrorKind::Expired,
            ErrorCode::InvalidMaterial => SessionErrorKind::InvalidMaterial,
            ErrorCode::UnsupportedContract => SessionErrorKind::UnsupportedContract,
            _ => SessionErrorKind::ValidationUnavailable,
        };
        Self {
            kind,
            cause: Some(SessionCause::Service(error)),
        }
    }

    /// Keeps a blocking task failure without exposing its panic payload.
    fn worker(error: tokio::task::JoinError) -> Self {
        Self {
            kind: SessionErrorKind::StorageFailure,
            cause: Some(SessionCause::Worker(WorkerCause(error))),
        }
    }

    /// Preserves bounded read errors and classifies rejection, denial, and absence.
    fn read(error: HttpError) -> Self {
        let kind = match error.kind() {
            ErrorKind::SessionUnusable => SessionErrorKind::Rejected,
            ErrorKind::AccessDenied => SessionErrorKind::AccessDenied,
            ErrorKind::UpstreamStatus if error.status() == Some(404) => SessionErrorKind::NotFound,
            _ => SessionErrorKind::ReadFailed,
        };
        Self {
            kind,
            cause: Some(SessionCause::Read(error)),
        }
    }

    /// Returns the safe category without parsing presentation text.
    pub fn kind(&self) -> SessionErrorKind {
        self.kind
    }
}

impl fmt::Display for SessionError {
    /// Prints a fixed category and human recovery guidance, never external text.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.as_str())?;
        formatter.write_str(match self.kind {
            SessionErrorKind::Absent | SessionErrorKind::Rejected =>
                ": human browser login is required; run beyond-gate auth login; do not supply credentials to MCP",
            SessionErrorKind::Expired =>
                ": the service-issued bearer expired; cookie expiry is unknown",
            SessionErrorKind::StorageAccess =>
                ": unlock or allow access to your OS credential service",
            SessionErrorKind::StorageUnavailable | SessionErrorKind::StorageFailure =>
                ": check your OS credential service; no plaintext fallback is used",
            SessionErrorKind::AmbiguousEntry =>
                ": resolve duplicate entries in your OS credential manager",
            SessionErrorKind::UnsupportedPlatform =>
                ": production session validation and reuse are currently Windows-only",
            SessionErrorKind::AccessDenied => ": the service denied access to the selected resource",
            SessionErrorKind::NotFound => ": the selected resource was not found",
            SessionErrorKind::Changed =>
                ": session evidence changed during this read; retry the operation",
            _ => "",
        })
    }
}

impl fmt::Debug for SessionError {
    /// Uses exactly the safe ordinary diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for SessionError {
    /// Exposes only safe shared errors and their redacted cause chains.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.cause.as_ref()? {
            SessionCause::Storage(error) => Some(error),
            SessionCause::Service(error) => Some(error),
            SessionCause::Read(error) => Some(error),
            SessionCause::Worker(error) => Some(error),
        }
    }
}

/// Reports bounded service evidence without a token, account claim, or exact expiry.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SessionValidation {
    /// Fresh explicit validation and cached earlier evidence have different labels.
    validation: &'static str,
    /// Never substitutes bearer lifetime for reusable-cookie expiry.
    expiry: &'static str,
    /// Describes only accepted bearer evidence at the operation boundary.
    bearer_expiry: &'static str,
}

impl SessionValidation {
    /// Labels only accepted evidence, never stored presence or a failed save.
    fn fresh_evidence() -> Self {
        Self {
            validation: "service_validated",
            expiry: "unknown",
            bearer_expiry: "future",
        }
    }
}

/// Holds only the material needed to bind transient evidence to the fixed entry.
#[derive(Debug, Default)]
struct State {
    /// Owned cookie bytes are cleared on replacement or state removal.
    cookie: Option<BrowserSessionCookie>,
    /// Derived credentials are never written to protected storage.
    bearer: Option<ValidatedBearer>,
    /// Prevents automatic recovery with the same rejected material in this owner.
    rejected: bool,
}

/// Selects production storage lazily; synthetic constructors never open an OS store.
#[derive(Clone)]
enum Backend {
    /// Opening occurs only when a validated operation needs storage.
    Native,
    /// Exists only in test builds and explicitly enabled development fixtures.
    #[cfg(any(test, feature = "session-fixture"))]
    Synthetic(Arc<SessionStore>),
}

/// Owns one process-local cache shared by explicit validation and authenticated reads.
///
/// Each operation rereads the fixed entry. No caller can retrieve a bearer or
/// supply a production exchange URL. Dropping an operation cancels network work;
/// any already-running blocking native read may finish, but cannot install evidence.
/// Logout in another process is observed on the next load, not in-flight.
pub struct Session {
    /// Serializes validation, refresh, reads, and cache removal.
    state: Arc<Mutex<State>>,
    /// Does not contact a store at construction.
    backend: Backend,
    /// Pins the first protected load across one multi-request resource operation.
    binding: Option<Arc<Mutex<Option<BrowserSessionCookie>>>>,
    /// Synthetic service selection cannot be configured in a product invocation.
    #[cfg(any(test, feature = "session-fixture"))]
    exchange: Option<(reqwest::Client, String)>,
    /// Keeps only this owner's local synthetic service alive.
    #[cfg(any(test, feature = "session-fixture"))]
    fixture: Option<Arc<fixture::Service>>,
    /// Keeps only compiled synthetic library services alive; native owners have none.
    #[cfg(any(test, feature = "session-fixture"))]
    pub(crate) library_fixture: Option<Arc<crate::library::fixture::Source>>,
    /// Keeps only compiled synthetic character/campaign services alive.
    #[cfg(any(test, feature = "session-fixture"))]
    pub(crate) character_campaign_fixture: Option<Arc<crate::character_campaign::fixture::Source>>,
    /// Keeps only compiled synthetic class catalog services alive.
    #[cfg(any(test, feature = "session-fixture"))]
    pub(crate) class_catalog_fixture: Option<Arc<crate::class_catalog::fixture::Source>>,
    /// Keeps only compiled synthetic monster services alive.
    #[cfg(any(test, feature = "session-fixture"))]
    pub(crate) monster_fixture: Option<Arc<crate::monster::fixture::Source>>,
}

impl Default for Session {
    /// Constructs an empty owner without store, network, or browser activity.
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
            backend: Backend::Native,
            binding: None,
            #[cfg(any(test, feature = "session-fixture"))]
            exchange: None,
            #[cfg(any(test, feature = "session-fixture"))]
            fixture: None,
            #[cfg(any(test, feature = "session-fixture"))]
            library_fixture: None,
            #[cfg(any(test, feature = "session-fixture"))]
            character_campaign_fixture: None,
            #[cfg(any(test, feature = "session-fixture"))]
            class_catalog_fixture: None,
            #[cfg(any(test, feature = "session-fixture"))]
            monster_fixture: None,
        }
    }
}

impl fmt::Debug for Session {
    /// Does not expose credentials, cache state, store metadata, or service details.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Session { state: [REDACTED] }")
    }
}

impl Session {
    /// Shares the bearer cache while preventing account changes within one read operation.
    ///
    /// Construction does not open storage. The first protected load pins the
    /// cookie; subsequent loads must match. Nested scopes keep the same binding.
    pub(crate) fn read_scope(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            backend: self.backend.clone(),
            binding: Some(
                self.binding
                    .as_ref()
                    .map(Arc::clone)
                    .unwrap_or_else(|| Arc::new(Mutex::new(None))),
            ),
            #[cfg(any(test, feature = "session-fixture"))]
            exchange: self.exchange.clone(),
            #[cfg(any(test, feature = "session-fixture"))]
            fixture: self.fixture.clone(),
            #[cfg(any(test, feature = "session-fixture"))]
            library_fixture: self.library_fixture.clone(),
            #[cfg(any(test, feature = "session-fixture"))]
            character_campaign_fixture: self.character_campaign_fixture.clone(),
            #[cfg(any(test, feature = "session-fixture"))]
            class_catalog_fixture: self.class_catalog_fixture.clone(),
            #[cfg(any(test, feature = "session-fixture"))]
            monster_fixture: self.monster_fixture.clone(),
        }
    }

    /// Prevents unsupported production platforms from opening storage or networking.
    pub(crate) fn supported(&self) -> Result<(), SessionError> {
        match &self.backend {
            #[cfg(any(test, feature = "session-fixture"))]
            Backend::Synthetic(_) => Ok(()),
            Backend::Native if cfg!(windows) => Ok(()),
            Backend::Native => Err(SessionError::new(SessionErrorKind::UnsupportedPlatform)),
        }
    }

    /// Waits for the sole owner without extending the operation deadline.
    async fn lock(&self, deadline: Instant) -> Result<MutexGuard<'_, State>, SessionError> {
        timeout_at(deadline, self.state.lock())
            .await
            .map_err(|_| SessionError::new(SessionErrorKind::TimedOut))
    }

    /// Opens only the fixed protected entry off the executor, or clones a synthetic one.
    fn opener(&self) -> impl FnOnce() -> Result<Arc<SessionStore>, AuthError> + Send + 'static {
        #[cfg(any(test, feature = "session-fixture"))]
        let synthetic = match &self.backend {
            Backend::Synthetic(store) => Some(Arc::clone(store)),
            Backend::Native => None,
        };
        move || {
            #[cfg(any(test, feature = "session-fixture"))]
            if let Some(store) = synthetic {
                return Ok(store);
            }
            SessionStore::open().map(Arc::new)
        }
    }

    /// Loads and parses only the fixed entry within the protected-storage wait cap.
    async fn load_cookie(
        &self,
        deadline: Instant,
    ) -> Result<Option<BrowserSessionCookie>, SessionError> {
        if Instant::now() >= deadline {
            return Err(SessionError::new(SessionErrorKind::TimedOut));
        }
        let open = self.opener();
        let worker = tokio::task::spawn_blocking(move || open()?.load());
        let loaded = timeout_at(
            deadline.min(Instant::now() + Duration::from_secs(5)),
            worker,
        )
        .await
        .map_err(|_| SessionError::new(SessionErrorKind::TimedOut))?
        .map_err(SessionError::worker)?
        .map_err(SessionError::storage)?;
        let cookie = loaded
            .map(BrowserSessionCookie::from_persisted)
            .transpose()
            .map_err(SessionError::service)?;
        if let (Some(binding), Some(cookie)) = (&self.binding, &cookie) {
            let mut binding = timeout_at(deadline, binding.lock())
                .await
                .map_err(|_| SessionError::new(SessionErrorKind::TimedOut))?;
            if let Some(previous) = binding.as_ref() {
                if previous.0 != cookie.0 {
                    return Err(SessionError::new(SessionErrorKind::Changed));
                }
            } else {
                *binding = Some(BrowserSessionCookie(cookie.0.clone()));
            }
        }
        Ok(cookie)
    }

    /// Uses only the current cookie for a source page, never a bearer fallback or exchange.
    pub(crate) async fn get_book_html(
        &self,
        client: &HttpClient,
        url: &str,
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, SessionError> {
        self.supported()?;
        let target = client.check_url(url).map_err(SessionError::read)?;
        let website = target.origin().ascii_serialization() == "https://www.dndbeyond.com";
        #[cfg(any(test, feature = "session-fixture"))]
        let website = website
            || (matches!(&self.backend, Backend::Synthetic(_))
                && target.scheme() == "http"
                && matches!(target.host_str(), Some("127.0.0.1" | "[::1]")));
        if !website
            || target.query().is_some()
            || !target.path().starts_with("/sources/")
            || !target.path().split('/').skip(1).all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            })
            || self.binding.is_none()
        {
            return Err(SessionError::new(SessionErrorKind::UnsupportedContract));
        }
        self.get_cookie_html(client, url).await
    }

    /// Reads only the fixed monster listing and known filters using this scope's cookie.
    pub(crate) async fn get_monster_html(
        &self,
        client: &HttpClient,
        url: &str,
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, SessionError> {
        self.supported()?;
        let target = client.check_url(url).map_err(SessionError::read)?;
        let website = target.origin().ascii_serialization() == "https://www.dndbeyond.com";
        #[cfg(any(test, feature = "session-fixture"))]
        let website = website
            || (matches!(&self.backend, Backend::Synthetic(_))
                && target.scheme() == "http"
                && matches!(target.host_str(), Some("127.0.0.1" | "[::1]")));
        if !website
            || !target.username().is_empty()
            || target.password().is_some()
            || !crate::monster::query::allowed_list_url(&target)
            || self.binding.is_none()
        {
            return Err(SessionError::new(SessionErrorKind::UnsupportedContract));
        }
        self.get_cookie_html(client, url).await
    }

    /// Shares cookie/session checks after the caller applies its resource-specific URL guard.
    async fn get_cookie_html(
        &self,
        client: &HttpClient,
        url: &str,
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, SessionError> {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut state = self.lock(deadline).await?;
        let prior_bearer = state.bearer.take();
        let Some(cookie) = self.load_cookie(deadline).await? else {
            *state = State::default();
            return Err(SessionError::new(SessionErrorKind::Absent));
        };
        if !state
            .cookie
            .as_ref()
            .is_some_and(|previous| previous.0 == cookie.0)
        {
            return Err(SessionError::new(SessionErrorKind::Changed));
        }
        if state.rejected {
            return Err(SessionError::new(SessionErrorKind::Rejected));
        }
        let bearer = prior_bearer
            .ok_or_else(|| SessionError::new(SessionErrorKind::ValidationUnavailable))?;
        if !bearer.usable(SystemTime::now()) {
            return Err(SessionError::new(SessionErrorKind::Expired));
        }
        let headers = crate::http::SessionHeaders::cookie(
            "CobaltSession",
            std::str::from_utf8(&cookie.0)
                .map_err(|_| SessionError::new(SessionErrorKind::InvalidMaterial))?,
        )
        .map_err(SessionError::read)?;
        let read_deadline = deadline.min(bearer.deadline());
        state.cookie = Some(cookie);
        state.bearer = Some(bearer);
        let result = timeout_at(read_deadline, client.get_html(url, &headers)).await;
        match result {
            Ok(Ok(body))
                if state
                    .bearer
                    .as_ref()
                    .is_some_and(|bearer| bearer.usable(SystemTime::now())) =>
            {
                Ok(body)
            }
            Ok(Ok(_)) => {
                state.bearer = None;
                Err(SessionError::new(SessionErrorKind::Expired))
            }
            Ok(Err(error)) => {
                let error = SessionError::read(error);
                if error.kind() == SessionErrorKind::Rejected {
                    state.bearer = None;
                    state.rejected = true;
                }
                Err(error)
            }
            Err(_) => {
                let expired = state
                    .bearer
                    .as_ref()
                    .is_some_and(|bearer| !bearer.usable(SystemTime::now()));
                state.bearer = None;
                Err(SessionError::new(if expired {
                    SessionErrorKind::Expired
                } else {
                    SessionErrorKind::TimedOut
                }))
            }
        }
    }

    /// Clears prior evidence before loading so failures or cancellation cannot restore it.
    async fn prepare(
        &self,
        state: &mut State,
        deadline: Instant,
        fresh: bool,
    ) -> Result<(), SessionError> {
        let prior_bearer = state.bearer.take();
        let Some(cookie) = self.load_cookie(deadline).await? else {
            *state = State::default();
            return Err(SessionError::new(SessionErrorKind::Absent));
        };
        let same = state
            .cookie
            .as_ref()
            .is_some_and(|previous| previous.0 == cookie.0);
        state.cookie = Some(cookie);
        state.rejected &= same;
        if state.rejected {
            return Err(SessionError::new(SessionErrorKind::Rejected));
        }
        if !fresh
            && same
            && prior_bearer
                .as_ref()
                .is_some_and(|bearer| bearer.usable(SystemTime::now()))
        {
            state.bearer = prior_bearer;
            return Ok(());
        }
        drop(prior_bearer);
        let cookie = state
            .cookie
            .as_ref()
            .ok_or_else(|| SessionError::new(SessionErrorKind::InvalidMaterial))?;
        let result = self.exchange(cookie, deadline).await;
        match result {
            Ok(bearer) if bearer.usable(SystemTime::now()) && Instant::now() < deadline => {
                state.bearer = Some(bearer);
                Ok(())
            }
            Ok(_) => Err(SessionError::new(SessionErrorKind::Expired)),
            Err(error) => {
                state.rejected = error.kind() == SessionErrorKind::Rejected;
                Err(error)
            }
        }
    }

    /// Sends at most one fixed-service exchange under the whole-operation deadline.
    async fn exchange(
        &self,
        cookie: &BrowserSessionCookie,
        deadline: Instant,
    ) -> Result<ValidatedBearer, SessionError> {
        #[cfg(any(test, feature = "session-fixture"))]
        let (client, url) = match &self.exchange {
            Some((client, url)) => (client.clone(), url.as_str()),
            None => (
                service::client().map_err(SessionError::service)?,
                service::SERVICE_URL,
            ),
        };
        #[cfg(not(any(test, feature = "session-fixture")))]
        let (client, url) = (
            service::client().map_err(SessionError::service)?,
            service::SERVICE_URL,
        );
        timeout_at(deadline, service::exchange(cookie, &client, url))
            .await
            .map_err(|_| SessionError::new(SessionErrorKind::TimedOut))?
            .map_err(SessionError::service)
    }

    /// Makes one fresh service exchange and returns only safe validation evidence.
    ///
    /// # Errors
    ///
    /// Returns distinct protected-store, absent, rejected, expired-bearer,
    /// invalid-contract, unavailable-validation, deadline, or unsupported failures.
    /// It never saves credentials or uses cached evidence as current acceptance.
    pub async fn validate(&self) -> Result<SessionValidation, SessionError> {
        self.supported()?;
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut state = self.lock(deadline).await?;
        self.prepare(&mut state, deadline, true).await?;
        Ok(SessionValidation::fresh_evidence())
    }

    /// Returns only the accepted account identifier for a fixed source request.
    ///
    /// The caller must keep this `Session` scoped across the matching resource
    /// request. The identifier is never returned by an adapter or persisted.
    ///
    /// # Errors
    ///
    /// Returns protected-store, session-establishment, expiry, or platform failures.
    pub(crate) async fn account_id(&self) -> Result<u64, SessionError> {
        self.supported()?;
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut state = self.lock(deadline).await?;
        self.prepare(&mut state, deadline, false).await?;
        state
            .bearer
            .as_ref()
            .filter(|bearer| bearer.usable(SystemTime::now()))
            .map(ValidatedBearer::account_id)
            .ok_or_else(|| SessionError::new(SessionErrorKind::Expired))
    }

    /// Reads bounded JSON with shared session reuse and the client's exact origin policy.
    ///
    /// No production resource host is added here. The caller must establish a
    /// resource contract and construct the bounded HTTP client. HTTP 403 remains
    /// content denial; HTTP 401 stops reuse without an automatic exchange retry.
    ///
    /// # Errors
    ///
    /// Returns session lifecycle failures or a redacted typed HTTP read cause.
    /// Expiry cancels this operation; it does not claim that the cookie expired.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        client: &HttpClient,
        url: &str,
    ) -> Result<T, SessionError> {
        self.get_json_counted(client, url)
            .await
            .map(|(value, _)| value)
    }

    /// Keeps all protected read checks while reporting actual admitted resource bytes.
    pub(crate) async fn get_json_counted<T: DeserializeOwned>(
        &self,
        client: &HttpClient,
        url: &str,
    ) -> Result<(T, usize), SessionError> {
        self.supported()?;
        client.check_url(url).map_err(SessionError::read)?;
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut state = self.lock(deadline).await?;
        self.prepare(&mut state, deadline, false).await?;
        let bearer = state
            .bearer
            .as_ref()
            .ok_or_else(|| SessionError::new(SessionErrorKind::ValidationUnavailable))?;
        if !bearer.usable(SystemTime::now()) {
            state.bearer = None;
            return Err(SessionError::new(SessionErrorKind::Expired));
        }
        let headers = bearer.headers().map_err(SessionError::service)?;
        let read_deadline = deadline.min(bearer.deadline());
        let result = timeout_at(read_deadline, client.get_json_counted(url, Some(&headers))).await;
        match result {
            Ok(Ok(value)) => {
                if !state
                    .bearer
                    .as_ref()
                    .is_some_and(|bearer| bearer.usable(SystemTime::now()))
                {
                    state.bearer = None;
                    return Err(SessionError::new(SessionErrorKind::Expired));
                }
                Ok(value)
            }
            Ok(Err(error)) => {
                let error = SessionError::read(error);
                if error.kind() == SessionErrorKind::Rejected {
                    state.bearer = None;
                    state.rejected = true;
                }
                Err(error)
            }
            Err(_) => {
                let expired = state
                    .bearer
                    .as_ref()
                    .is_some_and(|bearer| !bearer.usable(SystemTime::now()));
                state.bearer = None;
                Err(SessionError::new(if expired {
                    SessionErrorKind::Expired
                } else {
                    SessionErrorKind::TimedOut
                }))
            }
        }
    }

    /// Clears this owner's cache and removes only the fixed local protected entry.
    ///
    /// # Errors
    ///
    /// Returns a safe storage or timeout failure if deletion cannot be confirmed.
    /// A started blocking delete can finish after cancellation. Local logout does
    /// not revoke upstream credentials or stop another process's in-flight request.
    pub async fn logout(&self) -> Result<(), SessionError> {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut state = self.lock(deadline).await?;
        *state = State::default();
        if Instant::now() >= deadline {
            return Err(SessionError::new(SessionErrorKind::TimedOut));
        }
        let open = self.opener();
        let worker = tokio::task::spawn_blocking(move || open()?.logout());
        timeout_at(
            deadline.min(Instant::now() + Duration::from_secs(5)),
            worker,
        )
        .await
        .map_err(|_| SessionError::new(SessionErrorKind::TimedOut))?
        .map_err(SessionError::worker)?
        .map_err(SessionError::storage)
    }
}

/// Runs invented sessions against an in-memory store and a local mock service.
#[cfg(any(test, feature = "session-fixture"))]
pub mod fixture;

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

#[cfg(all(test, windows))]
#[path = "session/library_probe.rs"]
mod library_probe;
