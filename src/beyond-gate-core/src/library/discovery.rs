//! Fixed library JSON contracts; no crawling, arbitrary URLs, or content cache.

use super::*;
use crate::{auth::session::Session, http::HttpClient};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use tokio::time::timeout;

/// Bounds the complete unpaged metadata catalog independently of observation pages.
pub(super) const MAX_CATALOG_RECORDS: usize = 1_000;
/// Selects only fixed public configuration and authenticated library routes.
const CONFIG_ORIGIN: &str = "https://www.dndbeyond.com";
const CONFIG: &str = "https://www.dndbeyond.com/api/config/json";
const ORIGIN: &str = "https://api.dndbeyond.com";
const SOURCES: &str = "https://api.dndbeyond.com/library/sources?platform=Web";
const OWNERSHIP: &str = "https://api.dndbeyond.com/library/sources/me";

/// Reads only source/category identity, never configuration titles or URL fallbacks.
#[derive(Deserialize)]
struct Configuration {
    sources: Vec<ConfiguredSource>,
    #[serde(rename = "sourceCategories")]
    categories: Vec<SourceCategory>,
}

/// Joins the public category classification to a library source ID.
#[derive(Deserialize)]
struct ConfiguredSource {
    id: u64,
    #[serde(rename = "sourceCategoryId")]
    category: u64,
}

/// Resolves the upstream hidden category by name instead of a fixed numeric ID.
#[derive(Deserialize)]
struct SourceCategory {
    id: u64,
    name: String,
}

impl Configuration {
    /// Validates the complete bounded classification before accessing protected storage.
    fn selection(self) -> Result<HashMap<u64, bool>, LibraryError> {
        if self.sources.len() > MAX_CATALOG_RECORDS || self.categories.len() > MAX_CATALOG_RECORDS {
            return Err(LibraryError::LimitExceeded);
        }
        let mut categories = HashSet::new();
        let mut hidden = None;
        for category in self.categories {
            if category.id == 0 || !valid_text(&category.name) {
                return Err(LibraryError::InvalidMetadata);
            }
            if !categories.insert(category.id) {
                return Err(LibraryError::ConflictingMetadata);
            }
            if category.name.eq_ignore_ascii_case("Hidden Sources")
                && hidden.replace(category.id).is_some()
            {
                return Err(LibraryError::ConflictingMetadata);
            }
        }
        let hidden = hidden.ok_or(LibraryError::InvalidMetadata)?;
        let mut selection = HashMap::new();
        for source in self.sources {
            if source.id == 0 || !categories.contains(&source.category) {
                return Err(LibraryError::InvalidMetadata);
            }
            if selection
                .insert(source.id, source.category != hidden)
                .is_some()
            {
                return Err(LibraryError::ConflictingMetadata);
            }
        }
        Ok(selection)
    }
}

/// Requires a complete default collection, never inferred paging parameters.
#[derive(Deserialize)]
struct Collection<T> {
    items: Vec<T>,
    #[serde(rename = "totalCount")]
    total_count: u64,
    #[serde(default)]
    pagination: Option<Value>,
    #[serde(default, rename = "nextCursor")]
    next_cursor: Option<Value>,
    #[serde(default)]
    next: Option<Value>,
    #[serde(default, rename = "hasMore")]
    has_more: Option<bool>,
}

impl<T> Collection<T> {
    /// Rejects incomplete, continued, or oversized data before the next request.
    fn validate(&self, limit: usize) -> Result<(), LibraryError> {
        if self.items.len() > limit {
            return Err(LibraryError::LimitExceeded);
        }
        if self.total_count != self.items.len() as u64 {
            return Err(LibraryError::IncompleteDiscovery);
        }
        if self.pagination.is_some()
            || self.next_cursor.is_some()
            || self.next.is_some()
            || self.has_more == Some(true)
        {
            return Err(LibraryError::InvalidPagination);
        }
        Ok(())
    }
}

/// Retains required attribution fields; catalog ownership and free flags are not grants.
#[derive(Deserialize, Eq, PartialEq)]
struct Source {
    #[serde(rename = "sourceId")]
    id: u64,
    name: String,
    #[serde(default, rename = "relativeSlugPath")]
    relative_path: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default, rename = "marketplaceUrl")]
    marketplace_url: Option<String>,
}

impl Source {
    /// Uses the first-party book-path rule or an explicitly supplied marketplace link.
    fn url(&self) -> Result<String, LibraryError> {
        let path = match self
            .relative_path
            .as_deref()
            .filter(|path| !path.is_empty())
        {
            Some(path) => Some(path.to_owned()),
            None => self
                .slug
                .as_deref()
                .filter(|slug| !slug.is_empty())
                .map(|slug| format!("sources/dnd/{slug}")),
        };
        if let Some(path) = path {
            if !path.starts_with("sources/")
                || !path.split('/').all(|segment| {
                    !segment.is_empty()
                        && segment
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
                })
            {
                return Err(LibraryError::InvalidMetadata);
            }
            return Ok(format!("https://www.dndbeyond.com/{path}"));
        }
        let supplied = self
            .marketplace_url
            .as_ref()
            .ok_or(LibraryError::InvalidMetadata)?;
        let url = reqwest::Url::parse(supplied).map_err(|_| LibraryError::InvalidMetadata)?;
        if url.scheme() != "https"
            || url.host_str() != Some("marketplace.dndbeyond.com")
            || url.port().is_some_and(|port| port != 443)
        {
            return Err(LibraryError::InvalidMetadata);
        }
        Ok(supplied.clone())
    }

    /// Validates metadata before ownership is requested, without granting catalog access.
    fn record(&self, evidence: AccessEvidence) -> Result<BookRecord, LibraryError> {
        if self.id == 0 {
            return Err(LibraryError::InvalidMetadata);
        }
        let record = BookRecord {
            id: self.id.to_string(),
            title: self.name.clone(),
            source_url: self.url()?,
            evidence,
        };
        Book::from_record(record.clone())?;
        Ok(record)
    }
}

/// Names account-level ownership only; no campaign context is inferred.
#[derive(Deserialize)]
struct OwnershipRecord {
    #[serde(rename = "sourceId")]
    id: u64,
    ownership: Vec<Observation>,
}

/// Accepts only the two first-party ownership classifications.
#[derive(Deserialize)]
struct Observation {
    #[serde(rename = "type")]
    kind: String,
}

impl Observation {
    /// Preserves purchase and account-sharing as independent explicit observations.
    fn evidence(&self) -> Result<AccessEvidence, LibraryError> {
        match self.kind.as_str() {
            "owned" => Ok(AccessEvidence::Purchased {
                state: AccessState::Granted,
            }),
            "shared" => Ok(AccessEvidence::SharedWithAccount {
                state: AccessState::Granted,
            }),
            _ => Err(LibraryError::InvalidEvidence),
        }
    }
}

/// Checks every raw ID and conflict, cleaning only invalid visible title whitespace before ownership.
fn validate_catalog(
    catalog: &mut Collection<Source>,
    selection: &HashMap<u64, bool>,
) -> Result<(), LibraryError> {
    catalog.validate(MAX_CATALOG_RECORDS)?;
    let mut books = Vec::new();
    let mut hidden = HashMap::new();
    for source in &mut catalog.items {
        if source.id == 0 {
            return Err(LibraryError::InvalidMetadata);
        }
        let included = selection
            .get(&source.id)
            .ok_or(LibraryError::InvalidMetadata)?;
        if *included {
            if !valid_text(&source.name) {
                source.name = source.name.split_whitespace().collect::<Vec<_>>().join(" ");
            }
            append_record(&mut books, source.record(AccessEvidence::CatalogOnly)?)?;
        } else if let Some(previous) = hidden.insert(source.id, &*source)
            && previous != source
        {
            return Err(LibraryError::ConflictingMetadata);
        }
    }
    Ok(())
}

/// Validates all account evidence and emits only Web-catalog sources, counting raw duplicates.
fn assemble(
    catalog: Collection<Source>,
    ownership: Collection<OwnershipRecord>,
    selection: &HashMap<u64, bool>,
) -> Result<LibrarySnapshot, LibraryError> {
    ownership.validate(MAX_ENTRIES_PER_PAGE)?;
    let mut evidence = HashMap::<u64, Vec<AccessEvidence>>::new();
    let mut observations = 0;
    for record in ownership.items {
        if record.id == 0 {
            return Err(LibraryError::InvalidEvidence);
        }
        if record.ownership.len() > MAX_ENTRIES - observations {
            return Err(LibraryError::LimitExceeded);
        }
        observations += record.ownership.len();
        for observation in record.ownership {
            evidence
                .entry(record.id)
                .or_default()
                .push(observation.evidence()?);
        }
        evidence.entry(record.id).or_default();
    }
    if evidence.keys().any(|id| !selection.contains_key(id)) {
        return Err(LibraryError::UnmatchedOwnership);
    }
    let mut books = Vec::new();
    let mut raw_observations = 0;
    for source in catalog.items {
        let supplied = evidence.get(&source.id).filter(|items| !items.is_empty());
        let catalog_only = [AccessEvidence::CatalogOnly];
        let supplied = supplied.map_or(catalog_only.as_slice(), Vec::as_slice);
        if supplied.len() > MAX_ENTRIES - raw_observations {
            return Err(LibraryError::LimitExceeded);
        }
        raw_observations += supplied.len();
        if !*selection
            .get(&source.id)
            .ok_or(LibraryError::InvalidMetadata)?
        {
            continue;
        }
        for observation in supplied {
            append_record(&mut books, source.record(observation.clone())?)?;
        }
    }
    Ok(LibrarySnapshot { books })
}

/// Validates public classification and all account evidence, then joins only Web-catalog sources.
async fn discover(
    session: &Session,
    client: &HttpClient,
    config_client: &HttpClient,
    config: &str,
    sources: &str,
    ownership: &str,
) -> Result<LibrarySnapshot, LibraryError> {
    let configuration: Configuration = config_client
        .get_json(config, None)
        .await
        .map_err(LibraryError::PageRead)?;
    let selection = configuration.selection()?;
    let mut catalog: Collection<Source> = session
        .get_json(client, sources)
        .await
        .map_err(LibraryError::SessionRead)?;
    validate_catalog(&mut catalog, &selection)?;
    let owned = session
        .get_json(client, ownership)
        .await
        .map_err(LibraryError::SessionRead)?;
    assemble(catalog, owned, &selection)
}

/// Discovers complete bounded Web-catalog metadata through the protected Windows session.
///
/// Uses one public and two authenticated fixed GETs without retries or pagination. The complete
/// operation is bounded to 30 seconds. Session loading and exchange remain in
/// the shared lifecycle; each read can make at most one exchange, so the worst
/// case is five HTTP attempts, normally four with bearer reuse. Returned
/// observations cannot authorize a later outline or section read.
/// Account evidence must name configured sources and pass all raw limits.
/// Configured sources outside the Web catalog do not create result entries.
///
/// # Errors
///
/// Returns typed session, transport, malformed-response, completeness, metadata,
/// evidence, conflict, or limit failures. No partial snapshot is returned.
pub async fn discover_library(session: &Session) -> Result<LibrarySnapshot, LibraryError> {
    session.supported().map_err(LibraryError::SessionRead)?;
    #[cfg(any(test, feature = "session-fixture"))]
    if let Some(source) = &session.library_fixture {
        return timeout(
            Duration::from_secs(30),
            discover(
                session,
                &source.client,
                &source.client,
                &source.config,
                &source.sources,
                &source.ownership,
            ),
        )
        .await
        .map_err(|_| LibraryError::TimedOut)?;
    }
    let client = HttpClient::single_attempt(&[ORIGIN]).map_err(LibraryError::PageRead)?;
    let config_client =
        HttpClient::single_attempt(&[CONFIG_ORIGIN]).map_err(LibraryError::PageRead)?;
    timeout(
        Duration::from_secs(30),
        discover(session, &client, &config_client, CONFIG, SOURCES, OWNERSHIP),
    )
    .await
    .map_err(|_| LibraryError::TimedOut)?
}

#[cfg(test)]
mod tests;
