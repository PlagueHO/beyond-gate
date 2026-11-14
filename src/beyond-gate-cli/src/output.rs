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
    table_byte_limit: Option<usize>,
}

impl ListOptions {
    /// Rejects header control for JSON before the selected operation can run.
    pub(crate) fn parse(matches: &ArgMatches) -> Result<Self, Failure> {
        let format = matches
            .get_one::<String>("output")
            .ok_or(Failure::Usage("A list format is required."))?;
        let headers = !matches.get_flag("no-headers");
        if format == "json" && !headers {
            return Err(Failure::Usage(
                "--no-headers cannot be used with --output json",
            ));
        }
        Ok(Self {
            format: format.clone(),
            headers,
            table_byte_limit: None,
        })
    }

    /// Bounds padded table bytes for this operation without restricting JSON or TSV.
    pub(crate) fn with_table_byte_limit(mut self, bytes: usize) -> Self {
        self.table_byte_limit = Some(bytes);
        self
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
            AccessEvidence::SharedWithAccount { state } => {
                json!({"kind": "shared_with_account", "state": state_name(*state)})
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
    let capacity = if options.format == "table"
        && let Some(limit) = options.table_byte_limit
    {
        table_bytes(&cells, &widths)
            .filter(|bytes| *bytes <= limit)
            .ok_or(Failure::PresentationLimit(limit))?
    } else {
        0
    };
    let mut result = String::with_capacity(capacity);
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

/// Adds selected page fields to table/TSV rows without changing the JSON envelope.
pub(crate) fn paginated_list(
    options: &ListOptions,
    fields: &[&str],
    value: &Value,
    pagination_fields: &[&str],
    invalid_response: Failure,
) -> Result<Vec<u8>, Failure> {
    let mut rows = value["entries"]
        .as_array()
        .cloned()
        .ok_or(invalid_response)?;
    for row in &mut rows {
        for field in pagination_fields {
            row[*field] = value["pagination"][*field].clone();
        }
    }
    list(options, fields, &rows, Some(value))
}

/// Counts escaped UTF-8 bytes, character-based padding, separators, and newlines.
///
/// Cells include headers when selected. The last column has no padding.
/// Returns `None` on arithmetic overflow before any padded output is allocated.
fn table_bytes(cells: &[Vec<String>], widths: &[usize]) -> Option<usize> {
    let mut bytes = 0usize;
    for row in cells {
        for (index, cell) in row.iter().enumerate() {
            bytes = bytes.checked_add(cell.len())?;
            if index + 1 < row.len() {
                let padding = widths[index]
                    .checked_sub(cell.chars().count())?
                    .checked_add(2)?;
                bytes = bytes.checked_add(padding)?;
            }
        }
        bytes = bytes.checked_add(1)?;
    }
    Some(bytes)
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

    /// Returns the caller's typed malformed-page error even when JSON is selected.
    #[test]
    fn paginated_json_rejects_invalid_entries() {
        let error = paginated_list(
            &options("json", true, 0),
            &["id", "page"],
            &json!({"entries": null, "pagination": {"page": 1}}),
            &["page"],
            Failure::Item(beyond_gate_core::item_catalog::ItemError::InvalidResponse),
        )
        .expect_err("invalid entries");
        assert!(matches!(
            error,
            Failure::Item(beyond_gate_core::item_catalog::ItemError::InvalidResponse)
        ));
    }

    /// Selects one format with a testable table-only byte boundary.
    fn options(format: &str, headers: bool, limit: usize) -> ListOptions {
        ListOptions {
            format: format.into(),
            headers,
            table_byte_limit: None,
        }
        .with_table_byte_limit(limit)
    }

    /// Rejects the reported shape before allocating its roughly 600 MB padded table.
    #[test]
    fn table_limit_rejects_pathological_catalog_without_truncating_other_formats() {
        let rows: Vec<_> = (1..=1_000)
            .map(|id| {
                json!({
                    "kind": "class", "id": id.to_string(),
                    "name": if id == 1 { "x".repeat(600_000) } else { "x".into() },
                    "parent_class_id": null, "source_ids": [7]
                })
            })
            .collect();
        let fields = ["id", "name", "parent_class_id", "source_ids"];
        assert!(serde_json::to_vec(&rows).expect("compact metadata").len() < 1024 * 1024);
        for headers in [false, true] {
            let error = list(
                &options("table", headers, crate::MAX_CLASS_TABLE_BYTES),
                &fields,
                &rows,
                None,
            )
            .expect_err("padded table must fail before allocation");
            assert!(matches!(
                error,
                Failure::PresentationLimit(crate::MAX_CLASS_TABLE_BYTES)
            ));
            assert_eq!(error.exit_code(), 1);
            let tsv = list(&options("tsv", headers, 0), &fields, &rows, None)
                .expect("compact TSV remains available");
            let text = String::from_utf8(tsv).expect("UTF-8");
            assert_eq!(text.lines().count(), 1_000 + usize::from(headers));
            assert!(text.contains(&"x".repeat(600_000)));
        }
        let json = list(&options("json", true, 0), &fields, &rows, None)
            .expect("complete JSON remains available");
        assert_eq!(
            serde_json::from_slice::<Vec<Value>>(&json).expect("complete metadata"),
            rows
        );
    }

    /// Counts escaped cells in bytes but pads columns by characters, at exact boundaries.
    #[test]
    fn table_limit_includes_utf8_escapes_headers_separators_and_newlines() {
        let fields = ["id", "name", "source_ids"];
        let rows = [
            json!({"id": "\u{754c}\t", "name": "a\n\u{1b}\\", "source_ids": [7, 8]}),
            json!({"id": "a", "name": "\u{1f3ee}", "source_ids": []}),
        ];
        let body = "\u{754c}\\t  a\\n\\u{1b}\\\\  [7,8]\na    \u{1f3ee}            []\n";
        for headers in [false, true] {
            let expected = if headers {
                format!("id   name         source_ids\n{body}")
            } else {
                body.into()
            };
            for limit in [expected.len(), expected.len() + 1] {
                assert_eq!(
                    list(&options("table", headers, limit), &fields, &rows, None)
                        .expect("table fits byte limit"),
                    expected.as_bytes()
                );
            }
            assert!(matches!(
                list(
                    &options("table", headers, expected.len() - 1),
                    &fields,
                    &rows,
                    None
                ),
                Err(Failure::PresentationLimit(_))
            ));
        }
        let tsv = list(&options("tsv", true, 0), &fields, &rows, None)
            .expect("TSV is not padded or table-limited");
        assert_eq!(
            tsv,
            "id\tname\tsource_ids\n\u{754c}\\t\ta\\n\\u{1b}\\\\\t[7,8]\na\t\u{1f3ee}\t[]\n"
                .as_bytes()
        );
    }

    /// Counts only selected headers for empty catalogs, including the zero-byte case.
    #[test]
    fn table_limit_handles_empty_catalogs() {
        assert!(
            list(&options("table", false, 0), &["id", "name"], &[], None)
                .expect("no output")
                .is_empty()
        );
        assert_eq!(
            list(&options("table", true, 9), &["id", "name"], &[], None)
                .expect("exact header bytes"),
            b"id  name\n"
        );
        assert!(matches!(
            list(&options("table", true, 8), &["id", "name"], &[], None),
            Err(Failure::PresentationLimit(8))
        ));
    }

    /// Rejects padding, newline, and total-size overflow without allocating large cells.
    #[test]
    fn table_byte_count_rejects_arithmetic_overflow() {
        let row = vec![String::new(), String::new()];
        assert_eq!(
            table_bytes(std::slice::from_ref(&row), &[usize::MAX, 0]),
            None
        );
        assert_eq!(
            table_bytes(std::slice::from_ref(&row), &[usize::MAX - 2, 0]),
            None
        );
        assert_eq!(table_bytes(&[row.clone(), row], &[usize::MAX / 2, 0]), None);
    }

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
