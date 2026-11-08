//! Fixed, bounded parsers for approved character and campaign source envelopes.

use serde_json::Value;

use super::{CampaignRecord, CharacterCampaignError, CharacterRecord, MAX_ENTRIES_PER_PAGE};

/// Parses the complete default character collection response.
pub(super) fn character_list(value: Value) -> Result<Vec<CharacterRecord>, CharacterCampaignError> {
    let response = object(&value)?;
    if response.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(CharacterCampaignError::InvalidResponse);
    }
    let data = object(
        response
            .get("data")
            .ok_or(CharacterCampaignError::InvalidResponse)?,
    )?;
    let characters = data
        .get("characters")
        .and_then(Value::as_array)
        .ok_or(CharacterCampaignError::InvalidResponse)?;
    let pagination = object(
        response
            .get("pagination")
            .ok_or(CharacterCampaignError::InvalidResponse)?,
    )?;
    let skip = positive_or_zero(pagination.get("skip"))?;
    let take = positive(pagination.get("take"))?;
    let total = positive_or_zero(pagination.get("total"))?;
    if skip != 0
        || total
            != u64::try_from(characters.len()).map_err(|_| CharacterCampaignError::LimitExceeded)?
        || characters.len() > MAX_ENTRIES_PER_PAGE
        || characters.len() > usize::try_from(take).unwrap_or(usize::MAX)
    {
        return Err(CharacterCampaignError::IncompleteDiscovery);
    }
    characters.iter().cloned().map(character_record).collect()
}

/// Parses one selected character response and checks its opaque identity.
pub(super) fn character_show(
    value: Value,
    selected_id: &str,
) -> Result<CharacterRecord, CharacterCampaignError> {
    let response = object(&value)?;
    if response.get("success").and_then(Value::as_bool) != Some(true)
        || !response.get("pagination").is_some_and(Value::is_null)
    {
        return Err(CharacterCampaignError::InvalidResponse);
    }
    let data = response
        .get("data")
        .ok_or(CharacterCampaignError::InvalidResponse)?;
    let record = character_record(data.clone())?;
    if record.id != selected_id {
        return Err(CharacterCampaignError::InvalidResponse);
    }
    Ok(record)
}

/// Parses one summary from a campaign collection.
pub(super) fn campaign_list(value: Value) -> Result<Vec<CampaignRecord>, CharacterCampaignError> {
    let response = object(&value)?;
    if response.get("status").and_then(Value::as_str) != Some("success") {
        return Err(CharacterCampaignError::InvalidResponse);
    }
    let entries = response
        .get("data")
        .and_then(Value::as_array)
        .ok_or(CharacterCampaignError::InvalidResponse)?;
    if entries.len() > MAX_ENTRIES_PER_PAGE {
        return Err(CharacterCampaignError::LimitExceeded);
    }
    entries.iter().map(campaign_summary).collect()
}

/// Parses the active roster without treating its character data as full records.
pub(super) fn campaign_roster(value: Value) -> Result<Vec<Value>, CharacterCampaignError> {
    let response = object(&value)?;
    let entries = response
        .get("data")
        .and_then(Value::as_array)
        .ok_or(CharacterCampaignError::InvalidResponse)?;
    if entries.len() > MAX_ENTRIES_PER_PAGE {
        return Err(CharacterCampaignError::LimitExceeded);
    }
    for entry in entries {
        let fields = object(entry)?;
        positive(fields.get("id"))?;
        if fields.get("name").and_then(Value::as_str).is_none()
            || positive(fields.get("userId")).is_err()
        {
            return Err(CharacterCampaignError::InvalidResponse);
        }
    }
    Ok(entries.clone())
}

/// Returns one positive numeric source identifier as canonical decimal text.
pub(super) fn numeric_source_id(
    value: &Value,
    field: &str,
) -> Result<String, CharacterCampaignError> {
    let id = positive(object(value)?.get(field))?;
    Ok(id.to_string())
}

/// Builds a summary record with associations explicitly marked as not loaded.
pub(super) fn campaign_summary(value: &Value) -> Result<CampaignRecord, CharacterCampaignError> {
    let fields = object(value)?;
    let id = positive(fields.get("id"))?.to_string();
    let name = fields
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| {
            !name.is_empty() && name.trim() == *name && !name.chars().any(char::is_control)
        })
        .ok_or(CharacterCampaignError::InvalidResponse)?
        .to_owned();
    positive_or_zero(fields.get("playerCount"))?;
    Ok(CampaignRecord {
        id,
        name: Some(name),
        source_url: None,
        character_ids: None,
        raw: value.clone(),
    })
}

/// Converts one source character object while preserving every supplied field.
fn character_record(value: Value) -> Result<CharacterRecord, CharacterCampaignError> {
    let fields = object(&value)?;
    let id = positive(fields.get("id"))?.to_string();
    let name = fields
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| {
            !name.is_empty() && name.trim() == *name && !name.chars().any(char::is_control)
        })
        .ok_or(CharacterCampaignError::InvalidResponse)?
        .to_owned();
    Ok(CharacterRecord {
        id,
        name: Some(name),
        source_url: None,
        raw: value,
    })
}

/// Requires a JSON object at a fixed contract boundary.
fn object(value: &Value) -> Result<&serde_json::Map<String, Value>, CharacterCampaignError> {
    value
        .as_object()
        .ok_or(CharacterCampaignError::InvalidResponse)
}

/// Reads a positive numeric source identifier.
fn positive(value: Option<&Value>) -> Result<u64, CharacterCampaignError> {
    value
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or(CharacterCampaignError::InvalidResponse)
}

/// Reads a nonnegative numeric count.
fn positive_or_zero(value: Option<&Value>) -> Result<u64, CharacterCampaignError> {
    value
        .and_then(Value::as_u64)
        .ok_or(CharacterCampaignError::InvalidResponse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Accepts only a complete bounded character list and keeps unknown fields.
    #[test]
    fn character_list_requires_complete_default_snapshot() {
        let response = json!({
            "success": true,
            "data": {"characters": [{"id": 17, "name": "Invented", "unknown": {"v": 1}}]},
            "pagination": {"skip": 0, "take": 100, "total": 1}
        });
        let records = character_list(response).expect("complete source list");
        assert_eq!(records[0].id, "17");
        assert_eq!(records[0].raw["unknown"]["v"], 1);

        for pagination in [
            json!({"skip": 1, "take": 100, "total": 1}),
            json!({"skip": 0, "take": 100, "total": 2}),
            json!({"skip": 0, "take": "100", "total": 1}),
        ] {
            assert!(matches!(
                character_list(json!({
                    "success": true,
                    "data": {"characters": [{"id": 17, "name": "Invented"}]},
                    "pagination": pagination
                })),
                Err(CharacterCampaignError::IncompleteDiscovery
                    | CharacterCampaignError::InvalidResponse)
            ));
        }
    }

    /// Rejects a selected character response that names another record.
    #[test]
    fn character_show_checks_selection_and_null_pagination() {
        let valid =
            json!({"success": true, "data": {"id": 17, "name": "Invented"}, "pagination": null});
        assert_eq!(
            character_show(valid.clone(), "17")
                .expect("selected record")
                .id,
            "17"
        );
        assert!(matches!(
            character_show(valid, "18"),
            Err(CharacterCampaignError::InvalidResponse)
        ));
    }

    /// Requires the fixed campaign discriminator and marks summaries unloaded.
    #[test]
    fn campaign_summaries_keep_unknown_fields_and_unloaded_rosters() {
        let records = campaign_list(json!({
            "status": "success",
            "data": [{"id": 8, "name": "Invented", "playerCount": 2, "unknown": true}]
        }))
        .expect("campaign summaries");
        assert_eq!(records[0].id, "8");
        assert!(records[0].character_ids.is_none());
        assert_eq!(records[0].raw["unknown"], true);
        assert!(matches!(
            campaign_list(json!({"status": "ok", "data": []})),
            Err(CharacterCampaignError::InvalidResponse)
        ));
    }

    /// Parses only bounded roster records and preserves their complete values.
    #[test]
    fn campaign_roster_preserves_unknown_fields() {
        let response =
            json!({"data": [{"id": 42, "name": "Invented", "userId": 3, "unknown": true}]});
        let roster = campaign_roster(response).expect("valid selected roster");
        assert_eq!(roster[0]["unknown"], true);
    }
}
