//! Shared, credential-free browser handoff fixture loading for integration tests.

use std::path::Path;

use serde_json::Value;

/// Reads only the repository's synthetic fixture, independent of the working directory.
pub(super) fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("fixtures")
        .join("browser-handoff-v1.json");
    let bytes = std::fs::read(path).expect("read canonical synthetic fixture");
    serde_json::from_slice(&bytes).expect("parse canonical synthetic fixture")
}
