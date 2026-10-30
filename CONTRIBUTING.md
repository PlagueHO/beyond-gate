# Contributing

Start with the [roadmap](docs/roadmap.md). The project is a foundation, not a working
D&D Beyond client.

## Issues

Select the form that matches the work:

| Form | Use |
|------|-----|
| Bug report | An observable failure, with reproduction steps, environment, and safe diagnostics. |
| Feature request or implementation | New product behavior or one coherent part of the roadmap, with scope and acceptance criteria. |
| Chore | Maintenance, tooling, dependencies, or CI work without new product behavior. |
| Documentation | Documentation-only corrections, missing guidance, or clearer examples. |

Use synthetic data only. Do not include credentials, private URLs, account
identifiers, real account data, or proprietary content. For a feature request or
implementation, write `None` if there are no dependencies. For a bug report,
write `N/A` if there are no safe diagnostics. Leave optional fields empty if
they do not apply.

## Local development

Choose native setup or the development container. Both use the toolchain in
`rust-toolchain.toml`, not a separate moving `stable` toolchain.

For native setup, install rustup and PowerShell 7. Windows needs Visual Studio
2022 Build Tools with the C++ workload and Windows SDK. On Ubuntu, install
`build-essential`, `cmake`, `perl`, `pkg-config`, and `ca-certificates`. On macOS,
install the Xcode command line tools. Cargo manages Rust dependencies.
PowerShell is development tooling, not a product runtime.

From the workspace root, prepare dependencies and the Markdown checker:

```powershell
rustup show
pwsh -NoProfile -File .\scripts\cargo.ps1 fetch --locked
pwsh -NoProfile -File .\scripts\install-doc-tools.ps1
```

In POSIX shells, use `/` separators for these script paths.

### Development container

Install Docker with a running Linux engine and the VS Code Dev Containers
extension. Open the checkout in VS Code. Select **Dev Containers: Reopen in
Container** from the Command Palette.

[The container definition](.devcontainer/devcontainer.json) uses the non-root
`vscode` user. Setup selects `rust-toolchain.toml`, adds rust-analyzer and Rust
sources, fetches locked dependencies, and installs the pinned Markdown checker.
It does not run live tests or start the MCP server. No HTTP ports are forwarded.
Do not add host credential-store mounts or forward account credentials.

Run **Validate workspace** after setup. Linux container checks do not prove
Windows Credential Manager, macOS Keychain, or host Secret Service integration.
The container does not provide product login or a substitute credential store.

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

```powershell
pwsh -NoProfile -File ./scripts/cargo.ps1 test -p beyond-gate-core --test auth_probe --locked --no-run
```

### GitHub Copilot App tasks

[`.github/github-app.yml`](.github/github-app.yml) defines manual build,
release build, formatting check/apply, Clippy (`Lint`), workspace and focused
process tests, doctests, API documentation, help, and documentation-check tasks.
Each Cargo task uses the wrapper so Windows setup and Cargo run in the same
process. Select the named task from the app's task menu.
The run tasks show help only. They do not access credentials or start live
operations. No task runs automatically.

Review and accept this repository configuration in the app before using the
tasks. See GitHub's [repository configuration reference](https://docs.github.com/copilot/reference/github-copilot-app-reference/repository-configuration).
These tasks do not change GitHub Actions runner configuration.

### VS Code tasks

Accept the [recommended extensions](.vscode/extensions.json).
Select **Terminal: Run Task** from the Command Palette, then choose a task.
**Build** and **Test** are the default build and test tasks.
**Validate workspace** runs all checks in sequence, including the release build,
focused process tests, both help commands, Markdown, and offline links.
It also tests the documentation checker's failure cases.
It stops when a prerequisite fails. **Apply formatting** changes Rust files;
it is not part of validation.

[Tasks](.vscode/tasks.json) use argument arrays and the workspace directory, so
checkout paths can contain spaces. Every Windows Cargo task runs PowerShell 7
and `scripts\cargo.ps1`. Other platforms use Cargo directly.
Documentation tasks reject rustdoc warnings on all platforms.

[Editor settings](.vscode/settings.json) also use the wrapper for rust-analyzer
build-script checks and Clippy diagnostics. PowerShell 7 must be on `PATH`.
These commands retain JSON compiler diagnostics. Cargo metadata discovery does
not compile code. No machine-specific compiler path is configured.

Run tasks are manual help commands, not server tasks.
To start stdio MCP manually after a build, use:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 run --locked -p beyond-gate-mcp
```

The process waits for MCP protocol input. It has no HTTP address or health port.
Do not type ordinary text into the protocol stream. Press **Ctrl+C** to stop
manual execution, or stop the process from its terminal. Normal MCP clients
start and stop their own child process. Do not configure this as an unattended
background task.

### Copilot setup on GitHub Actions

[Copilot setup steps](.github/workflows/copilot-setup-steps.yml) prepare native
build dependencies, the repository toolchain, locked Cargo dependencies, and the
Markdown checker. The required job is named `copilot-setup-steps`.
Checkout uses read-only permissions and does not persist Git credentials.
Setup does not access account credentials or enable live authentication tests.

This workflow applies only to Copilot sessions on GitHub Actions. It does not
configure desktop sessions or cloud sandboxes. It must be on the default branch
for Copilot to use it. See GitHub's
[agent environment reference](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/customize-the-agent-environment).

### Workspace checks

Run from the workspace root:

```powershell
pwsh -NoProfile -File ./scripts/cargo.ps1 build --workspace --locked
pwsh -NoProfile -File ./scripts/cargo.ps1 fmt --all -- --check
pwsh -NoProfile -File ./scripts/cargo.ps1 clippy --workspace --all-targets --locked -- -D warnings
pwsh -NoProfile -File ./scripts/cargo.ps1 test --workspace --locked
pwsh -NoProfile -File ./scripts/cargo.ps1 test --workspace --doc --locked
```

Build all API documentation, including private items, with warnings as errors.
Use the wrapper to build API documentation with warnings as errors:

```powershell
pwsh -NoProfile -File ./scripts/cargo.ps1 doc --workspace --no-deps --document-private-items --locked
```

For a focused change, use the crate and test selectors in [AGENTS.md](AGENTS.md).
After an intentional dependency update, regenerate `Cargo.lock` with Cargo and
review it. Do not change package registries to work around a local setup problem.

### Markdown and local links

The development-only checker is pinned to rumdl 0.3.2. The installer uses its
published Cargo lockfile and writes only to `target/dev-tools`. It does not add
product dependencies or require Node.js or Python.

```powershell
pwsh -NoProfile -File .\scripts\install-doc-tools.ps1
pwsh -NoProfile -File .\scripts\check-docs.ps1 -Mode Style
pwsh -NoProfile -File .\scripts\check-docs.ps1 -Mode Links
pwsh -NoProfile -File .\scripts\check-docs.ps1
pwsh -NoProfile -File .\scripts\test-doc-checks.ps1
```

The default mode checks both style and local links. The VS Code and Copilot App
tasks run the same checks as [documentation CI](.github/workflows/docs.yml).
The shared [configuration](.rumdl.toml) checks root Markdown, `docs`, and
Markdown templates and guidance under `.github`. It excludes build output and
generated documentation. Missing tools, wrong tool versions, style violations,
missing local files, and invalid heading anchors fail with nonzero exit codes.

MD051 checks same-file and cross-file anchors. MD057 checks local target paths.
All selected documents are checked together. The checker does not request
external links or send private URLs to a service. See the pinned
[MD051](https://github.com/rvben/rumdl/blob/v0.3.2/docs/md051.md) and
[MD057](https://github.com/rvben/rumdl/blob/v0.3.2/docs/md057.md) references.
The checker tests use temporary synthetic files in a path with spaces. They
verify that style errors, broken files/anchors, wrong versions, and missing tools
fail. They also verify `.github` coverage and build-output exclusion.

### Dependency maintenance

[Dependabot](.github/dependabot.yml) follows `dependabot-monthly-v1` for the root
Cargo workspace and GitHub Actions. Routine checks run on the first day of each
month at 09:00 `Pacific/Auckland`. Routine releases have a three-day cooldown.
Each ecosystem allows five open routine update PRs.

Minor and patch updates are grouped by ecosystem. Major updates remain separate.
Security updates are event-driven and have separate security groups. They do not
wait for the monthly schedule or routine cooldown. This depends on Dependabot
security updates being enabled in repository settings; the configuration does
not change that setting. See the [Dependabot options reference](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference).

For a routine update outside the schedule, select **Check for updates** in the
repository's Dependabot interface. The routine cooldown and PR limit still
apply. Review release notes, compatibility, manifest changes, and `Cargo.lock`.
Keep action references pinned to full commit SHAs. Run the relevant workspace,
process, documentation, and packaging checks. No automatic approval or merging
is configured. No dependency labels are added.

Dependabot does not cover the Rust toolchain, container image and features,
native packages, PowerShell, editor extensions, or the pinned rumdl installer.
Review these tools manually. See [maintainer responsibilities](MAINTAINERS.md).

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
There is no blanket upstream-permission development gate. See the
[development policy](docs/roadmap.md#development-policy-and-technical-status).
Live login, account reads, and credential access still need separate explicit
approval. Define the selected hosts, operations, request budget, stop conditions,
and cleanup before a live test. Stop after an access denial; do not bypass it.

The [session-exchange probe](docs/authentication.md#opt-in-session-exchange-probe)
is an ignored developer test, not product login. Its ordinary tests use synthetic
data and local mocks. Run its live case only from an unrecorded interactive
terminal with separate explicit test approval. Never pass a cookie through agent tools,
command arguments, environment variables, or files.

## Pull requests

Keep each change small and coherent. Use the PR template. Update tests, affected
documentation, and `CHANGELOG.md` under `Unreleased`. Use Keep a Changelog
categories such as Added, Changed, Fixed, and Security. State which checks ran.
Do not describe a planned feature as implemented.
See [MAINTAINERS.md](MAINTAINERS.md#changelog-and-release-review) for owner review,
dated release entries, synchronized versions, and explicit release approval.

## Packaging and future releases

The Release packaging workflow builds both binaries on fixed native runners:
Ubuntu 24.04 x86_64, Windows Server 2022 x86_64, and macOS 15 Apple Silicon.
These are test baselines, not verified minimum OS versions.
A relevant pull request runs the same package checks without creating a release.
A manual run uploads archives and SHA-256 checksums without creating a release.
Assets include the Rust target triple to identify architecture and platform.
The packages include both binaries, the README, license, changelog, and the
self-contained [installation guide](docs/installation.md).

On Windows, packaging CI installs the 2022 Build Tools product if it is missing;
the runner's Visual Studio Enterprise installation is not a substitute for the
repository-selected environment. Every packaging Cargo command uses
`scripts\cargo.ps1`. The developer-shell banner goes to the host stream so that
in-process callers can parse Cargo metadata from the success stream.

Before upload, `scripts\test-package.ps1` verifies each checksum and exact archive
contents, then extracts into a temporary path with spaces. It tests both packaged
binaries with a restricted child PATH, offline errors, stdio MCP discovery, and
registration against temporary synthetic client settings. It does not access
real client files or credentials. `scripts\test-package-failures.ps1` checks that
bad checksums, changed bytes, wrong versions/targets, missing files, empty files,
and unexpected archive members fail. These development scripts need PowerShell
7.4 or later. Product binaries do not need PowerShell.

To check an existing Windows package locally:

```powershell
pwsh -NoProfile -File .\scripts\test-package.ps1 `
    -Archive .\dist\beyond-gate-v0.1.0-x86_64-pc-windows-msvc.zip `
    -Checksum .\dist\beyond-gate-v0.1.0-x86_64-pc-windows-msvc.zip.sha256 `
    -Version 0.1.0 -Target x86_64-pc-windows-msvc
pwsh -NoProfile -File .\scripts\test-package-failures.ps1 `
    -Archive .\dist\beyond-gate-v0.1.0-x86_64-pc-windows-msvc.zip `
    -Version 0.1.0 -Target x86_64-pc-windows-msvc
```

Use the candidate's actual version and target. On Linux and macOS, use the
matching `.tar.gz` archive on its native CPU. The packaging log records shared
library dependencies with `ldd`, `otool`, or `dumpbin`. Restricted-PATH checks are
not clean-desktop installation, real credential-store tests, or download trust
checks.

An explicit maintainer push of a `vX.Y.Z` tag also prepares a **draft** GitHub
release after all build jobs pass. The tag must match the workspace package version.
The draft needs review before publication. Never publish from a pull request.
No crates are published to crates.io. No release is part of this foundation change.

Before a real release, resolve the [release blockers](docs/installation.md#release-readiness)
in the release-usability roadmap issue. Offline package checks do not prove
working login, live CLI/MCP parity, real client discovery, or protected-store
behavior. Those checks remain blocked or need separate approval.
Change the shared version in `Cargo.toml` and lockfile together. Add a dated
changelog entry before tagging. A maintainer must review the version, lockfile,
changelog, assets, evidence, and draft before publication.
