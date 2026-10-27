//! Shared code for the Beyond Gate CLI and MCP server.
//!
//! Account access is not implemented. This foundation shares only a status message.
//! Future read operations belong here so both interfaces use the same behavior.

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
