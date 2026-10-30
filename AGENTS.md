# Beyond Gate - agent operations

## Commands

Use the repository-selected Rust toolchain. Cargo installs its declared dependencies.
Keep `Cargo.lock` committed.

On Windows, load **Visual Studio 2022 Build Tools**, not the newest Visual Studio
installation. Use PowerShell 7 and the wrapper for each Cargo invocation:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test --workspace --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 clippy --workspace --all-targets --locked -- -D warnings
```

On Windows, run every Cargo command below through the wrapper as shown above.
It loads `Launch-VsDevShell.ps1` with `-Arch amd64 -HostArch amd64 -SkipAutomaticLocation`
and runs Cargo in the same process. Separate tool calls do not share environment
changes. Do not use unrestricted `vswhere -latest` or machine-specific linker paths. See
[Contributing](CONTRIBUTING.md#windows-build-environment) for direct setup and
[Copilot App tasks](CONTRIBUTING.md#github-copilot-app-tasks) for manual tasks.

```shell
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --doc --locked
cargo doc --workspace --no-deps --document-private-items --locked
```

The wrapper sets `RUSTDOCFLAGS` to `-D warnings`; set it yourself for direct documentation builds.
For focused process checks, use:

```shell
cargo test -p beyond-gate-cli --test cli --locked
cargo test -p beyond-gate-cli --test adapter_process --locked
cargo test -p beyond-gate-mcp --test cli --locked
```

## Layout

| Path | Responsibility |
|------|----------------|
| `src/beyond-gate-core` | Local sessions, offline HTTP policy, record assembly, and bounded book reads |
| `src/beyond-gate-cli` | CLI parsing, output, completions, local MCP start/install, and process tests |
| `src/beyond-gate-mcp` | Local rmcp stdio server, read tools, and protocol process tests |
| `docs` | Architecture, login design, and linked roadmap |
| `scripts` | Cargo environment setup and offline package checks |
| `.github/workflows` | Checks and native binary packaging |

Follow the [roadmap's development policy](docs/roadmap.md#development-policy-and-technical-status).
The historical permission stop gate is removed; live features still need technical contracts.

## Change checklist

1. Read the relevant design document and implementation issue.
2. Keep the current implementation boundary unless the issue changes it.
3. Change the smallest responsible crate.
4. Add offline tests for affected behavior.
5. Update usage, design decisions, and `CHANGELOG.md` as needed.
6. Run the checks above for Rust changes; check documentation-only changes against repository evidence.
7. Report checks that could not run.

Use CLI `tests/cli.rs` for exit codes and output streams, CLI
`tests/adapter_process.rs` for synthetic successful output and continuation, and
MCP `tests/cli.rs` for protocol checks. Parser-only tests do not replace them.

## CI pipeline

- **Rust checks** run on Linux, macOS, and Windows.
- **Formatting, clippy, tests, doctests, rustdoc, and build must pass.**
- **Missing public documentation fails compilation; unsafe code is forbidden.**
- Review checks useful documentation for private functions; rustc does not enforce it.
- Release packaging uses the same checks before optimized builds on fixed native runners.
- Packaging checks checksums, archive contents, extracted binaries, and rejection cases.
- Manual packaging uploads artifacts only; a version tag can create a draft release.
- No workflow publishes Cargo crates.

## Conventions

| Concern | Rule |
|---------|------|
| Workspace metadata | Inherit package version, edition, license, and lints |
| Files | UTF-8, LF, final newline; use EditorConfig and rustfmt |
| Changes | Keep each commit focused; use the PR template |
| Changelog | Use Keep a Changelog categories under `Unreleased` |

## Permission boundaries

- Local source edits and offline checks are normal development work.
- Get explicit approval before live account tests or credential access.
- Do not enable the ignored live authentication test during offline checks.
- Define hosts, operations, request budgets, stop conditions, and cleanup before live tests.
- Get explicit approval before releases, crate publication, or repository policy changes.
- Do not merge a PR without an explicit request.
- Stop and report an access denial; do not bypass it.

See [.github/copilot-instructions.md](.github/copilot-instructions.md) for authoring rules.
