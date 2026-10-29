//! Bounded wire encoding for untrusted section positions, never access grants.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use super::{BookError, SectionCursor, model::source_url};
use crate::library::valid_text;

/// Maximum encoded continuation bytes, including the version prefix.
pub const MAX_CURSOR_BYTES: usize = 16_384;

/// Identifies the only supported continuation wire version.
const PREFIX: &str = "bg1.";

/// Keeps wire fields separate from the opaque validated Rust value.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCursor {
    book_id: String,
    section_id: String,
    revision: String,
    source_url: String,
    next_byte: usize,
}

impl SectionCursor {
    /// Encodes this position for an explicit later call, without storing content.
    ///
    /// The token is URL-safe, unpadded base64 with a `bg1.` prefix. It is not
    /// encrypted or signed and cannot authorize access. Its size, including the
    /// prefix, is at most [`MAX_CURSOR_BYTES`].
    ///
    /// # Errors
    ///
    /// Returns [`BookError::ContinuationTooLarge`] if metadata cannot fit, or
    /// [`BookError::InvalidContinuation`] if serialization fails.
    pub fn to_token(&self) -> Result<String, BookError> {
        let metadata_bytes = self
            .book_id
            .len()
            .saturating_add(self.section_id.len())
            .saturating_add(self.revision.len())
            .saturating_add(self.source_url.as_str().len());
        if metadata_bytes > MAX_CURSOR_BYTES {
            return Err(BookError::ContinuationTooLarge);
        }
        let wire = WireCursor {
            book_id: self.book_id.clone(),
            section_id: self.section_id.clone(),
            revision: self.revision.clone(),
            source_url: self.source_url.to_string(),
            next_byte: self.next_byte,
        };
        let bytes = serde_json::to_vec(&wire).map_err(|_| BookError::InvalidContinuation)?;
        let encoded_len =
            base64::encoded_len(bytes.len(), false).ok_or(BookError::ContinuationTooLarge)?;
        if encoded_len > MAX_CURSOR_BYTES - PREFIX.len() {
            return Err(BookError::ContinuationTooLarge);
        }
        Ok(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes)))
    }

    /// Parses a bounded untrusted token without loading a source or granting access.
    ///
    /// Source-dependent checks remain in [`super::read_section`]. That operation
    /// checks fresh access, selected identity, revision, URL, and UTF-8 position.
    ///
    /// # Errors
    ///
    /// Returns [`BookError::ContinuationTooLarge`] before decoding oversized
    /// input. All malformed encoding, version, field, or metadata failures
    /// return [`BookError::InvalidContinuation`] without echoing token data.
    pub fn from_token(token: &str) -> Result<Self, BookError> {
        if token.len() > MAX_CURSOR_BYTES {
            return Err(BookError::ContinuationTooLarge);
        }
        let encoded = token
            .strip_prefix(PREFIX)
            .ok_or(BookError::InvalidContinuation)?;
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| BookError::InvalidContinuation)?;
        let wire: WireCursor =
            serde_json::from_slice(&bytes).map_err(|_| BookError::InvalidContinuation)?;
        if !valid_text(&wire.book_id)
            || !valid_text(&wire.section_id)
            || !valid_text(&wire.revision)
            || wire.next_byte == 0
        {
            return Err(BookError::InvalidContinuation);
        }
        let source_url =
            source_url(&wire.source_url).map_err(|_| BookError::InvalidContinuation)?;
        Ok(Self {
            book_id: wire.book_id,
            section_id: wire.section_id,
            revision: wire.revision,
            source_url,
            next_byte: wire.next_byte,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates an untrusted wire object without including source content.
    fn wire() -> serde_json::Value {
        serde_json::json!({
            "book_id": "invented-book",
            "section_id": "opening",
            "revision": "v1",
            "source_url": "https://example.com/book#opening",
            "next_byte": 4096
        })
    }

    /// Encodes arbitrary JSON to exercise parser failures.
    fn token(value: &serde_json::Value) -> String {
        format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(value.to_string()))
    }

    /// Preserves all cursor fields and omits private metadata from diagnostics.
    #[test]
    fn round_trip_is_opaque_and_redacted() {
        let encoded = token(&wire());
        let cursor = SectionCursor::from_token(&encoded).expect("synthetic token");
        let canonical = cursor.to_token().expect("encode");
        assert_eq!(
            SectionCursor::from_token(&canonical).expect("decode"),
            cursor
        );
        assert!(!format!("{cursor:?}").contains("invented-book"));
    }

    /// Rejects invalid versions, encodings, fields, and source metadata.
    #[test]
    fn invalid_tokens_never_echo_metadata() {
        for invalid in ["", "bg2.e30", "bg1.%%%%", "bg1.e30=", "bg1.e30"] {
            assert!(SectionCursor::from_token(invalid).is_err());
        }
        for (field, value) in [
            ("book_id", serde_json::json!(" padded")),
            ("section_id", serde_json::json!("bad\nid")),
            ("revision", serde_json::json!("")),
            (
                "source_url",
                serde_json::json!("https://user:secret@example.com"),
            ),
            ("source_url", serde_json::json!("http://example.com")),
            ("next_byte", serde_json::json!(0)),
            ("next_byte", serde_json::json!(-1)),
            ("extra", serde_json::json!("private")),
        ] {
            let mut value_wire = wire();
            value_wire[field] = value;
            let error = SectionCursor::from_token(&token(&value_wire)).expect_err("invalid");
            assert!(matches!(error, BookError::InvalidContinuation));
            assert!(!error.to_string().contains("private"));
        }
        let duplicate = r#"{"book_id":"one","book_id":"two","section_id":"s","revision":"r","source_url":"https://example.com","next_byte":1}"#;
        assert!(
            SectionCursor::from_token(&format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(duplicate)))
                .is_err()
        );
        let mut missing = wire();
        missing.as_object_mut().expect("object").remove("revision");
        assert!(SectionCursor::from_token(&token(&missing)).is_err());
    }

    /// Accepts the exact encoded cap and rejects the next byte before decoding.
    #[test]
    fn encoded_limit_is_exact_in_both_directions() {
        let mut value = wire();
        let base_len = value.to_string().len();
        // 12,285 payload bytes encode to 16,380 bytes plus the four-byte prefix.
        value["revision"] = serde_json::json!("r".repeat(12_285 - base_len + 2));
        let encoded = token(&value);
        assert_eq!(encoded.len(), MAX_CURSOR_BYTES);
        let cursor = SectionCursor::from_token(&encoded).expect("exact limit");
        assert_eq!(
            cursor.to_token().expect("encode at limit").len(),
            MAX_CURSOR_BYTES
        );
        assert!(matches!(
            SectionCursor::from_token(&(encoded + "A")),
            Err(BookError::ContinuationTooLarge)
        ));
        let mut oversized = cursor;
        oversized.revision.push('r');
        assert!(matches!(
            oversized.to_token(),
            Err(BookError::ContinuationTooLarge)
        ));
    }
}
