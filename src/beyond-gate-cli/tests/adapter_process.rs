//! Self-spawning offline contract checks; child arguments exist only in this test.
//!
//! The child uses the shipped parser and runner with original synthetic core
//! inputs. No fixture mode is compiled into the product binary.

use std::{
    io::{self, Write},
    process::{Command, ExitCode, Output},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use beyond_gate_cli::ReadOperations;
use beyond_gate_core::{
    book::{
        self, BookError, BookOutline, OutlineEntryRecord, OutlineRecord, SectionChunk,
        SectionRecord, SectionRequest,
    },
    character_campaign::{
        self, Campaign, CampaignPage, CampaignRecord, CampaignSnapshot, Character,
        CharacterCampaignError, CharacterPage, CharacterRecord, CharacterSnapshot,
    },
    library::{
        self, AccessEvidence, AccessState, BookRecord, LibraryError, LibraryPage, LibrarySnapshot,
    },
};
use serde_json::{Value, json};

/// Selects synthetic source behavior only in this integration-test executable.
struct Fixtures {
    scenario: String,
}

impl Fixtures {
    /// Supplies original text with no account data or proprietary content.
    fn text(&self) -> String {
        match self.scenario.as_str() {
            "empty" => String::new(),
            "exact" => "x".repeat(book::MAX_SECTION_BYTES),
            "unicode" => "\u{754c}".repeat(2_000),
            _ => "x".repeat(5_000),
        }
    }

    /// Checks the current synthetic access state before a section or outline load.
    fn access(&self) -> Result<AccessState, BookError> {
        match self.scenario.as_str() {
            "denied" => Ok(AccessState::Unavailable),
            "unknown" => Ok(AccessState::Unknown),
            "unavailable" => Err(BookError::Unavailable),
            "unsupported-section" => Err(BookError::UnsupportedSection),
            "malformed" => Err(BookError::InvalidMetadata),
            _ => Ok(AccessState::Granted),
        }
    }
}

impl ReadOperations for Fixtures {
    /// Builds validated characters with optional metadata and unknown JSON fields.
    fn list_characters(&self) -> Result<CharacterSnapshot, CharacterCampaignError> {
        if self.scenario == "malformed" {
            return Err(CharacterCampaignError::InvalidMetadata);
        }
        let entries = if self.scenario == "empty" {
            vec![]
        } else {
            vec![
                CharacterRecord {
                    id: "hero".into(),
                    name: Some("Invented \u{754c} hero".into()),
                    source_url: Some("https://example.com/hero".into()),
                    raw: json!({"unknown": {"nested": [1, null, {"original": "\u{754c}"}]}, "level": 3}),
                },
                CharacterRecord {
                    id: "unnamed".into(),
                    name: None,
                    source_url: None,
                    raw: json!({"preserved": true}),
                },
            ]
        };
        character_campaign::assemble_characters([Ok(CharacterPage {
            cursor: None,
            entries,
            next_cursor: None,
        })])
    }

    /// Looks up only the explicit synthetic selection.
    fn get_character(&self, id: &str) -> Result<Character, CharacterCampaignError> {
        self.list_characters()?
            .get(id)
            .cloned()
            .ok_or(CharacterCampaignError::InvalidMetadata)
    }

    /// Preserves synthetic campaign associations and nested source fields.
    fn list_campaigns(&self) -> Result<CampaignSnapshot, CharacterCampaignError> {
        if self.scenario == "malformed" {
            return Err(CharacterCampaignError::InvalidMetadata);
        }
        let entries = if self.scenario == "empty" {
            vec![]
        } else {
            vec![CampaignRecord {
                id: "campaign".into(),
                name: None,
                source_url: None,
                character_ids: vec!["hero".into(), "hero\\two".into()],
                raw: json!({"unknown_campaign": {"items": [true, 2]}}),
            }]
        };
        character_campaign::assemble_campaigns([Ok(CampaignPage {
            cursor: None,
            entries,
            next_cursor: None,
        })])
    }

    /// Selects one synthetic campaign without inferring character visibility.
    fn get_campaign(&self, id: &str) -> Result<Campaign, CharacterCampaignError> {
        self.list_campaigns()?
            .get(id)
            .cloned()
            .ok_or(CharacterCampaignError::InvalidMetadata)
    }

    /// Assembles independent purchase and sharing evidence without a later grant.
    fn discover_library(&self) -> Result<LibrarySnapshot, LibraryError> {
        if self.scenario == "malformed" {
            return Err(LibraryError::InvalidMetadata);
        }
        let evidence = [
            AccessEvidence::Purchased {
                state: AccessState::Granted,
            },
            AccessEvidence::Shared {
                context_id: "campaign".into(),
                state: AccessState::Granted,
            },
        ];
        let mut entries = Vec::new();
        if self.scenario != "empty" {
            for evidence in evidence {
                entries.push(BookRecord {
                    id: "book".into(),
                    title: "An invented book".into(),
                    source_url: "https://example.com/book".into(),
                    evidence,
                });
            }
        }
        library::assemble_library([Ok(LibraryPage {
            cursor: None,
            entries,
            next_cursor: None,
        })])
    }

    /// Uses the actual core outline validation after checking synthetic access.
    fn get_book_outline(&self, id: &str) -> Result<BookOutline, BookError> {
        book::assemble_outline(
            id,
            |_| self.access(),
            |selected| {
                Ok(OutlineRecord {
                    book_id: selected.into(),
                    source_url: "https://example.com/book".into(),
                    entries: if self.scenario == "empty" {
                        vec![]
                    } else {
                        vec![OutlineEntryRecord {
                            id: "section".into(),
                            title: "An invented opening".into(),
                            parent_id: None,
                            source_url: "https://example.com/book#section".into(),
                        }]
                    },
                })
            },
        )
    }

    /// Uses actual core chunking and fresh access checks, including continuation.
    fn read_book_section(&self, request: SectionRequest<'_>) -> Result<SectionChunk, BookError> {
        book::read_section(
            request,
            |_, _| self.access(),
            |book_id, section_id| {
                Ok(SectionRecord {
                    book_id: book_id.into(),
                    section_id: section_id.into(),
                    revision: match self.scenario.as_str() {
                        "changed-revision" => "v2".into(),
                        "huge-cursor" => "r".repeat(20_000),
                        _ => "v1".into(),
                    },
                    source_url: if self.scenario == "changed-url" {
                        "https://example.com/changed#section".into()
                    } else {
                        "https://example.com/book#section".into()
                    },
                    text: self.text(),
                })
            },
        )
    }
}

/// Starts a fresh offline child for each observable stdout/stderr/exit contract.
fn run(scenario: &str, arguments: &[&str]) -> Output {
    Command::new(std::env::current_exe().expect("test path"))
        .args(["--fixture-child", scenario])
        .args(arguments)
        .output()
        .expect("offline child")
}

/// Parses an independently emitted successful JSON document.
fn read_json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).expect("complete JSON result")
}

/// Requires no result bytes on failure and checks that the diagnostic is useful.
fn failure(output: Output, code: i32, message: &str) {
    assert_eq!(output.status.code(), Some(code));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(message),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Checks list fields, raw preservation, optional values, and all header controls.
fn list_contracts() {
    let lists: &[(&[&str], &str)] = &[
        (&["character", "list"], "id\tname\tsource_url\n"),
        (
            &["campaign", "list"],
            "id\tname\tsource_url\tcharacter_ids\n",
        ),
        (&["book", "list"], "id\ttitle\taccess\tsource_url\n"),
        (
            &["book", "outline", "book"],
            "id\ttitle\tparent_id\tsource_url\n",
        ),
    ];
    for (arguments, header) in lists {
        let output = run("normal", arguments);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(!output.stdout.contains(&b'\t'));
        let table = String::from_utf8_lossy(&output.stdout);
        let table_header = table.lines().next().expect("table header");
        assert_eq!(
            table_header.split_whitespace().collect::<Vec<_>>(),
            header.trim_end().split('\t').collect::<Vec<_>>(),
        );
        assert!(table.lines().count() > 1);
        let mut tsv_args = arguments.to_vec();
        tsv_args.extend(["--format", "tsv"]);
        let output = run("normal", &tsv_args);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).starts_with(header));
        assert_eq!(run("empty", &tsv_args).stdout, header.as_bytes());
        tsv_args.push("--no-headers");
        let output = run("normal", &tsv_args);
        assert!(output.status.success());
        assert!(!String::from_utf8_lossy(&output.stdout).starts_with(header));
        let output = run("empty", &tsv_args);
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        let mut table_args = arguments.to_vec();
        table_args.push("--no-headers");
        assert!(run("empty", &table_args).stdout.is_empty());
        let mut json_args = arguments.to_vec();
        json_args.extend(["--format", "json"]);
        let value = read_json(&run("empty", &json_args));
        if arguments[1] == "outline" {
            assert_eq!(value["entries"], json!([]));
            assert_eq!(value["book_id"], "book");
        } else {
            assert_eq!(value, json!([]));
        }
        json_args.push("--no-headers");
        failure(run("normal", &json_args), 2, "--no-headers");
    }
    let characters = read_json(&run("normal", &["character", "list", "--format", "json"]));
    assert_eq!(
        characters[0]["raw"]["unknown"]["nested"][2]["original"],
        "\u{754c}"
    );
    assert!(characters[1]["name"].is_null());
    assert!(characters[1]["source_url"].is_null());
    let shown = read_json(&run("normal", &["character", "show", "hero"]));
    assert_eq!(shown, characters[0]);
    let tsv = run("normal", &["character", "list", "--format", "tsv"]);
    assert!(String::from_utf8_lossy(&tsv.stdout).ends_with("unnamed\t\t\n"));
    let campaigns = read_json(&run("normal", &["campaign", "list", "--format", "json"]));
    assert_eq!(campaigns[0]["character_ids"], json!(["hero", "hero\\two"]));
    assert_eq!(
        read_json(&run("normal", &["campaign", "show", "campaign"])),
        campaigns[0]
    );
    let books = read_json(&run("normal", &["book", "list", "--format", "json"]));
    assert_eq!(books[0]["access"], "purchased_and_shared");
    assert_eq!(books[0]["evidence"][0]["kind"], "purchased");
    assert_eq!(books[0]["evidence"][1]["context_id"], "campaign");
    let outline = read_json(&run(
        "normal",
        &["book", "outline", "book", "--format", "json"],
    ));
    assert_eq!(outline["entries"][0]["id"], "section");
    assert!(outline["entries"][0]["parent_id"].is_null());
    for arguments in [
        vec!["character", "list", "--format", "json"],
        vec!["campaign", "list", "--format", "tsv"],
        vec!["book", "list"],
    ] {
        failure(run("malformed", &arguments), 1, "metadata is invalid");
    }
}

/// Reconstructs section text across separate processes with explicit tokens only.
fn section_contracts() {
    for scenario in ["normal", "unicode", "exact", "empty"] {
        let expected = Fixtures {
            scenario: scenario.into(),
        }
        .text();
        let mut text = String::new();
        let mut token: Option<String> = None;
        loop {
            let mut arguments = vec!["book", "read", "book", "section"];
            if let Some(token) = token.as_deref() {
                arguments.extend(["--continuation", token]);
            }
            let value = read_json(&run(scenario, &arguments));
            let chunk = value["text"].as_str().expect("text");
            assert!(chunk.len() <= book::MAX_SECTION_BYTES);
            assert_eq!(value["max_text_bytes"], 4096);
            assert_eq!(value["book_id"], "book");
            assert_eq!(value["section_id"], "section");
            assert_eq!(value["source_url"], "https://example.com/book#section");
            text.push_str(chunk);
            if value["is_complete"] == true {
                assert!(value["next_cursor"].is_null());
                break;
            }
            assert!(!chunk.is_empty());
            token = Some(value["next_cursor"].as_str().expect("token").to_owned());
            assert!(text.len() < expected.len());
        }
        assert_eq!(text, expected);
    }
    let value = read_json(&run("normal", &["book", "read", "book", "section"]));
    assert_eq!(value["text"].as_str().expect("text").len(), 4096);
    assert_eq!(value["is_complete"], false);
    let token = value["next_cursor"].as_str().expect("continuation");
    cursor_rejections(token);
    for (scenario, diagnostic) in [
        ("denied", "access is denied"),
        ("unknown", "not established"),
        ("changed-revision", "older or changed"),
        ("changed-url", "older or changed"),
        ("unavailable", "unavailable"),
        ("unsupported-section", "does not support"),
        ("malformed", "metadata is invalid"),
    ] {
        failure(
            run(
                scenario,
                &["book", "read", "book", "section", "--continuation", token],
            ),
            1,
            diagnostic,
        );
    }
    failure(
        run(
            "normal",
            &[
                "book",
                "read",
                "other-book",
                "section",
                "--continuation",
                token,
            ],
        ),
        1,
        "scope or position",
    );
    failure(
        run(
            "normal",
            &[
                "book",
                "read",
                "book",
                "other-section",
                "--continuation",
                token,
            ],
        ),
        1,
        "scope or position",
    );
    failure(
        run("huge-cursor", &["book", "read", "book", "section"]),
        1,
        "16 KiB",
    );
    let invalid = "private-invalid-token";
    let output = run(
        "normal",
        &["book", "read", "book", "section", "--continuation", invalid],
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains(invalid));
    failure(output, 2, "continuation");
    let oversized = "A".repeat(book::MAX_CURSOR_BYTES + 1);
    failure(
        run(
            "normal",
            &[
                "book",
                "read",
                "book",
                "section",
                "--continuation",
                &oversized,
            ],
        ),
        2,
        "16 KiB",
    );
    for scenario in [
        "denied",
        "unknown",
        "unavailable",
        "unsupported-section",
        "malformed",
    ] {
        let output = run(scenario, &["book", "outline", "book"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

/// Exercises wire-bound and source-position failures through fresh CLI processes.
fn cursor_rejections(original: &str) {
    let payload = URL_SAFE_NO_PAD
        .decode(original.strip_prefix("bg1.").expect("version"))
        .expect("base64");
    let original_wire: Value = serde_json::from_slice(&payload).expect("wire JSON");
    for (field, value, code, message) in [
        ("next_byte", json!(0), 2, "scope or position"),
        ("next_byte", json!(5_000), 1, "scope or position"),
        ("next_byte", json!(5_001), 1, "scope or position"),
        ("extra", json!(true), 2, "scope or position"),
        ("revision", json!("changed"), 1, "older or changed"),
    ] {
        let mut wire = original_wire.clone();
        wire[field] = value;
        let token = format!("bg1.{}", URL_SAFE_NO_PAD.encode(wire.to_string()));
        failure(
            run(
                "normal",
                &["book", "read", "book", "section", "--continuation", &token],
            ),
            code,
            message,
        );
    }
    let mut wire = original_wire.clone();
    wire["next_byte"] = json!(1);
    let token = format!("bg1.{}", URL_SAFE_NO_PAD.encode(wire.to_string()));
    failure(
        run(
            "unicode",
            &["book", "read", "book", "section", "--continuation", &token],
        ),
        1,
        "scope or position",
    );
    let mut wire = original_wire;
    let length = wire.to_string().len();
    wire["revision"] = json!("r".repeat(12_285 - length + 2));
    let token = format!("bg1.{}", URL_SAFE_NO_PAD.encode(wire.to_string()));
    assert_eq!(token.len(), book::MAX_CURSOR_BYTES);
    failure(
        run(
            "normal",
            &["book", "read", "book", "section", "--continuation", &token],
        ),
        1,
        "older or changed",
    );
    failure(
        run(
            "normal",
            &[
                "book",
                "read",
                "book",
                "section",
                "--continuation",
                &(token + "A"),
            ],
        ),
        2,
        "16 KiB",
    );
    failure(
        run(
            "normal",
            &[
                "book",
                "read",
                "book",
                "section",
                "--continuation",
                &original.replacen("bg1.", "bg2.", 1),
            ],
        ),
        2,
        "scope or position",
    );
}

/// Simulates an unavailable output stream without a platform-specific broken pipe.
struct BrokenWriter;

impl Write for BrokenWriter {
    /// Fails every write instead of silently consuming output.
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "synthetic output failure",
        ))
    }

    /// Completes flush without adding another artificial failure.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Accepts result bytes but fails when the runner flushes them.
struct BrokenFlush;

impl Write for BrokenFlush {
    /// Accepts all bytes so only flush behavior determines the result.
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        Ok(bytes.len())
    }

    /// Reports an explicit flush failure rather than a false successful result.
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("synthetic flush failure"))
    }
}

/// Checks writer failures for results, help, completions, and diagnostics.
fn output_failures() {
    for arguments in [
        vec!["beyond-gate", "character", "list"],
        vec!["beyond-gate", "--help"],
        vec!["beyond-gate", "completions", "powershell"],
    ] {
        let mut stderr = Vec::new();
        let code = beyond_gate_cli::run(
            arguments,
            &Fixtures {
                scenario: "normal".into(),
            },
            &mut BrokenWriter,
            &mut stderr,
        );
        assert_eq!(code, 1);
        assert!(String::from_utf8_lossy(&stderr).contains("could not be written"));
    }
    let mut stderr = Vec::new();
    assert_eq!(
        beyond_gate_cli::run(
            ["beyond-gate", "character", "list"],
            &Fixtures {
                scenario: "normal".into()
            },
            &mut BrokenFlush,
            &mut stderr,
        ),
        1
    );
    assert!(String::from_utf8_lossy(&stderr).contains("could not be written"));
    assert_eq!(
        beyond_gate_cli::run(
            ["beyond-gate", "book", "read", "book", "section"],
            &Fixtures {
                scenario: "denied".into()
            },
            &mut Vec::new(),
            &mut BrokenWriter,
        ),
        1
    );
}

/// Runs parent assertions or a child with the real command runner and synthetic reads.
fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().collect();
    if arguments
        .get(1)
        .is_some_and(|argument| argument == "--fixture-child")
    {
        let scenario = arguments
            .get(2)
            .expect("scenario")
            .to_string_lossy()
            .into_owned();
        let cli_arguments = std::iter::once(std::ffi::OsString::from("beyond-gate"))
            .chain(arguments.into_iter().skip(3));
        return ExitCode::from(beyond_gate_cli::run(
            cli_arguments,
            &Fixtures { scenario },
            &mut io::stdout().lock(),
            &mut io::stderr().lock(),
        ));
    }
    list_contracts();
    section_contracts();
    output_failures();
    ExitCode::SUCCESS
}
