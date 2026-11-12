//! Observed feat website selectors and the matching cookie transport URL guard.

use super::{FeatError, WEBSITE};
use crate::public_config::valid_text;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub use crate::monster::query::PartneredContent;

/// Maximum distinct selected IDs in each multi-select control.
pub const MAX_FILTER_IDS: usize = 50;
/// Maximum literal name bytes, measured as UTF-8.
pub const MAX_NAME_BYTES: usize = 256;
/// Maximum encoded query bytes.
pub const MAX_QUERY_BYTES: usize = 16 * 1024;
/// Shared model field, CLI flag, and observed website multi-select control.
pub const ID_FILTERS: [(&str, &str, &str); 3] = [
    ("tag_ids", "tag-id", "filter-tags"),
    ("source_ids", "source-id", "filter-source"),
    (
        "source_category_ids",
        "source-category-id",
        "filter-source-category",
    ),
];

/// Website option IDs are selectors, not rules identities or access grants.
#[derive(Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct FeatQuery {
    /// Optional literal feat name, without padding or control characters.
    #[serde(
        default,
        deserialize_with = "present_name",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "String", length(min = 1, max = 256))]
    pub name: Option<String>,
    /// One positive option ID from the website's single prerequisite control.
    #[serde(
        default,
        deserialize_with = "present_id",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "u32", range(min = 1))]
    pub prerequisite_subtype_id: Option<u32>,
    /// Up to 50 distinct positive website tag option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub tag_ids: Vec<u32>,
    /// Up to 50 distinct positive website source option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub source_ids: Vec<u32>,
    /// Up to 50 distinct positive website source category option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub source_category_ids: Vec<u32>,
    /// Website yes/no/any selector; defaults to no.
    pub partnered_content: PartneredContent,
}

/// Rejects explicit null rather than silently dropping a supplied name.
fn present_name<'de, D: serde::Deserializer<'de>>(value: D) -> Result<Option<String>, D::Error> {
    String::deserialize(value).map(Some)
}

/// Rejects explicit null rather than silently dropping a supplied prerequisite.
fn present_id<'de, D: serde::Deserializer<'de>>(value: D) -> Result<Option<u32>, D::Error> {
    u32::deserialize(value).map(Some)
}

impl FeatQuery {
    /// Validates local text, scalar/array IDs, duplicates, and encoded size.
    ///
    /// # Errors
    ///
    /// Returns invalid input before protected storage or network use.
    pub fn validate(&self) -> Result<(), FeatError> {
        if self
            .name
            .as_deref()
            .is_some_and(|s| s.len() > MAX_NAME_BYTES || !valid_text(s))
            || self.prerequisite_subtype_id == Some(0)
            || self.id_values().iter().any(|ids| {
                ids.len() > MAX_FILTER_IDS
                    || ids.contains(&0)
                    || ids.iter().collect::<HashSet<_>>().len() != ids.len()
            })
            || self.website_url(WEBSITE)?.query().map_or(0, str::len) > MAX_QUERY_BYTES
        {
            return Err(FeatError::InvalidInput);
        }
        Ok(())
    }

    /// Keeps arrays in the same order as the shared control mapping.
    pub(crate) fn id_values(&self) -> [&[u32]; 3] {
        [&self.tag_ids, &self.source_ids, &self.source_category_ids]
    }

    /// Builds the fixed listing using literal text and observed form controls.
    pub(crate) fn website_url(&self, origin: &str) -> Result<reqwest::Url, FeatError> {
        let mut url =
            reqwest::Url::parse(&format!("{origin}/feats")).map_err(|_| FeatError::InvalidInput)?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(name) = &self.name {
                query.append_pair("filter-name", name);
            }
            if let Some(id) = self.prerequisite_subtype_id {
                query.append_pair("filter-prereq-subtype", &id.to_string());
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

/// Restricts cookie reads to the fixed listing and checked selector shapes.
pub(crate) fn allowed_list_url(url: &reqwest::Url) -> bool {
    if url.path() != "/feats"
        || url.fragment().is_some()
        || !url.query().is_some_and(|q| q.len() <= MAX_QUERY_BYTES)
    {
        return false;
    }
    let mut query = FeatQuery::default();
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
            "filter-prereq-subtype"
            | "filter-tags"
            | "filter-source"
            | "filter-source-category" => {
                let Ok(id) = value.parse::<u32>() else {
                    return false;
                };
                if id.to_string() != value {
                    return false;
                }
                match key.as_ref() {
                    "filter-prereq-subtype" if query.prerequisite_subtype_id.is_none() => {
                        query.prerequisite_subtype_id = Some(id)
                    }
                    "filter-tags" => query.tag_ids.push(id),
                    "filter-source" => query.source_ids.push(id),
                    "filter-source-category" => query.source_category_ids.push(id),
                    _ => return false,
                }
            }
            _ => return false,
        }
    }
    partnered && query.validate().is_ok()
}
