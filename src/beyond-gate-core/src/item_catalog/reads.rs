//! One identity-bound item page, checked before local filtering or selection.

use super::{ItemKind, MagicItemQuery, classify};
use crate::{
    auth::session::{Session, SessionError},
    http::{HttpClient, HttpError},
    public_config::{Configuration, ConfigurationError, valid_text},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashSet, error::Error, fmt, time::Duration};

/// Fixed raw mixed-source page size; either filtered item view can be smaller.
pub const PAGE_SIZE: usize = 100;
/// Maximum explicit one-based page accepted before credential access.
pub const MAX_PAGE: u32 = 1_000_000;
/// Maximum compact JSON result, independent of presentation.
const MAX_RESULT_BYTES: usize = 1024 * 1024;
/// Maximum source references in one raw definition.
const MAX_SOURCES: usize = 1_000;
/// Fixed public identity configuration origin.
const WEBSITE: &str = "https://www.dndbeyond.com";
/// Fixed item service origin, never selected by a record or caller URL.
const SERVICE: &str = "https://character-service.dndbeyond.com";

/// Requests one mixed source page without filling a filtered item view.
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ItemQuery {
    /// One-based page; source requests convert it to zero-based `page`.
    #[serde(default = "first_page")]
    #[schemars(range(min = 1, max = 1_000_000))]
    pub page: u32,
}

/// Supplies the same default first page to JSON input and Rust callers.
fn first_page() -> u32 {
    1
}

impl Default for ItemQuery {
    /// Defaults only omitted paging, never malformed or explicit null input.
    fn default() -> Self {
        Self { page: first_page() }
    }
}

impl ItemQuery {
    /// Checks the explicit page before protected storage or networking.
    ///
    /// # Errors
    ///
    /// Rejects zero or a page beyond [`MAX_PAGE`].
    pub fn validate(&self) -> Result<(), ItemError> {
        if (1..=MAX_PAGE).contains(&self.page) {
            Ok(())
        } else {
            Err(ItemError::InvalidInput)
        }
    }
}

/// Selects one definition from a required fresh source page, never a global scan.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ItemSelection {
    /// Canonical `entityTypeId:id` definition key copied from the list's `id`.
    #[schemars(
        length(max = 41),
        regex(pattern = "^[1-9][0-9]{0,19}:[1-9][0-9]{0,19}$")
    )]
    pub id: String,
    /// Required one-based source page; absence cannot imply global lookup.
    #[schemars(range(min = 1, max = 1_000_000))]
    pub page: u32,
}

impl ItemSelection {
    /// Validates the full identity and explicit source-page context locally.
    ///
    /// # Errors
    ///
    /// Rejects noncanonical identities and invalid pages.
    pub fn validate(&self) -> Result<(), ItemError> {
        validate_id(&self.id)?;
        ItemQuery { page: self.page }.validate()
    }
}

/// Typed, redacted failures shared by equipment and magic-item adapters.
pub enum ItemError {
    /// The source-page selector or composite identity is invalid.
    InvalidInput,
    /// A magic-item source selector or local filter is invalid.
    InvalidMagicInput,
    /// A required envelope, identity, classification, or source field is invalid.
    InvalidResponse,
    /// Returned paging or row count disagrees with the requested source page.
    IncompleteResult,
    /// A full definition key occurs more than once within the page.
    ConflictingIdentity,
    /// The identity is absent from this checked page, not necessarily the catalog.
    MissingRecord,
    /// The selected raw definition is a magic item rather than equipment.
    WrongKind,
    /// The selected raw definition is equipment rather than a magic item.
    WrongMagicKind,
    /// An unverified access or envelope contract requires separate research.
    UnsupportedOperation,
    /// A raw page, references, page selector, or compact result exceeds policy.
    LimitExceeded,
    /// The whole identity-bound operation exceeded its deadline.
    TimedOut,
    /// Preserves the typed HTTP setup or public-source cause.
    SourceRead(HttpError),
    /// Preserves session rejection, expiry, denial, and protected read causes.
    SessionRead(SessionError),
}

impl fmt::Display for ItemError {
    /// Emits safe diagnostics without raw definitions, credentials, or selectors.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid equipment input"),
            Self::InvalidMagicInput => f.write_str("invalid magic-item input"),
            Self::InvalidResponse => f.write_str("invalid item response"),
            Self::IncompleteResult => f.write_str("item source page is incomplete or inconsistent"),
            Self::ConflictingIdentity => f.write_str("conflicting item definition identities"),
            Self::MissingRecord => f.write_str("item record not found in the checked source page"),
            Self::WrongKind => f.write_str("selected item is not equipment"),
            Self::WrongMagicKind => f.write_str("selected item is not a magic item"),
            Self::UnsupportedOperation => {
                f.write_str("item access or source contract is unsupported")
            }
            Self::LimitExceeded => f.write_str("item result limit exceeded"),
            Self::TimedOut => f.write_str("item operation timed out"),
            Self::SourceRead(error) => write!(f, "item source read failed: {error}"),
            Self::SessionRead(error) => {
                write!(f, "item session read failed: {error}")?;
                if let Some(http) = error.source().and_then(|e| e.downcast_ref::<HttpError>()) {
                    write!(f, ": {http}")?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Debug for ItemError {
    /// Keeps debug output as restricted as ordinary diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for ItemError {
    /// Retains typed source and session failures without flattening their causes.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceRead(error) => Some(error),
            Self::SessionRead(error) => Some(error),
            _ => None,
        }
    }
}

/// Checks both positive decimal components of a raw `definitionKey`.
///
/// # Errors
///
/// Rejects bare IDs, zero, leading zeroes, URLs, padding, and components beyond `u64`.
pub fn validate_id(id: &str) -> Result<(), ItemError> {
    let (entity, numeric) = id.split_once(':').ok_or(ItemError::InvalidInput)?;
    for part in [entity, numeric] {
        if !part
            .parse::<u64>()
            .ok()
            .is_some_and(|n| n > 0 && n.to_string() == part)
        {
            return Err(ItemError::InvalidInput);
        }
    }
    Ok(())
}

/// Lists equipment metadata from exactly one freshly checked mixed source page.
///
/// # Errors
///
/// Rejects malformed, mismatched, denied, unsupported, or over-limit responses.
/// A page containing only magic items can legitimately have empty equipment entries.
pub async fn list(session: &Session, query: &ItemQuery) -> Result<Value, ItemError> {
    query.validate()?;
    read(session, query, None, ItemKind::Equipment, None).await
}

/// Returns one unchanged equipment definition from its required fresh source page.
///
/// # Errors
///
/// Returns page-scoped absence or wrong kind only after every row passes validation.
/// Prior metadata and composite identities are not access grants.
pub async fn show(session: &Session, selection: &ItemSelection) -> Result<Value, ItemError> {
    selection.validate()?;
    read(
        session,
        &ItemQuery {
            page: selection.page,
        },
        Some(&selection.id),
        ItemKind::Equipment,
        None,
    )
    .await
}

/// Lists locally filtered magic metadata from one fresh mixed source page.
///
/// # Errors
///
/// Rejects invalid filters and any invalid source row without partial results.
pub async fn list_magic_items(
    session: &Session,
    query: &MagicItemQuery,
) -> Result<Value, ItemError> {
    query.validate()?;
    read(
        session,
        &ItemQuery { page: query.page },
        None,
        ItemKind::MagicItem,
        Some(query),
    )
    .await
}

/// Returns one unchanged magic definition from its required fresh source page.
///
/// # Errors
///
/// Preserves typed source failures, page-scoped absence, and wrong-kind errors.
pub async fn show_magic_item(
    session: &Session,
    selection: &ItemSelection,
) -> Result<Value, ItemError> {
    selection
        .validate()
        .map_err(|_| ItemError::InvalidMagicInput)?;
    read(
        session,
        &ItemQuery {
            page: selection.page,
        },
        Some(&selection.id),
        ItemKind::MagicItem,
        None,
    )
    .await
}

/// Shares fixed request construction, attribution checks, and assembly across both reads.
async fn read(
    session: &Session,
    query: &ItemQuery,
    selected: Option<&str>,
    kind: ItemKind,
    filters: Option<&MagicItemQuery>,
) -> Result<Value, ItemError> {
    session.supported().map_err(ItemError::SessionRead)?;
    let session = session.read_scope();
    tokio::time::timeout(Duration::from_secs(60), async {
        let (client, origin) = source_client(&session, WEBSITE)?;
        let config: Configuration = client
            .get_json(&format!("{origin}/api/config/json"), None)
            .await
            .map_err(ItemError::SourceRead)?;
        config.validate().map_err(|e| match e {
            ConfigurationError::Invalid => ItemError::InvalidResponse,
            ConfigurationError::Conflict => ItemError::ConflictingIdentity,
            ConfigurationError::Limit => ItemError::LimitExceeded,
        })?;
        let sources = config.sources.iter().map(|s| s.id).collect();
        let (client, origin) = source_client(&session, SERVICE)?;
        let body = session
            .get_json(&client, &source_url(&origin, query.page))
            .await
            .map_err(ItemError::SessionRead)?;
        finish(assemble_kind(
            body, query, selected, &sources, kind, filters,
        )?)
    })
    .await
    .map_err(|_| ItemError::TimedOut)?
}

/// Chooses only a fixed production origin or this session owner's compiled fixture.
fn source_client(session: &Session, origin: &str) -> Result<(HttpClient, String), ItemError> {
    session.supported().map_err(ItemError::SessionRead)?;
    #[cfg(any(test, feature = "session-fixture"))]
    if let Some(source) = &session.item_fixture {
        return Ok((source.client.clone(), source.origin.clone()));
    }
    Ok((
        HttpClient::single_attempt(&[origin]).map_err(ItemError::SourceRead)?,
        origin.to_owned(),
    ))
}

/// Maps one-based page context to the observed first-party zero-based query.
fn source_url(origin: &str, page: u32) -> String {
    format!(
        "{origin}/character/v5.1/game-data/items?sharingSetting=2&page={}&pageSize={PAGE_SIZE}",
        page - 1
    )
}

/// Checks a page's exact envelope and all raw rows before producing any result.
#[cfg(test)]
pub(super) fn assemble(
    value: Value,
    query: &ItemQuery,
    selected: Option<&str>,
    sources: &HashSet<u64>,
) -> Result<Value, ItemError> {
    assemble_kind(value, query, selected, sources, ItemKind::Equipment, None)
}

/// Checks the same whole mixed page for either view before local selection.
fn assemble_kind(
    value: Value,
    query: &ItemQuery,
    selected: Option<&str>,
    sources: &HashSet<u64>,
    requested_kind: ItemKind,
    filters: Option<&MagicItemQuery>,
) -> Result<Value, ItemError> {
    query.validate()?;
    let envelope = value.as_object().ok_or(ItemError::InvalidResponse)?;
    if !envelope.get("id").is_some_and(Value::is_number)
        || envelope.get("success").and_then(Value::as_bool) != Some(true)
        || !envelope.get("message").is_some_and(Value::is_string)
    {
        return Err(ItemError::InvalidResponse);
    }
    if envelope.keys().any(|k| {
        !matches!(
            k.as_str(),
            "id" | "success" | "message" | "data" | "pagination"
        )
    }) {
        return Err(ItemError::UnsupportedOperation);
    }
    let rows = envelope
        .get("data")
        .and_then(Value::as_array)
        .ok_or(ItemError::InvalidResponse)?;
    if rows.len() > PAGE_SIZE {
        return Err(ItemError::LimitExceeded);
    }
    let paging = envelope
        .get("pagination")
        .and_then(Value::as_object)
        .ok_or(ItemError::IncompleteResult)?;
    let skip = u64::from(query.page - 1) * PAGE_SIZE as u64;
    let total = paging
        .get("total")
        .and_then(Value::as_u64)
        .ok_or(ItemError::IncompleteResult)?;
    if paging.len() != 3
        || paging.get("skip").and_then(Value::as_u64) != Some(skip)
        || paging.get("take").and_then(Value::as_u64) != Some(PAGE_SIZE as u64)
        || rows.len() as u64 != total.saturating_sub(skip).min(PAGE_SIZE as u64)
    {
        return Err(ItemError::IncompleteResult);
    }
    let next = if total > skip + rows.len() as u64 {
        Some(
            query
                .page
                .checked_add(1)
                .filter(|p| *p <= MAX_PAGE)
                .ok_or(ItemError::LimitExceeded)?,
        )
    } else {
        None
    };
    let url = source_url(SERVICE, query.page);
    let mut keys = HashSet::new();
    let mut entries = Vec::new();
    let mut match_record = None;
    for row in rows {
        let kind = validate_row(row, sources)?;
        let key = row["definitionKey"]
            .as_str()
            .ok_or(ItemError::InvalidResponse)?;
        if !keys.insert(key) {
            return Err(ItemError::ConflictingIdentity);
        }
        if selected == Some(key) {
            match_record = Some((row, kind));
        }
        if kind == requested_kind && selected.is_none() {
            if let Some(filters) = filters
                && !filters.matches(row)?
            {
                continue;
            }
            let mut entry = json!({
                "id": key, "source_id": row["id"], "entity_type_id": row["entityTypeId"],
                "name": row["name"], "filter_type": row["filterType"],
                "cost": row["cost"], "weight": row["weight"], "sources": row["sources"],
                "source_url": url
            });
            if requested_kind == ItemKind::MagicItem {
                entry["rarity"] = row["rarity"].clone();
                entry["tags"] = row["tags"].clone();
                entry["can_attune"] = row["canAttune"].clone();
            }
            entries.push(entry);
        }
    }
    if selected.is_some() {
        let (row, kind) = match_record.ok_or(ItemError::MissingRecord)?;
        if kind != requested_kind {
            return Err(match requested_kind {
                ItemKind::Equipment => ItemError::WrongKind,
                ItemKind::MagicItem => ItemError::WrongMagicKind,
            });
        }
        return Ok(json!({
            "id": row["definitionKey"], "source_id": row["id"],
            "entity_type_id": row["entityTypeId"], "name": row["name"],
            "page": query.page, "source_url": url, "sources": row["sources"], "raw": row
        }));
    }
    let mut result = json!({
        "entries": entries, "source_url": url,
        "pagination": {"page":query.page, "next_page":next,
            "source_page_size":PAGE_SIZE, "source_count":rows.len(), "source_total":total}
    });
    if let Some(filters) = filters {
        result["filters"] = filters.filters();
    }
    Ok(result)
}

/// Validates source identity and metadata without treating attribution as permission.
fn validate_row(row: &Value, sources: &HashSet<u64>) -> Result<ItemKind, ItemError> {
    let object = row.as_object().ok_or(ItemError::InvalidResponse)?;
    if object.contains_key("accessType") || object.contains_key("accessTypes") {
        return Err(ItemError::UnsupportedOperation);
    }
    let kind = classify(row).map_err(|_| ItemError::InvalidResponse)?;
    let id = row["id"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or(ItemError::InvalidResponse)?;
    let entity = row["entityTypeId"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or(ItemError::InvalidResponse)?;
    if row["definitionKey"].as_str() != Some(format!("{entity}:{id}").as_str())
        || !row["name"].as_str().is_some_and(valid_text)
        || !row["filterType"].as_str().is_some_and(valid_text)
        || !row["isHomebrew"].is_boolean()
        || !row["weight"]
            .as_f64()
            .is_some_and(|n| n.is_finite() && n >= 0.0)
        || !object
            .get("cost")
            .is_some_and(|v| v.is_null() || v.as_f64().is_some_and(|n| n.is_finite() && n >= 0.0))
    {
        return Err(ItemError::InvalidResponse);
    }
    let refs = row["sources"]
        .as_array()
        .ok_or(ItemError::InvalidResponse)?;
    if refs.len() > MAX_SOURCES {
        return Err(ItemError::LimitExceeded);
    }
    for reference in refs {
        if !reference["sourceId"]
            .as_u64()
            .is_some_and(|n| sources.contains(&n))
            || !reference["sourceType"].as_u64().is_some_and(|n| n > 0)
            || !reference
                .get("pageNumber")
                .is_some_and(|n| n.is_null() || n.as_u64().is_some())
        {
            return Err(ItemError::InvalidResponse);
        }
    }
    Ok(kind)
}

/// Rejects oversized compact results rather than truncating or returning partial data.
pub(super) fn finish(value: Value) -> Result<Value, ItemError> {
    if serde_json::to_vec(&value)
        .map_err(|_| ItemError::InvalidResponse)?
        .len()
        > MAX_RESULT_BYTES
    {
        return Err(ItemError::LimitExceeded);
    }
    Ok(value)
}
