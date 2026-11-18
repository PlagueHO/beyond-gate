# Installation

<!-- Keep extended checks in the packaged guide without expanding the main steps. -->
<!-- rumdl-configure-file { "MD033": { "allowed-elements": ["details", "summary"] } } -->

Follow steps 1 to 4 to install Beyond Gate and check that it starts.
Step 5 is optional.
You do not need Rust, Python, Node.js, or Visual Studio to run Beyond Gate.

**There is no public release yet.** Use a maintainer-reviewed test package.
Installing the tools does not sign you in to D&D Beyond.

## 1. Download your package

Download the archive for your computer and its matching `.sha256` file from the
[packaging workflow](https://github.com/PlagueHO/beyond-gate/actions/workflows/release.yml).
Use the run and commit reviewed by the maintainer.

An Actions download can contain an outer ZIP. Open that ZIP to get the package
archive and `.sha256` file. Do not extract the package itself until step 2 is complete.

### Package matrix

Replace `VERSION` in this guide with the version in your downloaded filename.

| Computer | Package archive | CI test system |
|----------|-----------------|----------------|
| Windows x64 | `beyond-gate-vVERSION-x86_64-pc-windows-msvc.zip` | Windows Server 2022 |
| Linux x64 | `beyond-gate-vVERSION-x86_64-unknown-linux-gnu.tar.gz` | Ubuntu 24.04 |
| Mac with Apple Silicon | `beyond-gate-vVERSION-aarch64-apple-darwin.tar.gz` | macOS 15 |

These are CI test systems, not minimum OS versions. Other OS versions and
emulation need separate checks. There are no Intel Mac, Linux ARM64, musl,
or Windows ARM64 packages.

## 2. Verify the download

Open a terminal in the folder that contains your download.
Check the checksum with the command for your computer below.
Then complete the build provenance check.
**Do not extract or run the package if either check fails.**

<details>
<summary>Windows checksum command</summary>

Run this in PowerShell. Replace `VERSION` first.

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

No error means the checksum matches.

</details>

<details>
<summary>Linux and macOS checksum commands</summary>

On Linux:

```sh
sha256sum --check beyond-gate-vVERSION-x86_64-unknown-linux-gnu.tar.gz.sha256
```

On macOS:

```sh
shasum -a 256 --check beyond-gate-vVERSION-aarch64-apple-darwin.tar.gz.sha256
```

The command must report `OK`.

</details>

<details>
<summary>Build provenance check (required)</summary>

### Verify build provenance

A checksum detects changed bytes. It does not prove who built the package.
Build provenance links the files to the repository, workflow, and source commit.

Use a current [GitHub CLI](https://cli.github.com/manual/gh_attestation_verify).
This check needs network access. GitHub CLI is not needed to run Beyond Gate.

Ask the maintainer for the reviewed full 40-character commit SHA and source ref.
Do not take them from an unverified sidecar.
Replace `ARCHIVE` and `REVIEWED_COMMIT_SHA` below.
For a default-branch test package, run:

```sh
gh attestation verify ARCHIVE --repo PlagueHO/beyond-gate --signer-workflow PlagueHO/beyond-gate/.github/workflows/release.yml --source-digest REVIEWED_COMMIT_SHA --source-ref refs/heads/main --predicate-type https://slsa.dev/provenance/v1 --deny-self-hosted-runners
```

Run it again with `ARCHIVE.sha256`. Both commands must succeed.
The same commands work in PowerShell.
Use the actual default-branch name if it changes.
For a reviewed version-tag package, use `--source-ref refs/tags/vVERSION` instead.

PR packages, other-ref packages, and older test packages can lack attestations.
Do not treat them as verified distribution.
Stop if an attestation is missing, the identity differs, or the service returns
an error. Do not fall back to checksums alone.

Provenance does not prove software safety or reproducibility.
It does not replace publisher signing, notarization, or OS trust checks.
See the [code signing policy][signing-policy].

</details>

## 3. Extract and run

Follow only the instructions for your computer.
Use an empty, permanent folder. Keep all files from the package together.

### Windows

Current Windows x64 builds link the Visual C++ runtime statically. These packages
do not require Visual Studio, a separate Visual C++ Redistributable, or
administrator installation. Windows-supplied system libraries remain required.
The [static-runtime candidate passed clean-Windows startup checks](desktop-acceptance.md#rdp-clean-windows-static-runtime-result)
under a standard Windows 11 account without a separate redistributable.
This does not establish a minimum supported Windows version.

Earlier dynamic-runtime archives still require Microsoft's
[supported x64 Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170).
The new evidence applies to the identified static-runtime candidate, not those
earlier downloads. Do not download individual DLLs from other sites.
Portable packaging does not remove optional browser setup or MCP registration.

1. Open PowerShell.
2. In your download folder, extract the verified archive:

   ```powershell
   $archive = 'beyond-gate-vVERSION-x86_64-pc-windows-msvc.zip'
   $destination = Join-Path $env:LOCALAPPDATA 'Programs\Beyond Gate'
   Expand-Archive -LiteralPath $archive -DestinationPath $destination
   ```

3. Make the commands available in this PowerShell window:

   ```powershell
   $env:PATH = "$destination;$env:PATH"
   ```

To keep this setting, add the installation folder to your **user** Path in
Windows Environment Variables. Open a new terminal after you change it.

### Linux and macOS

1. Open a terminal in your download folder.
2. Set the archive name for your computer.

   Linux:

   ```sh
   archive='beyond-gate-vVERSION-x86_64-unknown-linux-gnu.tar.gz'
   ```

   macOS:

   ```sh
   archive='beyond-gate-vVERSION-aarch64-apple-darwin.tar.gz'
   ```

3. Extract the verified archive and make its commands available:

   ```sh
   destination="$HOME/.local/opt/beyond-gate"
   mkdir -p "$destination"
   tar -xzf "$archive" -C "$destination"
   export PATH="$destination:$PATH"
   ```

To keep this setting, add `export PATH="$HOME/.local/opt/beyond-gate:$PATH"`
to your normal shell configuration.
No administrator installation or `sudo` is needed.

## 4. Check that the tools start

Run these commands in the same terminal:

```sh
beyond-gate --help
beyond-gate --version
beyond-gate-mcp --help
beyond-gate-mcp --version
```

Each command must show help or a version and finish without an error.
These checks do not sign in, open the credential store, or contact D&D Beyond.
Do not use an account read or `auth login` as an installation check.

**Installation is complete.** Use the [CLI reference](cli.md) for terminal
commands, or continue below to connect an MCP client.
See [Authentication](authentication.md) to sign in or use a supplied session.

## 5. Connect an MCP client (optional)

Choose **one** client that is already installed.
Its command (`code`, `claude`, or `copilot`) must be available in your terminal.

1. Preview the setup for your client:

   | Client | Preview command |
   |--------|-----------------|
   | VS Code | `beyond-gate mcp install vscode --dry-run` |
   | Claude Code | `beyond-gate mcp install claude-code --dry-run` |
   | GitHub Copilot CLI | `beyond-gate mcp install copilot-cli --dry-run` |

2. Run your selected command again without `--dry-run`.
3. Check the server path and confirm the change.
4. Open your client's MCP server list and find `beyond-gate`.

The server list confirms registration, not account access.
Do not put credentials in client settings or MCP tool inputs.
See [client setup details](cli.md#mcp-install) and the [MCP reference](mcp.md)
for requirements, settings locations, and available tools.

## Troubleshooting

| Problem | What to do |
|---------|------------|
| Command not found | Repeat the PATH step in your current terminal. Check that both binaries are in the installation folder. |
| Windows reports a missing runtime DLL | Install the Microsoft x64 runtime from step 3. Do not install Visual Studio or download individual DLLs. |
| Wrong executable format or missing Linux loader | Check the package, CPU, and OS. These Linux packages require glibc, not musl. |
| Permission denied on Linux or macOS | Restore execute permission on the two verified binaries with `chmod u+x "$destination/beyond-gate" "$destination/beyond-gate-mcp"`. |
| Windows or macOS blocks the program | Stop and review the download source and OS warning. Do not disable SmartScreen, Gatekeeper, or quarantine. |
| MCP waits without output | Start it through your MCP client. It waits for protocol messages, not terminal commands. |
| Client not found or settings conflict | Check the client's command-line launcher. Back up settings before fixing a conflicting `beyond-gate` entry. The installer will not overwrite it. |
| Credential store is locked or unavailable | Use normal OS unlock controls. There is no plaintext fallback. Process-only authentication does not require the native store. |

## Additional setup details

Open these sections only when you need them.

<details>
<summary>Package contents, moving, and removal</summary>

Every archive contains the CLI and MCP binaries, `README.md`, `LICENSE`,
`CHANGELOG.md`, and this `installation.md` guide.
Windows also includes `beyond-gate-native-host.exe` and `beyond-gate-browser.zip`.
The browser ZIP includes the extension manifest, scripts, login page, styles,
logo, and icons. It does not include development fixtures or tests.

MCP setup records an absolute server path. If you move the installation, remove
its old registration and register the new path. Do not copy only the CLI.

To remove an installation, stop the server in the client. Remove only the
`beyond-gate` entry from the settings file's `mcpServers` object.
Keep unrelated entries. For browser setup, run the matching `auth uninstall`
command before deleting the native host and extension files.
The command preserves other browsers and machine-wide entries.
Edge can still find a Chrome fallback after explicit Edge removal.
Then delete the installation files and remove any PATH entry you added.

Removing the tools does not delete a saved credential or revoke an upstream
session. See [logout](cli.md#auth-logout) before removal if needed.

</details>

<details>
<summary>OS services and download trust</summary>

### System services and trust limits

**Windows:** The binaries require the Microsoft Visual C++ v14 x64
Redistributable and system runtime libraries.
Use the latest supported x64 runtime. Its version must be at least as recent
as the build tools used for the package. A passing CI check does not prove
that a clean Windows desktop has this runtime.
Saved sessions use Windows Credential Manager.
No Windows publisher signing is configured. Respect SmartScreen and organization policy.

**Linux:** The packages require a compatible glibc runtime.
Ubuntu 24.04 CI does not establish support for older distributions.
See the [package-only evidence](desktop-acceptance.md#azure-linux-package-only-follow-up)
for a tested desktop combination.
Saved sessions require a user D-Bus service and an unlocked Secret Service
provider, such as GNOME Keyring or a compatible KDE Wallet.
Headless systems and WSL may not provide them.

**macOS:** Saved sessions use the login Keychain.
Packages do not have Developer ID signing or notarization.
A local CI run does not establish Gatekeeper trust for a browser download.
Do not remove quarantine or disable Gatekeeper to run the package.
See Apple's [notarization guidance](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).
macOS production reads remain gated.

An OS prompt or store error is not proof that no saved session exists.
Windows and Linux process-only authentication does not require a native store.
See [session input choices](authentication-design.md#agreed-authentication-follow-up).
Real account and credential-store tests need separate explicit approval.

</details>

<details>
<summary>Release checks for maintainers</summary>

### Release readiness

Installation checks do not establish release readiness or live account support.
Use the [desktop acceptance matrix](desktop-acceptance.md#remaining-acceptance-matrix)
for candidate-specific evidence and remaining gaps.
Native stores, real clients, browser setup, and account reads have separate
approval gates. Package evidence does not grant those approvals.

Before publication, review checksums, final-byte provenance, native startup,
OS download trust, client setup, version, lockfile, dated changelog, and assets.
Trusted tag and default-branch manual runs must attest all six distribution
assets. Missing or failed provenance blocks draft preparation.
Manual packaging uploads test artifacts only. A maintainer-approved version
tag can prepare a draft. Do not publish while release blockers remain.

Windows/Linux client acceptance remains in
[#34](https://github.com/PlagueHO/beyond-gate/issues/34).
Shared authentication is implemented in
[#52](https://github.com/PlagueHO/beyond-gate/issues/52).
macOS and Claude Code acceptance remain in
[#62](https://github.com/PlagueHO/beyond-gate/issues/62).
Candidate-release decisions remain in
[#41](https://github.com/PlagueHO/beyond-gate/issues/41).
There is no crate publishing, cloud hosting, auto-updater, or required cloud service.

</details>

[signing-policy]: https://github.com/PlagueHO/beyond-gate/blob/main/MAINTAINERS.md#code-signing-policy
