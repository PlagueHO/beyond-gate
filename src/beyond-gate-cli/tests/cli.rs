//! Process-level checks for the public CLI contract.

use std::process::{Command, Output};

/// Runs the compiled binary without a shell.
///
/// Cargo supplies the binary path through `env!`, so the tests do not need PATH
/// changes or an installed copy of Beyond Gate.
///
/// # Panics
///
/// Panics if the operating system cannot start or wait for the test binary.
fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_beyond-gate"))
        .args(arguments)
        .output()
        .expect("the CLI test binary must run")
}

/// Checks that help is successful and states the foundation limit.
#[test]
fn help_reports_foundation_scope() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: beyond-gate"));
    assert!(stdout.contains(beyond_gate_core::FOUNDATION_NOTICE));
    assert!(stdout.contains("local MCP serving are available"));
    assert!(!stdout.contains("MCP serving are not implemented"));
}

/// Checks that version output uses the shared workspace package version.
#[test]
fn version_reports_package_version() {
    let output = run(&["--version"]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        concat!("beyond-gate ", env!("CARGO_PKG_VERSION"))
    );
}

/// Checks that absent or unknown operations cannot return fake success.
#[test]
fn missing_or_unknown_operation_fails() {
    for arguments in [vec![], vec!["unknown", "list"], vec!["--unknown"]] {
        let output = run(&arguments);

        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

/// Checks all resource routes fail honestly without storage, network, or fake data.
#[test]
fn live_resource_reads_are_explicitly_unsupported() {
    for arguments in [
        vec!["character", "list"],
        vec!["character", "show", "invented"],
        vec!["campaign", "list"],
        vec!["campaign", "show", "invented"],
        vec!["book", "list"],
        vec!["book", "outline", "invented"],
        vec!["book", "read", "invented", "opening"],
        vec!["character", "list", "--format", "json"],
        vec![
            "book",
            "outline",
            "invented",
            "--format",
            "tsv",
            "--no-headers",
        ],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported"));
    }
}

/// Checks root, resource, and action help against the actual command definition.
#[test]
fn resource_help_matches_available_commands() {
    for arguments in [
        vec!["character", "--help"],
        vec!["character", "list", "--help"],
        vec!["character", "show", "--help"],
        vec!["campaign", "--help"],
        vec!["campaign", "list", "--help"],
        vec!["campaign", "show", "--help"],
        vec!["book", "--help"],
        vec!["book", "list", "--help"],
        vec!["book", "outline", "--help"],
        vec!["book", "read", "--help"],
        vec!["completions", "--help"],
        vec!["mcp", "--help"],
        vec!["mcp", "start", "--help"],
        vec!["mcp", "install", "--help"],
    ] {
        let output = run(&arguments);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: beyond-gate"));
    }
    let output = run(&["book", "read", "--help"]);
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("4,096"));
    assert!(help.contains("--continuation"));
    assert!(help.contains("unsupported"));
    let output = run(&["mcp", "install", "--help"]);
    let help = String::from_utf8_lossy(&output.stdout);
    for expected in ["vscode", "claude-code", "copilot-cli", "--dry-run"] {
        assert!(help.contains(expected));
    }
}

/// Checks invalid input fails before any source or credential service is used.
#[test]
fn invalid_resource_usage_has_no_result() {
    for arguments in [
        vec!["character"],
        vec!["campaign"],
        vec!["book"],
        vec!["character", "show"],
        vec!["campaign", "show"],
        vec!["book", "outline"],
        vec!["book", "read", "book"],
        vec!["character", "show", " padded"],
        vec!["book", "read", "book", "bad\nsection"],
        vec!["character", "list", "--format", "yaml"],
        vec!["character", "list", "--format", "json", "--no-headers"],
        vec!["character", "show", "hero", "--format", "table"],
        vec![
            "book",
            "read",
            "book",
            "section",
            "--continuation",
            "private-invalid",
        ],
        vec!["completions"],
        vec!["completions", "unknown"],
        vec!["mcp"],
        vec!["mcp", "unknown"],
        vec!["mcp", "install"],
        vec!["mcp", "install", "unknown"],
        vec!["mcp", "install", "copilot-cli", "--unknown"],
        vec!["--fixture-child", "normal"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("private-invalid"));
    }
}

/// Generates each supported shell's script from the actual resource command tree.
#[test]
fn completions_include_real_commands_and_options() {
    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        let output = run(&["completions", shell]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let script = String::from_utf8_lossy(&output.stdout);
        for expected in [
            "character",
            "campaign",
            "book",
            "mcp",
            "start",
            "install",
            "outline",
            "read",
            "auth",
            "completions",
            "continuation",
            "no-headers",
        ] {
            assert!(script.contains(expected), "{shell}: missing {expected}");
        }
    }
}
/// Checks nested auth help without opening the user's protected credential store.
#[test]
fn auth_help_describes_local_commands() {
    for arguments in [
        vec!["auth", "--help"],
        vec!["auth", "login", "--help"],
        vec!["auth", "status", "--help"],
        vec!["auth", "logout", "--help"],
    ] {
        let output = run(&arguments);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: beyond-gate auth"));
    }
    let output = run(&["auth", "--help"]);
    let help = String::from_utf8_lossy(&output.stdout);
    for name in ["login", "status", "logout"] {
        assert!(help.contains(name));
    }
}

/// Checks blocked login fails before terminal input, storage, or network activity.
#[test]
fn auth_login_reports_the_blocker_without_stdout() {
    let output = run(&["auth", "login"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("no working session handoff"));
    assert!(diagnostic.contains("do not supply a password or cookie"));
}

/// Checks malformed auth commands cannot accept credentials or access storage.
#[test]
fn invalid_auth_usage_fails_before_storage() {
    for arguments in [
        vec!["auth"],
        vec!["auth", "unknown"],
        vec!["auth", "login", "--cookie", "synthetic-only"],
        vec!["auth", "login", "synthetic-only"],
        vec!["auth", "status", "--token", "synthetic-only"],
        vec!["auth", "logout", "--all"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}
