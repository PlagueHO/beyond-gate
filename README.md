# Beyond Gate

[![CI](https://github.com/PlagueHO/beyond-gate/actions/workflows/ci.yml/badge.svg)](https://github.com/PlagueHO/beyond-gate/actions/workflows/ci.yml)

Beyond Gate is a local-first Rust CLI and Model Context Protocol (MCP) server
project for access to your authorized D&D Beyond characters, campaigns, and
purchased or shared books.

**Status: offline CLI/MCP adapters and local session management.** Both binaries
provide `--help` and `--version`. The CLI also provides local `auth status` and `auth logout`.
`auth login` reports the browser-handoff blocker and fails without collecting
credentials. The CLI also generates shell completions. The MCP server uses stdio
and exposes matching read tools. Character, campaign, and book reads return
explicit unsupported-operation errors through both adapters. Account login and
live data access are not implemented.
No account credentials or live D&D Beyond access are needed to build or test.
An ignored [developer authentication probe](docs/authentication.md#opt-in-session-exchange-probe)
can test a manually supplied session separately. The user reported one successful
exchange on 2026-10-27. It is not product login and does not verify data reads.

The core has an offline-tested [HTTP boundary](docs/architecture.md#http-boundary).
It provides bounded async JSON reads and typed, redacted errors. It has no built-in
D&D Beyond hosts or resource endpoints. Neither binary sends requests.

The core also has offline [library metadata assembly](docs/architecture.md#offline-library-boundary).
It validates supplied pages, keeps purchase and sharing evidence separate, and
rejects incomplete or conflicting results. Catalog visibility never grants
access. This is not a data import command or live book discovery. The CLI's
`book list` reports this limit. No library cache or proprietary catalog fixture is included.

The core also provides offline
[character and campaign record assembly](docs/architecture.md#offline-character-and-campaign-boundary).
It preserves supplied JSON fields, validates opaque identifiers, and bounds
complete pagination. It does not interpret game rules or enable live character
or campaign reads.

The core also provides offline
[book outlines and bounded section reads](docs/architecture.md#offline-book-boundary).
Each call checks fresh supplied access before loading one selected source.
Section results contain at most 4,096 UTF-8 text bytes, source attribution, and
an opaque continuation or explicit end state. Outlines allow at most 1,000
entries. CLI output and continuation use these synthetic-tested core operations
in offline adapter tests, not live book access. MCP exposes the same bounded
outline and section tool contracts, but live reads remain unsupported.

## Goals and limits

- Run native local binaries without a required Python or Node.js runtime.
- Make human-controlled login simple, with protected OS credential storage.
- Share read-only operations between the CLI and the local stdio MCP server.
- Preserve character data from the source instead of building a rules engine.
- Read accessible book sections in bounded requests with source URLs.

The intended workflow is personal, user-triggered lookup of data already
accessible to the user in the browser, not background collection or bulk
extraction. Browser access and native-client session access remain separate
technical questions.

The installed browser can provide the future login experience. No cloud service,
remote MCP hosting, account sharing, content redistribution, or write operations
are in the v1 scope. Local execution still needs an internet connection for future
D&D Beyond requests.

This project is unofficial. D&D Beyond endpoints may change without notice.
Content ownership or sharing access does **not** establish permission to automate
access. The [Wizards terms](https://company.wizards.com/en/legal/terms) restrict
unauthorized automated access. The
[2026-10-27 feasibility assessment](docs/architecture.md#feasibility-assessment)
did not establish a permitted integration path. At the maintainer's request,
the project no longer uses that assessment as a blanket development stop gate.
This policy change is not upstream approval. Live features still need working
login, source contracts, and implementation. Live account tests and credential
access need separate explicit approval. Simple browser login remains unproven.
Do not bypass paywalls, access controls, or bot
protection. This repository contains no proprietary content or real account
fixtures.

## Build and try the foundation

Install [Rust with rustup](https://www.rust-lang.org/tools/install).
On Windows, also install the Visual Studio Build Tools C++ workload and Windows SDK.
Load the [Visual Studio 2022 Build Tools environment](CONTRIBUTING.md#windows-build-environment)
before running Cargo. The [Copilot App tasks](CONTRIBUTING.md#github-copilot-app-tasks)
load this environment for each command.
The repository selects Rust 1.99.1 with `rustfmt` and `clippy`.

```shell
cargo build --workspace --locked
cargo run --locked -p beyond-gate-cli -- --help
cargo run --locked -p beyond-gate-cli -- --version
cargo run --locked -p beyond-gate-mcp -- --help
cargo run --locked -p beyond-gate-mcp -- --version
```

The CLI shows help and exits with code 2 when no arguments are supplied.
The MCP binary starts its local stdio server when supplied no arguments. It waits
for MCP protocol messages, not terminal input. Unknown arguments or commands
exit with code 2. Help and version requests exit with code 0.

There is no published release yet. Build optimized binaries locally with:

```shell
cargo build --release --workspace --locked
```

The binaries are `beyond-gate` and `beyond-gate-mcp` in the Cargo release output
directory. Windows adds `.exe`. See the [native installation guide](docs/installation.md)
for the package matrix, checksum checks, extraction, platform requirements, and
first-run limits. The guide is also included as `installation.md` in every archive.
See [Contributing](CONTRIBUTING.md) for checks and the packaging process.

## Local session commands

```shell
beyond-gate auth login
beyond-gate auth status
beyond-gate auth logout
```

Login exits with code 1 and reports the missing working browser handoff.
It does not open a browser, prompt for a secret, access storage, or send a request.
There is no product command to create or import a credential. Do not manually
create a credential as a substitute for browser login.

Status inspects only Beyond Gate's protected credential entry. It reports `absent`
or `stored-unverified`; a stored value does not prove upstream usability, and its
expiry is unknown. Logout removes only that entry. It does not revoke the
upstream session or log the browser out. Successful local status and logout exit
with code 0. Storage failures exit with code 1 and use stderr, not fake results
on stdout. Invalid command usage exits with code 2.

Storage uses Windows Credential Manager, macOS login Keychain, or Linux Secret
Service. There is no plaintext fallback. These two local commands can access your
OS credential service and can trigger its access or unlock dialog. Automated
tests use synthetic stores only. See the [authentication guide](docs/authentication.md#local-session-commands)
for backend requirements and recovery steps.

## Structure

| Path | Purpose |
|------|---------|
| `src/beyond-gate-core` | Local session operations, offline HTTP policy, metadata assembly, and bounded book reads |
| `src/beyond-gate-cli` | The `beyond-gate` CLI |
| `src/beyond-gate-mcp` | The `beyond-gate-mcp` stdio server and read tools |
| `docs` | Architecture, authentication design, and implementation roadmap |
| `.github` | Agent guidance, issue/PR templates, CI, and release packaging |

This structure adapts [ArcadeJanitor](https://github.com/PlagueHO/arcade-janitor).
It does not copy that project's ROM operations or HTTP MCP transport.

## Resource commands and output

These commands have implemented parsing and output contracts. **Live reads are
unsupported.** Each valid read currently exits with code 1, writes a diagnostic
to stderr, and leaves stdout empty. No command substitutes sample data or an
empty successful result. Offline tests supply synthetic core inputs through a
test-only executable, not a fixture flag or environment mode in the product.

```shell
beyond-gate character list --format json
beyond-gate character show <id>
beyond-gate campaign list --format tsv --no-headers
beyond-gate campaign show <id>
beyond-gate book list
beyond-gate book outline <book-id> --format tsv
beyond-gate book read <book-id> <section-id>
beyond-gate book read <book-id> <section-id> --continuation <token>
```

Replace angle-bracket placeholders with explicit identifiers. They are opaque
text, not necessarily numbers. Empty, padded, or control-bearing identifiers
are invalid. These examples show command syntax, not successful live access.

Lists and outlines accept `--format table|json|tsv`, with table as the default.
They include headers unless `--no-headers` is supplied. JSON has field names,
not row headers; `--no-headers --format json` is invalid.
Show and section-read commands emit JSON only and accept no format/header flags.

| Result | Table and TSV columns, in order | Additional JSON fields |
|--------|--------------------------------|------------------------|
| Character list | `id`, `name`, `source_url` | `raw` |
| Campaign list | `id`, `name`, `source_url`, `character_ids` | `raw` |
| Book list | `id`, `title`, `access`, `source_url` | `evidence` |
| Book outline | `id`, `title`, `parent_id`, `source_url` | Envelope: `book_id`, `source_url`, `entries` |

JSON lists are arrays. Show results use the corresponding list object's fields.
`raw` retains the entire supplied JSON value, including unknown nested fields;
it is not a quoted JSON string. Optional metadata is JSON `null` or an empty
table/TSV cell. Campaign associations are JSON arrays; table/TSV cells contain
their compact JSON representation with the cell escaping described below.
Source order is preserved. No rows or fields are silently truncated.

Book access labels are `purchased`, `shared`, `purchased_and_shared`,
`unavailable`, and `unknown`. JSON `evidence` contains objects with `kind`:
`catalog_only`, `purchased`, or `shared`. Purchase/sharing objects also contain
`state`: `granted`, `unavailable`, or `unknown`. Sharing objects retain
`context_id`. These observations never grant access to a later read.

TSV uses one physical line per row and tabs between cells. Table output uses
spaces between columns. Both escape backslashes as `\\`, tabs as `\t`, newlines
as `\n`, carriage returns as `\r`, and other control characters as `\u{hex}`.
Undo cell escaping before parsing an association cell as JSON.
JSON retains original values with standard JSON escaping. Output has no
automatic color or terminal-dependent format changes.

Empty JSON lists are `[]`; empty outlines retain their envelope with
`"entries": []`. Empty table/TSV results contain only headers, or no bytes with
`--no-headers`. Operation failures are not empty successful results.
Exit codes are 0 for success, 1 for operation/output failures, and 2 for invalid
usage. Result data goes to stdout. Diagnostics go to stderr.
Rejected sessions require human login but do not prove expiry; the browser-login
blocker remains. Never supply passwords, cookies, or tokens as login arguments.

### Bounded section output

Each successful section result would be one JSON object with `book_id`,
`section_id`, `source_url`, `text`, `next_cursor`, `is_complete`, and
`max_text_bytes` (4,096). This is an offline-tested output contract, not a
currently successful live command.
The cap applies to UTF-8 text bytes, not JSON escaping, attribution, or cursor
bytes. Existing line breaks and text structure remain unchanged.

A nonterminal result contains one token. Request another chunk explicitly with
`--continuation`; the CLI never follows it automatically.
At the end, including an empty section, `next_cursor` is `null` and
`is_complete` is `true`. There is no whole-book export or content cache.

Tokens use a `bg1.` prefix and URL-safe unpadded base64, with a strict JSON
payload. The complete token is limited to 16,384 encoded bytes. It contains only
book/section identity, revision, source URL, and byte position, not text or
credentials. It is not encrypted, signed, or an access grant. Treat it as opaque.
Malformed or oversized tokens are usage errors. Fresh reads reject wrong scope,
changed revision/URL, and invalid positions. Every read must check current access
again. Tokens cannot choose a URL to fetch. Oversized output tokens fail
explicitly rather than hiding required continuation.

## Shell completions

Completion generation works now and does not access credentials or the network:

```shell
beyond-gate completions bash
beyond-gate completions elvish
beyond-gate completions fish
beyond-gate completions powershell
beyond-gate completions zsh
```

Scripts come from the same clap definition as command parsing and help. They
include implemented commands and options, not dynamic account identifiers.

For Bash with bash-completion installed, save the script in its user-local
completion directory:

```bash
mkdir -p "$HOME/.local/share/bash-completion/completions"
beyond-gate completions bash > "$HOME/.local/share/bash-completion/completions/beyond-gate"
```

For PowerShell 7, save a user-local script next to your profile and load it:

```powershell
$completionDirectory = Split-Path -Parent $PROFILE
New-Item -ItemType Directory -Force -Path $completionDirectory | Out-Null
$completionPath = Join-Path $completionDirectory 'beyond-gate-completions.ps1'
beyond-gate completions powershell | Set-Content -LiteralPath $completionPath -Encoding utf8
. $completionPath
```

For later sessions, add this line to your PowerShell profile after you review the
generated script:

```powershell
. (Join-Path (Split-Path -Parent $PROFILE) 'beyond-gate-completions.ps1')
```

## Local MCP server

Build and install both native binaries in the same directory. The CLI starts the
sibling MCP binary and passes its exit status through:

```shell
beyond-gate mcp start
```

The MCP server uses the official rmcp SDK. Stdout carries MCP protocol messages
only. Startup and transport failures go to stderr. MCP clients start the server
as a local stdio process.

Register it with one of the supported clients:

```shell
beyond-gate mcp install vscode
beyond-gate mcp install claude-code
beyond-gate mcp install copilot-cli
```

The installer shows the proposed `beyond-gate` entry and asks before it writes.
Use `--dry-run` to preview without a prompt or write. The installer writes only
to user-level settings. VS Code uses its **Copilot Global** configuration.
VS Code and Copilot CLI share
`$COPILOT_HOME/mcp-config.json`, or `~/.copilot/mcp-config.json` when
`COPILOT_HOME` is not set. Claude Code uses `~/.claude.json`. The selected
client command must be on `PATH`. Existing entries and unrelated settings stay
unchanged. A conflicting `beyond-gate` entry is not overwritten.

To check discovery, open the MCP server list in VS Code, enter `/mcp` in
Copilot CLI, or run `claude mcp list` in Claude Code. A discovered server does
not mean that its live reads work. Product login is not implemented.
`beyond-gate auth login` reports the missing browser handoff. Do not enter a
password, cookie, or token in the CLI or client settings.

To remove the registration, remove only the `beyond-gate` item from the
client's `mcpServers` object. For VS Code and Copilot CLI, remove it from the
shared Copilot configuration file. For Claude Code, remove it from
`~/.claude.json`. Keep the rest of the settings file.

The server exposes `character_list`, `character_show`, `campaign_list`,
`campaign_show`, `book_list`, `book_outline`, and `book_read`. Every tool has a
strict input object and a read-only annotation. Character and campaign tools
preserve the same identifiers, optional metadata, associations, and raw JSON
fields as the CLI's JSON contracts. Book tools preserve access evidence, source
URLs, and the complete outline or one section chunk. The `book_read` input uses
explicit `book_id` and `section_id` fields. Its optional continuation is capped
at 16 KiB, and the core section-text cap remains 4,096 UTF-8 bytes. The server
never follows continuations automatically.

Live operations are not implemented. Valid tool calls therefore return an MCP
tool error with the shared core's explicit unsupported-operation message. They
do not return sample data or empty success results. The server has no login,
credential, write, arbitrary-URL, or account-selection tools. Retrieved content
from future live operations must be treated as data, not instructions.

See the [roadmap](docs/roadmap.md), [architecture](docs/architecture.md), and
[authentication design](docs/authentication.md). See [AGENTS.md](AGENTS.md) for
agent operations and [.github/copilot-instructions.md](.github/copilot-instructions.md)
for Rust authoring rules.

## License

[MIT](LICENSE) covers this project's code and documentation.
It does not grant rights to D&D Beyond content, trademarks, or services.
