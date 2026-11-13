//! Shared classification of raw equipment and magic-item definitions.
//!
//! Classification is not identity validation or content-access evidence.
//! Equipment reads validate one fresh explicit mixed source page without scans.

#[cfg(any(test, feature = "session-fixture"))]
pub mod fixture;
#[cfg(test)]
mod read_tests;
mod reads;

pub use reads::{
    ItemError, ItemQuery, ItemSelection, MAX_PAGE, PAGE_SIZE, list, show, validate_id,
};

use serde_json::Value;
use std::{error::Error, fmt};

/// Separates catalog views using the source definition's explicit magic flag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemKind {
    /// The raw definition explicitly declares that the item is not magical.
    Equipment,
    /// The raw definition explicitly declares that the item is magical.
    MagicItem,
}

/// Rejects missing or malformed classification without retaining source data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClassificationError;

impl fmt::Display for ClassificationError {
    /// Emits a fixed diagnostic without source records or arbitrary field values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("item definition requires an explicit boolean magic field")
    }
}

impl Error for ClassificationError {}

/// Classifies a raw definition without changing it or inferring an access grant.
///
/// Categories, rarity, and derived character inventory flags do not determine
/// the catalog split. Callers must separately validate identity and access.
///
/// # Errors
///
/// Returns [`ClassificationError`] if the definition is not an object with
/// an explicit boolean `magic` field. Missing values never default to equipment.
///
/// # Examples
///
/// ```
/// use beyond_gate_core::item_catalog::{ItemKind, classify};
/// use serde_json::json;
///
/// assert_eq!(classify(&json!({"magic": false})), Ok(ItemKind::Equipment));
/// assert_eq!(classify(&json!({"magic": true})), Ok(ItemKind::MagicItem));
/// assert!(classify(&json!({"rarity": "Common"})).is_err());
/// ```
pub fn classify(definition: &Value) -> Result<ItemKind, ClassificationError> {
    match definition.get("magic").and_then(Value::as_bool) {
        Some(false) => Ok(ItemKind::Equipment),
        Some(true) => Ok(ItemKind::MagicItem),
        None => Err(ClassificationError),
    }
}

#[cfg(test)]
mod tests {
    use super::{ClassificationError, ItemKind, classify};
    use serde_json::json;

    /// Keeps overlapping and future categories independent of the magic flag.
    #[test]
    fn categories_do_not_determine_classification() {
        for category in ["Other Gear", "Weapon", "Armor", "Wondrous item", "Future"] {
            for (magic, expected) in [(false, ItemKind::Equipment), (true, ItemKind::MagicItem)] {
                assert_eq!(
                    classify(&json!({"magic": magic, "filterType": category})),
                    Ok(expected)
                );
            }
        }
    }

    /// Uses raw catalog evidence rather than rarity or derived inventory state.
    #[test]
    fn derived_state_and_rarity_do_not_replace_raw_evidence() {
        assert_eq!(
            classify(&json!({
                "magic": false,
                "rarity": "Legendary",
                "isMagic": true,
                "infusion": {"future": true}
            })),
            Ok(ItemKind::Equipment)
        );
        assert_eq!(
            classify(&json!({"magic": true, "rarity": "Common", "isMagic": false})),
            Ok(ItemKind::MagicItem)
        );
    }

    /// Rejects absent, nested, and wrong-type flags instead of inventing a default.
    #[test]
    fn malformed_classification_fails_explicitly() {
        for definition in [
            json!(null),
            json!(false),
            json!([]),
            json!({}),
            json!({"magic": null}),
            json!({"magic": 0}),
            json!({"magic": 1}),
            json!({"magic": "false"}),
            json!({"magic": []}),
            json!({"magic": {}}),
            json!({"isMagic": false}),
            json!({"definition": {"magic": false}}),
        ] {
            assert_eq!(classify(&definition), Err(ClassificationError));
        }
    }
}
