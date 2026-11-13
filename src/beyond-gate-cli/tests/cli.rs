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
    assert!(stdout.contains("Web-catalog metadata"));
    assert!(!stdout.contains("discovery remains blocked"));
    assert!(!stdout.contains("MCP serving are not implemented"));
}

/// Reports Windows book reads without claiming support for every source layout.
#[test]
fn book_help_reports_web_catalog_scope() {
    let output = run(&["book", "--help"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Bounded Windows library, outline, and selected-section reads"));
    assert!(stdout.contains("canonical"));
    assert!(!stdout.contains("discovery blocked"));
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
        vec!["auth", "install", "--help"],
        vec!["auth", "uninstall", "--help"],
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
    assert!(help.contains("fresh access checks"));
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
        vec!["species"],
        vec!["character", "show"],
        vec!["campaign", "show"],
        vec!["species", "show"],
        vec!["book", "outline"],
        vec!["book", "read", "book"],
        vec!["character", "show", " padded"],
        vec!["book", "read", "book", "bad\nsection"],
        vec!["character", "list", "--format", "yaml"],
        vec!["character", "list", "--format", "json", "--no-headers"],
        vec!["character", "show", "hero", "--output", "yaml"],
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
        vec!["auth", "install", "edge"],
        vec!["auth", "uninstall", "chrome"],
        vec!["auth", "install", "unknown", "--directory", "."],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("private-invalid"));
    }
}

/// Rejects invalid catalog selectors and combinations in the actual shipped parser.
#[test]
fn class_metadata_help_and_invalid_usage() {
    for resource in ["class", "subclass"] {
        for action in ["list", "show"] {
            let output = run(&[resource, action, "--help"]);
            assert!(output.status.success());
            assert!(output.stderr.is_empty());
            assert!(String::from_utf8_lossy(&output.stdout).contains("metadata"));
        }
    }
    for arguments in [
        vec!["class", "show", "01"],
        vec!["subclass", "show", "0"],
        vec!["class", "show", "18446744073709551616"],
        vec!["class", "show", "10", "--name", "Fighter"],
        vec!["class", "list", "--class-id", "10"],
        vec!["class", "list", "--name", ""],
        vec!["class", "list", "--name", " padded"],
        vec!["class", "list", "--query", "Fighter"],
        vec!["class", "list", "10"],
        vec!["subclass", "list", "--class-id", "-1"],
        vec!["class", "list", "--output", "json", "--no-headers"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    let output = run(&["completions", "powershell"]);
    assert!(output.status.success());
    let completions = String::from_utf8_lossy(&output.stdout);
    assert!(completions.contains("subclass"));
    assert!(completions.contains("--class-id"));
    assert!(completions.contains("--name"));
}
/// Checks shipped monster help, completions, and input failures without session access.
#[test]
fn monster_help_and_invalid_usage() {
    for action in ["list", "show"] {
        let output = run(&["monster", action, "--help"]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("monster"));
    }
    for arguments in [
        vec!["monster"],
        vec!["monster", "show"],
        vec!["monster", "show", "01"],
        vec!["monster", "show", "0"],
        vec!["monster", "show", "18446744073709551616"],
        vec!["monster", "show", "17", "--name", "beast"],
        vec!["monster", "show", "17", "--output", "json", "--no-headers"],
        vec!["monster", "list", "--name", ""],
        vec!["monster", "list", "--name", " padded"],
        vec!["monster", "list", "--skip", "-1"],
        vec!["monster", "list", "--skip", "0"],
        vec!["monster", "list", "--take", "20"],
        vec![
            "monster",
            "list",
            "--skip",
            "18446744073709551615",
            "--take",
            "1",
        ],
        vec!["monster", "list", "--take", "0"],
        vec!["monster", "list", "--take", "101"],
        vec!["monster", "list", "--take", "1.5"],
        vec!["monster", "list", "--page", "0"],
        vec!["monster", "list", "--page", "-1"],
        vec!["monster", "list", "--size-id", "0"],
        vec!["monster", "list", "--size-id", "4", "--size-id", "4"],
        vec!["monster", "list", "--cr-min", "2", "--cr-max", "1/2"],
        vec![
            "monster",
            "list",
            "--armor-class-min",
            "10",
            "--armor-class-max",
            "9",
        ],
        vec!["monster", "list", "--legendary", "yes"],
        vec!["monster", "list", "--partnered-content", "false"],
        vec!["monster", "list", "--output", "json", "--no-headers"],
        vec!["monster", "list", "--output", "yaml"],
        vec!["monster", "list", "--homebrew"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("session_absent"));
    }
    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        let output = run(&["completions", shell]);
        assert!(output.status.success());
        let text = String::from_utf8_lossy(&output.stdout);
        for expected in ["monster", "page", "size-id", "partnered-content"] {
            assert!(text.contains(expected), "{shell}: {expected}");
        }
    }
}

/// Checks shipped species help, completions, and input failures before session access.
#[test]
fn species_help_and_invalid_usage() {
    for action in ["list", "show"] {
        let output = run(&["species", action, "--help"]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("species"));
    }
    for arguments in [
        vec!["species"],
        vec!["species", "show"],
        vec!["species", "show", "01"],
        vec!["species", "show", "0"],
        vec!["species", "show", "18446744073709551616"],
        vec!["species", "show", "17", "--name", "Origin"],
        vec!["species", "list", "--page", "1"],
        vec!["species", "list", "--output", "json", "--no-headers"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("session_absent"));
    }
    let output = run(&["completions", "powershell"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("species"));
}

/// Checks spell parsing, completions, and failures without native storage access.
#[test]
fn spell_help_completions_and_invalid_usage() {
    for action in ["list", "show"] {
        let output = run(&["spell", action, "--help"]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
    }
    for args in [
        vec!["spell"],
        vec!["spell", "show"],
        vec!["spell", "show", "17"],
        vec!["spell", "show", "01-invented"],
        vec!["spell", "show", "17-a%2fb"],
        vec!["spell", "show", "https://www.dndbeyond.com/spells/17-a"],
        vec!["spell", "list", "--level", "10"],
        vec!["spell", "list", "--level", "0", "--level", "0"],
        vec!["spell", "list", "--class-id", "17", "--class-id", "17"],
        vec!["spell", "list", "--school-id", "2"],
        vec!["spell", "list", "--school-id", "11"],
        vec!["spell", "list", "--page", "0"],
        vec!["spell", "list", "--verbal", "null"],
        vec!["spell", "list", "--take", "1"],
        vec!["spell", "list", "--output", "json", "--no-headers"],
        vec!["spell", "show", "17-a", "--class-id", "17"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("session_absent"));
    }
    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        let output = run(&["completions", shell]);
        assert!(output.status.success());
        let text = String::from_utf8_lossy(&output.stdout);
        for expected in ["spell", "casting-time-id", "school-id", "concentration"] {
            assert!(text.contains(expected), "{shell}: {expected}");
        }
    }
}

/// Checks background filters, help, completions, and input failures before native access.
#[test]
fn background_help_completions_and_invalid_usage() {
    for action in ["list", "show"] {
        let output = run(&["background", action, "--help"]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("background"));
    }
    for args in [
        vec!["background"],
        vec!["background", "show"],
        vec!["background", "show", "01"],
        vec!["background", "show", "0"],
        vec!["background", "show", "18446744073709551616"],
        vec!["background", "list", "--name", ""],
        vec!["background", "list", "--name", " padded"],
        vec!["background", "list", "--skill-id", "0"],
        vec!["background", "list", "--skill-id", "7", "--skill-id", "7"],
        vec!["background", "list", "--partnered-content", "false"],
        vec!["background", "list", "--page", "1"],
        vec!["background", "list", "--take", "1"],
        vec!["background", "list", "--skip", "0"],
        vec!["background", "list", "--output", "json", "--no-headers"],
        vec![
            "background",
            "show",
            "17",
            "--output",
            "json",
            "--no-headers",
        ],
        vec!["background", "show", "17", "--skill-id", "7"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("session_absent"));
    }
    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        let output = run(&["completions", shell]);
        assert!(output.status.success());
        let text = String::from_utf8_lossy(&output.stdout);
        for expected in ["background", "skill-id", "tag-id", "partnered-content"] {
            assert!(text.contains(expected), "{shell}: {expected}");
        }
    }
}

/// Checks equipment's explicit source context, completions, and pre-session rejection.
#[test]
fn equipment_help_completions_and_invalid_usage() {
    for action in ["list", "show"] {
        let output = run(&["equipment", action, "--help"]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("source page"));
    }
    for args in [
        vec!["equipment"],
        vec!["equipment", "show", "7:17"],
        vec!["equipment", "show", "17", "--page", "1"],
        vec!["equipment", "show", "7:017", "--page", "1"],
        vec!["equipment", "show", "7:17", "--page", "0"],
        vec!["equipment", "list", "--page", "1000001"],
        vec!["equipment", "list", "--name", "invented"],
        vec!["equipment", "list", "--skip", "0"],
        vec!["equipment", "list", "--take", "1"],
        vec!["equipment", "list", "--page-size", "1"],
        vec!["equipment", "list", "-o", "json", "--no-headers"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("session_absent"));
    }
    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        let output = run(&["completions", shell]);
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("equipment"));
    }
}

/// Checks feat help, completions, and invalid inputs before protected session access.
#[test]
fn feat_help_completions_and_invalid_usage() {
    for action in ["list", "show"] {
        let output = run(&["feat", action, "--help"]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("feat"));
    }
    for args in [
        vec!["feat"],
        vec!["feat", "show"],
        vec!["feat", "show", "01"],
        vec!["feat", "show", "0"],
        vec!["feat", "show", "18446744073709551616"],
        vec!["feat", "list", "--name", ""],
        vec!["feat", "list", "--name", " padded"],
        vec!["feat", "list", "--prerequisite-subtype-id", "0"],
        vec![
            "feat",
            "list",
            "--prerequisite-subtype-id",
            "7",
            "--prerequisite-subtype-id",
            "8",
        ],
        vec!["feat", "list", "--tag-id", "7", "--tag-id", "7"],
        vec!["feat", "list", "--partnered-content", "false"],
        vec!["feat", "list", "--page", "1"],
        vec!["feat", "list", "--take", "1"],
        vec!["feat", "list", "--skip", "0"],
        vec!["feat", "list", "--output", "json", "--no-headers"],
        vec!["feat", "show", "17", "--tag-id", "7"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("session_absent"));
    }
    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        let output = run(&["completions", shell]);
        assert!(output.status.success());
        let text = String::from_utf8_lossy(&output.stdout);
        for expected in [
            "feat",
            "prerequisite-subtype-id",
            "tag-id",
            "partnered-content",
        ] {
            assert!(text.contains(expected), "{shell}: {expected}");
        }
    }
}

/// Invalid detail IDs fail before session storage or network access.
#[test]
fn feat_show_rejects_invalid_id_before_session_access() {
    let output = run(&["feat", "show", "01"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("canonical positive decimal integer"));
}

/// Names the preferred output option when rejecting JSON header suppression.
#[test]
fn json_no_headers_error_uses_output_option_name() {
    let output = run(&["character", "list", "--output", "json", "--no-headers"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--no-headers cannot be used with --output json")
    );
}

/// Setup refuses a development directory before any product registry write.
#[test]
fn browser_setup_refuses_the_source_tree() {
    let output = run(&[
        "auth",
        "install",
        "edge",
        "--directory",
        env!("CARGO_MANIFEST_DIR"),
        "--dry-run",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    #[cfg(windows)]
    assert!(error.contains("stable local installation directory"));
    #[cfg(not(windows))]
    assert!(error.contains("only on Windows"));
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
        vec!["auth", "validate", "--help"],
        vec!["auth", "logout", "--help"],
    ] {
        let output = run(&arguments);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: beyond-gate auth"));
    }
    let output = run(&["auth", "--help"]);
    let help = String::from_utf8_lossy(&output.stdout);
    for name in ["login", "status", "logout", "validate"] {
        assert!(help.contains(name));
    }
}

/// Checks redirected login fails before terminal input, storage, or network activity.
#[test]
fn auth_login_reports_the_blocker_without_stdout() {
    let output = run(&["auth", "login"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    #[cfg(windows)]
    assert!(diagnostic.contains("pairing requires interactive stdin, stdout, and stderr"));
    #[cfg(not(windows))]
    {
        assert!(diagnostic.contains("no working session handoff"));
        assert!(diagnostic.contains("do not supply a password or cookie"));
    }
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
        vec!["auth", "validate", "--cookie", "synthetic-only"],
        vec!["auth", "logout", "--all"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}
