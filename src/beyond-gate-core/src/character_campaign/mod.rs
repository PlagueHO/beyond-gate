//! Offline character and campaign records with bounded page assembly.
//!
//! The normalized records are not an upstream schema. Assembly validates
//! explicitly supplied pages only. Live list and read operations remain
//! unsupported until usable upstream contracts and login are implemented.

mod error;
mod model;

pub use error::CharacterCampaignError;
pub use model::{
    Campaign, CampaignPage, CampaignRecord, CampaignSnapshot, Character, CharacterPage,
    CharacterRecord, CharacterSnapshot, ResourcePage,
};

use std::collections::HashSet;

use crate::http::HttpError;
use model::valid_text;

/// Maximum raw records in one supplied page, before duplicate removal.
pub const MAX_ENTRIES_PER_PAGE: usize = 100;
/// Maximum supplied pages in one complete assembly, including empty pages.
pub const MAX_PAGES: usize = 10;
/// Maximum raw records in one assembly, including duplicates.
pub const MAX_ENTRIES: usize = 1_000;

/// Validates a complete, explicitly supplied sequence of character pages.
///
/// The first page must have no cursor. Each later cursor must match the
/// preceding `next_cursor`, and the final page must have no continuation.
/// These are local safety limits, not upstream limits. No page is fetched and
/// no prior result is reused. The limits apply after callers have allocated
/// each page and its JSON values; they do not impose an input byte limit.
///
/// # Examples
///
/// ```
/// use beyond_gate_core::character_campaign::{
///     CharacterPage, assemble_characters,
/// };
///
/// let page = CharacterPage {
///     cursor: None,
///     entries: vec![],
///     next_cursor: None,
/// };
/// let snapshot = assemble_characters([Ok(page)])?;
/// assert!(snapshot.characters().is_empty());
/// # Ok::<(), beyond_gate_core::character_campaign::CharacterCampaignError>(())
/// ```
///
/// # Errors
///
/// Returns a typed validation, conflict, pagination, limit, incomplete input,
/// or supplied HTTP failure. It never returns a partial snapshot.
pub fn assemble_characters(
    pages: impl IntoIterator<Item = Result<CharacterPage, HttpError>>,
) -> Result<CharacterSnapshot, CharacterCampaignError> {
    let mut characters: Vec<Character> = Vec::new();
    let mut expected_cursor: Option<String> = None;
    let mut seen_cursors = HashSet::new();
    let mut complete = false;
    let mut entries = 0_usize;

    for (index, page) in pages.into_iter().enumerate() {
        if index >= MAX_PAGES {
            return Err(CharacterCampaignError::LimitExceeded);
        }
        if complete {
            return Err(CharacterCampaignError::InvalidPagination);
        }
        let page = page.map_err(CharacterCampaignError::PageRead)?;
        if page.cursor != expected_cursor {
            return Err(CharacterCampaignError::InvalidPagination);
        }
        if let Some(next) = &page.next_cursor
            && (!valid_text(next) || !seen_cursors.insert(next.clone()))
        {
            return Err(CharacterCampaignError::InvalidPagination);
        }
        if page.entries.len() > MAX_ENTRIES_PER_PAGE || page.entries.len() > MAX_ENTRIES - entries {
            return Err(CharacterCampaignError::LimitExceeded);
        }
        entries += page.entries.len();
        for record in page.entries {
            let character = Character::from_record(record)?;
            match characters
                .iter_mut()
                .find(|existing| existing.id() == character.id())
            {
                Some(existing) => existing.merge(character)?,
                None => characters.push(character),
            }
        }
        complete = page.next_cursor.is_none();
        expected_cursor = page.next_cursor;
    }
    if !complete {
        return Err(CharacterCampaignError::IncompleteDiscovery);
    }
    Ok(CharacterSnapshot { characters })
}

/// Validates a complete, explicitly supplied sequence of campaign pages.
///
/// Character associations remain opaque identifiers. Assembly does not claim
/// that associated characters are visible in a separate character result.
/// Pages are never fetched, and these fixed caps are local policy.
/// The caps do not bound caller-side page or JSON allocations.
///
/// # Errors
///
/// Returns a typed validation, conflict, pagination, limit, incomplete input,
/// or supplied HTTP failure. It never returns a partial snapshot.
pub fn assemble_campaigns(
    pages: impl IntoIterator<Item = Result<CampaignPage, HttpError>>,
) -> Result<CampaignSnapshot, CharacterCampaignError> {
    let mut campaigns: Vec<Campaign> = Vec::new();
    let mut expected_cursor: Option<String> = None;
    let mut seen_cursors = HashSet::new();
    let mut complete = false;
    let mut entries = 0_usize;

    for (index, page) in pages.into_iter().enumerate() {
        if index >= MAX_PAGES {
            return Err(CharacterCampaignError::LimitExceeded);
        }
        if complete {
            return Err(CharacterCampaignError::InvalidPagination);
        }
        let page = page.map_err(CharacterCampaignError::PageRead)?;
        if page.cursor != expected_cursor {
            return Err(CharacterCampaignError::InvalidPagination);
        }
        if let Some(next) = &page.next_cursor
            && (!valid_text(next) || !seen_cursors.insert(next.clone()))
        {
            return Err(CharacterCampaignError::InvalidPagination);
        }
        if page.entries.len() > MAX_ENTRIES_PER_PAGE || page.entries.len() > MAX_ENTRIES - entries {
            return Err(CharacterCampaignError::LimitExceeded);
        }
        entries += page.entries.len();
        for record in page.entries {
            let campaign = Campaign::from_record(record)?;
            match campaigns
                .iter_mut()
                .find(|existing| existing.id() == campaign.id())
            {
                Some(existing) => existing.merge(campaign)?,
                None => campaigns.push(campaign),
            }
        }
        complete = page.next_cursor.is_none();
        expected_cursor = page.next_cursor;
    }
    if !complete {
        return Err(CharacterCampaignError::IncompleteDiscovery);
    }
    Ok(CampaignSnapshot { campaigns })
}

/// Reports that live character discovery is not available.
///
/// This function does not construct an HTTP client, access credentials, or
/// send a request. A successful empty list must not be used as a fallback.
///
/// # Errors
///
/// Always returns [`CharacterCampaignError::UnsupportedOperation`].
pub fn list_characters() -> Result<CharacterSnapshot, CharacterCampaignError> {
    Err(CharacterCampaignError::UnsupportedOperation)
}

/// Reports that live character reads are not available.
///
/// This function does not construct an HTTP client, access credentials, or
/// send a request. It does not interpret an identifier without a read contract.
///
/// # Errors
///
/// Always returns [`CharacterCampaignError::UnsupportedOperation`].
pub fn get_character(_id: &str) -> Result<Character, CharacterCampaignError> {
    Err(CharacterCampaignError::UnsupportedOperation)
}

/// Reports that live campaign discovery is not available.
///
/// This function does not construct an HTTP client, access credentials, or
/// send a request. A successful empty list must not be used as a fallback.
///
/// # Errors
///
/// Always returns [`CharacterCampaignError::UnsupportedOperation`].
pub fn list_campaigns() -> Result<CampaignSnapshot, CharacterCampaignError> {
    Err(CharacterCampaignError::UnsupportedOperation)
}

/// Reports that live campaign reads are not available.
///
/// This function does not construct an HTTP client, access credentials, or
/// send a request. It does not interpret an identifier without a read contract.
///
/// # Errors
///
/// Always returns [`CharacterCampaignError::UnsupportedOperation`].
pub fn get_campaign(_id: &str) -> Result<Campaign, CharacterCampaignError> {
    Err(CharacterCampaignError::UnsupportedOperation)
}

#[cfg(test)]
mod tests;
