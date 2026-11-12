//! Bounded, explicit website spell pages and selected untrusted HTML detail.

#[cfg(any(test, feature = "session-fixture"))]
pub mod fixture;
pub mod query;
#[cfg(test)]
mod tests;
mod website;

use crate::{
    auth::session::{Session, SessionError, SessionErrorKind},
    http::{HttpClient, HttpError},
};
pub use query::SpellQuery;
use serde_json::Value;
use std::{error::Error, fmt, time::Duration};

/// Maximum raw rows in one requested website page.
pub const MAX_RECORDS: usize = 100;
/// Maximum compact JSON bytes before presentation.
pub const MAX_RESULT_BYTES: usize = 1024 * 1024;
/// Fixed production origin; returned links never select a request.
const WEBSITE: &str = "https://www.dndbeyond.com";

/// Typed, redacted spell failures shared by both adapters.
pub enum SpellError {
    /// A selector is not canonical or exceeds input policy.
    InvalidInput,
    /// Required response structure or metadata is malformed.
    InvalidResponse,
    /// A page is incomplete or its filters or paging disagree.
    IncompleteResult,
    /// A page repeats a source identity.
    ConflictingIdentity,
    /// The selected detail route returned HTTP 404.
    MissingRecord,
    /// The response does not establish the supported content contract.
    UnsupportedOperation,
    /// A row count, response, or result exceeds local policy.
    LimitExceeded,
    /// The complete operation exceeded thirty seconds.
    TimedOut,
    /// Preserves the typed HTTP setup cause.
    SourceRead(HttpError),
    /// Preserves denial, rejection, expiry, and session-change causes.
    SessionRead(SessionError),
}

impl fmt::Display for SpellError {
    /// Never includes selected text, HTML, credentials, or arbitrary selectors.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid spell input"),
            Self::InvalidResponse => f.write_str("invalid spell response"),
            Self::IncompleteResult => f.write_str("spell result is incomplete or inconsistent"),
            Self::ConflictingIdentity => f.write_str("conflicting spell identities"),
            Self::MissingRecord => f.write_str("spell record not found"),
            Self::UnsupportedOperation => {
                f.write_str("spell operation or access contract is unsupported")
            }
            Self::LimitExceeded => f.write_str("spell result limit exceeded"),
            Self::TimedOut => f.write_str("spell operation timed out"),
            Self::SourceRead(error) => write!(f, "spell source read failed: {error}"),
            Self::SessionRead(error) => write!(f, "spell session read failed: {error}"),
        }
    }
}

impl fmt::Debug for SpellError {
    /// Uses only the redacted ordinary diagnostic.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for SpellError {
    /// Keeps source and session failures typed for callers.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceRead(error) => Some(error),
            Self::SessionRead(error) => Some(error),
            _ => None,
        }
    }
}

/// Checks a complete numeric-ID/slug website key before session access.
///
/// # Errors
///
/// Rejects URLs, bare IDs, padding, escapes, separators, and noncanonical IDs.
pub fn validate_slug(slug: &str) -> Result<(), SpellError> {
    let (id, name) = slug.split_once('-').ok_or(SpellError::InvalidInput)?;
    if slug.len() > 256
        || id
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0 && n.to_string() == id)
            .is_none()
        || name.is_empty()
        || name.split('-').any(str::is_empty)
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(SpellError::InvalidInput);
    }
    Ok(())
}

/// Reads exactly one explicit spell page without detail or next-page fan-out.
///
/// # Errors
///
/// Returns typed input, access, session, incomplete, malformed, and limit failures.
pub async fn list(session: &Session, query: &SpellQuery) -> Result<Value, SpellError> {
    query.validate()?;
    read(session, query.website_url(WEBSITE)?.as_str(), |body| {
        website::list(body, query)
    })
    .await
}

/// Reads one canonical detail and preserves its selected subtree as untrusted HTML.
///
/// # Errors
///
/// Returns invalid input before storage access and returns no content on an
/// unverified access structure, missing detail, denial, or invalid identity.
pub async fn show(session: &Session, slug: &str) -> Result<Value, SpellError> {
    validate_slug(slug)?;
    read(session, &format!("{WEBSITE}/spells/{slug}"), |body| {
        website::show(body, slug)
    })
    .await
}

/// Pins one cookie identity and bounds validation, one resource read, and assembly.
async fn read(
    session: &Session,
    url: &str,
    parse: impl FnOnce(&[u8]) -> Result<Value, SpellError>,
) -> Result<Value, SpellError> {
    session.supported().map_err(SpellError::SessionRead)?;
    let session = session.read_scope();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    tokio::time::timeout_at(deadline, async {
        let (client, origin) = source_client(&session)?;
        let target = url.replacen(WEBSITE, &origin, 1);
        session
            .account_id()
            .await
            .map_err(SpellError::SessionRead)?;
        let body = session
            .get_spell_html(&client, &target)
            .await
            .map_err(|error| {
                if error.kind() == SessionErrorKind::NotFound && url.contains("/spells/") {
                    SpellError::MissingRecord
                } else {
                    SpellError::SessionRead(error)
                }
            })?;
        let value = finish(parse(&body)?)?;
        if tokio::time::Instant::now() >= deadline {
            return Err(SpellError::TimedOut);
        }
        Ok(value)
    })
    .await
    .map_err(|_| SpellError::TimedOut)?
}

/// Selects only the fixed production origin or this owner's compiled fixture.
fn source_client(session: &Session) -> Result<(HttpClient, String), SpellError> {
    #[cfg(any(test, feature = "session-fixture"))]
    if let Some(source) = &session.spell_fixture {
        return Ok((source.client.clone(), source.origin.clone()));
    }
    #[cfg(not(any(test, feature = "session-fixture")))]
    let _ = session;
    Ok((
        HttpClient::single_attempt(&[WEBSITE]).map_err(SpellError::SourceRead)?,
        WEBSITE.to_owned(),
    ))
}

/// Bounds compact output before either adapter presents or serializes it.
fn finish(value: Value) -> Result<Value, SpellError> {
    let bytes = serde_json::to_vec(&value).map_err(|_| SpellError::InvalidResponse)?;
    if bytes.len() > MAX_RESULT_BYTES {
        return Err(SpellError::LimitExceeded);
    }
    Ok(value)
}
