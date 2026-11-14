//! Strict page-local magic-item selectors; these never become network parameters.

use super::{ItemError, ItemQuery};
use crate::public_config::valid_text;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;

/// Maximum UTF-8 bytes in each supplied literal filter.
pub const MAX_TEXT_BYTES: usize = 256;
/// Maximum distinct labels in each repeated filter.
pub const MAX_FILTER_VALUES: usize = 50;
/// Shared model fields and repeated CLI flags, using raw labels rather than IDs.
pub const LABEL_FILTERS: [(&str, &str); 3] =
    [("types", "type"), ("rarities", "rarity"), ("tags", "tag")];

/// Selects the raw catalog attunement flag, without interpreting rules text.
#[derive(Clone, Copy, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Attunement {
    /// Does not restrict attunement.
    #[default]
    Any,
    /// Requires raw `canAttune=true`.
    Yes,
    /// Requires raw `canAttune=false`.
    No,
}

/// Filters magic definitions within one checked mixed source page only.
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct MagicItemQuery {
    /// One-based source page, defaulting to 1.
    #[serde(default = "first_page")]
    #[schemars(range(min = 1, max = 1_000_000))]
    pub page: u32,
    /// Case-insensitive literal name substring, not a pattern or global search.
    #[serde(
        deserialize_with = "present_name",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "String", length(min = 1, max = 256))]
    pub name: Option<String>,
    /// Case-insensitive exact raw `filterType` labels; any selected value matches.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub types: Vec<String>,
    /// Case-insensitive exact raw rarity labels; any selected value matches.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub rarities: Vec<String>,
    /// Case-insensitive exact raw tag labels; any selected value matches.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub tags: Vec<String>,
    /// Raw attunement choice; defaults to any.
    pub attunement: Attunement,
}

impl Default for MagicItemQuery {
    /// Uses the same page and unrestricted selectors for Rust and JSON callers.
    fn default() -> Self {
        Self {
            page: first_page(),
            name: None,
            types: Vec::new(),
            rarities: Vec::new(),
            tags: Vec::new(),
            attunement: Attunement::Any,
        }
    }
}

/// Supplies the same default page as the existing item selector.
fn first_page() -> u32 {
    ItemQuery::default().page
}

/// Rejects explicit null rather than discarding a supplied name.
fn present_name<'de, D: serde::Deserializer<'de>>(value: D) -> Result<Option<String>, D::Error> {
    String::deserialize(value).map(Some)
}

impl MagicItemQuery {
    /// Validates selectors before protected storage or network access.
    ///
    /// # Errors
    ///
    /// Rejects invalid pages, text, duplicate labels, or excessive filter counts.
    pub fn validate(&self) -> Result<(), ItemError> {
        if (ItemQuery { page: self.page }).validate().is_err()
            || self
                .name
                .as_deref()
                .is_some_and(|value| !valid_filter_text(value))
            || self.label_values().iter().any(|values| {
                values.len() > MAX_FILTER_VALUES
                    || values.iter().any(|value| !valid_filter_text(value))
                    || values
                        .iter()
                        .map(|value| value.to_lowercase())
                        .collect::<HashSet<_>>()
                        .len()
                        != values.len()
            })
        {
            return Err(ItemError::InvalidMagicInput);
        }
        Ok(())
    }

    /// Keeps repeated values in the shared CLI field mapping order.
    pub fn label_values(&self) -> [&[String]; 3] {
        [&self.types, &self.rarities, &self.tags]
    }

    /// Checks all required magic metadata before applying local matching.
    pub(super) fn matches(&self, row: &Value) -> Result<bool, ItemError> {
        let rarity = row["rarity"]
            .as_str()
            .filter(|s| valid_text(s))
            .ok_or(ItemError::InvalidResponse)?;
        let attune = row["canAttune"]
            .as_bool()
            .ok_or(ItemError::InvalidResponse)?;
        let tags = row["tags"].as_array().ok_or(ItemError::InvalidResponse)?;
        if tags.iter().any(|tag| !tag.as_str().is_some_and(valid_text)) {
            return Err(ItemError::InvalidResponse);
        }
        let name = row["name"].as_str().ok_or(ItemError::InvalidResponse)?;
        let kind = row["filterType"]
            .as_str()
            .ok_or(ItemError::InvalidResponse)?;
        Ok(self
            .name
            .as_ref()
            .is_none_or(|value| name.to_lowercase().contains(&value.to_lowercase()))
            && matches_label(&self.types, kind)
            && matches_label(&self.rarities, rarity)
            && (self.tags.is_empty()
                || tags.iter().any(|tag| {
                    tag.as_str()
                        .is_some_and(|value| matches_label(&self.tags, value))
                }))
            && match self.attunement {
                Attunement::Any => true,
                Attunement::Yes => attune,
                Attunement::No => !attune,
            })
    }

    /// Reports local selectors separately from unchanged mixed-source pagination.
    pub(super) fn filters(&self) -> Value {
        json!({"name":self.name,"types":self.types,"rarities":self.rarities,
            "tags":self.tags,"attunement":self.attunement})
    }
}

/// Checks bounded literal input without trimming or changing caller intent.
pub fn valid_filter_text(value: &str) -> bool {
    value.len() <= MAX_TEXT_BYTES && valid_text(value)
}

/// Matches any exact label without treating an unmatched label as invalid input.
fn matches_label(values: &[String], label: &str) -> bool {
    values.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase() == label.to_lowercase())
}
