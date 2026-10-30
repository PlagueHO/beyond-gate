//! Checks the test-build marker without starting protocol or credential access.

use std::process::Command;

/// Keeps the isolated build marker absent from the ordinary product binary.
#[test]
fn context_handshake_requires_an_explicit_test_build() {
    let output = Command::new(env!("CARGO_BIN_EXE_beyond-gate-mcp"))
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
