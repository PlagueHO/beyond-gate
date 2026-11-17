//! Process-level checks for the public CLI contract.

use std::{
    io::Write,
    process::{Command, Output},
};

/// Runs the compiled binary without a shell.
///
/// Cargo supplies the binary path through `env!`, so the tests do not need PATH
/// changes or an installed copy of Beyond Gate.
///
/// # Panics
///
/// Panics if the operating system cannot start or wait for the test binary.
fn run(arguments: &[&str]) -> Output {
    run_with_environment(arguments, &[])
}

/// Sets color controls only in the child, independent of the test runner's terminal.
///
/// # Panics
///
/// Panics if the operating system cannot start or wait for the test binary.
fn run_with_environment(arguments: &[&str], environment: &[(&str, &str)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_beyond-gate"))
        .args(arguments)
        .env_remove("BEYOND_GATE_COBALT_SESSION")
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("TERM")
        .env_remove("COLORTERM")
        .env_remove("CI")
        .envs(environment.iter().copied())
        .output()
        .expect("the CLI test binary must run")
}

/// Environment presence and logout never require a native store or service.
#[test]
fn environment_status_and_logout_are_process_only_and_redacted() {
    let environment = [("BEYOND_GATE_COBALT_SESSION", "invented-private-cookie")];
    let status = run_with_environment(&["auth", "status"], &environment);
    assert!(status.status.success());
    let text = String::from_utf8_lossy(&status.stdout);
    assert!(text.contains("source=environment"));
    assert!(text.contains("provided-unverified"));
    assert!(!text.contains("invented-private-cookie"));
    let logout = run_with_environment(&["auth", "logout"], &environment);
    assert!(logout.status.success());
    let text = String::from_utf8_lossy(&logout.stdout);
    assert!(text.contains("Saved credentials are unchanged"));
    assert!(text.contains("parent environment"));
    let restarted = run_with_environment(&["auth", "status"], &environment);
    assert!(String::from_utf8_lossy(&restarted.stdout).contains("provided-unverified"));
}

/// Malformed environment input cannot affect help, parsing, or process cleanup.
#[test]
fn invalid_environment_fails_only_when_selected_input_is_needed() {
    let environment = [("BEYOND_GATE_COBALT_SESSION", "invented private sentinel")];
    for args in [
        vec!["--help"],
        vec!["auth", "import", "--help"],
        vec!["--auth-prompt", "auth", "validate", "--help"],
        vec!["completions", "bash"],
    ] {
        assert!(run_with_environment(&args, &environment).status.success());
    }
    for args in [
        vec!["auth", "status"],
        vec!["auth", "validate"],
        vec!["auth", "import", "--env"],
    ] {
        let output = run_with_environment(&args, &environment);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        let category = if args == ["auth", "validate"] && cfg!(target_os = "macos") {
            "unsupported_platform"
        } else {
            "invalid_material"
        };
        assert!(diagnostic.contains(category));
        assert!(!diagnostic.contains("sentinel"));
    }
    assert!(
        run_with_environment(&["auth", "logout"], &environment)
            .status
            .success()
    );
}

/// Prompting and importing cannot be selected accidentally through another command.
#[test]
fn prompt_and_import_usage_is_explicit_and_redirected_input_is_refused() {
    for args in [
        vec!["auth", "import"],
        vec!["auth", "import", "--env", "--prompt"],
        vec!["--auth-prompt", "auth", "status"],
        vec!["--auth-prompt", "auth", "logout"],
        vec!["--auth-prompt", "mcp", "start"],
        vec!["--auth-prompt", "completions", "bash"],
    ] {
        assert_eq!(run(&args).status.code(), Some(2), "{args:?}");
    }
    for args in [
        vec!["--auth-prompt", "auth", "validate"],
        vec!["auth", "import", "--prompt"],
    ] {
        let output = run(&args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("terminal stdin and stderr"));
    }
    let absent = run(&["auth", "import", "--env"]);
    assert_eq!(absent.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&absent.stderr).contains("cookie_unavailable"));
}

/// Rejects invalid prepared requests before a hidden prompt can read terminal input.
#[test]
fn invalid_prompted_requests_preserve_usage_errors_without_terminal_input() {
    let mut cases: Vec<Vec<&str>> = [
        "character",
        "campaign",
        "book",
        "class",
        "subclass",
        "monster",
        "spell",
        "equipment",
        "magic-item",
        "feat",
        "background",
        "species",
    ]
    .into_iter()
    .map(|resource| vec![resource, "list", "--output", "json", "--no-headers"])
    .collect();
    cases.extend([
        vec![
            "book",
            "read",
            "1",
            "section",
            "--continuation",
            "bad-token",
        ],
        vec!["character", "show", "42", "--html-field", "name"],
        vec!["monster", "list", "--cr-min", "10", "--cr-max", "1"],
        vec!["feat", "list", "--tag-id", "1", "--tag-id", "1"],
        vec!["background", "list", "--tag-id", "1", "--tag-id", "1"],
        vec!["spell", "list", "--level", "1", "--level", "1"],
    ]);
    for args in cases {
        let ordinary = run(&args);
        assert_eq!(ordinary.status.code(), Some(2), "{args:?}");
        let mut prompted = vec!["--auth-prompt"];
        prompted.extend_from_slice(&args);
        let prompted = run(&prompted);
        assert_eq!(prompted.status.code(), Some(2), "{args:?}");
        assert!(prompted.stdout.is_empty(), "{args:?}");
        assert_eq!(prompted.stderr, ordinary.stderr, "{args:?}");
    }
}

/// Rejects pure core book selection failures before hidden input or session access.
#[test]
fn invalid_book_selections_fail_before_hidden_input() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let section = format!("s-{}", "a".repeat(64));
    let other_section = format!("s-{}", "b".repeat(64));
    let uppercase_section = format!("s-{}", "A".repeat(64));
    let mut cases = vec![
        vec!["book", "read", "1", "section"],
        vec!["book", "read", "1", "s-abc"],
        vec!["book", "read", "1", &other_section[..65]],
        vec!["book", "read", "1", &uppercase_section],
    ];
    for book in ["0", "01", "18446744073709551616"] {
        cases.push(vec!["book", "outline", book]);
        cases.push(vec!["book", "read", book, &section]);
    }
    let mut tokens = Vec::new();
    for (book, cursor_section) in [("2", &section), ("1", &other_section)] {
        let wire = serde_json::json!({
            "book_id": book,
            "section_id": cursor_section,
            "revision": "invented",
            "source_url": "https://example.com/invented",
            "next_byte": 4096,
        });
        tokens.push(format!("bg1.{}", URL_SAFE_NO_PAD.encode(wire.to_string())));
    }
    for token in &tokens {
        cases.push(vec!["book", "read", "1", &section, "--continuation", token]);
    }
    for arguments in cases {
        let ordinary = run(&arguments);
        assert_eq!(ordinary.status.code(), Some(1), "{arguments:?}");
        let mut prompted = vec!["--auth-prompt"];
        prompted.extend_from_slice(&arguments);
        let prompted = run(&prompted);
        assert_eq!(
            prompted.status.code(),
            ordinary.status.code(),
            "{arguments:?}"
        );
        assert!(prompted.stdout.is_empty());
        assert_eq!(prompted.stderr, ordinary.stderr, "{arguments:?}");
        assert!(!String::from_utf8_lossy(&prompted.stderr).contains("terminal"));
    }
}

/// Checks that the startup artwork precedes unchanged help without changing usage status.
#[test]
fn no_command_shows_plain_logo_before_help() {
    let output = run(&[]);
    let help = run(&["--help"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("⠙⢿⣿⣿⣿"));
    assert!(stderr.contains("⣻⣿⣿⣿⣿⣿⣿⡇"));
    assert!(!stderr.contains('\x1b'));
    assert!(output.stderr.ends_with(&help.stdout));
    let logo_length = output.stderr.len() - help.stdout.len();
    assert!(
        stderr[..logo_length]
            .lines()
            .all(|line| line.chars().count() <= 80)
    );
}

/// Keeps the plain artwork and help identical when color is disabled or unsupported.
#[test]
fn no_command_color_disabled_preserves_plain_output() {
    let plain = run(&[]);
    for environment in [
        vec![("NO_COLOR", "1")],
        vec![("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")],
        vec![("CLICOLOR", "0")],
        vec![("TERM", "dumb")],
    ] {
        let output = run_with_environment(&[], &environment);
        assert_eq!(output.status.code(), Some(2), "{environment:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, plain.stderr, "{environment:?}");
    }
}

/// Forces white and red in a pipe and resets both styles before the existing help.
#[test]
fn no_command_forced_color_changes_only_logo_styles() {
    let plain = run(&[]);
    let output = run_with_environment(&[], &[("CLICOLOR_FORCE", "1")]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(stderr.starts_with("\x1b[37m⠙⢿⣿⣿⣿"));
    assert!(stderr.contains("\x1b[0m\n\x1b[31m"));
    assert_eq!(stderr.matches("\x1b[0m").count(), 2);
    let (_, help) = stderr.rsplit_once("\x1b[0m\n").expect("reset before help");
    assert_eq!(help.as_bytes(), run(&["--help"]).stdout);
    let mut stripped = anstream::AutoStream::never(Vec::new());
    stripped
        .write_all(&output.stderr)
        .expect("strip logo styles");
    assert_eq!(stripped.into_inner(), plain.stderr);
}

/// Does not attach the startup logo to explicit requests or incomplete resource commands.
#[test]
fn logo_is_exclusive_to_no_argument_startup() {
    for arguments in [
        vec!["--help"],
        vec!["-h"],
        vec!["help"],
        vec!["--version"],
        vec!["book"],
        vec!["auth"],
        vec!["unknown"],
        vec!["--unknown"],
    ] {
        let output = run_with_environment(&arguments, &[("CLICOLOR_FORCE", "1")]);
        for bytes in [&output.stdout, &output.stderr] {
            let text = String::from_utf8_lossy(bytes);
            assert!(!text.contains('⣿'), "{arguments:?}");
            assert!(!text.contains('\x1b'), "{arguments:?}");
        }
    }
}

/// Checks that root help explains authentication and the first protected read.
#[test]
fn help_reports_basic_authentication_usage() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: beyond-gate"));
    assert!(stdout.contains("Commands:"));
    assert!(stdout.contains("Options:"));
    assert!(stdout.contains("Authentication and getting started"));
    assert!(stdout.contains("Read your D&D Beyond characters, campaigns, books, and game content"));
    assert!(!stdout.contains("Local-first access to authorized D&D Beyond data"));
    assert!(stdout.contains("BEYOND_GATE_COBALT_SESSION"));
    assert!(stdout.contains("Invalid selected input fails without fallback"));
    assert!(stdout.contains("does not unlock"));
    assert!(stdout.contains("Character and campaign reads use fixed bounded sources"));
    assert!(stdout.contains("beyond-gate auth install edge"));
    assert!(stdout.contains("beyond-gate auth login"));
    assert!(stdout.contains("beyond-gate auth validate"));
    assert!(stdout.contains("beyond-gate character list"));
    assert!(stdout.contains("operating system's protected storage"));
    assert!(stdout.contains("Do not enter your password"));
    assert!(!stdout.contains(beyond_gate_core::FOUNDATION_NOTICE));
    assert!(
        stdout.lines().all(|line| line.chars().count() <= 100),
        "root help must fit within 100 columns"
    );
}

/// Reports book access and the shared authentication requirement.
#[test]
fn book_help_reports_web_catalog_scope() {
    let output = run(&["book", "--help"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Find books, browse their sections, and read selected text"));
    assert!(stdout.contains("Requires selected Beyond Gate session input"));
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

/// Checks that unknown operations cannot return fake success.
#[test]
fn unknown_operation_fails() {
    for arguments in [vec!["unknown", "list"], vec!["--unknown"]] {
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
        vec!["--help"],
        vec!["help", "auth"],
        vec!["auth", "--help"],
        vec!["auth", "login", "--help"],
        vec!["auth", "status", "--help"],
        vec!["auth", "validate", "--help"],
        vec!["auth", "logout", "--help"],
        vec!["auth", "install", "--help"],
        vec!["auth", "uninstall", "--help"],
        vec!["character", "--help"],
        vec!["character", "list", "--help"],
        vec!["character", "show", "--help"],
        vec!["campaign", "--help"],
        vec!["campaign", "list", "--help"],
        vec!["campaign", "show", "--help"],
        vec!["class", "--help"],
        vec!["class", "list", "--help"],
        vec!["class", "show", "--help"],
        vec!["subclass", "--help"],
        vec!["subclass", "list", "--help"],
        vec!["subclass", "show", "--help"],
        vec!["monster", "--help"],
        vec!["monster", "list", "--help"],
        vec!["monster", "show", "--help"],
        vec!["spell", "--help"],
        vec!["spell", "list", "--help"],
        vec!["spell", "show", "--help"],
        vec!["equipment", "--help"],
        vec!["equipment", "list", "--help"],
        vec!["equipment", "show", "--help"],
        vec!["magic-item", "--help"],
        vec!["magic-item", "list", "--help"],
        vec!["magic-item", "show", "--help"],
        vec!["feat", "--help"],
        vec!["feat", "list", "--help"],
        vec!["feat", "show", "--help"],
        vec!["background", "--help"],
        vec!["background", "list", "--help"],
        vec!["background", "show", "--help"],
        vec!["species", "--help"],
        vec!["species", "list", "--help"],
        vec!["species", "show", "--help"],
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
        assert!(
            output.status.success(),
            "{arguments:?}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(help.contains("Usage: beyond-gate"), "{arguments:?}");
        assert!(!help.trim().is_empty(), "{arguments:?}");
        let summary = help
            .lines()
            .find(|line| !line.trim().is_empty())
            .expect("help summary");
        let summary_position = help.find(summary).expect("summary position");
        let usage_position = help.find("Usage:").expect("usage position");
        assert!(summary_position < usage_position, "{arguments:?}\n{help}");
        for line in help.lines() {
            assert!(
                line.chars().count() <= 100,
                "{arguments:?} has an overlong line: {line}"
            );
        }
    }
    for arguments in [
        vec!["character", "list", "--help"],
        vec!["character", "show", "--help"],
        vec!["campaign", "list", "--help"],
        vec!["campaign", "show", "--help"],
        vec!["class", "list", "--help"],
        vec!["class", "show", "--help"],
        vec!["subclass", "list", "--help"],
        vec!["subclass", "show", "--help"],
        vec!["monster", "list", "--help"],
        vec!["monster", "show", "--help"],
        vec!["spell", "list", "--help"],
        vec!["spell", "show", "--help"],
        vec!["equipment", "list", "--help"],
        vec!["equipment", "show", "--help"],
        vec!["magic-item", "list", "--help"],
        vec!["magic-item", "show", "--help"],
        vec!["feat", "list", "--help"],
        vec!["feat", "show", "--help"],
        vec!["background", "list", "--help"],
        vec!["background", "show", "--help"],
        vec!["species", "list", "--help"],
        vec!["species", "show", "--help"],
        vec!["book", "list", "--help"],
        vec!["book", "outline", "--help"],
        vec!["book", "read", "--help"],
    ] {
        let output = run(&arguments);
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{arguments:?}");
        assert!(
            help.contains("Requires selected Beyond Gate session input"),
            "{arguments:?}\n{help}"
        );
        let usage = help.find("Usage:").expect("action usage");
        let details = help
            .find("Requires selected Beyond Gate session input")
            .expect("session details");
        assert!(usage < details, "{arguments:?}\n{help}");
    }
    let output = run(&["book", "read", "--help"]);
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("4,096 bytes of text"));
    assert!(help.contains("--continuation"));
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
fn class_help_and_invalid_usage() {
    for resource in ["class", "subclass"] {
        for action in ["list", "show"] {
            let output = run(&[resource, action, "--help"]);
            assert!(output.status.success());
            assert!(output.stderr.is_empty());
            let help = String::from_utf8_lossy(&output.stdout);
            if action == "list" {
                assert!(help.contains("names and IDs"));
            } else {
                assert!(help.contains("names and source details"));
            }
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
        vec!["spell", "list", "--name", " padded"],
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
        if args == ["spell", "list", "--name", " padded"] {
            assert!(String::from_utf8_lossy(&output.stderr).contains("Spell name must"));
        }
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
        assert!(String::from_utf8_lossy(&output.stdout).contains("Page number"));
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

/// Checks magic-item help, completions, and strict local selectors before session access.
#[test]
fn magic_item_help_completions_and_invalid_usage() {
    for action in ["list", "show"] {
        let output = run(&["magic-item", action, "--help"]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(help.contains("Page number"));
        if action == "list" {
            assert!(help.contains("Match an exact item type label"));
            assert!(help.contains("do not search other pages"));
        }
    }
    for args in [
        vec!["magic-item"],
        vec!["magic-item", "show", "7:18"],
        vec!["magic-item", "show", "18", "--page", "1"],
        vec!["magic-item", "show", "7:018", "--page", "1"],
        vec!["magic-item", "list", "--page", "0"],
        vec!["magic-item", "list", "--name", ""],
        vec!["magic-item", "list", "--tag", " padded"],
        vec!["magic-item", "list", "--type", "Weapon", "--type", "weapon"],
        vec!["magic-item", "list", "--attunement", "true"],
        vec!["magic-item", "list", "--source-id", "7"],
        vec!["magic-item", "list", "-o", "json", "--no-headers"],
        vec![
            "magic-item",
            "show",
            "7:18",
            "--page",
            "1",
            "--tag",
            "Utility",
        ],
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
        for expected in ["magic-item", "attunement", "rarity", "tag"] {
            assert!(text.contains(expected), "{shell}: {expected}");
        }
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

/// Explains readable defaults and rejects unsupported formats before source access.
#[test]
fn readable_formats_and_html_selection_are_explicit() {
    for resource in [
        "character",
        "campaign",
        "class",
        "subclass",
        "monster",
        "species",
        "background",
        "feat",
        "equipment",
        "magic-item",
        "spell",
    ] {
        let output = run(&[resource, "show", "--help"]);
        assert!(output.status.success());
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(help.contains("default: text"), "{resource}: {help}");
        assert!(help.contains("markdown"), "{resource}: {help}");
    }
    for arguments in [
        vec![
            "character",
            "show",
            "hero",
            "--output",
            "markdown",
            "--no-headers",
        ],
        vec![
            "character",
            "show",
            "hero",
            "--output",
            "html",
            "--html-field",
            "raw.any_url",
        ],
        vec!["character", "list", "--output", "text"],
        vec!["spell", "list", "--output", "html"],
        vec![
            "spell",
            "show",
            "17-invented",
            "--html-field",
            "raw.url",
            "-o",
            "html",
        ],
        vec![
            "spell",
            "show",
            "17-invented",
            "--html-field",
            "source_url",
            "-o",
            "json",
        ],
        vec!["spell", "show", "17-invented", "-o", "html", "--no-headers"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(output.stdout.is_empty());
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(!diagnostic.contains("session_absent"), "{diagnostic}");
    }
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
    let summary = help
        .find("Sign in through your browser and manage the saved sign-in")
        .expect("auth summary");
    let usage = help.find("Usage: beyond-gate auth").expect("auth usage");
    let options = help.find("Options:").expect("auth options");
    let details = help.find("Sign-in works on Windows").expect("auth details");
    assert!(summary < usage);
    assert!(options < details);
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
