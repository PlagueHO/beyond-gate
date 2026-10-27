# Beyond Gate

[![CI](https://github.com/PlagueHO/beyond-gate/actions/workflows/ci.yml/badge.svg)](https://github.com/PlagueHO/beyond-gate/actions/workflows/ci.yml)

Beyond Gate is a local-first Rust CLI and Model Context Protocol (MCP) server
project for access to your authorized D&D Beyond characters, campaigns, and
purchased or shared books.

**Status: local session management only.** Both binaries provide `--help` and
`--version`. The CLI also provides local `auth status` and `auth logout`.
`auth login` reports the browser-handoff blocker and fails without collecting
credentials. Account login, data access, and MCP serving are not implemented.
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
access. This is not a data import command or live book discovery. Neither binary
can list books, and no library cache or proprietary catalog fixture is included.

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
did not establish a permitted integration path. Account integration remains
blocked pending upstream permission and a supported technical contract. Simple
browser login remains unproven. Do not bypass paywalls, access controls, or bot
protection. This repository contains no proprietary content or real account
fixtures.

## Build and try the foundation

Install [Rust with rustup](https://www.rust-lang.org/tools/install).
On Windows, also install the Visual Studio Build Tools C++ workload and Windows SDK.
The repository selects Rust 1.99.1 with `rustfmt` and `clippy`.

```shell
cargo build --workspace --locked
cargo run --locked -p beyond-gate-cli -- --help
cargo run --locked -p beyond-gate-cli -- --version
cargo run --locked -p beyond-gate-mcp -- --help
cargo run --locked -p beyond-gate-mcp -- --version
```

The CLI shows help and exits with code 2 when no arguments are supplied.
The MCP binary exits with code 1 and a stderr diagnostic when started without
arguments. It does not pretend to start a server. Unknown arguments or commands
exit with code 2. Help and version requests exit with code 0.

There is no published release yet. Build optimized binaries locally with:

```shell
cargo build --release --workspace --locked
```

The binaries are `beyond-gate` and `beyond-gate-mcp` in the Cargo release output
directory. Windows adds `.exe`. See [Contributing](CONTRIBUTING.md) for checks and
the packaging process.

## Local session commands

```shell
beyond-gate auth login
beyond-gate auth status
beyond-gate auth logout
```

Login exits with code 1 and reports the missing permitted browser handoff.
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
| `src/beyond-gate-core` | Local session operations, offline HTTP policy, and library metadata assembly |
| `src/beyond-gate-cli` | The `beyond-gate` CLI |
| `src/beyond-gate-mcp` | The `beyond-gate-mcp` binary; server work is pending |
| `docs` | Architecture, authentication design, and implementation roadmap |
| `.github` | Agent guidance, issue/PR templates, CI, and release packaging |

This structure adapts [ArcadeJanitor](https://github.com/PlagueHO/arcade-janitor).
It does not copy that project's ROM operations or HTTP MCP transport.

## Remaining planned interfaces

Local `auth` commands are available as described above. The remaining intended
CLI resources are `character`, `campaign`, `book`, `mcp`, and `completions`.
List commands will support table, JSON, and TSV
output. Result data goes to stdout. Diagnostics go to stderr. MCP stdout will
contain protocol messages only. These remaining interfaces are proposed, not available.

See the [roadmap](docs/roadmap.md), [architecture](docs/architecture.md), and
[authentication design](docs/authentication.md). See [AGENTS.md](AGENTS.md) for
agent operations and [.github/copilot-instructions.md](.github/copilot-instructions.md)
for Rust authoring rules.

## License

[MIT](LICENSE) covers this project's code and documentation.
It does not grant rights to D&D Beyond content, trademarks, or services.
