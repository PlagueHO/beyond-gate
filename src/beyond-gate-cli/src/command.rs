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
            resource("auth", "Inspect or remove the protected local session; browser login is blocked")
                .subcommand(Command::new("login").about("Report the browser-login blocker without collecting credentials"))
                .subcommand(Command::new("status").about("Report local presence only, without checking upstream usability"))
                .subcommand(Command::new("logout").about("Remove the local credential without upstream revocation")),
        )
        .subcommand(
            resource("character", "Character reads (live source and login are not implemented)")
                .subcommand(list_options(Command::new("list").about("List character summaries; live reads are unsupported")))
                .subcommand(Command::new("show").about("Show character metadata and unchanged raw JSON; live reads are unsupported").arg(identifier("id"))),
        )
        .subcommand(
            resource("campaign", "Campaign reads (live source and login are not implemented)")
                .subcommand(list_options(Command::new("list").about("List campaigns and character associations; live reads are unsupported")))
                .subcommand(Command::new("show").about("Show campaign metadata, associations, and raw JSON; live reads are unsupported").arg(identifier("id"))),
        )
        .subcommand(
            resource("book", "Book reads (live source and login are not implemented)")
                .subcommand(list_options(Command::new("list").about("List book metadata and access evidence; live discovery is unsupported")))
                .subcommand(list_options(Command::new("outline").about("List section navigation; live reads are unsupported").arg(identifier("book-id"))))
                .subcommand(
                    Command::new("read")
                        .about("Read one JSON chunk, at most 4,096 UTF-8 text bytes; live reads are unsupported")
                        .arg(identifier("book-id"))
                        .arg(identifier("section-id"))
                        .arg(Arg::new("continuation").long("continuation").value_name("TOKEN")
                            .help("Explicitly continue this section; token limit: 16 KiB; never follows automatically")
                            .hide_possible_values(true)),
                ),
        )
        .subcommand(
            Command::new("completions")
                .about("Generate a shell completion script on stdout without account access")
                .arg(Arg::new("shell").required(true).value_parser(value_parser!(Shell))),
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

/// Adds only the formatting controls that apply to list-like results.
fn list_options(command: Command) -> Command {
    command
        .arg(
            Arg::new("format")
                .long("format")
                .default_value("table")
                .value_parser(["table", "json", "tsv"])
                .help("Result format; JSON preserves structured fields"),
        )
        .arg(
            Arg::new("no-headers")
                .long("no-headers")
                .action(ArgAction::SetTrue)
                .help("Omit table/TSV headers; invalid with JSON"),
        )
}
