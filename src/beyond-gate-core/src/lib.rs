//! Shared code for the Beyond Gate CLI and MCP server.
//!
//! Account access is not implemented. The offline-tested HTTP boundary has no
//! built-in upstream hosts or endpoints. Local auth operations use one protected
//! credential entry. Windows CLI login calls the shared service-validation/save
//! API after paired submission; browser setup remains incomplete. The local
//! MCP server exposes explicit unsupported reads. Character, campaign, and library
//! assembly accept supplied offline pages only. Book outlines and bounded section
//! reads use fresh supplied access evidence and normalized offline records, not
//! live account data.

pub mod auth;
pub mod book;
pub mod character_campaign;
pub mod http;
pub mod library;

/// Describes the current boundary of the runnable foundation.
///
/// A constant string has a `'static` lifetime: its text is stored in the binary,
/// so callers can borrow it without allocating or managing an owned `String`.
///
/// # Examples
///
/// ```
/// use beyond_gate_core::FOUNDATION_NOTICE;
///
/// assert!(FOUNDATION_NOTICE.contains("not implemented"));
/// ```
pub const FOUNDATION_NOTICE: &str = "Local auth status/logout, CLI completions, and local MCP serving are available. Windows browser pairing is wired; extension setup is incomplete. Live resource reads and account access are not implemented.";
