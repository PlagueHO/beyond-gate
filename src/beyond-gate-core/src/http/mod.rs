//! Bounded async JSON reads with explicit origin and session boundaries.
//!
//! No upstream origin or resource endpoint is built in. Neither adapter uses this
//! boundary yet. Local mocks establish policy behavior, not an upstream contract.

mod error;

pub use error::{CauseKind, ErrorKind, FailureCause, HttpError};

use std::{fmt, sync::Arc, time::Duration};

use reqwest::{
    Client, Response, Url,
    header::{
        ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_ENCODING, COOKIE, HeaderValue, RETRY_AFTER,
    },
};
use serde::de::DeserializeOwned;
use tokio::{
    sync::Semaphore,
    time::{Instant, sleep, timeout_at},
};

/// Holds only the credential headers supplied by a future session operation.
///
/// Values are memory-only and marked sensitive. No login, refresh, browser access,
/// or persistent storage is performed. Debug never includes the header values.
#[derive(Clone)]
pub struct SessionHeaders {
    /// Supplies a bearer token without a global client default.
    authorization: Option<HeaderValue>,
    /// Supplies a single validated cookie, not an unrestricted header bag.
    cookie: Option<HeaderValue>,
}

impl SessionHeaders {
    /// Validates a bearer token and marks the resulting header as sensitive.
    ///
    /// # Errors
    ///
    /// Returns `InvalidInput` for empty, oversized, or invalid token input.
    pub fn bearer(token: &str) -> Result<Self, HttpError> {
        if token.is_empty()
            || token.len() > 8 * 1024
            || !token.bytes().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'+' | b'/' | b'=')
            })
        {
            return Err(HttpError::new(ErrorKind::InvalidInput));
        }
        Ok(Self {
            authorization: Some(sensitive_header(&format!("Bearer {token}"))?),
            cookie: None,
        })
    }

    /// Validates one cookie name and value without accepting a cookie list.
    ///
    /// # Errors
    ///
    /// Returns `InvalidInput` for empty, oversized, or unsafe input.
    pub fn cookie(name: &str, value: &str) -> Result<Self, HttpError> {
        if name.is_empty()
            || value.is_empty()
            || name.len().saturating_add(value.len()) > 8 * 1024
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || !value.bytes().all(
                |byte| matches!(byte, 0x21 | 0x23..=0x2b | 0x2d..=0x3a | 0x3c..=0x5b | 0x5d..=0x7e),
            )
        {
            return Err(HttpError::new(ErrorKind::InvalidInput));
        }
        Ok(Self {
            authorization: None,
            cookie: Some(sensitive_header(&format!("{name}={value}"))?),
        })
    }
}

impl fmt::Debug for SessionHeaders {
    /// Omits all credential values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionHeaders { credentials: [REDACTED] }")
    }
}

/// Builds a sensitive header without retaining invalid input in an error.
fn sensitive_header(value: &str) -> Result<HeaderValue, HttpError> {
    let mut header =
        HeaderValue::from_str(value).map_err(|_| HttpError::new(ErrorKind::InvalidInput))?;
    header.set_sensitive(true);
    Ok(header)
}

/// Contains only the limits used by this concrete read boundary.
#[derive(Clone, Copy)]
struct RequestPolicy {
    /// Bounds connection establishment.
    connect: Duration,
    /// Bounds a single request through body consumption.
    attempt: Duration,
    /// Bounds queue wait, all attempts, and retry delays together.
    total: Duration,
    /// Bounds collected response bytes.
    bytes: usize,
    /// Bounds active logical reads, including retry waits.
    concurrency: usize,
    /// Bounds retries in addition to the initial attempt.
    retries: u8,
}

impl Default for RequestPolicy {
    /// Supplies local safety limits, not claims about upstream limits.
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(5),
            attempt: Duration::from_secs(15),
            total: Duration::from_secs(30),
            bytes: 1024 * 1024,
            concurrency: 4,
            retries: 2,
        }
    }
}

impl RequestPolicy {
    /// Rejects invalid values and any widening of the agreed safety caps.
    fn validate(self) -> Result<Self, HttpError> {
        let caps = Self::default();
        if self.connect.is_zero()
            || self.attempt.is_zero()
            || self.total.is_zero()
            || self.connect > caps.connect
            || self.attempt > caps.attempt
            || self.total > caps.total
            || self.bytes == 0
            || self.bytes > caps.bytes
            || self.concurrency == 0
            || self.concurrency > caps.concurrency
            || self.retries > caps.retries
        {
            return Err(HttpError::new(ErrorKind::InvalidInput));
        }
        Ok(self)
    }
}

/// Reuses a connection pool and a shared concurrency bound for safe JSON reads.
///
/// Construction requires an explicit list of exact HTTPS origins. An empty list
/// denies every request. This crate ships no approved D&D Beyond host list.
///
/// Clones share their permits; cloning does not increase concurrency.
///
/// # Examples
///
/// ```
/// use beyond_gate_core::http::HttpClient;
///
/// // This configuration cannot send a request to any origin.
/// let client = HttpClient::new(&[])?;
/// # Ok::<(), beyond_gate_core::http::HttpError>(())
/// ```
#[derive(Clone)]
pub struct HttpClient {
    /// Avoids rebuilding a pool or changing its safety settings per attempt.
    client: Client,
    /// Stores normalized scheme, host, and effective port without credentials.
    origins: Arc<[String]>,
    /// Applies to all clones of this client.
    permits: Arc<Semaphore>,
    /// Keeps local request limits private.
    policy: RequestPolicy,
}

impl fmt::Debug for HttpClient {
    /// Does not expose client internals, origins, or request data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HttpClient { configuration: [REDACTED] }")
    }
}

impl HttpClient {
    /// Builds a bounded client from exact HTTPS origins such as `https://example.com`.
    ///
    /// Origins must not contain paths other than `/`, userinfo, queries, or
    /// fragments. Listing an origin is configuration, not upstream permission.
    ///
    /// # Errors
    ///
    /// Returns `InvalidInput` for invalid origins or `ClientSetup` if TLS or
    /// resolver initialization fails.
    pub fn new(allowed_origins: &[&str]) -> Result<Self, HttpError> {
        Self::build(allowed_origins, RequestPolicy::default(), false)
    }

    /// Validates configuration and keeps the loopback HTTP exception test-only.
    fn build(
        allowed_origins: &[&str],
        policy: RequestPolicy,
        test_loopback: bool,
    ) -> Result<Self, HttpError> {
        let policy = policy.validate()?;
        let mut origins = Vec::new();
        for origin in allowed_origins {
            let url = parse_url(origin, test_loopback)?;
            if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
                return Err(HttpError::new(ErrorKind::InvalidInput));
            }
            origins.push(url.origin().ascii_serialization());
        }
        let client = Client::builder()
            .https_only(!test_loopback)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .referer(false)
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .connection_verbose(false)
            .connect_timeout(policy.connect)
            .timeout(policy.attempt)
            .pool_max_idle_per_host(policy.concurrency)
            .user_agent(concat!("beyond-gate/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| {
                let mut failure = HttpError::transport(error);
                failure.kind = ErrorKind::ClientSetup;
                failure
            })?;
        Ok(Self {
            client,
            origins: origins.into(),
            permits: Arc::new(Semaphore::new(policy.concurrency)),
            policy,
        })
    }

    /// Reads bounded JSON from an allowed origin using optional session headers.
    ///
    /// Only GET is supported. Redirects are never followed. The total deadline
    /// includes queue waiting and retries. Dropping this future cancels its work
    /// and releases its concurrency permit.
    ///
    /// # Errors
    ///
    /// Returns a typed policy, transport, status, size, timeout, or JSON failure.
    /// HTTP 401 means the session was rejected, not proof of a known expiry time.
    ///
    /// # Panics
    ///
    /// Panics if polled without a Tokio runtime with time and I/O enabled.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        session: Option<&SessionHeaders>,
    ) -> Result<T, HttpError> {
        let deadline = Instant::now() + self.policy.total;
        let url = parse_url(url, cfg!(test))?;
        if !self
            .origins
            .iter()
            .any(|origin| *origin == url.origin().ascii_serialization())
        {
            return Err(HttpError::new(ErrorKind::UnapprovedOrigin));
        }
        let _permit = timeout_at(deadline, self.permits.acquire())
            .await
            .map_err(HttpError::deadline)?
            .map_err(|_| HttpError::new(ErrorKind::ClientSetup))?;
        let mut attempts = 0;
        loop {
            if Instant::now() >= deadline {
                let mut failure = HttpError::new(ErrorKind::Timeout);
                failure.attempts = attempts;
                return Err(failure);
            }
            attempts += 1;
            let result = timeout_at(deadline, self.attempt(&url, session)).await;
            let mut failure = match result {
                Ok(Ok(value)) if Instant::now() < deadline => return Ok(value),
                Ok(Ok(_)) => HttpError::new(ErrorKind::Timeout),
                Ok(Err(failure)) => failure,
                Err(error) => HttpError::deadline(error),
            };
            failure.attempts = attempts;
            if attempts > self.policy.retries || !failure.is_retryable() {
                return Err(failure);
            }
            let backoff = Duration::from_millis(250 * (1_u64 << (attempts - 1)));
            let delay = failure.retry_after.unwrap_or(backoff).max(backoff);
            let remaining = deadline.saturating_duration_since(Instant::now());
            if delay >= remaining {
                return Err(failure);
            }
            sleep(delay).await;
            if Instant::now() >= deadline {
                return Err(failure);
            }
        }
    }

    /// Sends one GET and checks status and bounded body before decoding JSON.
    async fn attempt<T: DeserializeOwned>(
        &self,
        url: &Url,
        session: Option<&SessionHeaders>,
    ) -> Result<T, HttpError> {
        let mut request = self
            .client
            .get(url.clone())
            .header(ACCEPT, "application/json")
            .header(ACCEPT_ENCODING, "identity");
        if let Some(session) = session {
            if let Some(header) = &session.authorization {
                request = request.header(AUTHORIZATION, header.clone());
            }
            if let Some(header) = &session.cookie {
                request = request.header(COOKIE, header.clone());
            }
        }
        let mut response = request.send().await.map_err(HttpError::transport)?;
        check_status(&response)?;
        if response
            .headers()
            .get_all(CONTENT_ENCODING)
            .iter()
            .any(|encoding| !encoding.as_bytes().eq_ignore_ascii_case(b"identity"))
        {
            return Err(HttpError::new(ErrorKind::MalformedResponse));
        }
        if response
            .content_length()
            .is_some_and(|size| size > self.policy.bytes as u64)
        {
            return Err(HttpError::new(ErrorKind::BodyTooLarge));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(HttpError::transport)? {
            if chunk.len() > self.policy.bytes - body.len() {
                return Err(HttpError::new(ErrorKind::BodyTooLarge));
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body).map_err(HttpError::json)
    }
}

/// Parses a destination without exposing invalid input in its error.
///
/// Only test builds can accept literal loopback HTTP, never a hostname that
/// requires DNS to establish whether it is local.
fn parse_url(input: &str, test_loopback: bool) -> Result<Url, HttpError> {
    let url = Url::parse(input).map_err(|_| HttpError::new(ErrorKind::InvalidInput))?;
    let loopback = cfg!(test)
        && test_loopback
        && url.scheme() == "http"
        && matches!(url.host_str(), Some("127.0.0.1" | "[::1]"));
    if (url.scheme() != "https" && !loopback)
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(HttpError::new(ErrorKind::InvalidInput));
    }
    Ok(url)
}

/// Maps statuses without reading or retaining a private upstream error body.
fn check_status(response: &Response) -> Result<(), HttpError> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let kind = match status.as_u16() {
        401 => ErrorKind::SessionUnusable,
        403 => ErrorKind::AccessDenied,
        429 => ErrorKind::RateLimited,
        300..=399 => ErrorKind::Redirect,
        _ => ErrorKind::UpstreamStatus,
    };
    let mut failure = HttpError::new(kind);
    failure.status = Some(status.as_u16());
    failure.retry_after = response
        .headers()
        .get_all(RETRY_AFTER)
        .iter()
        .filter_map(|value| retry_after(Some(value)))
        .max();
    Err(failure)
}

/// Parses a server wait; numeric overflow becomes a non-retryable-length wait.
///
/// Missing or malformed values use local backoff. An overflowing decimal value
/// must not shorten a valid server wait. Dates in the past permit immediate retry.
fn retry_after(value: Option<&HeaderValue>) -> Option<Duration> {
    let value = value?.to_str().ok()?.trim();
    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Some(Duration::from_secs(value.parse().unwrap_or(u64::MAX)));
    }
    let date = httpdate::parse_http_date(value).ok()?;
    Some(
        date.duration_since(std::time::SystemTime::now())
            .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests;
