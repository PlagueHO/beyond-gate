# Beyond Gate

[![CI](https://github.com/PlagueHO/beyond-gate/actions/workflows/ci.yml/badge.svg)](https://github.com/PlagueHO/beyond-gate/actions/workflows/ci.yml)

Beyond Gate is a local-first Rust CLI and Model Context Protocol (MCP) server
project for access to your authorized D&D Beyond characters, campaigns, and
purchased or shared books.

**Status: Windows login, Web-library discovery, and bounded book reads.**
The CLI and MCP binaries provide `--help` and `--version`.
The CLI also provides local `auth status` and `auth logout`.
Windows `auth validate` and MCP `auth_validate` now make one fresh service check
through the same protected-session lifecycle. Synthetic stores, local services,
and actual adapter process tests cover validation and bounded bearer reuse.
Windows `auth login` now starts one-use terminal pairing and calls shared
completion after a bound submit. Windows setup, real human-controlled login,
protected save, restart/status, and logout passed with Chrome and Edge Dev.
Other platforms retain the unsupported error.
The CLI also generates shell completions. The MCP server uses stdio
and exposes matching read tools. Windows `book list` and MCP `book_list` now
have bounded live metadata transport through the shared stored session.
Category selection and title cleanup are implemented and checked offline.
The Web catalog defines the result scope. Native CLI/MCP acceptance passed with
matching metadata and evidence for 169 books.
Windows outline and section reads now use fresh library evidence and checked
canonical cookie pages. Unknown access and unsupported layouts return no content.
The production source path has synthetic CLI/MCP process checks.
[Native book acceptance](docs/api-survey.md#native-book-adapter-acceptance) passed
for one selected chapter. Live continuation and sharing remain unchecked.
Character and campaign reads use fixed, bounded operations through the stored
Windows session. Synthetic process checks cover adapter parity; live source
compatibility has not been validated.
Linux production login and session use
are tracked in [#52](https://github.com/PlagueHO/beyond-gate/issues/52).
No account credentials or live D&D Beyond access are needed to build or test.
An ignored [developer authentication probe](docs/authentication.md#opt-in-session-exchange-probe)
can test a manually supplied session separately. The user reported one successful
exchange on 2026-10-27. It is not product login and does not verify data reads.

The [manual capability suite](docs/testing.md) runs both real executables locally
or through a manual Actions workflow. It reports unsupported contracts separately
from blocked live reads. Its credential-free subset can pass; full coverage
cannot pass until production live prerequisites exist.

The core has an offline-tested [HTTP boundary](docs/architecture.md#http-boundary).
It provides bounded async JSON reads and typed, redacted errors. It has no built-in
D&D Beyond hosts in the generic client. Library discovery selects one fixed public
configuration route and two fixed protected API routes.
Windows paired login and explicit validation share one fixed service exchange.
The shared lifecycle keeps derived bearers in process memory only.

The core also has offline [library metadata assembly](docs/architecture.md#offline-library-boundary).
It validates supplied pages, keeps purchase and sharing evidence separate, and
rejects incomplete or conflicting results. Catalog visibility never grants
access. Live discovery uses a complete catalog capped at 1,000 raw records and
current-account ownership capped at 100 records. Each response stays within 1 MiB.
Account sharing has no invented campaign identifier. Missing ownership remains
unknown. Public configuration excludes the explicit `Hidden Sources` category;
all raw ownership IDs and evidence still validate against public configuration.
Configured ownership-only sources outside the Web catalog do not create books.
Their absence from the Web result is not an access denial. Invalid visible title
whitespace is cleaned before strict metadata validation. Valid titles and URL rules
remain unchanged. No invalid visible records are silently dropped.
No library cache or proprietary fixture is included.

The core also provides offline
[character and campaign record assembly](docs/architecture.md#offline-character-and-campaign-boundary).
It preserves supplied JSON fields, validates opaque identifiers, and bounds
complete pagination. It does not interpret game rules or enable live character
or campaign reads.

The core provides shared
[book outlines and bounded section reads](docs/architecture.md#live-book-reads),
plus offline validators. Each live call checks fresh positive library evidence
and the selected canonical cookie page within one session identity.
Section results contain at most 4,096 UTF-8 text bytes, source attribution, and
an opaque continuation or explicit end state. Outlines allow at most 1,000
raw links. Both adapters use the same source parser and explicit continuation.
Tokens never grant access or select a fetch URL. No book-content cache is added.

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

The installed browser provides human-controlled Windows login. No cloud service,
remote MCP hosting, account sharing, content redistribution, or write operations
are in the v1 scope. Windows login and future D&D Beyond reads need an internet
connection.

This project is unofficial. D&D Beyond endpoints may change without notice.
Content ownership or sharing access does **not** establish permission to automate
access. The [Wizards terms](https://company.wizards.com/en/legal/terms) restrict
unauthorized automated access. The
[2026-10-27 feasibility assessment](docs/architecture.md#feasibility-assessment)
did not establish a permitted integration path. At the maintainer's request,
the project no longer uses that assessment as a blanket development stop gate.
This policy change is not upstream approval. Live reads still need verified
source contracts and adapter implementation. Live account tests and credential
access need separate explicit approval. Windows browser login has approved
product evidence; other platforms remain deferred.
Do not bypass paywalls, access controls, or bot
protection. This repository contains no proprietary content or real account
fixtures.

## Build and try the foundation

Install [Rust with rustup](https://www.rust-lang.org/tools/install).
On Windows, also install the Visual Studio Build Tools C++ workload and Windows SDK.
Use the repository Cargo wrapper for each command. On Windows, it loads the
[Visual Studio 2022 Build Tools environment](CONTRIBUTING.md#windows-build-environment).
The [Copilot App tasks](CONTRIBUTING.md#github-copilot-app-tasks) use this wrapper.
The repository selects Rust 1.99.1 with `rustfmt` and `clippy`.

```powershell
pwsh -NoProfile -File ./scripts/cargo.ps1 build --workspace --locked
pwsh -NoProfile -File ./scripts/cargo.ps1 run --locked -p beyond-gate-cli -- --help
pwsh -NoProfile -File ./scripts/cargo.ps1 run --locked -p beyond-gate-cli -- --version
pwsh -NoProfile -File ./scripts/cargo.ps1 run --locked -p beyond-gate-mcp -- --help
pwsh -NoProfile -File ./scripts/cargo.ps1 run --locked -p beyond-gate-mcp -- --version
```

The CLI shows help and exits with code 2 when no arguments are supplied.
The MCP binary starts its local stdio server when supplied no arguments. It waits
for MCP protocol messages, not terminal input. Unknown arguments or commands
exit with code 2. Help and version requests exit with code 0.

There is no published release yet. Build optimized binaries locally with:

```powershell
pwsh -NoProfile -File ./scripts/cargo.ps1 build --release --workspace --locked
```

The binaries are `beyond-gate` and `beyond-gate-mcp` in the Cargo release output
directory. Windows adds `.exe` and `beyond-gate-native-host.exe`.
See the [native installation guide](docs/installation.md)
for the package matrix, checksum checks, extraction, platform requirements, and
first-run limits. The guide is also included as `installation.md` in every archive.
See [Contributing](CONTRIBUTING.md) for checks and the packaging process.
Choose [native setup](CONTRIBUTING.md#local-development) or the
[development container](CONTRIBUTING.md#development-container).
Use [VS Code tasks](CONTRIBUTING.md#vs-code-tasks) or
[Copilot App tasks](CONTRIBUTING.md#github-copilot-app-tasks) for repeatable checks.
See [Markdown checks](CONTRIBUTING.md#markdown-and-local-links),
[dependency maintenance](CONTRIBUTING.md#dependency-maintenance), and
[maintainer guidance](MAINTAINERS.md) for repository maintenance.

## Local session commands

`auth status` inspects protected local credential presence only. It does not
verify upstream usability or expiry. **`auth logout` deletes the local entry**;
it does not revoke the upstream session or log the browser out.
Both commands can trigger an OS access or unlock dialog.

Windows `auth login` requires interactive stdin, stdout, and stderr. It starts
one-use pairing. The manual browser form and explicit native-host setup commands
are implemented. Both browsers passed synthetic native/core completion checks.
Approved real Chrome and Edge Dev login, protected save, restart/status, and
logout checks passed. See the [tested scope and limits](docs/testing.md#approved-windows-product-login-verification).
Other platforms report unsupported login.
There is no product credential import command. See the
[CLI authentication reference](docs/cli.md#authentication-commands) for syntax,
results, and errors, and the [authentication guide](docs/authentication.md#local-session-commands)
for protected-store requirements and recovery.

## Structure

| Path | Purpose |
|------|---------|
| `src/beyond-gate-core` | Local session operations, offline HTTP policy, metadata assembly, and bounded book reads |
| `src/beyond-gate-cli` | The `beyond-gate` CLI |
| `src/beyond-gate-mcp` | The `beyond-gate-mcp` stdio server and read tools |
| `beyond-gate-browser-extension` | The manual Chrome/Edge login extension and its offline tests |
| `docs` | CLI/MCP references, installation, architecture, authentication, and roadmap |
| `.github` | Agent guidance, issue/PR templates, CI, and release packaging |

This structure adapts [ArcadeJanitor](https://github.com/PlagueHO/arcade-janitor).
It does not copy that project's ROM operations or HTTP MCP transport.

## Resource commands and output

The [CLI command reference](docs/cli.md) covers every command, option, output
field, format, and error. The resource actions are `character list/show`,
`campaign list/show`, `class list/show`, `subclass list/show`, `monster list/show`,
`background list/show`, and
`book list/outline/read`.

**Windows Web-library discovery is implemented and verified through both adapters.**
`book list` uses public category classification and the fixed protected session.
An earlier native check stopped on ownership-only sources outside the Web catalog.
The user then approved Web-scoped results with validation of all account evidence.
The approved native check returned matching results: 36 purchased and 133 unknown.
No live sharing was observed; sharing behavior has synthetic coverage.
Windows book outline/read operations are implemented for the checked HTML layout.
Use a book ID from `book list` and a section ID from `book outline`.
Character and campaign reads use fixed, bounded Windows operations through the
stored session. Failures exit with code 1, write a diagnostic to stderr, and
leave stdout empty. Offline tests use synthetic services through
the real shared transport and adapters; the product has no fixture mode.
Class/subclass commands return bounded catalog metadata, not descriptions,
features, or raw definitions. `--name` / `-n` matches locally and keeps duplicate
names. Subclasses support a parent selector or bounded global discovery.
Browser catalog contracts were inspected; native compatibility, sharing, and
live global traversal remain unverified.
Monster commands support website filters and explicit `--page` continuation,
or one selected raw detail. Repeated ID filters, literal CR bounds, AC/HP ranges,
state booleans, and partnered content have matching MCP inputs.
Nonempty lists use one website read and one exact selected-ID access check.
Only observed access code 1 permits output; code 4 is denied, and
unverified codes, including 2 and 3, are unsupported. Browser probes checked
selected routes. Native cookie compatibility, native monster account acceptance,
larger batches, and owned/shared behavior remain unverified.
Background commands use observed website filters for metadata and select one
unchanged raw definition from a fresh complete authenticated collection.
No ownership or monster access code is inferred. The observed live collection
exceeds the unchanged 1 MiB transport cap, so show rejects it explicitly.
Native background compatibility, account acceptance, and sharing remain
unverified. See the [background contract](docs/cli.md#background-metadata-and-selected-detail).
See [output contracts](docs/cli.md#output-contracts) for table, JSON, and TSV
fields, escaping, source attribution, and bounded explicit continuation.

## Shell completions

`beyond-gate completions <shell>` generates scripts without account or network
access. See [completion commands and setup](docs/cli.md#completions) for
supported shells and Bash/PowerShell examples.

## Local MCP server

Start `beyond-gate-mcp` directly or use `beyond-gate mcp start` with both binaries
in the same directory. The server uses local stdio only; stdout is reserved for
MCP protocol messages. `book_list` uses shared Windows library discovery.
`book_outline` and `book_read` use the shared checked source path.
Character and campaign tools use the shared core reads. They have synthetic
process coverage, but no live acceptance.
`class_list` and `class_show` use the same metadata-only core operations.
`monster_list` and `monster_show` use the same bounded monster core operations.
`background_list` and `background_show` use the same bounded background core
operations and explicit size failures.

See the [MCP tool reference](docs/mcp.md) for startup, client registration, strict
input schemas, output envelopes, errors, and CLI mappings.
**Registration changes client settings after confirmation**; use the
[installer preview](docs/cli.md#mcp-install) before applying changes.
Do not put credentials in CLI arguments, tool inputs, or client settings.

See the [roadmap](docs/roadmap.md), [architecture](docs/architecture.md), and
[authentication design](docs/authentication.md). See [AGENTS.md](AGENTS.md) for
agent operations and [.github/copilot-instructions.md](.github/copilot-instructions.md)
for Rust authoring rules.
Use the [shared API call catalog](docs/api-catalog.md) to plan resource work.
It separates implemented authentication, observed reads, unverified candidates, and
excluded integration calls. The [API survey](docs/api-survey.md) retains supporting evidence.

## License

[MIT](LICENSE) covers this project's code and documentation.
It does not grant rights to D&D Beyond content, trademarks, or services.
