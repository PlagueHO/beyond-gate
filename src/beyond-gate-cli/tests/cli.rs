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
    for arguments in [vec![], vec!["character", "list"], vec!["--unknown"]] {
        let output = run(&arguments);

        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
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
