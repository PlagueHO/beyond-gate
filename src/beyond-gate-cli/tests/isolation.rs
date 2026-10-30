//! Process checks for test-build identity gates without native-store access.

use std::process::Command;

/// Checks the build handshake without reading or writing a credential.
#[test]
fn context_handshake_is_not_a_product_import_interface() {
    let output = Command::new(env!("CARGO_BIN_EXE_beyond-gate"))
        .arg("--e2e-context-check")
        .env(
            "BEYOND_GATE_E2E_CONTEXT",
            "0123456789abcdef0123456789abcdef",
        )
        .env("BEYOND_GATE_E2E_NATIVE_APPROVED", "0")
        .output()
        .expect("run binary");
    #[cfg(feature = "e2e-isolation")]
    {
        assert!(output.status.success());
        assert_eq!(output.stdout, b"beyond-gate-e2e-isolation-v1\n");
        assert!(output.stderr.is_empty());
    }
    #[cfg(not(feature = "e2e-isolation"))]
    {
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

/// Ensures feature builds reject native auth before opening storage without approval.
#[cfg(feature = "e2e-isolation")]
#[test]
fn auth_rejects_missing_or_unapproved_test_context() {
    for action in ["status", "logout"] {
        for context in ["", "local-session", "0123456789abcdef0123456789abcdef"] {
            let output = Command::new(env!("CARGO_BIN_EXE_beyond-gate"))
                .args(["auth", action])
                .env("BEYOND_GATE_E2E_CONTEXT", context)
                .env("BEYOND_GATE_E2E_NATIVE_APPROVED", "0")
                .output()
                .expect("run gated auth");
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("isolated test storage requires")
            );
        }
    }
}
