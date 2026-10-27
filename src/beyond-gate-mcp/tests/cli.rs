//! Process-level checks for the future MCP server's startup boundary.

use std::process::{Command, Output};

/// Runs the compiled MCP binary without a shell or real MCP client.
///
/// # Panics
///
/// Panics if the operating system cannot start or wait for the test binary.
fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_beyond-gate-mcp"))
        .args(arguments)
        .output()
        .expect("the MCP test binary must run")
}

/// Checks that help states that no MCP service is implemented.
#[test]
fn help_reports_foundation_scope() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: beyond-gate-mcp"));
    assert!(stdout.contains(beyond_gate_core::FOUNDATION_NOTICE));
}

/// Checks that version output uses the shared workspace package version.
#[test]
fn version_reports_package_version() {
    let output = run(&["--version"]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        concat!("beyond-gate-mcp ", env!("CARGO_PKG_VERSION"))
    );
}

/// Checks that startup fails explicitly without writing fake protocol data.
#[test]
fn serving_fails_explicitly() {
    let output = run(&[]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains(beyond_gate_core::FOUNDATION_NOTICE));
}

/// Checks that unsupported transport flags are not silently accepted.
#[test]
fn unknown_arguments_fail() {
    let output = run(&["--transport", "stdio"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
