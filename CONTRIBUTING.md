# Contributing

Start with the [roadmap](docs/roadmap.md). The project is a foundation, not a working
D&D Beyond client. Use an implementation issue for a new product operation.

## Local development

Install rustup. Windows needs Visual Studio Build Tools with the C++ workload and
Windows SDK. Use the toolchain in `rust-toolchain.toml`; do not depend on a
machine-specific path. Cargo manages all Rust dependencies.

If Windows reports a missing `msvcrt.lib`, open the Developer Command Prompt or
Developer PowerShell for a complete Visual Studio Build Tools installation.
Run Cargo in that same terminal. A second, incomplete Visual Studio installation
can otherwise be selected automatically. Do not commit local linker paths.

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
