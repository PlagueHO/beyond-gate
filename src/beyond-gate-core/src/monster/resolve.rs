//! Public lookup labels for authorized detail, separate from unchanged raw JSON.

use super::{MonsterError, valid_text};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// Bounds each public lookup table and each raw reference array.
const MAX_LOOKUP_ENTRIES: usize = 1_000;

/// Relates raw scalar references to output names and official configuration tables.
const SCALARS: [(&str, &str, &str); 4] = [
    ("alignmentId", "alignment", "alignments"),
    ("challengeRatingId", "challenge_rating", "challengeRatings"),
    ("sizeId", "size", "creatureSizes"),
    ("typeId", "type", "monsterTypes"),
];

/// Relates reference arrays to tables; an ID key identifies object-valued entries.
const ARRAYS: [(&str, &str, &str, Option<&str>); 10] = [
    (
        "conditionImmunities",
        "condition_immunities",
        "conditions",
        None,
    ),
    (
        "damageAdjustments",
        "damage_adjustments",
        "damageAdjustments",
        None,
    ),
    ("environments", "environments", "environments", None),
    ("subTypes", "subtypes", "monsterSubTypes", None),
    ("languages", "languages", "languages", Some("languageId")),
    ("movements", "movements", "movements", Some("movementId")),
    ("savingThrows", "saving_throws", "stats", Some("statId")),
    ("senses", "senses", "senses", Some("senseId")),
    ("skills", "skills", "abilitySkills", Some("skillId")),
    ("stats", "stats", "stats", Some("statId")),
];

/// Resolves only known reference paths without interpreting source or account IDs.
///
/// Missing fields stay absent and null fields stay null. Object references keep
/// their source fields, including scores, speeds, notes, and unknown extensions.
pub(super) fn detail(raw: &Value, configuration: &Value) -> Result<Value, MonsterError> {
    if !configuration.is_object() {
        return Err(MonsterError::InvalidConfiguration);
    }
    let mut result = Map::new();
    for (input, output, table) in SCALARS {
        let Some(value) = raw.get(input) else {
            continue;
        };
        let resolved = if value.is_null() {
            Value::Null
        } else {
            let id = value.as_u64().ok_or(MonsterError::InvalidResponse)?;
            let names = lookup(configuration, table)?;
            reference(json!({"id": id}), names.get(&id))
        };
        result.insert(output.to_owned(), resolved);
    }
    for (input, output, table, id_key) in ARRAYS {
        if let Some(value) = raw.get(input) {
            result.insert(
                output.to_owned(),
                array(value, configuration, table, id_key)?,
            );
        }
    }
    Ok(Value::Object(result))
}

/// Preserves reference order and scalar IDs while adding names to nested objects.
fn array(
    value: &Value,
    configuration: &Value,
    table: &str,
    id_key: Option<&str>,
) -> Result<Value, MonsterError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let entries = value.as_array().ok_or(MonsterError::InvalidResponse)?;
    if entries.len() > MAX_LOOKUP_ENTRIES {
        return Err(MonsterError::LimitExceeded);
    }
    if entries.is_empty() {
        return Ok(json!([]));
    }
    let names = lookup(configuration, table)?;
    let resolved = entries
        .iter()
        .map(|entry| {
            let value = match id_key {
                Some(key) => entry.get(key),
                None => Some(entry),
            };
            let id = value
                .and_then(Value::as_u64)
                .ok_or(MonsterError::InvalidResponse)?;
            let projected = if id_key.is_some() {
                entry.clone()
            } else {
                json!({"id": id})
            };
            Ok(reference(projected, names.get(&id)))
        })
        .collect::<Result<Vec<_>, MonsterError>>()?;
    Ok(Value::Array(resolved))
}

/// Marks missing lookup IDs explicitly rather than guessing or dropping entries.
fn reference(mut value: Value, name: Option<&String>) -> Value {
    value["name"] = json!(name);
    value["lookup_status"] = json!(if name.is_some() {
        "resolved"
    } else {
        "unresolved"
    });
    value
}

/// Validates a complete selected table and copies labels, never rules descriptions.
fn lookup(configuration: &Value, table: &str) -> Result<HashMap<u64, String>, MonsterError> {
    let entries = configuration[table]
        .as_array()
        .ok_or(MonsterError::InvalidConfiguration)?;
    if entries.len() > MAX_LOOKUP_ENTRIES {
        return Err(MonsterError::LimitExceeded);
    }
    let mut names = HashMap::with_capacity(entries.len());
    for entry in entries {
        let definition = if table == "conditions" {
            &entry["definition"]
        } else {
            entry
        };
        let id = definition["id"]
            .as_u64()
            .ok_or(MonsterError::InvalidConfiguration)?;
        let name = if table == "challengeRatings" {
            challenge_rating(&definition["value"])?
        } else {
            definition["name"]
                .as_str()
                .filter(|name| valid_text(name))
                .ok_or(MonsterError::InvalidConfiguration)?
                .to_owned()
        };
        if names.insert(id, name).is_some() {
            return Err(MonsterError::InvalidConfiguration);
        }
    }
    Ok(names)
}

/// Formats the configured CR value, including fractions, without assuming ID ordinals.
fn challenge_rating(value: &Value) -> Result<String, MonsterError> {
    let number = value
        .as_number()
        .filter(|number| number.as_f64().is_some_and(|value| value >= 0.0))
        .ok_or(MonsterError::InvalidConfiguration)?;
    let label = match number.as_f64() {
        Some(0.125) => "1/8".to_owned(),
        Some(0.25) => "1/4".to_owned(),
        Some(0.5) => "1/2".to_owned(),
        _ => {
            let label = number.to_string();
            label.strip_suffix(".0").unwrap_or(&label).to_owned()
        }
    };
    Ok(label)
}
