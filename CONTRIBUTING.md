# Contributing

Start with the [roadmap](docs/roadmap.md). The project is a foundation, not a working
D&D Beyond client. Use an implementation issue for a new product operation.

## Local development

Install rustup. Windows needs Visual Studio Build Tools with the C++ workload and
Windows SDK. Use the toolchain in `rust-toolchain.toml`; do not depend on a
machine-specific path. Cargo manages all Rust dependencies.

### Windows build environment

Use the **Visual Studio 2022 Build Tools** developer environment for Cargo run,
build, test, Clippy, and documentation commands. A newer Visual Studio installation
can be incomplete and cause missing `msvcrt.lib`, `vcruntime.h`, or `stdint.h`
errors. The TLS dependency also compiles C code. Do not choose an installation
with an unrestricted `vswhere -latest` query.

To initialize an existing PowerShell terminal at the standard installation path:

```powershell
& 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\Launch-VsDevShell.ps1' -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
```

Run Cargo in that same process. `-SkipAutomaticLocation` keeps the current
directory. A separate agent shell call does not retain these environment changes.
Do not add compiler or linker paths to Cargo configuration.

For repeatable commands, use the repository wrapper in PowerShell 7:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 build --workspace --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test --workspace --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 clippy --workspace --all-targets --locked -- -D warnings
```

The wrapper loads the standard 2022 Build Tools installation on Windows. If that
path does not exist, it uses `vswhere` to find the 2022 Build Tools product with
C++ tools, not another Visual Studio version. It runs from the workspace root,
forwards Cargo arguments, and returns Cargo's exit code. For `doc` and `rustdoc`,
it adds `-D warnings` to `RUSTDOCFLAGS`. On other platforms it runs Cargo without
Visual Studio setup. PowerShell 7 is needed only for this development helper;
the product binaries do not require it.

See Microsoft's [developer shell guidance](https://learn.microsoft.com/en-us/visualstudio/ide/reference/command-prompt-powershell).
Check probe compilation without requesting a cookie or sending a live request:

```shell
cargo test -p beyond-gate-core --test auth_probe --locked --no-run
```

### GitHub Copilot App tasks

[`.github/github-app.yml`](.github/github-app.yml) defines manual Run CLI help,
Run MCP help, Build, Test, Lint, Check formatting, Test API examples, and Build API
docs tasks. Each task uses the wrapper so Windows environment setup and Cargo
run in the same process. The run tasks show help only; they do not access
credentials or start a live operation. No task runs automatically.

Review and accept this repository configuration in the app before using the
tasks. See GitHub's [repository configuration reference](https://docs.github.com/copilot/reference/github-copilot-app-reference/repository-configuration).
These tasks do not change GitHub Actions runner configuration.

### Workspace checks

Run from the workspace root:

```shell
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --doc --locked
```

Build all API documentation, including private items, with warnings as errors.
In PowerShell:

```powershell
$env:RUSTDOCFLAGS = "-D warnings"
cargo doc --workspace --no-deps --document-private-items --locked
```

In a POSIX shell:

```sh
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --locked
```

For a focused change, use the crate and test selectors in [AGENTS.md](AGENTS.md).
After an intentional dependency update, regenerate `Cargo.lock` with Cargo and
review it. Do not change package registries to work around a local setup problem.

## Code and tests

Follow [.github/copilot-instructions.md](.github/copilot-instructions.md).
Document every function with useful Rust documentation, including private helpers
and test functions. Explain non-obvious ownership, borrowing, and error behavior
where it helps a reader learn Rust. Use examples and Errors/Panics/Safety sections
when applicable. Do not add empty sections or comments that repeat obvious code.
See the [Rust documentation guide](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html).

Workspace lints deny missing public documentation and forbid unsafe code.
Clippy and rustdoc warnings fail CI. Review must check private-function
documentation and its usefulness; compiler checks cannot prove this.

Tests must run offline without a real account. Use synthetic examples, not copies
of character responses or book text. Mock network boundaries when they exist.
Do not execute a live login or content read without explicit approval and the
feasibility gate in the roadmap.

The [session-exchange probe](docs/authentication.md#opt-in-session-exchange-probe)
is an ignored developer test, not product login. Its ordinary tests use synthetic
data and local mocks. Run its live case only from an unrecorded interactive
terminal with the required permission. Never pass a cookie through agent tools,
command arguments, environment variables, or files.

## Pull requests

Keep each change small and coherent. Use the PR template. Update tests, affected
documentation, and `CHANGELOG.md` under `Unreleased`. Use Keep a Changelog
categories such as Added, Changed, Fixed, and Security. State which checks ran.
Do not describe a planned feature as implemented.

## Packaging and future releases

The Release packaging workflow builds both binaries on Linux, macOS, and Windows.
A manual run uploads archives and SHA-256 checksums without creating a release.
Assets include the Rust target triple to identify architecture and platform.
The packages include the README, license, and changelog.

An explicit maintainer push of a `vX.Y.Z` tag also prepares a **draft** GitHub
release after all build jobs pass. The tag must match the workspace package version.
The draft needs review before publication. Never publish from a pull request.
No crates are published to crates.io. No release is part of this foundation change.

Before a real release, finish the release-usability roadmap issue. It covers
installation instructions, package smoke tests, and platform-specific limits.
Change the shared version in `Cargo.toml` and lockfile together. Add a dated
changelog entry before tagging.
