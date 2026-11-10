//! Strict background website filters shared by CLI, MCP, and the cookie URL guard.

use super::BackgroundError;
use crate::public_config::valid_text;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub use crate::monster::query::PartneredContent;

/// Maximum selected option IDs per filter.
pub const MAX_FILTER_IDS: usize = 50;
/// Maximum UTF-8 bytes in a literal name filter.
pub const MAX_NAME_BYTES: usize = 256;
/// Maximum encoded website query bytes.
pub const MAX_QUERY_BYTES: usize = 16 * 1024;
/// Shared model fields, CLI flags, and observed website controls.
pub const ID_FILTERS: [(&str, &str, &str); 4] = [
    ("skill_ids", "skill-id", "filter-skills"),
    ("tag_ids", "tag-id", "filter-tags"),
    ("source_ids", "source-id", "filter-source"),
    (
        "source_category_ids",
        "source-category-id",
        "filter-source-category",
    ),
];

/// Observed background controls; no unverified pagination or API filters.
#[derive(Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct BackgroundQuery {
    /// Literal website name filter, within 256 UTF-8 bytes.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "String", length(min = 1, max = 256))]
    pub name: Option<String>,
    /// Public skill option IDs, not inferred game-data IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub skill_ids: Vec<u32>,
    /// Public tag option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub tag_ids: Vec<u32>,
    /// Public source option IDs, not access grants.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub source_ids: Vec<u32>,
    /// Public source-category option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub source_category_ids: Vec<u32>,
    /// Website yes/no/any choice; defaults to no.
    pub partnered_content: PartneredContent,
}

/// Rejects explicit null rather than silently removing a supplied filter.
fn present<'de, D: serde::Deserializer<'de>>(value: D) -> Result<Option<String>, D::Error> {
    String::deserialize(value).map(Some)
}

impl BackgroundQuery {
    /// Checks local text, option counts, duplicate IDs, and encoded query size.
    ///
    /// # Errors
    ///
    /// Returns invalid input before any credential or network access.
    pub fn validate(&self) -> Result<(), BackgroundError> {
        if self
            .name
            .as_deref()
            .is_some_and(|s| s.len() > MAX_NAME_BYTES || !valid_text(s))
            || self.id_values().iter().any(|ids| {
                ids.len() > MAX_FILTER_IDS
                    || ids.contains(&0)
                    || ids.iter().collect::<HashSet<_>>().len() != ids.len()
            })
            || self
                .website_url(super::WEBSITE)?
                .query()
                .map_or(0, str::len)
                > MAX_QUERY_BYTES
        {
            return Err(BackgroundError::InvalidInput);
        }
        Ok(())
    }

    /// Keeps option arrays in the exact order of the shared field mapping.
    pub(crate) fn id_values(&self) -> [&[u32]; 4] {
        [
            &self.skill_ids,
            &self.tag_ids,
            &self.source_ids,
            &self.source_category_ids,
        ]
    }

    /// Builds only the fixed listing with observed repeated-key filter encoding.
    pub(crate) fn website_url(&self, origin: &str) -> Result<reqwest::Url, BackgroundError> {
        let mut url = reqwest::Url::parse(&format!("{origin}/backgrounds"))
            .map_err(|_| BackgroundError::InvalidInput)?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(name) = &self.name {
                query.append_pair("filter-name", name);
            }
            for ((_, _, field), ids) in ID_FILTERS.iter().zip(self.id_values()) {
                for id in ids {
                    query.append_pair(field, &id.to_string());
                }
            }
            query.append_pair(
                "filter-partnered-content",
                match self.partnered_content {
                    PartneredContent::Any => "",
                    PartneredContent::Yes => "t",
                    PartneredContent::No => "f",
                },
            );
        }
        Ok(url)
    }
}

/// Restricts cookie transport to the fixed background listing and known selectors.
pub(crate) fn allowed_list_url(url: &reqwest::Url) -> bool {
    if url.path() != "/backgrounds"
        || url.fragment().is_some()
        || !url.query().is_some_and(|q| q.len() <= MAX_QUERY_BYTES)
    {
        return false;
    }
    let mut query = BackgroundQuery::default();
    let mut partnered = false;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "filter-name" if query.name.is_none() => query.name = Some(value.into_owned()),
            "filter-partnered-content" if !partnered => {
                query.partnered_content = match value.as_ref() {
                    "" => PartneredContent::Any,
                    "t" => PartneredContent::Yes,
                    "f" => PartneredContent::No,
                    _ => return false,
                };
                partnered = true;
            }
            "filter-skills" | "filter-tags" | "filter-source" | "filter-source-category" => {
                let Ok(id) = value.parse::<u32>() else {
                    return false;
                };
                if id.to_string() != value {
                    return false;
                }
                let ids = match key.as_ref() {
                    "filter-skills" => &mut query.skill_ids,
                    "filter-tags" => &mut query.tag_ids,
                    "filter-source" => &mut query.source_ids,
                    _ => &mut query.source_category_ids,
                };
                ids.push(id);
            }
            _ => return false,
        }
    }
    partnered && query.validate().is_ok()
}
