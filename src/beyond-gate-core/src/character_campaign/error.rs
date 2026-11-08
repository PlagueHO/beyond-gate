//! Typed character and campaign validation failures with safe diagnostics.

use std::{error::Error, fmt};

use crate::{auth::session::SessionError, http::HttpError};

/// Explains why no complete character or campaign result can be returned.
///
/// HTTP page failures retain their typed, redacted cause. A missing identifier
/// in a completed offline snapshot is represented by its lookup returning
/// `None`; it does not establish an upstream missing-resource contract.
#[derive(Debug)]
pub enum CharacterCampaignError {
    /// An identifier, optional name, source URL, or character association is invalid.
    InvalidMetadata,
    /// One identifier has records with different metadata or raw data.
    ConflictingRecord,
    /// Pages are out of order, repeat a cursor, or follow a terminal page.
    InvalidPagination,
    /// No terminal page was supplied, including when no pages were supplied.
    IncompleteDiscovery,
    /// The page, per-page record, or total raw-record cap was exceeded.
    LimitExceeded,
    /// The selected record is absent from a complete authorized source response.
    MissingRecord,
    /// A source response does not match the fixed read contract.
    InvalidResponse,
    /// A protected-session read failed and retains its safe cause.
    SessionRead(SessionError),
    /// A bounded source request failed and retains its safe cause.
    PageRead(HttpError),
    /// The complete multi-request operation reached its fixed deadline.
    TimedOut,
}

impl fmt::Display for CharacterCampaignError {
    /// Gives a safe diagnostic without displaying record or cursor values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidMetadata => "Character or campaign metadata is invalid.",
            Self::ConflictingRecord => {
                "Character or campaign records contain conflicting metadata."
            }
            Self::InvalidPagination => {
                "Character or campaign pages are out of order or repeat a continuation."
            }
            Self::IncompleteDiscovery => {
                "Character or campaign discovery is incomplete; no snapshot is available."
            }
            Self::LimitExceeded => {
                "Character or campaign discovery exceeds a local page or record limit."
            }
            Self::MissingRecord => "The selected character or campaign was not found.",
            Self::InvalidResponse => {
                "The character or campaign source returned an invalid response."
            }
            Self::PageRead(_) => {
                "A character or campaign page read failed; no snapshot is available."
            }
            Self::TimedOut => "The character or campaign operation reached its time limit.",
            Self::SessionRead(error) => {
                return write!(
                    formatter,
                    "The protected session could not read this source: {error}"
                );
            }
        };
        formatter.write_str(message)
    }
}

impl Error for CharacterCampaignError {
    /// Preserves a supplied HTTP classification without exposing raw diagnostics.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PageRead(error) => Some(error),
            Self::SessionRead(error) => Some(error),
            _ => None,
        }
    }
}
