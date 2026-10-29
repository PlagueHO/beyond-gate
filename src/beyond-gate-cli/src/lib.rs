//! CLI parsing and presentation for shared reads, with an offline test boundary.
//!
//! The production read implementation delegates to unsupported live core entry
//! points. Synthetic success paths belong only to test processes.

mod command;
mod error;
mod mcp;
mod output;

use std::{
    ffi::OsString,
    io::{self, BufRead, Write},
};

use beyond_gate_core::{
    auth::{self, AuthError, SessionStatus, SessionStore},
    book::{self, BookError, BookOutline, SectionChunk, SectionCursor, SectionRequest},
    character_campaign::{
        self, Campaign, CampaignSnapshot, Character, CharacterCampaignError, CharacterSnapshot,
    },
    library::{self, LibraryError, LibrarySnapshot},
};
use clap::ArgMatches;
use clap_complete::Shell;
use serde_json::Value;

pub use command::command;
use error::Failure;
use output::ListOptions;

/// Supplies only the shared resource operations required by the command runner.
///
/// Production uses [`LiveReads`]. Tests supply synthetic core results without
/// adding account credentials, fixture flags, or environment modes to the CLI.
pub trait ReadOperations {
    /// Lists characters, or returns the typed shared failure.
    ///
    /// # Errors
    ///
    /// Returns source, validation, access, or unsupported-operation failures.
    fn list_characters(&self) -> Result<CharacterSnapshot, CharacterCampaignError>;
    /// Reads one explicitly selected character without interpreting its rules.
    ///
    /// # Errors
    ///
    /// Returns the shared read failure without an empty-result fallback.
    fn get_character(&self, id: &str) -> Result<Character, CharacterCampaignError>;
    /// Lists campaigns while preserving their supplied character associations.
    ///
    /// # Errors
    ///
    /// Returns the shared discovery failure without partial results.
    fn list_campaigns(&self) -> Result<CampaignSnapshot, CharacterCampaignError>;
    /// Reads one explicitly selected campaign.
    ///
    /// # Errors
    ///
    /// Returns the shared read failure without an empty-result fallback.
    fn get_campaign(&self, id: &str) -> Result<Campaign, CharacterCampaignError>;
    /// Lists library metadata without granting later book access.
    ///
    /// # Errors
    ///
    /// Returns the shared discovery failure without partial results.
    fn discover_library(&self) -> Result<LibrarySnapshot, LibraryError>;
    /// Reads navigation for one book after its source checks current access.
    ///
    /// # Errors
    ///
    /// Returns the shared outline failure without partial navigation.
    fn get_book_outline(&self, id: &str) -> Result<BookOutline, BookError>;
    /// Reads one bounded chunk after a fresh access check, never auto-continuing.
    ///
    /// # Errors
    ///
    /// Returns the shared access, source, or continuation failure.
    fn read_book_section(&self, request: SectionRequest<'_>) -> Result<SectionChunk, BookError>;
}

/// Starts the MCP server or registers it with one supported local client.
fn execute_mcp(
    matches: &ArgMatches,
    stdin: &mut impl BufRead,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> i32 {
    let Some((action, arguments)) = matches.subcommand() else {
        return report(
            &Failure::Usage("Select `mcp start` or `mcp install`."),
            stderr,
        );
    };
    match action {
        "start" => match mcp::start_server() {
            Ok(status) => status.code().unwrap_or(1),
            Err(error) => report(&Failure::Mcp(error), stderr),
        },
        "install" => {
            let client = match arguments.get_one::<String>("client").map(String::as_str) {
                Some("vscode") => mcp::Client::Vscode,
                Some("claude-code") => mcp::Client::ClaudeCode,
                Some("copilot-cli") => mcp::Client::CopilotCli,
                _ => {
                    return report(&Failure::Usage("Select a supported MCP client."), stderr);
                }
            };
            let executable = match mcp::companion_executable() {
                Ok(path) => path,
                Err(error) => return report(&Failure::Mcp(error), stderr),
            };
            let preview_only = arguments.get_flag("dry-run");
            match mcp::install(client, &executable, preview_only, stdin, stdout) {
                Ok(()) => match stdout.flush() {
                    Ok(()) => 0,
                    Err(error) => report(&Failure::Output(error), stderr),
                },
                Err(error) => report(&Failure::Mcp(error), stderr),
            }
        }
        _ => report(
            &Failure::Usage("Select `mcp start` or `mcp install`."),
            stderr,
        ),
    }
}

/// Delegates resource operations to the core's explicit live-read boundary.
pub struct LiveReads;

impl ReadOperations for LiveReads {
    /// Preserves the core character-discovery result and failure classification.
    fn list_characters(&self) -> Result<CharacterSnapshot, CharacterCampaignError> {
        character_campaign::list_characters()
    }

    /// Passes only the explicitly selected identifier to the core.
    fn get_character(&self, id: &str) -> Result<Character, CharacterCampaignError> {
        character_campaign::get_character(id)
    }

    /// Preserves the core campaign-discovery result and failure classification.
    fn list_campaigns(&self) -> Result<CampaignSnapshot, CharacterCampaignError> {
        character_campaign::list_campaigns()
    }

    /// Passes only the selected campaign identifier to the core.
    fn get_campaign(&self, id: &str) -> Result<Campaign, CharacterCampaignError> {
        character_campaign::get_campaign(id)
    }

    /// Uses the library boundary without inventing account data or access.
    fn discover_library(&self) -> Result<LibrarySnapshot, LibraryError> {
        library::discover_library()
    }

    /// Uses the core outline entry point without granting synthetic access.
    fn get_book_outline(&self, id: &str) -> Result<BookOutline, BookError> {
        book::get_book_outline(id)
    }

    /// Preserves the explicit selection and any untrusted continuation claim.
    fn read_book_section(&self, request: SectionRequest<'_>) -> Result<SectionChunk, BookError> {
        book::read_book_section(request)
    }
}

/// Enumerates only the supported local auth operations.
#[derive(Clone, Copy)]
enum AuthAction {
    Login,
    Status,
    Logout,
}

/// Converts clap's validated nested command into a local auth action.
fn auth_action(matches: &ArgMatches) -> Option<AuthAction> {
    match matches.subcommand_matches("auth")?.subcommand_name()? {
        "login" => Some(AuthAction::Login),
        "status" => Some(AuthAction::Status),
        "logout" => Some(AuthAction::Logout),
        _ => None,
    }
}

/// Runs local auth operations through injectable storage calls for offline tests.
///
/// # Errors
///
/// Returns the shared failure rather than a successful status or logout message.
fn execute_auth(
    action: AuthAction,
    status: impl FnOnce() -> Result<SessionStatus, AuthError>,
    logout: impl FnOnce() -> Result<(), AuthError>,
) -> Result<&'static str, AuthError> {
    match action {
        AuthAction::Login => Err(auth::login_blocker()),
        AuthAction::Status => Ok(match status()? {
            SessionStatus::Absent => "absent: no local Beyond Gate session is stored.",
            SessionStatus::StoredUnverified => {
                "stored-unverified: local session material exists; upstream usability is not checked; expiry is unknown."
            }
        }),
        AuthAction::Logout => {
            logout()?;
            Ok(
                "Local Beyond Gate session removed or already absent. Upstream revocation is not implemented; the browser session can remain active.",
            )
        }
    }
}

/// Parses and executes one command, keeping result data separate from diagnostics.
///
/// Returns 0 on success, 1 on operation/output failure, 2 on invalid usage, or
/// the child status for `mcp start`.
/// Help, version, completions, and resource parsing never open native storage.
/// Only explicit auth status/logout can access the protected credential service.
/// Output failures return 1, including when stderr itself cannot be written.
pub fn run<I, T>(
    arguments: I,
    reads: &impl ReadOperations,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let mut stdin = io::stdin().lock();
    run_with_input(arguments, reads, &mut stdin, stdout, stderr)
}

/// Parses and executes one command with an explicit input stream.
///
/// The input stream is used only by the MCP installer confirmation. All other
/// commands keep their existing storage and output behavior.
pub fn run_with_input<I, T>(
    arguments: I,
    reads: &impl ReadOperations,
    stdin: &mut impl BufRead,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = match command().try_get_matches_from(arguments) {
        Ok(matches) => matches,
        Err(error) => {
            let code = if error.exit_code() == 0 { 0 } else { 2 };
            let bytes = error.to_string();
            let result = if error.use_stderr() {
                write_output(stderr, bytes.as_bytes())
            } else {
                write_output(stdout, bytes.as_bytes())
            };
            return match result {
                Ok(()) => code,
                Err(error) => report(&error, stderr),
            };
        }
    };
    if let Some(("mcp", arguments)) = matches.subcommand() {
        return execute_mcp(arguments, stdin, stdout, stderr);
    }
    match execute(&matches, reads).and_then(|bytes| write_output(stdout, &bytes)) {
        Ok(()) => 0,
        Err(error) => report(&error, stderr),
    }
}

/// Writes and flushes through a fallible path rather than panic-prone print macros.
fn write_output(writer: &mut impl Write, bytes: &[u8]) -> Result<(), Failure> {
    writer
        .write_all(bytes)
        .and_then(|()| writer.flush())
        .map_err(Failure::Output)
}

/// Reports one safe failure, returning nonzero even when diagnostics cannot be written.
fn report(error: &Failure, stderr: &mut impl Write) -> i32 {
    match write_output(stderr, format!("error: {error}\n").as_bytes()) {
        Ok(()) => error.exit_code(),
        Err(_) => 1,
    }
}

/// Dispatches the validated command before any presentation is written.
fn execute(matches: &ArgMatches, reads: &impl ReadOperations) -> Result<Vec<u8>, Failure> {
    if let Some(action) = auth_action(matches) {
        let message = execute_auth(
            action,
            || SessionStore::open()?.status(),
            || SessionStore::open()?.logout(),
        )
        .map_err(Failure::Auth)?;
        return Ok(format!("{message}\n").into_bytes());
    }
    let (resource, arguments) = matches.subcommand().ok_or(Failure::Usage(
        "Select an operation; see beyond-gate --help.",
    ))?;
    if resource == "completions" {
        let shell = arguments
            .get_one::<Shell>("shell")
            .ok_or(Failure::Usage("Select a completion shell."))?;
        let mut bytes = Vec::new();
        clap_complete::generate(*shell, &mut command(), "beyond-gate", &mut bytes);
        return Ok(bytes);
    }
    let (action, arguments) = arguments
        .subcommand()
        .ok_or(Failure::Usage("Select a resource operation."))?;
    match resource {
        "character" => execute_character(action, arguments, reads),
        "campaign" => execute_campaign(action, arguments, reads),
        "book" => execute_book(action, arguments, reads),
        _ => Err(Failure::Usage("Select a supported resource operation.")),
    }
}

/// Routes character operations without interpreting raw character fields.
fn execute_character(
    action: &str,
    arguments: &ArgMatches,
    reads: &impl ReadOperations,
) -> Result<Vec<u8>, Failure> {
    match action {
        "list" => {
            let options = ListOptions::parse(arguments)?;
            let snapshot = reads.list_characters().map_err(Failure::Character)?;
            let rows: Vec<_> = snapshot
                .characters()
                .iter()
                .map(output::character)
                .collect();
            render_list(&options, &["id", "name", "source_url"], rows)
        }
        "show" => {
            let value = reads
                .get_character(selection(arguments, "id")?)
                .map_err(Failure::Character)?;
            output::json_bytes(&output::character(&value))
        }
        _ => Err(Failure::Usage("Select a supported character operation.")),
    }
}

/// Routes campaign operations while retaining opaque character associations.
fn execute_campaign(
    action: &str,
    arguments: &ArgMatches,
    reads: &impl ReadOperations,
) -> Result<Vec<u8>, Failure> {
    match action {
        "list" => {
            let options = ListOptions::parse(arguments)?;
            let snapshot = reads.list_campaigns().map_err(Failure::Character)?;
            let rows: Vec<_> = snapshot.campaigns().iter().map(output::campaign).collect();
            render_list(
                &options,
                &["id", "name", "source_url", "character_ids"],
                rows,
            )
        }
        "show" => {
            let value = reads
                .get_campaign(selection(arguments, "id")?)
                .map_err(Failure::Character)?;
            output::json_bytes(&output::campaign(&value))
        }
        _ => Err(Failure::Usage("Select a supported campaign operation.")),
    }
}

/// Routes book metadata, navigation, and one explicitly selected section chunk.
fn execute_book(
    action: &str,
    arguments: &ArgMatches,
    reads: &impl ReadOperations,
) -> Result<Vec<u8>, Failure> {
    match action {
        "list" => {
            let options = ListOptions::parse(arguments)?;
            let snapshot = reads.discover_library().map_err(Failure::Library)?;
            let rows: Vec<_> = snapshot.books().iter().map(output::book).collect();
            render_list(&options, &["id", "title", "access", "source_url"], rows)
        }
        "outline" => {
            let options = ListOptions::parse(arguments)?;
            let value = reads
                .get_book_outline(selection(arguments, "book-id")?)
                .map_err(Failure::Book)?;
            let json = output::outline(&value);
            let rows = json["entries"]
                .as_array()
                .ok_or(Failure::Usage("Outline entries must be an array."))?;
            output::list(
                &options,
                &["id", "title", "parent_id", "source_url"],
                rows,
                Some(&json),
            )
        }
        "read" => {
            let cursor = arguments
                .get_one::<String>("continuation")
                .map(|token| SectionCursor::from_token(token))
                .transpose()
                .map_err(Failure::Cursor)?;
            let value = reads
                .read_book_section(SectionRequest {
                    book_id: selection(arguments, "book-id")?,
                    section_id: selection(arguments, "section-id")?,
                    continuation: cursor.as_ref(),
                })
                .map_err(Failure::Book)?;
            output::json_bytes(&output::section(&value)?)
        }
        _ => Err(Failure::Usage("Select a supported book operation.")),
    }
}

/// Retrieves an identifier only after the actual command definition validates it.
fn selection<'a>(matches: &'a ArgMatches, name: &str) -> Result<&'a str, Failure> {
    matches
        .get_one::<String>(name)
        .map(String::as_str)
        .ok_or(Failure::Usage("A resource identifier is required."))
}

/// Presents a list without losing additional structured JSON-only fields.
fn render_list(
    options: &ListOptions,
    fields: &[&str],
    rows: Vec<Value>,
) -> Result<Vec<u8>, Failure> {
    output::list(options, fields, &rows, None)
}

#[cfg(test)]
mod tests;
