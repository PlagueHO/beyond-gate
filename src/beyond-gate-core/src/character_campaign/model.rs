//! Normalized character and campaign records, not an upstream schema.

use std::fmt;

use reqwest::Url;
use serde_json::Value;

use super::CharacterCampaignError;

/// Supplies one explicitly normalized page without a fetch URL.
///
/// Cursors are opaque metadata. The first page has no cursor, and `next_cursor`
/// is absent only on the terminal page.
#[derive(Clone)]
pub struct ResourcePage<T> {
    /// Identifies this page; `None` is required for the initial page.
    pub cursor: Option<String>,
    /// Contains raw observations; duplicates still consume the entry budget.
    pub entries: Vec<T>,
    /// Identifies the required next page, or explicitly marks completion.
    pub next_cursor: Option<String>,
}

impl<T> fmt::Debug for ResourcePage<T> {
    /// Omits continuation values and all raw record metadata.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResourcePage")
            .field("entries", &self.entries.len())
            .field("continuation", &self.next_cursor.is_some())
            .finish_non_exhaustive()
    }
}

/// A normalized character page supplied to offline assembly.
pub type CharacterPage = ResourcePage<CharacterRecord>;
/// A normalized campaign page supplied to offline assembly.
pub type CampaignPage = ResourcePage<CampaignRecord>;

/// Supplies one normalized character record for offline assembly.
///
/// `id` is opaque. `name` and `source_url` are optional local summary metadata.
/// `raw` preserves the supplied JSON without interpreting game rules or
/// claiming that its shape matches an upstream response.
#[derive(Clone)]
pub struct CharacterRecord {
    /// Opaque identifier used for duplicate detection in one assembly.
    pub id: String,
    /// Optional display metadata; not inferred from the identifier.
    pub name: Option<String>,
    /// Optional HTTPS source location; metadata only, never a request instruction.
    pub source_url: Option<String>,
    /// Entire supplied JSON value, including fields unknown to this crate.
    pub raw: Value,
}

impl fmt::Debug for CharacterRecord {
    /// Omits identifiers, names, source URLs, and raw account data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CharacterRecord { metadata: [REDACTED] }")
    }
}

/// Contains validated character metadata and the original supplied JSON value.
#[derive(Clone, Eq, PartialEq)]
pub struct Character {
    id: String,
    name: Option<String>,
    source_url: Option<Url>,
    raw: Value,
}

impl Character {
    /// Validates one normalized record before it enters a snapshot.
    pub(super) fn from_record(record: CharacterRecord) -> Result<Self, CharacterCampaignError> {
        let source_url = validate_record(
            &record.id,
            record.name.as_deref(),
            record.source_url.as_deref(),
        )?;
        Ok(Self {
            id: record.id,
            name: record.name,
            source_url,
            raw: record.raw,
        })
    }

    /// Returns the opaque character identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns optional validated display metadata.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the optional source URL as metadata, not request authorization.
    pub fn source_url(&self) -> Option<&Url> {
        self.source_url.as_ref()
    }

    /// Returns the original JSON value, including unknown fields.
    pub fn raw(&self) -> &Value {
        &self.raw
    }

    /// Rejects different observations for the same opaque identifier.
    pub(super) fn merge(&mut self, other: Self) -> Result<(), CharacterCampaignError> {
        if self.id != other.id
            || self.name != other.name
            || self.source_url != other.source_url
            || self.raw != other.raw
        {
            return Err(CharacterCampaignError::ConflictingRecord);
        }
        Ok(())
    }
}

impl fmt::Debug for Character {
    /// Omits identifiers, names, URLs, and raw account data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Character { metadata: [REDACTED] }")
    }
}

/// Supplies one normalized campaign record for offline assembly.
///
/// `character_ids` contains opaque associations. The crate does not infer a
/// campaign schema or require associated characters to appear in another list.
/// `raw` preserves all supplied JSON fields without interpretation.
#[derive(Clone)]
pub struct CampaignRecord {
    /// Opaque identifier used for duplicate detection in one assembly.
    pub id: String,
    /// Optional display metadata; not inferred from the identifier.
    pub name: Option<String>,
    /// Optional HTTPS source location; metadata only, never a request instruction.
    pub source_url: Option<String>,
    /// Opaque character identifiers associated by the supplied observation.
    pub character_ids: Vec<String>,
    /// Entire supplied JSON value, including fields unknown to this crate.
    pub raw: Value,
}

impl fmt::Debug for CampaignRecord {
    /// Omits identifiers, names, source URLs, associations, and raw account data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CampaignRecord { metadata: [REDACTED] }")
    }
}

/// Contains validated campaign metadata and the original supplied JSON value.
#[derive(Clone, Eq, PartialEq)]
pub struct Campaign {
    id: String,
    name: Option<String>,
    source_url: Option<Url>,
    character_ids: Vec<String>,
    raw: Value,
}

impl Campaign {
    /// Validates one normalized record before it enters a snapshot.
    pub(super) fn from_record(record: CampaignRecord) -> Result<Self, CharacterCampaignError> {
        let source_url = validate_record(
            &record.id,
            record.name.as_deref(),
            record.source_url.as_deref(),
        )?;
        if record.character_ids.iter().any(|id| !valid_text(id)) {
            return Err(CharacterCampaignError::InvalidMetadata);
        }
        Ok(Self {
            id: record.id,
            name: record.name,
            source_url,
            character_ids: record.character_ids,
            raw: record.raw,
        })
    }

    /// Returns the opaque campaign identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns optional validated display metadata.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the optional source URL as metadata, not request authorization.
    pub fn source_url(&self) -> Option<&Url> {
        self.source_url.as_ref()
    }

    /// Returns the supplied character associations in their original order.
    pub fn character_ids(&self) -> &[String] {
        &self.character_ids
    }

    /// Returns the original JSON value, including unknown fields.
    pub fn raw(&self) -> &Value {
        &self.raw
    }

    /// Rejects different observations for the same opaque identifier.
    pub(super) fn merge(&mut self, other: Self) -> Result<(), CharacterCampaignError> {
        if self.id != other.id
            || self.name != other.name
            || self.source_url != other.source_url
            || self.character_ids != other.character_ids
            || self.raw != other.raw
        {
            return Err(CharacterCampaignError::ConflictingRecord);
        }
        Ok(())
    }
}

impl fmt::Debug for Campaign {
    /// Omits identifiers, names, URLs, associations, and raw account data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Campaign { metadata: [REDACTED] }")
    }
}

/// Holds one complete assembly of supplied character pages.
#[derive(Clone, Eq, PartialEq)]
pub struct CharacterSnapshot {
    pub(super) characters: Vec<Character>,
}

impl CharacterSnapshot {
    /// Returns unique characters in first-seen order.
    pub fn characters(&self) -> &[Character] {
        &self.characters
    }

    /// Finds a character in this complete snapshot, or returns `None`.
    ///
    /// `None` means only that this supplied snapshot has no matching record. It
    /// does not prove that an upstream character is missing or inaccessible.
    pub fn get(&self, id: &str) -> Option<&Character> {
        self.characters
            .iter()
            .find(|character| character.id() == id)
    }
}

impl fmt::Debug for CharacterSnapshot {
    /// Reports only the number of characters, not their account data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CharacterSnapshot")
            .field("characters", &self.characters.len())
            .finish()
    }
}

/// Holds one complete assembly of supplied campaign pages.
#[derive(Clone, Eq, PartialEq)]
pub struct CampaignSnapshot {
    pub(super) campaigns: Vec<Campaign>,
}

impl CampaignSnapshot {
    /// Returns unique campaigns in first-seen order.
    pub fn campaigns(&self) -> &[Campaign] {
        &self.campaigns
    }

    /// Finds a campaign in this complete snapshot, or returns `None`.
    ///
    /// `None` means only that this supplied snapshot has no matching record. It
    /// does not prove that an upstream campaign is missing or inaccessible.
    pub fn get(&self, id: &str) -> Option<&Campaign> {
        self.campaigns.iter().find(|campaign| campaign.id() == id)
    }
}

impl fmt::Debug for CampaignSnapshot {
    /// Reports only the number of campaigns, not their account data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CampaignSnapshot")
            .field("campaigns", &self.campaigns.len())
            .finish()
    }
}

/// Validates opaque identity and optional display metadata.
fn validate_record(
    id: &str,
    name: Option<&str>,
    source_url: Option<&str>,
) -> Result<Option<Url>, CharacterCampaignError> {
    if !valid_text(id) || name.is_some_and(|value| !valid_text(value)) {
        return Err(CharacterCampaignError::InvalidMetadata);
    }
    source_url.map(parse_source_url).transpose()
}

/// Parses an HTTPS source URL without using it as a request instruction.
fn parse_source_url(value: &str) -> Result<Url, CharacterCampaignError> {
    let url = Url::parse(value).map_err(|_| CharacterCampaignError::InvalidMetadata)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || value.chars().any(char::is_control)
        || value.trim() != value
    {
        return Err(CharacterCampaignError::InvalidMetadata);
    }
    Ok(url)
}

/// Requires meaningful, unpadded metadata without terminal control characters.
pub(super) fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}
