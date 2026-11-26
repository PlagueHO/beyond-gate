//! Shared strict filter inputs and fixed monster JSON query construction.

use super::{MAX_NAME_BYTES, MonsterError, SERVICE};
use crate::public_config::valid_text;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;

/// Maximum selected IDs in one filter field.
pub const MAX_FILTER_IDS: usize = 50;
/// Maximum selected IDs across all filter fields.
pub const MAX_TOTAL_IDS: usize = 256;
/// Maximum encoded service query bytes.
pub const MAX_QUERY_BYTES: usize = 16 * 1024;
/// Maps an ID field, CLI flag, service key, and its query value.
pub type IdFilter = (
    &'static str,
    &'static str,
    &'static str,
    for<'a> fn(&'a MonsterQuery) -> &'a [u32],
);
/// Maps model fields, CLI flags, and service keys without adapter-specific policy.
pub const ID_FILTERS: [IdFilter; 16] = [
    ("type_ids", "type-id", "monsterTypes", |q| &q.type_ids),
    ("subtype_ids", "subtype-id", "monsterSubtypes", |q| {
        &q.subtype_ids
    }),
    ("environment_ids", "environment-id", "environments", |q| {
        &q.environment_ids
    }),
    ("alignment_ids", "alignment-id", "alignments", |q| {
        &q.alignment_ids
    }),
    ("sense_ids", "sense-id", "senses", |q| &q.sense_ids),
    (
        "save_proficiency_ids",
        "save-proficiency-id",
        "saveProficiencies",
        |q| &q.save_proficiency_ids,
    ),
    (
        "skill_proficiency_ids",
        "skill-proficiency-id",
        "skillProficiencies",
        |q| &q.skill_proficiency_ids,
    ),
    ("resistance_ids", "resistance-id", "resistances", |q| {
        &q.resistance_ids
    }),
    (
        "damage_immunity_ids",
        "damage-immunity-id",
        "damageImmunities",
        |q| &q.damage_immunity_ids,
    ),
    (
        "condition_immunity_ids",
        "condition-immunity-id",
        "conditionImmunities",
        |q| &q.condition_immunity_ids,
    ),
    (
        "vulnerability_ids",
        "vulnerability-id",
        "vulnerabilities",
        |q| &q.vulnerability_ids,
    ),
    ("language_ids", "language-id", "languages", |q| {
        &q.language_ids
    }),
    ("movement_ids", "movement-id", "movementTypes", |q| {
        &q.movement_ids
    }),
    ("tag_ids", "tag-id", "monsterTags", |q| &q.tag_ids),
    (
        "source_category_ids",
        "source-category-id",
        "sourceCategories",
        |q| &q.source_category_ids,
    ),
    ("source_ids", "source-id", "sources", |q| &q.source_ids),
];
/// Maps a numeric field, CLI flag, service key, and its query value.
pub type NumberFilter = (
    &'static str,
    &'static str,
    &'static str,
    fn(&MonsterQuery) -> Option<u32>,
);
/// Maps optional nonnegative range fields to CLI flags and service keys.
pub const NUMBER_FILTERS: [NumberFilter; 4] = [
    ("armor_class_min", "armor-class-min", "armorClassMin", |q| {
        q.armor_class_min
    }),
    ("armor_class_max", "armor-class-max", "armorClassMax", |q| {
        q.armor_class_max
    }),
    ("average_hp_min", "average-hp-min", "averageHpMin", |q| {
        q.average_hp_min
    }),
    ("average_hp_max", "average-hp-max", "averageHpMax", |q| {
        q.average_hp_max
    }),
];
/// Maps a boolean field, CLI flag, service key, and its query value.
pub type BoolFilter = (
    &'static str,
    &'static str,
    &'static str,
    fn(&MonsterQuery) -> Option<bool>,
);
/// Maps optional boolean fields to CLI flags and service keys.
pub const BOOL_FILTERS: [BoolFilter; 2] = [
    ("legendary", "legendary", "isLegendary", |q| q.legendary),
    ("has_lair", "has-lair", "hasLair", |q| q.has_lair),
];
/// Client-verified literal CR values and service IDs, including the missing ID 28.
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

/// Website choices used by other resource queries, not supported by monster search.
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

/// One explicit JSON batch with matching CLI and MCP selectors.
///
/// ID selectors are service filter values, not ownership or content grants.
/// Omitted optional fields are unrestricted; explicit null is not valid input.
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct MonsterQuery {
    /// Literal name search, at most 256 UTF-8 bytes.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String", length(min = 1, max = 256))]
    pub name: Option<String>,
    /// Source offset; continuation is never followed automatically.
    pub skip: u32,
    /// Local batch size from 1 through 100; defaults to 10.
    #[schemars(range(min = 1, max = 100))]
    pub take: u32,
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
    /// Minimum size ID: 2 Tiny, 3 Small, 4 Medium, 5 Large, 6 Huge, 7 Gargantuan.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32", range(min = 2, max = 7))]
    pub size_min_id: Option<u32>,
    /// Maximum size ID, not below the minimum.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32", range(min = 2, max = 7))]
    pub size_max_id: Option<u32>,
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
    /// Optional lair-state filter; omission leaves it unrestricted.
    #[serde(deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub has_lair: Option<bool>,
}

impl Default for MonsterQuery {
    /// Leaves optional filters unset and selects the first ten source records.
    fn default() -> Self {
        Self {
            name: None,
            skip: 0,
            take: 10,
            cr_min: None,
            cr_max: None,
            type_ids: Vec::new(),
            subtype_ids: Vec::new(),
            size_min_id: None,
            size_max_id: None,
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
            has_lair: None,
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
    /// Returns invalid input for text, CR, reversed ranges, IDs, take, or query bounds.
    pub fn validate(&self) -> Result<(), MonsterError> {
        if !(1..=100).contains(&self.take)
            || [self.size_min_id, self.size_max_id]
                .into_iter()
                .flatten()
                .any(|id| !(2..=7).contains(&id))
            || matches!((self.size_min_id, self.size_max_id), (Some(a), Some(b)) if a > b)
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
        let mut count = 0;
        for (_, _, _, values) in ID_FILTERS {
            let ids = values(self);
            count += ids.len();
            if ids.len() > MAX_FILTER_IDS
                || ids.contains(&0)
                || ids.iter().collect::<HashSet<_>>().len() != ids.len()
            {
                return Err(MonsterError::InvalidInput);
            }
        }
        if count > MAX_TOTAL_IDS
            || self.service_url(SERVICE)?.query().map_or(0, str::len) > MAX_QUERY_BYTES
        {
            return Err(MonsterError::InvalidInput);
        }
        Ok(())
    }

    /// Constructs the fixed JSON search path with client-verified query encodings.
    pub(crate) fn service_url(&self, origin: &str) -> Result<reqwest::Url, MonsterError> {
        let mut url = reqwest::Url::parse(&format!("{origin}/v1/Monster"))
            .map_err(|_| MonsterError::InvalidInput)?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("skip", &self.skip.to_string());
            query.append_pair("take", &self.take.to_string());
            query.append_pair("showHomebrew", "f");
            if let Some(name) = &self.name {
                query.append_pair("search", name);
            }
            for (field, value) in [
                ("challengeRatingMin", &self.cr_min),
                ("challengeRatingMax", &self.cr_max),
            ] {
                if let Some(value) = value {
                    query.append_pair(field, &challenge_id(value)?.to_string());
                }
            }
            for (field, value) in [
                ("monsterSizeMin", self.size_min_id),
                ("monsterSizeMax", self.size_max_id),
            ] {
                if let Some(value) = value {
                    query.append_pair(field, &value.to_string());
                }
            }
            for (_, _, field, values) in ID_FILTERS {
                for id in values(self) {
                    query.append_pair(field, &id.to_string());
                }
            }
            for (_, _, field, value) in NUMBER_FILTERS {
                if let Some(value) = value(self) {
                    query.append_pair(field, &value.to_string());
                }
            }
            for (_, _, field, value) in BOOL_FILTERS {
                if let Some(value) = value(self) {
                    query.append_pair(field, if value { "t" } else { "f" });
                }
            }
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
