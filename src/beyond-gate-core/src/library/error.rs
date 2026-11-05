//! Library failures without private metadata or raw response diagnostics.

use std::{error::Error, fmt};

use crate::auth::session::SessionError;
use crate::http::HttpError;

/// Explains why no complete library snapshot can be returned.
///
/// Variants contain no page metadata. HTTP failures keep their original typed,
/// redacted cause. No failure contains a partial successful library.
#[derive(Debug)]
pub enum LibraryError {
    /// A book identifier, title, or HTTPS source URL is invalid.
    InvalidMetadata,
    /// A sharing-context identifier is invalid.
    InvalidEvidence,
    /// One book identifier has different titles or source URLs.
    ConflictingMetadata,
    /// One purchase or sharing context has different observed states.
    ConflictingEvidence,
    /// Pages are out of order, repeat a cursor, or follow a terminal page.
    InvalidPagination,
    /// No terminal page was supplied, including when no pages were supplied.
    IncompleteDiscovery,
    /// The page, per-page entry, or total raw-entry cap was exceeded.
    LimitExceeded,
    /// A live library source contract and usable login are not implemented.
    UnsupportedDiscovery,
    /// A supplied page failed; the underlying HTTP classification is preserved.
    PageRead(HttpError),
    /// A protected session or authenticated resource read failed.
    SessionRead(SessionError),
    /// The complete operation reached its fixed deadline.
    TimedOut,
    /// Ownership names a source absent from validated public configuration.
    UnmatchedOwnership,
}

impl fmt::Display for LibraryError {
    /// Gives a fixed diagnostic without displaying untrusted page values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::SessionRead(error) = self {
            return write!(formatter, "Library discovery failed: {error}");
        }
        let message = match self {
            Self::InvalidMetadata => "Library metadata is invalid.",
            Self::InvalidEvidence => "Library access evidence is invalid.",
            Self::ConflictingMetadata => "Library records contain conflicting book metadata.",
            Self::ConflictingEvidence => "Library records contain conflicting access evidence.",
            Self::InvalidPagination => "Library pages are out of order or repeat a continuation.",
            Self::IncompleteDiscovery => {
                "Library discovery is incomplete; no snapshot is available."
            }
            Self::LimitExceeded => "Library discovery exceeds a local page or entry limit.",
            Self::UnsupportedDiscovery => {
                "Live library discovery is unsupported: a source contract and usable login are not implemented."
            }
            Self::PageRead(_) => "A library page read failed; no snapshot is available.",
            Self::SessionRead(_) => "Library discovery failed.",
            Self::TimedOut => "Library discovery reached its operation deadline.",
            Self::UnmatchedOwnership => "Library ownership has no matching source metadata.",
        };
        formatter.write_str(message)
    }
}

impl Error for LibraryError {
    /// Preserves the safe typed HTTP cause without copying private diagnostics.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PageRead(error) => Some(error),
            Self::SessionRead(error) => Some(error),
            _ => None,
        }
    }
}
