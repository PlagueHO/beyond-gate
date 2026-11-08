//! One resource-first clap definition for parsing, help, and completions.

use beyond_gate_core::FOUNDATION_NOTICE;
use clap::{Arg, ArgAction, Command, builder::ValueParser, value_parser};
use clap_complete::Shell;

/// Builds help and parsing without accessing credentials, sources, or storage.
pub fn command() -> Command {
    Command::new("beyond-gate")
        .bin_name("beyond-gate")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Local-first access to authorized D&D Beyond data")
        .after_help(FOUNDATION_NOTICE)
        .arg_required_else_help(true)
        .subcommand(
            resource("auth", "Manage the protected local session")
                .subcommand(Command::new("login").about("Complete human-controlled Windows browser login after explicit extension setup"))
                .subcommand(browser_setup("install", "Preview and register the Windows browser native host"))
                .subcommand(browser_setup("uninstall", "Preview and remove matching Windows browser native-host setup"))
                .subcommand(Command::new("status").about("Report local presence only, without checking upstream usability"))
                .subcommand(Command::new("validate").about("Check the stored Windows session with one fresh service exchange; never accepts credentials"))
                .subcommand(Command::new("logout").about("Remove the local credential without upstream revocation")),
        )
        .subcommand(
            resource("character", "Read authorized character data")
                .subcommand(list_options(Command::new("list").about("List complete character summaries")))
                .subcommand(show_options(Command::new("show").about("Show one character and its unchanged raw JSON").arg(identifier("id")))),
        )
        .subcommand(
            resource("campaign", "Read authorized campaign data")
                .subcommand(list_options(Command::new("list").about("List campaign summaries without loading rosters"))
                    .arg(Arg::new("active-only").long("active-only").action(ArgAction::SetTrue).help("List only campaigns from the active-campaign source")))
                .subcommand(show_options(Command::new("show").about("Show one campaign summary and its active roster").arg(identifier("id")))),
        )
        .subcommand(
            resource("book", "Bounded Windows library, outline, and selected-section reads")
                .subcommand(list_options(Command::new("list").about("Discover bounded Web-catalog metadata through public classification and the stored Windows session")))
                .subcommand(list_options(Command::new("outline").about("List at most 1,000 entries after fresh library and canonical source-page checks").arg(identifier("book-id"))))
                .subcommand(
                    Command::new("read")
                        .about("Read one selected JSON chunk, at most 4,096 UTF-8 text bytes, after fresh access checks")
                        .arg(identifier("book-id"))
                        .arg(identifier("section-id"))
                        .arg(Arg::new("continuation").long("continuation").value_name("TOKEN")
                            .help("Explicitly continue this section; token limit: 16 KiB; never follows automatically")
                            .hide_possible_values(true)),
                ),
        )
        .subcommand(
            resource("mcp", "Start or register the local MCP server")
                .subcommand(
                    Command::new("start")
                        .about("Start the sibling beyond-gate-mcp stdio server"),
                )
                .subcommand(
                    Command::new("install")
                        .about("Preview and register the local server with an MCP client")
                        .arg(
                            Arg::new("client")
                                .required(true)
                                .value_parser(["vscode", "claude-code", "copilot-cli"])
                                .help("User-level client configuration to update"),
                        )
                        .arg(
                            Arg::new("dry-run")
                                .long("dry-run")
                                .action(ArgAction::SetTrue)
                                .help("Show the proposed entry without prompting or writing"),
                        ),
                ),
        )
        .subcommand(
            Command::new("completions")
                .about("Generate a shell completion script on stdout without account access")
                .arg(Arg::new("shell").required(true).value_parser(value_parser!(Shell))),
        )
}

/// Requires an explicit stable installation directory and browser for host setup.
fn browser_setup(name: &'static str, about: &'static str) -> Command {
    Command::new(name)
        .about(about)
        .arg(Arg::new("browser").required(true).value_parser(["chrome", "edge"]))
        .arg(
            Arg::new("directory")
                .long("directory")
                .required(true)
                .value_name("PATH")
                .value_parser(value_parser!(std::path::PathBuf))
                .help("Existing stable installation directory; source and build directories are refused"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Check conflicts and show setup without prompting or writing"),
        )
}

/// Requires an explicit action and makes nested help deterministic.
fn resource(name: &'static str, about: &'static str) -> Command {
    Command::new(name)
        .about(about)
        .arg_required_else_help(true)
        .subcommand_required(true)
}

/// Validates opaque selections without inventing upstream identifier rules.
fn identifier(name: &'static str) -> Arg {
    Arg::new(name)
        .required(true)
        .value_parser(ValueParser::new(valid_identifier))
}

/// Rejects empty, padded, or control-bearing identifiers before any operation.
fn valid_identifier(value: &str) -> Result<String, &'static str> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        Err("identifier must be nonempty, unpadded text without control characters")
    } else {
        Ok(value.to_owned())
    }
}

/// Adds output controls with the established table default for lists.
fn list_options(command: Command) -> Command {
    output_options(command, "table").arg(
        Arg::new("no-headers")
            .long("no-headers")
            .action(ArgAction::SetTrue)
            .help("Omit table/TSV headers; invalid with JSON"),
    )
}

/// Adds output controls with a JSON default for one selected record.
fn show_options(command: Command) -> Command {
    output_options(command, "json").arg(
        Arg::new("no-headers")
            .long("no-headers")
            .action(ArgAction::SetTrue)
            .help("Omit table/TSV headers; invalid with JSON"),
    )
}

/// Defines one output selector shared by its modern name and compatibility alias.
fn output_options(command: Command, default: &'static str) -> Command {
    command.arg(
        Arg::new("output")
            .short('o')
            .long("output")
            .visible_alias("format")
            .default_value(default)
            .value_parser(["table", "json", "tsv"])
            .help("Output format; JSON preserves structured fields"),
    )
}
