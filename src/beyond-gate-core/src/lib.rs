//! Shared code for the Beyond Gate CLI and MCP server.
//!
//! Account access is not implemented. The offline-tested HTTP boundary has no
//! built-in upstream hosts or endpoints. Both adapters still share only a status
//! message; future read operations belong here.

pub mod http;

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
pub const FOUNDATION_NOTICE: &str =
    "Foundation only: account access and MCP serving are not implemented.";
