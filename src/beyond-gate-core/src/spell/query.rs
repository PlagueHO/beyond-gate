//! Strict website spell selectors and a resource-specific cookie URL guard.

use super::{SpellError, WEBSITE, validate_slug};
pub use crate::monster::query::PartneredContent;
use crate::public_config::valid_text;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Model fields, CLI flags, and observed website option controls.
pub const ID_FILTERS: [(&str, &str, &str); 11] = [
    ("class_ids", "class-id", "filter-class"),
    ("levels", "level", "filter-level"),
    ("school_ids", "school-id", "filter-school"),
    ("casting_time_ids", "casting-time-id", "filter-casting-time"),
    ("attack_type_ids", "attack-type-id", "filter-attack-type"),
    ("save_ids", "save-id", "filter-save-required"),
    ("damage_type_ids", "damage-type-id", "filter-damage-type"),
    ("condition_ids", "condition-id", "filter-condition"),
    ("tag_ids", "tag-id", "filter-tags"),
    ("source_ids", "source-id", "filter-source"),
    (
        "source_category_ids",
        "source-category-id",
        "filter-source-category",
    ),
];
/// Optional boolean controls; concentration uses 1/2, the others use t/f.
pub const BOOL_FILTERS: [(&str, &str, &str); 5] = [
    ("verbal", "verbal", "filter-verbal"),
    ("somatic", "somatic", "filter-somatic"),
    ("material", "material", "filter-material"),
    ("concentration", "concentration", "filter-concentration"),
    ("ritual", "ritual", "filter-ritual"),
];

/// One explicit website spell page; option IDs do not grant content access.
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct SpellQuery {
    /// Literal name filter within 256 UTF-8 bytes.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String", length(min = 1, max = 256))]
    pub name: Option<String>,
    /// Positive explicit website page.
    #[schemars(range(min = 1))]
    pub page: u32,
    /// Distinct website class IDs, including separate legacy identities.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub class_ids: Vec<u32>,
    /// Distinct levels 0 through 9; zero means cantrips.
    #[schemars(length(max = 10), extend("uniqueItems" = true))]
    pub levels: Vec<u32>,
    /// Distinct website school IDs, 3 through 10.
    #[schemars(length(max = 8), extend("uniqueItems" = true))]
    pub school_ids: Vec<u32>,
    /// Website casting-time option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub casting_time_ids: Vec<u32>,
    /// Website attack-type option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub attack_type_ids: Vec<u32>,
    /// Website required-save option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub save_ids: Vec<u32>,
    /// Website damage-type option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub damage_type_ids: Vec<u32>,
    /// Website condition option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub condition_ids: Vec<u32>,
    /// Website tag option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub tag_ids: Vec<u32>,
    /// Website source option IDs, not ownership evidence.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub source_ids: Vec<u32>,
    /// Website source-category option IDs.
    #[schemars(length(max = 50), extend("uniqueItems" = true))]
    pub source_category_ids: Vec<u32>,
    /// Optional verbal component restriction.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub verbal: Option<bool>,
    /// Optional somatic component restriction.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub somatic: Option<bool>,
    /// Optional material component restriction.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub material: Option<bool>,
    /// Optional concentration restriction.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub concentration: Option<bool>,
    /// Optional ritual restriction.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub ritual: Option<bool>,
    /// Website no/any/yes selection; omission means no.
    pub partnered_content: PartneredContent,
}

/// Rejects explicit null for an optional supplied selector.
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

impl Default for SpellQuery {
    /// Starts with the first website page and no partnered content.
    fn default() -> Self {
        Self {
            name: None,
            page: 1,
            class_ids: vec![],
            levels: vec![],
            school_ids: vec![],
            casting_time_ids: vec![],
            attack_type_ids: vec![],
            save_ids: vec![],
            damage_type_ids: vec![],
            condition_ids: vec![],
            tag_ids: vec![],
            source_ids: vec![],
            source_category_ids: vec![],
            verbal: None,
            somatic: None,
            material: None,
            concentration: None,
            ritual: None,
            partnered_content: PartneredContent::No,
        }
    }
}

impl SpellQuery {
    /// Returns option arrays in website mapping order.
    pub(crate) fn id_values(&self) -> [&[u32]; 11] {
        [
            &self.class_ids,
            &self.levels,
            &self.school_ids,
            &self.casting_time_ids,
            &self.attack_type_ids,
            &self.save_ids,
            &self.damage_type_ids,
            &self.condition_ids,
            &self.tag_ids,
            &self.source_ids,
            &self.source_category_ids,
        ]
    }

    /// Returns optional restrictions in shared mapping order.
    pub(crate) fn bool_values(&self) -> [Option<bool>; 5] {
        [
            self.verbal,
            self.somatic,
            self.material,
            self.concentration,
            self.ritual,
        ]
    }

    /// Checks local sizes, ranges, and uniqueness before any session access.
    ///
    /// # Errors
    ///
    /// Rejects invalid selectors without silently discarding duplicates.
    pub fn validate(&self) -> Result<(), SpellError> {
        let arrays = self.id_values();
        if self.page == 0
            || self
                .name
                .as_deref()
                .is_some_and(|s| s.len() > 256 || !valid_text(s))
            || arrays.iter().map(|a| a.len()).sum::<usize>() > 256
            || arrays.iter().enumerate().any(|(index, ids)| {
                ids.len() > 50
                    || ids.iter().collect::<HashSet<_>>().len() != ids.len()
                    || ids.iter().any(|id| {
                        if index == 1 {
                            *id > 9
                        } else {
                            *id == 0 || (index == 2 && !(3..=10).contains(id))
                        }
                    })
            })
            || self.website_url(WEBSITE)?.query().map_or(0, str::len) > 16 * 1024
        {
            return Err(SpellError::InvalidInput);
        }
        Ok(())
    }

    /// Builds the fixed listing with repeated website controls and explicit page.
    pub(crate) fn website_url(&self, origin: &str) -> Result<reqwest::Url, SpellError> {
        let mut url = reqwest::Url::parse(&format!("{origin}/spells"))
            .map_err(|_| SpellError::InvalidInput)?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("page", &self.page.to_string());
            if let Some(name) = &self.name {
                query.append_pair("filter-search", name);
            }
            for ((_, _, key), ids) in ID_FILTERS.iter().zip(self.id_values()) {
                for id in ids {
                    query.append_pair(key, &id.to_string());
                }
            }
            for ((field, _, key), value) in BOOL_FILTERS.iter().zip(self.bool_values()) {
                if let Some(value) = value {
                    query.append_pair(
                        key,
                        if *field == "concentration" {
                            if value { "1" } else { "2" }
                        } else if value {
                            "t"
                        } else {
                            "f"
                        },
                    );
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

/// Allows only canonical spell details or checked listing query controls.
pub(crate) fn allowed_url(url: &reqwest::Url) -> bool {
    if url.fragment().is_some() {
        return false;
    }
    if let Some(slug) = url.path().strip_prefix("/spells/") {
        return url.query().is_none() && validate_slug(slug).is_ok();
    }
    if url.path() != "/spells" || url.query().map_or(0, str::len) > 16 * 1024 {
        return false;
    }
    let mut fields = serde_json::Map::new();
    for (key, value) in url.query_pairs() {
        if let Some((field, _, _)) = ID_FILTERS.iter().find(|(_, _, k)| *k == key) {
            let Ok(id) = value.parse::<u32>() else {
                return false;
            };
            if id.to_string() != value {
                return false;
            }
            let Some(ids) = fields
                .entry((*field).to_owned())
                .or_insert_with(|| serde_json::json!([]))
                .as_array_mut()
            else {
                return false;
            };
            ids.push(id.into());
        } else {
            let (field, parsed) = if key == "page" {
                let Ok(page) = value.parse::<u32>() else {
                    return false;
                };
                if page.to_string() != value {
                    return false;
                }
                ("page", serde_json::json!(page))
            } else if key == "filter-search" {
                ("name", serde_json::json!(value))
            } else if key == "filter-partnered-content" {
                let choice = match value.as_ref() {
                    "" => "any",
                    "t" => "yes",
                    "f" => "no",
                    _ => return false,
                };
                ("partnered_content", serde_json::json!(choice))
            } else if let Some((field, _, _)) = BOOL_FILTERS.iter().find(|(_, _, k)| *k == key) {
                let parsed = match (*field, value.as_ref()) {
                    ("concentration", "1") => true,
                    ("concentration", "2") => false,
                    ("concentration", _) => return false,
                    (_, "t") => true,
                    (_, "f") => false,
                    _ => return false,
                };
                (*field, serde_json::json!(parsed))
            } else {
                return false;
            };
            if fields.insert(field.to_owned(), parsed).is_some() {
                return false;
            }
        }
    }
    fields.contains_key("page")
        && fields.contains_key("partnered_content")
        && serde_json::from_value::<SpellQuery>(fields.into()).is_ok_and(|q| q.validate().is_ok())
}
