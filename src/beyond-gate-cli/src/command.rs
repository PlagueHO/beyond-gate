//! One resource-first clap definition for parsing, help, and completions.

use clap::{Arg, ArgAction, Command, builder::ValueParser, value_parser};
use clap_complete::Shell;

const PROTECTED_READ_HELP: &str = "Requires selected Beyond Gate session input on Windows or Linux.\n\
     Use BEYOND_GATE_COBALT_SESSION, --auth-prompt, or a saved session.\n\
     Your D&D Beyond account must be able to open the requested content.";

/// Builds help and parsing without accessing credentials, sources, or storage.
pub fn command() -> Command {
    Command::new("beyond-gate")
        .arg(
            Arg::new("auth-prompt")
                .long("auth-prompt")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Process-only hidden input: resource reads or auth validate only; not login/import"),
        )
        .bin_name("beyond-gate")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Read your D&D Beyond characters, campaigns, books, and game content")
        .max_term_width(100)
        .after_help(
            r#"Authentication and getting started

On Windows and Linux, use BEYOND_GATE_COBALT_SESSION for process-only input.
Use --auth-prompt for a hidden cookie prompt before one CLI read or auth validate.
Prompt input takes precedence over environment input, then a saved session.
Invalid selected input fails without fallback. No process-only input is saved.
Your session does not unlock content that your account cannot open.
Character and campaign reads use fixed bounded sources.

First-time setup on Windows:
  1. Install the CLI, browser extension (add-on), and browser helper.
     Follow the installation guide.
  2. Preview browser setup for Chrome or Edge. Replace <INSTALL_DIR> with the
     folder that contains both Beyond Gate programs:
       beyond-gate auth install edge --directory <INSTALL_DIR> --dry-run
     Use `chrome` instead of `edge` for Chrome.
  3. Review the preview. Repeat the command without `--dry-run` to apply it.
  4. Run `beyond-gate auth login`. Follow the steps in the terminal and browser.
  5. Run `beyond-gate auth validate`, then try:
       beyond-gate character list

Each login method validates and saves to your operating system's protected storage:
  beyond-gate auth login             Windows browser pairing
  beyond-gate auth login --prompt    Hidden CobaltSession input
  beyond-gate auth login --from-env  Read BEYOND_GATE_COBALT_SESSION (no flag value)
Existing auth import --prompt/--env commands remain available for compatibility.
Process-only use needs no browser or store.
`auth status` reports local input, not upstream usability.
`auth validate` sends one fresh request to D&D Beyond.
`auth logout` clears the selected source. Environment logout does not delete saved
credentials or erase the parent environment. Use auth logout --stored to delete
the saved entry explicitly. Neither action revokes an upstream session.

Do not enter your password, cookies, or tokens in command arguments or chat."#,
        )
        .arg_required_else_help(true)
        .subcommand(
            resource(
                "auth",
                "Choose a sign-in method and manage session input",
            )
                .after_help(
                    "Sign-in works on Windows through optional browser assistance.\n\
                     Windows and Linux also accept BEYOND_GATE_COBALT_SESSION without a store or browser.\n\
                     Use --auth-prompt for one read or auth validate; stdin and stderr must be terminals.\n\
                     Each login method validates and saves to protected OS storage:\n\
                       auth login             Windows browser pairing\n\
                       auth login --prompt    Hidden CobaltSession input\n\
                       auth login --from-env  Read BEYOND_GATE_COBALT_SESSION; no flag value\n\
                     Existing auth import --prompt/--env commands remain compatible.\n\
                     --auth-prompt is not valid for login, import, status, logout, or setup.\n\
                     No command requests your D&D Beyond password.",
                )
                .subcommand(
                    Command::new("login")
                        .about("Validate and save a session through browser, hidden input, or environment")
                        .arg(
                            Arg::new("prompt").long("prompt")
                                .action(ArgAction::SetTrue)
                                .conflicts_with("from-env")
                                .help("Read hidden CobaltSession input, validate it, and save it"),
                        )
                        .arg(
                            Arg::new("from-env").long("from-env")
                                .action(ArgAction::SetTrue)
                                .help("Validate and save BEYOND_GATE_COBALT_SESSION; accepts no value"),
                        )
                        .after_help(
                            "Choose one method. Each method validates and saves to protected OS storage:\n\
                               beyond-gate auth login             Windows browser pairing\n\
                               beyond-gate auth login --prompt    Hidden CobaltSession input\n\
                               beyond-gate auth login --from-env  Read BEYOND_GATE_COBALT_SESSION\n\n\
                             Without a selector, Windows browser pairing requires the extension, registered browser helper, and three terminal streams.\n\
                             On Windows/Linux, --prompt needs terminal stdin and stderr; --from-env needs no terminal or browser.\n\
                             --prompt and --from-env take no value and cannot be combined. Invalid selected input never falls back.\n\
                             Only --from-env selects environment input for login. No selector always means browser pairing.\n\
                             Use --prompt here, not the process-only global --auth-prompt option.\n\
                             Existing auth import --prompt/--env commands remain compatible.\n\
                             Do not pass cookie values as arguments. This command never requests your D&D Beyond password.",
                        ),
                )
                .subcommand(browser_setup("install", "Set up the browser connection for Chrome or Edge"))
                .subcommand(browser_setup("uninstall", "Remove this browser's Beyond Gate connection"))
                .subcommand(
                    Command::new("import")
                        .about("Compatibility form of auth login --prompt or --from-env")
                        .after_help(
                            "Prefer auth login --prompt or auth login --from-env. Existing import commands keep their validation and protected-save behavior.\n\
                             The process-only global --auth-prompt option is not valid here.",
                        )
                        .arg(
                            Arg::new("prompt").long("prompt")
                                .action(ArgAction::SetTrue)
                                .required_unless_present("env")
                                .conflicts_with("env")
                                .help("Request a hidden cookie value in this terminal"),
                        )
                        .arg(
                            Arg::new("env").long("env")
                                .action(ArgAction::SetTrue)
                                .required_unless_present("prompt")
                                .help("Import BEYOND_GATE_COBALT_SESSION without a prompt"),
                        ),
                )
                .subcommand(
                    Command::new("status")
                        .arg(stored_selection())
                        .about("Report the selected input source without upstream validation")
                        .after_help(
                            "Checks local environment input syntax or saved presence. Use --stored to ignore environment input.\n\n\
                             It does not contact D&D Beyond, check whether the sign-in still works, \
                             or check access to specific content.",
                        ),
                )
                .subcommand(
                    Command::new("validate")
                        .about("Check the selected session with one fresh D&D Beyond request")
                        .after_help(
                            "Uses selected process-only or saved input to send one request to D&D Beyond. Prints \
                             one JSON result.\n\n\
                             This checks the sign-in only; it does not test access to every character \
                             or book. Use `auth status` to check whether a sign-in is stored without \
                             contacting D&D Beyond.",
                        ),
                )
                .subcommand(
                    Command::new("logout")
                        .arg(stored_selection())
                        .about("Clear the selected local session without upstream revocation")
                        .after_help(
                            "Environment selection clears only this process's session state, not saved credentials or the parent environment.\n\
                             Use --stored to delete the protected saved entry explicitly.\n\n\
                             Neither action signs your browser out of D&D Beyond.",
                        ),
                ),
        )
        .subcommand(
            protected_resource("character", "List your characters or show one character's details")
                .subcommand(protected_action_with_details(
                    list_options(Command::new("list").about("List your character names and IDs")),
                    "This is a short list, not complete character records. Copy an ID and use `character show` to see one full record.",
                ))
                .subcommand(protected_action_with_details(
                    show_options(
                        Command::new("show")
                            .about("Show the full record for one character ID")
                            .arg(identifier("id")),
                    ),
                    "Copy the character ID from `character list`. This returns one character record without calculating game rules.",
                )),
        )
        .subcommand(
            protected_resource("campaign", "List your campaigns or show one campaign and its characters")
                .subcommand(
                    protected_action_with_details(
                        output_options(
                            Command::new("list").about("List your campaigns"),
                            "table",
                        ),
                        "By default, this uses your standard campaign list. Add `--active-only` to use the active campaign list instead.\n\
                         This command does not load campaign characters. Use `campaign show` to see the active character list for one campaign.",
                    )
                    .arg(
                        Arg::new("active-only")
                            .long("active-only")
                            .action(ArgAction::SetTrue)
                            .help("Use the active campaign list instead of the standard list"),
                    ),
                )
                .subcommand(protected_action_with_details(
                    show_options(
                        Command::new("show")
                            .about("Show one campaign and its active characters")
                            .arg(identifier("id")),
                    ),
                    "Copy the campaign ID from `campaign list`. The character list is the active list returned for that campaign; it may not include every campaign member.",
                )),
        )
        .subcommand(
            class_resource("class", false),
        )
        .subcommand(
            class_resource("subclass", true),
        )
        .subcommand(
            protected_resource("monster", "Search monsters or show one monster from the results")
                .subcommand(monster_list())
                .subcommand(protected_action_with_details(
                    show_options(
                        Command::new("show")
                            .about("Show one monster using its ID from `monster list`")
                            .arg(
                                Arg::new("id")
                                    .required(true)
                                    .value_name("ID")
                                    .value_parser(ValueParser::new(valid_monster_id))
                                    .help("ID shown by `monster list`"),
                            ),
                    ),
                    "Copy an ID from `monster list`. The command checks again that your account can open the selected monster before showing it.",
                )),
        )
        .subcommand(
            protected_resource("spell", "Search spells or show one spell from the results")
                .subcommand(spell_list())
                .subcommand(protected_action_with_details(
                    show_options(
                        Command::new("show")
                            .about("Show one spell using its key from `spell list`")
                            .arg(
                                Arg::new("slug")
                                    .required(true)
                                    .value_name("SPELL_KEY")
                                    .value_parser(ValueParser::new(valid_spell_slug))
                                    .help("Spell key shown by `spell list`; copy it exactly"),
                            ),
                    ),
                    "Copy the complete spell key from `spell list`. You do not need to enter a web address.",
                )),
        )
        .subcommand(
            protected_resource("equipment", "List equipment or show an item from a chosen D&D Beyond page")
                .subcommand(protected_action_with_details(
                    list_options(Command::new("list").about("List equipment on one D&D Beyond page"))
                        .arg(item_page(false)),
                    "Each page contains up to 100 items. Choose another page with `--page`; this command does not search other pages.",
                ))
                .subcommand(protected_action_with_details(
                    show_options(
                        Command::new("show")
                            .about("Show one item using its ID and page from a list result")
                            .arg(item_identifier())
                            .arg(item_page(true)),
                    ),
                    "Copy the full item ID and page number from `equipment list`. This command checks only that page; it does not search other pages.",
                )),
        )
        .subcommand(
            protected_resource("magic-item", "Filter magic items on one page or show one listed item")
                .subcommand(magic_item_list())
                .subcommand(protected_action_with_details(
                    show_options(
                        Command::new("show")
                            .about("Show one magic item using its ID and page from a list result")
                            .arg(item_identifier())
                            .arg(item_page(true)),
                    ),
                    "Copy the full item ID and page number from `magic-item list`. This command checks only that page; it does not search other pages.",
                )),
        )
        .subcommand(
            protected_resource("feat", "List feats or show one feat's full record")
                .subcommand(feat_list())
                .subcommand(protected_action_with_details(
                    show_options(
                        Command::new("show")
                            .about("Show one feat using its ID from `feat list`")
                            .arg(
                                Arg::new("id")
                                    .required(true)
                                    .value_name("ID")
                                    .value_parser(ValueParser::new(valid_feat_id))
                                    .help("ID shown by `feat list`"),
                            ),
                    ),
                    "Copy the feat ID from `feat list`. This returns the full record for that ID, not a list of matching feats.",
                )),
        )
        .subcommand(
            protected_resource("background", "List backgrounds or show one background's details")
                .subcommand(background_list())
                .subcommand(protected_action_with_details(
                    show_options(
                        Command::new("show")
                            .about("Show one background using its ID from `background list`")
                            .arg(
                                Arg::new("id")
                                    .required(true)
                                    .value_name("ID")
                                    .value_parser(ValueParser::new(valid_background_id))
                                    .help("ID shown by `background list`"),
                            ),
                    ),
                    "Copy the background ID from `background list`. This returns the selected background record.",
                )),
        )
        .subcommand(
            protected_resource("species", "List species or show one species' details")
                .subcommand(protected_action_with_details(
                    list_options(Command::new("list").about("List species from D&D Beyond")),
                    "This shows the species returned by D&D Beyond. It does not search or load additional pages.",
                ))
                .subcommand(protected_action_with_details(
                    show_options(
                        Command::new("show")
                            .about("Show one species using its ID from `species list`")
                            .arg(
                                Arg::new("id")
                                    .required(true)
                                    .value_name("ID")
                                    .value_parser(ValueParser::new(valid_species_id))
                                    .help("ID shown by `species list`"),
                            ),
                    ),
                    "Copy the species ID from `species list`. This returns the selected species record.",
                )),
        )
        .subcommand(
            protected_resource("book", "Find books, browse their sections, and read selected text")
                .subcommand(protected_action_with_details(
                    list_options(
                        Command::new("list").about("List D&D Beyond books and show what is known about access"),
                    ),
                    "Check the `access` field to see what Beyond Gate knows about each book. `unknown` means access was not confirmed; it does not mean access was denied.",
                ))
                .subcommand(protected_action_with_details(
                    list_options(
                        Command::new("outline")
                            .about("List the chapter and section titles in one book")
                            .arg(identifier("book-id")),
                    ),
                    "Copy the book ID from `book list`. Use the section IDs from this output with `book read`.",
                ))
                .subcommand(
                    Command::new("read")
                        .about("Read one section of a book, one text part at a time")
                        .after_help(format!(
                             "First use `book list` to find a book, then `book outline` to find a section.\n\
                             The result is JSON and contains up to 4,096 bytes of text.\n\
                             If the result includes `next_cursor`, copy its value to `--continuation` with the same book and section IDs to get the next part.\n\
                             The CLI never gets the next part automatically.\n\n\
                             {PROTECTED_READ_HELP}"
                        ))
                        .arg(identifier("book-id"))
                        .arg(identifier("section-id"))
                        .arg(Arg::new("continuation").long("continuation").value_name("TOKEN")
                            .help("Copy `next_cursor` from an earlier result to get the next text part")
                            .hide_possible_values(true)),
                ),
        )
        .subcommand(
            resource("mcp", "Connect an MCP-compatible app to Beyond Gate")
                .after_help(
                    "MCP (Model Context Protocol) lets compatible apps use Beyond Gate tools.\n\
                     `mcp start` runs the local server. `mcp install` adds it to a supported app's settings.\n\n\
                     These commands do not require a saved sign-in. Data reads still use the same saved sign-in as the CLI.",
                )
                .subcommand(
                    Command::new("start")
                        .about("Run the local server used by supported apps")
                        .after_help(
                            "Starts the `beyond-gate-mcp` program installed beside this CLI.\n\
                             This server uses Model Context Protocol (MCP) to connect supported apps to Beyond Gate.\n\
                             Keep it running while your app uses Beyond Gate. Stop it when you are done.",
                        ),
                )
                .subcommand(
                    Command::new("install")
                        .about("Add Beyond Gate to VS Code, Claude Code, or Copilot CLI")
                        .after_help(
                            "Adds a local Model Context Protocol (MCP) server entry to settings for the selected app. It does not sign you in; use `auth login` separately before reading account data.\n\
                             The command shows the proposed change and asks before saving it. Use `--dry-run` to preview without changing settings.",
                        )
                        .arg(
                            Arg::new("client")
                                .required(true)
                                .value_name("CLIENT")
                                .value_parser(["vscode", "claude-code", "copilot-cli"])
                                .help("Choose an app: vscode (VS Code), claude-code (Claude Code), or copilot-cli (Copilot CLI)"),
                        )
                        .arg(
                            Arg::new("dry-run")
                                .long("dry-run")
                                .action(ArgAction::SetTrue)
                                .help("Show the proposed change without saving it"),
                        ),
                ),
        )
        .subcommand(
            Command::new("completions")
                .about("Add Beyond Gate command suggestions to your shell")
                .after_help(
                    "Prints a script that lets your shell suggest commands and options as you type.\n\
                     Save or load the script using your shell's usual completion setup. This does not sign in or contact D&D Beyond.",
                )
                .arg(
                    Arg::new("shell")
                        .required(true)
                        .value_name("SHELL")
                        .value_parser(value_parser!(Shell))
                        .help("Choose your shell: bash, elvish, fish, powershell, or zsh"),
                ),
        )
}

/// Makes access to the saved entry explicit when process input is selected.
fn stored_selection() -> Arg {
    Arg::new("stored")
        .long("stored")
        .action(ArgAction::SetTrue)
        .help("Select only the saved OS entry, ignoring environment input")
}

/// Builds the shared bounded one-based page selector, required for detail.
fn item_page(required: bool) -> Arg {
    let arg = Arg::new("page")
        .long("page")
        .value_name("PAGE")
        .value_parser(
            value_parser!(u32).range(1..=i64::from(beyond_gate_core::item_catalog::MAX_PAGE)),
        )
        .help(
            "Page number from 1 to 1,000,000; list defaults to 1; each page holds up to 100 items",
        );
    if required {
        arg.required(true)
    } else {
        arg.default_value("1")
    }
}

/// Builds bounded local filters using the same repeated-argument pattern as other resources.
fn magic_item_list() -> Command {
    let mut command = protected_action_with_details(
        list_options(Command::new("list").about("Filter magic items on one D&D Beyond page")),
        "Filters only check the page selected by `--page`; they do not search other pages.\n\
         Name matching ignores letter case and can match part of a name. Type, rarity, and tag must match complete labels from the results.\n\
         Each text value can contain up to 256 bytes. Repeat each type, rarity, or tag filter up to 50 times.",
    )
    .arg(item_page(false))
    .arg(
        Arg::new("name")
            .long("name")
            .short('n')
            .value_name("NAME")
            .value_parser(ValueParser::new(valid_item_filter))
            .help("Match part of an item name on this page; letter case does not matter; max 256 bytes"),
    )
    .arg(
        Arg::new("attunement")
            .long("attunement")
            .value_name("VALUE")
            .default_value("any")
            .value_parser(["yes", "no", "any"])
            .help("Choose items that require attunement: yes, no, or any (default)"),
    );
    for (field, flag) in beyond_gate_core::item_catalog::query::LABEL_FILTERS {
        let label = match field {
            "types" => "item type",
            "rarities" => "rarity",
            "tags" => "tag",
            _ => "label",
        };
        command = command.arg(Arg::new(flag).long(flag).value_name("LABEL")
            .value_parser(ValueParser::new(valid_item_filter)).action(ArgAction::Append)
            .help(format!("Match an exact {label} label; letter case does not matter. Repeat to choose more than one.")));
    }
    command
}

/// Rejects invalid local text before protected storage or network access.
fn valid_item_filter(value: &str) -> Result<String, &'static str> {
    if beyond_gate_core::item_catalog::query::valid_filter_text(value) {
        Ok(value.to_owned())
    } else {
        Err("Use nonempty unpadded text without controls, at most 256 UTF-8 bytes")
    }
}

/// Rejects incomplete or noncanonical composite item identities before session access.
fn valid_item_id(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::item_catalog::validate_id(value)
        .map(|()| value.to_owned())
        .map_err(|_| "ID must be a canonical positive entityTypeId:id definitionKey")
}

/// Builds feat flags from the shared website controls without adding API filters.
fn feat_list() -> Command {
    let mut command = protected_action_with_details(
        list_options(Command::new("list").about("List feats using D&D Beyond's filters")),
        "The list shows feat names and source details, not descriptions or game rules.\n\
         ID filters use numbers assigned to choices in D&D Beyond's filter menus. They are not feat IDs and do not unlock content.\n\
         Tag, source, and source-category filters accept up to 50 values each. The prerequisite filter accepts one.",
    )
    .arg(
        Arg::new("name")
            .long("name")
            .short('n')
            .value_name("NAME")
            .value_parser(ValueParser::new(valid_feat_name))
            .help("Filter by name text; at most 256 bytes"),
    )
    .arg(
        Arg::new("prerequisite-subtype-id")
            .long("prerequisite-subtype-id")
            .value_name("ID")
            .value_parser(value_parser!(u32).range(1..))
            .help("Choose a prerequisite option from D&D Beyond; this is not the feat's ID"),
    )
    .arg(
        Arg::new("partnered-content")
            .long("partnered-content")
            .value_name("VALUE")
            .default_value("no")
            .value_parser(["yes", "no", "any"])
            .help("Use yes to include partner content, no to exclude it (default), or any"),
    );
    for (_, flag, _) in beyond_gate_core::feat::query::ID_FILTERS {
        command = command.arg(
            Arg::new(flag)
                .long(flag)
                .value_name("ID")
                .value_parser(value_parser!(u32).range(1..))
                .action(ArgAction::Append)
                .help(
                    "Number assigned to a D&D Beyond filter choice; repeat to choose more than one",
                ),
        );
    }
    command
}

/// Checks a canonical feat ID before protected storage or network access.
fn valid_feat_id(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::feat::validate_id(value)
        .map(|()| value.to_owned())
        .map_err(|_| "ID must be a canonical positive decimal integer that fits u64")
}

/// Validates a literal name through the shared feat query policy.
fn valid_feat_name(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::feat::FeatQuery {
        name: Some(value.to_owned()),
        ..Default::default()
    }
    .validate()
    .map(|()| value.to_owned())
    .map_err(
        |_| "Name must be valid text within 256 UTF-8 bytes without padding or control characters",
    )
}

/// Builds background flags from the shared observed website control mapping.
fn background_list() -> Command {
    let mut command = protected_action_with_details(
        list_options(Command::new("list").about("List backgrounds using D&D Beyond's filters")),
        "ID filters use numbers assigned to choices in D&D Beyond's filter menus. They are not background IDs and do not unlock content.\n\
         Each ID filter accepts up to 50 values.",
    )
    .arg(
        Arg::new("name")
            .long("name")
            .short('n')
            .value_name("NAME")
            .value_parser(ValueParser::new(valid_background_name))
            .help("Filter by name text; at most 256 bytes"),
    )
    .arg(
        Arg::new("partnered-content")
            .long("partnered-content")
            .value_name("VALUE")
            .default_value("no")
            .value_parser(["yes", "no", "any"])
            .help("Use yes to include partner content, no to exclude it (default), or any"),
    );
    for (_, flag, _) in beyond_gate_core::background::query::ID_FILTERS {
        command = command.arg(
            Arg::new(flag)
                .long(flag)
                .value_name("ID")
                .value_parser(value_parser!(u32).range(1..))
                .action(ArgAction::Append)
                .help(
                    "Number assigned to a D&D Beyond filter choice; repeat to choose more than one",
                ),
        );
    }
    command
}

/// Checks a canonical background ID before credential or source access.
fn valid_background_id(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::background::validate_id(value)
        .map(|()| value.to_owned())
        .map_err(|_| "ID must be a canonical positive decimal integer that fits u64")
}

/// Checks a canonical species ID before credential or source access.
fn valid_species_id(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::species::validate_id(value)
        .map(|()| value.to_owned())
        .map_err(|_| "ID must be a canonical positive decimal integer that fits u64")
}

/// Checks literal background filter text without interpreting upstream syntax.
fn valid_background_name(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::background::BackgroundQuery {
        name: Some(value.to_owned()),
        ..Default::default()
    }
    .validate()
    .map(|()| value.to_owned())
    .map_err(|_| "name must be nonempty, unpadded, control-free text within 256 UTF-8 bytes")
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
    let mut command = protected_action_with_details(
        list_options(Command::new("list").about("Search one page of monsters")),
        "Use the `next_page` value in the output with `--page` to get another page. The CLI does not get more pages automatically.\n\
         A filter option ID is the number D&D Beyond assigns to a choice in its filter menu. These numbers are not monster IDs and do not unlock content. Each filter accepts up to 50 values; the combined limit is 256.\n\
         If you set both challenge-rating bounds, the minimum must not exceed the maximum.",
    )
    .arg(
        Arg::new("name")
            .long("name")
            .short('n')
            .value_name("NAME")
            .value_parser(ValueParser::new(valid_monster_name))
            .help("Search by monster name; at most 256 bytes"),
    )
    .arg(
        Arg::new("page")
            .long("page")
            .default_value("1")
            .value_parser(value_parser!(u32).range(1..))
            .value_name("PAGE")
            .help("Page number, starting at 1. Use `next_page` from the output to continue"),
    )
    .arg(
        Arg::new("partnered-content")
            .long("partnered-content")
            .default_value("no")
            .value_name("VALUE")
            .value_parser(["yes", "no", "any"])
            .help("Use yes to include partner content, no to exclude it (default), or any"),
    );
    for (flag, help) in [
        ("cr-min", "Lowest challenge rating to include"),
        ("cr-max", "Highest challenge rating to include"),
    ] {
        command = command.arg(
            Arg::new(flag)
                .long(flag)
                .value_name("RATING")
                .value_parser(clap::builder::PossibleValuesParser::new(
                    CHALLENGE_RATINGS.iter().map(|(label, _)| *label),
                ))
                .help(help),
        );
    }
    for (_, flag, _) in ID_FILTERS {
        command = command.arg(
            Arg::new(flag)
                .long(flag)
                .value_name("ID")
                .value_parser(value_parser!(u32).range(1..))
                .action(ArgAction::Append)
                .help(
                    "Number assigned to a D&D Beyond filter choice; repeat to choose more than one",
                ),
        );
    }
    for (_, flag, _) in NUMBER_FILTERS {
        let label = match flag {
            "armor-class-min" | "armor-class-max" => "armor class",
            "average-hp-min" | "average-hp-max" => "average hit points",
            _ => "value",
        };
        let bound = if flag.ends_with("-min") {
            "Lowest"
        } else {
            "Highest"
        };
        command = command.arg(
            Arg::new(flag)
                .long(flag)
                .value_name("NUMBER")
                .value_parser(value_parser!(u32))
                .help(format!("{bound} {label} to include; enter a whole number")),
        );
    }
    for (_, flag, _) in BOOL_FILTERS {
        let label = match flag {
            "legendary" => "whether the monster is legendary",
            "mythic" => "whether the monster is mythic",
            "has-lair" => "whether the monster has a lair",
            _ => "this filter",
        };
        command = command.arg(
            Arg::new(flag)
                .long(flag)
                .value_name("true|false")
                .value_parser(value_parser!(bool))
                .help(format!(
                    "Set {label} to true or false; omit to include either"
                )),
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

/// Builds class and subclass commands from the same completion definition.
fn class_resource(name: &'static str, subclass: bool) -> Command {
    let (resource_about, list_about, show_about, list_details, show_details) = if subclass {
        (
            "List subclasses for one class or for all classes",
            "List subclass names and IDs",
            "Show one subclass using its ID",
            "Use `--name` to match part of a subclass name. The search ignores letter case.",
            "Copy the subclass ID from `subclass list`.\nDefault and JSON output show names and source details only (metadata).\nExplicit html or markdown reads one checked subclass fragment; missing fragments fail.",
        )
    } else {
        (
            "List classes or show one class's details",
            "List class names and IDs",
            "Show one class using its ID",
            "Use `--name` to match part of a class name. The search ignores letter case.",
            "Copy the class ID from `class list`.\nDefault and JSON output show names and source details only (metadata).\nExplicit html or markdown reads checked primary class content and excludes subclasses.",
        )
    };
    let list = protected_action_with_details(
        list_options(Command::new("list").about(list_about)),
        list_details,
    )
    .arg(
        Arg::new("name")
            .long("name")
            .short('n')
            .value_name("NAME")
            .value_parser(ValueParser::new(valid_class_name))
            .help("Match part of a name; letter case does not matter"),
    );
    let show = protected_action_with_details(
        show_options(Command::new("show").about(show_about)),
        show_details,
    )
    .arg(
        Arg::new("id")
            .required(true)
            .value_name("ID")
            .value_parser(ValueParser::new(valid_class_identifier))
            .help(if subclass {
                "ID shown by `subclass list`"
            } else {
                "ID shown by `class list`"
            }),
    );
    let resource_details = if subclass {
        "These commands show names and source details only. They do not show game rules or full subclass text."
    } else {
        "These commands show names and source details only. They do not show game rules or full class text."
    };
    protected_resource(name, resource_about)
        .after_help(format!("{resource_details}\n\n{PROTECTED_READ_HELP}"))
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
            .help("Show subclasses for this class ID; omit to search all classes"),
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
        .after_help(if name == "install" {
            "Connects the browser extension to a small local helper program. The folder must contain `beyond-gate.exe` and `beyond-gate-native-host.exe`.\n\
             Choose a stable folder outside the source and build folders. Install the browser extension separately.\n\
             Use `--dry-run` to preview without changing browser settings."
        } else {
            "Removes the Beyond Gate connection for the selected browser. It does not remove the extension or either program.\n\
             Use `--dry-run` to preview without changing browser settings."
        })
        .arg(
            Arg::new("browser")
                .required(true)
                .value_name("BROWSER")
                .value_parser(["chrome", "edge"])
                .help("Choose your browser: chrome or edge"),
        )
        .arg(
            Arg::new("directory")
                .long("directory")
                .required(true)
                .value_name("PATH")
                .value_parser(value_parser!(std::path::PathBuf))
                .help("Folder containing `beyond-gate.exe` and `beyond-gate-native-host.exe`"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Show what would change without asking or saving"),
        )
}

/// Builds the shared spell filter surface without character-context API selectors.
fn spell_list() -> Command {
    let mut command = protected_action_with_details(
        list_options(Command::new("list").about("Search one page of spells")),
        "Use the `next_page` value in the output with `--page` to get another page. The CLI does not get more pages automatically.\n\
         Filter option IDs are numbers assigned to choices in D&D Beyond's filter menus; they are not spell IDs. Most accept up to 50 values; the combined limit is 256. Level is 0 through 9 and school is 3 through 10.\n\
         Level 0 means a cantrip.",
    )
    .arg(
        Arg::new("name")
            .long("name")
            .short('n')
            .value_name("NAME")
            .value_parser(ValueParser::new(valid_spell_name))
            .help("Search by spell name; at most 256 bytes"),
    )
    .arg(
        Arg::new("page")
            .long("page")
            .value_name("PAGE")
            .default_value("1")
            .value_parser(value_parser!(u32).range(1..))
            .help("Page number, starting at 1. Use `next_page` from the output to continue"),
    )
    .arg(
        Arg::new("partnered-content")
            .long("partnered-content")
            .value_name("VALUE")
            .default_value("no")
            .value_parser(["no", "any", "yes"])
            .help("Use yes to include partner content, no to exclude it (default), or any"),
    );
    for (field, flag, _) in beyond_gate_core::spell::query::ID_FILTERS {
        let parser = if field == "levels" {
            value_parser!(u32).range(0..=9)
        } else if field == "school_ids" {
            value_parser!(u32).range(3..=10)
        } else {
            value_parser!(u32).range(1..)
        };
        command = command.arg(
            Arg::new(flag)
                .long(flag)
                .value_name(if field == "levels" { "LEVEL" } else { "ID" })
                .value_parser(parser)
                .action(ArgAction::Append)
                .help(match field {
                    "levels" => "Spell level; 0 means a cantrip. Repeat to choose more than one.",
                    "school_ids" => "School option number from 3 to 10",
                    _ => "Number assigned to a D&D Beyond filter choice; repeat to choose more than one",
                }),
        );
    }
    for (_, flag, _) in beyond_gate_core::spell::query::BOOL_FILTERS {
        command = command.arg(
            Arg::new(flag)
                .long(flag)
                .value_name("BOOL")
                .value_parser(value_parser!(bool))
                .help("Set to true or false; omit to include either"),
        );
    }
    command
}

/// Checks a complete website identity before any session access.
fn valid_spell_slug(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::spell::validate_slug(value)
        .map(|()| value.to_owned())
        .map_err(|_| "slug must be a complete canonical numeric-ID/slug key within 256 ASCII bytes")
}

/// Checks literal spell searches before protected storage or network access.
fn valid_spell_name(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::spell::SpellQuery {
        name: Some(value.to_owned()),
        ..Default::default()
    }
    .validate()
    .map(|()| value.to_owned())
    .map_err(|_| "Spell name must be valid text within 256 UTF-8 bytes without padding or control characters")
}

/// Requires an explicit action and makes nested help deterministic.
fn resource(name: &'static str, about: &'static str) -> Command {
    Command::new(name)
        .about(about)
        .arg_required_else_help(true)
        .subcommand_required(true)
}

/// Adds the shared saved-session requirement to a data resource's long help.
fn protected_resource(name: &'static str, about: &'static str) -> Command {
    resource(name, about).after_help(PROTECTED_READ_HELP)
}

/// Validates opaque selections without inventing upstream identifier rules.
fn identifier(name: &'static str) -> Arg {
    Arg::new(name)
        .required(true)
        .value_name(match name {
            "book-id" => "BOOK_ID",
            "section-id" => "SECTION_ID",
            _ => "ID",
        })
        .help(match name {
            "book-id" => "Book ID shown by `book list`",
            "section-id" => "Section ID shown by `book outline`",
            _ => "ID shown by the matching list command",
        })
        .value_parser(ValueParser::new(valid_identifier))
}

/// Builds the full item identity required with an explicit mixed source page.
fn item_identifier() -> Arg {
    Arg::new("id")
        .required(true)
        .value_name("ITEM_ID")
        .value_parser(ValueParser::new(valid_item_id))
        .help("Full item ID shown by the matching list; keep both parts separated by a colon")
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
    protected_action(output_options(command, "table"))
}

/// Adds output controls with labelled text for one selected record.
fn show_options(command: Command) -> Command {
    protected_action(output_options(command, "text"))
        .mut_arg("output", |arg| {
            arg.value_parser(["text", "markdown", "html", "table", "json", "tsv"])
                .help("Choose text, markdown, html, table, json, or tsv (default: text); html returns checked selected content")
        })
        .arg(
            Arg::new("html-field")
                .long("html-field")
                .value_name("FIELD_PATH")
                .value_parser(ValueParser::new(valid_html_field))
                .help("Select a fixed snake_case HTML source field for html or markdown; required when supported links are ambiguous"),
        )
}

/// Rejects arbitrary URLs, upstream casing, and noncanonical array positions.
fn valid_html_field(value: &str) -> Result<String, &'static str> {
    beyond_gate_core::record_html::validate_field(value)
        .map(|()| value.to_owned())
        .map_err(|_| "select a documented snake_case HTML source field")
}

/// Adds the saved-session requirement to every protected read action.
fn protected_action(command: Command) -> Command {
    command.after_help(PROTECTED_READ_HELP)
}

/// Adds command details before the shared saved-session note.
fn protected_action_with_details(command: Command, details: &'static str) -> Command {
    command.after_help(format!("{details}\n\n{PROTECTED_READ_HELP}"))
}

/// Defines the shared output selector, compatibility alias, and header control.
fn output_options(command: Command, default: &'static str) -> Command {
    let formats = if default == "text" {
        vec!["text", "markdown", "table", "json", "tsv"]
    } else {
        vec!["table", "json", "tsv"]
    };
    command
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .visible_alias("format")
                .value_name("FORMAT")
                .default_value(default)
                .value_parser(formats.clone())
                .help(format!(
                    "Choose {} output (default: {default}); json keeps all fields",
                    formats.join(", ")
                )),
        )
        .arg(
            Arg::new("no-headers")
                .long("no-headers")
                .action(ArgAction::SetTrue)
                .help("Hide column names and table underlines; only valid with table or tsv"),
        )
}
