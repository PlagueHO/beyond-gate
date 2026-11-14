//! Typed adapter failures with fixed, safe diagnostic presentation.

use std::{error::Error, fmt, io};

use beyond_gate_core::{
    auth::{AuthError, session::SessionError},
    background::BackgroundError,
    book::BookError,
    character_campaign::CharacterCampaignError,
    class_catalog::ClassCatalogError,
    feat::FeatError,
    http::{ErrorKind, HttpError},
    item_catalog::ItemError,
    library::LibraryError,
    monster::MonsterError,
    species::SpeciesError,
    spell::SpellError,
};

/// Retains shared operation errors without copying private response values.
pub(crate) enum Failure {
    BrowserSetup(crate::browser_setup::SetupError),
    #[cfg(windows)]
    Login(crate::login::LoginError),
    Auth(AuthError),
    Session(SessionError),
    Runtime(io::Error),
    Cancelled,
    Signal(io::Error),
    Character(CharacterCampaignError),
    Class(ClassCatalogError),
    Monster(MonsterError),
    Species(SpeciesError),
    Spell(SpellError),
    Background(BackgroundError),
    Feat(FeatError),
    Item(ItemError),
    Library(LibraryError),
    Book(BookError),
    Cursor(BookError),
    Mcp(crate::mcp::McpError),
    Json(serde_json::Error),
    Output(io::Error),
    PresentationLimit(usize),
    Usage(&'static str),
}

impl Failure {
    /// Preserves the existing usage/operation split in process exit codes.
    pub(crate) fn exit_code(&self) -> i32 {
        if matches!(
            self,
            Self::Usage(_)
                | Self::Cursor(_)
                | Self::Class(ClassCatalogError::InvalidInput)
                | Self::Monster(MonsterError::InvalidInput)
                | Self::Species(SpeciesError::InvalidInput)
                | Self::Spell(SpellError::InvalidInput)
                | Self::Background(BackgroundError::InvalidInput)
                | Self::Feat(FeatError::InvalidInput)
                | Self::Item(ItemError::InvalidInput)
                | Self::Item(ItemError::InvalidMagicInput)
        ) {
            2
        } else {
            1
        }
    }

    /// Finds only the typed, redacted HTTP cause for actionable diagnostics.
    fn http(&self) -> Option<&HttpError> {
        match self {
            Self::Character(CharacterCampaignError::PageRead(error))
            | Self::Library(LibraryError::PageRead(error))
            | Self::Book(BookError::SourceRead(error)) => Some(error),
            _ => None,
        }
    }
}

impl fmt::Display for Failure {
    /// Never prints raw JSON, input tokens, or arbitrary transport source chains.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BrowserSetup(error) => write!(formatter, "{error}")?,
            #[cfg(windows)]
            Self::Login(error) => write!(formatter, "{error}")?,
            Self::Auth(error) => write!(formatter, "{error}")?,
            Self::Session(error) => write!(formatter, "{error}")?,
            Self::Runtime(_) => formatter.write_str("Session runtime could not start.")?,
            Self::Cancelled => formatter.write_str("cancelled")?,
            Self::Signal(_) => formatter.write_str("Session cancellation handler failed.")?,
            Self::Character(error) => write!(formatter, "{error}")?,
            Self::Class(error) => write!(formatter, "{error}")?,
            Self::Monster(error) => write!(formatter, "{error}")?,
            Self::Species(error) => write!(formatter, "{error}")?,
            Self::Spell(error) => write!(formatter, "{error}")?,
            Self::Background(error) => write!(formatter, "{error}")?,
            Self::Feat(error) => write!(formatter, "{error}")?,
            Self::Item(error) => write!(formatter, "{error}")?,
            Self::Library(error) => write!(formatter, "{error}")?,
            Self::Book(error) | Self::Cursor(error) => write!(formatter, "{error}")?,
            Self::Mcp(error) => write!(formatter, "{error}")?,
            Self::Json(_) => formatter.write_str("Result JSON could not be encoded.")?,
            Self::Output(_) => formatter.write_str("Result output could not be written.")?,
            Self::PresentationLimit(bytes) => write!(
                formatter,
                "Table presentation exceeds the {bytes}-byte limit; use --output json or --output tsv."
            )?,
            Self::Usage(message) => formatter.write_str(message)?,
        }
        if let Some(http) = self.http() {
            write!(formatter, " {http}")?;
            formatter.write_str(login_hint(http.kind()))?;
        }
        Ok(())
    }
}

/// Adds login guidance only when the typed cause rejects a session, not on denial.
fn login_hint(kind: ErrorKind) -> &'static str {
    if kind == ErrorKind::SessionUnusable {
        "; browser login is not implemented; do not supply a password, cookie, or token"
    } else {
        ""
    }
}

impl fmt::Debug for Failure {
    /// Keeps debug diagnostics as restricted as ordinary diagnostic text.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for Failure {
    /// Preserves typed causes for callers without flattening their classifications.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BrowserSetup(error) => Some(error),
            #[cfg(windows)]
            Self::Login(error) => Some(error),
            Self::Auth(error) => Some(error),
            Self::Session(error) => Some(error),
            Self::Runtime(error) => Some(error),
            Self::Signal(error) => Some(error),
            Self::Cancelled => None,
            Self::Character(error) => Some(error),
            Self::Class(error) => Some(error),
            Self::Monster(error) => Some(error),
            Self::Species(error) => Some(error),
            Self::Spell(error) => Some(error),
            Self::Background(error) => Some(error),
            Self::Feat(error) => Some(error),
            Self::Item(error) => Some(error),
            Self::Library(error) => Some(error),
            Self::Book(error) | Self::Cursor(error) => Some(error),
            Self::Mcp(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::PresentationLimit(_) => None,
            Self::Usage(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Does not mistake denied access, malformed data, or network errors for expiry.
    #[test]
    fn only_a_rejected_session_needs_human_login() {
        assert!(
            login_hint(ErrorKind::SessionUnusable).contains("browser login is not implemented")
        );
        assert!(!login_hint(ErrorKind::SessionUnusable).contains("expired"));
        for kind in [
            ErrorKind::InvalidInput,
            ErrorKind::UnapprovedOrigin,
            ErrorKind::ClientSetup,
            ErrorKind::Network,
            ErrorKind::Timeout,
            ErrorKind::RateLimited,
            ErrorKind::AccessDenied,
            ErrorKind::Redirect,
            ErrorKind::BodyTooLarge,
            ErrorKind::MalformedResponse,
            ErrorKind::UpstreamStatus,
        ] {
            assert!(login_hint(kind).is_empty());
        }
    }

    /// Keeps shared causes typed and error displays distinct without source data.
    #[test]
    fn operation_failures_preserve_typed_causes() {
        for error in [
            BookError::AccessDenied,
            BookError::AccessUnknown,
            BookError::Unavailable,
            BookError::UnsupportedSection,
            BookError::StaleContinuation,
            BookError::InvalidContinuation,
            BookError::InvalidMetadata,
            BookError::UnsupportedOperation,
        ] {
            let failure = Failure::Book(error);
            assert_eq!(failure.exit_code(), 1);
            assert!(failure.source().expect("typed cause").is::<BookError>());
            assert!(!failure.to_string().is_empty());
        }
        let failure = Failure::Character(CharacterCampaignError::InvalidMetadata);
        assert!(
            failure
                .source()
                .expect("typed cause")
                .is::<CharacterCampaignError>()
        );
        let failure = Failure::Library(LibraryError::IncompleteDiscovery);
        assert!(failure.source().expect("typed cause").is::<LibraryError>());
        assert_eq!(
            Failure::Cursor(BookError::InvalidContinuation).exit_code(),
            2
        );
    }
}
