//! Shared process-only cookie input and non-secret source labels.

use super::{
    handoff::ErrorCode,
    service::{BrowserSessionCookie, EstablishmentError},
};
use zeroize::Zeroizing;

/// Supplies a cookie value to one CLI or MCP process without implicit persistence.
pub const SESSION_ENV_VAR: &str = "BEYOND_GATE_COBALT_SESSION";

/// Identifies input provenance without disclosing credential material.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionSource {
    /// The fixed protected OS entry.
    Stored,
    /// A snapshot of the process environment.
    Environment,
    /// An explicitly requested hidden CLI input.
    Prompt,
}

impl SessionSource {
    /// Returns a stable, safe source label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stored => "stored",
            Self::Environment => "environment",
            Self::Prompt => "prompt",
        }
    }
}

/// Accepts only a bounded cookie value, never a copied header or assignment.
pub(super) fn cookie_value(bytes: Vec<u8>) -> Result<BrowserSessionCookie, EstablishmentError> {
    let mut bytes = Zeroizing::new(bytes);
    if bytes.starts_with(b"CobaltSession=")
        || bytes
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"cookie:"))
    {
        return Err(EstablishmentError::new(ErrorCode::InvalidMaterial));
    }
    BrowserSessionCookie::new(std::mem::take(&mut *bytes))
}
