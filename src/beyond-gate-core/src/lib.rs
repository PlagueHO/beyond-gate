//! Shared code for the Beyond Gate CLI and MCP server.
//!
//! The bounded HTTP boundary requires explicit origins. Local auth operations use one protected
//! credential entry. Windows CLI login calls the shared service-validation/save
//! API after paired submission; Windows browser setup is implemented. Both adapters
//! expose fresh Windows validation through shared process-local session reuse.
//! Windows library and book reads use the shared session. Book reads require fresh
//! positive library evidence and checked canonical cookie pages for each operation.
//! Character and campaign reads use fixed, bounded operations through a
//! protected Windows session. Synthetic tests validate adapter behavior, not
//! live source compatibility. No operation caches book content.
//! Class/subclass reads expose only checked catalog metadata, never raw rules definitions.
//! Monster reads expose one explicit page or unchanged selected detail with access code 1;
//! denied code 4 and unverified access cases never produce content.

pub mod auth;
pub mod book;
pub mod character_campaign;
pub mod class_catalog;
pub mod http;
pub mod library;
pub mod monster;
mod public_config;

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
/// assert!(FOUNDATION_NOTICE.contains("bounded"));
/// ```
pub const FOUNDATION_NOTICE: &str = "Local auth status/logout, CLI completions, and local MCP serving are available. Human-controlled Windows browser login and protected saving are available after explicit extension setup. Windows auth validate and MCP auth_validate use shared bounded session validation and reuse. Windows book list discovers bounded Web-catalog metadata and evidence. Book outline/read and their MCP tools check fresh library evidence and canonical cookie pages, with explicit bounded continuation. Character and campaign reads use fixed, bounded operations through a protected Windows session; live source compatibility has not been validated. Windows class/subclass list and show return bounded catalog metadata only, not rules text or raw definitions; native compatibility has not been validated. Windows monster list/show return one explicit page or unchanged selected detail only for observed access code 1; code 4 is denied and unverified codes, including 2 and 3, are unsupported. Native monster acceptance has not been run. Linux production login and session use remain deferred.";
