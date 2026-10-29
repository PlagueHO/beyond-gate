//! Deterministic transport presentation without changing shared core records.

use beyond_gate_core::{
    book::{BookOutline, MAX_SECTION_BYTES, SectionChunk},
    character_campaign::{Campaign, Character},
    library::{AccessEvidence, AccessState, Book, BookAccess},
};
use clap::ArgMatches;
use serde_json::{Value, json};

use crate::error::Failure;

/// Selects list presentation only after clap has validated the format name.
pub(crate) struct ListOptions {
    format: String,
    headers: bool,
}

impl ListOptions {
    /// Rejects header control for JSON before the selected operation can run.
    pub(crate) fn parse(matches: &ArgMatches) -> Result<Self, Failure> {
        let format = matches
            .get_one::<String>("format")
            .ok_or(Failure::Usage("A list format is required."))?;
        let headers = !matches.get_flag("no-headers");
        if format == "json" && !headers {
            return Err(Failure::Usage(
                "--no-headers cannot be used with --format json",
            ));
        }
        Ok(Self {
            format: format.clone(),
            headers,
        })
    }
}

/// Preserves optional metadata and the entire structured character value.
pub(crate) fn character(value: &Character) -> Value {
    json!({
        "id": value.id(), "name": value.name(),
        "source_url": value.source_url().map(|url| url.as_str()), "raw": value.raw()
    })
}

/// Preserves campaign associations independently of any character list.
pub(crate) fn campaign(value: &Campaign) -> Value {
    json!({
        "id": value.id(), "name": value.name(),
        "source_url": value.source_url().map(|url| url.as_str()),
        "character_ids": value.character_ids(), "raw": value.raw()
    })
}

/// Maps snapshot observations without treating them as later access grants.
pub(crate) fn book(value: &Book) -> Value {
    let evidence: Vec<_> = value
        .evidence()
        .iter()
        .map(|evidence| match evidence {
            AccessEvidence::CatalogOnly => json!({"kind": "catalog_only"}),
            AccessEvidence::Purchased { state } => {
                json!({"kind": "purchased", "state": state_name(*state)})
            }
            AccessEvidence::Shared { context_id, state } => {
                json!({"kind": "shared", "context_id": context_id, "state": state_name(*state)})
            }
        })
        .collect();
    let access = match value.access() {
        BookAccess::Purchased => "purchased",
        BookAccess::Shared => "shared",
        BookAccess::PurchasedAndShared => "purchased_and_shared",
        BookAccess::Unavailable => "unavailable",
        BookAccess::Unknown => "unknown",
    };
    json!({"id": value.id(), "title": value.title(), "source_url": value.source_url().as_str(),
        "access": access, "evidence": evidence})
}

/// Uses stable names instead of debug formatting for evidence states.
fn state_name(state: AccessState) -> &'static str {
    match state {
        AccessState::Granted => "granted",
        AccessState::Unavailable => "unavailable",
        AccessState::Unknown => "unknown",
    }
}

/// Keeps book attribution outside the complete ordered navigation sequence.
pub(crate) fn outline(value: &BookOutline) -> Value {
    let entries: Vec<_> = value
        .entries()
        .iter()
        .map(|entry| {
            json!({
                "id": entry.id(), "title": entry.title(), "parent_id": entry.parent_id(),
                "source_url": entry.source_url().as_str()
            })
        })
        .collect();
    json!({"book_id": value.book_id(), "source_url": value.source_url().as_str(), "entries": entries})
}

/// Encodes exactly one chunk and its continuation before any output is written.
pub(crate) fn section(value: &SectionChunk) -> Result<Value, Failure> {
    let cursor = value
        .next_cursor()
        .map(|cursor| cursor.to_token())
        .transpose()
        .map_err(Failure::Book)?;
    Ok(json!({
        "book_id": value.book_id(), "section_id": value.section_id(),
        "source_url": value.source_url().as_str(), "text": value.text(),
        "next_cursor": cursor, "is_complete": value.is_complete(),
        "max_text_bytes": MAX_SECTION_BYTES
    }))
}

/// Encodes JSON with a final newline, retaining structured values.
pub(crate) fn json_bytes(value: &Value) -> Result<Vec<u8>, Failure> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(Failure::Json)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Formats selected fields without truncation, color, or terminal-dependent defaults.
pub(crate) fn list(
    options: &ListOptions,
    fields: &[&str],
    rows: &[Value],
    json_value: Option<&Value>,
) -> Result<Vec<u8>, Failure> {
    if options.format == "json" {
        let mut bytes = match json_value {
            Some(value) => serde_json::to_vec_pretty(value),
            None => serde_json::to_vec_pretty(rows),
        }
        .map_err(Failure::Json)?;
        bytes.push(b'\n');
        return Ok(bytes);
    }
    let mut cells = Vec::new();
    if options.headers {
        cells.push(
            fields
                .iter()
                .map(|field| (*field).to_owned())
                .collect::<Vec<_>>(),
        );
    }
    for row in rows {
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            let value = match row.get(*field) {
                None | Some(Value::Null) => String::new(),
                Some(Value::String(value)) => value.clone(),
                Some(value) => serde_json::to_string(value).map_err(Failure::Json)?,
            };
            values.push(escape_cell(&value));
        }
        cells.push(values);
    }
    let mut widths = vec![0; fields.len()];
    for row in &cells {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }
    let mut result = String::new();
    for row in cells {
        for (index, cell) in row.iter().enumerate() {
            result.push_str(cell);
            if index + 1 < row.len() {
                if options.format == "tsv" {
                    result.push('\t');
                } else {
                    result.extend(std::iter::repeat_n(
                        ' ',
                        widths[index] - cell.chars().count() + 2,
                    ));
                }
            }
        }
        result.push('\n');
    }
    Ok(result.into_bytes())
}

/// Makes one physical cell unambiguous and prevents terminal control execution.
fn escape_cell(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            value if value.is_control() => escaped.extend(value.escape_unicode()),
            value => escaped.push(value),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Escapes cell delimiters and ANSI controls without losing their identity.
    #[test]
    fn cells_escape_controls_and_backslashes() {
        assert_eq!(
            escape_cell("a\\b\tc\nd\r\u{1b}[m"),
            "a\\\\b\\tc\\nd\\r\\u{1b}[m"
        );
        assert_eq!(escape_cell("lantern \u{1f3ee}"), "lantern \u{1f3ee}");
    }
}
