# Contributing

Start with the [roadmap](docs/roadmap.md) and
[Support and compatibility](docs/support.md). The project is pre-1.0.
Implemented reads and retained native acceptance have different limits.
Use [Privacy](docs/privacy.md) when reviewing authentication, storage, and clients.

## Issues

For a suspected vulnerability, follow the [security policy](SECURITY.md).
Do not post vulnerability details in a public issue or pull request.
For ordinary failures, follow the [safe reporting checklist](docs/support.md#report-a-problem).

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

Run **Workspace: Validate** after setup. Linux container checks do not prove
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

[`.github/github-app.yml`](.github/github-app.yml) defines the manual tasks in
the [developer task index](docs/development.md#developer-task-index).
The names match the VS Code task names and start with the affected subject.
Each Cargo task uses the wrapper so Windows setup and Cargo run in the same
process. Select the named task from the app's task menu.
**CLI: Run** builds both binaries and starts the CLI.
With no subcommand, the CLI shows its overview on stderr and exits with code 2.
Use **CLI: Help** for help with code 0, or the native commands below for subcommands.
**MCP: Serve stdio** builds both binaries and starts the protocol process.
Build messages use stderr. Product streams and failure status remain separate.
No task runs automatically. No task performs login or a live account test.
Do not type MCP messages into a task terminal. Use an MCP client for tool calls.
If the app task terminal cannot supply the required interactive streams, use a
normal terminal with the [native run commands](docs/development.md#native-build-and-run).
Do not redirect hidden input as a workaround.

Review and accept this repository configuration in the app before using the
tasks. See GitHub's [repository configuration reference](https://docs.github.com/copilot/reference/github-copilot-app-reference/repository-configuration).
These tasks do not change GitHub Actions runner configuration.

### VS Code tasks

Accept the [recommended extensions](.vscode/extensions.json).
Select **Terminal: Run Task** from the Command Palette, then choose a task.
**Rust: Build debug** and **Rust: Test workspace** are the default build and test tasks.
**Workspace: Validate** runs all ordinary offline checks in sequence, including
the release build, process tests, native launch checks, Markdown, and offline links.
It also tests the documentation checker's failure cases.
**Docs: Validate site** builds the website and runs its browser checks.
Install the documentation development dependencies before selecting that task.
**Workspace: Validate** does not require Node.js or Docker.
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

Run tasks use the shared build-before-run helper.
To start stdio MCP manually, use:

```powershell
pwsh -NoProfile -File .\scripts\run.ps1 mcp
```

The process waits for MCP protocol input. It has no HTTP address or health port.
Do not type ordinary text into the protocol stream. Press **Ctrl+C** to stop
manual execution, or stop the process from its terminal. Normal MCP clients
start and stop their own child process. Do not configure this as an unattended
background task.

Use the [local workflow guide](docs/development.md#native-build-and-run) for CLI
arguments, client configuration, restart, and
[Docker runtime commands](docs/development.md#local-runtime-container).
The runtime image is separate from the development container.

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

### Documentation website

The VitePress site uses the locked pnpm package in `docs`.
See [documentation development](docs/development.md#documentation-site) for Node.js,
pnpm, browser prerequisites, and dev/build/preview commands.
The named documentation tasks in VS Code and Copilot App use these commands.
Development and preview servers are manual tasks. They are not aggregate checks.
Node.js and pnpm are not requirements for the native binaries or Rust-only checks.
Documentation CI builds and checks the site. It does not publish it.

### Dependency maintenance

[Dependabot](.github/dependabot.yml) follows `dependabot-monthly-v1` for the root
Cargo workspace, the `/docs` pnpm package, and GitHub Actions.
Routine checks run on the first day of each
month at 09:00 `Pacific/Auckland`. Routine releases have a three-day cooldown.
Each ecosystem allows five open routine update PRs.

Minor and patch updates are grouped by ecosystem. Major updates remain separate.
Security updates are event-driven and have separate security groups. They do not
wait for the monthly schedule or routine cooldown. This depends on Dependabot
security updates being enabled in repository settings; the configuration does
not change that setting. See the [Dependabot options reference](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference).

For a routine update outside the schedule, select **Check for updates** in the
repository's Dependabot interface. The routine cooldown and PR limit still
apply. Review release notes, compatibility, manifest changes, `Cargo.lock`, and
`docs/pnpm-lock.yaml`. The docs package has a narrow Vite override for stable
VitePress. Check whether each VitePress update can remove it.
Keep action references pinned to full commit SHAs. Run the relevant workspace,
process, documentation, and packaging checks. No automatic approval or merging
is configured. No dependency labels are added.

Dependabot does not cover the Rust toolchain, container image and features,
native packages, PowerShell, editor extensions, or the pinned checker installers.
Review these tools manually. See [maintainer responsibilities](MAINTAINERS.md).

### Rust dependency advisory and license checks

The development-only checker is pinned to cargo-deny 0.22.0. Its installer uses
the tool's published lockfile and writes to `target/dev-tools`.
It does not add a product runtime. Install it explicitly, then run the checks:

```powershell
pwsh -NoProfile -File .\scripts\install-dependency-tools.ps1
pwsh -NoProfile -File .\scripts\check-dependencies.ps1
pwsh -NoProfile -File .\scripts\check-dependencies.ps1 -Offline
pwsh -NoProfile -File .\scripts\test-dependency-checks.ps1
```

The [CI workflow](.github/workflows/dependency-checks.yml) uses these same scripts
on pull requests, default-branch pushes, manual runs, and a weekly schedule.
The [shared policy](deny.toml) checks every workspace member and all features.
It includes build and test dependencies and does not filter by host or target.
Unpublished workspace packages are not exempt. These are graph checks, not
proof that each target builds or that every feature combination works.

The checker uses `--locked` and does not change `Cargo.lock`.
It rejects vulnerability, unsoundness, and unmaintained advisories throughout
the graph unless an explicitly approved, unexpired exception applies.
Yanked versions produce warnings. Licenses must satisfy the explicit
allow list; unknown, missing, and unacceptable licenses fail.
Missing or wrong checker versions fail without an implicit installation.
All check failures return nonzero exit codes.

Normal mode fetches the latest
[RustSec database](https://github.com/RustSec/advisory-db) through Git.
Cargo can also fetch public crate sources and registry data. This requires
network access to GitHub and the configured Cargo registries.
The commands do not upload source code or read product account credentials.
They use ordinary developer Cargo and Git configuration for dependency access.
Do not supply D&D Beyond credentials to these tools.
Network or database errors fail the check. They do not trigger an offline fallback.

Offline mode prevents dependency and advisory downloads. First run normal mode
successfully to populate the caches. Offline mode requires all selected crate
sources and registry data and an advisory database fetched within seven days.
The database normally lives under Cargo's `advisory-dbs` cache.
The checker uses the fetch timestamp, with a Git HEAD timestamp fallback.
Missing or stale database data fails. An offline pass cannot detect advisories
published after the cached fetch. Normal mode always refreshes the database.
See the [advisory reference](https://embarkstudios.github.io/cargo-deny/checks/advisories/cfg.html).

The controlled tests use an invented advisory for a cached dependency in an
isolated path with spaces. Fetch workspace dependencies first with
`pwsh -NoProfile -File .\scripts\cargo.ps1 fetch --locked`.
The tests make no network requests. They check advisory and license rejection,
test-only and optional foreign-target dependencies, expired exceptions, both
sides of the seven-day limit, missing database data, wrong or missing tools,
failure propagation, and lockfile preservation.

CodeQL remains the source-code scanner. Dependabot remains enabled for
dependency alerts and updates. Cargo-deny adds a local and CI policy for the
complete Rust graph, with some advisory overlap with Dependabot.
Secret scanning and push protection remain separate; neither checks licenses.
The checker does not inspect the documentation site's pnpm dependencies,
GitHub Actions implementations, native system packages, or its own separately
installed dependencies as part of this workspace policy.

Four unmaintained-package advisories have approved temporary exceptions through
2026-12-20. Compatible updates did not remove these dependencies.
Each exception names its advisory, reason, and tracking issue:

| Dependency | Advisory | Tracking issue |
| --- | --- | --- |
| zvariant 5.15.1 | RUSTSEC-2026-0370 | [#109](https://github.com/PlagueHO/beyond-gate/issues/109) |
| zbus_names 4.3.5 | RUSTSEC-2026-0371 | [#112](https://github.com/PlagueHO/beyond-gate/issues/112) |
| zvariant_derive 5.15.1 | RUSTSEC-2026-0372 | [#110](https://github.com/PlagueHO/beyond-gate/issues/110) |
| chrono 0.4.45 | RUSTSEC-2026-0543 | [#111](https://github.com/PlagueHO/beyond-gate/issues/111) |

These are maintenance advisories, not vulnerability exceptions.
The Linux keyring chain still requires zbus 5. The MCP SDK still requires chrono.
The check resumes rejection after expiry. Remove exceptions when the affected
dependencies leave the graph; unused advisory exceptions also fail the check.
Do not renew them without maintainer approval.
See [#38](https://github.com/PlagueHO/beyond-gate/issues/38).

Review checker releases manually. Update the version in
`scripts/install-dependency-tools.ps1`, review configuration changes against
that version, and run the real-graph and controlled checks.
Do not weaken the policy to make an update pass.
Any exception needs maintainer approval, the exact advisory ID or crate/version,
a reason, a tracking issue, and an enforced expiry.
Advisory exceptions must use cargo-deny's explicit `expiry` field.
A proposed license exception must add an enforced expiry check before use;
the current policy has no license exceptions.

### Dependency license review

The allow list includes MPL-2.0 by maintainer decision. It does not change the
MIT license of Beyond Gate's own code. Dependencies keep their own licenses.
A checker pass is not proof of legal compliance or a replacement for legal advice.
See the [license-check reference](https://embarkstudios.github.io/cargo-deny/checks/licenses/cfg.html).

Before binary distribution, review the selected dependency license expressions
and their notice, attribution, and source-availability duties.
Preserve required license and copyright notices. For MPL-2.0 code, tell
recipients where they can obtain the covered source, including any changes,
under MPL-2.0. Its file-level copyleft does not by itself relicense separate
Beyond Gate source files. See Mozilla's
[MPL-2.0 FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/), especially questions
8, 11, and 12. Do not claim that the allow list or a binary checksum completes
these distribution duties.

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

The [session-exchange probe](docs/authentication-design.md#opt-in-session-exchange-probe)
is an ignored developer test, not product login. Its ordinary tests use synthetic
data and local mocks. Run its live case only from an unrecorded interactive
terminal with separate explicit test approval. Never pass a cookie through agent tools,
command arguments or files. That probe does not use environment input.
Product environment input uses only `BEYOND_GATE_COBALT_SESSION`; supply it locally,
never through agent tools, chat, shell profiles, or literal client settings.
Process-only tests must use invented values and counting/failing stores.
The synthetic `hidden_terminal` process test uses a development-only PTY library
to check no echo and terminal restoration. It does not use real credentials.

### Manual capability suite

The [manual testing guide](docs/testing.md) describes the shared CLI/MCP process
suite, isolated native-store tests, and `workflow_dispatch` workflow on
`windows-latest`. Use `scripts\start-live-e2e.ps1 -Selection contracts` for the
credential-free subset. Full coverage currently fails with blocked live reads.
The helper does not request or upload a cookie while production prerequisites
are absent. Native credential access still needs separate explicit approval.

The test-only feature is an exception for isolated test identities, not a product
credential-import interface. Normal login stays human-controlled and persistent
secrets stay in protected OS storage. Do not enable native or live cases in
ordinary workspace checks.

## Pull requests

Keep each change small and coherent. Use the PR template. Update tests, affected
documentation, and `CHANGELOG.md` under `Unreleased`. Use Keep a Changelog
categories such as Added, Changed, Fixed, and Security. State which checks ran.
Do not describe a planned feature as implemented.
See [repository protections](docs/repository-protections.md#required-checks)
for the exact required checks, approval record, and applied configuration.
The default-branch rule requires a PR and up-to-date passing checks,
not an approving review. It does not authorize an automatic merge.
See [MAINTAINERS.md](MAINTAINERS.md#changelog-and-release-review) for owner review,
dated release entries, synchronized versions, and explicit release approval.

## Packaging and future releases

The Release packaging workflow builds both binaries on fixed native runners:
Ubuntu 24.04 x86_64, Windows Server 2022 x86_64, and macOS 15 Apple Silicon.
These are test baselines, not verified minimum OS versions.
A relevant pull request runs the same package checks without creating a release.
A manual run uploads archives and SHA-256 checksums without creating a release.
Only a manual run on the default branch or a version-tag push can also create
release provenance. PR and other-ref manual packages are unattested test outputs.
Assets include the Rust target triple to identify architecture and platform.
The packages include both binaries, the README, license, changelog, and the
self-contained [installation guide](docs/installation.md).

On Windows, packaging CI installs the 2022 Build Tools product if it is missing;
the runner's Visual Studio Enterprise installation is not a substitute for the
repository-selected environment. Every packaging Cargo command uses
`scripts\cargo.ps1`. The developer-shell banner goes to the host stream so that
in-process callers can parse Cargo metadata from the success stream.

The shared [Cargo configuration](.cargo/config.toml) sets
`-C target-feature=+crt-static` for `x86_64-pc-windows-msvc`. Local builds,
CI checks, and release builds use the same setting. Other targets are unchanged.
Do not override it with `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS`. Cargo environment
overrides can replace target flags. The extracted-package check remains the
final guard against a dynamic Visual C++ runtime dependency.

Static runtime security fixes require a new build and package release.
Contributor builds still need Visual Studio 2022 Build Tools and the Windows SDK.
Static linking does not remove optional browser setup or MCP registration.

Before upload, `scripts\test-package.ps1` verifies each checksum and exact archive
contents, then extracts into a temporary path with spaces. It tests both packaged
binaries with a restricted child PATH, offline errors, stdio MCP discovery, and
registration against temporary synthetic client settings. It does not access
real client files or credentials. `scripts\test-package-failures.ps1` checks that
bad checksums, changed bytes, wrong versions/targets, missing files, empty files,
and unexpected archive members fail. These development scripts need PowerShell
7.4 or later. Product binaries do not need PowerShell.

On Windows, the verifier uses Visual Studio 2022 `dumpbin /IMPORTS` on all three
extracted executables before startup. It rejects separate Visual C++ runtime
DLLs in normal and delay-loaded imports. It permits reviewed Windows system
DLLs and API sets, not arbitrary dependencies found on the development host.
New DLL imports require review. Missing tools and failed inspections fail the
check. The rejection suite compiles a synthetic `/MD` executable and requires
rejection in each of the three archive positions.

To check an existing Windows package locally:

```powershell
pwsh -NoProfile -File .\scripts\test-windows-runtime.ps1
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

For a clean Windows desktop, use the built-in Windows PowerShell
[portable startup check](docs/testing.md#windows-portable-runtime-checks).
Run it as a non-administrator without the redistributable. Do not copy
development tools or runtime DLLs into that guest. Keep this result separate
from checks run on a development or CI host.

An explicit maintainer push of a `vX.Y.Z` tag also prepares a **draft** GitHub
release after all build and provenance jobs pass. The tag must match the workspace
package version.
The draft needs review before publication. Never publish from a pull request.
No crates are published to crates.io. No release is part of this foundation change.
See [version-tag controls](docs/repository-protections.md#version-tags) and
[release permissions](docs/repository-protections.md#releases-and-permissions)
for the proposed and applied policy. Get separate approval before a tag or
publication. Review all assets in the draft before they become immutable.

### Release provenance checks

Follow the [code signing policy](MAINTAINERS.md#code-signing-policy) and
[download verification instructions](docs/installation.md#verify-build-provenance).
Build jobs keep read-only permissions. Only the separate provenance job gets
OIDC token and attestation write permissions. It checks the current run's exact
six-file inventory and checksums, then attests the final bytes with a pinned
`actions/attest`. Do not modify those files afterward.

The job verifies each asset and rejects changed archive bytes or an incorrect
source SHA, ref, or signer workflow. The draft job checks the downloaded assets
again before draft creation. Bundles and results stay in the separate
`release-provenance-evidence` Actions artifact, not the release assets.

Run the local synthetic checks without a GitHub token or network access:

```powershell
pwsh -NoProfile -File .\scripts\test-release-provenance.ps1
```

Packaging runs these tests on each native runner. They use a `gh` test double
to check policy arguments, failure propagation, invalid inputs, and asset
rejection. They do not verify real signatures. Service integration still needs
verification for each trusted candidate before a provenance claim.

Before a real release, resolve the [release blockers](docs/installation.md#release-readiness)
in the release-usability roadmap issue. Offline package checks do not prove
working login, live CLI/MCP parity, real client discovery, or protected-store
behavior. Those checks remain blocked or need separate approval.
Change the shared version in `Cargo.toml` and lockfile together. Add a dated
changelog entry before tagging. A maintainer must review the version, lockfile,
changelog, assets, evidence, and draft before publication.

#### Verified manual candidate

On 2026-11-17, approved manual
[run 44048471076, attempt 1](https://github.com/PlagueHO/beyond-gate/actions/runs/44048471076/attempts/1)
passed for `refs/heads/main` at commit
`59ec128e32ae071a350a299a690d9cc42c113a5c`, version `0.1.0`.
Native Rust and package checks passed on Linux x86_64, Windows x86_64, and
macOS Apple Silicon. The provenance job attested and verified all three final
archives and their three checksum sidecars.

The downloaded six-file inventory and checksums passed local checks. All six
files also passed the repository's provenance verifier against that exact source
commit, ref, and release workflow.
[Attestation 68531490](https://github.com/PlagueHO/beyond-gate/attestations/68531490)
records the six SHA-256 digests and GitHub-hosted workflow identity.
CI and local bundle checks rejected changed archive bytes, a wrong source commit,
a wrong ref, and a wrong signer workflow. Valid verification passed before and
after the negative checks.

The run retained package reports and `release-provenance-evidence` separately
from the candidate assets. Draft creation was skipped. No tag or release was
created. The version-tag draft path remains untested. This evidence establishes
candidate provenance, not native publisher signing, notarization, desktop trust,
live account acceptance, or reproducible builds. Later candidates need their own
verification.
