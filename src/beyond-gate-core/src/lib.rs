//! Shared code for the Beyond Gate CLI and MCP server.
//!
//! Account access is not implemented. The offline-tested HTTP boundary has no
//! built-in upstream hosts or endpoints. Local auth operations use one protected
//! credential entry. Browser login, session exchange, and MCP serving remain
//! blocked. Character, campaign, and library assembly accept supplied offline
//! pages only.

pub mod auth;
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
pub const FOUNDATION_NOTICE: &str = "Local auth status and logout only: browser login, account access, and MCP serving are not implemented.";
