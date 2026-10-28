//! Normalized book observations, not an upstream response or entitlement schema.

use std::fmt;

use reqwest::Url;

use super::LibraryError;

/// Describes evidence for one purchase or sharing context in one discovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessState {
    /// The supplied evidence explicitly records access in this context.
    Granted,
    /// The supplied evidence explicitly records denial or revocation.
    Unavailable,
    /// The supplied evidence does not establish access or denial.
    Unknown,
}

/// Keeps catalog visibility separate from purchase and sharing evidence.
///
/// Callers must supply actual evidence, not infer a grant from a title or URL.
/// These observations cannot authorize a later book read.
#[derive(Clone, Eq, PartialEq)]
pub enum AccessEvidence {
    /// A catalog observation without any entitlement evidence.
    CatalogOnly,
    /// An observation about the current account's purchase access.
    Purchased {
        /// Records the observed purchase state, not a permanent entitlement.
        state: AccessState,
    },
    /// An observation about one independent sharing context.
    Shared {
        /// Identifies the context opaquely; it is not a verified campaign schema.
        context_id: String,
        /// Records the observed state for this context only.
        state: AccessState,
    },
}

impl AccessEvidence {
    /// Checks that a sharing observation has a meaningful context identifier.
    pub(super) fn validate(&self) -> Result<(), LibraryError> {
        if let Self::Shared { context_id, .. } = self
            && !valid_text(context_id)
        {
            return Err(LibraryError::InvalidEvidence);
        }
        Ok(())
    }

    /// Compares the scope, not the state, so contradictory observations fail.
    pub(super) fn same_context(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::CatalogOnly, Self::CatalogOnly)
            | (Self::Purchased { .. }, Self::Purchased { .. }) => true,
            (
                Self::Shared { context_id, .. },
                Self::Shared {
                    context_id: other_id,
                    ..
                },
            ) => context_id == other_id,
            _ => false,
        }
    }
}

impl fmt::Debug for AccessEvidence {
    /// Omits private sharing identifiers while retaining the safe state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CatalogOnly => formatter.write_str("CatalogOnly"),
            Self::Purchased { state } => formatter
                .debug_struct("Purchased")
                .field("state", state)
                .finish(),
            Self::Shared { state, .. } => formatter
                .debug_struct("Shared")
                .field("context_id", &"[REDACTED]")
                .field("state", state)
                .finish(),
        }
    }
}

/// Summarizes evidence in a snapshot, never permission for a later read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookAccess {
    /// At least one purchase grant was observed, without a sharing grant.
    Purchased,
    /// At least one sharing grant was observed, without a purchase grant.
    Shared,
    /// Both a purchase grant and a sharing grant were observed.
    PurchasedAndShared,
    /// Every supplied observation explicitly records unavailable access.
    Unavailable,
    /// No grant was observed and at least one observation is unknown or catalog-only.
    Unknown,
}

/// Supplies one metadata record and one access observation for offline assembly.
///
/// Multiple records can describe the same book with independent evidence.
/// Public fields are untrusted input; assembly validates them before returning
/// a snapshot. This is not a D&D Beyond wire format or a user-import interface.
#[derive(Clone)]
pub struct BookRecord {
    /// Opaque identifier used for duplicate detection within this discovery.
    pub id: String,
    /// Human-readable metadata, not book content.
    pub title: String,
    /// HTTPS source location, not a runtime origin allowlist.
    pub source_url: String,
    /// One explicit observation; catalog visibility alone cannot grant access.
    pub evidence: AccessEvidence,
}

impl fmt::Debug for BookRecord {
    /// Prevents raw input metadata from entering debug diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BookRecord { metadata: [REDACTED] }")
    }
}

/// Contains validated metadata and context-specific observations for one book.
///
/// Values are only as current and reliable as the supplied evidence. No
/// upstream freshness, atomicity across pages, or access right is established.
#[derive(Clone, Eq, PartialEq)]
pub struct Book {
    /// Retains the opaque identifier without imposing an upstream numeric type.
    id: String,
    /// Retains metadata without inventing a title from an identifier.
    title: String,
    /// Stores a parsed source location, never a request configuration.
    source_url: Url,
    /// Retains independent contexts in their first-seen order.
    evidence: Vec<AccessEvidence>,
}

impl Book {
    /// Validates one observation before it enters the result.
    pub(super) fn from_record(record: BookRecord) -> Result<Self, LibraryError> {
        if !valid_text(&record.id) || !valid_text(&record.title) {
            return Err(LibraryError::InvalidMetadata);
        }
        let source_url =
            Url::parse(&record.source_url).map_err(|_| LibraryError::InvalidMetadata)?;
        if source_url.scheme() != "https"
            || source_url.host_str().is_none()
            || !source_url.username().is_empty()
            || source_url.password().is_some()
            || source_url.fragment().is_some()
            || record.source_url.chars().any(char::is_control)
            || record.source_url.trim() != record.source_url
        {
            return Err(LibraryError::InvalidMetadata);
        }
        record.evidence.validate()?;
        Ok(Self {
            id: record.id,
            title: record.title,
            source_url,
            evidence: vec![record.evidence],
        })
    }

    /// Returns the opaque book identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the validated title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the HTTPS source URL without authorizing a request to it.
    pub fn source_url(&self) -> &Url {
        &self.source_url
    }

    /// Returns all distinct observations in their first-seen order.
    pub fn evidence(&self) -> &[AccessEvidence] {
        &self.evidence
    }

    /// Derives a summary without storing a permanent usable-access flag.
    ///
    /// An independent grant survives denial in another context. Without any
    /// grant, unknown evidence prevents a claim that every context is unavailable.
    /// Future reads must check access again.
    pub fn access(&self) -> BookAccess {
        let purchased = self.evidence.iter().any(|evidence| {
            matches!(
                evidence,
                AccessEvidence::Purchased {
                    state: AccessState::Granted
                }
            )
        });
        let shared = self.evidence.iter().any(|evidence| {
            matches!(
                evidence,
                AccessEvidence::Shared {
                    state: AccessState::Granted,
                    ..
                }
            )
        });
        match (purchased, shared) {
            (true, true) => BookAccess::PurchasedAndShared,
            (true, false) => BookAccess::Purchased,
            (false, true) => BookAccess::Shared,
            (false, false) => {
                if self.evidence.iter().all(|evidence| {
                    matches!(
                        evidence,
                        AccessEvidence::Purchased {
                            state: AccessState::Unavailable
                        } | AccessEvidence::Shared {
                            state: AccessState::Unavailable,
                            ..
                        }
                    )
                }) {
                    BookAccess::Unavailable
                } else {
                    BookAccess::Unknown
                }
            }
        }
    }

    /// Merges one validated duplicate without choosing a conflicting observation.
    pub(super) fn merge(&mut self, other: Self) -> Result<(), LibraryError> {
        if self.title != other.title || self.source_url != other.source_url {
            return Err(LibraryError::ConflictingMetadata);
        }
        for observation in other.evidence {
            match self
                .evidence
                .iter()
                .find(|existing| existing.same_context(&observation))
            {
                Some(existing) if existing != &observation => {
                    return Err(LibraryError::ConflictingEvidence);
                }
                Some(_) => {}
                None => self.evidence.push(observation),
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Book {
    /// Omits identifiers, titles, URLs, and sharing-context values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Book { metadata: [REDACTED] }")
    }
}

/// Requires meaningful, unpadded metadata without terminal control characters.
pub(crate) fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}
