//! Self-spawning CLI/MCP parity checks with actual shared synthetic session exchanges.
//!
//! Fixture selectors belong only to this test executable, never product commands.

use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Output, Stdio},
    sync::mpsc,
    time::Duration,
};

use beyond_gate_core::auth::session::fixture::{self, Scenario};
use beyond_gate_core::{auth::session::Session, library::fixture as library_fixture};
use rmcp::{ServiceExt, transport::stdio};
use serde_json::{Value, json};

/// Maps fixed non-secret test labels to compiled invented scenarios.
fn scenario(name: &str) -> Scenario {
    match name {
        "absent" => Scenario::Absent,
        "rejected" => Scenario::Rejected,
        "expired" => Scenario::Expired,
        "invalid" => Scenario::Invalid,
        "profile" => Scenario::UnknownProfile,
        "locked" => Scenario::Locked,
        "unavailable" => Scenario::Unavailable,
        "ambiguous" => Scenario::Ambiguous,
        "malformed" => Scenario::Malformed,
        "rate-limited" => Scenario::RateLimited,
        "waiting" => Scenario::Waiting,
        "valid" => Scenario::Valid,
        _ => panic!("unknown synthetic scenario"),
    }
}

/// Selects fixed invented source cases only in this self-spawning test executable.
fn session(name: &str) -> Session {
    let source = match name {
        "library-normal" => Some(library_fixture::Scenario::Normal),
        "library-lost" => Some(library_fixture::Scenario::SharingLost),
        "library-empty" => Some(library_fixture::Scenario::Empty),
        "library-absent" => Some(library_fixture::Scenario::Absent),
        "library-rejected" => Some(library_fixture::Scenario::Rejected),
        "library-expired" => Some(library_fixture::Scenario::Expired),
        "library-waiting" => Some(library_fixture::Scenario::Waiting),
        "library-malformed" => Some(library_fixture::Scenario::Malformed),
        "library-incomplete" => Some(library_fixture::Scenario::Incomplete),
        "library-oversized" => Some(library_fixture::Scenario::Oversized),
        "library-denied" => Some(library_fixture::Scenario::Denied),
        "library-rate-limited" => Some(library_fixture::Scenario::RateLimited),
        "library-invalid-config" => Some(library_fixture::Scenario::InvalidConfiguration),
        "library-invalid-title" => Some(library_fixture::Scenario::InvalidTitle),
        "library-hidden-evidence" => Some(library_fixture::Scenario::InvalidHiddenEvidence),
        "library-unmatched-ownership" => Some(library_fixture::Scenario::UnmatchedOwnership),
        "library-outside-web-catalog" => Some(library_fixture::Scenario::OutsideWebCatalog),
        "library-invalid-outside-evidence" => {
            Some(library_fixture::Scenario::InvalidOutsideEvidence)
        }
        "book-redirect" => Some(library_fixture::Scenario::BookRedirect),
        "book-rejected" => Some(library_fixture::Scenario::BookRejected),
        "book-denied" => Some(library_fixture::Scenario::BookDenied),
        "book-preview" => Some(library_fixture::Scenario::BookPreview),
        "book-changed" => Some(library_fixture::Scenario::BookChanged),
        "book-moved" => Some(library_fixture::Scenario::BookMoved),
        "book-malformed" => Some(library_fixture::Scenario::BookMalformed),
        "book-oversized" => Some(library_fixture::Scenario::BookOversized),
        "book-waiting" => Some(library_fixture::Scenario::BookWaiting),
        "book-wrong-media" => Some(library_fixture::Scenario::BookWrongMedia),
        _ => None,
    };
    match source {
        Some(source) => library_fixture::session(source).expect("invented library services"),
        None => fixture::session(scenario(name)).expect("invented auth service"),
    }
}

/// Runs an actual CLI parser/runner in a separate process using the invented core owner.
fn cli(name: &str, arguments: &[&str]) -> Output {
    Command::new(std::env::current_exe().expect("test executable"))
        .args(["--cli-child", name])
        .args(arguments)
        .output()
        .expect("CLI child")
}

/// Exchanges protocol messages with the actual rmcp stdio server until validation completes.
fn mcp(name: &str, arguments: Value, cancel: bool) -> (Output, Vec<Value>) {
    mcp_tool(name, "auth_validate", arguments, cancel)
}

/// Calls an actual registered tool while preserving protocol-only stdout and bounded waits.
fn mcp_tool(name: &str, tool: &str, arguments: Value, cancel: bool) -> (Output, Vec<Value>) {
    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--mcp-child", name])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("MCP child");
    let mut stdin = child.stdin.take().expect("stdin");
    let reader = BufReader::new(child.stdout.take().expect("stdout"));
    let (send, receive) = mpsc::channel();
    let reader_worker = std::thread::spawn(move || {
        for line in reader.lines() {
            let message: Value = serde_json::from_str(&line.expect("protocol output"))
                .expect("protocol-only stdout");
            if send.send(message).is_err() {
                break;
            }
        }
    });
    for message in [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2026-07-28","capabilities":{},
            "clientInfo":{"name":"session-fixture","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
            "name":tool,"arguments":arguments}}),
    ] {
        writeln!(stdin, "{message}").expect("protocol input");
    }
    if cancel {
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":3}})
        )
        .expect("cancel notification");
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc":"2.0","id":4,"method":"tools/list","params":{}})
        )
        .expect("post-cancel request");
    }
    stdin.flush().expect("flush");
    let mut messages = Vec::new();
    loop {
        let message = match receive.recv_timeout(Duration::from_secs(10)) {
            Ok(message) => message,
            Err(error) => {
                child.kill().expect("stop this stalled test child");
                let output = child.wait_with_output().expect("reap test child");
                panic!(
                    "bounded MCP response wait failed ({name}/{tool}): {error}; {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        };
        let done = message["id"] == if cancel { 4 } else { 3 };
        messages.push(message);
        if done {
            break;
        }
    }
    drop(stdin);
    loop {
        match receive.recv_timeout(Duration::from_secs(10)) {
            Ok(message) => messages.push(message),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(error) => {
                child.kill().expect("stop this stalled test child");
                child.wait().expect("reap test child");
                panic!("bounded MCP EOF wait failed: {error}");
            }
        }
    }
    reader_worker.join().expect("protocol reader");
    let output = child.wait_with_output().expect("MCP EOF");
    (output, messages)
}

/// Checks real process success/failure, strict empty inputs, and equal adapter results.
fn contracts() {
    for (name, category) in [
        ("valid", None),
        ("absent", Some("session_absent")),
        ("rejected", Some("session_rejected")),
        ("expired", Some("session_expired")),
        ("invalid", Some("invalid_material")),
        ("profile", Some("unsupported_contract")),
        ("locked", Some("storage_access")),
        ("unavailable", Some("storage_unavailable")),
        ("ambiguous", Some("ambiguous_entry")),
        ("malformed", Some("validation_unavailable")),
        ("rate-limited", Some("validation_unavailable")),
    ] {
        let cli = cli(name, &["auth", "validate"]);
        let (output, messages) = mcp(name, json!({}), false);
        assert!(output.status.success());
        assert!(
            output.stderr.is_empty(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result = &messages
            .iter()
            .find(|value| value["id"] == 3)
            .expect("result")["result"];
        let text = result["content"][0]["text"].as_str().expect("result text");
        if let Some(category) = category {
            assert_eq!(cli.status.code(), Some(1));
            assert!(cli.stdout.is_empty());
            assert_eq!(result["isError"], true);
            assert!(text.starts_with(category));
            assert_eq!(
                String::from_utf8_lossy(&cli.stderr).trim(),
                format!("error: {text}")
            );
        } else {
            assert!(
                cli.status.success(),
                "{name}: {}",
                String::from_utf8_lossy(&cli.stderr)
            );
            assert!(cli.stderr.is_empty());
            let data: Value = serde_json::from_str(text).expect("MCP safe data");
            assert_eq!(
                data,
                serde_json::from_slice::<Value>(&cli.stdout).expect("CLI data")
            );
            assert_eq!(
                data,
                json!({"validation":"service_validated","expiry":"unknown","bearer_expiry":"future"})
            );
        }
        for private in ["invented-session", "invented-private", "c3ludGhldGlj"] {
            assert!(!text.contains(private));
            assert!(!String::from_utf8_lossy(&cli.stderr).contains(private));
            assert!(!String::from_utf8_lossy(&cli.stdout).contains(private));
        }
    }
    let (_, messages) = mcp("valid", json!({"unexpected":true}), false);
    assert_eq!(
        messages
            .iter()
            .find(|value| value["id"] == 3)
            .expect("input rejected")["result"]["isError"],
        true
    );
    let listed = &messages
        .iter()
        .find(|value| value["id"] == 2)
        .expect("list")["result"]["tools"];
    let tool = listed
        .as_array()
        .expect("tools")
        .iter()
        .find(|tool| tool["name"] == "auth_validate")
        .expect("auth tool");
    assert_eq!(tool["inputSchema"]["additionalProperties"], false);
    assert!(
        tool["inputSchema"]["properties"]
            .as_object()
            .is_none_or(|properties| properties.is_empty())
    );
    assert_eq!(
        cli("valid", &["auth", "validate", "--cookie", "invented"])
            .status
            .code(),
        Some(2)
    );
    let logout = cli("valid", &["auth", "logout"]);
    assert!(logout.status.success());
    assert!(
        String::from_utf8_lossy(&logout.stdout).contains("Upstream revocation is not implemented")
    );
    let (output, messages) = mcp("waiting", json!({}), true);
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "waiting: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if let Some(message) = messages.iter().find(|value| value["id"] == 3) {
        assert_eq!(message["result"]["isError"], true);
        assert_eq!(message["result"]["content"][0]["text"], "cancelled");
    }
    assert!(
        messages
            .iter()
            .any(|value| value["id"] == 4 && value["result"]["tools"].is_array())
    );
}

/// Checks library transport, complete output, evidence parity, strict inputs, and error streams.
fn library_contracts() {
    for name in [
        "library-normal",
        "library-lost",
        "library-empty",
        "library-absent",
        "library-rejected",
        "library-expired",
        "library-malformed",
        "library-incomplete",
        "library-oversized",
        "library-denied",
        "library-rate-limited",
        "library-invalid-config",
        "library-invalid-title",
        "library-hidden-evidence",
        "library-unmatched-ownership",
        "library-outside-web-catalog",
        "library-invalid-outside-evidence",
    ] {
        let cli = cli(name, &["book", "list", "--format", "json"]);
        let (output, messages) = mcp_tool(name, "book_list", json!({}), false);
        assert!(output.status.success());
        assert!(
            output.stderr.is_empty(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result = &messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("library result")["result"];
        let text = result["content"][0]["text"]
            .as_str()
            .expect("library result text");
        if matches!(
            name,
            "library-normal" | "library-lost" | "library-empty" | "library-outside-web-catalog"
        ) {
            assert!(
                cli.status.success(),
                "{name}: {}",
                String::from_utf8_lossy(&cli.stderr)
            );
            assert!(cli.stderr.is_empty());
            let data: Value = serde_json::from_str(text).expect("MCP complete data");
            assert_eq!(
                data,
                serde_json::from_slice::<Value>(&cli.stdout).expect("CLI complete data")
            );
            match name {
                "library-normal" | "library-outside-web-catalog" => {
                    assert_eq!(data.as_array().expect("books").len(), 5);
                    assert_eq!(data[0]["title"], "Invented volume 1");
                    assert!(
                        data.as_array()
                            .expect("books")
                            .iter()
                            .all(|book| !matches!(book["id"].as_str(), Some("6" | "7" | "8")))
                    );
                    assert_eq!(data[0]["access"], "purchased");
                    assert_eq!(data[1]["access"], "shared");
                    assert_eq!(data[2]["access"], "purchased_and_shared");
                    assert_eq!(data[3]["access"], "unknown");
                    assert_eq!(data[4]["access"], "unknown");
                    assert_eq!(
                        data[1]["evidence"],
                        json!([{"kind":"shared_with_account","state":"granted"}])
                    );
                }
                "library-lost" => {
                    assert_eq!(data[0]["access"], "purchased");
                    assert_eq!(data[1]["access"], "unknown");
                }
                _ => assert_eq!(data, json!([])),
            }
        } else {
            assert_eq!(cli.status.code(), Some(1));
            assert!(cli.stdout.is_empty());
            assert_eq!(result["isError"], true);
            if name == "library-unmatched-ownership" {
                assert_eq!(text, "Library ownership has no matching source metadata.");
            }
            if name == "library-invalid-outside-evidence" {
                assert_eq!(text, "Library access evidence is invalid.");
            }
            assert_eq!(
                String::from_utf8_lossy(&cli.stderr).trim(),
                format!("error: {text}")
            );
        }
        for private in ["invented-session", "invented-private", "c3ludGhldGlj"] {
            assert!(!text.contains(private));
            assert!(!String::from_utf8_lossy(&cli.stderr).contains(private));
        }
    }
    let table = cli("library-normal", &["book", "list", "--format", "tsv"]);
    assert!(table.status.success());
    assert!(table.stderr.is_empty());
    let table = String::from_utf8_lossy(&table.stdout);
    assert!(table.starts_with("id\ttitle\taccess\tsource_url\n"));
    assert!(table.contains(
        "2\tInvented volume 2\tshared\thttps://www.dndbeyond.com/sources/dnd/invented-2"
    ));
    let table = cli("library-normal", &["book", "list"]);
    assert!(table.status.success());
    assert!(!table.stdout.contains(&b'\t'));
    assert_eq!(
        String::from_utf8_lossy(&table.stdout)
            .lines()
            .next()
            .expect("table header")
            .split_whitespace()
            .collect::<Vec<_>>(),
        ["id", "title", "access", "source_url"]
    );
    let (output, messages) = mcp_tool(
        "library-normal",
        "book_list",
        json!({"url":"https://foreign.invalid"}),
        false,
    );
    assert!(output.status.success());
    assert_eq!(
        messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("input failure")["result"]["isError"],
        true
    );
    let (output, messages) = mcp_tool("library-waiting", "book_list", json!({}), true);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    if let Some(message) = messages.iter().find(|message| message["id"] == 3) {
        assert_eq!(message["result"]["isError"], true);
        assert_eq!(message["result"]["content"][0]["text"], "cancelled");
    }
}

/// Extracts one actual tool result, checking process streams before interpreting its data.
fn tool_result(output: &Output, messages: &[Value]) -> Value {
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    messages
        .iter()
        .find(|message| message["id"] == 3)
        .expect("tool result")["result"]
        .clone()
}

/// Checks real CLI/MCP source parsing, bounded continuation, access loss, and safe failures.
fn book_contracts() {
    let arguments = ["book", "outline", "1", "--format", "json"];
    let outline = cli("library-normal", &arguments);
    assert!(
        outline.status.success(),
        "{}",
        String::from_utf8_lossy(&outline.stderr)
    );
    assert!(outline.stderr.is_empty());
    let outline: Value = serde_json::from_slice(&outline.stdout).expect("CLI outline");
    let (output, messages) = mcp_tool(
        "library-normal",
        "book_outline",
        json!({"book_id":"1"}),
        false,
    );
    let result = tool_result(&output, &messages);
    assert_eq!(
        serde_json::from_str::<Value>(result["content"][0]["text"].as_str().expect("text"))
            .expect("MCP outline"),
        outline
    );
    let id = outline["entries"][1]["id"]
        .as_str()
        .expect("opaque section ID");
    assert_eq!(outline["entries"].as_array().expect("entries").len(), 3);
    let mut token: Option<String> = None;
    let mut complete_text = String::new();
    for index in 0..10 {
        let mut arguments = vec!["book", "read", "1", id];
        if let Some(token) = &token {
            arguments.extend(["--continuation", token]);
        }
        let cli = cli("library-normal", &arguments);
        assert!(
            cli.status.success(),
            "{}",
            String::from_utf8_lossy(&cli.stderr)
        );
        assert!(cli.stderr.is_empty());
        let cli: Value = serde_json::from_slice(&cli.stdout).expect("CLI section");
        let mut arguments = json!({"book_id":"1","section_id":id});
        if let Some(token) = &token {
            arguments["continuation"] = json!(token);
        }
        let (output, messages) = mcp_tool("library-normal", "book_read", arguments, false);
        let result = tool_result(&output, &messages);
        let mcp: Value = serde_json::from_str(result["content"][0]["text"].as_str().expect("text"))
            .expect("MCP section");
        assert_eq!(cli, mcp);
        let text = cli["text"].as_str().expect("section text");
        assert!(text.len() <= 4096);
        complete_text.push_str(text);
        token = cli["next_cursor"].as_str().map(str::to_owned);
        if index == 0 {
            let token = token.as_deref().expect("nonterminal chunk");
            assert!(token.len() <= 16384);
            for name in [
                "book-changed",
                "book-moved",
                "book-denied",
                "book-preview",
                "book-malformed",
            ] {
                let cli = self::cli(name, &["book", "read", "1", id, "--continuation", token]);
                let (output, messages) = mcp_tool(
                    name,
                    "book_read",
                    json!({"book_id":"1","section_id":id,"continuation":token}),
                    false,
                );
                let result = tool_result(&output, &messages);
                assert_eq!(cli.status.code(), Some(1));
                assert!(cli.stdout.is_empty());
                assert_eq!(result["isError"], true);
                assert_eq!(
                    String::from_utf8_lossy(&cli.stderr).trim(),
                    format!(
                        "error: {}",
                        result["content"][0]["text"].as_str().expect("safe error")
                    )
                );
            }
        }
        if cli["is_complete"] == true {
            assert!(token.is_none());
            break;
        }
    }
    assert!(
        token.is_none(),
        "explicit bounded continuations must reach a terminal result"
    );
    assert!(complete_text.starts_with("An invented opening\n"));
    assert!(complete_text.contains("An invented sidebar"));
    assert!(!complete_text.contains("Adjacent topic"));
    assert_eq!(
        complete_text,
        format!(
            "An invented opening\n{}\nAn invented sidebar\nA useful invented note.\n",
            "An invented lantern glows \u{754c}. "
                .repeat(240)
                .trim_end()
        )
    );
    for name in [
        "book-redirect",
        "book-rejected",
        "book-oversized",
        "book-wrong-media",
    ] {
        let cli = cli(name, &["book", "read", "1", id]);
        let (output, messages) = mcp_tool(
            name,
            "book_read",
            json!({"book_id":"1","section_id":id}),
            false,
        );
        let result = tool_result(&output, &messages);
        assert_eq!(cli.status.code(), Some(1));
        assert!(cli.stdout.is_empty());
        assert_eq!(result["isError"], true);
        assert_eq!(
            String::from_utf8_lossy(&cli.stderr).trim(),
            format!(
                "error: {}",
                result["content"][0]["text"].as_str().expect("safe error")
            )
        );
    }
    for arguments in [
        json!({"book_id":"1","unexpected":true}),
        json!({"book_id":"1","section_id":id,"continuation":"bad-token"}),
        json!({"book_id":"1","section_id":id,"continuation":"x".repeat(16385)}),
    ] {
        let (output, messages) = mcp_tool("library-normal", "book_read", arguments, false);
        assert_eq!(tool_result(&output, &messages)["isError"], true);
    }
    let table = cli(
        "library-normal",
        &["book", "outline", "1", "--format", "tsv", "--no-headers"],
    );
    assert!(table.status.success());
    assert!(!table.stdout.starts_with(b"id\t"));
    assert_eq!(String::from_utf8_lossy(&table.stdout).lines().count(), 3);
    let (output, messages) = mcp_tool(
        "book-waiting",
        "book_read",
        json!({"book_id":"1","section_id":id}),
        true,
    );
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    if let Some(message) = messages.iter().find(|message| message["id"] == 3) {
        assert_eq!(message["result"]["isError"], true);
        assert_eq!(message["result"]["content"][0]["text"], "cancelled");
    }
}

/// Selects a synthetic child before running parent checks; no native entry is accessed.
fn main() {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("--cli-child") => {
            let session = session(&arguments.next().expect("scenario"));
            let args = std::iter::once("beyond-gate".to_owned()).chain(arguments);
            let code = beyond_gate_cli::run_with_session_input(
                args,
                &beyond_gate_cli::LiveReads,
                &session,
                &mut std::io::empty(),
                &mut std::io::stdout().lock(),
                &mut std::io::stderr().lock(),
            );
            std::process::exit(code);
        }
        Some("--mcp-child") => {
            let session = session(&arguments.next().expect("scenario"));
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("runtime");
            runtime.block_on(async {
                let service = beyond_gate_mcp::BeyondGateServer::with_session(session)
                    .serve(stdio())
                    .await
                    .expect("serve");
                service.waiting().await.expect("EOF");
            });
        }
        _ => {
            contracts();
            library_contracts();
            book_contracts();
        }
    }
}
