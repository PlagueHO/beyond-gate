//! Self-spawning offline contract checks; child arguments exist only in this test.
//!
//! The child uses the shipped parser and runner with original synthetic core
//! inputs. No fixture mode is compiled into the product binary.

use std::{
    io::{self, Write},
    process::{Command, Output},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use beyond_gate_cli::ReadOperations;
use beyond_gate_core::{
    auth::session::Session,
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
    /// Supplies synthetic website metadata for two species.
    async fn list_species(
        &self,
        _session: &Session,
    ) -> Result<Value, beyond_gate_core::species::SpeciesError> {
        if self.scenario == "empty" {
            return Err(beyond_gate_core::species::SpeciesError::IncompleteResult);
        }
        Ok(json!([
            {"id":"17","name":"Invented Origin","source":"Invented Source",
                "source_url":"https://www.dndbeyond.com/species/17-invented","is_legacy":false},
            {"id":"18","name":"Invented Coastal Origin","source":"Invented Source",
                "source_url":"https://www.dndbeyond.com/species/18-invented","is_legacy":true}
        ]))
    }

    /// Supplies one selected invented record with an unknown nested field.
    async fn show_species(
        &self,
        _session: &Session,
        id: &str,
    ) -> Result<Value, beyond_gate_core::species::SpeciesError> {
        if id != "18" || self.scenario == "empty" {
            return Err(beyond_gate_core::species::SpeciesError::MissingRecord);
        }
        Ok(json!({
            "id":"18","name":"Invented Coastal Origin","is_homebrew":false,
            "is_legacy":true,"is_subrace":true,"base_race_name":"Invented Origin",
            "raw":{"entityRaceId":18,"fullName":"Invented Coastal Origin",
                "isLegacy":true,"isSubRace":true,"future_definition":{"nested":[true,null,3]}}
        }))
    }

    /// Uses the shared background listing with the supplied synthetic owner.
    async fn list_backgrounds(
        &self,
        session: &Session,
        query: &beyond_gate_core::background::BackgroundQuery,
    ) -> Result<Value, beyond_gate_core::background::BackgroundError> {
        beyond_gate_core::background::list(session, query).await
    }
    /// Uses shared background selection without a product fixture switch.
    async fn show_background(
        &self,
        session: &Session,
        id: &str,
    ) -> Result<Value, beyond_gate_core::background::BackgroundError> {
        beyond_gate_core::background::show(session, id).await
    }
    /// Uses the shared monster page operation without a product fixture selector.
    async fn list_monsters(
        &self,
        session: &Session,
        query: &beyond_gate_core::monster::MonsterQuery,
    ) -> Result<Value, beyond_gate_core::monster::MonsterError> {
        beyond_gate_core::monster::list(session, query).await
    }

    /// Uses shared selected detail, including its access gate.
    async fn show_monster(
        &self,
        session: &Session,
        id: &str,
    ) -> Result<Value, beyond_gate_core::monster::MonsterError> {
        beyond_gate_core::monster::show(session, id).await
    }
    /// Uses the shared metadata operation with the explicitly supplied fixture owner.
    async fn list_classes(
        &self,
        session: &Session,
        kind: beyond_gate_core::class_catalog::ClassKind,
        name: Option<&str>,
        class_id: Option<&str>,
    ) -> Result<Value, beyond_gate_core::class_catalog::ClassCatalogError> {
        beyond_gate_core::class_catalog::list(session, kind, name, class_id).await
    }

    /// Uses shared metadata selection without creating a success-shaped fixture fallback.
    async fn show_class(
        &self,
        session: &Session,
        kind: beyond_gate_core::class_catalog::ClassKind,
        id: &str,
        class_id: Option<&str>,
    ) -> Result<Value, beyond_gate_core::class_catalog::ClassCatalogError> {
        beyond_gate_core::class_catalog::show(session, kind, id, class_id).await
    }

    /// Builds validated characters with optional metadata and unknown JSON fields.
    async fn list_characters(
        &self,
        _session: &Session,
    ) -> Result<CharacterSnapshot, CharacterCampaignError> {
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
    async fn get_character(
        &self,
        _session: &Session,
        id: &str,
    ) -> Result<Character, CharacterCampaignError> {
        self.list_characters(_session)
            .await?
            .get(id)
            .cloned()
            .ok_or(CharacterCampaignError::InvalidMetadata)
    }

    /// Preserves synthetic campaign associations and nested source fields.
    async fn list_campaigns(
        &self,
        _session: &Session,
        _active_only: bool,
    ) -> Result<CampaignSnapshot, CharacterCampaignError> {
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
                character_ids: Some(vec!["hero".into(), "hero\\two".into()]),
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
    async fn get_campaign(
        &self,
        _session: &Session,
        id: &str,
    ) -> Result<Campaign, CharacterCampaignError> {
        self.list_campaigns(_session, false)
            .await?
            .get(id)
            .cloned()
            .ok_or(CharacterCampaignError::InvalidMetadata)
    }

    /// Assembles independent purchase and sharing evidence without a later grant.
    async fn discover_library(
        &self,
        _session: &beyond_gate_core::auth::session::Session,
    ) -> Result<LibrarySnapshot, LibraryError> {
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
    async fn get_book_outline(
        &self,
        _session: &beyond_gate_core::auth::session::Session,
        id: &str,
    ) -> Result<BookOutline, BookError> {
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
    async fn read_book_section(
        &self,
        _session: &beyond_gate_core::auth::session::Session,
        request: SectionRequest<'_>,
    ) -> Result<SectionChunk, BookError> {
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
    for (resource, expected) in [("character", &characters), ("campaign", &campaigns)] {
        for option in ["--output", "-o", "--format"] {
            let value = read_json(&run("normal", &[resource, "list", option, "json"]));
            assert_eq!(&value, expected, "{resource} output option {option}");
        }
    }
    let character_tsv = run("normal", &["character", "show", "hero", "--output", "tsv"]);
    let character_tsv = String::from_utf8_lossy(&character_tsv.stdout);
    assert!(character_tsv.starts_with("id\tname\tsource_url\traw\n"));
    assert!(character_tsv.contains(r#""unknown":{"nested""#));
    let campaign_tsv = run(
        "normal",
        &["campaign", "show", "campaign", "-o", "tsv", "--no-headers"],
    );
    let campaign_tsv = String::from_utf8_lossy(&campaign_tsv.stdout);
    assert!(!campaign_tsv.starts_with("id\t"));
    assert!(campaign_tsv.contains(r#""unknown_campaign""#));
    failure(
        run(
            "normal",
            &[
                "campaign",
                "show",
                "campaign",
                "--output",
                "json",
                "--no-headers",
            ],
        ),
        2,
        "--no-headers",
    );
    let active_campaigns = read_json(&run(
        "normal",
        &["campaign", "list", "--active-only", "--output", "json"],
    ));
    assert_eq!(active_campaigns, campaigns);
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

/// Checks species table/JSON output, selected raw preservation, and missing IDs.
fn species_contracts() {
    let rows = read_json(&run("normal", &["species", "list", "--output", "json"]));
    assert_eq!(rows.as_array().expect("species rows").len(), 2);
    assert_eq!(rows[1]["is_legacy"], true);
    assert_eq!(rows[1]["source"], "Invented Source");
    let shown = read_json(&run("normal", &["species", "show", "18"]));
    assert_eq!(shown["id"], "18");
    assert_eq!(
        shown["raw"]["future_definition"]["nested"],
        json!([true, null, 3])
    );
    let tsv = run("normal", &["species", "list", "--format", "tsv"]);
    assert!(tsv.status.success());
    let tsv_output = String::from_utf8_lossy(&tsv.stdout);
    assert!(
        tsv_output.starts_with("id\tname\tsource\tsource_url\tis_legacy\n"),
        "{tsv_output}"
    );
    failure(
        run("normal", &["species", "show", "17"]),
        1,
        "species record not found",
    );
    failure(
        run("empty", &["species", "list", "-o", "json"]),
        1,
        "species result is incomplete",
    );
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

/// Checks actual core spell envelopes and selected HTML through the CLI process.
fn spell_contracts() {
    let value = read_json(&run(
        "spell-next",
        &[
            "spell",
            "list",
            "--level",
            "0",
            "--class-id",
            "17",
            "-o",
            "json",
        ],
    ));
    assert_eq!(value["entries"][0]["id"], "900001");
    assert_eq!(value["entries"][0]["level"], 0);
    assert_eq!(value["pagination"], json!({"page":1,"next_page":2}));
    let value = read_json(&run("spell-empty", &["spell", "list", "-o", "json"]));
    assert_eq!(value["entries"], json!([]));
    let value = read_json(&run(
        "spell-normal",
        &["spell", "show", "900001-invented-spell"],
    ));
    assert!(
        value["raw_html"]
            .as_str()
            .expect("HTML")
            .contains("Unknown invented element")
    );
    assert!(
        !value["raw_html"]
            .as_str()
            .expect("HTML")
            .contains("Outside")
    );
    failure(
        run("spell-denied", &["spell", "list", "-o", "json"]),
        1,
        "access_denied",
    );
    failure(run("spell-normal", &["spell", "show", "17"]), 2, "slug");
}

/// Checks real feat CLI output against shared invented website and collection sources.
fn feat_contracts() {
    let rows = read_json(&run("feat-normal", &["feat", "list", "-o", "json"]));
    assert_eq!(rows.as_array().expect("rows").len(), 2);
    assert!(rows[0].get("raw").is_none());
    let value = read_json(&run("feat-normal", &["feat", "show", "18"]));
    assert_eq!(value["is_repeatable"], true);
    assert_eq!(value["repeatable_parent_id"], "17");
    assert_eq!(value["raw"]["future"]["nested"], json!([true, null, 3]));
    assert_eq!(value["sources"][0]["future"], true);
    assert_eq!(
        read_json(&run("feat-empty", &["feat", "list", "-o", "json"])),
        json!([])
    );
    let output = run(
        "feat-normal",
        &["feat", "list", "--format", "tsv", "--no-headers"],
    );
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("TSV");
    assert_eq!(text.lines().count(), 2);
    assert!(!text.contains("source_url"));
    let output = run("feat-normal", &["feat", "show", "18", "--format", "tsv"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("TSV");
    assert_eq!(text.lines().count(), 2);
    assert!(!text.contains('\u{1b}'));
    failure(
        run("feat-denied", &["feat", "list", "-o", "json"]),
        1,
        "access_denied",
    );
    failure(run("feat-normal", &["feat", "show", "999"]), 1, "not found");
}

/// Checks equipment pages, raw preservation, TSV safety, and page-scoped failures.
fn equipment_contracts() {
    let value = read_json(&run(
        "equipment-normal",
        &["equipment", "list", "-o", "json"],
    ));
    assert_eq!(value["entries"].as_array().expect("entries").len(), 50);
    assert_eq!(value["pagination"]["source_count"], 100);
    assert_eq!(value["pagination"]["next_page"], 2);
    let selected = read_json(&run(
        "equipment-normal",
        &["equipment", "show", "7:17", "--page", "1"],
    ));
    assert_eq!(selected["raw"]["future"]["nested"], json!([true, null, 3]));
    assert_eq!(selected["sources"][0]["future"]["ref"], true);
    let empty = read_json(&run(
        "equipment-normal",
        &["equipment", "list", "--page", "3", "-o", "json"],
    ));
    assert_eq!(empty["entries"], json!([]));
    assert_eq!(empty["pagination"]["next_page"], Value::Null);
    let magic = read_json(&run(
        "equipment-magic-only",
        &["equipment", "list", "-o", "json"],
    ));
    assert_eq!(magic["entries"], json!([]));
    assert_eq!(magic["pagination"]["next_page"], 2);
    let output = run(
        "equipment-normal",
        &["equipment", "show", "7:17", "--page", "1", "-o", "tsv"],
    );
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("TSV");
    assert_eq!(text.lines().count(), 2);
    assert!(!text.contains('\u{1b}'));
    failure(
        run("equipment-denied", &["equipment", "list", "-o", "json"]),
        1,
        "access_denied",
    );
    failure(
        run(
            "equipment-normal",
            &["equipment", "show", "7:17", "--page", "2"],
        ),
        1,
        "checked source page",
    );
    failure(
        run(
            "equipment-normal",
            &["equipment", "show", "7:18", "--page", "1"],
        ),
        1,
        "not equipment",
    );
}

/// Checks local magic filters, raw output, escaping, and page-scoped failures.
fn magic_item_contracts() {
    let value = read_json(&run(
        "equipment-normal",
        &[
            "magic-item",
            "list",
            "-o",
            "json",
            "--name",
            "vEnTeD",
            "--type",
            "other GEAR",
            "--rarity",
            "rArE",
            "--tag",
            "UTILITY",
            "--attunement",
            "yes",
        ],
    ));
    assert_eq!(value["entries"].as_array().expect("entries").len(), 17);
    assert_eq!(value["entries"][0]["id"], "7:18");
    assert_eq!(value["filters"]["attunement"], "yes");
    assert_eq!(value["pagination"]["source_count"], 100);
    let empty = read_json(&run(
        "equipment-normal",
        &["magic-item", "list", "-o", "json", "--type", "missing"],
    ));
    assert_eq!(empty["entries"], json!([]));
    assert_eq!(empty["pagination"]["next_page"], 2);
    let detail = read_json(&run(
        "equipment-normal",
        &["magic-item", "show", "7:18", "--page", "1"],
    ));
    assert_eq!(detail["raw"]["magic"], true);
    assert_eq!(detail["raw"]["future"]["nested"], json!([true, null, 3]));
    for action in [
        vec!["magic-item", "list", "-o", "tsv"],
        vec!["magic-item", "show", "7:18", "--page", "1", "-o", "tsv"],
    ] {
        let output = run("equipment-normal", &action);
        assert!(output.status.success());
        assert!(!String::from_utf8_lossy(&output.stdout).contains('\u{1b}'));
    }
    failure(
        run(
            "equipment-normal",
            &["magic-item", "show", "7:17", "--page", "1"],
        ),
        1,
        "not a magic item",
    );
    failure(
        run("equipment-denied", &["magic-item", "list", "-o", "json"]),
        1,
        "access_denied",
    );
}

/// Runs parent assertions or a child with the real command runner and synthetic reads.
fn main() {
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
        if let Some(name) = scenario.strip_prefix("equipment-") {
            use beyond_gate_core::item_catalog::fixture::{self, Scenario};
            let source = match name {
                "normal" => Scenario::Normal,
                "magic-only" => Scenario::MagicOnly,
                "denied" => Scenario::Denied,
                _ => panic!("unknown synthetic equipment source"),
            };
            let session = fixture::session(source).expect("invented item service");
            std::process::exit(beyond_gate_cli::run_with_session_input(
                cli_arguments,
                &beyond_gate_cli::LiveReads,
                &session,
                &mut io::empty(),
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            ));
        }
        if let Some(name) = scenario.strip_prefix("feat-") {
            use beyond_gate_core::feat::fixture::{self, Scenario};
            let source = match name {
                "normal" => Scenario::Normal,
                "empty" => Scenario::Empty,
                "denied" => Scenario::Denied,
                _ => panic!("unknown synthetic feat source"),
            };
            let session = fixture::session(source).expect("invented feat service");
            std::process::exit(beyond_gate_cli::run_with_session_input(
                cli_arguments,
                &beyond_gate_cli::LiveReads,
                &session,
                &mut io::empty(),
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            ));
        }
        if let Some(name) = scenario.strip_prefix("spell-") {
            use beyond_gate_core::spell::fixture::{self, Scenario};
            let source = match name {
                "next" => Scenario::Next,
                "empty" => Scenario::Empty,
                "denied" => Scenario::Denied,
                "normal" => Scenario::Normal,
                _ => panic!("unknown synthetic spell source"),
            };
            let session = fixture::session(source).expect("invented spell service");
            std::process::exit(beyond_gate_cli::run_with_session_input(
                cli_arguments,
                &beyond_gate_cli::LiveReads,
                &session,
                &mut io::empty(),
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            ));
        }
        std::process::exit(beyond_gate_cli::run(
            cli_arguments,
            &Fixtures { scenario },
            &mut io::stdout().lock(),
            &mut io::stderr().lock(),
        ));
    }
    list_contracts();
    species_contracts();
    spell_contracts();
    feat_contracts();
    equipment_contracts();
    magic_item_contracts();
    section_contracts();
    output_failures();
}
