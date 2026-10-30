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

The [manual capability suite](docs/testing.md) runs both real executables locally
or through a manual Actions workflow. It reports unsupported contracts separately
from blocked live reads. Its credential-free subset can pass; full coverage
cannot pass until production live prerequisites exist.

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
directory. Windows adds `.exe`. See the [native installation guide](docs/installation.md)
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

`auth login` reports the missing browser handoff without collecting credentials.
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
| `docs` | CLI/MCP references, installation, architecture, authentication, and roadmap |
| `.github` | Agent guidance, issue/PR templates, CI, and release packaging |

This structure adapts [ArcadeJanitor](https://github.com/PlagueHO/arcade-janitor).
It does not copy that project's ROM operations or HTTP MCP transport.

## Resource commands and output

The [CLI command reference](docs/cli.md) covers every command, option, output
field, format, and error. The resource actions are `character list/show`,
`campaign list/show`, and `book list/outline/read`.

**Live reads remain unsupported.** Valid reads exit with code 1, write a
diagnostic to stderr, and leave stdout empty. Offline tests check successful
output with synthetic inputs; the product has no fixture mode.
See [output contracts](docs/cli.md#output-contracts) for table, JSON, and TSV
fields, escaping, source attribution, and bounded explicit continuation.

## Shell completions

`beyond-gate completions <shell>` generates scripts without account or network
access. See [completion commands and setup](docs/cli.md#completions) for
supported shells and Bash/PowerShell examples.

## Local MCP server

Start `beyond-gate-mcp` directly or use `beyond-gate mcp start` with both binaries
in the same directory. The server uses local stdio only; stdout is reserved for
MCP protocol messages. Its seven read-only tools currently return explicit
unsupported-operation errors.

See the [MCP tool reference](docs/mcp.md) for startup, client registration, strict
input schemas, output envelopes, errors, and CLI mappings.
**Registration changes client settings after confirmation**; use the
[installer preview](docs/cli.md#mcp-install) before applying changes.
Do not put credentials in CLI arguments, tool inputs, or client settings.

See the [roadmap](docs/roadmap.md), [architecture](docs/architecture.md), and
[authentication design](docs/authentication.md). See [AGENTS.md](AGENTS.md) for
agent operations and [.github/copilot-instructions.md](.github/copilot-instructions.md)
for Rust authoring rules.

## License

[MIT](LICENSE) covers this project's code and documentation.
It does not grant rights to D&D Beyond content, trademarks, or services.
