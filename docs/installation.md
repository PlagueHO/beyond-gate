# Native installation and first-run checks

**Status: offline foundation, not a usable v1 D&D Beyond client.**
There is no published release yet. The packaging workflow produces test archives.
Login and live reads are not implemented. Local MCP serving and registration
work, but discovery does not establish access to account data.

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

Keep both binaries together in one stable directory. MCP registration records
the absolute server path. If you move the files later, remove the old registration
and register the new path. Do not copy only the CLI.

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

Both locally checked Windows binaries import `VCRUNTIME140.dll` and Windows
Universal CRT libraries. Install the supported x64 Visual C++ runtime if it is
not already available. Do not install Visual Studio to run a package.
If Windows reports a missing `VCRUNTIME` or `MSVCP` DLL, use Microsoft's
[supported x64 Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170).
Its version must support the build tools used for that package. Do not download
individual DLLs from other sites. Package CI logs record native dependencies;
execution on a developer machine does not prove a clean Windows desktop install.

## System services and trust limits

**Linux:** These packages use the system glibc runtime, not musl. Packaging logs
record `ldd` output for both binaries. A build on Ubuntu 24.04 is not proof of
compatibility with an older glibc. A missing loader or shared library needs a
compatible OS/runtime, not a Python or Node.js installation.

Local `auth status` and `auth logout` also require a user session D-Bus service
and a Secret Service provider with an unlocked default collection. Examples are
GNOME Keyring and a compatible KDE Wallet. Headless systems and WSL may not have
these services. There is no plaintext or environment-variable credential fallback.
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

After PATH setup, these checks are safe without an account:

```sh
beyond-gate auth login
beyond-gate character list --format json
beyond-gate book read synthetic opening
```

Each currently exits with code 1 and leaves stdout empty. Login reports the
missing working session handoff. Reads report unsupported live operations.
These are implementation limits, not verified missing-session or expiry errors.
Do not supply a password, cookie, or token. Do not create a credential manually.
Do not run `auth status` or `auth logout` as an automated package check:
they access the real OS store.

Starting `beyond-gate-mcp` with no arguments starts a stdio server. It waits for
MCP messages, not terminal commands. It is not a login prompt. Use an MCP client
for normal operation. `beyond-gate mcp start` starts the sibling server with the
same transport.

## MCP registration

The supported clients are VS Code, Claude Code, and GitHub Copilot CLI.
Their commands (`code`, `claude`, or `copilot`) must be available on PATH.
Client applications have their own requirements; they are not a runtime
dependency of the Beyond Gate binaries.

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
`claude mcp list` in Claude Code. The server exposes seven read-only tools:
`character_list`, `character_show`, `campaign_list`, `campaign_show`, `book_list`,
`book_outline`, and `book_read`. Valid reads currently return explicit tool
errors. Do not interpret discovery as successful login or live read access.

To remove registration, stop the server in the client. Remove only the
`beyond-gate` entry from the settings file's `mcpServers` object. Keep unrelated
entries. Remove both installed binaries and their documentation if you no longer
need them. Remove any PATH line you added. Unregistering or deleting the binaries
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
| Login blocker or unsupported read | This is the current product limit. Runtime installation will not resolve it. |

## Release readiness

An offline package check is not a v1 acceptance decision.

| Requirement | Evidence or blocker |
|-------------|---------------------|
| Checksums, exact archive contents, extraction, help/version, and executable paths | Automated on each selected native packaging runner. Results apply only after that workflow succeeds for the candidate commit. |
| Runtime-independent native startup | Checked with a restricted child PATH. Clean desktop installation and OS trust prompts remain untested. |
| Local MCP startup and registration | Protocol checks and temporary synthetic settings. Actual client discovery remains a manual release blocker. |
| CLI/MCP errors and output contracts, source attribution, book bounds | Existing offline suites use synthetic records. Live parity and source contracts remain blocked. |
| Missing, expired, or rejected upstream session errors | Blocked by missing session-aware live operations. Unsupported errors are not substitutes. |
| Simple browser login and native credential-store operation | Blocked or untested. Require separate explicit approval for real credential access. |
| Live login, reads, and client walkthrough | Not tested by packaging. Require explicit approval and a permitted access path. Stop on access denial. |
| Version, lockfile, dated changelog, assets, evidence, and draft release | Require maintainer review before publication. No tag or publication is authorized by issue #10 alone. |

Manual packaging uploads artifacts only. A maintainer-approved version tag can
prepare a draft after all package jobs pass. Do not publish the draft while
these release blockers remain. There is no crate publishing, cloud hosting,
auto-updater, or required cloud service.
