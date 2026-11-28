//! Deterministic transport presentation without changing shared core records.

use beyond_gate_core::{
    book::{BookOutline, MAX_SECTION_BYTES, SectionChunk},
    character_campaign::{Campaign, Character},
    library::{AccessEvidence, AccessState, Book, BookAccess},
};
use clap::ArgMatches;
use serde_json::{Value, json};

use crate::error::Failure;

mod html;

/// Maximum bytes in a complete text or Markdown record, including framing.
const MAX_RECORD_BYTES: usize = 8 * 1024 * 1024;

/// Selects presentation only after clap has validated the format name.
pub(crate) struct ListOptions {
    format: String,
    headers: bool,
    table_byte_limit: Option<usize>,
}

impl ListOptions {
    /// Rejects incompatible header and HTML controls before source operations.
    pub(crate) fn parse(matches: &ArgMatches) -> Result<Self, Failure> {
        let format = matches
            .get_one::<String>("output")
            .ok_or(Failure::Usage("A list format is required."))?;
        let headers = !matches.get_flag("no-headers");
        if !matches!(format.as_str(), "table" | "tsv") && !headers {
            if format == "json" {
                return Err(Failure::Usage(
                    "--no-headers cannot be used with --output json",
                ));
            }
            return Err(Failure::Usage(
                "--no-headers can only be used with --output table or --output tsv",
            ));
        }
        if matches.ids().any(|id| id == "html-field")
            && matches.get_one::<String>("html-field").is_some()
            && !matches!(format.as_str(), "html" | "markdown")
        {
            return Err(Failure::Usage(
                "--html-field can only be used with --output html or --output markdown",
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

    /// Selects a supported additional page only for explicit HTML or Markdown.
    pub(crate) fn uses_html_source(&self) -> bool {
        matches!(self.format.as_str(), "html" | "markdown")
    }

    /// Selects human-readable table columns without changing JSON or TSV output.
    pub(crate) fn is_table(&self) -> bool {
        self.format == "table"
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
    if options.format == "table" && options.headers {
        let underlines = widths.iter().map(|width| "-".repeat(*width)).collect();
        cells.insert(1, underlines);
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

/// Presents one complete record without changing JSON or existing table/TSV projections.
pub(crate) fn record(
    options: &ListOptions,
    fields: &[&str],
    value: &Value,
) -> Result<Vec<u8>, Failure> {
    if options.format == "html" {
        let content = value["raw_html"].as_str().ok_or(Failure::InvalidHtml)?;
        let mut bytes = content.as_bytes().to_vec();
        if !bytes.ends_with(b"\n") {
            bytes.push(b'\n');
        }
        return Ok(bytes);
    }
    if matches!(options.format.as_str(), "text" | "markdown") {
        let markdown = options.format == "markdown";
        let mut result = String::new();
        render_value(value, 0, markdown, &mut result)?;
        return Ok(result.into_bytes());
    }
    list(options, fields, std::slice::from_ref(value), Some(value))
}

/// Presents one selected subtree separately without changing the original record.
pub(crate) fn selected_html(
    options: &ListOptions,
    value: &beyond_gate_core::record_html::ReadResult,
) -> Result<Vec<u8>, Failure> {
    let Some(selected) = &value.selected else {
        return record(options, &[], &value.record);
    };
    if options.format == "html" {
        let mut bytes = selected.raw_html.as_bytes().to_vec();
        if !bytes.ends_with(b"\n") {
            bytes.push(b'\n');
        }
        return Ok(bytes);
    }
    let mut result = String::new();
    render_value(&value.record, 0, true, &mut result)?;
    render_field(
        "HTML source field",
        &json!(selected.field),
        0,
        true,
        &mut result,
    )?;
    if let Some(url) = &selected.source_url {
        render_field("HTML source URL", &json!(url), 0, true, &mut result)?;
    }
    append(&mut result, "\n## Selected content\n\n")?;
    append(&mut result, &html::convert(&selected.raw_html, true)?)?;
    Ok(result.into_bytes())
}

/// Walks all structured fields with bounded indentation and explicit empty values.
fn render_value(
    value: &Value,
    depth: usize,
    markdown: bool,
    result: &mut String,
) -> Result<(), Failure> {
    if depth > 128 {
        return Err(Failure::RecordPresentationLimit);
    }
    match value {
        Value::Object(fields) if !fields.is_empty() => {
            for (key, value) in fields {
                render_field(&label(key), value, depth, markdown, result)?;
            }
        }
        Value::Array(values) if !values.is_empty() => {
            for (index, value) in values.iter().enumerate() {
                render_field(
                    &format!("Item {}", index + 1),
                    value,
                    depth,
                    markdown,
                    result,
                )?;
            }
        }
        _ => {
            append(result, &"  ".repeat(depth))?;
            append(result, &scalar(value, markdown)?)?;
            append(result, "\n")?;
        }
    }
    Ok(())
}

/// Writes a readable label and retains every nested field rather than a summary.
fn render_field(
    name: &str,
    value: &Value,
    depth: usize,
    markdown: bool,
    result: &mut String,
) -> Result<(), Failure> {
    append(result, &"  ".repeat(depth))?;
    if markdown {
        append(result, "- **")?;
        append(result, &markdown_text(name))?;
        append(result, ":**")?;
    } else {
        append(result, name)?;
        append(result, ":")?;
    }
    let nested = value.as_object().is_some_and(|fields| !fields.is_empty())
        || value.as_array().is_some_and(|values| !values.is_empty());
    if nested {
        append(result, "\n")?;
        render_value(value, depth + 1, markdown, result)
    } else if (name == "Description HTML"
        && depth > 0
        && value.as_str().is_some_and(|content| !content.is_empty()))
        || (name == "Description"
            && depth > 0
            && value.as_str().is_some_and(|content| content.contains('<')))
    {
        let content = value.as_str().ok_or(Failure::InvalidHtml)?;
        let converted = html::convert(content, markdown)?;
        append(result, "\n\n")?;
        for line in converted.lines() {
            if !line.is_empty() {
                append(result, &"  ".repeat(depth + 1))?;
                append(result, line)?;
            }
            append(result, "\n")?;
        }
        append(result, "\n")
    } else {
        append(result, " ")?;
        append(result, &scalar(value, markdown)?)?;
        append(result, "\n")
    }
}

/// Keeps null and empty values explicit while making strings safe for their output format.
fn scalar(value: &Value, markdown: bool) -> Result<String, Failure> {
    let text = match value {
        Value::String(value) => {
            if value.is_empty() {
                return Ok("\"\"".to_owned());
            }
            escape_cell(value)
        }
        value => serde_json::to_string(value).map_err(Failure::Json)?,
    };
    Ok(if markdown { markdown_text(&text) } else { text })
}

/// Uses readable labels without renaming or changing structured output keys.
fn label(key: &str) -> String {
    let mut result = String::new();
    let mut prior_lower = false;
    for character in key.chars() {
        if character == '_' {
            result.push(' ');
        } else {
            if character.is_uppercase() && prior_lower {
                result.push(' ');
            }
            result.extend(character.to_lowercase());
        }
        prior_lower = character.is_lowercase() || character.is_ascii_digit();
    }
    let mut words: Vec<_> = result.split_whitespace().map(str::to_owned).collect();
    for word in &mut words {
        if matches!(word.as_str(), "id" | "ids" | "url" | "urls" | "html") {
            *word = match word.as_str() {
                "ids" => "IDs".into(),
                "urls" => "URLs".into(),
                word => word.to_ascii_uppercase(),
            };
        }
    }
    let result = words.join(" ");
    let mut characters = result.chars();
    match characters.next() {
        Some(first) => {
            escape_cell(&(first.to_uppercase().collect::<String>() + characters.as_str()))
        }
        None => "\"\"".into(),
    }
}

/// Escapes Markdown syntax and HTML delimiters so source strings cannot inject markup.
fn markdown_text(value: &str) -> String {
    let mut result = String::new();
    for character in value.chars() {
        match character {
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '&' => result.push_str("&amp;"),
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.'
            | '!' | '|' => {
                result.push('\\');
                result.push(character);
            }
            value => result.push(value),
        }
    }
    result
}

/// Rejects an over-limit append before expanding the presentation buffer.
fn append(result: &mut String, text: &str) -> Result<(), Failure> {
    if result
        .len()
        .checked_add(text.len())
        .is_none_or(|bytes| bytes > MAX_RECORD_BYTES)
    {
        return Err(Failure::RecordPresentationLimit);
    }
    result.push_str(text);
    Ok(())
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

    /// Returns the caller's typed malformed-page error for every list format and entry shape.
    #[test]
    fn paginated_lists_reject_malformed_entries_in_every_format() {
        let invalid_responses = [
            json!({"pagination": {"page": 1}}),
            json!({"entries": null, "pagination": {"page": 1}}),
            json!({"entries": {"id": 1}, "pagination": {"page": 1}}),
        ];
        for format in ["json", "table", "tsv"] {
            for value in &invalid_responses {
                let error = paginated_list(
                    &options(format, true, 0),
                    &["id", "page"],
                    value,
                    &["page"],
                    Failure::Item(beyond_gate_core::item_catalog::ItemError::InvalidResponse),
                )
                .expect_err("invalid entries");
                assert!(matches!(
                    error,
                    Failure::Item(beyond_gate_core::item_catalog::ItemError::InvalidResponse)
                ));
            }
        }
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
                &options("table", headers, crate::MAX_CATALOG_TABLE_BYTES),
                &fields,
                &rows,
                None,
            )
            .expect_err("padded table must fail before allocation");
            assert!(matches!(
                error,
                Failure::PresentationLimit(crate::MAX_CATALOG_TABLE_BYTES)
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
                format!("id   name         source_ids\n---  -----------  ----------\n{body}")
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
            list(&options("table", true, 18), &["id", "name"], &[], None)
                .expect("exact header bytes"),
            b"id  name\n--  ----\n"
        );
        assert!(matches!(
            list(&options("table", true, 17), &["id", "name"], &[], None),
            Err(Failure::PresentationLimit(17))
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

    /// Keeps nested and empty values while neutralizing terminal and Markdown injection.
    #[test]
    fn readable_records_preserve_nested_values_and_escape_markup() {
        let value = json!({
            "id": "17", "source_url": "https://example.com/test",
            "raw_html": "<aside>Invented &amp; source markup</aside>",
            "raw": {"moreDetailsUrl": "/test", "description_html": "Invented &amp; safe.",
                "empty": {}, "items": [null, [], "<script>\u{1b}[m"],
                "bad\u{1b}[31m": "key"}
        });
        let text =
            String::from_utf8(record(&options("text", true, 0), &["id"], &value).expect("text"))
                .expect("UTF-8");
        assert!(text.contains("ID: 17\n"));
        assert!(text.contains("Source URL: https://example.com/test\n"));
        assert!(text.contains("Raw HTML: <aside>Invented &amp; source markup</aside>\n"));
        assert!(text.contains("  More details URL: /test\n"));
        assert!(text.contains("  Description HTML:\n\n    Invented & safe.\n"));
        assert!(text.contains("  Empty: {}\n"));
        assert!(text.contains("    Item 1: null\n"));
        assert!(text.contains("    Item 2: []\n"));
        assert!(text.contains("\\u{1b}"));
        assert!(!text.contains('\u{1b}'));
        let markdown = String::from_utf8(
            record(&options("markdown", true, 0), &["id"], &value).expect("Markdown"),
        )
        .expect("UTF-8");
        assert!(markdown.contains("- **ID:** 17\n"));
        assert!(markdown.contains("&lt;script&gt;"));
        assert!(!markdown.contains("<script>"));
        let json = record(&options("json", true, 0), &["id"], &value).expect("JSON");
        assert_eq!(
            serde_json::from_slice::<Value>(&json).expect("original record"),
            value
        );
    }

    /// Preserves empty or nonstring extension fields instead of parsing them as HTML.
    #[test]
    fn readable_records_preserve_empty_and_non_string_html_fields() {
        for value in [
            json!(null),
            json!(7),
            json!(false),
            json!([]),
            json!({}),
            json!(""),
        ] {
            let record_value = json!({"raw": {"description_html": value}});
            let text = String::from_utf8(
                record(&options("text", true, 0), &[], &record_value).expect("text"),
            )
            .expect("UTF-8");
            assert!(text.contains(&format!("Description HTML: {value}\n")));
            let markdown = String::from_utf8(
                record(&options("markdown", true, 0), &[], &record_value).expect("Markdown"),
            )
            .expect("UTF-8");
            assert!(markdown.contains("**Description HTML:**"));
        }
    }

    /// Checks exact byte limits and nested depth without permitting partial output.
    #[test]
    fn readable_record_limits_fail_explicitly() {
        let mut text = String::new();
        append(&mut text, &"x".repeat(MAX_RECORD_BYTES)).expect("exact limit");
        assert!(matches!(
            append(&mut text, "x"),
            Err(Failure::RecordPresentationLimit)
        ));
        let mut nested = json!(0);
        for _ in 0..130 {
            nested = json!({"nested": nested});
        }
        assert!(matches!(
            record(&options("markdown", true, 0), &[], &nested),
            Err(Failure::RecordPresentationLimit)
        ));
    }
}
