//! Typed HTTP failures with safe diagnostics, including their source chain.

use std::{error::Error, fmt, io, time::Duration};

/// Identifies a failure without including request or response data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// The client policy, allowed origin, URL, or session header is invalid.
    InvalidInput,
    /// The request origin is not explicitly allowed.
    UnapprovedOrigin,
    /// Building the certificate-validated HTTP client failed.
    ClientSetup,
    /// A connection, DNS, TLS, or response transport failed.
    Network,
    /// A connection, attempt, queue, or total deadline elapsed.
    Timeout,
    /// HTTP 429 exhausted the retry budget or required too long a wait.
    RateLimited,
    /// HTTP 401 rejected the session; expiry is not independently established.
    SessionUnusable,
    /// HTTP 403 denied access.
    AccessDenied,
    /// A redirect was rejected without following its destination.
    Redirect,
    /// The response exceeded the configured byte limit.
    BodyTooLarge,
    /// JSON or content encoding did not meet the response contract.
    MalformedResponse,
    /// Another non-success upstream status was received.
    UpstreamStatus,
}

/// Classifies a retained cause without printing its potentially private details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CauseKind {
    /// An HTTP transport or client construction error.
    Transport,
    /// A JSON syntax, data, I/O, or end-of-input error.
    Json(serde_json::error::Category),
    /// A Tokio operation deadline elapsed.
    Deadline,
}

/// Holds the original typed cause behind a deliberately redacted error boundary.
///
/// Raw serde errors can include private field values. Raw transport source chains
/// can include URLs. This type does not expose either through `Error::source`.
pub struct FailureCause {
    /// Retains the original cause for classification, not diagnostic formatting.
    inner: Cause,
}

/// Stores the concrete cause without deriving secret-bearing diagnostics.
enum Cause {
    Transport(reqwest::Error),
    Json(serde_json::Error),
    Deadline(tokio::time::error::Elapsed),
}

impl FailureCause {
    /// Returns safe type information about the retained cause.
    pub fn kind(&self) -> CauseKind {
        match &self.inner {
            Cause::Transport(_) => CauseKind::Transport,
            Cause::Json(error) => CauseKind::Json(error.classify()),
            Cause::Deadline(_) => CauseKind::Deadline,
        }
    }

    /// Returns a JSON line and column without returning any response text.
    pub fn json_position(&self) -> Option<(usize, usize)> {
        match &self.inner {
            Cause::Json(error) => Some((error.line(), error.column())),
            _ => None,
        }
    }

    /// Reports whether a transport cause was a timeout.
    pub fn is_timeout(&self) -> bool {
        match &self.inner {
            Cause::Transport(error) => error.is_timeout(),
            Cause::Deadline(_) => true,
            Cause::Json(_) => false,
        }
    }

    /// Reports whether a transport cause occurred while connecting.
    pub fn is_connect(&self) -> bool {
        matches!(&self.inner, Cause::Transport(error) if error.is_connect())
    }

    /// Reports whether a transport cause was a DNS lookup failure.
    pub fn is_dns(&self) -> bool {
        matches!(&self.inner, Cause::Transport(error) if error.is_dns())
    }

    /// Selects conservative transient causes without retrying TLS or DNS failures.
    pub(super) fn is_transient(&self) -> bool {
        let Cause::Transport(error) = &self.inner else {
            return false;
        };
        if error.is_timeout() {
            return true;
        }
        let mut source = error.source();
        while let Some(cause) = source {
            if let Some(io) = cause.downcast_ref::<io::Error>() {
                return matches!(
                    io.kind(),
                    io::ErrorKind::ConnectionRefused
                        | io::ErrorKind::ConnectionReset
                        | io::ErrorKind::ConnectionAborted
                        | io::ErrorKind::BrokenPipe
                        | io::ErrorKind::UnexpectedEof
                        | io::ErrorKind::Interrupted
                        | io::ErrorKind::TimedOut
                );
            }
            source = cause.source();
        }
        false
    }
}

impl fmt::Display for FailureCause {
    /// Prints safe cause metadata, never the underlying error message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} cause; private details omitted",
            self.kind()
        )
    }
}

impl fmt::Debug for FailureCause {
    /// Keeps debug output as safe as ordinary diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for FailureCause {}

/// Reports an HTTP failure, attempt count, status, and safe typed cause.
///
/// An attempt is counted when a request is submitted, not while waiting for a
/// concurrency permit. Inspect the kind for decisions; do not parse Display.
pub struct HttpError {
    /// Determines the user action without private upstream data.
    pub(super) kind: ErrorKind,
    /// Records a response status when one was received.
    pub(super) status: Option<u16>,
    /// Counts submitted attempts, including the first request.
    pub(super) attempts: u8,
    /// Records a valid, representable server-requested minimum wait.
    pub(super) retry_after: Option<Duration>,
    /// Retains the original failure behind safe diagnostic formatting.
    cause: Option<FailureCause>,
}

impl HttpError {
    /// Returns the stable category of this failure.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the received HTTP status, if available.
    pub fn status(&self) -> Option<u16> {
        self.status
    }

    /// Returns the number of submitted attempts.
    pub fn attempts(&self) -> u8 {
        self.attempts
    }

    /// Returns the server's minimum delay, if valid and representable.
    pub fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }

    /// Returns the typed, redacted cause without exposing its raw error.
    pub fn cause(&self) -> Option<&FailureCause> {
        self.cause.as_ref()
    }

    /// Creates a failure with no underlying private data.
    pub(super) fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            status: None,
            attempts: 0,
            retry_after: None,
            cause: None,
        }
    }

    /// Removes the transport URL and retains the cause behind redacted output.
    pub(super) fn transport(error: reqwest::Error) -> Self {
        let kind = if error.is_timeout() {
            ErrorKind::Timeout
        } else {
            ErrorKind::Network
        };
        let mut failure = Self::new(kind);
        failure.cause = Some(FailureCause {
            inner: Cause::Transport(error.without_url()),
        });
        failure
    }

    /// Keeps JSON classification without exposing private response values.
    pub(super) fn json(error: serde_json::Error) -> Self {
        let mut failure = Self::new(ErrorKind::MalformedResponse);
        failure.cause = Some(FailureCause {
            inner: Cause::Json(error),
        });
        failure
    }

    /// Retains an elapsed deadline as a typed timeout cause.
    pub(super) fn deadline(error: tokio::time::error::Elapsed) -> Self {
        let mut failure = Self::new(ErrorKind::Timeout);
        failure.cause = Some(FailureCause {
            inner: Cause::Deadline(error),
        });
        failure
    }

    /// Determines whether retrying a safe read can recover this failure.
    pub(super) fn is_retryable(&self) -> bool {
        matches!(self.status, Some(429 | 502 | 503 | 504))
            || self.cause.as_ref().is_some_and(FailureCause::is_transient)
    }
}

impl fmt::Display for HttpError {
    /// Prints only category, status, and attempt count.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "HTTP read failed: {:?}", self.kind)?;
        if let Some(status) = self.status {
            write!(formatter, " (HTTP {status})")?;
        }
        write!(formatter, "; {} attempt(s)", self.attempts)?;
        if self.kind == ErrorKind::SessionUnusable {
            write!(formatter, "; human login is required")?;
        }
        Ok(())
    }
}

impl fmt::Debug for HttpError {
    /// Avoids deriving Debug over retained transport and JSON causes.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for HttpError {
    /// Returns a typed source whose own diagnostic chain remains redacted.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|cause| cause as &dyn Error)
    }
}
