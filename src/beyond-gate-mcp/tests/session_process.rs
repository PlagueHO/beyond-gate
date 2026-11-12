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
use beyond_gate_core::{
    auth::session::Session, background::fixture as background_fixture,
    character_campaign::fixture as character_campaign_fixture,
    class_catalog::fixture as class_fixture, library::fixture as library_fixture,
    monster::fixture as monster_fixture, species::fixture as species_fixture,
};
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
    if let Some(name) = name.strip_prefix("feat-") {
        use beyond_gate_core::feat::fixture::{self, Scenario};
        let scenario = match name {
            "normal" => Scenario::Normal,
            "empty" => Scenario::Empty,
            "denied" => Scenario::Denied,
            "rejected" => Scenario::Rejected,
            "expired" => Scenario::Expired,
            "malformed" => Scenario::Malformed,
            "incomplete" => Scenario::Incomplete,
            "duplicate" => Scenario::Duplicate,
            "invalid-source" => Scenario::InvalidSource,
            "unsupported" => Scenario::Unsupported,
            "oversized" => Scenario::Oversized,
            "waiting" => Scenario::Waiting,
            _ => panic!("unknown synthetic feat scenario"),
        };
        return fixture::session(scenario).expect("invented feat service");
    }
    if let Some(name) = name.strip_prefix("spell-") {
        use beyond_gate_core::spell::fixture::{self, Scenario};
        let scenario = match name {
            "normal" => Scenario::Normal,
            "next" => Scenario::Next,
            "missing-next-href" => Scenario::MissingNextHref,
            "empty" => Scenario::Empty,
            "denied" => Scenario::Denied,
            "rejected" => Scenario::Rejected,
            "expired" => Scenario::Expired,
            "missing" => Scenario::Missing,
            "malformed" => Scenario::Malformed,
            "incomplete" => Scenario::Incomplete,
            "duplicate" => Scenario::Duplicate,
            "limit" => Scenario::TooManyRecords,
            "oversized" => Scenario::Oversized,
            "unsupported" => Scenario::Unsupported,
            "wrong-identity" => Scenario::WrongIdentity,
            "wrong-canonical" => Scenario::WrongCanonical,
            "missing-markers" => Scenario::MissingMarkers,
            "unsafe-detail" => Scenario::UnsafeDetail,
            "waiting" => Scenario::Waiting,
            "redirect" => Scenario::Redirect,
            _ => panic!("unknown synthetic spell scenario"),
        };
        return fixture::session(scenario).expect("invented spell service");
    }
    let species_source = match name {
        "species-normal" => Some(species_fixture::Scenario::Normal),
        "species-empty" => Some(species_fixture::Scenario::Empty),
        "species-denied" => Some(species_fixture::Scenario::Denied),
        "species-rejected" => Some(species_fixture::Scenario::Rejected),
        "species-expired" => Some(species_fixture::Scenario::Expired),
        "species-malformed" => Some(species_fixture::Scenario::Malformed),
        "species-incomplete" => Some(species_fixture::Scenario::Incomplete),
        "species-unsupported" => Some(species_fixture::Scenario::Unsupported),
        "species-duplicate" => Some(species_fixture::Scenario::Duplicate),
        "species-limit" => Some(species_fixture::Scenario::TooManyRecords),
        "species-oversized" => Some(species_fixture::Scenario::Oversized),
        "species-waiting" => Some(species_fixture::Scenario::Waiting),
        _ => None,
    };
    if let Some(source) = species_source {
        return species_fixture::session(source).expect("invented species service");
    }
    let background_source = match name {
        "background-normal" => Some(background_fixture::Scenario::Normal),
        "background-empty" => Some(background_fixture::Scenario::Empty),
        "background-denied" => Some(background_fixture::Scenario::Denied),
        "background-rejected" => Some(background_fixture::Scenario::Rejected),
        "background-expired" => Some(background_fixture::Scenario::Expired),
        "background-malformed" => Some(background_fixture::Scenario::Malformed),
        "background-incomplete" => Some(background_fixture::Scenario::Incomplete),
        "background-duplicate" => Some(background_fixture::Scenario::Duplicate),
        "background-invalid-source" => Some(background_fixture::Scenario::InvalidSource),
        "background-unsupported" => Some(background_fixture::Scenario::Unsupported),
        "background-oversized" => Some(background_fixture::Scenario::Oversized),
        "background-waiting" => Some(background_fixture::Scenario::Waiting),
        "background-wide-table" => Some(background_fixture::Scenario::WideTable),
        _ => None,
    };
    if let Some(source) = background_source {
        return background_fixture::session(source).expect("invented background services");
    }
    let monster_source = match name {
        "monster-normal" => Some(monster_fixture::Scenario::Normal),
        "monster-empty" => Some(monster_fixture::Scenario::Empty),
        "monster-denied" => Some(monster_fixture::Scenario::Denied),
        "monster-rejected" => Some(monster_fixture::Scenario::Rejected),
        "monster-expired" => Some(monster_fixture::Scenario::Expired),
        "monster-denied-code" => Some(monster_fixture::Scenario::DeniedCode),
        "monster-unsupported" => Some(monster_fixture::Scenario::UnsupportedCode),
        "monster-shared" => Some(monster_fixture::Scenario::SharedCode),
        "monster-mixed" => Some(monster_fixture::Scenario::MixedAccess),
        "monster-missing-access" => Some(monster_fixture::Scenario::MissingAccess),
        "monster-malformed" => Some(monster_fixture::Scenario::Malformed),
        "monster-incomplete" => Some(monster_fixture::Scenario::Incomplete),
        "monster-changed" => Some(monster_fixture::Scenario::ChangedIdentity),
        "monster-oversized" => Some(monster_fixture::Scenario::Oversized),
        "monster-waiting" => Some(monster_fixture::Scenario::Waiting),
        "monster-wide-table" => Some(monster_fixture::Scenario::WideTable),
        _ => None,
    };
    if let Some(source) = monster_source {
        return monster_fixture::session(source).expect("invented monster services");
    }
    let class_source = match name {
        "class-normal" => Some(class_fixture::Scenario::Normal),
        "class-ambiguous" => Some(class_fixture::Scenario::Ambiguous),
        "class-empty" => Some(class_fixture::Scenario::Empty),
        "class-denied" => Some(class_fixture::Scenario::Denied),
        "class-malformed" => Some(class_fixture::Scenario::Malformed),
        "class-incomplete" => Some(class_fixture::Scenario::Incomplete),
        "class-expired" => Some(class_fixture::Scenario::Expired),
        "class-rejected" => Some(class_fixture::Scenario::Rejected),
        "class-waiting" => Some(class_fixture::Scenario::Waiting),
        "class-wide-table" => Some(class_fixture::Scenario::WideTable),
        _ => None,
    };
    if let Some(source) = class_source {
        return class_fixture::session(source).expect("invented class services");
    }
    let character_source = match name {
        "character-normal" => Some(character_campaign_fixture::Scenario::Normal),
        "character-denied-roster" => Some(character_campaign_fixture::Scenario::DeniedRoster),
        "character-incomplete" => Some(character_campaign_fixture::Scenario::IncompleteCharacters),
        "character-malformed-campaigns" => {
            Some(character_campaign_fixture::Scenario::MalformedCampaigns)
        }
        "character-missing-campaign" => Some(character_campaign_fixture::Scenario::MissingCampaign),
        "character-changed" => Some(character_campaign_fixture::Scenario::ChangedCharacter),
        _ => None,
    };
    if let Some(source) = character_source {
        return character_campaign_fixture::session(source).expect("invented source services");
    }
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

/// Checks real CLI/MCP process parity through shared synthetic resource operations.
fn character_campaign_contracts() {
    for (tool, arguments, command) in [
        (
            "character_list",
            json!({}),
            vec!["character", "list", "--output", "json"],
        ),
        (
            "character_show",
            json!({"id":"42"}),
            vec!["character", "show", "42"],
        ),
        (
            "campaign_list",
            json!({}),
            vec!["campaign", "list", "--output", "json"],
        ),
        (
            "campaign_list",
            json!({"active_only":true}),
            vec!["campaign", "list", "--active-only", "--output", "json"],
        ),
        (
            "campaign_show",
            json!({"id":"8"}),
            vec!["campaign", "show", "8"],
        ),
    ] {
        let cli = cli("character-normal", &command);
        assert!(
            cli.status.success(),
            "CLI {tool}: {}",
            String::from_utf8_lossy(&cli.stderr)
        );
        assert!(cli.stderr.is_empty());
        let cli_value: Value = serde_json::from_slice(&cli.stdout).expect("CLI JSON data");

        let (output, messages) = mcp_tool("character-normal", tool, arguments, false);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let result = &messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("tool result")["result"];
        assert_eq!(result["isError"], false, "{tool}: {result}");
        let text = result["content"][0]["text"].as_str().expect("MCP JSON");
        let mcp_value: Value = serde_json::from_str(text).expect("MCP JSON data");
        assert_eq!(mcp_value, cli_value, "{tool} CLI/MCP output differs");
    }

    let (output, messages) = mcp_tool(
        "character-normal",
        "campaign_list",
        json!({"active_only":"yes"}),
        false,
    );
    assert!(output.status.success());
    assert_eq!(
        messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("strict active-only response")["result"]["isError"],
        true
    );
}

/// Rejects padded amplification while preserving complete compact metadata formats.
fn class_table_limit_contracts() {
    for options in [vec![], vec!["--no-headers"], vec!["--format", "table"]] {
        let mut arguments = vec!["class", "list"];
        arguments.extend(options);
        let output = cli("class-wide-table", &arguments);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("Table presentation exceeds the 8388608-byte limit")
        );
    }
    let output = cli("class-wide-table", &["class", "list", "-o", "json"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let rows: Value = serde_json::from_slice(&output.stdout).expect("complete metadata");
    let rows = rows.as_array().expect("metadata list");
    assert_eq!(rows.len(), 1_000);
    assert_eq!(rows.last().expect("long name")["name"], "x".repeat(9_000));
    for options in [
        vec!["--output", "tsv"],
        vec!["--format", "tsv", "--no-headers"],
    ] {
        let mut arguments = vec!["class", "list"];
        arguments.extend(options);
        let output = cli("class-wide-table", &arguments);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(output.stdout.len() < 1024 * 1024);
        let text = String::from_utf8(output.stdout).expect("UTF-8 TSV");
        assert!(text.contains(&"x".repeat(9_000)));
        assert_eq!(
            text.lines().count(),
            if arguments.contains(&"--no-headers") {
                1_000
            } else {
                1_001
            }
        );
    }
}

/// Checks each reviewed metadata workflow through actual CLI and MCP child processes.
fn class_contracts() {
    for (tool, arguments, command) in [
        (
            "class_list",
            json!({}),
            vec!["class", "list", "--output", "json"],
        ),
        (
            "class_list",
            json!({"name":"Fighter"}),
            vec!["class", "list", "--name", "Fighter", "--output", "json"],
        ),
        (
            "class_list",
            json!({"name":"fight"}),
            vec!["class", "list", "-n", "fight", "--output", "json"],
        ),
        (
            "class_show",
            json!({"id":"2190879"}),
            vec!["class", "show", "2190879"],
        ),
        (
            "class_show",
            json!({"id":"10"}),
            vec!["class", "show", "10"],
        ),
        (
            "class_list",
            json!({"kind":"subclass"}),
            vec!["subclass", "list", "--output", "json"],
        ),
        (
            "class_list",
            json!({"kind":"subclass","name":"Champion"}),
            vec!["subclass", "list", "--name", "Champion", "--output", "json"],
        ),
        (
            "class_list",
            json!({"kind":"subclass","class_id":"2190879"}),
            vec![
                "subclass",
                "list",
                "--class-id",
                "2190879",
                "--output",
                "json",
            ],
        ),
        (
            "class_list",
            json!({"kind":"subclass","class_id":"10","name":"Champion"}),
            vec![
                "subclass",
                "list",
                "--class-id",
                "10",
                "--name",
                "Champion",
                "--output",
                "json",
            ],
        ),
        (
            "class_show",
            json!({"kind":"subclass","id":"7001","class_id":"2190879"}),
            vec!["subclass", "show", "7001", "--class-id", "2190879"],
        ),
        (
            "class_show",
            json!({"kind":"subclass","id":"7001"}),
            vec!["subclass", "show", "7001"],
        ),
    ] {
        let cli = cli("class-normal", &command);
        assert!(
            cli.status.success(),
            "{command:?}: {}",
            String::from_utf8_lossy(&cli.stderr)
        );
        assert!(cli.stderr.is_empty());
        let cli_value: Value = serde_json::from_slice(&cli.stdout).expect("CLI metadata");
        let (output, messages) = mcp_tool("class-normal", tool, arguments, false);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let result = &messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("result")["result"];
        assert_eq!(result["isError"], false, "{tool}: {result}");
        let text = result["content"][0]["text"]
            .as_str()
            .expect("metadata JSON");
        assert!(!text.contains("DISCARDED"));
        assert_eq!(
            serde_json::from_str::<Value>(text).expect("JSON metadata"),
            cli_value
        );
        let tools = messages
            .iter()
            .find(|message| message["id"] == 2)
            .expect("tools")["result"]["tools"]
            .as_array()
            .expect("tools");
        for name in ["class_list", "class_show"] {
            let schema = &tools
                .iter()
                .find(|tool| tool["name"] == name)
                .expect("class tool")["inputSchema"];
            assert_eq!(schema["additionalProperties"], false);
            assert!(
                !schema["required"]
                    .as_array()
                    .is_some_and(|required| required.contains(&json!("kind")))
            );
            assert_eq!(schema["properties"]["class_id"]["type"], "string");
        }
    }
    for (tool, arguments) in [
        ("class_list", json!({"id":"10"})),
        ("class_list", json!({"class_id":"10"})),
        ("class_list", json!({"kind":"other"})),
        ("class_list", json!({"name":null})),
        ("class_list", json!({"name":""})),
        ("class_list", json!({"kind":"subclass","class_id":null})),
        ("class_list", json!({"unknown":true})),
        ("class_show", json!({"id":"10","name":"Fighter"})),
        ("class_show", json!({"id":"10","class_id":"10"})),
        ("class_show", json!({"id":10})),
        ("class_show", json!({"id":"01"})),
        ("class_show", json!({"id":"18446744073709551616"})),
        ("class_show", json!({})),
    ] {
        let (output, messages) = mcp_tool("absent", tool, arguments, false);
        assert!(output.status.success());
        let result = &messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("invalid input response")["result"];
        assert_eq!(result["isError"], true, "{tool}: {result}");
        assert!(!result.to_string().contains("session_absent"));
    }
    for (scenario, command, tool, arguments, expected) in [
        (
            "class-ambiguous",
            vec!["subclass", "show", "7001"],
            "class_show",
            json!({"kind":"subclass","id":"7001"}),
            "ambiguous",
        ),
        (
            "class-denied",
            vec!["subclass", "list", "--output", "json"],
            "class_list",
            json!({"kind":"subclass"}),
            "access_denied",
        ),
        (
            "class-malformed",
            vec!["class", "list", "--output", "json"],
            "class_list",
            json!({}),
            "invalid class catalog metadata",
        ),
        (
            "class-incomplete",
            vec!["class", "list", "--output", "json"],
            "class_list",
            json!({}),
            "complete terminal",
        ),
        (
            "class-expired",
            vec!["class", "list", "--output", "json"],
            "class_list",
            json!({}),
            "session_expired",
        ),
        (
            "class-rejected",
            vec!["class", "list", "--output", "json"],
            "class_list",
            json!({}),
            "session_rejected",
        ),
        (
            "class-normal",
            vec!["class", "show", "999"],
            "class_show",
            json!({"id":"999"}),
            "not found",
        ),
    ] {
        let cli = cli(scenario, &command);
        assert_eq!(cli.status.code(), Some(1));
        assert!(cli.stdout.is_empty());
        assert!(String::from_utf8_lossy(&cli.stderr).contains(expected));
        let (output, messages) = mcp_tool(scenario, tool, arguments, false);
        assert!(output.status.success());
        let result = &messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("failure result")["result"];
        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .expect("failure")
                .contains(expected)
        );
    }
    let output = cli("class-empty", &["class", "list", "-o", "json"]);
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).expect("empty array"),
        json!([])
    );
    let output = cli(
        "class-normal",
        &["class", "list", "--format", "tsv", "--no-headers"],
    );
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 2);
    let output = cli("class-normal", &["class", "list"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("source_ids"));
    let (output, _) = mcp_tool("class-waiting", "class_list", json!({}), true);
    assert!(output.status.success());
}

/// Checks species data parity, strict tool schemas, and typed shared failures.
fn species_contracts() {
    for (scenario, tool, arguments, command) in [
        (
            "species-normal",
            "species_list",
            json!({}),
            vec!["species", "list", "--output", "json"],
        ),
        (
            "species-normal",
            "species_show",
            json!({"id":"18"}),
            vec!["species", "show", "18"],
        ),
    ] {
        let cli = cli(scenario, &command);
        assert!(
            cli.status.success(),
            "{command:?}: {}",
            String::from_utf8_lossy(&cli.stderr)
        );
        assert!(cli.stderr.is_empty());
        let cli_value: Value = serde_json::from_slice(&cli.stdout).expect("CLI JSON");
        let (output, messages) = mcp_tool(scenario, tool, arguments, false);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let result = &messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("tool result")["result"];
        assert_eq!(result["isError"], false, "{result}");
        let text = result["content"][0]["text"].as_str().expect("JSON text");
        assert_eq!(
            serde_json::from_str::<Value>(text).expect("MCP JSON"),
            cli_value
        );
        if tool == "species_show" {
            assert_eq!(
                cli_value["raw"]["future_definition"]["nested"],
                json!([true, null, 3])
            );
            assert_eq!(cli_value["is_subrace"], true);
            assert_eq!(cli_value["is_legacy"], true);
        }
        let tools = messages
            .iter()
            .find(|message| message["id"] == 2)
            .expect("tool discovery")["result"]["tools"]
            .as_array()
            .expect("tools");
        for name in ["species_list", "species_show"] {
            let schema = &tools
                .iter()
                .find(|candidate| candidate["name"] == name)
                .expect("species tool");
            assert_eq!(schema["inputSchema"]["additionalProperties"], false);
            assert_eq!(schema["annotations"]["readOnlyHint"], true);
            assert_eq!(schema["annotations"]["destructiveHint"], false);
        }
        let show_schema = &tools
            .iter()
            .find(|candidate| candidate["name"] == "species_show")
            .expect("species show");
        assert_eq!(
            show_schema["inputSchema"]["properties"]["id"]["pattern"],
            "^[1-9][0-9]{0,19}$"
        );
    }

    for (tool, arguments) in [
        ("species_list", json!({"id":"17"})),
        ("species_list", json!({"unknown":true})),
        ("species_show", json!({"id":17})),
        ("species_show", json!({"id":"01"})),
        ("species_show", json!({"id":"18446744073709551616"})),
        ("species_show", json!({"id":"18","unknown":true})),
    ] {
        let (output, messages) = mcp_tool("species-normal", tool, arguments, false);
        assert!(output.status.success());
        let response = messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("response");
        assert!(
            response.get("error").is_some() || response["result"]["isError"] == true,
            "{response}"
        );
    }

    for (scenario, command, tool, arguments, expected) in [
        (
            "species-empty",
            vec!["species", "list", "--output", "json"],
            "species_list",
            json!({}),
            "incomplete",
        ),
        (
            "species-denied",
            vec!["species", "list", "--output", "json"],
            "species_list",
            json!({}),
            "access_denied",
        ),
        (
            "species-rejected",
            vec!["species", "list", "--output", "json"],
            "species_list",
            json!({}),
            "session_rejected",
        ),
        (
            "species-expired",
            vec!["species", "list", "--output", "json"],
            "species_list",
            json!({}),
            "session_expired",
        ),
        (
            "species-malformed",
            vec!["species", "list", "--output", "json"],
            "species_list",
            json!({}),
            "incomplete",
        ),
        (
            "species-incomplete",
            vec!["species", "list", "--output", "json"],
            "species_list",
            json!({}),
            "unsupported",
        ),
        (
            "species-duplicate",
            vec!["species", "list", "--output", "json"],
            "species_list",
            json!({}),
            "conflicting",
        ),
        (
            "species-unsupported",
            vec!["species", "list", "--output", "json"],
            "species_list",
            json!({}),
            "unsupported",
        ),
        (
            "species-limit",
            vec!["species", "list", "--output", "json"],
            "species_list",
            json!({}),
            "limit exceeded",
        ),
        (
            "species-oversized",
            vec!["species", "list", "--output", "json"],
            "species_list",
            json!({}),
            "authenticated_read_failed",
        ),
        (
            "species-denied",
            vec!["species", "show", "17", "--output", "json"],
            "species_show",
            json!({"id":"17"}),
            "access_denied",
        ),
        (
            "species-rejected",
            vec!["species", "show", "17", "--output", "json"],
            "species_show",
            json!({"id":"17"}),
            "session_rejected",
        ),
        (
            "species-expired",
            vec!["species", "show", "17", "--output", "json"],
            "species_show",
            json!({"id":"17"}),
            "session_expired",
        ),
        (
            "species-malformed",
            vec!["species", "show", "17", "--output", "json"],
            "species_show",
            json!({"id":"17"}),
            "invalid species response",
        ),
        (
            "species-incomplete",
            vec!["species", "show", "17", "--output", "json"],
            "species_show",
            json!({"id":"17"}),
            "incomplete or inconsistent",
        ),
        (
            "species-limit",
            vec!["species", "show", "17", "--output", "json"],
            "species_show",
            json!({"id":"17"}),
            "species result limit exceeded",
        ),
        (
            "species-oversized",
            vec!["species", "show", "17", "--output", "json"],
            "species_show",
            json!({"id":"17"}),
            "authenticated_read_failed",
        ),
        (
            "species-normal",
            vec!["species", "show", "999"],
            "species_show",
            json!({"id":"999"}),
            "not found",
        ),
    ] {
        let cli = cli(scenario, &command);
        assert_eq!(cli.status.code(), Some(1), "{command:?}");
        assert!(cli.stdout.is_empty());
        let diagnostic = String::from_utf8_lossy(&cli.stderr);
        assert!(diagnostic.contains(expected), "{diagnostic}");
        let (output, messages) = mcp_tool(scenario, tool, arguments, false);
        assert!(output.status.success());
        let result = &messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("failure result")["result"];
        assert_eq!(result["isError"], true, "{result}");
        assert!(result.get("structuredContent").is_none(), "{result}");
        assert!(
            result["content"][0]["text"]
                .as_str()
                .expect("error text")
                .contains(expected)
        );
    }
}

/// Checks monster data parity, strict inputs, paging, presentation, and safe failures.
fn monster_contracts() {
    for (scenario, tool, arguments, command) in [
        (
            "monster-normal",
            "monster_list",
            json!({}),
            vec!["monster", "list", "-o", "json"],
        ),
        (
            "monster-normal",
            "monster_list",
            json!({"name":"beast & take=100","cr_min":"1/8","cr_max":"24","size_ids":[4,5],"legendary":false,"partnered_content":"any"}),
            vec![
                "monster",
                "list",
                "-n",
                "beast & take=100",
                "--cr-min",
                "1/8",
                "--cr-max",
                "24",
                "--size-id",
                "4",
                "--size-id",
                "5",
                "--legendary",
                "false",
                "--partnered-content",
                "any",
                "--format",
                "json",
            ],
        ),
        (
            "monster-normal",
            "monster_list",
            json!({"page":2}),
            vec!["monster", "list", "--page", "2", "-o", "json"],
        ),
        (
            "monster-normal",
            "monster_list",
            json!({"name":"missing"}),
            vec!["monster", "list", "--name", "missing", "-o", "json"],
        ),
        (
            "monster-empty",
            "monster_list",
            json!({}),
            vec!["monster", "list", "-o", "json"],
        ),
        (
            "monster-normal",
            "monster_list",
            json!({"page":2,"type_ids":[1],"subtype_ids":[1],"size_ids":[1],"environment_ids":[1],
                "alignment_ids":[1],"sense_ids":[1],"save_proficiency_ids":[1],"skill_proficiency_ids":[1],
                "resistance_ids":[1],"damage_immunity_ids":[1],"condition_immunity_ids":[1],
                "vulnerability_ids":[1],"language_ids":[1],"movement_ids":[1],"tag_ids":[1],
                "source_category_ids":[1],"source_ids":[1,2],"armor_class_min":0,"armor_class_max":99,
                "average_hp_min":1,"average_hp_max":1000,"legendary":true,"mythic":false,"has_lair":true,"partnered_content":"yes"}),
            vec![
                "monster",
                "list",
                "--page",
                "2",
                "--type-id",
                "1",
                "--subtype-id",
                "1",
                "--size-id",
                "1",
                "--environment-id",
                "1",
                "--alignment-id",
                "1",
                "--sense-id",
                "1",
                "--save-proficiency-id",
                "1",
                "--skill-proficiency-id",
                "1",
                "--resistance-id",
                "1",
                "--damage-immunity-id",
                "1",
                "--condition-immunity-id",
                "1",
                "--vulnerability-id",
                "1",
                "--language-id",
                "1",
                "--movement-id",
                "1",
                "--tag-id",
                "1",
                "--source-category-id",
                "1",
                "--source-id",
                "1",
                "--source-id",
                "2",
                "--armor-class-min",
                "0",
                "--armor-class-max",
                "99",
                "--average-hp-min",
                "1",
                "--average-hp-max",
                "1000",
                "--legendary",
                "true",
                "--mythic",
                "false",
                "--has-lair",
                "true",
                "--partnered-content",
                "yes",
                "-o",
                "json",
            ],
        ),
        (
            "monster-normal",
            "monster_show",
            json!({"id":"17"}),
            vec!["monster", "show", "17"],
        ),
    ] {
        let output = cli(scenario, &command);
        assert!(
            output.status.success(),
            "{command:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let cli_value: Value = serde_json::from_slice(&output.stdout).expect("CLI JSON");
        let (output, messages) = mcp_tool(scenario, tool, arguments, false);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let result = &messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("response")["result"];
        assert_eq!(result["isError"], false, "{result}");
        let value: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().expect("JSON"))
                .expect("MCP JSON");
        assert_eq!(value, cli_value);
        let tools = messages
            .iter()
            .find(|message| message["id"] == 2)
            .expect("discovery")["result"]["tools"]
            .as_array()
            .expect("tools");
        for name in ["monster_list", "monster_show"] {
            let schema = &tools
                .iter()
                .find(|tool| tool["name"] == name)
                .expect("monster tool")["inputSchema"];
            assert_eq!(schema["additionalProperties"], false);
            assert_eq!(
                schema["properties"]["id"]["type"],
                if name == "monster_show" {
                    json!("string")
                } else {
                    Value::Null
                }
            );
            if name == "monster_list" {
                assert_eq!(schema["properties"]["name"]["type"], "string");
                assert_eq!(schema["properties"]["page"]["minimum"], 1);
                assert_eq!(schema["properties"]["size_ids"]["type"], "array");
                assert!(schema["properties"].get("skip").is_none());
                assert!(schema["properties"].get("take").is_none());
            }
        }
    }
    for (tool, arguments) in [
        ("monster_list", json!({"unknown":true})),
        ("monster_list", json!({"id":"17"})),
        ("monster_list", json!({"name":null})),
        ("monster_list", json!({"name":""})),
        ("monster_list", json!({"name":" padded"})),
        ("monster_list", json!({"name":"x".repeat(257)})),
        ("monster_list", json!({"skip":null})),
        ("monster_list", json!({"skip":-1})),
        ("monster_list", json!({"skip":"1"})),
        ("monster_list", json!({"skip":u64::MAX,"take":1})),
        ("monster_list", json!({"take":0})),
        ("monster_list", json!({"take":101})),
        ("monster_list", json!({"take":1.5})),
        ("monster_list", json!({"take":null})),
        ("monster_list", json!({"page":0})),
        ("monster_list", json!({"page":null})),
        ("monster_list", json!({"size_ids":[0]})),
        ("monster_list", json!({"size_ids":[4,4]})),
        ("monster_list", json!({"legendary":null})),
        (
            "monster_list",
            json!({"armor_class_min":2,"armor_class_max":1}),
        ),
        ("monster_list", json!({"cr_min":"24","cr_max":"23"})),
        ("monster_list", json!({"partnered_content":"invalid"})),
        ("monster_show", json!({"id":17})),
        ("monster_show", json!({"id":null})),
        ("monster_show", json!({"id":"01"})),
        ("monster_show", json!({"id":"0"})),
        ("monster_show", json!({"id":"18446744073709551616"})),
        ("monster_show", json!({"id":"17","name":"beast"})),
        ("monster_show", json!({})),
    ] {
        let (output, messages) = mcp_tool("absent", tool, arguments, false);
        assert!(output.status.success());
        let result = &messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("invalid response")["result"];
        assert_eq!(result["isError"], true, "{result}");
        assert!(!result.to_string().contains("session_absent"));
    }
    for (scenario, expected) in [
        ("monster-denied", "access_denied"),
        ("monster-rejected", "session_rejected"),
        ("monster-expired", "session_expired"),
        ("monster-denied-code", "monster access denied"),
        ("monster-unsupported", "not supported"),
        ("monster-shared", "not supported"),
        ("monster-missing-access", "invalid monster response"),
        ("monster-malformed", "invalid monster response"),
        ("monster-oversized", "authenticated_read_failed"),
    ] {
        for (command, tool, arguments) in [
            (
                vec!["monster", "list", "-o", "json"],
                "monster_list",
                json!({}),
            ),
            (
                vec!["monster", "show", "17"],
                "monster_show",
                json!({"id":"17"}),
            ),
        ] {
            let output = cli(scenario, &command);
            assert_eq!(output.status.code(), Some(1), "{scenario}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(expected),
                "{scenario}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let (output, messages) = mcp_tool(scenario, tool, arguments, false);
            assert!(output.status.success());
            let result = &messages
                .iter()
                .find(|message| message["id"] == 3)
                .expect("failure")["result"];
            assert_eq!(result["isError"], true);
            assert!(
                result["content"][0]["text"]
                    .as_str()
                    .expect("error")
                    .contains(expected),
                "{scenario}: {result}"
            );
        }
    }
    for (scenario, command, tool, arguments, expected, exit_code) in [
        (
            "monster-normal",
            vec!["monster", "show", "99"],
            "monster_show",
            json!({"id":"99"}),
            "not found",
            1,
        ),
        (
            "monster-changed",
            vec!["monster", "show", "17"],
            "monster_show",
            json!({"id":"17"}),
            "invalid monster response",
            1,
        ),
        (
            "monster-incomplete",
            vec!["monster", "list", "-o", "json"],
            "monster_list",
            json!({}),
            "incomplete",
            1,
        ),
        (
            "monster-mixed",
            vec!["monster", "list", "-o", "json"],
            "monster_list",
            json!({}),
            "monster access denied",
            1,
        ),
        (
            "monster-normal",
            vec!["monster", "list", "--size-id", "99", "-o", "json"],
            "monster_list",
            json!({"size_ids":[99]}),
            "invalid monster input",
            2,
        ),
    ] {
        let output = cli(scenario, &command);
        assert_eq!(output.status.code(), Some(exit_code));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
        let (_, messages) = mcp_tool(scenario, tool, arguments, false);
        let result = &messages
            .iter()
            .find(|message| message["id"] == 3)
            .expect("failure")["result"];
        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .expect("error")
                .contains(expected)
        );
    }
    for (format, headers) in [
        ("table", true),
        ("table", false),
        ("tsv", true),
        ("tsv", false),
    ] {
        let mut command = vec!["monster", "list", "-o", format];
        if !headers {
            command.push("--no-headers");
        }
        let output = cli("monster-normal", &command);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).expect("table/TSV");
        assert!(text.contains("Invented Beast"));
        assert_eq!(text.lines().count(), if headers { 3 } else { 2 });
        if headers {
            assert!(text.contains("next_page"));
        }
        let output = cli("monster-normal", &["monster", "show", "17", "-o", format]);
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Synthetic action only."));
    }
    let output = cli("monster-wide-table", &["monster", "list"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("8388608-byte limit"));
    for format in ["json", "tsv"] {
        let output = cli("monster-wide-table", &["monster", "list", "-o", format]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
    }
    for tool in ["monster_list", "monster_show"] {
        let (output, messages) = mcp_tool(
            "monster-waiting",
            tool,
            if tool == "monster_show" {
                json!({"id":"17"})
            } else {
                json!({})
            },
            true,
        );
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(messages.iter().any(|message| message["id"] == 4));
        assert!(
            !messages
                .iter()
                .any(|message| message["id"] == 3 && message["result"]["isError"] == false)
        );
    }
}

/// Requires actual CLI/MCP background data, strict schemas, errors, presentation, and cancellation parity.
fn background_contracts() {
    for (scenario, tool, arguments, command) in [
        (
            "background-normal",
            "background_list",
            json!({}),
            vec!["background", "list", "-o", "json"],
        ),
        (
            "background-normal",
            "background_list",
            json!({"name":"beast & take=100","skill_ids":[240,241],"tag_ids":[7],"source_ids":[7],"source_category_ids":[8],"partnered_content":"any"}),
            vec![
                "background",
                "list",
                "-n",
                "beast & take=100",
                "--skill-id",
                "240",
                "--skill-id",
                "241",
                "--tag-id",
                "7",
                "--source-id",
                "7",
                "--source-category-id",
                "8",
                "--partnered-content",
                "any",
                "--format",
                "json",
            ],
        ),
        (
            "background-empty",
            "background_list",
            json!({}),
            vec!["background", "list", "-o", "json"],
        ),
        (
            "background-normal",
            "background_show",
            json!({"id":"17"}),
            vec!["background", "show", "17"],
        ),
    ] {
        let output = cli(scenario, &command);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let cli_value: Value = serde_json::from_slice(&output.stdout).expect("CLI JSON");
        let (output, messages) = mcp_tool(scenario, tool, arguments, false);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let result = &messages.iter().find(|m| m["id"] == 3).expect("response")["result"];
        assert_eq!(result["isError"], false, "{result}");
        let value: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().expect("text"))
                .expect("MCP JSON");
        assert_eq!(value, cli_value);
        if tool == "background_show" {
            assert_eq!(value["raw"]["future"]["nested"], json!([true, null, 3]));
            assert_eq!(value["sources"][0]["future"], true);
        } else {
            assert!(
                value
                    .as_array()
                    .expect("metadata")
                    .iter()
                    .all(|v| v.get("raw").is_none())
            );
        }
        let tools = messages.iter().find(|m| m["id"] == 2).expect("discovery")["result"]["tools"]
            .as_array()
            .expect("tools");
        for name in ["background_list", "background_show"] {
            let tool = tools
                .iter()
                .find(|t| t["name"] == name)
                .expect("background tool");
            let schema = &tool["inputSchema"];
            assert_eq!(schema["additionalProperties"], false);
            assert_eq!(tool["annotations"]["readOnlyHint"], true);
            if name == "background_list" {
                assert_eq!(schema["properties"]["name"]["type"], "string");
                assert_eq!(schema["properties"]["skill_ids"]["type"], "array");
                assert_eq!(schema["properties"]["skill_ids"]["maxItems"], 50);
                assert_eq!(schema["properties"]["skill_ids"]["uniqueItems"], true);
                assert!(schema["properties"].get("page").is_none());
                assert!(schema["properties"].get("skip").is_none());
            } else {
                assert_eq!(schema["properties"]["id"]["type"], "string");
            }
        }
    }
    for (tool, arguments) in [
        ("background_list", json!({"page":1})),
        ("background_list", json!({"name":null})),
        ("background_list", json!({"skill_ids":null})),
        ("background_list", json!({"skill_ids":[0]})),
        ("background_list", json!({"skill_ids":[7,7]})),
        ("background_list", json!({"name":" padded"})),
        ("background_list", json!({"partnered_content":"false"})),
        ("background_show", json!({"id":"01"})),
        ("background_show", json!({"id":17})),
        ("background_show", json!({"id":"17","unknown":true})),
    ] {
        let (output, messages) = mcp_tool("background-normal", tool, arguments, false);
        assert!(output.status.success());
        let result = messages.iter().find(|m| m["id"] == 3).expect("response");
        assert!(
            result.get("error").is_some() || result["result"]["isError"] == true,
            "{result}"
        );
    }
    for (scenario, tool, arguments, command, diagnostic) in [
        (
            "background-denied",
            "background_show",
            json!({"id":"17"}),
            vec!["background", "show", "17"],
            "access_denied",
        ),
        (
            "background-rejected",
            "background_show",
            json!({"id":"17"}),
            vec!["background", "show", "17"],
            "session_rejected",
        ),
        (
            "background-expired",
            "background_show",
            json!({"id":"17"}),
            vec!["background", "show", "17"],
            "session_expired",
        ),
        (
            "background-malformed",
            "background_show",
            json!({"id":"17"}),
            vec!["background", "show", "17"],
            "invalid background response",
        ),
        (
            "background-incomplete",
            "background_show",
            json!({"id":"17"}),
            vec!["background", "show", "17"],
            "not complete",
        ),
        (
            "background-duplicate",
            "background_show",
            json!({"id":"17"}),
            vec!["background", "show", "17"],
            "conflicting",
        ),
        (
            "background-invalid-source",
            "background_show",
            json!({"id":"17"}),
            vec!["background", "show", "17"],
            "invalid background response",
        ),
        (
            "background-unsupported",
            "background_show",
            json!({"id":"17"}),
            vec!["background", "show", "17"],
            "unsupported",
        ),
        (
            "background-oversized",
            "background_show",
            json!({"id":"17"}),
            vec!["background", "show", "17"],
            "BodyTooLarge",
        ),
        (
            "background-normal",
            "background_show",
            json!({"id":"999"}),
            vec!["background", "show", "999"],
            "not found",
        ),
        (
            "background-denied",
            "background_list",
            json!({}),
            vec!["background", "list", "-o", "json"],
            "access_denied",
        ),
        (
            "background-incomplete",
            "background_list",
            json!({}),
            vec!["background", "list", "-o", "json"],
            "unsupported",
        ),
        (
            "background-normal",
            "background_list",
            json!({"skill_ids":[999]}),
            vec!["background", "list", "--skill-id", "999", "-o", "json"],
            "invalid background input",
        ),
    ] {
        let output = cli(scenario, &command);
        assert_eq!(
            output.status.code(),
            Some(if diagnostic == "invalid background input" {
                2
            } else {
                1
            })
        );
        assert!(output.stdout.is_empty());
        let cli_error = String::from_utf8(output.stderr).expect("diagnostic");
        assert!(cli_error.contains(diagnostic), "{cli_error}");
        let (output, messages) = mcp_tool(scenario, tool, arguments, false);
        assert!(output.status.success());
        let result = &messages.iter().find(|m| m["id"] == 3).expect("response")["result"];
        assert_eq!(result["isError"], true, "{result}");
        let error = result["content"][0]["text"].as_str().expect("error");
        assert!(error.contains(diagnostic), "{error}");
        assert!(cli_error.contains(error));
    }
    for (action, format) in [
        ("list", "table"),
        ("list", "tsv"),
        ("show", "table"),
        ("show", "tsv"),
    ] {
        let mut args = vec!["background", action];
        if action == "show" {
            args.push("17");
        }
        args.extend(["-o", format]);
        let output = cli("background-normal", &args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).expect("text");
        assert!(text.contains("Invented Origin"));
        assert!(text.contains("source_url"));
        args.push("--no-headers");
        let output = cli("background-normal", &args);
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).expect("text");
        if action == "list" {
            assert!(!text.contains("source_url"));
        }
    }
    for (tool, args) in [
        ("background_list", json!({})),
        ("background_show", json!({"id":"17"})),
    ] {
        let (output, messages) = mcp_tool("background-waiting", tool, args, true);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(messages.iter().any(|m| m["id"] == 4));
        assert!(
            !messages
                .iter()
                .any(|m| m["id"] == 3 && m["result"]["isError"] == false)
        );
    }
}

/// Checks background padded-table bounds separately from complete JSON/TSV output.
fn background_table_limit_contracts() {
    let output = cli(
        "background-wide-table",
        &["background", "list", "-o", "json"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows: Value = serde_json::from_slice(&output.stdout).expect("JSON");
    assert_eq!(rows.as_array().expect("rows").len(), 1000);
    for extra in [vec![], vec!["--no-headers"]] {
        let mut args = vec!["background", "list", "-o", "table"];
        args.extend(extra);
        let output = cli("background-wide-table", &args);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("limit"));
    }
    let output = cli(
        "background-wide-table",
        &["background", "list", "-o", "tsv", "--no-headers"],
    );
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)
            .expect("TSV")
            .lines()
            .count(),
        1000
    );
    let output = cli(
        "background-normal",
        &["background", "show", "17", "-o", "tsv"],
    );
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("TSV");
    assert_eq!(text.lines().count(), 2);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains("\\\\t"));
}

/// Checks shared feat CLI/MCP data, strict schemas, safe errors, and cancellation.
fn feat_contracts() {
    for (scenario, command, tool, arguments) in [
        (
            "feat-normal",
            vec!["feat", "list", "-o", "json"],
            "feat_list",
            json!({}),
        ),
        (
            "feat-empty",
            vec!["feat", "list", "-o", "json"],
            "feat_list",
            json!({}),
        ),
        (
            "feat-normal",
            vec!["feat", "show", "18"],
            "feat_show",
            json!({"id":"18"}),
        ),
        (
            "feat-normal",
            vec![
                "feat",
                "list",
                "-n",
                "literal & take=10",
                "--prerequisite-subtype-id",
                "240",
                "--tag-id",
                "7",
                "--tag-id",
                "8",
                "--source-id",
                "7",
                "--source-category-id",
                "8",
                "--partnered-content",
                "any",
                "-o",
                "json",
            ],
            "feat_list",
            json!({"name":"literal & take=10","prerequisite_subtype_id":240,"tag_ids":[7,8],
                "source_ids":[7],"source_category_ids":[8],"partnered_content":"any"}),
        ),
    ] {
        let output = cli(scenario, &command);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let cli_value: Value = serde_json::from_slice(&output.stdout).expect("CLI JSON");
        let (output, messages) = mcp_tool(scenario, tool, arguments, false);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let result = &messages.iter().find(|m| m["id"] == 3).expect("response")["result"];
        assert_eq!(result["isError"], false, "{result}");
        let value: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().expect("text"))
                .expect("MCP JSON");
        assert_eq!(value, cli_value);
        if tool == "feat_show" {
            assert_eq!(value["raw"]["future"]["nested"], json!([true, null, 3]));
            assert_eq!(value["repeatable_parent_id"], "17");
        } else {
            assert!(
                value
                    .as_array()
                    .expect("rows")
                    .iter()
                    .all(|v| v.get("raw").is_none())
            );
        }
        let tools = messages.iter().find(|m| m["id"] == 2).expect("discovery")["result"]["tools"]
            .as_array()
            .expect("tools");
        for name in ["feat_list", "feat_show"] {
            let tool = tools.iter().find(|t| t["name"] == name).expect("feat tool");
            let schema = &tool["inputSchema"];
            assert_eq!(schema["additionalProperties"], false);
            assert_eq!(tool["annotations"]["readOnlyHint"], true);
            if name == "feat_list" {
                assert_eq!(
                    schema["properties"]["prerequisite_subtype_id"]["type"],
                    "integer"
                );
                assert_eq!(schema["properties"]["tag_ids"]["maxItems"], 50);
                assert_eq!(schema["properties"]["tag_ids"]["uniqueItems"], true);
            } else {
                assert_eq!(schema["properties"]["id"]["pattern"], "^[1-9][0-9]{0,19}$");
            }
        }
    }
    for (tool, arguments) in [
        ("feat_list", json!({"page":1})),
        ("feat_list", json!({"name":null})),
        ("feat_list", json!({"tag_ids":null})),
        ("feat_list", json!({"tag_ids":[0]})),
        ("feat_list", json!({"tag_ids":[7,7]})),
        ("feat_list", json!({"name":" padded"})),
        ("feat_list", json!({"prerequisite_subtype_id":null})),
        ("feat_list", json!({"prerequisite_subtype_id":[7,8]})),
        ("feat_list", json!({"prerequisite_subtype_id":0})),
        ("feat_list", json!({"partnered_content":"false"})),
        ("feat_show", json!({"id":"01"})),
        ("feat_show", json!({"id":17})),
        ("feat_show", json!({"id":"17","unknown":true})),
    ] {
        let (output, messages) = mcp_tool("absent", tool, arguments, false);
        assert!(output.status.success());
        let response = messages.iter().find(|m| m["id"] == 3).expect("response");
        assert!(response.get("error").is_some() || response["result"]["isError"] == true);
        assert!(!response.to_string().contains("session_absent"));
    }
    for (scenario, action, id, diagnostic) in [
        ("feat-denied", "show", "17", "access_denied"),
        ("feat-rejected", "show", "17", "session_rejected"),
        ("feat-expired", "show", "17", "session_expired"),
        ("feat-malformed", "show", "17", "invalid feat response"),
        ("feat-incomplete", "show", "17", "not complete"),
        ("feat-duplicate", "show", "17", "conflicting"),
        ("feat-invalid-source", "show", "17", "invalid feat response"),
        ("feat-unsupported", "show", "17", "unsupported"),
        ("feat-oversized", "show", "17", "BodyTooLarge"),
        ("feat-normal", "show", "999", "not found"),
        ("feat-denied", "list", "", "access_denied"),
        ("feat-incomplete", "list", "", "unsupported"),
    ] {
        let (command, tool, arguments) = if action == "show" {
            (vec!["feat", action, id], "feat_show", json!({"id":id}))
        } else {
            (vec!["feat", action, "-o", "json"], "feat_list", json!({}))
        };
        let output = cli(scenario, &command);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let cli_error = String::from_utf8(output.stderr).expect("diagnostic");
        assert!(cli_error.contains(diagnostic), "{cli_error}");
        let (output, messages) = mcp_tool(scenario, tool, arguments, false);
        assert!(output.status.success());
        let result = &messages.iter().find(|m| m["id"] == 3).expect("response")["result"];
        assert_eq!(result["isError"], true, "{result}");
        let error = result["content"][0]["text"].as_str().expect("error");
        assert!(error.contains(diagnostic), "{error}");
        assert!(cli_error.contains(error));
    }
    for (tool, args) in [("feat_list", json!({})), ("feat_show", json!({"id":"17"}))] {
        let (output, messages) = mcp_tool("feat-waiting", tool, args, true);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(messages.iter().any(|m| m["id"] == 4));
        assert!(
            !messages
                .iter()
                .any(|m| m["id"] == 3 && m["result"]["isError"] == false)
        );
    }
}

/// Checks spell CLI/MCP parity, strict inputs, no partial output, and cancellation.
fn spell_contracts() {
    for (scenario, command, tool, arguments) in [
        (
            "spell-normal",
            vec!["spell", "list", "-o", "json"],
            "spell_list",
            json!({}),
        ),
        (
            "spell-empty",
            vec!["spell", "list", "-o", "json"],
            "spell_list",
            json!({}),
        ),
        (
            "spell-next",
            vec![
                "spell",
                "list",
                "--class-id",
                "17",
                "--class-id",
                "18",
                "--level",
                "0",
                "--level",
                "9",
                "--verbal",
                "false",
                "--concentration",
                "true",
                "--ritual",
                "false",
                "-o",
                "json",
            ],
            "spell_list",
            json!({"class_ids":[17,18],"levels":[0,9],"verbal":false,"concentration":true,"ritual":false}),
        ),
        (
            "spell-normal",
            vec!["spell", "list", "--page", "2", "--level", "9", "-o", "json"],
            "spell_list",
            json!({"page":2,"levels":[9]}),
        ),
        (
            "spell-normal",
            vec![
                "spell",
                "list",
                "--school-id",
                "9",
                "--school-id",
                "10",
                "-o",
                "json",
            ],
            "spell_list",
            json!({"school_ids":[9,10]}),
        ),
        (
            "spell-normal",
            vec!["spell", "show", "900001-invented-spell"],
            "spell_show",
            json!({"slug":"900001-invented-spell"}),
        ),
    ] {
        let cli = cli(scenario, &command);
        assert!(
            cli.status.success(),
            "{}",
            String::from_utf8_lossy(&cli.stderr)
        );
        assert!(cli.stderr.is_empty());
        let cli_value: Value = serde_json::from_slice(&cli.stdout).expect("CLI JSON");
        let (output, messages) = mcp_tool(scenario, tool, arguments, false);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let result = &messages.iter().find(|m| m["id"] == 3).expect("response")["result"];
        assert_ne!(result["isError"], true, "{result}");
        let mcp_value: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().expect("text"))
                .expect("MCP JSON");
        assert_eq!(cli_value, mcp_value);
    }
    for (scenario, action) in [
        ("denied", "list"),
        ("rejected", "list"),
        ("expired", "list"),
        ("malformed", "list"),
        ("incomplete", "list"),
        ("missing-next-href", "list"),
        ("duplicate", "list"),
        ("limit", "list"),
        ("oversized", "list"),
        ("redirect", "list"),
        ("missing", "show"),
        ("unsupported", "show"),
        ("wrong-identity", "show"),
        ("wrong-canonical", "show"),
        ("missing-markers", "show"),
        ("unsafe-detail", "show"),
    ] {
        let scenario = format!("spell-{scenario}");
        let command = if action == "list" {
            vec!["spell", "list", "-o", "json"]
        } else {
            vec!["spell", "show", "900001-invented-spell"]
        };
        let cli = cli(&scenario, &command);
        assert_eq!(cli.status.code(), Some(1), "{scenario}");
        assert!(cli.stdout.is_empty(), "{scenario}");
        let (output, messages) = mcp_tool(
            &scenario,
            if action == "list" {
                "spell_list"
            } else {
                "spell_show"
            },
            if action == "list" {
                json!({})
            } else {
                json!({"slug":"900001-invented-spell"})
            },
            false,
        );
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let result = &messages
            .iter()
            .find(|m| m["id"] == 3)
            .expect("error response")["result"];
        assert_eq!(result["isError"], true);
        assert_eq!(
            String::from_utf8_lossy(&cli.stderr).trim(),
            format!(
                "error: {}",
                result["content"][0]["text"].as_str().expect("error")
            )
        );
    }
    for (tool, arguments) in [
        ("spell_list", json!({"unknown":1})),
        ("spell_list", json!({"name":null})),
        ("spell_list", json!({"verbal":null})),
        ("spell_list", json!({"class_ids":[17,17]})),
        ("spell_list", json!({"levels":[10]})),
        ("spell_list", json!({"levels":[0,0]})),
        ("spell_list", json!({"school_ids":[2]})),
        ("spell_list", json!({"school_ids":[11]})),
        ("spell_list", json!({"page":0})),
        ("spell_show", json!({"slug":"17"})),
        ("spell_show", json!({"slug":"01-invented"})),
        ("spell_show", json!({"slug":"17-a%2fb"})),
        (
            "spell_show",
            json!({"slug":"https://www.dndbeyond.com/spells/17-a"}),
        ),
        ("spell_show", json!({"slug":"17-a","unknown":1})),
    ] {
        let (output, messages) = mcp_tool("absent", tool, arguments, false);
        assert!(output.status.success());
        let result = &messages
            .iter()
            .find(|m| m["id"] == 3)
            .expect("strict response")["result"];
        assert_eq!(result["isError"], true);
        assert!(!result.to_string().contains("session_absent"));
    }
    for format in ["table", "tsv"] {
        let output = cli("spell-normal", &["spell", "list", "-o", format]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("next_page"));
        let output = cli(
            "spell-normal",
            &["spell", "list", "-o", format, "--no-headers"],
        );
        assert!(output.status.success());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("next_page"));
    }
    for (tool, args) in [
        ("spell_list", json!({})),
        ("spell_show", json!({"slug":"900001-invented-spell"})),
    ] {
        let (output, messages) = mcp_tool("spell-waiting", tool, args, true);
        assert!(output.status.success());
        assert!(messages.iter().any(|m| m["id"] == 4));
        assert!(
            !messages
                .iter()
                .any(|m| m["id"] == 3 && m["result"]["isError"] == false)
        );
    }
}

/// Selects a synthetic child before running parent checks; no native entry is accessed.
fn main() {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("--spell-contracts") => spell_contracts(),
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
            character_campaign_contracts();
            class_table_limit_contracts();
            class_contracts();
            species_contracts();
            monster_contracts();
            background_contracts();
            background_table_limit_contracts();
            spell_contracts();
            feat_contracts();
        }
    }
}
