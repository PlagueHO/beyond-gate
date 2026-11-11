//! Bounded species listing metadata and selected authenticated race details.
//!
//! Listing metadata comes from the public species page. Selected detail comes
//! from a fresh authenticated collection; no rules engine is applied.

#[cfg(any(test, feature = "session-fixture"))]
pub mod fixture;
#[cfg(test)]
mod tests;
pub(crate) mod website;

use crate::{
    auth::session::{Session, SessionError, SessionErrorKind},
    http::{HttpClient, HttpError},
    public_config::valid_text,
};
use serde_json::{Value, json};
use std::{collections::HashSet, error::Error, fmt, time::Duration};
use tokio::time::timeout;

/// Maximum species entries in one website listing or source collection.
pub const MAX_RECORDS: usize = 1_000;
/// Maximum compact JSON output bytes before adapter presentation.
pub const MAX_RESULT_BYTES: usize = 1024 * 1024;
/// Fixed public species listing origin.
const WEBSITE: &str = "https://www.dndbeyond.com";
/// Fixed service origin; returned data never selects a request destination.
const SERVICE: &str = "https://character-service.dndbeyond.com";
/// Whole-operation bound for the protected collection read and assembly.
const DEADLINE: Duration = Duration::from_secs(60);
/// Whole-operation bound for the cookie-backed website listing.
const LIST_DEADLINE: Duration = Duration::from_secs(30);
/// Candidate route confirmed in the public character application.
const COLLECTION: &str = "/character/v5/game-data/races";

/// Redacted source, access, input, and bounded-result failures shared by adapters.
pub enum SpeciesError {
    /// The selected ID is not a canonical positive decimal integer.
    InvalidInput,
    /// A required source identity or name is malformed.
    InvalidResponse,
    /// The response shape does not establish a complete species collection.
    IncompleteResult,
    /// A source identity occurs more than once.
    ConflictingIdentity,
    /// The selected ID is absent from the complete collection.
    MissingRecord,
    /// The source requires an unverified route or access contract.
    UnsupportedOperation,
    /// The raw collection or canonical output exceeds local policy.
    LimitExceeded,
    /// The whole operation reached its deadline.
    TimedOut,
    /// Keeps the typed, redacted HTTP client setup cause.
    SourceRead(HttpError),
    /// Keeps session, HTTP denial, expiry, and transport causes distinct.
    SessionRead(SessionError),
}

impl fmt::Display for SpeciesError {
    /// Uses fixed diagnostics without raw records or supplied selectors.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid species input"),
            Self::InvalidResponse => formatter.write_str("invalid species response"),
            Self::IncompleteResult => {
                formatter.write_str("species result is incomplete or inconsistent")
            }
            Self::ConflictingIdentity => formatter.write_str("conflicting species identities"),
            Self::MissingRecord => formatter.write_str("species record not found"),
            Self::UnsupportedOperation => {
                formatter.write_str("species operation or access contract is unsupported")
            }
            Self::LimitExceeded => formatter.write_str("species result limit exceeded"),
            Self::TimedOut => formatter.write_str("species operation timed out"),
            Self::SourceRead(error) => write!(formatter, "species source read failed: {error}"),
            Self::SessionRead(error) => write!(formatter, "species session read failed: {error}"),
        }
    }
}

impl fmt::Debug for SpeciesError {
    /// Omits source content and arbitrary dependency diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for SpeciesError {
    /// Preserves typed causes without flattening access and session failures.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceRead(error) => Some(error),
            Self::SessionRead(error) => Some(error),
            _ => None,
        }
    }
}

/// Validates a canonical positive decimal species ID without interpreting a URL.
///
/// # Errors
///
/// Rejects leading zeroes, signs, whitespace, zero, and values beyond `u64`.
pub fn validate_id(id: &str) -> Result<(), SpeciesError> {
    id.parse::<u64>()
        .ok()
        .filter(|value| *value > 0 && value.to_string() == id)
        .map(|_| ())
        .ok_or(SpeciesError::InvalidInput)
}

/// Lists bounded identity and source metadata from the fixed public species page.
///
/// The site has client-side search controls, but no observed server filter or
/// pagination contract. The operation reads the unfiltered page and makes no
/// ownership claim.
///
/// # Errors
///
/// Returns malformed, incomplete, duplicate, unsupported, limit, session, or
/// transport failures. No partial result is returned.
pub async fn list(session: &Session) -> Result<Value, SpeciesError> {
    session.supported().map_err(SpeciesError::SessionRead)?;
    let session = session.read_scope();
    timeout(LIST_DEADLINE, async {
        let (client, origin) = website_client(&session)?;
        session
            .account_id()
            .await
            .map_err(SpeciesError::SessionRead)?;
        let body = session
            .get_species_html(&client, &format!("{origin}/species"))
            .await
            .map_err(SpeciesError::SessionRead)?;
        finish(website::parse(&body)?)
    })
    .await
    .map_err(|_| SpeciesError::TimedOut)?
}

/// Shows one complete selected source record from a fresh collection read.
///
/// The returned `raw` value preserves every source field. Metadata, ownership,
/// or presence in another result is not used as a grant.
///
/// # Errors
///
/// Returns invalid input, malformed, incomplete, duplicate, missing, unsupported,
/// limit, session, or transport failures. No detail URL is guessed or requested.
pub async fn show(session: &Session, id: &str) -> Result<Value, SpeciesError> {
    validate_id(id)?;
    let records = collection(session).await?;
    let selected = records
        .iter()
        .find(|record| {
            record["entityRaceId"]
                .as_u64()
                .is_some_and(|value| value.to_string() == id)
        })
        .ok_or(SpeciesError::MissingRecord)?;
    let mut result = metadata(selected)?;
    result["raw"] = selected.clone();
    finish(result)
}

/// Reads the fixed authenticated collection inside the shared identity and time bounds.
async fn collection(session: &Session) -> Result<Vec<Value>, SpeciesError> {
    session.supported().map_err(SpeciesError::SessionRead)?;
    let session = session.read_scope();
    timeout(DEADLINE, async {
        let (client, origin) = source_client(&session)?;
        let value = session
            .get_json::<Value>(&client, &format!("{origin}{COLLECTION}"))
            .await
            .map_err(|error| {
                if error.kind() == SessionErrorKind::NotFound {
                    SpeciesError::UnsupportedOperation
                } else {
                    SpeciesError::SessionRead(error)
                }
            })?;
        parse_collection(value)
    })
    .await
    .map_err(|_| SpeciesError::TimedOut)?
}

/// Selects only the fixed production website or this owner's synthetic service.
fn website_client(session: &Session) -> Result<(HttpClient, String), SpeciesError> {
    #[cfg(any(test, feature = "session-fixture"))]
    if let Some(source) = &session.species_fixture {
        return Ok((source.client.clone(), source.origin.clone()));
    }
    #[cfg(not(any(test, feature = "session-fixture")))]
    let _ = session;
    Ok((
        HttpClient::single_attempt(&[WEBSITE]).map_err(SpeciesError::SourceRead)?,
        WEBSITE.to_owned(),
    ))
}

/// Selects only the fixed production service or this owner's synthetic service.
fn source_client(session: &Session) -> Result<(HttpClient, String), SpeciesError> {
    #[cfg(any(test, feature = "session-fixture"))]
    if let Some(source) = &session.species_fixture {
        return Ok((source.client.clone(), source.origin.clone()));
    }
    #[cfg(not(any(test, feature = "session-fixture")))]
    let _ = session;
    Ok((
        HttpClient::single_attempt(&[SERVICE]).map_err(SpeciesError::SourceRead)?,
        SERVICE.to_owned(),
    ))
}

/// Requires the observed terminal character-service envelope and checked array data.
fn parse_collection(value: Value) -> Result<Vec<Value>, SpeciesError> {
    let envelope = value.as_object().ok_or(SpeciesError::InvalidResponse)?;
    if envelope.contains_key("accessType") || envelope.contains_key("accessTypes") {
        return Err(SpeciesError::UnsupportedOperation);
    }
    if envelope.keys().any(|key| {
        !matches!(
            key.as_str(),
            "id" | "success" | "message" | "data" | "pagination" | "totalCount"
        )
    }) {
        return Err(SpeciesError::IncompleteResult);
    }
    if !envelope.get("id").is_some_and(Value::is_number)
        || envelope.get("success").and_then(Value::as_bool) != Some(true)
        || !envelope.get("message").is_some_and(Value::is_string)
    {
        return Err(SpeciesError::InvalidResponse);
    }
    if !envelope.get("pagination").is_some_and(Value::is_null) {
        return Err(SpeciesError::IncompleteResult);
    }
    let records = envelope
        .get("data")
        .and_then(Value::as_array)
        .ok_or(SpeciesError::InvalidResponse)?;
    if records.len() > MAX_RECORDS {
        return Err(SpeciesError::LimitExceeded);
    }
    if envelope
        .get("totalCount")
        .is_some_and(|total| total.as_u64() != Some(records.len() as u64))
    {
        return Err(SpeciesError::IncompleteResult);
    }
    let mut ids = HashSet::new();
    for record in records {
        if record.get("accessType").is_some() || record.get("accessTypes").is_some() {
            return Err(SpeciesError::UnsupportedOperation);
        }
        let id = record["entityRaceId"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or(SpeciesError::InvalidResponse)?;
        metadata(record)?;
        if !ids.insert(id) {
            return Err(SpeciesError::ConflictingIdentity);
        }
    }
    Ok(records.clone())
}

/// Projects the stable species identity and observed legacy/subrace metadata.
fn metadata(record: &Value) -> Result<Value, SpeciesError> {
    let fields = record.as_object().ok_or(SpeciesError::InvalidResponse)?;
    let id = fields
        .get("entityRaceId")
        .and_then(Value::as_u64)
        .filter(|id| *id > 0)
        .ok_or(SpeciesError::InvalidResponse)?;
    let name = record_name(fields)?;
    let is_homebrew = optional_bool(fields.get("isHomebrew"))?;
    let is_legacy = optional_bool(fields.get("isLegacy"))?;
    let is_subrace = optional_bool(fields.get("isSubRace"))?;
    let base_race_name = optional_text(fields.get("baseRaceName"))?;
    Ok(json!({
        "id": id.to_string(),
        "name": name,
        "is_homebrew": is_homebrew,
        "is_legacy": is_legacy,
        "is_subrace": is_subrace,
        "base_race_name": base_race_name
    }))
}

/// Uses the source's full-name preference and base-name fallback.
fn record_name(fields: &serde_json::Map<String, Value>) -> Result<&str, SpeciesError> {
    for field in ["fullName", "baseName"] {
        match fields.get(field) {
            None | Some(Value::Null) => continue,
            Some(Value::String(value)) if value.is_empty() => continue,
            Some(Value::String(value)) if valid_text(value) => return Ok(value),
            _ => return Err(SpeciesError::InvalidResponse),
        }
    }
    Err(SpeciesError::InvalidResponse)
}

/// Keeps absent variant flags distinct from malformed supplied values.
fn optional_bool(value: Option<&Value>) -> Result<Option<bool>, SpeciesError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        _ => Err(SpeciesError::InvalidResponse),
    }
}

/// Keeps absent parent metadata explicit while allowing an empty root-race value.
fn optional_text(value: Option<&Value>) -> Result<Option<String>, SpeciesError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if text.is_empty() || valid_text(text) => {
            Ok((!text.is_empty()).then(|| text.clone()))
        }
        _ => Err(SpeciesError::InvalidResponse),
    }
}

/// Checks canonical output size before either adapter can write a result.
fn finish(value: Value) -> Result<Value, SpeciesError> {
    if serde_json::to_vec(&value)
        .map_err(|_| SpeciesError::InvalidResponse)?
        .len()
        > MAX_RESULT_BYTES
    {
        return Err(SpeciesError::LimitExceeded);
    }
    Ok(value)
}
