//! Shared code for the Beyond Gate CLI and MCP server.
//!
//! The bounded HTTP boundary requires explicit origins. Local auth operations use one protected
//! credential entry. Windows CLI login calls the shared service-validation/save
//! API after paired submission; Windows browser setup is implemented. Both adapters
//! expose fresh Windows validation through shared process-local session reuse.
//! Windows library and book reads use the shared session. Book reads require fresh
//! positive library evidence and checked canonical cookie pages for each operation.
//! Character and campaign live reads remain unsupported. Offline assembly APIs
//! remain available for synthetic records. No operation caches book content.

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
pub const FOUNDATION_NOTICE: &str = "Local auth status/logout, CLI completions, and local MCP serving are available. Human-controlled Windows browser login and protected saving are available after explicit extension setup. Windows auth validate and MCP auth_validate use shared bounded session validation and reuse. Windows book list discovers bounded Web-catalog metadata and evidence. Book outline/read and their MCP tools check fresh library evidence and canonical cookie pages, with explicit bounded continuation. Unknown access and unsupported layouts return no content. Character and campaign live reads are not implemented; Linux production login and session use remain deferred.";
