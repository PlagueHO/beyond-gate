# Native installation and first-run checks

**Status: Windows login, Web-library discovery, and bounded book reads.**
There is no published release yet. The packaging workflow produces test archives.
Windows login and native-host setup are implemented.
Approved real Chrome and Edge Dev login/save/restart/logout checks passed.
Approved native CLI/MCP Web-library checks passed in the development checkout.
They do not verify a packaged release or authorize later content reads.
Local MCP serving and registration work; tool discovery alone does not establish
access to account data.

## Package matrix

Select the archive for your operating system and CPU. Replace `VERSION` with
the version of the artifact you are testing.

| OS test baseline | CPU | Rust target | Archive |
|------------------|-----|-------------|---------|
| Ubuntu 24.04 | x86_64 | `x86_64-unknown-linux-gnu` | `beyond-gate-vVERSION-x86_64-unknown-linux-gnu.tar.gz` |
| Windows Server 2022 | x86_64 | `x86_64-pc-windows-msvc` | `beyond-gate-vVERSION-x86_64-pc-windows-msvc.zip` |
| macOS 15 | Apple Silicon (ARM64) | `aarch64-apple-darwin` | `beyond-gate-vVERSION-aarch64-apple-darwin.tar.gz` |

These are the fixed native CI runners, not a verified minimum OS version.
The [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
documents their architectures. Other Linux distributions, older OS versions,
Windows desktop versions, WSL, and headless sessions need separate validation.
There are no Intel macOS, Linux ARM64, musl, or Windows ARM64 packages.
Do not assume emulation is tested.

Each archive contains exactly these files at its root:

- `beyond-gate` and `beyond-gate-mcp` (with `.exe` on Windows).
- `README.md`, `LICENSE`, `CHANGELOG.md`, and `installation.md` (this guide).

Windows archives also contain `beyond-gate-native-host.exe` and
`beyond-gate-browser.zip`. The browser ZIP contains only `manifest.json`,
`launch.mjs`, `flow.mjs`, `login.html`, `login.css`, and `login.mjs`.
Development fixtures and tests are not included.

Keep both binaries together in one stable directory. MCP registration records
the absolute server path. If you move the files later, remove the old registration
and register the new path. Do not copy only the CLI.

For Windows browser setup, keep the native host beside the CLI in the stable
installation directory. Extract `beyond-gate-browser.zip` into a separate
`beyond-gate-browser-extension` subdirectory. Load that unpacked directory from
`chrome://extensions` or `edge://extensions` with Developer mode enabled.
Check that its ID is `gnhdconfgjfdbbgkemadegjcjnnodefh`.
Do not continue if the ID differs. No browser-store package is published.

Use `auth install edge --directory PATH --dry-run` or
`auth install chrome --directory PATH --dry-run` to preview native-host setup.
Remove `--dry-run` to request confirmation.
Use the same directory with `auth uninstall` to preview or remove matching
per-user setup. The command preserves binaries, other browsers, and machine-wide
entries. Edge can still find a Chrome fallback after explicit Edge removal.
See the [Windows setup commands](cli.md#auth-install-and-auth-uninstall) for the
trust boundary and conflict rules. Do not register a temporary source worktree
or build directory. These instructions do not establish completed live login.

## Verify the download

Download the archive and its matching `.sha256` file from the same trusted
workflow artifact or a future maintainer-reviewed release. An Actions artifact
can have an outer ZIP container; extract that container to obtain the native
archive and sidecar first.

Check SHA-256 before extracting the native archive. A checksum detects damaged
or changed bytes. It is not a digital signature or proof of the publisher.
If the check fails, stop and obtain a fresh copy from a trusted source.

On Linux, in the download directory:

```sh
sha256sum --check beyond-gate-vVERSION-x86_64-unknown-linux-gnu.tar.gz.sha256
```

On macOS:

```sh
shasum -a 256 --check beyond-gate-vVERSION-aarch64-apple-darwin.tar.gz.sha256
```

On Windows, use PowerShell in the download directory:

```powershell
$archive = 'beyond-gate-vVERSION-x86_64-pc-windows-msvc.zip'
$line = [System.IO.File]::ReadAllText("$PWD\$archive.sha256")
$match = [regex]::Match($line, '\A([0-9a-f]{64})  ([^\r\n]+)\r?\n?\z')
if (!$match.Success -or $match.Groups[2].Value -cne $archive) {
    throw 'Invalid checksum file or archive name.'
}
$actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -cne $match.Groups[1].Value) { throw 'SHA-256 mismatch. Do not extract.' }
```

## Extract and run

The product binaries do not require Rust, Python, Node.js, PowerShell, or Visual
Studio Build Tools. The commands below use the OS shell to extract and start
them. Contributor builds have separate tool requirements.

### Linux and macOS

Use an empty user-local directory. Select the matching archive from the matrix.
This example uses Linux; change the archive name for macOS:

```sh
archive='beyond-gate-vVERSION-x86_64-unknown-linux-gnu.tar.gz'
destination="$HOME/.local/opt/beyond-gate"
mkdir -p "$destination"
tar -xzf "$archive" -C "$destination"
"$destination/beyond-gate" --help
"$destination/beyond-gate" --version
"$destination/beyond-gate-mcp" --help
"$destination/beyond-gate-mcp" --version
```

Both binaries must retain their executable permission. If a copy tool removes
it, restore permission on these two verified files only:

```sh
chmod u+x "$destination/beyond-gate" "$destination/beyond-gate-mcp"
```

Optionally add the directory to your shell's `PATH`:

```sh
export PATH="$HOME/.local/opt/beyond-gate:$PATH"
```

To keep this setting, add that line to your normal shell configuration.
No administrator installation or `sudo` is needed.

### Windows

The Windows x64 binaries require the Microsoft Visual C++ v14 x64
Redistributable. They import `VCRUNTIME140.dll` and Windows Universal CRT
libraries. The archive does not include or install this system dependency.
Install the latest supported x64 runtime before the first run. Its version must
be at least as recent as the MSVC tools used to build the package.

If WinGet is available, install the Microsoft package:

```powershell
winget install --id Microsoft.VCRedist.2015+.x64 --exact --source winget --interactive
```

Otherwise, use Microsoft's
[supported x64 Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170).
Do not install Visual Studio to run the package. Do not download individual DLLs
from other sites.

Use PowerShell and an empty directory under your user profile:

```powershell
$archive = 'beyond-gate-vVERSION-x86_64-pc-windows-msvc.zip'
$destination = Join-Path $env:LOCALAPPDATA 'Programs\Beyond Gate'
Expand-Archive -LiteralPath $archive -DestinationPath $destination
& (Join-Path $destination 'beyond-gate.exe') --help
& (Join-Path $destination 'beyond-gate.exe') --version
& (Join-Path $destination 'beyond-gate-mcp.exe') --help
& (Join-Path $destination 'beyond-gate-mcp.exe') --version
```

The call operator `&` supports executable paths that contain spaces.
Optionally add this directory to your **user** Path in Windows Environment
Variables. Start a new terminal after you change it. For the current terminal:

```powershell
$env:PATH = "$destination;$env:PATH"
```

If Windows reports a missing `VCRUNTIME` or `MSVCP` DLL, install the runtime
above and retry. Package CI logs record native dependencies, but the CI runner
already has its system libraries. A passing package check on that runner does
not prove that a new Windows desktop has the required runtime.

## System services and trust limits

**Linux:** These packages use the system glibc runtime, not musl. Packaging logs
record `ldd` output for both binaries. A build on Ubuntu 24.04 is not proof of
compatibility with an older glibc. A missing loader or shared library needs a
compatible OS/runtime, not a Python or Node.js installation.
The retained `0.1.0` package also passed startup and shell lifecycle checks on
Ubuntu 24.04.5 x64 with glibc `2.39` and `libgcc-s1`
`14.2.0-4ubuntu2~24.04.1`. These are tested combinations, not minimum versions.
The image included system Python 3. No extra development runtime was installed.
See the [Linux package-only evidence](desktop-acceptance.md#azure-linux-package-only-follow-up)
for the inventory, PATH, removal, and desktop limits.

Local `auth status` and `auth logout` also require a user session D-Bus service
and a Secret Service provider with an unlocked default collection. Examples are
GNOME Keyring and a compatible KDE Wallet. Headless systems and WSL may not have
these services. There is no plaintext fallback. Environment input is not
implemented yet; the [agreed #52 authentication follow-up](authentication.md#agreed-authentication-follow-up)
will allow process-only CLI/MCP use without these services or a browser.
Package checks do not unlock or access a real credential store.

**macOS:** Local session commands use the user's login Keychain. Unlock it through
normal OS controls if needed. The workflow does not use a Developer ID signing
identity or notarize packages. A linker-generated ad hoc signature is not
Developer ID signing. Local CI execution does not test a quarantined browser
download or establish Gatekeeper trust. Apple explains the difference in its
[notarization guidance](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).
If macOS blocks a download, stop and review the source and OS warning. Do not
disable Gatekeeper or remove quarantine as an install step.

**Windows:** Local session commands use Windows Credential Manager under the
current Windows account. SmartScreen or organization policy can block an
untrusted executable. No Windows signing process is configured. Do not disable
those protections to make a smoke check pass.

Native-store access can cause an OS prompt. See the repository's authentication
design for backend details. A locked store or a service error is not an absent
session. Real store checks need separate approval.

## First-run results

The help and version commands above must exit with code 0 and no stderr output.
The CLI with no arguments shows usage on stderr and exits with code 2.
Unknown arguments also exit with code 2.

After PATH setup, use help commands for a smoke check without an account:

```sh
beyond-gate character list --help
beyond-gate campaign show --help
```

These commands do not access the protected store or contact D&D Beyond.
Do not run a valid character, campaign, or book read as a package smoke check.
Those operations use the real OS store and can contact D&D Beyond.
Do not supply a password, cookie, or token. Do not create a credential manually.
Do not run `auth status` or `auth logout` as an automated package check:
they access the real OS store.
On Windows, valid character, campaign, and book reads through either adapter use the real
OS store and can contact D&D Beyond. Package checks reject unknown CLI options
and MCP input fields before discovery instead. Synthetic process tests check
successful discovery and session failures without a real account.
Do not run interactive `auth login` as an account-free check. After human pairing
and consent, it reads the selected browser cookie, validates it, and replaces the
protected local entry. Separate explicit approval is required for real account
or credential-store tests. Redirected package checks verify refusal before those
operations.

Starting `beyond-gate-mcp` with no arguments starts a stdio server. It waits for
MCP messages, not terminal commands. It is not a login prompt. Use an MCP client
for normal operation. `beyond-gate mcp start` starts the sibling server with the
same transport.

## MCP registration

The supported clients are VS Code, Claude Code, and GitHub Copilot CLI.
Their commands (`code`, `claude`, or `copilot`) must be available on PATH.
Client applications have their own requirements; they are not a runtime
dependency of the Beyond Gate binaries.
The VS Code installer checks only `code`. An Insiders-only installation with
`code-insiders` does not meet that prerequisite. This does not establish whether
manual MCP registration works in Insiders.

Preview the selected registration first:

```sh
beyond-gate mcp install vscode --dry-run
beyond-gate mcp install claude-code --dry-run
beyond-gate mcp install copilot-cli --dry-run
```

Choose one installed client. Run its command again without `--dry-run`, review
the absolute server path, and confirm the write. A repeated identical install
does not change settings. A conflicting `beyond-gate` entry is not overwritten.
Back up existing client settings before manual edits.

VS Code uses Copilot Global settings. VS Code and Copilot CLI share
`$COPILOT_HOME/mcp-config.json`, or `~/.copilot/mcp-config.json` when
`COPILOT_HOME` is not set. Claude Code uses `~/.claude.json`.
No entry contains a secret, remote URL, Python command, or Node.js command.

Check the MCP server list in VS Code, use `/mcp` in Copilot CLI, or run
`claude mcp list` in Claude Code. The server exposes thirteen resource read tools:
`character_list`, `character_show`, `campaign_list`, `campaign_show`, `book_list`,
`book_outline`, `book_read`, `class_list`, `class_show`, `monster_list`, and
`monster_show`, `background_list`, `background_show`, `feat_list`, and `feat_show`.
Feat list reads bounded website metadata. Feat show selects one checked raw
definition from a fresh bounded protected collection.
Background tools share website metadata filters and selected raw definitions.
The observed live definition collection exceeds the unchanged 1 MiB transport
cap, so show fails explicitly for it. Native background acceptance has not run.
Class tools return bounded class/subclass metadata, not raw definitions or rules
text. Native class compatibility has not been validated.
Monster lists support website filters and explicit page-based continuation,
with a fresh selected-ID access check before output. Monster tools permit
only observed access code 1; code 4 is denied, and
unverified codes, including 2 and 3, are unsupported. Native monster account
acceptance has not run.
Windows book reads use fresh library evidence
and checked canonical cookie pages. Character and campaign reads use fixed
bounded routes through the protected session. The additional `auth_validate`
tool accepts no credential input, but loads the protected Windows entry and
makes one fresh service check. It is not an account-free smoke check and does
not grant content access.
Do not interpret discovery as successful login or live read access.
Linux production login and session use are tracked in
[#52](https://github.com/PlagueHO/beyond-gate/issues/52).

To remove registration, stop the server in the client. Remove only the
`beyond-gate` entry from the settings file's `mcpServers` object. Keep unrelated
entries. For browser login removal, also use the matching `auth uninstall`
command before removing the native host and extension files. Remove the installed
binaries and their documentation if you no longer need them. Remove any PATH
line you added. Unregistering or deleting the binaries
does not revoke an upstream session or remove a stored credential.

## Troubleshooting

| Condition | Action |
|-----------|--------|
| Wrong CPU or executable format | Select the exact OS/CPU archive. Do not assume emulation support. |
| Checksum mismatch or missing package file | Stop. Obtain the complete archive and matching checksum again. |
| Permission denied on Unix | Check executable permissions on the verified binaries. Respect OS trust controls. |
| Missing loader, shared library, or Windows runtime DLL | Check the package baseline and native-dependency logs. Use the OS vendor's runtime installer where applicable. |
| Missing sibling MCP executable | Put both binaries from the same package in the same directory. |
| Client not found on PATH | Install the selected client and enable its documented command-line launcher. |
| Malformed client JSON or conflicting server name | Repair or remove only the conflicting entry after a backup. The installer will not overwrite it. |
| MCP waits without output | It expects MCP JSON-RPC on stdin. Start it through a client, not as a terminal REPL. |
| Locked or unavailable credential service | Use the desktop's normal unlock/service controls. Do not use a file fallback. |
| Windows login setup fails | Check the pinned extension ID, matching native-host registration, and three interactive terminal streams. Use a new login key after failure. |
| Other-platform login or unsupported read | This is the current product limit. Runtime installation will not resolve it. |

## Release readiness

An offline package check is not a v1 acceptance decision.
See the [desktop acceptance matrix](desktop-acceptance.md#remaining-acceptance-matrix)
for candidate-specific passes and remaining gaps. That guide also defines
the procedures and required evidence.
Native stores, real clients, browser setup, and account reads have separate
approval gates. Package evidence never grants those approvals.
Implement shared authentication and Linux terminal use in
[#52](https://github.com/PlagueHO/beyond-gate/issues/52) first. Then
[#34](https://github.com/PlagueHO/beyond-gate/issues/34) checks Windows/Linux
CLI and real MCP clients. Environment input and a hidden CLI prompt remain
planned, not usable commands. Optional browser/protected-save checks and deeper
hardening remain separate; no live test is authorized by the backlog revision.
macOS desktop and Keychain checks and Claude Code client checks are deferred to
[#62](https://github.com/PlagueHO/beyond-gate/issues/62).
Candidate-release decisions remain in
[#41](https://github.com/PlagueHO/beyond-gate/issues/41).

| Requirement | Evidence or blocker |
|-------------|---------------------|
| Checksums, exact archive contents, extraction, help/version, and executable paths | Automated on each selected native packaging runner. Results apply only after that workflow succeeds for the candidate commit. |
| Native startup and Windows runtime prerequisite | Package checks use a restricted child PATH and process-local PATH lookup. The Windows CI runner still provides system runtime libraries. `dumpbin /DEPENDENTS` records native imports, but does not prove a clean Windows desktop has the required Visual C++ Redistributable. The user must install it before first run. Clean-desktop, two-version upgrade, and OS trust checks remain manual acceptance cases. |
| Local MCP startup and registration | Protocol checks and temporary synthetic settings. Retained Windows VS Code and Copilot CLI discovery results have candidate-specific evidence; remaining lifecycle and isolation gaps stay open. |
| CLI/MCP errors and output contracts, source attribution, book bounds | Native Web-library parity and one selected chapter's outline/initial-chunk parity passed separately from packaging. Synthetic source checks cover continuation and failures. Live continuation and sharing remain unchecked. |
| Missing, expired, or rejected upstream session errors | Shared-session library and book source operations have synthetic failure checks. Live evidence is separate. |
| Simple browser login and native credential-store operation | Approved Windows Chrome and Edge Dev product checks passed separately from packaging. Other-platform login remains deferred. New real-store tests need explicit approval. |
| Live reads and client walkthrough | Native Web-library acceptance and one retained-candidate VS Code `book_list` call passed separately from packaging. Other real-client reads, model-driven routing, and sharing remain unverified. New account checks need explicit approval. Stop on access denial. |
| Version, lockfile, dated changelog, assets, evidence, and draft release | Require maintainer review before publication. No tag or publication is authorized by issue #10 alone. |

Manual packaging uploads artifacts only. A maintainer-approved version tag can
prepare a draft after all package jobs pass. Do not publish the draft while
these release blockers remain. There is no crate publishing, cloud hosting,
auto-updater, or required cloud service.
