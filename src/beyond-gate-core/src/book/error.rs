//! Book failures with fixed diagnostics and redacted HTTP causes.

use std::{error::Error, fmt};

use crate::http::HttpError;

/// Explains why no outline or selected-section result can be returned.
#[derive(Debug)]
pub enum BookError {
    /// An identifier, title, revision, or HTTPS source location is invalid.
    InvalidMetadata,
    /// Outline identifiers repeat, parents are missing, or links form a cycle.
    InvalidOutline,
    /// Current supplied evidence explicitly denies access.
    AccessDenied,
    /// Current supplied evidence does not establish access.
    AccessUnknown,
    /// The selected content is unavailable in the supplied source.
    Unavailable,
    /// The source cannot represent or read the selected section.
    UnsupportedSection,
    /// A cursor has the wrong scope or an invalid byte position.
    InvalidContinuation,
    /// A cursor refers to a different source revision or location.
    StaleContinuation,
    /// The raw outline entry limit was exceeded.
    LimitExceeded,
    /// Live book reads have no implemented source contract or usable login.
    UnsupportedOperation,
    /// An access check or source load failed with a typed HTTP cause.
    SourceRead(HttpError),
}

impl fmt::Display for BookError {
    /// Gives a fixed message without printing text, metadata, or cursor values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidMetadata => "Book or section metadata is invalid.",
            Self::InvalidOutline => "Book outline links are invalid or identifiers repeat.",
            Self::AccessDenied => "Current book or section access is denied.",
            Self::AccessUnknown => "Current book or section access is not established.",
            Self::Unavailable => "The selected book or section is unavailable.",
            Self::UnsupportedSection => "The source does not support the selected section.",
            Self::InvalidContinuation => "Section continuation scope or position is invalid.",
            Self::StaleContinuation => "Section continuation refers to an older or changed source.",
            Self::LimitExceeded => "Book outline exceeds the local entry limit.",
            Self::UnsupportedOperation => {
                "Live book reads are unsupported: source contracts and usable login are not implemented."
            }
            Self::SourceRead(_) => "A book access check or source read failed.",
        })
    }
}

impl Error for BookError {
    /// Retains only the safe typed HTTP cause, without returning partial content.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceRead(error) => Some(error),
            _ => None,
        }
    }
}
