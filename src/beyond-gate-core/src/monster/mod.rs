//! Bounded monster metadata pages and selected detail with fresh access checks.
//!
//! Lists preserve numeric access codes and add known labels without exposing raw content.
//! Detail requires observed access code 1. Code 4 is denied; other numeric codes
//! remain unsupported for detail. Raw detail is untrusted data, not a rules engine.

#[cfg(any(test, feature = "session-fixture"))]
pub mod fixture;
pub mod query;
mod resolve;
#[cfg(test)]
mod tests;
pub use query::MonsterQuery;

use crate::{
    auth::session::{Session, SessionError, SessionErrorKind},
    http::{HttpClient, HttpError},
    public_config::valid_text,
};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    error::Error,
    fmt,
    io::{self, Write},
    time::Duration,
};
use tokio::time::{Instant, timeout};

/// Local maximum number of raw records in one requested page.
pub const MAX_PAGE_RECORDS: usize = 100;
/// Maximum UTF-8 bytes in a literal upstream name search.
pub const MAX_NAME_BYTES: usize = 256;
/// Maximum compact JSON result bytes before adapter presentation.
pub const MAX_RESULT_BYTES: usize = 1024 * 1024;
/// Fixed service origin; returned links never select a request.
const SERVICE: &str = "https://monster-service.dndbeyond.com";
/// Fixed display-link origin, distinct from the service request origin.
const WEBSITE: &str = "https://www.dndbeyond.com";
/// Bounds queue, exchange, request, parsing, and result assembly together.
const DEADLINE: Duration = Duration::from_secs(30);

/// Redacted source, access, input, and bounded-result failures shared by adapters.
pub enum MonsterError {
    /// An ID, filter, name search, or page is invalid.
    InvalidInput,
    /// A source field, selected identity, or access map is malformed.
    InvalidResponse,
    /// A public lookup table is missing, malformed, or repeats an ID.
    InvalidConfiguration,
    /// Pagination does not describe the complete requested page.
    IncompletePage,
    /// A raw page repeats a source identity.
    ConflictingIdentity,
    /// The selected source route returned HTTP 404.
    MissingRecord,
    /// The source supplied the observed denied access code.
    AccessDenied,
    /// A numeric access code has not been verified for selected raw detail.
    UnsupportedAccess,
    /// Raw count or encoded result exceeds local policy.
    LimitExceeded,
    /// The whole operation reached its deadline.
    TimedOut,
    /// Keeps the typed, redacted HTTP client setup cause.
    SourceRead(HttpError),
    /// Keeps session, HTTP denial, expiry, and transport causes distinct.
    SessionRead(SessionError),
}

impl fmt::Display for MonsterError {
    /// Uses fixed diagnostics without raw records, URLs, or supplied selectors.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid monster input"),
            Self::InvalidResponse => formatter.write_str("invalid monster response"),
            Self::InvalidConfiguration => {
                formatter.write_str("invalid monster lookup configuration")
            }
            Self::IncompletePage => {
                formatter.write_str("monster page is incomplete or inconsistent")
            }
            Self::ConflictingIdentity => formatter.write_str("conflicting monster identities"),
            Self::MissingRecord => formatter.write_str("monster record not found"),
            Self::AccessDenied => formatter.write_str("monster access denied"),
            Self::UnsupportedAccess => formatter.write_str(
                "monster access code is not supported; only verified code 1 is accepted",
            ),
            Self::LimitExceeded => formatter.write_str("monster result limit exceeded"),
            Self::TimedOut => formatter.write_str("monster operation timed out"),
            Self::SourceRead(error) => write!(formatter, "monster source read failed: {error}"),
            Self::SessionRead(error) => write!(formatter, "monster session read failed: {error}"),
        }
    }
}

impl fmt::Debug for MonsterError {
    /// Omits source content and arbitrary dependency diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for MonsterError {
    /// Preserves typed causes without flattening access and session failures.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceRead(error) => Some(error),
            Self::SessionRead(error) => Some(error),
            _ => None,
        }
    }
}

/// Checks a canonical positive decimal source ID without interpreting a URL.
///
/// # Errors
///
/// Rejects leading zeroes, signs, whitespace, zero, and values beyond `u64`.
pub fn validate_id(id: &str) -> Result<(), MonsterError> {
    id.parse::<u64>()
        .ok()
        .filter(|value| *value > 0 && value.to_string() == id)
        .map(|_| ())
        .ok_or(MonsterError::InvalidInput)
}

/// Reads exactly one bounded page with explicit numeric continuation metadata.
///
/// Names are literal upstream searches. Every metadata row and numeric access
/// entry is checked before output. Reported codes are not detail-access grants.
/// Raw content is never included, even when its reported access code is 1.
///
/// # Errors
///
/// Returns input, page, access, limit, session, or transport failures. No partial
/// page or automatically fetched continuation is returned.
pub async fn list(session: &Session, query: &MonsterQuery) -> Result<Value, MonsterError> {
    query.validate()?;
    let session = session.read_scope();
    let started = Instant::now();
    timeout(DEADLINE, async {
        let (client, origin) = source_client(&session, SERVICE)?;
        let url = query.service_url(&origin)?;
        session
            .account_id()
            .await
            .map_err(MonsterError::SessionRead)?;
        let response = session
            .get_json(&client, url.as_str())
            .await
            .map_err(MonsterError::SessionRead)?;
        finish(parse_collection(response, query)?, started)
    })
    .await
    .map_err(|_| MonsterError::TimedOut)?
}

/// Reads one selected detail route, preserving every authorized raw source field.
///
/// List candidates and previous access observations are not grants for this read.
/// The selected response must supply access code 1 again.
/// Unless `raw` is true, one credential-free public configuration read then
/// supplies separate labels. Raw mode omits `resolved`, not the detail envelope.
///
/// # Errors
///
/// Returns input, missing, denied, unsupported-access, malformed, limit, session,
/// or transport failures. It does not substitute a list summary for detail.
pub async fn show(session: &Session, id: &str, raw: bool) -> Result<Value, MonsterError> {
    validate_id(id)?;
    let session = session.read_scope();
    let started = Instant::now();
    timeout(DEADLINE, async {
        let (client, origin) = source_client(&session, SERVICE)?;
        let response = session
            .get_json(&client, &format!("{origin}/v1/Monster/{id}"))
            .await
            .map_err(|error| {
                if error.kind() == SessionErrorKind::NotFound {
                    MonsterError::MissingRecord
                } else {
                    MonsterError::SessionRead(error)
                }
            })?;
        let mut result = parse_detail(response, id)?;
        if !raw {
            result["resolved"] = json!({});
            let resolved_limit = resolution_limit(&result)?;
            let (public, website) = source_client(&session, WEBSITE)?;
            let configuration: Value = public
                .get_json(&format!("{website}/api/config/json"), None)
                .await
                .map_err(MonsterError::SourceRead)?;
            result["resolved"] = resolve::detail(&result["raw"], &configuration, resolved_limit)?;
        }
        finish(result, started)
    })
    .await
    .map_err(|_| MonsterError::TimedOut)?
}

/// Selects only the fixed origin or a compiled, owner-scoped synthetic service.
fn source_client(session: &Session, origin: &str) -> Result<(HttpClient, String), MonsterError> {
    session.supported().map_err(MonsterError::SessionRead)?;
    #[cfg(any(test, feature = "session-fixture"))]
    if let Some(source) = &session.monster_fixture {
        return Ok((source.client.clone(), source.origin.clone()));
    }
    Ok((
        HttpClient::single_attempt(&[origin]).map_err(MonsterError::SourceRead)?,
        origin.to_owned(),
    ))
}

/// Requires the observed detail-access contract; metadata is never a grant.
fn detail_access(value: Option<&Value>) -> Result<(), MonsterError> {
    match value.and_then(Value::as_u64) {
        Some(1) => Ok(()),
        Some(4) => Err(MonsterError::AccessDenied),
        Some(_) => Err(MonsterError::UnsupportedAccess),
        None => Err(MonsterError::InvalidResponse),
    }
}

/// Labels reported website access without changing selected-detail authorization.
fn access_label(code: u64) -> &'static str {
    match code {
        1..=3 => "available",
        4 => "stat_block_locked",
        _ => "unknown",
    }
}

/// Checks complete pagination, exact numeric access keys, and metadata in source order.
fn parse_collection(value: Value, query: &MonsterQuery) -> Result<Value, MonsterError> {
    let entries = value["data"]
        .as_array()
        .ok_or(MonsterError::InvalidResponse)?;
    if entries.len() > MAX_PAGE_RECORDS {
        return Err(MonsterError::LimitExceeded);
    }
    let pagination = parse_pagination(&value["pagination"], query, entries.len())?;
    let codes = value["accessType"]
        .as_object()
        .ok_or(MonsterError::InvalidResponse)?;
    if codes.len() != entries.len() {
        return Err(MonsterError::InvalidResponse);
    }
    let mut seen = HashSet::new();
    let mut summaries = Vec::with_capacity(entries.len());
    for entry in entries {
        let mut summary = summary(entry)?;
        let id = summary["id"]
            .as_str()
            .ok_or(MonsterError::InvalidResponse)?;
        if !seen.insert(id.to_owned()) {
            return Err(MonsterError::ConflictingIdentity);
        }
        let code = codes
            .get(id)
            .and_then(Value::as_u64)
            .ok_or(MonsterError::InvalidResponse)?;
        summary["access_code"] = json!(code);
        summary["access"] = json!(access_label(code));
        summaries.push(summary);
    }
    Ok(json!({
        "entries": summaries,
        "pagination": pagination
    }))
}

/// Rejects incomplete or unrepresentable offsets without inventing an end marker.
fn parse_pagination(
    value: &Value,
    query: &MonsterQuery,
    count: usize,
) -> Result<Value, MonsterError> {
    let number = |field: &str| {
        value[field]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(MonsterError::InvalidResponse)
    };
    let (skip, take, total) = (number("skip")?, number("take")?, number("total")?);
    if skip != query.skip
        || take != query.take
        || count as u64 != u64::from(take.min(total.saturating_sub(skip)))
    {
        return Err(MonsterError::IncompletePage);
    }
    let next = if total.saturating_sub(skip) > take {
        Some(
            skip.checked_add(take)
                .ok_or(MonsterError::InvalidResponse)?,
        )
    } else {
        None
    };
    Ok(json!({"skip":skip,"take":take,"total":total,"next_skip":next}))
}

/// Requires a fresh scalar grant and exact selected identity before exposing raw data.
fn parse_detail(mut value: Value, id: &str) -> Result<Value, MonsterError> {
    if !value.is_object() {
        return Err(MonsterError::InvalidResponse);
    }
    detail_access(value.get("accessType"))?;
    let raw = value
        .get_mut("data")
        .ok_or(MonsterError::InvalidResponse)?
        .take();
    let mut result = summary(&raw)?;
    if result["id"].as_str() != Some(id) {
        return Err(MonsterError::InvalidResponse);
    }
    result["access_code"] = json!(1);
    result["access"] = json!(access_label(1));
    result["raw"] = raw;
    Ok(result)
}

/// Projects identity and attribution without inventing combat fields or access labels.
fn summary(value: &Value) -> Result<Value, MonsterError> {
    let fields = value.as_object().ok_or(MonsterError::InvalidResponse)?;
    let id = fields
        .get("id")
        .and_then(Value::as_u64)
        .filter(|id| *id > 0)
        .ok_or(MonsterError::InvalidResponse)?;
    let name = fields
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| valid_text(name))
        .ok_or(MonsterError::InvalidResponse)?;
    let link = fields
        .get("url")
        .and_then(Value::as_str)
        .ok_or(MonsterError::InvalidResponse)?;
    let url = source_url(&id.to_string(), link)?;
    Ok(json!({"id": id.to_string(), "name": name, "source_url": url}))
}

/// Checks canonical display attribution without using it as a fetch destination.
fn source_url(id: &str, link: &str) -> Result<String, MonsterError> {
    let url = reqwest::Url::parse(link).map_err(|_| MonsterError::InvalidResponse)?;
    let prefix = format!("/monsters/{id}");
    let suffix = url
        .path()
        .strip_prefix(&prefix)
        .ok_or(MonsterError::InvalidResponse)?;
    if !link.starts_with(&format!("{WEBSITE}/monsters/"))
        || url.as_str() != link
        || url.origin().ascii_serialization() != WEBSITE
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !(suffix.is_empty()
            || suffix.starts_with('-')
                && suffix.len() > 1
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'))
    {
        return Err(MonsterError::InvalidResponse);
    }
    Ok(url.to_string())
}

/// Bounds the complete canonical result and checks synchronous work against the deadline.
fn finish(value: Value, started: Instant) -> Result<Value, MonsterError> {
    if encoded_len(&value)? > MAX_RESULT_BYTES {
        return Err(MonsterError::LimitExceeded);
    }
    if started.elapsed() >= DEADLINE {
        return Err(MonsterError::TimedOut);
    }
    Ok(value)
}

/// Computes the remaining encoded-byte budget for the resolved object.
///
/// The value must contain an empty `resolved` object. Its compact JSON size
/// includes the complete envelope and raw detail, so only bytes available
/// beyond that placeholder can be used by lookup metadata.
fn resolution_limit(value: &Value) -> Result<usize, MonsterError> {
    let placeholder_size = encoded_len(value)?;
    MAX_RESULT_BYTES
        .checked_sub(placeholder_size)
        .and_then(|remaining| remaining.checked_add(2))
        .ok_or(MonsterError::LimitExceeded)
}

/// Counts compact JSON bytes without allocating a serialized copy.
pub(super) fn encoded_len<T: serde::Serialize + ?Sized>(value: &T) -> Result<usize, MonsterError> {
    let mut counter = ByteCounter(0);
    serde_json::to_writer(&mut counter, value).map_err(|_| MonsterError::InvalidResponse)?;
    Ok(counter.0)
}

/// Counts bytes written by the JSON serializer.
struct ByteCounter(usize);

impl Write for ByteCounter {
    /// Adds the bytes from one serializer write.
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("JSON byte count overflow"))?;
        Ok(bytes.len())
    }

    /// Accepts serializer flushes without changing the count.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
