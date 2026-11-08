//! Checked metadata projection that cannot serialize a raw class definition.

use super::{ClassCatalogError as Failure, ClassKind, MAX_RECORDS, WEBSITE};
use crate::public_config::valid_text;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};

/// Compact identity and attribution shared by lists and selected metadata.
#[derive(Serialize)]
pub(super) struct Summary {
    /// Selects the definition catalog, not a content-access state.
    kind: ClassKind,
    /// Canonical decimal source ID.
    pub(super) id: String,
    /// Checked source name; duplicate names remain distinct.
    pub(super) name: String,
    /// Canonical parent ID or null for a base class.
    parent_class_id: Option<String>,
    /// Distinct, sorted, configured source identities.
    source_ids: BTreeSet<u64>,
    /// Keeps numeric ordering without parsing serialized identifiers.
    #[serde(skip)]
    pub(super) numeric_id: u64,
    /// Keeps parent context separate from the definition ID.
    #[serde(skip)]
    pub(super) numeric_parent: Option<u64>,
}

/// Selected metadata; unknown definition fields are never stored in this output type.
#[derive(Serialize)]
pub(super) struct Metadata {
    /// Uses the same identity fields and names as list results.
    #[serde(flatten)]
    pub(super) summary: Summary,
    /// Upstream boolean, never a content grant.
    is_homebrew: bool,
    /// Optional checked source slug.
    slug: Option<String>,
    /// Optional validated display link, never used for a request.
    more_details_url: Option<String>,
    /// Preserves source-reference metadata extensions, not raw definitions.
    sources: Vec<Value>,
}

/// Requires a complete terminal envelope and validates every raw record before filtering.
pub(super) fn collection(
    value: Value,
    kind: ClassKind,
    parent: Option<u64>,
    sources: &HashSet<u64>,
    count: &mut usize,
) -> Result<Vec<Metadata>, Failure> {
    let envelope = value.as_object().ok_or(Failure::InvalidMetadata)?;
    if !envelope.get("id").is_some_and(Value::is_number)
        || envelope.get("success").and_then(Value::as_bool) != Some(true)
        || envelope.get("message").and_then(Value::as_str).is_none()
    {
        return Err(Failure::InvalidMetadata);
    }
    if !envelope.get("pagination").is_some_and(Value::is_null)
        || envelope.keys().any(|key| {
            !matches!(
                key.as_str(),
                "id" | "success" | "message" | "data" | "pagination" | "totalCount"
            )
        })
    {
        return Err(Failure::IncompleteDiscovery);
    }
    let data = envelope
        .get("data")
        .and_then(Value::as_array)
        .ok_or(Failure::InvalidMetadata)?;
    if let Some(total) = envelope.get("totalCount")
        && total.as_u64() != Some(data.len() as u64)
    {
        return Err(Failure::IncompleteDiscovery);
    }
    if data.len() > MAX_RECORDS - *count {
        return Err(Failure::LimitExceeded);
    }
    *count += data.len();
    let mut ids = HashSet::new();
    data.iter()
        .map(|value| {
            let record = metadata(value, kind, parent, sources)?;
            if !ids.insert(record.summary.numeric_id) {
                return Err(Failure::ConflictingMetadata);
            }
            Ok(record)
        })
        .collect()
}

/// Projects only approved fields after source identities and parent relations pass.
fn metadata(
    value: &Value,
    kind: ClassKind,
    parent: Option<u64>,
    configured: &HashSet<u64>,
) -> Result<Metadata, Failure> {
    let fields = value.as_object().ok_or(Failure::InvalidMetadata)?;
    let id = positive(fields.get("id"))?;
    let name = fields
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| valid_text(name))
        .ok_or(Failure::InvalidMetadata)?
        .to_owned();
    let parent_value = fields
        .get("parentClassId")
        .ok_or(Failure::InvalidMetadata)?;
    match kind {
        ClassKind::Class if !parent_value.is_null() || parent.is_some() => {
            return Err(Failure::InvalidMetadata);
        }
        ClassKind::Subclass if Some(positive(Some(parent_value))?) != parent => {
            return Err(Failure::InvalidMetadata);
        }
        _ => {}
    }
    let references = fields
        .get("sources")
        .and_then(Value::as_array)
        .ok_or(Failure::InvalidMetadata)?;
    if references.len() > MAX_RECORDS {
        return Err(Failure::LimitExceeded);
    }
    let mut source_ids = BTreeSet::new();
    for source in references {
        let source = source.as_object().ok_or(Failure::InvalidMetadata)?;
        let id = positive(source.get("sourceId"))?;
        if !configured.contains(&id) {
            return Err(Failure::InvalidMetadata);
        }
        source_ids.insert(id);
    }
    let slug = optional_text(fields.get("slug"))?;
    let more_details_url = optional_text(fields.get("moreDetailsUrl"))?
        .map(|value| link(&value))
        .transpose()?;
    Ok(Metadata {
        summary: Summary {
            kind,
            id: id.to_string(),
            name,
            parent_class_id: parent.map(|id| id.to_string()),
            source_ids,
            numeric_id: id,
            numeric_parent: parent,
        },
        is_homebrew: fields
            .get("isHomebrew")
            .and_then(Value::as_bool)
            .ok_or(Failure::InvalidMetadata)?,
        slug,
        more_details_url,
        sources: references.clone(),
    })
}

/// Rejects floats, strings, zero, and negative source IDs.
fn positive(value: Option<&Value>) -> Result<u64, Failure> {
    value
        .and_then(Value::as_u64)
        .filter(|id| *id > 0)
        .ok_or(Failure::InvalidMetadata)
}

/// Represents absent metadata explicitly while rejecting malformed supplied text.
fn optional_text(value: Option<&Value>) -> Result<Option<String>, Failure> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if valid_text(text) => Ok(Some(text.clone())),
        _ => Err(Failure::InvalidMetadata),
    }
}

/// Checks HTTPS website metadata links while retaining valid section fragments.
fn link(value: &str) -> Result<String, Failure> {
    let base = reqwest::Url::parse(WEBSITE).map_err(|_| Failure::InvalidMetadata)?;
    let url = if value.starts_with('/') && !value.starts_with("//") {
        base.join(value)
    } else {
        reqwest::Url::parse(value)
    }
    .map_err(|_| Failure::InvalidMetadata)?;
    if url.origin() != base.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
    {
        return Err(Failure::InvalidMetadata);
    }
    Ok(url.to_string())
}
