//! Offline book navigation and selected-section reads with fresh supplied access.
//!
//! These normalized inputs are not a verified upstream contract or import format.
//! Each operation calls an access check before its selected source loader. The
//! caller must supply current, truthful evidence and change source revisions when
//! content changes. A library snapshot and a continuation are never access grants.
//! No function reads credentials, fetches URLs, or persists content.
//!
//! # Examples
//!
//! Navigate invented metadata and explicitly continue one selected section.
//! The unconditional access grant below is for synthetic data only.
//!
//! ```
//! use beyond_gate_core::{
//!     book::{
//!         BookError, OutlineEntryRecord, OutlineRecord, SectionRecord,
//!         SectionRequest, assemble_outline, read_section,
//!     },
//!     library::AccessState,
//! };
//!
//! let outline = assemble_outline(
//!     "invented-book",
//!     |_| Ok(AccessState::Granted),
//!     |book| Ok(OutlineRecord {
//!         book_id: book.into(),
//!         source_url: "https://example.com/invented".into(),
//!         entries: vec![OutlineEntryRecord {
//!             id: "opening".into(),
//!             title: "An invented opening".into(),
//!             parent_id: None,
//!             source_url: "https://example.com/invented#opening".into(),
//!         }],
//!     }),
//! )?;
//! let selected = outline.entries().first().ok_or(BookError::Unavailable)?;
//! let source = SectionRecord {
//!     book_id: outline.book_id().into(),
//!     section_id: selected.id().into(),
//!     revision: "synthetic-v1".into(),
//!     source_url: selected.source_url().to_string(),
//!     text: "An invented lantern glows.\n".repeat(200),
//! };
//! let request = SectionRequest {
//!     book_id: outline.book_id(),
//!     section_id: selected.id(),
//!     continuation: None,
//! };
//! let first = read_section(
//!     request, |_, _| Ok(AccessState::Granted), |_, _| Ok(source.clone()),
//! )?;
//! let cursor = first.next_cursor().ok_or(BookError::InvalidContinuation)?;
//! let second = read_section(
//!     SectionRequest { continuation: Some(cursor), ..request },
//!     |_, _| Ok(AccessState::Granted),
//!     |_, _| Ok(source.clone()),
//! )?;
//! assert!(second.is_complete());
//! assert_eq!(format!("{}{}", first.text(), second.text()), source.text);
//! # Ok::<(), BookError>(())
//! ```

mod error;
mod model;

pub use error::BookError;
pub use model::{
    BookOutline, OutlineEntry, OutlineEntryRecord, OutlineRecord, SectionChunk, SectionCursor,
    SectionRecord, SectionRequest,
};

use std::collections::HashMap;

use crate::library::{AccessState, valid_text};
use model::source_url;

/// Maximum raw entries in a complete supplied outline, including duplicates.
pub const MAX_OUTLINE_ENTRIES: usize = 1_000;
/// Maximum UTF-8 text bytes per result, excluding metadata and cursor state.
pub const MAX_SECTION_BYTES: usize = 4_096;

/// Checks current supplied access before loading and validating one outline.
///
/// Callbacks receive the explicit book selection, never a cursor or URL as a
/// fetch instruction. No prior result is consulted. Entries retain source order;
/// parents can appear later in the sequence. Empty complete outlines are valid.
/// The entry cap does not bound caller allocations or metadata bytes.
///
/// # Errors
///
/// Returns access, source, metadata, link, or limit failures without a partial
/// outline. Unknown evidence cannot authorize a load.
pub fn assemble_outline(
    book_id: &str,
    check_access: impl FnOnce(&str) -> Result<AccessState, BookError>,
    load: impl FnOnce(&str) -> Result<OutlineRecord, BookError>,
) -> Result<BookOutline, BookError> {
    if !valid_text(book_id) {
        return Err(BookError::InvalidMetadata);
    }
    require_access(check_access(book_id)?)?;
    let record = load(book_id)?;
    if record.book_id != book_id {
        return Err(BookError::InvalidMetadata);
    }
    if record.entries.len() > MAX_OUTLINE_ENTRIES {
        return Err(BookError::LimitExceeded);
    }
    let source_url = source_url(&record.source_url)?;
    let mut entries = Vec::with_capacity(record.entries.len());
    for entry in record.entries {
        if !valid_text(&entry.id)
            || !valid_text(&entry.title)
            || entry.parent_id.as_deref().is_some_and(|id| !valid_text(id))
        {
            return Err(BookError::InvalidMetadata);
        }
        entries.push(OutlineEntry {
            id: entry.id,
            title: entry.title,
            parent_id: entry.parent_id,
            source_url: model::source_url(&entry.source_url)?,
        });
    }
    validate_links(&entries)?;
    Ok(BookOutline {
        book_id: record.book_id,
        source_url,
        entries,
    })
}

/// Checks fresh supplied access and returns one bounded chunk of one section.
///
/// Both callbacks receive only the explicit book and section selection. Access
/// runs on every call, including continuation, before the loader is invoked.
/// The loader must return only that section and report a new revision after any
/// text change. These offline checks cannot detect a caller's unreported change.
///
/// Text remains untrusted data. No HTML extraction, instruction execution, rule
/// interpretation, or automatic continuation occurs. The original bytes and line
/// breaks are preserved. UTF-8 boundaries need not be word or grapheme boundaries.
/// Callers have already allocated source text; this is an output-text bound only.
///
/// # Errors
///
/// Returns access, unavailable, unsupported-section, source, metadata, invalid
/// continuation, or stale continuation failures without any returned text.
pub fn read_section(
    request: SectionRequest<'_>,
    check_access: impl FnOnce(&str, &str) -> Result<AccessState, BookError>,
    load: impl FnOnce(&str, &str) -> Result<SectionRecord, BookError>,
) -> Result<SectionChunk, BookError> {
    if !valid_text(request.book_id) || !valid_text(request.section_id) {
        return Err(BookError::InvalidMetadata);
    }
    require_access(check_access(request.book_id, request.section_id)?)?;
    if let Some(cursor) = request.continuation
        && (cursor.book_id != request.book_id || cursor.section_id != request.section_id)
    {
        return Err(BookError::InvalidContinuation);
    }
    let record = load(request.book_id, request.section_id)?;
    if record.book_id != request.book_id
        || record.section_id != request.section_id
        || !valid_text(&record.revision)
    {
        return Err(BookError::InvalidMetadata);
    }
    let source_url = source_url(&record.source_url)?;
    let start = continuation_start(request.continuation, &record, &source_url)?;
    let end = chunk_end(&record.text, start);
    let next_cursor = (end < record.text.len()).then(|| SectionCursor {
        book_id: record.book_id.clone(),
        section_id: record.section_id.clone(),
        revision: record.revision.clone(),
        source_url: source_url.clone(),
        next_byte: end,
    });
    Ok(SectionChunk {
        book_id: record.book_id,
        section_id: record.section_id,
        source_url,
        text: record.text[start..end].to_owned(),
        next_cursor,
    })
}

/// Rejects unknown evidence and denial without treating either as a grant.
fn require_access(state: AccessState) -> Result<(), BookError> {
    match state {
        AccessState::Granted => Ok(()),
        AccessState::Unavailable => Err(BookError::AccessDenied),
        AccessState::Unknown => Err(BookError::AccessUnknown),
    }
}

/// Checks parent references iteratively to avoid recursion on untrusted outlines.
fn validate_links(entries: &[OutlineEntry]) -> Result<(), BookError> {
    let mut parents = HashMap::with_capacity(entries.len());
    for entry in entries {
        if parents
            .insert(entry.id.as_str(), entry.parent_id.as_deref())
            .is_some()
        {
            return Err(BookError::InvalidOutline);
        }
    }
    for entry in entries {
        let mut parent = entry.parent_id.as_deref();
        let mut depth = 0;
        while let Some(id) = parent {
            if depth >= entries.len() {
                return Err(BookError::InvalidOutline);
            }
            parent = *parents.get(id).ok_or(BookError::InvalidOutline)?;
            depth += 1;
        }
    }
    Ok(())
}

/// Validates the cursor against fresh source metadata and an interior UTF-8 cut.
fn continuation_start(
    cursor: Option<&SectionCursor>,
    record: &SectionRecord,
    source_url: &reqwest::Url,
) -> Result<usize, BookError> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    if cursor.revision != record.revision || cursor.source_url != *source_url {
        return Err(BookError::StaleContinuation);
    }
    if cursor.next_byte == 0
        || cursor.next_byte >= record.text.len()
        || !record.text.is_char_boundary(cursor.next_byte)
    {
        return Err(BookError::InvalidContinuation);
    }
    Ok(cursor.next_byte)
}

/// Finds a progressing UTF-8 cut, preferring a complete supplied line when split.
fn chunk_end(text: &str, start: usize) -> usize {
    let remaining = text.len() - start;
    if remaining <= MAX_SECTION_BYTES {
        return text.len();
    }
    let mut end = start + MAX_SECTION_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    match text[start..end].rfind('\n') {
        Some(index) => start + index + 1,
        None => end,
    }
}

/// Reports that live book outline reads have no implemented source contract.
///
/// # Errors
///
/// Always returns [`BookError::UnsupportedOperation`] without credentials,
/// network access, or a successful empty-outline fallback.
pub fn get_book_outline(_book_id: &str) -> Result<BookOutline, BookError> {
    Err(BookError::UnsupportedOperation)
}

/// Reports that live book section reads have no implemented source contract.
///
/// # Errors
///
/// Always returns [`BookError::UnsupportedOperation`] without credentials,
/// network access, or a successful empty-text fallback.
pub fn read_book_section(_request: SectionRequest<'_>) -> Result<SectionChunk, BookError> {
    Err(BookError::UnsupportedOperation)
}

#[cfg(test)]
mod tests;
