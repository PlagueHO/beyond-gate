# Beyond Gate

[![CI](https://github.com/PlagueHO/beyond-gate/actions/workflows/ci.yml/badge.svg)](https://github.com/PlagueHO/beyond-gate/actions/workflows/ci.yml)

Beyond Gate is a local-first Rust CLI and Model Context Protocol (MCP) server
project for access to your authorized D&D Beyond characters, campaigns, and
purchased or shared books.

**Status: foundation only.** Both binaries provide `--help` and `--version`.
Account login, data access, and MCP serving are not implemented. No account
credentials or network access are needed to build or test this foundation.

## Goals and limits

- Run native local binaries without a required Python or Node.js runtime.
- Make human-controlled login simple, with protected OS credential storage.
- Share read-only operations between the CLI and the local stdio MCP server.
- Preserve character data from the source instead of building a rules engine.
- Read accessible book sections in bounded requests with source URLs.

The installed browser can provide the future login experience. No cloud service,
remote MCP hosting, account sharing, content redistribution, or write operations
are in the v1 scope. Local execution still needs an internet connection for future
D&D Beyond requests.

This project is unofficial. D&D Beyond endpoints may change without notice.
Content ownership or sharing access does **not** establish permission to automate
access. The [Wizards terms](https://company.wizards.com/en/legal/terms) restrict
unauthorized automated access. The first roadmap item must establish a permitted
and technically viable path before account integration proceeds. Do not bypass
paywalls, access controls, or bot protection. This repository contains no
proprietary content or real account fixtures.

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

## Structure

| Path | Purpose |
|------|---------|
| `src/beyond-gate-core` | Shared library; currently the foundation status message |
| `src/beyond-gate-cli` | The `beyond-gate` CLI |
| `src/beyond-gate-mcp` | The `beyond-gate-mcp` binary; server work is pending |
| `docs` | Architecture, authentication design, and implementation roadmap |
| `.github` | Agent guidance, issue/PR templates, CI, and release packaging |

This structure adapts [ArcadeJanitor](https://github.com/PlagueHO/arcade-janitor).
It does not copy that project's ROM operations or HTTP MCP transport.

## Planned interface, not working commands

The intended CLI uses resource-first commands: `auth`, `character`, `campaign`,
`book`, `mcp`, and `completions`. List commands will support table, JSON, and TSV
output. Result data goes to stdout. Diagnostics go to stderr. MCP stdout will
contain protocol messages only. These interfaces are proposed, not available.

See the [roadmap](docs/roadmap.md), [architecture](docs/architecture.md), and
[authentication design](docs/authentication.md). See [AGENTS.md](AGENTS.md) for
agent operations and [.github/copilot-instructions.md](.github/copilot-instructions.md)
for Rust authoring rules.

## License

[MIT](LICENSE) covers this project's code and documentation.
It does not grant rights to D&D Beyond content, trademarks, or services.
