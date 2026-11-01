//! Bounded framing and parsing for the proposed version 1 handoff.
//!
//! Parsing proves only message shape and syntax. It does not authenticate a
//! browser, extension, CLI, or session, and cannot authorize cookie access or a
//! protected write. Windows also recognizes the bounded `cobalt-session-v1`
//! profile; other platforms recognize only the invented `synthetic-v1` profile.
//! Windows CLI orchestration enables paired submission and shared completion.
//! Product browser selection, registration, and end-to-end evidence remain
//! incomplete. Generic parsers alone never establish a validated login.

mod error;
mod framing;
mod message;

/// Relays one consented Windows browser exchange to the authenticated pending CLI.
#[cfg(windows)]
pub mod browser;

/// Monitors cancellation while the real shared operation validates and saves.
#[cfg(windows)]
pub mod completion;

/// Provides a single-instance Windows stream with explicit logon-only client rights.
#[cfg(windows)]
pub mod endpoint;

/// Binds one consented submission to an already paired connection and transaction.
pub mod flow;

/// Implements the approved pairing direction without enabling adapter ingress.
pub mod pairing;

/// Reserves one pending login per Windows logon without accepting connections.
pub mod slot;

/// Exchanges paired, bounded host-to-CLI messages over an owned local stream.
pub mod transport;

pub use error::{ErrorCode, HandoffCause, HandoffError};
pub use framing::{FrameReader, FrameWriter};
pub use message::{
    Browser, Message, Profile, Protocol, Transaction, encode_message, parse_message,
};

/// Bounds a payload before allocation or JSON deserialization.
const MAX_FRAME_BYTES: usize = 16 * 1024;
/// Bounds structural JSON nesting before deserialization.
const MAX_JSON_DEPTH: usize = 8;
/// Bounds reads and writes independently on one connection.
const MAX_FRAMES: u8 = 8;
