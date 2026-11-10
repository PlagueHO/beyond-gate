//! Website-filtered background metadata and selected authenticated source definitions.
//!
//! Show selects from a fresh complete collection, not a guessed detail route.
//! Returned definitions and source IDs grant no subsequent reads or ownership.
//! The 1 MiB transport cap is unchanged; oversized collections fail explicitly.

#[cfg(any(test, feature = "session-fixture"))]
pub mod fixture;
pub mod query;
#[cfg(test)]
mod tests;
mod website;

pub use query::BackgroundQuery;

use crate::{
    auth::session::{Session, SessionError},
    http::{HttpClient, HttpError},
    public_config::{Configuration, ConfigurationError, valid_text},
};
use serde_json::{Value, json};
use std::{collections::HashSet, error::Error, fmt, time::Duration};
use tokio::time::timeout;

/// Maximum raw listing rows or complete collection definitions before selection.
pub const MAX_RECORDS: usize = 1_000;
/// Maximum compact result bytes before adapter presentation.
pub const MAX_RESULT_BYTES: usize = 1024 * 1024;
/// Fixed website origin; display links never choose a network request.
const WEBSITE: &str = "https://www.dndbeyond.com";
/// Fixed authenticated collection origin.
const SERVICE: &str = "https://character-service.dndbeyond.com";

/// Safe shared failures without raw content, credentials, or arbitrary selectors.
pub enum BackgroundError {
    /// A locally supplied ID or filter is invalid.
    InvalidInput,
    /// A required response field or identity is malformed.
    InvalidResponse,
    /// The response does not establish a complete terminal result.
    IncompleteResult,
    /// A source identity occurs more than once.
    ConflictingIdentity,
    /// No selected ID exists in the complete checked collection.
    MissingRecord,
    /// The source requires an unverified operation or content-access contract.
    UnsupportedOperation,
    /// A raw count or compact result exceeds policy.
    LimitExceeded,
    /// The complete operation reached its deadline.
    TimedOut,
    /// Preserves the redacted public transport cause.
    SourceRead(HttpError),
    /// Preserves denial, expiry, rejection, and protected transport causes.
    SessionRead(SessionError),
}

impl fmt::Display for BackgroundError {
    /// Emits fixed messages without reflecting source records or input values.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid background input"),
            Self::InvalidResponse => f.write_str("invalid background response"),
            Self::IncompleteResult => f.write_str("background result is not complete and terminal"),
            Self::ConflictingIdentity => f.write_str("conflicting background identities"),
            Self::MissingRecord => {
                f.write_str("background record not found in the checked collection")
            }
            Self::UnsupportedOperation => {
                f.write_str("background operation or access contract is unsupported")
            }
            Self::LimitExceeded => f.write_str("background result limit exceeded"),
            Self::TimedOut => f.write_str("background operation timed out"),
            Self::SourceRead(error) => write!(f, "background source read failed: {error}"),
            Self::SessionRead(error) => {
                write!(f, "background session read failed: {error}")?;
                if let Some(http) = error
                    .source()
                    .and_then(|cause| cause.downcast_ref::<HttpError>())
                {
                    write!(f, ": {http}")?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Debug for BackgroundError {
    /// Uses only the redacted ordinary diagnostic.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for BackgroundError {
    /// Keeps typed source and session causes without flattening their classifications.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceRead(error) => Some(error),
            Self::SessionRead(error) => Some(error),
            _ => None,
        }
    }
}

/// Checks a canonical positive decimal source ID before session access.
///
/// # Errors
///
/// Rejects zero, signs, padding, leading zeroes, URLs, and values beyond `u64`.
pub fn validate_id(id: &str) -> Result<(), BackgroundError> {
    id.parse::<u64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == id)
        .map(|_| ())
        .ok_or(BackgroundError::InvalidInput)
}

/// Lists one complete bounded website result using its actual filter controls.
///
/// # Errors
///
/// Returns input, malformed, incomplete, unsupported paging, limit, or session
/// failures. No rules text, detail fan-out, partial result, or access grant is emitted.
pub async fn list(session: &Session, query: &BackgroundQuery) -> Result<Value, BackgroundError> {
    query.validate()?;
    let session = session.read_scope();
    timeout(Duration::from_secs(30), async {
        let (client, origin) = source_client(&session, WEBSITE)?;
        let url = query.website_url(&origin)?;
        session
            .account_id()
            .await
            .map_err(BackgroundError::SessionRead)?;
        let body = session
            .get_background_html(&client, url.as_str())
            .await
            .map_err(BackgroundError::SessionRead)?;
        finish(website::parse(&body, query)?)
    })
    .await
    .map_err(|_| BackgroundError::TimedOut)?
}

/// Selects one unchanged source definition from a fresh checked collection.
///
/// The first-party background reader returns definitions directly, without the
/// monster access map. HTTP denial stays a denial; no ownership is inferred.
/// A collection larger than the existing 1 MiB transport limit fails, even when
/// the selected record alone would fit. No guessed selected-ID filter is sent.
///
/// # Errors
///
/// Returns missing, malformed, incomplete, unsupported, limit, session, or
/// transport errors. Unknown source definition and reference fields survive.
pub async fn show(session: &Session, id: &str) -> Result<Value, BackgroundError> {
    validate_id(id)?;
    let session = session.read_scope();
    timeout(Duration::from_secs(60), async {
        let (client, website) = source_client(&session, WEBSITE)?;
        let configuration: Configuration = client
            .get_json(&format!("{website}/api/config/json"), None)
            .await
            .map_err(BackgroundError::SourceRead)?;
        configuration.validate().map_err(|error| match error {
            ConfigurationError::Invalid => BackgroundError::InvalidResponse,
            ConfigurationError::Conflict => BackgroundError::ConflictingIdentity,
            ConfigurationError::Limit => BackgroundError::LimitExceeded,
        })?;
        let sources = configuration.sources.iter().map(|s| s.id).collect();
        let (client, origin) = source_client(&session, SERVICE)?;
        let response = session
            .get_json(
                &client,
                &format!("{origin}/character/v5/game-data/backgrounds?sharingSetting=2"),
            )
            .await
            .map_err(BackgroundError::SessionRead)?;
        finish(selected(response, id, &sources)?)
    })
    .await
    .map_err(|_| BackgroundError::TimedOut)?
}

/// Selects only fixed origins or this owner's compiled synthetic source.
fn source_client(session: &Session, origin: &str) -> Result<(HttpClient, String), BackgroundError> {
    session.supported().map_err(BackgroundError::SessionRead)?;
    #[cfg(any(test, feature = "session-fixture"))]
    if let Some(source) = &session.background_fixture {
        return Ok((source.client.clone(), source.origin.clone()));
    }
    Ok((
        HttpClient::single_attempt(&[origin]).map_err(BackgroundError::SourceRead)?,
        origin.to_owned(),
    ))
}

/// Requires the observed terminal envelope and checks every definition before selection.
fn selected(value: Value, id: &str, sources: &HashSet<u64>) -> Result<Value, BackgroundError> {
    let envelope = value.as_object().ok_or(BackgroundError::InvalidResponse)?;
    if !envelope.get("id").is_some_and(Value::is_number)
        || envelope.get("success").and_then(Value::as_bool) != Some(true)
        || !envelope.get("message").is_some_and(Value::is_string)
    {
        return Err(BackgroundError::InvalidResponse);
    }
    if envelope
        .keys()
        .any(|k| matches!(k.as_str(), "accessType" | "accessTypes"))
    {
        return Err(BackgroundError::UnsupportedOperation);
    }
    if !envelope.get("pagination").is_some_and(Value::is_null)
        || envelope.keys().any(|k| {
            !matches!(
                k.as_str(),
                "id" | "success" | "message" | "data" | "pagination" | "totalCount"
            )
        })
    {
        return Err(BackgroundError::IncompleteResult);
    }
    let rows = envelope
        .get("data")
        .and_then(Value::as_array)
        .ok_or(BackgroundError::InvalidResponse)?;
    if rows.len() > MAX_RECORDS {
        return Err(BackgroundError::LimitExceeded);
    }
    if envelope
        .get("totalCount")
        .is_some_and(|v| v.as_u64() != Some(rows.len() as u64))
    {
        return Err(BackgroundError::IncompleteResult);
    }
    let mut ids = HashSet::new();
    let mut result = None;
    for row in rows {
        let numeric = row["id"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or(BackgroundError::InvalidResponse)?;
        let name = row["name"]
            .as_str()
            .filter(|s| valid_text(s))
            .ok_or(BackgroundError::InvalidResponse)?;
        let references = row["sources"]
            .as_array()
            .ok_or(BackgroundError::InvalidResponse)?;
        if references.len() > MAX_RECORDS {
            return Err(BackgroundError::LimitExceeded);
        }
        for reference in references {
            if !reference["sourceId"]
                .as_u64()
                .is_some_and(|id| sources.contains(&id))
            {
                return Err(BackgroundError::InvalidResponse);
            }
        }
        if !ids.insert(numeric) {
            return Err(BackgroundError::ConflictingIdentity);
        }
        if row.get("accessType").is_some() || row.get("accessTypes").is_some() {
            return Err(BackgroundError::UnsupportedOperation);
        }
        let link = source_url(
            &numeric.to_string(),
            row["moreDetailsUrl"]
                .as_str()
                .ok_or(BackgroundError::InvalidResponse)?,
        )?;
        if numeric.to_string() == id {
            result =
                Some(json!({"id":id,"name":name,"source_url":link,"sources":references,"raw":row}));
        }
    }
    result.ok_or(BackgroundError::MissingRecord)
}

/// Validates a display link and identity without using either to select a request.
fn source_url(id: &str, text: &str) -> Result<String, BackgroundError> {
    if !valid_text(text) || text.contains('\\') || text.contains('%') {
        return Err(BackgroundError::InvalidResponse);
    }
    let absolute = if text.starts_with("/backgrounds/") {
        format!("{WEBSITE}{text}")
    } else {
        text.to_owned()
    };
    let url = reqwest::Url::parse(&absolute).map_err(|_| BackgroundError::InvalidResponse)?;
    let suffix = url
        .path()
        .strip_prefix("/backgrounds/")
        .ok_or(BackgroundError::InvalidResponse)?;
    let (selected, slug) = suffix
        .split_once('-')
        .ok_or(BackgroundError::InvalidResponse)?;
    if url.as_str() != absolute
        || url.origin().ascii_serialization() != WEBSITE
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || selected != id
        || slug.is_empty()
        || !slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(BackgroundError::InvalidResponse);
    }
    Ok(url.to_string())
}

/// Checks the exact canonical JSON result size before adapters emit any output.
fn finish(value: Value) -> Result<Value, BackgroundError> {
    if serde_json::to_vec(&value)
        .map_err(|_| BackgroundError::InvalidResponse)?
        .len()
        > MAX_RESULT_BYTES
    {
        return Err(BackgroundError::LimitExceeded);
    }
    Ok(value)
}
