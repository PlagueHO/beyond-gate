//! Shared strict filter inputs and fixed website query construction.

use super::{MAX_NAME_BYTES, MonsterError, WEBSITE};
use crate::public_config::valid_text;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;

/// Maximum selected IDs in one filter field.
pub const MAX_FILTER_IDS: usize = 50;
/// Maximum selected IDs across all filter fields.
pub const MAX_TOTAL_IDS: usize = 256;
/// Maximum encoded website query bytes.
pub const MAX_QUERY_BYTES: usize = 16 * 1024;
/// Maps model fields, CLI flags, and website controls without adapter-specific policy.
pub const ID_FILTERS: [(&str, &str, &str); 17] = [
    ("type_ids", "type-id", "filter-type"),
    ("subtype_ids", "subtype-id", "filter-sub-types"),
    ("size_ids", "size-id", "filter-size"),
    ("environment_ids", "environment-id", "filter-environment"),
    ("alignment_ids", "alignment-id", "filter-alignment"),
    ("sense_ids", "sense-id", "filter-senses"),
    (
        "save_proficiency_ids",
        "save-proficiency-id",
        "filter-save-proficiency",
    ),
    (
        "skill_proficiency_ids",
        "skill-proficiency-id",
        "filter-skill-proficiency",
    ),
    ("resistance_ids", "resistance-id", "filter-resistance"),
    (
        "damage_immunity_ids",
        "damage-immunity-id",
        "filter-damage-immunity",
    ),
    (
        "condition_immunity_ids",
        "condition-immunity-id",
        "filter-condition-immunity",
    ),
    (
        "vulnerability_ids",
        "vulnerability-id",
        "filter-vulnerability",
    ),
    ("language_ids", "language-id", "filter-language"),
    ("movement_ids", "movement-id", "filter-movement"),
    ("tag_ids", "tag-id", "filter-tags"),
    (
        "source_category_ids",
        "source-category-id",
        "filter-source-category",
    ),
    ("source_ids", "source-id", "filter-source"),
];
/// Maps optional nonnegative range fields to CLI flags and website controls.
pub const NUMBER_FILTERS: [(&str, &str, &str); 4] = [
    (
        "armor_class_min",
        "armor-class-min",
        "filter-armor-class-min",
    ),
    (
        "armor_class_max",
        "armor-class-max",
        "filter-armor-class-max",
    ),
    ("average_hp_min", "average-hp-min", "filter-average-hp-min"),
    ("average_hp_max", "average-hp-max", "filter-average-hp-max"),
];
/// Maps optional boolean fields to CLI flags and website controls.
pub const BOOL_FILTERS: [(&str, &str, &str); 3] = [
    ("legendary", "legendary", "filter-is-legendary"),
    ("mythic", "mythic", "filter-is-mythic"),
    ("has_lair", "has-lair", "filter-has-lair"),
];
/// Observed literal CR values and website IDs, including the missing ID 28.
pub const CHALLENGE_RATINGS: [(&str, u32); 34] = [
    ("0", 1),
    ("1/8", 2),
    ("1/4", 3),
    ("1/2", 4),
    ("1", 5),
    ("2", 6),
    ("3", 7),
    ("4", 8),
    ("5", 9),
    ("6", 10),
    ("7", 11),
    ("8", 12),
    ("9", 13),
    ("10", 14),
    ("11", 15),
    ("12", 16),
    ("13", 17),
    ("14", 18),
    ("15", 19),
    ("16", 20),
    ("17", 21),
    ("18", 22),
    ("19", 23),
    ("20", 24),
    ("21", 25),
    ("22", 26),
    ("23", 27),
    ("24", 29),
    ("25", 30),
    ("26", 31),
    ("27", 32),
    ("28", 33),
    ("29", 34),
    ("30", 35),
];

/// Represents the website's three partnered-content choices, not an access grant.
#[derive(Clone, Copy, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PartneredContent {
    /// Leaves this website filter unspecified.
    Any,
    /// Selects the website's Yes value.
    Yes,
    /// Uses the website's default No value.
    #[default]
    No,
}

/// One explicit website page with matching CLI and MCP selectors.
///
/// ID selectors are public website option IDs, not ownership or content grants.
/// Omitted optional fields are unrestricted; explicit null is not valid input.
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct MonsterQuery {
    /// Literal name search, at most 256 UTF-8 bytes.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String", length(min = 1, max = 256))]
    pub name: Option<String>,
    /// Positive website page; continuation is never followed automatically.
    #[schemars(range(min = 1))]
    pub page: u32,
    /// Minimum literal CR: 0, 1/8, 1/4, 1/2, or an integer from 1 through 30.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub cr_min: Option<String>,
    /// Maximum literal CR, with the same accepted values as cr_min.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub cr_max: Option<String>,
    /// Monster type option IDs; at most 50 unique positive IDs.
    pub type_ids: Vec<u32>,
    /// Monster subtype option IDs; at most 50 unique positive IDs.
    pub subtype_ids: Vec<u32>,
    /// Size option IDs; for example, 4 Medium and 5 Large.
    pub size_ids: Vec<u32>,
    /// Environment option IDs; at most 50 unique positive IDs.
    pub environment_ids: Vec<u32>,
    /// Alignment option IDs; at most 50 unique positive IDs.
    pub alignment_ids: Vec<u32>,
    /// Sense option IDs: 1 Blindsight, 2 Darkvision, 3 Tremorsense, 4 Truesight.
    pub sense_ids: Vec<u32>,
    /// Saving-throw proficiency option IDs; 1 through 6 select STR through CHA.
    pub save_proficiency_ids: Vec<u32>,
    /// Skill proficiency option IDs; at most 50 unique positive IDs.
    pub skill_proficiency_ids: Vec<u32>,
    /// Resistance option IDs; at most 50 unique positive IDs.
    pub resistance_ids: Vec<u32>,
    /// Damage-immunity option IDs; at most 50 unique positive IDs.
    pub damage_immunity_ids: Vec<u32>,
    /// Condition-immunity option IDs; at most 50 unique positive IDs.
    pub condition_immunity_ids: Vec<u32>,
    /// Vulnerability option IDs; at most 50 unique positive IDs.
    pub vulnerability_ids: Vec<u32>,
    /// Language option IDs; at most 50 unique positive IDs.
    pub language_ids: Vec<u32>,
    /// Movement IDs: 1 Walk, 2 Burrow, 3 Climb, 4 Fly, 5 Swim.
    pub movement_ids: Vec<u32>,
    /// Tag option IDs; at most 50 unique positive IDs.
    pub tag_ids: Vec<u32>,
    /// Source-category option IDs, not content-access evidence.
    pub source_category_ids: Vec<u32>,
    /// Source option IDs, not ownership or content-access evidence.
    pub source_ids: Vec<u32>,
    /// Optional nonnegative minimum armor class.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32")]
    pub armor_class_min: Option<u32>,
    /// Optional maximum armor class, not below the minimum.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32")]
    pub armor_class_max: Option<u32>,
    /// Optional nonnegative minimum average hit points.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32")]
    pub average_hp_min: Option<u32>,
    /// Optional maximum average hit points, not below the minimum.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32")]
    pub average_hp_max: Option<u32>,
    /// Optional legendary-state filter; omission leaves it unrestricted.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub legendary: Option<bool>,
    /// Optional mythic-state filter; omission leaves it unrestricted.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub mythic: Option<bool>,
    /// Optional lair-state filter; omission leaves it unrestricted.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub has_lair: Option<bool>,
    /// Website yes/no/any choice; defaults to no, matching the site.
    pub partnered_content: PartneredContent,
}

impl Default for MonsterQuery {
    /// Leaves optional filters unset and selects website page 1 with partnered content No.
    fn default() -> Self {
        Self {
            name: None,
            page: 1,
            cr_min: None,
            cr_max: None,
            type_ids: Vec::new(),
            subtype_ids: Vec::new(),
            size_ids: Vec::new(),
            environment_ids: Vec::new(),
            alignment_ids: Vec::new(),
            sense_ids: Vec::new(),
            save_proficiency_ids: Vec::new(),
            skill_proficiency_ids: Vec::new(),
            resistance_ids: Vec::new(),
            damage_immunity_ids: Vec::new(),
            condition_immunity_ids: Vec::new(),
            vulnerability_ids: Vec::new(),
            language_ids: Vec::new(),
            movement_ids: Vec::new(),
            tag_ids: Vec::new(),
            source_category_ids: Vec::new(),
            source_ids: Vec::new(),
            armor_class_min: None,
            armor_class_max: None,
            average_hp_min: None,
            average_hp_max: None,
            legendary: None,
            mythic: None,
            has_lair: None,
            partnered_content: PartneredContent::No,
        }
    }
}

/// Deserializes only a present non-null selector; missing fields use the model default.
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

impl MonsterQuery {
    /// Rejects invalid selectors before any session or source access.
    ///
    /// # Errors
    ///
    /// Returns invalid input for text, CR, reversed ranges, IDs, page, or query bounds.
    pub fn validate(&self) -> Result<(), MonsterError> {
        if self.page == 0
            || self
                .name
                .as_deref()
                .is_some_and(|s| s.len() > MAX_NAME_BYTES || !valid_text(s))
            || matches!((self.armor_class_min, self.armor_class_max), (Some(a), Some(b)) if a > b)
            || matches!((self.average_hp_min, self.average_hp_max), (Some(a), Some(b)) if a > b)
        {
            return Err(MonsterError::InvalidInput);
        }
        let min = self.cr_min.as_deref().map(challenge_id).transpose()?;
        let max = self.cr_max.as_deref().map(challenge_id).transpose()?;
        if matches!((min, max), (Some(a), Some(b)) if a > b) {
            return Err(MonsterError::InvalidInput);
        }
        let fields = self.id_values();
        let mut count = 0;
        for ids in fields {
            count += ids.len();
            if ids.len() > MAX_FILTER_IDS
                || ids.contains(&0)
                || ids.iter().collect::<HashSet<_>>().len() != ids.len()
            {
                return Err(MonsterError::InvalidInput);
            }
        }
        if count > MAX_TOTAL_IDS
            || self.website_url(WEBSITE)?.query().map_or(0, str::len) > MAX_QUERY_BYTES
        {
            return Err(MonsterError::InvalidInput);
        }
        Ok(())
    }

    /// Supplies ID arrays in the exact order of ID_FILTERS.
    pub(crate) fn id_values(&self) -> [&[u32]; 17] {
        [
            &self.type_ids,
            &self.subtype_ids,
            &self.size_ids,
            &self.environment_ids,
            &self.alignment_ids,
            &self.sense_ids,
            &self.save_proficiency_ids,
            &self.skill_proficiency_ids,
            &self.resistance_ids,
            &self.damage_immunity_ids,
            &self.condition_immunity_ids,
            &self.vulnerability_ids,
            &self.language_ids,
            &self.movement_ids,
            &self.tag_ids,
            &self.source_category_ids,
            &self.source_ids,
        ]
    }

    /// Constructs only the fixed listing path with observed website filter encodings.
    pub(crate) fn website_url(&self, origin: &str) -> Result<reqwest::Url, MonsterError> {
        let mut url = reqwest::Url::parse(&format!("{origin}/monsters"))
            .map_err(|_| MonsterError::InvalidInput)?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("page", &self.page.to_string());
            if self.type_ids.is_empty() {
                query.append_pair("filter-type", "0");
            }
            if let Some(name) = &self.name {
                query.append_pair("filter-search", name);
            }
            for (field, value) in [
                ("filter-cr-min", &self.cr_min),
                ("filter-cr-max", &self.cr_max),
            ] {
                if let Some(value) = value {
                    query.append_pair(field, &challenge_id(value)?.to_string());
                }
            }
            for ((_, _, field), ids) in ID_FILTERS.iter().zip(self.id_values()) {
                for id in ids {
                    query.append_pair(field, &id.to_string());
                }
            }
            for ((_, _, field), value) in NUMBER_FILTERS.iter().zip([
                self.armor_class_min,
                self.armor_class_max,
                self.average_hp_min,
                self.average_hp_max,
            ]) {
                if let Some(value) = value {
                    query.append_pair(field, &value.to_string());
                }
            }
            for ((_, _, field), value) in
                BOOL_FILTERS
                    .iter()
                    .zip([self.legendary, self.mythic, self.has_lair])
            {
                if let Some(value) = value {
                    query.append_pair(field, if value { "t" } else { "f" });
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

/// Maps exact literal CR selectors without assuming continuous numeric option IDs.
fn challenge_id(value: &str) -> Result<u32, MonsterError> {
    CHALLENGE_RATINGS
        .iter()
        .find(|(label, _)| *label == value)
        .map(|(_, id)| *id)
        .ok_or(MonsterError::InvalidInput)
}

/// Limits cookie transport to the observed listing path and known query controls.
pub(crate) fn allowed_list_url(url: &reqwest::Url) -> bool {
    url.path() == "/monsters"
        && url.fragment().is_none()
        && url
            .query()
            .is_some_and(|query| query.len() <= MAX_QUERY_BYTES)
        && url.query_pairs().all(|(key, value)| {
            if key == "filter-search" {
                return value.len() <= MAX_NAME_BYTES && valid_text(&value);
            }
            if key == "page" {
                return value
                    .parse::<u32>()
                    .is_ok_and(|n| n > 0 && n.to_string() == value);
            }
            if key == "filter-partnered-content"
                || BOOL_FILTERS.iter().any(|(_, _, field)| *field == key)
            {
                return matches!(value.as_ref(), "" | "t" | "f");
            }
            let numeric = matches!(key.as_ref(), "filter-cr-min" | "filter-cr-max")
                || ID_FILTERS
                    .iter()
                    .chain(NUMBER_FILTERS.iter())
                    .any(|(_, _, field)| *field == key);
            numeric && value.parse::<u32>().is_ok_and(|n| n.to_string() == value)
        })
}
