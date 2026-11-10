//! Bounded monster pages and selected detail with explicitly verified access cases.
//!
//! Only observed access code 1 permits output. Code 4 is denied; other numeric
//! codes remain unsupported. Raw detail is untrusted data, not a rules engine.

#[cfg(any(test, feature = "session-fixture"))]
pub mod fixture;
pub mod query;
#[cfg(test)]
mod tests;
mod website;
pub use query::MonsterQuery;

use crate::{
    auth::session::{Session, SessionError, SessionErrorKind},
    http::{HttpClient, HttpError},
    public_config::valid_text,
};
use serde_json::{Value, json};
use std::{collections::HashMap, error::Error, fmt, time::Duration};
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
    /// Pagination does not describe the complete requested page.
    IncompletePage,
    /// A raw page repeats a source identity.
    ConflictingIdentity,
    /// The selected source route returned HTTP 404.
    MissingRecord,
    /// The source supplied the observed denied access code.
    AccessDenied,
    /// A numeric access code has not been verified for monster content.
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
/// Names are literal upstream searches. Every raw record and access entry is
/// checked before output; a denied or unsupported entry fails the whole page.
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
        let (client, origin) = source_client(&session)?;
        let website_origin = if origin == SERVICE { WEBSITE } else { &origin };
        let url = query.website_url(website_origin)?;
        session
            .account_id()
            .await
            .map_err(MonsterError::SessionRead)?;
        let body = session
            .get_monster_html(&client, url.as_str())
            .await
            .map_err(MonsterError::SessionRead)?;
        let page = website::parse(&body, query)?;
        if page.ids.is_empty() {
            return finish(
                json!({"entries": [], "pagination": {"page":query.page,"next_page":null}}),
                started,
            );
        }
        let mut selected = reqwest::Url::parse(&format!("{origin}/v1/Monster"))
            .map_err(|_| MonsterError::InvalidResponse)?;
        for id in &page.ids {
            selected.query_pairs_mut().append_pair("ids", id);
        }
        let response = session
            .get_json(&client, selected.as_str())
            .await
            .map_err(MonsterError::SessionRead)?;
        finish(
            parse_collection(response, &page.ids, query.page, page.next)?,
            started,
        )
    })
    .await
    .map_err(|_| MonsterError::TimedOut)?
}

/// Reads one selected detail route, preserving every authorized raw source field.
///
/// List candidates and previous access observations are not grants for this read.
/// The selected response must supply access code 1 again.
///
/// # Errors
///
/// Returns input, missing, denied, unsupported-access, malformed, limit, session,
/// or transport failures. It does not substitute a list summary for detail.
pub async fn show(session: &Session, id: &str) -> Result<Value, MonsterError> {
    validate_id(id)?;
    let session = session.read_scope();
    let started = Instant::now();
    timeout(DEADLINE, async {
        let (client, origin) = source_client(&session)?;
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
        finish(parse_detail(response, id)?, started)
    })
    .await
    .map_err(|_| MonsterError::TimedOut)?
}

/// Selects only the fixed origin or a compiled, owner-scoped synthetic service.
fn source_client(session: &Session) -> Result<(HttpClient, String), MonsterError> {
    session.supported().map_err(MonsterError::SessionRead)?;
    #[cfg(any(test, feature = "session-fixture"))]
    if let Some(source) = &session.monster_fixture {
        return Ok((source.client.clone(), source.origin.clone()));
    }
    Ok((
        HttpClient::single_attempt(&[SERVICE, WEBSITE]).map_err(MonsterError::SourceRead)?,
        SERVICE.to_owned(),
    ))
}

/// Requires the observed numeric access contract; unknown is never a grant.
fn access(value: Option<&Value>) -> Result<(), MonsterError> {
    match value.and_then(Value::as_u64) {
        Some(1) => Ok(()),
        Some(4) => Err(MonsterError::AccessDenied),
        Some(_) => Err(MonsterError::UnsupportedAccess),
        None => Err(MonsterError::InvalidResponse),
    }
}

/// Checks the exact selected set and fresh grants, then restores website order.
fn parse_collection(
    value: Value,
    ids: &[String],
    page: u32,
    next: Option<u32>,
) -> Result<Value, MonsterError> {
    let entries = value["data"]
        .as_array()
        .ok_or(MonsterError::InvalidResponse)?;
    if entries.len() > MAX_PAGE_RECORDS {
        return Err(MonsterError::LimitExceeded);
    }
    if entries.len() != ids.len() {
        return Err(MonsterError::IncompletePage);
    }
    if !value.get("pagination").is_some_and(Value::is_null) {
        return Err(MonsterError::InvalidResponse);
    }
    let grants = value["accessType"]
        .as_object()
        .ok_or(MonsterError::InvalidResponse)?;
    if grants.len() != entries.len() {
        return Err(MonsterError::InvalidResponse);
    }
    let mut summaries = HashMap::new();
    for entry in entries {
        let summary = summary(entry)?;
        let id = summary["id"]
            .as_str()
            .ok_or(MonsterError::InvalidResponse)?;
        if summaries.contains_key(id) {
            return Err(MonsterError::ConflictingIdentity);
        }
        access(grants.get(id))?;
        summaries.insert(id.to_owned(), summary);
    }
    let entries = ids
        .iter()
        .map(|id| summaries.remove(id).ok_or(MonsterError::InvalidResponse))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "entries": entries,
        "pagination": {"page": page, "next_page": next}
    }))
}

/// Requires a fresh scalar grant and exact selected identity before exposing raw data.
fn parse_detail(mut value: Value, id: &str) -> Result<Value, MonsterError> {
    if !value.is_object() {
        return Err(MonsterError::InvalidResponse);
    }
    access(value.get("accessType"))?;
    let raw = value
        .get_mut("data")
        .ok_or(MonsterError::InvalidResponse)?
        .take();
    let mut result = summary(&raw)?;
    if result["id"].as_str() != Some(id) {
        return Err(MonsterError::InvalidResponse);
    }
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
    Ok(json!({"id": id.to_string(), "name": name, "source_url": url, "access_code": 1}))
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
    if serde_json::to_vec(&value)
        .map_err(|_| MonsterError::InvalidResponse)?
        .len()
        > MAX_RESULT_BYTES
    {
        return Err(MonsterError::LimitExceeded);
    }
    if started.elapsed() >= DEADLINE {
        return Err(MonsterError::TimedOut);
    }
    Ok(value)
}
