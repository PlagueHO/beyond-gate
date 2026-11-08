//! Shared bounded source/category identities, independent of resource visibility or ownership.

use serde::Deserialize;
use std::collections::HashSet;

/// Public source identities and category references, not content-access evidence.
#[derive(Deserialize)]
pub(crate) struct Configuration {
    /// Every configured source, including those outside the Web book catalog.
    pub(crate) sources: Vec<ConfiguredSource>,
    /// Categories retain their names for the book-specific visibility selection.
    #[serde(rename = "sourceCategories")]
    pub(crate) categories: Vec<SourceCategory>,
}

/// Connects a public source ID with one configured category.
#[derive(Deserialize)]
pub(crate) struct ConfiguredSource {
    /// Positive public source identifier.
    pub(crate) id: u64,
    /// Category identifier validated against the same configuration.
    #[serde(rename = "sourceCategoryId")]
    pub(crate) category: u64,
}

/// Holds only category identity and its source-provided name.
#[derive(Deserialize)]
pub(crate) struct SourceCategory {
    /// Positive category identifier.
    pub(crate) id: u64,
    /// Checked category name, not a permission classification.
    pub(crate) name: String,
}

/// Separates malformed identities, duplicates, and bounds without exposing input values.
pub(crate) enum ConfigurationError {
    /// A required identity, category link, or name is invalid.
    Invalid,
    /// An identity occurs more than once.
    Conflict,
    /// A raw collection exceeds its fixed bound.
    Limit,
}

impl Configuration {
    /// Checks every raw identity before a resource applies its own visibility policy.
    pub(crate) fn validate(&self) -> Result<(), ConfigurationError> {
        if self.sources.len() > 1_000 || self.categories.len() > 1_000 {
            return Err(ConfigurationError::Limit);
        }
        let mut categories = HashSet::new();
        for category in &self.categories {
            if category.id == 0 || !valid_text(&category.name) {
                return Err(ConfigurationError::Invalid);
            }
            if !categories.insert(category.id) {
                return Err(ConfigurationError::Conflict);
            }
        }
        let mut sources = HashSet::new();
        for source in &self.sources {
            if source.id == 0 || !categories.contains(&source.category) {
                return Err(ConfigurationError::Invalid);
            }
            if !sources.insert(source.id) {
                return Err(ConfigurationError::Conflict);
            }
        }
        Ok(())
    }
}

/// Requires nonempty, unpadded metadata text without terminal controls.
pub(crate) fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}
