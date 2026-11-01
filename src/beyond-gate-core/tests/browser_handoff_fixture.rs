//! Consistency checks for the proposed, synthetic browser handoff fixture.
//!
//! These tests read the shared examples and expected effects. They do not apply
//! event recipes, authenticate peers, implement a protocol, or establish login.

use std::collections::BTreeSet;

use beyond_gate_core::auth::handoff::ErrorCode;
use serde_json::Value;

mod support;
use support::fixture;

/// Requires a named string without treating missing data as an empty value.
fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    value[field]
        .as_str()
        .expect("fixture field must be a string")
}

/// Checks exact object fields without formatting their possibly secret-like values.
fn exact_fields(value: &Value, fields: &[&str]) {
    let actual: BTreeSet<_> = value
        .as_object()
        .expect("fixture value must be an object")
        .keys()
        .map(String::as_str)
        .collect();
    let expected: BTreeSet<_> = fields.iter().copied().collect();
    assert_eq!(actual, expected, "fixture object fields differ");
}

/// Verifies a canonical example's shape and bounds, not hostile wire parsing.
fn canonical_message(message: &Value, limits: &Value) {
    let protocol = text(message, "protocol");
    let kind = text(message, "type");
    assert_eq!(message["version"].as_u64(), Some(1));
    let fields: &[&str] = match (protocol, kind) {
        ("bg-browser-handoff", "hello") | ("bg-host-cli", "open") => {
            &["browser", "profile", "consent"]
        }
        ("bg-browser-handoff", "pending") | ("bg-host-cli", "authorized") => {
            &["transaction", "profile", "expires_in_ms"]
        }
        ("bg-browser-handoff" | "bg-host-cli", "submit") => &["transaction", "profile", "value"],
        ("bg-browser-handoff" | "bg-host-cli", "cancel") => &["transaction"],
        ("bg-browser-handoff" | "bg-host-cli", "result") => {
            &["transaction", "outcome", "validation", "expiry"]
        }
        ("bg-browser-handoff" | "bg-host-cli", "error") => &["transaction", "code"],
        _ => panic!("unsupported canonical message protocol or type"),
    };
    let fields: Vec<_> = ["protocol", "version", "type"]
        .into_iter()
        .chain(fields.iter().copied())
        .collect();
    exact_fields(message, &fields);

    if let Some(transaction) = message.get("transaction")
        && (kind != "error" || !transaction.is_null())
    {
        let transaction = transaction.as_str().expect("transaction must be a string");
        assert_eq!(transaction.len(), 32);
        assert!(
            transaction
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
    }
    if message.get("profile").is_some() {
        assert_eq!(text(message, "profile"), "synthetic-v1");
    }
    if message.get("browser").is_some() {
        assert!(matches!(text(message, "browser"), "chrome" | "edge"));
    }
    if message.get("consent").is_some() {
        assert_eq!(message["consent"].as_bool(), Some(true));
    }
    if let Some(value) = message.get("value") {
        let value = value.as_str().expect("synthetic value must be a string");
        assert!(value.starts_with("synthetic-"));
        assert!(value.len() as u64 <= limits["value_bytes"].as_u64().expect("value bound"));
    }
    if let Some(expiry) = message.get("expires_in_ms") {
        let expiry = expiry.as_u64().expect("relative expiry must be an integer");
        assert!(expiry > 0);
        assert!(expiry <= limits["pending_ms"].as_u64().expect("pending bound"));
    }
    if kind == "result" {
        assert_eq!(text(message, "outcome"), "saved");
        assert_eq!(text(message, "validation"), "service-validated");
        assert_eq!(text(message, "expiry"), "unknown");
    }
    if kind == "error" {
        assert_ne!(text(message, "code"), "ok");
        assert!(ErrorCode::parse(text(message, "code")).is_ok());
    }
    let bytes = serde_json::to_vec(message).expect("encode canonical synthetic example");
    assert!(!bytes.is_empty());
    assert!(bytes.len() as u64 <= limits["frame_bytes"].as_u64().expect("frame bound"));
}

/// Pins the fixture's synthetic-only, proposed status without enabling a product profile.
#[test]
fn fixture_is_explicitly_synthetic_and_proposed() {
    let fixture = fixture();
    exact_fields(
        &fixture,
        &[
            "fixture_version",
            "status",
            "synthetic_only",
            "clock_unix_seconds",
            "limits",
            "profile",
            "default_conditions",
            "cookie_fixture",
            "messages",
            "cases",
        ],
    );
    assert_eq!(fixture["fixture_version"].as_u64(), Some(1));
    assert_eq!(fixture["synthetic_only"].as_bool(), Some(true));
    assert_eq!(text(&fixture, "status"), "proposed-blocked");
    assert_eq!(text(&fixture["profile"], "id"), "synthetic-v1");
    assert_eq!(
        text(&fixture["profile"], "url"),
        "https://login.beyond-gate.invalid/"
    );
    assert_eq!(
        text(&fixture["cookie_fixture"], "name"),
        "BeyondGateSyntheticSession"
    );
    assert!(text(&fixture["cookie_fixture"], "value").starts_with("synthetic-"));
}

/// Detects unreviewed changes to the version 1 proposal's safety bounds.
#[test]
fn proposed_limits_match_the_versioned_examples() {
    let fixture = fixture();
    let limits = &fixture["limits"];
    for (name, expected) in [
        ("frame_bytes", 16384),
        ("value_bytes", 8192),
        ("pending_ms", 300000),
        ("hello_ms", 10000),
        ("cookie_query_ms", 15000),
        ("validation_ms", 15000),
        ("connect_ms", 5000),
        ("save_wait_ms", 5000),
        ("json_depth", 8),
        ("frames_per_direction", 8),
    ] {
        assert_eq!(limits[name].as_u64(), Some(expected), "{name}");
    }
}

/// Reads every canonical message and requires separate Chrome and Edge examples.
#[test]
fn canonical_messages_have_exact_shapes_and_bounded_values() {
    let fixture = fixture();
    let messages = fixture["messages"].as_object().expect("canonical messages");
    assert!(!messages.is_empty());
    for message in messages.values() {
        canonical_message(message, &fixture["limits"]);
    }
    for name in [
        "chrome_hello",
        "browser_hello",
        "cli_chrome_open",
        "cli_open",
    ] {
        assert!(messages.contains_key(name), "missing browser example");
    }
}

/// Checks every case ID and message reference without executing injected events.
#[test]
fn cases_have_unique_ids_and_resolved_message_references() {
    let fixture = fixture();
    let messages = fixture["messages"].as_object().expect("canonical messages");
    let cases = fixture["cases"].as_array().expect("conformance cases");
    assert!(!cases.is_empty());
    let mut ids = BTreeSet::new();
    for case in cases {
        exact_fields(
            case,
            &["id", "boundary", "state", "input", "conditions", "expected"],
        );
        assert!(ids.insert(text(case, "id")), "duplicate case ID");
        assert!(matches!(
            text(case, "boundary"),
            "browser" | "host-cli" | "core" | "combined"
        ));
        if let Some(message) = case["input"].get("message") {
            assert!(messages.contains_key(message.as_str().expect("message reference")));
        }
        if let Some(sequence) = case["input"].get("sequence") {
            for entry in sequence.as_array().expect("sequence entries") {
                let name = entry.as_str().expect("sequence entry must be a string");
                assert!(
                    messages.contains_key(name)
                        || matches!(name, "service_validated" | "save_confirmed"),
                    "unresolved sequence entry"
                );
            }
        }
    }
}

/// Prevents fixture failures from granting new reads, validation, or writes.
#[test]
fn expected_errors_allow_no_new_side_effects() {
    let fixture = fixture();
    for case in fixture["cases"].as_array().expect("conformance cases") {
        let expected = &case["expected"];
        exact_fields(expected, &["state", "code", "storage", "effects"]);
        let effects = &expected["effects"];
        exact_fields(effects, &["cookie_read", "validate", "save"]);
        let code = text(expected, "code");
        assert!(code == "ok" || ErrorCode::parse(code).is_ok());
        for field in ["cookie_read", "validate", "save"] {
            let effect = effects[field].as_bool().expect("effect must be a boolean");
            if code != "ok" {
                assert!(!effect, "error expectation grants a new side effect");
            }
        }
        assert!(matches!(
            text(expected, "storage"),
            "unchanged" | "saved" | "unknown"
        ));
        if code == "save_indeterminate" {
            assert_eq!(text(expected, "storage"), "unknown");
            assert_eq!(text(expected, "state"), "indeterminate");
        }
    }
}
