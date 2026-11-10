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
            class_resource("class", false),
        )
        .subcommand(
            class_resource("subclass", true),
        )
        .subcommand(
            resource("monster", "Read bounded monster pages and selected detail; only verified access code 1")
                .subcommand(monster_list())
                .subcommand(show_options(Command::new("show").about("Read one selected monster with unchanged raw detail"))
                    .arg(Arg::new("id").required(true).value_parser(ValueParser::new(valid_monster_id)))),
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

/// Checks literal monster searches without interpreting upstream query syntax.
fn valid_monster_name(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::monster::MonsterQuery {
        name: Some(value.to_owned()),
        ..Default::default()
    }
    .validate()
    .map(|()| value.to_owned())
    .map_err(|_| "name must be nonempty, unpadded, control-free text within 256 UTF-8 bytes")
}

/// Builds website filter flags from shared field mappings for parsing and completions.
fn monster_list() -> Command {
    use beyond_gate_core::monster::query::{
        BOOL_FILTERS, CHALLENGE_RATINGS, ID_FILTERS, NUMBER_FILTERS,
    };
    let mut command = list_options(Command::new("list").about("Read one website-filtered monster page with fresh access checks; never follows continuation"))
        .arg(Arg::new("name").long("name").short('n').value_name("NAME")
            .value_parser(ValueParser::new(valid_monster_name)).help("Literal name search; at most 256 UTF-8 bytes"))
        .arg(Arg::new("page").long("page").default_value("1").value_parser(value_parser!(u32).range(1..))
            .help("Explicit website page; use pagination.next_page to continue"))
        .arg(Arg::new("partnered-content").long("partnered-content").default_value("no")
            .value_parser(["yes", "no", "any"]).help("Website partnered-content choice; no matches the site default"));
    for flag in ["cr-min", "cr-max"] {
        command = command.arg(
            Arg::new(flag)
                .long(flag)
                .value_parser(clap::builder::PossibleValuesParser::new(
                    CHALLENGE_RATINGS.iter().map(|(label, _)| *label),
                ))
                .help("Literal challenge rating, not a website option ID"),
        );
    }
    for (_, flag, _) in ID_FILTERS {
        command = command.arg(Arg::new(flag).long(flag).value_name("ID")
            .value_parser(value_parser!(u32).range(1..)).action(ArgAction::Append)
            .help("Public website option ID; repeat for multiple values (maximum 50); not an access grant"));
    }
    for (_, flag, _) in NUMBER_FILTERS {
        command = command.arg(
            Arg::new(flag)
                .long(flag)
                .value_parser(value_parser!(u32))
                .help("Nonnegative bound; minimum must not exceed maximum"),
        );
    }
    for (_, flag, _) in BOOL_FILTERS {
        command = command.arg(
            Arg::new(flag)
                .long(flag)
                .value_parser(value_parser!(bool))
                .help("Website true/false filter; omission leaves it unrestricted"),
        );
    }
    command
}

/// Checks a canonical selected ID before the command accesses a session.
fn valid_monster_id(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::monster::validate_id(value)
        .map(|()| value.to_owned())
        .map_err(|_| "ID must be a canonical positive decimal integer that fits u64")
}

/// Builds metadata-only class/subclass commands from the same completion definition.
fn class_resource(name: &'static str, subclass: bool) -> Command {
    let list = list_options(Command::new("list").about("List complete bounded metadata; never rules text"))
        .arg(Arg::new("name").long("name").short('n').value_name("NAME")
            .value_parser(ValueParser::new(valid_class_name))
            .help("Local case-insensitive substring filter; keeps duplicate names; at most 256 UTF-8 bytes"));
    let show = show_options(
        Command::new("show").about("Show selected metadata, not raw definitions or rules text"),
    )
    .arg(
        Arg::new("id")
            .required(true)
            .value_parser(ValueParser::new(valid_class_identifier)),
    );
    resource(name, "Browse bounded Windows class catalog metadata")
        .subcommand(if subclass { parent_option(list) } else { list })
        .subcommand(if subclass { parent_option(show) } else { show })
}

/// Adds an optional parent selector only to subclass commands.
fn parent_option(command: Command) -> Command {
    command.arg(
        Arg::new("class-id")
            .long("class-id")
            .value_name("ID")
            .value_parser(ValueParser::new(valid_class_identifier))
            .help("Scope to one parent; omitted means bounded global discovery"),
    )
}

/// Requires canonical positive decimal IDs before any operation starts.
fn valid_class_identifier(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::class_catalog::validate_input(
        beyond_gate_core::class_catalog::ClassKind::Class,
        None,
        None,
        Some(value),
    )
    .map(|()| value.to_owned())
    .map_err(|_| "ID must be a canonical positive decimal integer that fits u64")
}

/// Validates local name filters without interpreting a query language.
fn valid_class_name(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::class_catalog::validate_input(
        beyond_gate_core::class_catalog::ClassKind::Class,
        Some(value),
        None,
        None,
    )
    .map(|()| value.to_owned())
    .map_err(|_| "name must be nonempty, unpadded, control-free text within 256 UTF-8 bytes")
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
