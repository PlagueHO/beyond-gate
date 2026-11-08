//! Bounded, read-only character and campaign reads and record assembly.
//!
//! The normalized records are not an upstream schema. Assembly validates
//! explicitly supplied pages. Live reads use fixed source routes and a
//! protected Windows session. Synthetic fixtures do not establish live access.

mod error;
mod model;
mod source;

#[cfg(any(test, feature = "session-fixture"))]
pub mod fixture;

pub use error::CharacterCampaignError;
pub use model::{
    Campaign, CampaignPage, CampaignRecord, CampaignSnapshot, Character, CharacterPage,
    CharacterRecord, CharacterSnapshot, ResourcePage,
};

use std::collections::HashSet;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::time::timeout;

use crate::{
    auth::session::Session,
    http::{HttpClient, HttpError},
};
use model::valid_text;

/// Fixed character-service origin for approved read-only routes.
const CHARACTER_ORIGIN: &str = "https://character-service.dndbeyond.com";
/// Fixed website origin for approved campaign routes.
const WEBSITE_ORIGIN: &str = "https://www.dndbeyond.com";
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

/// Reads the complete account-scoped character collection.
///
/// The service's default response is accepted only when its offset is zero,
/// its reported total matches the raw returned count, and all records fit
/// within the existing page limit.
///
/// # Errors
///
/// Returns session, transport, malformed-response, completeness, or limit
/// failures. No partial snapshot is returned.
pub async fn list_characters(
    session: &Session,
) -> Result<CharacterSnapshot, CharacterCampaignError> {
    let session = session.read_scope();
    timeout(Duration::from_secs(30), async {
        let account_id = session
            .account_id()
            .await
            .map_err(CharacterCampaignError::SessionRead)?;
        let (client, origin) = source_client(&session, CHARACTER_ORIGIN)?;
        let url = format!("{origin}/character/v5/characters/list?userId={account_id}");
        let response: Value = session
            .get_json(&client, &url)
            .await
            .map_err(CharacterCampaignError::SessionRead)?;
        let entries = source::character_list(response)?;
        assemble_characters([Ok(CharacterPage {
            cursor: None,
            entries,
            next_cursor: None,
        })])
    })
    .await
    .map_err(|_| CharacterCampaignError::TimedOut)?
}

/// Reads one explicitly selected character from the fixed character service.
///
/// The selection must be a positive decimal source identifier. The returned
/// raw JSON keeps campaign and unknown fields without interpreting them.
///
/// # Errors
///
/// Returns session, transport, malformed-response, or selection failures.
pub async fn get_character(
    session: &Session,
    id: &str,
) -> Result<Character, CharacterCampaignError> {
    let numeric_id = selected_id(id)?;
    let session = session.read_scope();
    timeout(Duration::from_secs(30), async {
        let (client, origin) = source_client(&session, CHARACTER_ORIGIN)?;
        let url = format!("{origin}/character/v5/character/{numeric_id}?includeCustomItems=true");
        let response: Value = session.get_json(&client, &url).await.map_err(|error| {
            if error.kind() == crate::auth::session::SessionErrorKind::NotFound {
                CharacterCampaignError::MissingRecord
            } else {
                CharacterCampaignError::SessionRead(error)
            }
        })?;
        let record = source::character_show(response, id)?;
        let snapshot = assemble_characters([Ok(CharacterPage {
            cursor: None,
            entries: vec![record],
            next_cursor: None,
        })])?;
        snapshot
            .get(id)
            .cloned()
            .ok_or(CharacterCampaignError::MissingRecord)
    })
    .await
    .map_err(|_| CharacterCampaignError::TimedOut)?
}

/// Reads bounded campaign summaries from the selected fixed collection.
///
/// `active_only` selects the source's active-campaign route. Campaign list
/// reads do not request rosters, so their association state remains unknown.
///
/// # Errors
///
/// Returns session, transport, malformed-response, or limit failures.
pub async fn list_campaigns(
    session: &Session,
    active_only: bool,
) -> Result<CampaignSnapshot, CharacterCampaignError> {
    let session = session.read_scope();
    timeout(Duration::from_secs(30), async {
        let (client, origin) = source_client(&session, WEBSITE_ORIGIN)?;
        let url = if active_only {
            format!("{origin}/api/campaign/stt/active-campaigns")
        } else {
            format!("{origin}/api/campaign/stt/user-campaigns")
        };
        let response: Value = session
            .get_json(&client, &url)
            .await
            .map_err(CharacterCampaignError::SessionRead)?;
        let entries = source::campaign_list(response)?;
        assemble_campaigns([Ok(CampaignPage {
            cursor: None,
            entries,
            next_cursor: None,
        })])
    })
    .await
    .map_err(|_| CharacterCampaignError::TimedOut)?
}

/// Reads one default campaign summary and its selected active roster.
///
/// Only the selected campaign's roster is requested. The returned associations
/// describe that active roster, not inactive participants or full details.
///
/// # Errors
///
/// Returns session, transport, malformed-response, or missing-record failures.
pub async fn get_campaign(session: &Session, id: &str) -> Result<Campaign, CharacterCampaignError> {
    let numeric_id = selected_id(id)?;
    let session = session.read_scope();
    timeout(Duration::from_secs(30), async {
        let (client, origin) = source_client(&session, WEBSITE_ORIGIN)?;
        let user_campaigns = format!("{origin}/api/campaign/stt/user-campaigns");
        let response: Value = session
            .get_json(&client, &user_campaigns)
            .await
            .map_err(CharacterCampaignError::SessionRead)?;
        let mut summaries = source::campaign_list(response)?;
        let selected = summaries
            .iter()
            .position(|campaign| campaign.id == id)
            .ok_or(CharacterCampaignError::MissingRecord)?;
        let mut summary = summaries.swap_remove(selected);

        let roster_url = format!("{origin}/api/campaign/stt/active-short-characters/{numeric_id}");
        let roster: Value = session
            .get_json(&client, &roster_url)
            .await
            .map_err(CharacterCampaignError::SessionRead)?;
        let characters = source::campaign_roster(roster)?;
        let character_ids = characters
            .iter()
            .map(|character| source::numeric_source_id(character, "id"))
            .collect::<Result<Vec<_>, _>>()?;
        summary.character_ids = Some(character_ids);
        summary.raw = json!({"summary": summary.raw, "characters": characters});
        Campaign::from_record(summary)
    })
    .await
    .map_err(|_| CharacterCampaignError::TimedOut)?
}

/// Selects only a compiled synthetic source or the fixed production origin.
fn source_client(
    session: &Session,
    production_origin: &str,
) -> Result<(HttpClient, String), CharacterCampaignError> {
    #[cfg(any(test, feature = "session-fixture"))]
    if let Some(source) = &session.character_campaign_fixture {
        return Ok((source.client.clone(), source.origin.clone()));
    }
    #[cfg(not(any(test, feature = "session-fixture")))]
    let _ = session;
    let client = HttpClient::single_attempt(&[production_origin])
        .map_err(CharacterCampaignError::PageRead)?;
    Ok((client, production_origin.to_owned()))
}

/// Validates the canonical decimal form used to build fixed numeric routes.
fn selected_id(id: &str) -> Result<u64, CharacterCampaignError> {
    let parsed = id
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .filter(|value| value.to_string() == id)
        .ok_or(CharacterCampaignError::InvalidMetadata)?;
    Ok(parsed)
}

#[cfg(test)]
mod tests;
