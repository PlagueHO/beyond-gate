//! Shared code for the Beyond Gate CLI and MCP server.
//!
//! Account access is not implemented. The offline-tested HTTP boundary has no
//! built-in upstream hosts or endpoints. Local auth operations use one protected
//! credential entry. Windows CLI login calls the shared service-validation/save
//! API after paired submission; Windows browser setup is implemented. Both adapters
//! expose fresh Windows validation through shared process-local session reuse.
//! The MCP server exposes explicit unsupported resource reads. Character, campaign, and library
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
pub const FOUNDATION_NOTICE: &str = "Local auth status/logout, CLI completions, and local MCP serving are available. Human-controlled Windows browser login and protected saving are available after explicit extension setup. Windows auth validate and MCP auth_validate use shared bounded session validation and reuse. Windows book list and MCP book_list discover bounded Web-catalog metadata and access evidence through the shared session. Other live reads are not implemented; Linux production login and session use remain deferred.";
