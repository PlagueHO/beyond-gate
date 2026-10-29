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
    let mut child = Command::new(env!("CARGO_BIN_EXE_beyond-gate-mcp"))
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

/// Checks that help describes stdio serving without implying live reads.
#[test]
fn help_reports_local_server_scope() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Local read-only MCP server for Beyond Gate"));
    assert!(stdout.contains("Live reads are not implemented"));
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

/// Checks initialization, discovery, validation, tool errors, cancellation, and EOF.
#[test]
fn stdio_protocol_handles_read_tools_and_lifecycle() {
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
            "id": 3,
            "method": "tools/call",
            "params": {"name": "character_list", "arguments": {}}
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": {"requestId": 3, "reason": "synthetic cancellation"}
        }),
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
            "id": 7,
            "method": "tools/call",
            "params": {"name": "campaign_list", "arguments": {}}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {"name": "campaign_show", "arguments": {"id": "campaign"}}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {"name": "book_list", "arguments": {}}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {"name": "book_outline", "arguments": {"book_id": "book"}}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "book_read",
                "arguments": {"book_id": "book", "section_id": "section"}
            }
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
            "book_list",
            "book_outline",
            "book_read",
            "campaign_list",
            "campaign_show",
            "character_list",
            "character_show"
        ]
    );
    for tool in tools {
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["annotations"]["destructiveHint"], false);
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
    for id in [7, 8, 9, 10, 11] {
        let response = responses
            .iter()
            .find(|message| message["id"] == id)
            .expect("read tool response");
        assert_eq!(response["result"]["isError"], true);
        assert!(
            response["result"]["content"][0]["text"]
                .as_str()
                .expect("tool error text")
                .contains("not implemented")
        );
    }
}

/// Keeps unsupported command-line arguments out of the server invocation.
#[test]
fn unknown_arguments_fail_without_protocol_output() {
    let output = run(&["--transport", "stdio"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
