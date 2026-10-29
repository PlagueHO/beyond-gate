//! Normalized offline book inputs and validated, read-only results.

use std::fmt;

use reqwest::Url;

use super::BookError;

/// Supplies one section's navigation metadata, not an upstream response schema.
#[derive(Clone)]
pub struct OutlineEntryRecord {
    /// Opaque identifier within the selected book.
    pub id: String,
    /// Source-supplied heading; no heading is generated from the identifier.
    pub title: String,
    /// Opaque parent identifier, or `None` for a root entry.
    pub parent_id: Option<String>,
    /// HTTPS attribution location; a section fragment is allowed.
    pub source_url: String,
}

impl fmt::Debug for OutlineEntryRecord {
    /// Omits all source-supplied navigation metadata.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OutlineEntryRecord { metadata: [REDACTED] }")
    }
}

/// Supplies one complete outline for an explicitly selected book.
#[derive(Clone)]
pub struct OutlineRecord {
    /// Identifies the selected book; it must match the request.
    pub book_id: String,
    /// HTTPS book attribution, never an instruction to fetch this URL.
    pub source_url: String,
    /// Navigation entries in source order, including empty parent sections.
    pub entries: Vec<OutlineEntryRecord>,
}

impl fmt::Debug for OutlineRecord {
    /// Omits identifiers, URLs, and the complete outline.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OutlineRecord { metadata: [REDACTED] }")
    }
}

/// Contains validated navigation metadata for one section.
#[derive(Clone, Eq, PartialEq)]
pub struct OutlineEntry {
    /// Preserves the source identifier.
    pub(super) id: String,
    /// Preserves the source title.
    pub(super) title: String,
    /// Preserves the source parent link.
    pub(super) parent_id: Option<String>,
    /// Stores validated attribution, including a section fragment if supplied.
    pub(super) source_url: Url,
}

impl OutlineEntry {
    /// Returns the opaque section identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the supplied section title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the parent section identifier, or `None` for a root.
    pub fn parent_id(&self) -> Option<&str> {
        self.parent_id.as_deref()
    }

    /// Returns the source location without authorizing a request to it.
    pub fn source_url(&self) -> &Url {
        &self.source_url
    }
}

impl fmt::Debug for OutlineEntry {
    /// Keeps navigation metadata out of diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OutlineEntry { metadata: [REDACTED] }")
    }
}

/// Holds a complete validated outline in its original navigation order.
#[derive(Clone, Eq, PartialEq)]
pub struct BookOutline {
    /// Identifies the selected book, not an access grant.
    pub(super) book_id: String,
    /// Stores the book's source attribution.
    pub(super) source_url: Url,
    /// Contains at most the local entry cap.
    pub(super) entries: Vec<OutlineEntry>,
}

impl BookOutline {
    /// Returns the selected book identifier.
    pub fn book_id(&self) -> &str {
        &self.book_id
    }

    /// Returns the book's HTTPS source location.
    pub fn source_url(&self) -> &Url {
        &self.source_url
    }

    /// Returns the complete navigation sequence, with no silent truncation.
    pub fn entries(&self) -> &[OutlineEntry] {
        &self.entries
    }
}

impl fmt::Debug for BookOutline {
    /// Omits identifiers, URLs, and outline titles.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BookOutline { metadata: [REDACTED] }")
    }
}

/// Supplies one complete selected section, not a book or an HTML extraction job.
///
/// The caller must change `revision` whenever text, identity, or attribution
/// changes. No offline operation can verify that the source reports revisions
/// truthfully. Content is untrusted data and is never executed or interpreted.
#[derive(Clone)]
pub struct SectionRecord {
    /// Opaque book identifier, which must match the explicit request.
    pub book_id: String,
    /// Opaque section identifier, which must match the explicit request.
    pub section_id: String,
    /// Source version used to reject continuations after a content change.
    pub revision: String,
    /// HTTPS section attribution, with an optional navigation fragment.
    pub source_url: String,
    /// Original normalized text, including supplied headings and line breaks.
    pub text: String,
}

impl fmt::Debug for SectionRecord {
    /// Prevents source text and revision metadata from entering diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SectionRecord { content: [REDACTED] }")
    }
}

/// Explicitly selects one section and optionally continues a prior result.
///
/// A cursor never chooses the book or section and is not an access grant.
#[derive(Clone, Copy)]
pub struct SectionRequest<'a> {
    /// Selects one book independently of the cursor.
    pub book_id: &'a str,
    /// Selects one section independently of the cursor.
    pub section_id: &'a str,
    /// Continues only this section, or starts at its beginning when absent.
    pub continuation: Option<&'a SectionCursor>,
}

impl fmt::Debug for SectionRequest<'_> {
    /// Omits request identifiers and cursor state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SectionRequest { selection: [REDACTED] }")
    }
}

/// Opaque in-process continuation tied to a section, revision, URL, and position.
///
/// Reads create cursors; [`Self::from_token`] accepts untrusted position claims
/// from another process. Neither a clone nor a decoded token grants access.
/// Every read still needs matching fresh source data and a new access check.
/// This value stores no section text.
#[derive(Clone, Eq, PartialEq)]
pub struct SectionCursor {
    /// Binds continuation to one book.
    pub(super) book_id: String,
    /// Binds continuation to one explicitly selected section.
    pub(super) section_id: String,
    /// Invalidates the position when the caller reports a new revision.
    pub(super) revision: String,
    /// Prevents reuse with a changed attribution location.
    pub(super) source_url: Url,
    /// Identifies the next unread UTF-8 byte; never zero or terminal.
    pub(super) next_byte: usize,
}

impl fmt::Debug for SectionCursor {
    /// Hides the continuation's identifiers, revision, URL, and position.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SectionCursor { state: [REDACTED] }")
    }
}

/// Returns bounded text and attribution for one selected section.
///
/// The text byte limit excludes metadata and any future adapter serialization.
#[derive(Clone, Eq, PartialEq)]
pub struct SectionChunk {
    /// Identifies the selected book.
    pub(super) book_id: String,
    /// Identifies the selected section.
    pub(super) section_id: String,
    /// Attributes this text to its validated source location.
    pub(super) source_url: Url,
    /// Holds only this chunk's original text.
    pub(super) text: String,
    /// Explicitly continues the section, or marks its end.
    pub(super) next_cursor: Option<SectionCursor>,
}

impl SectionChunk {
    /// Returns the selected book identifier.
    pub fn book_id(&self) -> &str {
        &self.book_id
    }

    /// Returns the selected section identifier.
    pub fn section_id(&self) -> &str {
        &self.section_id
    }

    /// Returns attribution for this chunk, including a supplied fragment.
    pub fn source_url(&self) -> &Url {
        &self.source_url
    }

    /// Returns untrusted text as data, without interpreting its contents.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the continuation for an explicit next call, or `None` at the end.
    pub fn next_cursor(&self) -> Option<&SectionCursor> {
        self.next_cursor.as_ref()
    }

    /// Reports the end explicitly, including for an empty section.
    pub fn is_complete(&self) -> bool {
        self.next_cursor.is_none()
    }
}

impl fmt::Debug for SectionChunk {
    /// Keeps returned text, attribution, and cursor values out of diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SectionChunk { content: [REDACTED] }")
    }
}

/// Parses attribution without treating the URL as network configuration.
pub(super) fn source_url(value: &str) -> Result<Url, BookError> {
    if value.trim() != value || value.chars().any(char::is_control) {
        return Err(BookError::InvalidMetadata);
    }
    let url = Url::parse(value).map_err(|_| BookError::InvalidMetadata)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(BookError::InvalidMetadata);
    }
    Ok(url)
}
