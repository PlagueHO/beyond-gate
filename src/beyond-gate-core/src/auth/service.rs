//! Shared fixed-service exchange, cookie encoding, and redacted establishment errors.
//!
//! The accepted direct TLS response supplies bearer evidence, not an independent
//! signature check, reusable-cookie expiry, or permission to read content.

#[cfg(windows)]
use super::AuthError;
use super::{SessionSecret, handoff::ErrorCode};
use crate::http::{HttpError, SessionHeaders, direct_client_builder, validate_cookie};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, de::Visitor};
use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::Instant;
use zeroize::Zeroizing;

/// Fixes the only production exchange origin and path.
pub(super) const SERVICE_URL: &str = "https://auth-service.dndbeyond.com/v1/cobalt-token";
/// Includes the eight-byte format header in Windows' native blob limit.
pub(super) const MAX_STORED_BYTES: usize = 2_560;
/// Leaves room for magic, version, profile, and little-endian value length.
pub(crate) const MAX_COOKIE_BYTES: usize = MAX_STORED_BYTES - 8;
/// Bounds both the upstream response and every decoded allocation within it.
pub(super) const MAX_RESPONSE_BYTES: usize = 64 * 1024;
/// Caps the transient token independently of the response.
pub(super) const MAX_TOKEN_BYTES: usize = 8 * 1024;
/// Identifies the portable persistent-cookie encoding.
const MAGIC: &[u8; 4] = b"BGSE";
/// Owns one RFC 6265 value that fits the versioned Windows credential entry.
pub struct BrowserSessionCookie(pub(super) Zeroizing<Vec<u8>>);

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
    #[cfg(any(windows, test, feature = "session-fixture"))]
    pub(super) fn encode(&self) -> SessionSecret {
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

/// Preserves dependency types behind an inaccessible, redacted source boundary.
pub(super) enum Cause {
    /// Retains direct TLS and bounded-body failures.
    Transport(reqwest::Error),
    /// Retains shared header validation failures.
    Http(HttpError),
    /// Retains parser failures without reflected fields.
    Json(serde_json::Error),
    /// Retains encoding failures without the token.
    Encoding(base64::DecodeError),
    /// Retains the native store's already redacted classification.
    #[cfg(windows)]
    Storage(AuthError),
    /// Retains worker failure without task panic text.
    #[cfg(windows)]
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
            #[cfg(windows)]
            Self::Storage(error) => return fmt::Display::fmt(error, formatter),
            #[cfg(windows)]
            Self::Worker(error) if error.is_cancelled() => "storage worker was cancelled",
            #[cfg(windows)]
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
    pub(super) fn new(code: ErrorCode) -> Self {
        Self { code, cause: None }
    }

    /// Retains a typed failure with a safe source boundary.
    pub(super) fn caused(code: ErrorCode, cause: Cause) -> Self {
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

/// Retains the accepted positive account identifier only with process-local bearer evidence.
#[derive(Default)]
struct PositiveIdentifier(Option<u64>);

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
                Ok(PositiveIdentifier((value > 0).then_some(value)))
            }

            /// Rejects negative or zero numeric identifiers.
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(PositiveIdentifier(
                    u64::try_from(value).ok().filter(|value| *value > 0),
                ))
            }

            /// Accepts bounded decimal digits only, with a positive representable value.
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(PositiveIdentifier(
                    (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
                        .then(|| value.parse::<u64>().ok())
                        .flatten()
                        .filter(|number| *number > 0),
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

/// Owns service-issued credentials and both wall-clock and monotonic expiry limits.
pub(super) struct ValidatedBearer {
    /// Cleared when discarded; never persisted or returned to an adapter.
    token: Zeroizing<String>,
    /// Comes only from the accepted service response and stays process-local.
    account_id: u64,
    /// Comes only from the accepted direct response.
    expires: SystemTime,
    /// Prevents a backward wall-clock change from extending cached evidence.
    deadline: Instant,
}

impl ValidatedBearer {
    /// Returns the account identity bound to this accepted bearer.
    pub(super) fn account_id(&self) -> u64 {
        self.account_id
    }

    /// Rejects exact expiry and either clock reaching its original bound.
    pub(super) fn usable(&self, now: SystemTime) -> bool {
        now < self.expires && Instant::now() < self.deadline
    }

    /// Caps an authenticated read at the original service-issued bearer lifetime.
    pub(super) fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Constructs sensitive headers only for the shared bounded HTTP path.
    pub(super) fn headers(&self) -> Result<SessionHeaders, EstablishmentError> {
        SessionHeaders::bearer(&self.token).map_err(|error| {
            EstablishmentError::caused(ErrorCode::ValidationUnavailable, Cause::Http(error))
        })
    }
}

impl fmt::Debug for ValidatedBearer {
    /// Omits credential values, claims, and exact timestamps.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedBearer { evidence: [REDACTED] }")
    }
}

/// Applies the accepted direct-service rule without independently verifying a signature.
pub(super) fn validate_response(
    bytes: &[u8],
    now: SystemTime,
) -> Result<ValidatedBearer, EstablishmentError> {
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
    let account_id = claims
        .identifier
        .0
        .ok_or_else(|| EstablishmentError::new(ErrorCode::SessionRejected))?;
    let expiry = claims
        .exp
        .ok_or_else(|| EstablishmentError::new(ErrorCode::SessionRejected))?;
    let expiry = UNIX_EPOCH
        .checked_add(Duration::from_secs(expiry))
        .ok_or_else(|| EstablishmentError::new(ErrorCode::ValidationUnavailable))?;
    if expiry <= now {
        return Err(EstablishmentError::new(ErrorCode::SessionExpired));
    }
    let remaining = expiry
        .duration_since(now)
        .map_err(|_| EstablishmentError::new(ErrorCode::SessionExpired))?;
    let deadline = Instant::now()
        .checked_add(remaining)
        .ok_or_else(|| EstablishmentError::new(ErrorCode::ValidationUnavailable))?;
    Ok(ValidatedBearer {
        token: Zeroizing::new(token.to_owned()),
        account_id,
        expires: expiry,
        deadline,
    })
}

/// Sends exactly one bounded direct POST and returns memory-only bearer evidence.
pub(super) async fn exchange(
    cookie: &BrowserSessionCookie,
    client: &Client,
    url: &str,
) -> Result<ValidatedBearer, EstablishmentError> {
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

/// Builds the same fixed, direct transport for establishment and later session use.
pub(super) fn client() -> Result<Client, EstablishmentError> {
    direct_client_builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| {
            EstablishmentError::caused(ErrorCode::ValidationUnavailable, Cause::Transport(error))
        })
}
