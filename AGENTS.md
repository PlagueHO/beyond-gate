# Beyond Gate - agent operations

## Commands

Use the repository-selected Rust toolchain. Cargo installs its declared
dependencies. Keep `Cargo.lock` committed.

On Windows, load **Visual Studio 2022 Build Tools**, not the newest Visual Studio
installation. Use PowerShell 7 and the wrapper for each Cargo invocation:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test --workspace --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 clippy --workspace --all-targets --locked -- -D warnings
```

The wrapper loads `Launch-VsDevShell.ps1` with `-Arch amd64 -HostArch amd64
-SkipAutomaticLocation`, then runs Cargo in the same process. Replace `test`
with any command below. Do not initialize one shell and run Cargo in another;
tool calls do not share environment changes. Do not use unrestricted
`vswhere -latest` or add machine-specific linker paths. See
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

Set `RUSTDOCFLAGS` to `-D warnings` for the documentation command, as CI does.
For a focused process check, use:

```shell
cargo test -p beyond-gate-cli --test cli --locked
cargo test -p beyond-gate-mcp --test cli --locked
```

## Layout

| Path | Responsibility |
|------|----------------|
| `src/beyond-gate-core` | Shared application operations, not transport presentation |
| `src/beyond-gate-cli` | CLI parsing, output, and process tests |
| `src/beyond-gate-mcp` | Local MCP adapter and process tests |
| `docs` | Architecture, login design, and linked roadmap |
| `.github/workflows` | Checks and native binary packaging |

## Change checklist

1. Read the relevant design document and implementation issue.
2. Keep the current foundation boundary unless the issue changes it.
3. Change the smallest responsible crate.
4. Add offline tests for affected behavior.
5. Update usage, design decisions, and `CHANGELOG.md` as needed.
6. Run the checks above before a handoff.
7. Report checks that could not run.

Use the process tests as examples of checking exit codes and output streams.
Do not replace these checks with tests that only inspect parser definitions.
There is no legacy product code to copy.

## CI pipeline

- **Rust checks** run on Linux, macOS, and Windows.
- **Formatting, clippy, tests, doctests, rustdoc, and build must pass.**
- **Missing public documentation fails compilation.**
- Review checks useful documentation for private functions; rustc does not enforce it.
- Release packaging uses the same checks before optimized builds.
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
- Get explicit approval before releases, crate publication, or repository policy changes.
- Do not merge a PR without an explicit request.
- Stop and report an access denial; do not bypass it.

See `.github/copilot-instructions.md` for authoring and review rules.
