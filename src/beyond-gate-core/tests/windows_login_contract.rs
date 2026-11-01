//! Shared invented production-profile recipes; parsing does not validate an account.

#![cfg(windows)]

use beyond_gate_core::auth::handoff::{
    ErrorCode, Message, Protocol, browser, encode_message, parse_message,
};
use serde_json::Value;

/// Reads only the committed invented recipe, never an account or local credential.
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../docs/fixtures/windows-login-v1.json"))
        .expect("shared invented fixture")
}

/// Checks package identity and each runtime message against the actual shared codec.
#[test]
fn production_recipes_match_runtime_without_granting_success() {
    let fixture = fixture();
    assert_eq!(fixture["synthetic_only"], true);
    assert_eq!(fixture["host_name"], browser::HOST_NAME);
    assert_eq!(fixture["extension_id"], browser::EXTENSION_ID);
    for name in [
        "pair",
        "open",
        "pending",
        "authorized",
        "submit",
        "result",
        "ack",
        "host_ack",
    ] {
        let recipe = &fixture["messages"][name];
        let protocol = if recipe["protocol"] == "bg-host-cli" {
            Protocol::HostCli
        } else {
            Protocol::Browser
        };
        let message = parse_message(protocol, serde_json::to_vec(recipe).expect("recipe"))
            .expect("runtime message");
        if matches!(message, Message::Pair { .. }) {
            assert_eq!(
                encode_message(protocol, &message)
                    .expect_err("input-only key")
                    .code(),
                ErrorCode::ProtocolError
            );
        } else {
            let encoded: Value =
                serde_json::from_slice(&encode_message(protocol, &message).expect("encode"))
                    .expect("JSON");
            assert_eq!(&encoded, recipe);
        }
    }
}
