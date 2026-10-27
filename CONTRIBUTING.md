# Contributing

Start with the [roadmap](docs/roadmap.md). The project is a foundation, not a working
D&D Beyond client. Use an implementation issue for a new product operation.

## Local development

Install rustup. Windows needs Visual Studio Build Tools with the C++ workload and
Windows SDK. Use the toolchain in `rust-toolchain.toml`; do not depend on a
machine-specific path. Cargo manages all Rust dependencies.

If Windows reports a missing `msvcrt.lib`, `vcruntime.h`, or `stdint.h`, initialize
the Developer Command Prompt or Developer PowerShell for a complete Visual Studio
Build Tools installation. The probe's TLS dependency also compiles C code.
Run Cargo in that same terminal. A second, incomplete Visual Studio installation
can otherwise be selected automatically. Do not commit local compiler or linker
paths.

To initialize an existing PowerShell terminal, select a registered installation
with C++ tools. This changes only the current shell environment and keeps the
current directory:

```powershell
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) {
    throw 'No complete Visual Studio C++ Build Tools installation was found.'
}
& (Join-Path $vs 'Common7\Tools\Launch-VsDevShell.ps1') -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
```

See Microsoft's [developer shell guidance](https://learn.microsoft.com/en-us/visualstudio/ide/reference/command-prompt-powershell).
Check probe compilation without requesting a cookie or sending a live request:

```shell
cargo test -p beyond-gate-core --test auth_probe --locked --no-run
```

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
