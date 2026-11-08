//! Bounded class/subclass metadata catalogs, not rules-text or content-access grants.
//!
//! Show selects an ID from checked collections. No metadata link selects a request.
//! Unknown source-reference metadata survives; raw definition fields do not.

#[cfg(any(test, feature = "session-fixture"))]
pub mod fixture;
mod model;
#[cfg(test)]
mod tests;

use crate::{
    auth::session::{Session, SessionError},
    http::{HttpClient, HttpError},
    public_config::{Configuration, ConfigurationError, valid_text},
};
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashSet, error::Error, fmt, time::Duration};
use tokio::time::{Instant, timeout};

/// Maximum parents traversed by one global subclass operation.
pub const MAX_PARENTS: usize = 32;
/// Maximum raw definitions across all requested catalogs, before filtering.
pub const MAX_RECORDS: usize = 1_000;
/// Maximum admitted resource bytes, including public configuration.
pub const MAX_RESOURCE_BYTES: usize = 8 * 1024 * 1024;
/// Maximum compact canonical result bytes, excluding adapter framing.
pub const MAX_RESULT_BYTES: usize = 1024 * 1024;
/// Maximum UTF-8 bytes in a supplied name filter.
pub const MAX_NAME_BYTES: usize = 256;
/// Fixed public website origin; metadata links never change this request scope.
const WEBSITE: &str = "https://www.dndbeyond.com";
/// Fixed service origin for class and parent-scoped subclass collections.
const SERVICE: &str = "https://character-service.dndbeyond.com";

/// Selects a metadata catalog without adding an action parameter.
#[derive(Clone, Copy, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ClassKind {
    /// Base class definitions.
    #[default]
    Class,
    /// Subclass definitions with explicit parent context.
    Subclass,
}

/// Redacted typed failures; none contain raw definitions or supplied selectors.
pub enum ClassCatalogError {
    /// A selector or its argument combination is invalid.
    InvalidInput,
    /// A source field or identity is malformed.
    InvalidMetadata,
    /// A collection is not a verified terminal response.
    IncompleteDiscovery,
    /// An ID occurs more than once within its parent/catalog.
    ConflictingMetadata,
    /// The selected record or parent is absent from the complete checked scope.
    MissingRecord,
    /// Several parents contain the selected subclass ID.
    AmbiguousRecord,
    /// Raw count, resource bytes, parent count, or result bytes exceed policy.
    LimitExceeded,
    /// The whole operation exceeded its deadline.
    TimedOut,
    /// Public transport failure with its redacted cause.
    SourceRead(HttpError),
    /// Protected session/transport failure with its typed cause.
    SessionRead(SessionError),
}

impl fmt::Display for ClassCatalogError {
    /// Reports fixed diagnostics without source text or arbitrary values.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid class catalog input"),
            Self::InvalidMetadata => f.write_str("invalid class catalog metadata"),
            Self::IncompleteDiscovery => {
                f.write_str("class catalog is not a complete terminal collection")
            }
            Self::ConflictingMetadata => f.write_str("conflicting class catalog identities"),
            Self::MissingRecord => {
                f.write_str("record or parent not found in the checked metadata catalog")
            }
            Self::AmbiguousRecord => {
                f.write_str("subclass ID is ambiguous; supply class_id (--class-id)")
            }
            Self::LimitExceeded => f.write_str("class catalog limit exceeded"),
            Self::TimedOut => f.write_str("class catalog operation timed out"),
            Self::SourceRead(error) => write!(f, "class catalog source read failed: {error}"),
            Self::SessionRead(error) => write!(f, "class catalog session read failed: {error}"),
        }
    }
}

impl fmt::Debug for ClassCatalogError {
    /// Uses only the redacted ordinary diagnostic.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for ClassCatalogError {
    /// Preserves session and transport classifications without flattening them.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceRead(error) => Some(error),
            Self::SessionRead(error) => Some(error),
            _ => None,
        }
    }
}

/// Validates selectors before storage or network activity.
///
/// # Errors
///
/// Rejects noncanonical IDs, invalid name text, or a parent selector on a class read.
pub fn validate_input(
    kind: ClassKind,
    name: Option<&str>,
    class_id: Option<&str>,
    id: Option<&str>,
) -> Result<(), ClassCatalogError> {
    if kind == ClassKind::Class && class_id.is_some()
        || name.is_some() && id.is_some()
        || name.is_some_and(|name| !valid_text(name) || name.len() > MAX_NAME_BYTES)
    {
        return Err(ClassCatalogError::InvalidInput);
    }
    for id in [class_id, id].into_iter().flatten() {
        selected_id(id)?;
    }
    Ok(())
}

/// Parses a positive canonical decimal identifier without accepting leading zeroes.
fn selected_id(id: &str) -> Result<u64, ClassCatalogError> {
    id.parse::<u64>()
        .ok()
        .filter(|value| *value > 0 && value.to_string() == id)
        .ok_or(ClassCatalogError::InvalidInput)
}

/// Lists compact metadata after complete checked discovery and local name filtering.
///
/// # Errors
///
/// Returns typed input, metadata, completeness, bound, session, or transport failures.
/// No partial results, rules text, or content-access evidence are returned.
pub async fn list(
    session: &Session,
    kind: ClassKind,
    name: Option<&str>,
    class_id: Option<&str>,
) -> Result<Value, ClassCatalogError> {
    read(session, kind, name, class_id, None).await
}

/// Shows selected metadata by ID, optionally scoped to a subclass's parent.
///
/// # Errors
///
/// Returns discovery failures, missing-record or cross-parent ambiguity errors.
/// A metadata result never grants access to rules text.
pub async fn show(
    session: &Session,
    kind: ClassKind,
    id: &str,
    class_id: Option<&str>,
) -> Result<Value, ClassCatalogError> {
    read(session, kind, None, class_id, Some(id)).await
}

/// Runs discovery, selection, and serialization inside the whole-operation deadline.
async fn read(
    session: &Session,
    kind: ClassKind,
    name: Option<&str>,
    class_id: Option<&str>,
    id: Option<&str>,
) -> Result<Value, ClassCatalogError> {
    validate_input(kind, name, class_id, id)?;
    session
        .supported()
        .map_err(ClassCatalogError::SessionRead)?;
    let session = session.read_scope();
    let started = Instant::now();
    timeout(Duration::from_secs(120), async {
        let mut records = discover(&session, kind, class_id).await?;
        records.sort_by_key(|record| {
            (
                record.summary.name.to_lowercase(),
                record.summary.name.clone(),
                record.summary.numeric_parent,
                record.summary.numeric_id,
            )
        });
        let result = if let Some(id) = id {
            let mut matches = records.iter().filter(|record| record.summary.id == id);
            let selected = matches.next().ok_or(ClassCatalogError::MissingRecord)?;
            if matches.next().is_some() {
                return Err(ClassCatalogError::AmbiguousRecord);
            }
            result_value(selected)?
        } else {
            let filter = name.map(str::to_lowercase);
            let summaries: Vec<_> = records
                .iter()
                .filter(|record| {
                    filter
                        .as_ref()
                        .is_none_or(|filter| record.summary.name.to_lowercase().contains(filter))
                })
                .map(|record| &record.summary)
                .collect();
            result_value(&summaries)?
        };
        if started.elapsed() >= Duration::from_secs(120) {
            return Err(ClassCatalogError::TimedOut);
        }
        Ok(result)
    })
    .await
    .map_err(|_| ClassCatalogError::TimedOut)?
}

/// Accounts raw resource bodies and definitions independently of filters and output.
#[derive(Default)]
struct Budget {
    /// Includes configuration bytes and every admitted catalog response.
    bytes: usize,
    /// Includes all raw class/subclass entries before duplicate checks.
    records: usize,
}

impl Budget {
    /// Rejects the first resource body that exceeds the aggregate bound.
    fn admit_bytes(&mut self, bytes: usize) -> Result<(), ClassCatalogError> {
        if bytes > MAX_RESOURCE_BYTES - self.bytes {
            return Err(ClassCatalogError::LimitExceeded);
        }
        self.bytes += bytes;
        Ok(())
    }
}

/// Reads fixed complete collections sequentially, stopping at the first failure.
async fn discover(
    session: &Session,
    kind: ClassKind,
    class_id: Option<&str>,
) -> Result<Vec<model::Metadata>, ClassCatalogError> {
    let mut budget = Budget::default();
    let (public, website) = source_client(session, WEBSITE)?;
    let (value, bytes): (Value, usize) = public
        .get_json_counted(&format!("{website}/api/config/json"), None)
        .await
        .map_err(ClassCatalogError::SourceRead)?;
    budget.admit_bytes(bytes)?;
    let configuration: Configuration =
        serde_json::from_value(value).map_err(|_| ClassCatalogError::InvalidMetadata)?;
    configuration.validate().map_err(|error| match error {
        ConfigurationError::Invalid => ClassCatalogError::InvalidMetadata,
        ConfigurationError::Conflict => ClassCatalogError::ConflictingMetadata,
        ConfigurationError::Limit => ClassCatalogError::LimitExceeded,
    })?;
    let sources: HashSet<_> = configuration
        .sources
        .iter()
        .map(|source| source.id)
        .collect();
    let (client, service) = source_client(session, SERVICE)?;
    let classes = catalog(
        session,
        &client,
        &format!("{service}/character/v5/game-data/classes?sharingSetting=2"),
        ClassKind::Class,
        None,
        &sources,
        &mut budget,
    )
    .await?;
    if kind == ClassKind::Class {
        return Ok(classes);
    }
    let parents: Vec<_> = if let Some(parent) = class_id {
        vec![
            classes
                .iter()
                .find(|record| record.summary.id == parent)
                .ok_or(ClassCatalogError::MissingRecord)?
                .summary
                .numeric_id,
        ]
    } else {
        if classes.len() > MAX_PARENTS {
            return Err(ClassCatalogError::LimitExceeded);
        }
        let mut parents: Vec<_> = classes
            .iter()
            .map(|record| record.summary.numeric_id)
            .collect();
        parents.sort_unstable();
        parents
    };
    let mut subclasses = Vec::new();
    for parent in parents {
        subclasses.extend(catalog(session, &client,
            &format!("{service}/character/v5/game-data/subclasses?sharingSetting=2&baseClassId={parent}"),
            ClassKind::Subclass, Some(parent), &sources, &mut budget).await?);
    }
    Ok(subclasses)
}

/// Reads one protected collection, counting actual bytes before metadata assembly.
async fn catalog(
    session: &Session,
    client: &HttpClient,
    url: &str,
    kind: ClassKind,
    parent: Option<u64>,
    sources: &HashSet<u64>,
    budget: &mut Budget,
) -> Result<Vec<model::Metadata>, ClassCatalogError> {
    let (value, bytes) = session
        .get_json_counted::<Value>(client, url)
        .await
        .map_err(ClassCatalogError::SessionRead)?;
    budget.admit_bytes(bytes)?;
    model::collection(value, kind, parent, sources, &mut budget.records)
}

/// Selects fixed production origins or an explicitly compiled synthetic service.
fn source_client(
    session: &Session,
    origin: &str,
) -> Result<(HttpClient, String), ClassCatalogError> {
    #[cfg(any(test, feature = "session-fixture"))]
    if let Some(source) = &session.class_catalog_fixture {
        return Ok((source.client.clone(), source.origin.clone()));
    }
    #[cfg(not(any(test, feature = "session-fixture")))]
    let _ = session;
    Ok((
        HttpClient::single_attempt(&[origin]).map_err(ClassCatalogError::SourceRead)?,
        origin.to_owned(),
    ))
}

/// Checks compact canonical JSON size before adapters can render any result.
fn result_value(value: &impl Serialize) -> Result<Value, ClassCatalogError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ClassCatalogError::InvalidMetadata)?;
    if bytes.len() > MAX_RESULT_BYTES {
        return Err(ClassCatalogError::LimitExceeded);
    }
    serde_json::from_slice(&bytes).map_err(|_| ClassCatalogError::InvalidMetadata)
}
