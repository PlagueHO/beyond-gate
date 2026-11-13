//! Website-filtered feat metadata and bounded selected definitions.
//!
//! Show checks one fresh terminal collection before it returns one definition.
//! Metadata, prerequisite options, and repeatable parents are not access grants.

#[cfg(any(test, feature = "session-fixture"))]
pub mod fixture;
pub mod query;
#[cfg(test)]
mod tests;
mod website;

pub use query::FeatQuery;

use crate::{
    auth::session::{Session, SessionError},
    http::{HttpClient, HttpError},
    public_config::{Configuration, ConfigurationError, valid_text},
};
use serde_json::Value;
use std::{collections::HashSet, error::Error, fmt, time::Duration};
use tokio::time::timeout;

/// Maximum listing rows, collection definitions, or references per definition.
pub const MAX_RECORDS: usize = 1_000;
/// Maximum compact JSON result bytes before adapter presentation.
pub const MAX_RESULT_BYTES: usize = 1024 * 1024;
/// Fixed website origin; returned links never select a network destination.
const WEBSITE: &str = "https://www.dndbeyond.com";
/// Fixed character-service origin for the checked feat collection.
const SERVICE: &str = "https://character-service.dndbeyond.com";

/// Redacted input, source, completeness, and access failures shared by adapters.
pub enum FeatError {
    /// A local selector is malformed or outside its bounds.
    InvalidInput,
    /// Required source structure, metadata, or identity is malformed.
    InvalidResponse,
    /// The source does not establish a complete terminal result.
    IncompleteResult,
    /// More than one record has the same source identity.
    ConflictingIdentity,
    /// The selected ID is absent from the complete checked collection.
    MissingRecord,
    /// The source indicates an unsupported challenge, paging, or access shape.
    UnsupportedOperation,
    /// The raw count or compact result exceeds local policy.
    LimitExceeded,
    /// The complete operation reached its deadline.
    TimedOut,
    /// Preserves the typed, redacted HTTP cause.
    SourceRead(HttpError),
    /// Preserves denied, rejected, expired, and transport/session causes.
    SessionRead(SessionError),
}

impl fmt::Display for FeatError {
    /// Emits fixed diagnostics without raw records, credentials, or input selectors.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid feat input"),
            Self::InvalidResponse => f.write_str("invalid feat response"),
            Self::IncompleteResult => f.write_str("feat result is not complete and terminal"),
            Self::ConflictingIdentity => f.write_str("conflicting feat identities"),
            Self::MissingRecord => f.write_str("feat record not found in the checked collection"),
            Self::UnsupportedOperation => {
                f.write_str("feat operation or access contract is unsupported")
            }
            Self::LimitExceeded => f.write_str("feat result limit exceeded"),
            Self::TimedOut => f.write_str("feat operation timed out"),
            Self::SourceRead(error) => write!(f, "feat source read failed: {error}"),
            Self::SessionRead(error) => {
                write!(f, "feat session read failed: {error}")?;
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

impl fmt::Debug for FeatError {
    /// Keeps debug output as restricted as ordinary diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for FeatError {
    /// Retains typed causes without flattening access failures into missing records.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceRead(error) => Some(error),
            Self::SessionRead(error) => Some(error),
            _ => None,
        }
    }
}

/// Validates a canonical positive decimal source ID before session access.
///
/// # Errors
///
/// Rejects zero, padding, signs, URLs, leading zeroes, and values beyond `u64`.
pub fn validate_id(id: &str) -> Result<(), FeatError> {
    id.parse::<u64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == id)
        .map(|_| ())
        .ok_or(FeatError::InvalidInput)
}

/// Reads one bounded website result using only observed feat filter controls.
///
/// # Errors
///
/// Returns input, malformed, incomplete, unsupported paging, limits, or typed
/// session/source failures. No rules text or partial result is returned.
pub async fn list(session: &Session, query: &FeatQuery) -> Result<Value, FeatError> {
    query.validate()?;
    session.supported().map_err(FeatError::SessionRead)?;
    let session = session.read_scope();
    timeout(Duration::from_secs(30), async {
        let (client, origin) = source_client(&session, WEBSITE)?;
        let url = query.website_url(&origin)?;
        session.account_id().await.map_err(FeatError::SessionRead)?;
        let body = session
            .get_feat_html(&client, url.as_str())
            .await
            .map_err(FeatError::SessionRead)?;
        finish(website::parse(&body, query)?)
    })
    .await
    .map_err(|_| FeatError::TimedOut)?
}

/// Selects one unchanged definition from a fresh bounded feat collection.
///
/// Every row and source reference must pass before selection. Source identities
/// are checked against public configuration, not treated as access grants.
///
/// # Errors
///
/// Returns missing, malformed, incomplete, unsupported, limit, session, or
/// transport errors. Unknown definition and source fields remain unchanged.
pub async fn show(session: &Session, id: &str) -> Result<Value, FeatError> {
    validate_id(id)?;
    session.supported().map_err(FeatError::SessionRead)?;
    let session = session.read_scope();
    timeout(Duration::from_secs(60), async {
        let (client, website) = source_client(&session, WEBSITE)?;
        let configuration: Configuration = client
            .get_json(&format!("{website}/api/config/json"), None)
            .await
            .map_err(FeatError::SourceRead)?;
        configuration.validate().map_err(|error| match error {
            ConfigurationError::Invalid => FeatError::InvalidResponse,
            ConfigurationError::Conflict => FeatError::ConflictingIdentity,
            ConfigurationError::Limit => FeatError::LimitExceeded,
        })?;
        let sources = configuration
            .sources
            .iter()
            .map(|source| source.id)
            .collect();
        let (client, origin) = source_client(&session, SERVICE)?;
        let response = session
            .get_json(
                &client,
                &format!("{origin}/character/v5/game-data/feats?sharingSetting=2"),
            )
            .await
            .map_err(FeatError::SessionRead)?;
        finish(selected(response, id, &sources)?)
    })
    .await
    .map_err(|_| FeatError::TimedOut)?
}

/// Selects one fixed source origin or this session owner's compiled fixture.
fn source_client(session: &Session, origin: &str) -> Result<(HttpClient, String), FeatError> {
    session.supported().map_err(FeatError::SessionRead)?;
    #[cfg(any(test, feature = "session-fixture"))]
    if let Some(source) = &session.feat_fixture {
        return Ok((source.client.clone(), source.origin.clone()));
    }
    Ok((
        HttpClient::single_attempt(&[origin]).map_err(FeatError::SourceRead)?,
        origin.to_owned(),
    ))
}

/// Checks the terminal collection and every row before selecting one definition.
fn selected(value: Value, id: &str, sources: &HashSet<u64>) -> Result<Value, FeatError> {
    use serde_json::json;
    let envelope = value.as_object().ok_or(FeatError::InvalidResponse)?;
    if !envelope.get("id").is_some_and(Value::is_number)
        || envelope.get("success").and_then(Value::as_bool) != Some(true)
        || !envelope.get("message").is_some_and(Value::is_string)
    {
        return Err(FeatError::InvalidResponse);
    }
    if envelope.contains_key("accessType") || envelope.contains_key("accessTypes") {
        return Err(FeatError::UnsupportedOperation);
    }
    if !envelope.get("pagination").is_some_and(Value::is_null)
        || envelope.keys().any(|key| {
            !matches!(
                key.as_str(),
                "id" | "success" | "message" | "data" | "pagination" | "totalCount"
            )
        })
    {
        return Err(FeatError::IncompleteResult);
    }
    let rows = envelope
        .get("data")
        .and_then(Value::as_array)
        .ok_or(FeatError::InvalidResponse)?;
    if rows.len() > MAX_RECORDS {
        return Err(FeatError::LimitExceeded);
    }
    if envelope
        .get("totalCount")
        .is_some_and(|n| n.as_u64() != Some(rows.len() as u64))
    {
        return Err(FeatError::IncompleteResult);
    }
    let mut ids = HashSet::new();
    let mut result = None;
    for row in rows {
        let numeric = row["id"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or(FeatError::InvalidResponse)?;
        let name = row["name"]
            .as_str()
            .filter(|s| valid_text(s))
            .ok_or(FeatError::InvalidResponse)?;
        let references = row["sources"]
            .as_array()
            .ok_or(FeatError::InvalidResponse)?;
        if references.len() > MAX_RECORDS {
            return Err(FeatError::LimitExceeded);
        }
        for reference in references {
            if !reference["sourceId"]
                .as_u64()
                .is_some_and(|source| sources.contains(&source))
            {
                return Err(FeatError::InvalidResponse);
            }
        }
        if !ids.insert(numeric) {
            return Err(FeatError::ConflictingIdentity);
        }
        if row.get("accessType").is_some() || row.get("accessTypes").is_some() {
            return Err(FeatError::UnsupportedOperation);
        }
        let repeatable = match row.get("isRepeatable") {
            None | Some(Value::Null) => None,
            Some(Value::Bool(value)) => Some(*value),
            _ => return Err(FeatError::InvalidResponse),
        };
        let parent = match row.get("repeatableParentId") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                value
                    .as_u64()
                    .filter(|n| *n > 0)
                    .ok_or(FeatError::InvalidResponse)?
                    .to_string(),
            ),
        };
        let link = source_url(
            &numeric.to_string(),
            row["moreDetailsUrl"]
                .as_str()
                .ok_or(FeatError::InvalidResponse)?,
        )?;
        if numeric.to_string() == id {
            result = Some(
                json!({"id":id,"name":name,"source_url":link,"sources":references,
                "is_repeatable":repeatable,"repeatable_parent_id":parent,"raw":row}),
            );
        }
    }
    result.ok_or(FeatError::MissingRecord)
}

/// Validates a display link against its identity without permitting a fetch.
fn source_url(id: &str, text: &str) -> Result<String, FeatError> {
    if !valid_text(text) || text.contains('\\') || text.contains('%') {
        return Err(FeatError::InvalidResponse);
    }
    let absolute = if text.starts_with("/feats/") {
        format!("{WEBSITE}{text}")
    } else {
        text.to_owned()
    };
    let url = reqwest::Url::parse(&absolute).map_err(|_| FeatError::InvalidResponse)?;
    let (selected, slug) = url
        .path()
        .strip_prefix("/feats/")
        .and_then(|s| s.split_once('-'))
        .ok_or(FeatError::InvalidResponse)?;
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
        return Err(FeatError::InvalidResponse);
    }
    Ok(url.to_string())
}

/// Measures the exact canonical result before either adapter presents it.
fn finish(value: Value) -> Result<Value, FeatError> {
    if serde_json::to_vec(&value)
        .map_err(|_| FeatError::InvalidResponse)?
        .len()
        > MAX_RESULT_BYTES
    {
        return Err(FeatError::LimitExceeded);
    }
    Ok(value)
}
