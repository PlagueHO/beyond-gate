//! Bounded live library discovery and offline assembly with explicit access evidence.
//!
//! Catalog visibility never grants content access. Snapshots are not persisted;
//! outline and section operations must establish fresh access independently.

mod discovery;
mod error;
mod model;

#[cfg(any(test, feature = "session-fixture"))]
pub mod fixture;

pub use discovery::discover_library;
pub use error::LibraryError;
pub use model::{AccessEvidence, AccessState, Book, BookAccess, BookRecord};

use std::{collections::HashSet, fmt};

use crate::http::HttpError;
pub(crate) use model::valid_text;

/// Maximum raw observations in one supplied page, before duplicate removal.
pub const MAX_ENTRIES_PER_PAGE: usize = 100;
/// Maximum supplied pages in one complete discovery, including empty pages.
pub const MAX_PAGES: usize = 10;
/// Maximum raw observations in one discovery, including duplicates.
pub const MAX_ENTRIES: usize = 1_000;

/// Supplies one normalized page, not an upstream response or a fetch instruction.
///
/// The first page must have no cursor. Each later cursor must equal the preceding
/// `next_cursor`. A missing `next_cursor` explicitly ends discovery. Cursors
/// remain opaque metadata and are never interpreted as URLs.
#[derive(Clone)]
pub struct LibraryPage {
    /// Identifies this page; `None` is required for the initial page.
    pub cursor: Option<String>,
    /// Contains raw observations; duplicates still consume the entry budget.
    pub entries: Vec<BookRecord>,
    /// Identifies the required next page, or explicitly marks completion.
    pub next_cursor: Option<String>,
}

impl fmt::Debug for LibraryPage {
    /// Omits continuation values and all raw record metadata.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LibraryPage")
            .field("entries", &self.entries.len())
            .field("continuation", &self.next_cursor.is_some())
            .finish_non_exhaustive()
    }
}

/// Holds one complete assembly of supplied pages, with no cache or live guarantee.
///
/// A subsequent assembly starts empty. This type does not establish whether the
/// input is fresh or truthful; future book reads must check access independently.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibrarySnapshot {
    /// Contains unique books in their first-seen order.
    books: Vec<Book>,
}

impl LibrarySnapshot {
    /// Returns metadata and observations, including unavailable and unknown books.
    pub fn books(&self) -> &[Book] {
        &self.books
    }
}

/// Validates a complete, explicitly supplied sequence of normalized pages.
///
/// Limits are local policy, not upstream limits. No page is fetched and no prior
/// snapshot is reused. Book identifiers select duplicates; conflicting metadata
/// or same-context evidence fails the entire operation. Raw observations count
/// toward limits before duplicate removal. Even an empty result needs a terminal
/// page. Input iteration stops on the first failure or limit.
///
/// # Examples
///
/// ```
/// use beyond_gate_core::library::{LibraryPage, assemble_library};
///
/// let page = LibraryPage { cursor: None, entries: vec![], next_cursor: None };
/// let snapshot = assemble_library([Ok(page)])?;
/// assert!(snapshot.books().is_empty());
/// # Ok::<(), beyond_gate_core::library::LibraryError>(())
/// ```
///
/// # Errors
///
/// Returns a typed validation, conflict, pagination, limit, or supplied HTTP
/// failure. Never returns partial results after a failure or incomplete input.
pub fn assemble_library(
    pages: impl IntoIterator<Item = Result<LibraryPage, HttpError>>,
) -> Result<LibrarySnapshot, LibraryError> {
    let mut books: Vec<Book> = Vec::new();
    let mut expected_cursor: Option<String> = None;
    let mut seen_cursors = HashSet::new();
    let mut complete = false;
    let mut entries = 0_usize;

    for (index, page) in pages.into_iter().enumerate() {
        if index >= MAX_PAGES {
            return Err(LibraryError::LimitExceeded);
        }
        if complete {
            return Err(LibraryError::InvalidPagination);
        }
        let page = page.map_err(LibraryError::PageRead)?;
        if page.cursor != expected_cursor {
            return Err(LibraryError::InvalidPagination);
        }
        if let Some(next) = &page.next_cursor
            && (!valid_text(next) || !seen_cursors.insert(next.clone()))
        {
            return Err(LibraryError::InvalidPagination);
        }
        if page.entries.len() > MAX_ENTRIES_PER_PAGE || page.entries.len() > MAX_ENTRIES - entries {
            return Err(LibraryError::LimitExceeded);
        }
        entries += page.entries.len();
        for record in page.entries {
            append_record(&mut books, record)?;
        }
        complete = page.next_cursor.is_none();
        expected_cursor = page.next_cursor;
    }
    if !complete {
        return Err(LibraryError::IncompleteDiscovery);
    }
    Ok(LibrarySnapshot { books })
}

/// Shares duplicate and conflict handling between offline and live observations.
fn append_record(books: &mut Vec<Book>, record: BookRecord) -> Result<(), LibraryError> {
    let book = Book::from_record(record)?;
    match books.iter_mut().find(|existing| existing.id() == book.id()) {
        Some(existing) => existing.merge(book),
        None => {
            books.push(book);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
