//! Process-level checks for the local MCP stdio protocol.

use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

use serde_json::{Value, json};

/// Runs the MCP binary for a command-line help or version request.
///
/// # Panics
///
/// Panics if the operating system cannot start or wait for the test binary.
fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_beyond-gate-mcp"))
        .env_remove("BEYOND_GATE_COBALT_SESSION")
        .args(arguments)
        .output()
        .expect("the MCP test binary must run")
}

/// Runs JSON-RPC messages through the actual stdio process and closes its input.
///
/// # Panics
///
/// Panics if the process cannot start, the input cannot be written, or it cannot
/// be waited on.
fn run_protocol(messages: impl IntoIterator<Item = Value>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_beyond-gate-mcp"));
    command.env_remove("BEYOND_GATE_COBALT_SESSION");
    run_protocol_command(command, messages)
}

/// Sends protocol input to an owned command, including a real sibling launcher.
fn run_protocol_command(mut command: Command, messages: impl IntoIterator<Item = Value>) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the MCP server process must start");
    let mut stdin = child.stdin.take().expect("the child stdin is piped");
    for message in messages {
        let line = serde_json::to_vec(&message).expect("protocol messages are JSON");
        stdin
            .write_all(&line)
            .and_then(|()| stdin.write_all(b"\n"))
            .expect("the MCP server must accept protocol input");
    }
    drop(stdin);
    child
        .wait_with_output()
        .expect("the MCP server process must exit")
}

/// Proves product startup selects environment input without breaking discovery.
#[test]
fn malformed_environment_is_lazy_for_direct_and_sibling_startup() {
    let direct = std::path::Path::new(env!("CARGO_BIN_EXE_beyond-gate-mcp"));
    let cli = direct.with_file_name(if cfg!(windows) {
        "beyond-gate.exe"
    } else {
        "beyond-gate"
    });
    for sibling in [false, true] {
        let mut command = Command::new(if sibling { cli.as_path() } else { direct });
        if sibling {
            command.args(["mcp", "start"]);
        }
        command.env("BEYOND_GATE_COBALT_SESSION", "invented invalid sentinel");
        let output = run_protocol_command(
            command,
            [
                json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
                "protocolVersion":"2026-07-28","capabilities":{},
                "clientInfo":{"name":"environment-test","version":"1"}}}),
                json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
                json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
                json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
                "name":"auth_validate","arguments":{}}}),
            ],
        );
        assert!(
            output.status.success(),
            "sibling={sibling}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let messages = protocol_output(&output);
        assert!(
            messages
                .iter()
                .any(|message| message["id"] == 2 && message["result"]["tools"].is_array())
        );
        let result = &messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("validation result")["result"];
        assert_eq!(result["isError"], true);
        let text = result["content"][0]["text"].as_str().expect("safe error");
        let category = if cfg!(target_os = "macos") {
            "unsupported_platform"
        } else {
            "invalid_material"
        };
        assert!(text.starts_with(category), "{text}");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("sentinel"));
    }
}

/// Parses every stdout line as one protocol message.
///
/// # Panics
///
/// Panics if protocol output is not valid UTF-8 JSON Lines.
fn protocol_output(output: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout contains only protocol JSON"))
        .collect()
}

/// Checks that help describes the local stdio server and shared read scope.
#[test]
fn help_reports_local_server_scope() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Local read-only MCP server for Beyond Gate"));
    assert!(stdout.contains("Character, campaign, and book reads"));
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

/// Checks initialization, discovery, and invalid tool input without live reads.
#[test]
fn stdio_protocol_handles_discovery_and_strict_input() {
    let output = run_protocol([
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2026-07-28",
                "capabilities": {},
                "clientInfo": {"name": "beyond-gate-test", "version": "0.1.0"}
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {"name": "character_show", "arguments": {"id": "padded "}            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "book_read",
                "arguments": {
                    "book_id": "invented-book",
                    "section_id": "opening",
                    "continuation": "not-a-token"
                }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "character_show",
                "arguments": {"id": "valid", "extra": true}
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {"name": "book_list", "arguments": {"unexpected": true}}
        }),
    ]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let responses = protocol_output(&output);
    assert!(responses.iter().all(|message| message["jsonrpc"] == "2.0"));

    let initialized = responses
        .iter()
        .find(|message| message["id"] == 1)
        .expect("initialize response");
    assert!(initialized["result"]["capabilities"]["tools"].is_object());
    let instructions = initialized["result"]["instructions"]
        .as_str()
        .expect("server instructions");
    assert!(instructions.contains("Web-catalog metadata"));
    assert!(!instructions.contains("blocked"));

    let listed = responses
        .iter()
        .find(|message| message["id"] == 2)
        .expect("tool list response");
    let tools = listed["result"]["tools"]
        .as_array()
        .expect("tool list is an array");
    let mut names: Vec<_> = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "auth_validate",
            "background_list",
            "background_show",
            "book_list",
            "book_outline",
            "book_read",
            "campaign_list",
            "campaign_show",
            "character_list",
            "character_show",
            "class_list",
            "class_show",
            "equipment_list",
            "equipment_show",
            "feat_list",
            "feat_show",
            "magic_item_list",
            "magic_item_show",
            "monster_list",
            "monster_show",
            "species_list",
            "species_show",
            "spell_list",
            "spell_show"
        ]
    );
    for tool in tools {
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["annotations"]["destructiveHint"], false);
        assert_eq!(tool["annotations"]["openWorldHint"], true);
        assert_eq!(
            tool["annotations"]["idempotentHint"],
            tool["name"] != "auth_validate"
        );
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
    }
    let book_read = tools
        .iter()
        .find(|tool| tool["name"] == "book_read")
        .expect("book read tool");
    assert_eq!(
        book_read["inputSchema"]["properties"]["continuation"]["maxLength"],
        16_384
    );
    assert_eq!(
        book_read["inputSchema"]["properties"]["book_id"]["minLength"],
        1
    );
    assert_eq!(book_read["inputSchema"]["additionalProperties"], false);
    let campaign_list = tools
        .iter()
        .find(|tool| tool["name"] == "campaign_list")
        .expect("campaign list tool");
    assert_eq!(
        campaign_list["inputSchema"]["properties"]["active_only"]["type"],
        "boolean"
    );
    assert_eq!(campaign_list["inputSchema"]["additionalProperties"], false);

    let invalid = responses
        .iter()
        .find(|message| message["id"] == 4)
        .expect("invalid tool input response");
    assert_eq!(invalid["result"]["isError"], true);
    let bad_cursor = responses
        .iter()
        .find(|message| message["id"] == 5)
        .expect("continuation response");
    assert_eq!(bad_cursor["result"]["isError"], true);
    assert!(
        !bad_cursor["result"]["content"][0]["text"]
            .as_str()
            .expect("safe continuation error")
            .contains("not-a-token")
    );
    let extra_field = responses
        .iter()
        .find(|message| message["id"] == 6)
        .expect("unknown argument response");
    assert_eq!(extra_field["result"]["isError"], true);
    let strict_library = responses
        .iter()
        .find(|message| message["id"] == 9)
        .expect("strict library input");
    assert_eq!(strict_library["result"]["isError"], true);
}

/// Keeps unsupported command-line arguments out of the server invocation.
#[test]
fn unknown_arguments_fail_without_protocol_output() {
    let output = run(&["--transport", "stdio"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
