# Desktop acceptance

**Status: partial desktop evidence, not full desktop acceptance.**
Issue [#34](https://github.com/PlagueHO/beyond-gate/issues/34) stays open for
Windows/Linux simple-use acceptance and explicit claim decisions.
A package check, synthetic registration,
or development-machine run is not a clean desktop result.

## Issue completion checks

These statuses describe the checks in [#34](https://github.com/PlagueHO/beyond-gate/issues/34).
They do not change the overall partial acceptance status.

| Completion check | Status | Evidence or remaining work |
|------------------|--------|----------------------------|
| Record candidate versions, platforms, commands, results, and gaps | completed for the checks run | [Candidate evidence](#candidate-and-evidence) and the [acceptance matrix](#remaining-acceptance-matrix) record results and limits |
| Validate terminal startup without development runtimes and state tested runtime requirements | partial | Windows startup and Linux shell startup have evidence; new-candidate and terminal-only coverage remain incomplete; older minimums stay separate |
| Validate process-only environment authentication and hidden CLI input | blocked | #52 implementation is required; these paths do not exist yet |
| Check explicit protected-save and locked/unavailable store claims separately | incomplete | Linux synthetic lifecycle and failure states passed; Windows locked and unavailable states remain untested |
| Check real client discovery, shutdown, and supported reads when prerequisites exist | partial | VS Code discovery, shutdown, and one supported read passed; Copilot CLI discovery and shutdown passed offline; other client cases remain unverified or deferred |
| Update installation and troubleshooting guidance and identify untested claims | completed | [Installation and troubleshooting](installation.md) and the [acceptance matrix](#remaining-acceptance-matrix) state known requirements and gaps |
| Check optional extension/native-host lifecycle for each claimed browser/OS combination | partial | Windows Chrome and Edge setup, direct host launch, and removal passed; distinct-candidate updates and other lifecycle cases remain untested |

The user deferred Claude Code and macOS checks to [#62](https://github.com/PlagueHO/beyond-gate/issues/62).
The user revised #34 to separate Windows/Linux terminal-only use from optional
browser/protected-save paths and later hardening. Implement shared authentication
and Linux session/read support in [#52](https://github.com/PlagueHO/beyond-gate/issues/52)
first. Environment mode must not require native storage, a browser, desktop,
D-Bus, or Secret Service. Then check identified packages through CLI and real
VS Code/Copilot CLI clients. These new paths remain unimplemented.

Windows character and campaign reads are now implemented, but their live
acceptance remains unverified. No new native or live tests ran for this backlog
revision. Hardening and required release claims remain explicit under
[#41](https://github.com/PlagueHO/beyond-gate/issues/41).
The original issue remains open. Reuse completed
evidence within its recorded candidate, platform, and test scope. New checks
are still required when changes or release claims exceed that scope.

## Candidate and evidence

Use the [package matrix](installation.md#package-matrix). Its three native CI
baselines are not minimum supported OS versions. No other CPU, emulation mode,
WSL session, or headless desktop has acceptance evidence from these checks.
Do not expand support claims from a successful build.

Record one result for each candidate, OS/CPU, client, and browser combination.
Use `passed` only when the stated check ran and met its assertions.
Use `failed` for an assertion or infrastructure failure, `blocked` for a missing
prerequisite or approval, and `not_applicable` only for an explicitly excluded case.
Do not count blocked cases as successful or silently omit them.

| Field | Required evidence |
|-------|-------------------|
| Candidate | Package version, archive name, SHA-256, source commit, and whether the source had local changes |
| Host | Exact OS/build, CPU, native runtime versions, and desktop/session type |
| Components | Both binary versions; Windows native-host and extension versions, extension ID, and browser flavor/version |
| Client | Exact client version, configuration destination, isolation method, and relevant policy limits |
| Check | Case ID, exact command with private paths replaced by placeholders, expected result, actual result, and exit code |
| Approval | Exact permitted identity, operations, request/credential limits, deadlines, stop conditions, and cleanup |
| Cleanup | Owned processes stopped, owned entries/files removed, and unrelated settings unchanged |
| Limits | Checks not run, blockers, and the precise claim each result supports |

Keep reports free of credentials, account identifiers, private paths, raw client
settings, account snapshots, proprietary text, and raw child output.
A source commit supplied to the checker is metadata, not build attestation.
The archive hash identifies the tested bytes. Record local source changes
separately instead of claiming that a dirty build came from an unchanged commit.

Current character, campaign, and shared-session sources differ from the retained
candidate sources. Earlier unsupported-read observations apply only to their
tested binaries. They are not account-free test instructions for current builds.
Review relevant source changes before reusing evidence for a new candidate.
Do not rebuild or repeat a check solely because work resumes.

## Offline package checks

These commands use PowerShell 7.4 or later as development tooling.
They do not require a real client, browser, credential service, or account.
Run them on the matching native OS/CPU:

```powershell
pwsh -NoProfile -File .\scripts\test-package.ps1 `
    -Archive <archive> -Checksum <checksum> -Version <version> -Target <target> `
    -SourceCommit <source-commit> -ReportPath <report.json>
pwsh -NoProfile -File .\scripts\test-package-failures.ps1 `
    -Archive <archive> -Version <version> -Target <target>
```

Replace the placeholders before execution. The report's command uses the same
portable placeholders to avoid retaining private paths. Its candidate fields
record the actual version, target, hash, and supplied source identity.
The report's parent directory must already exist. Keep it outside the archive
and outside the temporary installation. Relative report paths use the current
PowerShell location, including after `Set-Location`.
Without `-ReportPath`, existing command
usage still works. Without `-SourceCommit`, source identity is `unavailable`.

The verifier checks exact archive membership before extraction. It checks both
binaries, native MCP startup/discovery/shutdown, safe input errors, and synthetic
client registration. Windows also checks the native-host origin refusal and
browser package identity, version, permissions, and bounded members.
The extension version must equal the package version. Matching version numbers
do not prove protocol compatibility.

Lifecycle checks use only owned temporary state:

- Start both binaries by name with a process-local PATH.
- Check the explicit missing-sibling error.
- Extract the same candidate again and compare every package file hash.
- Move the installation to another path with spaces and start the sibling server.
- Confirm that old registrations cause a conflict without changing settings.
- Remove only the synthetic server entry, confirm fresh registration at the new
  path, and preserve unrelated synthetic settings.
- Remove the owned installation and verify temporary cleanup.

Same-candidate extraction is a reinstall check, not an upgrade between releases.
Manual JSON entry removal is a documented removal procedure, not a product
`mcp uninstall` command. No persistent PATH or real settings are changed.
Changing `HOME` does not isolate a native OS credential store.

The JSON report has schema version 1 and scope `offline-package`.
Its `cases` list records offline assertions and cleanup. Its separate `manual`
list remains blocked. `full_coverage` is always false, even when offline
`status` is `passed`. A failed assertion, cleanup failure, or report-write failure
fails the invocation. Early failures leave later cases blocked.
Failure reports use fixed categories, not exception messages.
`exit_code` is 0 for passed offline checks and 1 for failed checks.
If the report itself cannot be written, the command fails without a report.

Packaging CI retains `package-evidence-<target>` separately from native archives.
It attempts evidence upload even after a check fails. A build or packaging
failure before the verifier starts can leave no report; that is missing evidence,
not a pass. Evidence is not a release asset or authorization to publish.

### Local Windows offline evidence

The local candidate check used version `0.1.0`, target
`x86_64-pc-windows-msvc`, Windows build `10.0.26100.0`, X64 processes, and
PowerShell `7.6.9`. The supplied source commit was
`ff2196ec1e4d2cc0ad9226ec84d473003d24ac5d` with local script and documentation
changes. This was not an attested clean-source build.
The tested archive SHA-256 was
`af31d0a7e2257d0e9df83be778166de6fd299b1e9daa85955b558976273e5f56`.
The extension version was `0.1.0`, with the expected pinned ID.

| Command | Result |
|---------|--------|
| `pwsh -NoProfile -File .\scripts\cargo.ps1 build --release --workspace --locked` | Passed with Visual Studio 2022 Build Tools. |
| `pwsh -NoProfile -File .\scripts\test-package.ps1 -Archive <archive> -Checksum <checksum> -Version 0.1.0 -Target x86_64-pc-windows-msvc -SourceCommit ff2196ec1e4d2cc0ad9226ec84d473003d24ac5d -ReportPath <report>` | Passed, exit 0. All 16 offline groups passed; all 11 manual groups remained blocked. |
| `pwsh -NoProfile -File .\scripts\test-package.ps1 -Archive <archive> -Checksum <checksum> -Version 0.1.0 -Target x86_64-pc-windows-msvc` | Passed without the optional report inputs. |
| `pwsh -NoProfile -File .\scripts\test-package-failures.ps1 -Archive <archive> -Version 0.1.0 -Target x86_64-pc-windows-msvc` | Passed, exit 0. Rejections included browser version mismatch, cleanup failure, report-write failure, and preservation of the first failure. |
| `pwsh -NoProfile -File .\scripts\check-docs.ps1` | Passed with pinned rumdl `0.3.2`. Style and offline local-link checks found no issues. |

The temporary archive used the workflow's exact file layout. The report retained
safe candidate and host metadata only. Expected injected errors appeared in the
rejection tests; they were assertions of failure handling, not a package failure.
No native credential, real client, browser profile, registry mapping, persistent
PATH, or account was accessed or changed by these package checks.
At that checkpoint, Linux/macOS execution, hosted CI, clean desktops, minimum
runtimes, real upgrades, and all separately gated checks remained unverified.

### Retained CI candidate verification

The 2026-11-07 follow-up checked retained artifacts from successful packaging
[run 42182052563](https://github.com/PlagueHO/beyond-gate/actions/runs/42182052563).
The initial Windows/Linux review was later extended to all three native targets.
Each version `0.1.0` archive matched both its SHA-256 sidecar and retained
`package-evidence-<target>` report hash:

| Target | Archive SHA-256 | Actual CI host | Offline case results |
|--------|-----------------|----------------|----------------------|
| `x86_64-pc-windows-msvc` | `a3b91503ec3b5080aa5833f5c8836096d7334826c9b500e4a6ff87429770af5b` | Windows `10.0.20348.0`, X64 | 16 passed |
| `x86_64-unknown-linux-gnu` | `0bb9b8191023f4d0d956d656ea0bc52b84fed64fa1ade289dd1c7c24d73da87b` | Ubuntu 24.04.5 LTS, X64 | 15 passed; Windows browser package not applicable |
| `aarch64-apple-darwin` | `ecea5c99a46403050a1d1ef93a9be978d0dc7049737533d9f51f3c5a240d711c` | macOS 15.8.2, ARM64 | 15 passed; Windows browser package not applicable |

All three reports identify source commit
`f4c2b21e774b117a8a33e74d32309e7fd64f2001`, the PR merge commit, not the
PR head `b93d1418672fd4b086c7690d222089ed8b563b94`.
GitHub's Git commit API and local `git rev-parse 'HEAD^{tree}'` confirmed that
the reported source and checkout `03ef03d6fc0ee37f15d43dbf4655cc41bc9633b1`
share tree `0c026d13af9f0f957d1e30821191d338995d8107`.
Source-tree equality is not reproducible-build attestation.

Each schema-1 report has scope `offline-package`, exit code `0`, completed
cleanup, and 11 blocked manual groups. Each records `full_coverage: false`,
zero live requests, no credential access, and no real clients.

The downloaded Windows archive also passed these local checks:

| Command | Result |
|---------|--------|
| `pwsh -NoProfile -File .\scripts\test-package.ps1 -Archive <archive> -Checksum <checksum> -Version 0.1.0 -Target x86_64-pc-windows-msvc -SourceCommit f4c2b21e774b117a8a33e74d32309e7fd64f2001 -ReportPath <report>` | Passed, exit 0. All 16 offline groups passed; all 11 manual groups remained blocked. |
| `pwsh -NoProfile -File .\scripts\test-package-failures.ps1 -Archive <archive> -Version 0.1.0 -Target x86_64-pc-windows-msvc` | Passed, exit 0. Included escaped-path privacy, report destinations, cleanup failures, and existing archive rejection cases. |
| `pwsh -NoProfile -File .\scripts\test-e2e-harness.ps1` | Passed, exit 0. All 177 offline assertions passed after the login expectation fix. |
| `pwsh -NoProfile -File .\scripts\test-e2e.ps1 -BinaryDirectory <extracted-package> -Selection contracts -ReportPath <report>` | Passed after the login expectation fix, exit 0. 108 cases passed, 50 live cases remained blocked, and four native cases were not selected. |

The initial credential-free suite run failed only `cli.auth.login-blocker`.
Its Windows expectation still required the unsupported-platform message.
The harness now checks the interactive-terminal refusal required by the existing
CLI process test. Non-Windows expectations and product behavior are unchanged.
All 18 synthetic installation cases passed in both follow-up runs.
This does not establish a cause for the five historical installation failures.

The injected errors in rejection checks were expected. No real credentials,
client settings, browser profiles, registry mappings, or persistent PATH were
accessed or changed by these offline checks. Linux and macOS reports were
inspected, not rerun locally. The separately approved desktop results below
and the [remaining acceptance matrix](#remaining-acceptance-matrix) record
later passes and gaps. macOS desktop acceptance remains unverified.

### Azure Windows desktop baseline

The approved Azure test VM was created on 2026-11-07 in Australia East.
It uses the Windows 11 Enterprise 25H2 x64 image
`MicrosoftWindowsDesktop:windows-11:win11-25h2-ent:26200.9725.261024`
and `Standard_D2s_v5` with two vCPUs and 8 GiB RAM. A read-only guest inventory
reported Windows `10.0.26200`, build `26200`, and 64-bit OS and CPU.

The initial image did not have the x64 Visual C++ redistributable registered.
The inventory found no Rust, Cargo, Python, Node.js, PowerShell 7, VS Code,
Claude Code, or Copilot CLI commands. Windows PowerShell remains available as
part of the OS. This is the recorded pre-install baseline. The Windows package
requires the x64 Visual C++ v14 Redistributable; see the
[installation guide](installation.md#windows) for supported installation
methods.

The VM has a public address for direct RDP. Its network rule permits TCP 3389
only from the user-approved source address; all other public inbound traffic
remains denied. An automatic VM shutdown is enabled for 23:00 UTC. The public
address, source address, subscription identifier, and temporary administrator
credential are not recorded here.

No native credential, real client, browser profile, account, or live service
was accessed. The VM's OS image and RDP path are not evidence of physical
desktop trust behavior or minimum Windows requirements.

### Windows package startup and runtime

The [verified Windows x64 `0.1.0` archive](#retained-ci-candidate-verification)
was extracted to a user-local path with spaces.

Before installing a Visual C++ runtime, both `beyond-gate.exe` and
`beyond-gate-mcp.exe` exited with `-1073741515` (`0xC0000135`) and produced no
output. The VM inventory had already confirmed that the x64 runtime was absent.
This confirms that the package does not start on this image without its runtime
prerequisite.

After explicit approval, WinGet installed
`Microsoft.VCRedist.2015+.x64`, version `14.51.36247.0`. Both binaries then
passed `--version` and `--help` from the path with spaces, with exit code `0`.
The version output was `beyond-gate 0.1.0` and `beyond-gate-mcp 0.1.0`.
Adding the package directory to the current process `PATH` allowed both binary
names to resolve. Adding it to the test account's user `Path` also worked in a
new PowerShell process. The test account's `Path` entry was removed after the
check. Extracting the same archive again and copying the package to another
path with spaces also left both binaries runnable. This verifies the documented
prerequisite and recovery path on this VM. It does not establish a minimum
supported Windows build or runtime version.

After both historical package tests, the dedicated Azure resource group was
deleted and verified absent. The temporary Windows drive mapping and Linux SSH
key were removed. The separately approved Windows follow-up below used a new VM.

### Azure Linux desktop baseline

The approved Azure test VM used Ubuntu 24.04.5 LTS x64, image
`Canonical:ubuntu-24_04-lts:server:24.04.202610250`, and
`Standard_D2s_v5`. The 64-GiB OS disk used Standard SSD LRS. SSH and RDP were
restricted to the approved source address. Automatic shutdown was enabled for
00:10 UTC.

The guest tested the [verified Linux x64 `0.1.0` archive](#retained-ci-candidate-verification)
and reported `x86_64` and glibc `2.39`. `ldd` showed only the standard
glibc loader, `libc`, `libm`, and `libgcc_s` dependencies for both binaries;
no dependency was reported missing. Before installing the desktop environment,
both binaries passed `--version` and `--help` from `$HOME/Beyond Gate Test`.
The current SSH session also found both binaries by name after a temporary
session-only `PATH` update. The pre-desktop inventory did not enumerate all
development tools or installed runtimes.

XFCE, XRDP, Xorg XRDP support, and D-Bus X11 support were then installed to
provide an interactive desktop. The XRDP service was active. The user opened
an XFCE session and ran help and version for both package binaries there.
This confirms basic package startup in that Linux desktop session. It does not
test Secret Service availability or Linux credential storage. No client,
native-store, browser, sign-in, or account tests were run. The archive was
transferred over source-restricted SSH, so browser-download trust prompts were
not tested. This single image does not establish a minimum Linux or glibc
version.

### Separately approved Windows desktop follow-up

A new disposable VM repeated the [Windows baseline](#azure-windows-desktop-baseline)
on 2026-11-07 with the same image, size, region, OS build, and initial tool/runtime
absence. Azure Run Command supplied inventory only; the user ran all following
checks in the interactive RDP desktop, not as SYSTEM.
Separate approvals covered provisioning, installation, synthetic native storage,
and browser setup. The new USD 10 planning ceiling was not an enforced spending
cap. Licensing, source-restricted RDP, shutdown, and resource deletion were approved.

#### Desktop package result

The retained Windows `0.1.0` candidate passed seven installation checks and
owned-file cleanup. Both binaries first returned `-1073741515` before runtime
installation. The signed Microsoft x64 runtime installer from
`https://aka.ms/vc14/vc_redist.x64.exe` installed version `v14.51.36247.00`.
Its SHA-256 was
`843068991daaa1f73ad9f6239bce4d0f6a07a51f18c37ea2a867e9beca71295c`.
The script checked the Microsoft Authenticode signature. No restart was needed.
Startup, process-local PATH, same-candidate reinstall, and relocation passed.
Persistent user PATH was unchanged.

#### Synthetic native-store result

Separately built `e2e-isolation` binaries used source
`b65417252af1ca83c052b265ed3145c136add183` with local documentation changes only.
Product Rust sources were unchanged. Both build commands passed:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 build --workspace --locked --features 'beyond-gate-cli/e2e-isolation,beyond-gate-mcp/e2e-isolation' --target-dir target\manual-e2e
pwsh -NoProfile -File .\scripts\cargo.ps1 build -p beyond-gate-core --example e2e-session --features e2e-isolation --locked --target-dir target\manual-e2e
```

The Windows PowerShell orchestration checked all three binary hashes and exact
isolation handshakes before enabling native access. One random synthetic
`beyond-gate-e2e` entry was approved, with a limit of 12 local store commands.
Eight commands checked absence, synthetic stored-unverified state, logout,
repeated logout, final absence, and cleanup. All six reported cases, including
the handshake, passed. The product credential entry was not accessed.

#### Manual browser installation result

Regular Chrome `157.0.8103.39` and stable Edge `157.0.4341.42` loaded the packaged
extension in fresh test profiles. The package manifest version was `0.1.0`;
its public key produced ID `gnhdconfgjfdbbgkemadegjcjnnodefh`.
The user confirmed that ID in both browsers. The native-host binary SHA-256 was
`d53fa7fe1bbea6342920657fce7d4c8f76d993ec3d6e19a93cb23d6ce73b1ba7`.
Setup used a stable user-local installation with spaces, outside source and
build trees. The following commands used `$install` for that directory and
`$cli` for its `beyond-gate.exe`. Each write had interactive confirmation.

| Commands or action | Result |
|--------------------|--------|
| `& $cli auth install edge --directory $install --dry-run` and corresponding `chrome` preview | Passed; selected per-user mappings, pinned origin, and installed host path displayed |
| `& $cli auth install edge --directory $install` and corresponding `chrome` setup | Passed; independent checks found matching manifests and HKCU mappings in the checked registry views |
| Repeat both setup commands after capturing a verified baseline | Passed; both manifests remained unchanged and both mappings still matched |
| `& $cli auth uninstall edge --directory $install --dry-run`, then confirmed removal | Passed; Edge mapping and manifest absent, Chrome mapping preserved |
| Corresponding `chrome` removal preview and confirmed removal | Passed; both mappings and manifests absent, installation binaries preserved |
| Remove the extension in both browsers, close test windows, and delete the owned installation directory | Passed; user confirmed extension removal and the directory absence check passed |

The temporary session verifier was run as
`powershell.exe -NoProfile -File .\check-browser-setup.ps1 -Stage Installed`,
then with `EdgeRemoved` and `Removed`. A missing-script transfer error was
resolved before the verified baseline, repeat, and removal checks.

No login, cookie query, product credential access, or D&D Beyond request occurred
in these installation checks. Remaining browser cases are listed in the matrix.
The temporary orchestration and safe reports remain session evidence, not
shipped package checks.

The ownership-checked deletion command completed with exit code `0`.
It verified that the owned resource group was absent before removing its
protected local state. No VM, disk, or public IP from this run remains.
The final subscription-specific charge was not measured.

### Browser-to-native-host negative checks

On 2026-11-07, a separate approved disposable Windows 11 Enterprise 25H2 x64
VM used the retained `0.1.0` Windows archive and the same image, size, and
region as the [Windows desktop follow-up](#separately-approved-windows-desktop-follow-up).
The signed Microsoft Visual C++ runtime, stable Edge `157.0.4341.42`, and
regular Chrome `157.0.8103.39` were installed or verified. The package and native
host hashes and extension ID matched the
[manual installation result](#manual-browser-installation-result).

The user loaded the packaged extension in each fresh browser profile and
confirmed its ID. With no pending login, the user submitted one synthetic
43-character key and selected the required consent checkbox. The user observed
both the `Login did not complete` heading and the message `Start a new
interactive beyond-gate auth login command.` in each browser. Edge was tested
first; its registration was removed before Chrome was installed. Both selected
per-user registrations and their host manifests were absent after removal.

The expected `no_pending_login` response requires the browser extension to
reach the native host. The extension queries a cookie only after a pending
authorization response, which this test did not receive. No login or account
access was approved or attempted. This was not a network or cookie-access trace.
The test does not establish a successful login, cookie handoff, or Edge fallback.
The guest helper recorded package identity, browser versions, setup, and removal;
the browser UI results are user observations.

After both removal checks, the ownership-checked cleanup deleted the VM
resource group and protected local access files. A separate check confirmed
that the resource group and local access state were absent. The final Azure
charge was not measured; the USD 10 planning ceiling was not an enforced cap.

### Azure Linux package-only follow-up

A credential-free follow-up on 2026-11-07 used the retained Linux x64 `0.1.0`
archive and pinned image
`Canonical:ubuntu-24_04-lts:server:24.04.202610250`.
The VM used `Standard_D2s_v5`, a 32-GiB Standard SSD LRS OS disk, and Australia
East. SSH allowed only the runner's source address. The SSH host key was checked
through the Azure control plane before transfer. No development runtime or
desktop environment was installed for these checks.

The guest reported Ubuntu 24.04.5 LTS, `x86_64`, kernel
`7.0.0-1017-azure`, and glibc `2.39`. Package versions were
`libc6` `2.39-0ubuntu8.9`, `libgcc-s1` `14.2.0-4ubuntu2~24.04.1`, and
`bash` `5.2.21-2ubuntu4`. Command lookup found no Rust, Cargo, `python`, Node.js,
npm, PowerShell 7, .NET, VS Code, Claude Code, or Copilot CLI commands.
System `python3` was present at version `3.12.3`. This is an observed inventory,
not proof that all development runtimes were absent.

Session-only orchestration used
`pwsh -NoProfile -File <run-directory>\run-linux-package-vm.ps1`.
It transferred the verified archive and an owned shell script to the disposable
account. The guest command was
`timeout 180 bash "$HOME/verify-linux-package.sh"`.
The final orchestration returned exit code `0`. Its safe report,
`linux-package-shell-report.json`, records scope `linux-package-shell`,
`desktop_session: false`, and `full_desktop_acceptance: false`.

| Case | Expected and observed result |
|------|------------------------------|
| Inventory and identity | Command/runtime inventory recorded; archive SHA-256 matched the retained candidate; six regular package files and the root directory matched |
| Startup and dependencies | Both binaries retained executable permission; `ldd "$install/$binary"` found no missing library; `"$install/$binary" --help` and `"$install/$binary" --version` passed without stderr |
| Process-local PATH | Both binary names started from the installation path with spaces |
| New login shell PATH | `bash --login -c 'beyond-gate --version && beyond-gate-mcp --version'` read the disposable account's profile and returned both exact `0.1.0` versions |
| Same-candidate reinstall | All six extracted files matched byte for byte |
| Relocation | Both binaries started after moving the installation to another path with spaces |
| Removal and preservation | Both installations were removed; a fresh login shell no longer found either binary; an unrelated synthetic PATH entry and sentinel were preserved |
| Cleanup | The original profile was restored byte for byte; owned guest files and temporary local access files were removed; Azure resource-group absence was verified |

Two earlier attempts failed in the session scripts, not the product. The first
script had Windows line endings and did not run. The second archive check
rejected the valid `./` root directory; its failure key also failed the safe-output
parser. The scripts were corrected. LF format, Bash syntax, and the archive
contract were checked locally before the final passing run. Both failed reports
were retained, and each attempt's VM resources were deleted before the next
attempt. Only one VM existed at a time.

The three sequential attempts used the user's new USD 10 total planning ceiling.
The [Azure retail prices API](https://prices.azure.com/api/retail/prices)
returned a Linux compute quote of USD 0.12 per hour before provisioning.
Automatic shutdown was configured as a backup. The owned resource group was
verified absent after each run; final absence was checked again.
The actual subscription charge was not measured. The planning ceiling was not
an enforced spending cap.

Only package files, help, version, runtime lookup, and shell settings were checked.
No native credential, browser, real client, sign-in, model call, or D&D Beyond
request was used. A new login shell over noninteractive SSH is not a new desktop
terminal. These results do not establish desktop PATH behavior, a strict
runtime-free desktop baseline, download trust, a minimum OS/runtime, an actual
upgrade, or Secret Service behavior.

### Local VS Code Insiders prerequisite

A separately approved local preflight used installed VS Code Insiders
`1.145.0-insider`, commit `a3797664749856829367927a93d65834507e4d12`,
and the retained Windows `0.1.0` candidate. The process had fresh user-data,
extension, home, and `COPILOT_HOME` directories. Its PATH included the actual
Insiders command directory and Windows System32, but no `code` command or shim.

`beyond-gate mcp install vscode --dry-run` returned exit code `1`, empty stdout,
and `error: VS Code was not found on PATH.` The generated test configuration
was unchanged. The installer checks `code`, not `code-insiders`. No editor,
MCP server, sign-in, model call, credential access, or D&D Beyond request ran.
The owned local test directories were removed. The safe detection report remains
session evidence.

This is a demonstrated installer prerequisite limit, not a discovery result or
an Insiders compatibility test. The next approved client run uses stable VS Code
on a disposable Windows VM instead.

### Stable VS Code registration follow-up

A disposable Windows 11 Enterprise 25H2 x64 VM used the retained Windows
`0.1.0` candidate and stable VS Code `1.145.2`, commit
`c4453f8ea430795f941c1b2ff6c9321135a74a02`. The tested VS Code archive had
SHA-256 `33cf166a751090fe7d244bf0e594e3c69c832d5ae2af1769140ad82a35ea1e0b`.
The signed Microsoft Visual C++ x64 runtime was installed before testing.

In the interactive test account, `beyond-gate mcp install vscode --dry-run`
and the confirmed installation passed. Repeating installation preserved the
registration. An unrelated synthetic setting was unchanged. Detection used the
actual stable `code.cmd`, with no shim. The settings used a fresh `COPILOT_HOME`.

GitHub temporarily blocked sign-in before the server was started. Discovery,
invalid input, shutdown, restart, and entry removal were not tested. No model
request or D&D Beyond request ran. The resource group and public IP were
deleted. A local cleanup error was corrected and the cleanup repeated; final
resource absence was verified. The fixed reports remain session evidence.

#### Stable VS Code client retry

A second disposable VM used the same Windows image and candidate, stable
VS Code `1.145.2`, and an isolated test account and client profile. The user
used MSTSC with clipboard redirection. The actual stable `code.cmd` was on
the test process PATH. Registration, repeat registration, and preservation of
an unrelated synthetic setting passed again.

A session-only observer used the real VS Code `lm.tools` registry. It reported
all eight Beyond Gate tools. One model-free `lm.invokeTool` call with an
unknown field was rejected. The result does not identify whether the client or
server rejected the input. The observer saw two distinct server processes,
with an exit between them, and all eight tools remained available after the
restart. This does not prove that VS Code fetched a fresh tool list on restart.
No model-driven tool routing was tested.

The user closed VS Code after the restart. A separate process check confirmed
that the owned server and client were stopped. A removal helper deleted the
test entry but failed when checking the resulting empty collection. A corrected
helper verified the entry was absent and the unrelated synthetic setting
remained. The user reopened the isolated profile and confirmed that
`beyond-gate` was absent from **MCP: List Servers**. The first observer report
still says removal was pending; the separate removal report and user check
provide the later evidence.

The reopened VS Code log included a `safeStorage.decryptString` failure. Its
cause is unknown. This is not a successful authentication or credential-store
test. The helper also initially used a stale redirected-drive shortcut; it was
corrected before the client test. The test permitted no D&D Beyond requests
or product credential access. An outbound firewall rule blocked the packaged
MCP server. The observer made no model request. The test did not cover supported
account reads or a model-driven agent call.

After the test, the owned Azure resource group, public IP, and protected local
access files were deleted. A separate Azure query verified their absence.
The actual subscription charge was not measured.

#### Local unsigned VS Code preflight

Stable VS Code `1.143.1`, commit `e646868227c3e3a9e3f24277c8c7220be42c0be7`,
ran on the existing Windows account with fresh user-data, extensions, home,
and `COPILOT_HOME` directories. The process did not inherit authentication
token variables. GitHub and Microsoft authentication extensions were disabled.
The user started the retained packaged server in the test window.

A session-only observer found all eight tools in `lm.tools` and invoked the
unsupported `character_list` tool through `lm.invokeTool`. It reported an
unsupported error without a model request. The core operation does not load a
credential or send a request. The observer did not identify whether the client
or server supplied the error. The owned editor and server exited after the
check. No sign-in or live account read was performed.

This preflight shows that this client version can expose and invoke a tool
without sign-in in this test configuration. It does not prove complete
OS credential-store isolation or that a supported account read will succeed.

#### Local VS Code Web-library read

The user separately approved one `book_list` call on the same Windows account.
Stable VS Code `1.143.1` used the retained packaged server with fresh test
settings and disabled GitHub and Microsoft authentication extensions. The
session-only observer used `lm.invokeTool` without a model request. The test
used the existing protected Beyond Gate entry and did not ask for credentials.

The approved scope allowed at most two fixed-entry loads and five HTTP attempts:
the fixed public configuration and protected Web-library GETs, with up to two
authentication exchanges. It excluded retries, redirects, credential writes,
and book content reads. The product's discovery deadline was 30 seconds; the
observer cancelled after 35 seconds. Exact HTTP and credential-load counts
were not instrumented.

The call returned valid metadata for 169 books: 36 purchased and 133 unknown.
No shared or unavailable books were reported. The observer retained only fixed
status and aggregate counts, not book IDs, titles, URLs, or raw responses.
VS Code could have written temporary state or logs in the test directory; the
test did not inspect them. The owned editor and server exited, and the complete
test directory was removed. No model-driven routing or GitHub sign-in was
tested. This single read does not prove sharing or book-content access.

### Local Copilot CLI offline client

A separately approved local test used Copilot CLI `1.0.106` and the retained
Windows `0.1.0` candidate. The installer used the real `copilot` executable,
an installation path with spaces, a fresh `COPILOT_HOME`, and an empty working
directory. Preview, confirmed registration, repeat registration, and preservation
of an unrelated synthetic setting passed. `copilot mcp list --json` listed the
test registration. That list alone does not establish tool discovery.

The interactive client used a restricted process environment with no inherited
authentication tokens. `COPILOT_OFFLINE=true` disabled GitHub authentication
and network access in the client. A loopback provider address supplied the
required local provider configuration. No model request was sent. The client's
`/mcp` dashboard connected the packaged server and showed all eight tools.
The dashboard's Disable action stopped the owned server process. Enable started
a different process, and the dashboard again showed all eight tools. The dashboard
then removed the test registration. The second server process exited, the
registration disappeared, and the unrelated synthetic setting remained.

The dashboard also showed four other disabled server names as session entries.
Their source was not established. Therefore, the test does not prove complete
client settings isolation. It used the current Windows identity, not a clean
OS account. No sign-in, model-driven routing, model-free tool call, D&D Beyond
request, or intentional credential access was performed. The safe-error case
remains untested. Offline discovery is not evidence of an authenticated client
or supported account reads. The test did not inspect the other server entries.

### Linux Secret Service synthetic follow-up

A separately approved run on 2026-11-08 used one disposable Ubuntu 24.04 x64
Azure VM in Australia East. It used the image
`Canonical:ubuntu-24_04-lts:server:24.04.202610250`, size `Standard_D2s_v5`,
kernel `7.0.0-1017-azure`, XFCE, D-Bus `1.14.10-4ubuntu4.1`, GNOME Keyring
`46.1-2ubuntu0.2`, and Seahorse `43.0-3build2`. The first Remote Desktop session
failed because the Xorg backend was missing. Installing `xorgxrdp`
`1:0.9.19-1` and `xserver-xorg-core` allowed a later desktop sign-in.

The test used Linux binaries built from commit
`4f842080dd0c6e02d5294bab2081992962aec0b7` with the off-by-default
`e2e-isolation` feature. The stripped CLI SHA-256 was
`e20b40937bed7509d3c5d2b8dcc0f28b5ce8aba8a15a3e9cdac4d4ca4f6c9789`;
the stripped helper SHA-256 was
`643ed06e7fc48f83d7db5b5e458a940ded7ca004eb98b8751488d2f525f53743`.
These are feature-built test binaries, not the retained release package.

The user created a disposable default keyring and ran a session-only test
runner in the desktop account. The runner verified both binary hashes and
isolation handshakes. The user confirmed that `init` returned the ready result
and that `healthy` checked absence, one synthetic write, and stored-unverified
status. The commands were:

```bash
python3 ./linux-store-runner.py init
python3 ./linux-store-runner.py healthy
python3 ./linux-store-runner.py locked
python3 ./linux-store-runner.py emergency-cleanup
```

The `locked` step stopped before a native entry operation. The runner did not
report a specific cause, so the locked case is blocked, not passed. The
unavailable case was not run after that stop. The user then reported that
`emergency-cleanup` removed the synthetic entry and verified absence. A separate
safe state check recorded phase `cleaned` and five attempted entry operations.
The cleanup used the helper, not a repeated CLI logout check.

The test used no D&D Beyond account, browser login, or product credential entry.
The session-only runner and fixed-shape report are not shipped or run in CI.
An independent query confirmed that the VM resource group and protected local
access files were absent. Azure charges were not measured. The USD 10 planning
ceiling was not an enforced spending cap.

### Linux Secret Service locked-store retry

A separately approved retry on 2026-11-08 used a new disposable VM with the
same image, size, kernel, desktop, D-Bus, GNOME Keyring, Seahorse, xrdp, and
xorgxrdp versions as the earlier follow-up. It used the same feature-built
binaries. A separate copy of the session-only runner added fixed preflight stop
categories. Its offline guard tests passed.

The user created and unlocked a disposable default keyring. The runner checked
absence, wrote one synthetic entry, and reported stored-unverified status.
After the user locked the keyring, the locked `status` operation returned
`storage_access`. The user cancelled both unlock prompts. The locked `logout`
operation returned `unexpected_result`; the runner did not retain the native
output, so the cause and exact product response are unknown. The test stopped
at that result. The unavailable-service and repeated CLI logout checks did not
run.

After the user unlocked the keyring, helper cleanup and the final absence check
passed. A separate safe state check confirmed phase `cleaned` and seven attempted
entry operations. The test used no D&D Beyond account, browser login, or product
credential entry. These results do not prove Linux production login or session
use. An independent query confirmed that the VM resource group and protected
local access files were absent. Azure charges were not measured.

### Linux Secret Service complete synthetic run

A further separately approved run on 2026-11-08 used a disposable Ubuntu VM
with the same image, size, kernel, desktop, and package versions as the locked
store retry. A previous attempt stopped at initialization with phase `ready`,
zero entry operations, and no synthetic write. The user later reported that the
initialization command may have run twice. That attempt did not proceed.

The completed run used the same feature-built binaries and a revised session-only
runner. The runner recognizes both status and logout error formats without
recording native output. Its offline guard and diagnostic tests passed. The
user confirmed the following results from the desktop session:

1. Initial absence, one synthetic write, and stored-unverified status passed.
2. With the default keyring locked, status and logout both returned
   `storage_access`. The user cancelled the unlock prompts.
3. After the keyring was unlocked, stored-unverified status passed.
4. On an isolated D-Bus session without an owned or activatable Secret Service
   provider, status and logout both returned `storage_failure`. Both results
   were explicit errors, not absent-session results or plaintext fallbacks.
5. Two CLI logout operations and the final absence check passed.

A separate safe state check confirmed phase `cleaned` and 11 attempted entry
operations. The approved limit was 13, including two reserved for emergency
cleanup. The test did not use a D&D Beyond account, browser login, or product
credential entry. These feature-build results do not prove release-binary
storage behavior or Linux production login and session use. An independent
query confirmed that the VM resource group and protected local access files
were absent. Azure charges were not measured.

### Windows unavailable-store attempt

On 2026-11-08, a separately approved Windows 11 Enterprise 25H2 x64 VM was
created for a synthetic unavailable-store test. The test stopped before Remote
Desktop sign-in because the VM certificate fingerprint could not be verified.
The cause was not established. No native credential entry operation or
D&D Beyond request ran. The locked and unavailable Windows store cases remain
unverified.

The approved limit was 11 native entry operations, including two reserved for
emergency cleanup. A separate query confirmed that the VM resource group and
protected local access files were absent. The deletion guard stopped after that
check. Azure charges were not measured.

## Remaining acceptance matrix

This matrix separates existing evidence from required work. A passed row
applies only to its stated case and candidate. The new browser installation
results used the retained package; the native-store results used a separate
feature build. The blocked manual groups in CI reports remain unchanged;
later desktop evidence is separate.

| Case | Platform or client | Status | Evidence or blocker |
|------|--------------------|--------|---------------------|
| `package.offline` | Windows x64, Linux GNU x64, macOS ARM64 | passed | [Retained CI reports and archive hashes](#retained-ci-candidate-verification) |
| `desktop.startup` | Windows 11 Enterprise 25H2 x64 | passed | [Windows package results](#windows-package-startup-and-runtime) |
| `desktop.path-lifecycle` | Windows 11 Enterprise 25H2 x64 | passed | [Windows PATH, reinstall, and relocation](#windows-package-startup-and-runtime) |
| `desktop.startup` | Ubuntu 24.04.5 x64, XFCE | passed | [Linux desktop results](#azure-linux-desktop-baseline) |
| `package.linux-inventory` | Ubuntu 24.04.5 x64, noninteractive SSH | passed | [Observed command and runtime inventory](#azure-linux-package-only-follow-up); system Python 3 was present |
| `package.linux-shell-lifecycle` | Linux GNU x64, noninteractive SSH | passed | [Package startup, login-shell PATH, reinstall, relocation, removal, and cleanup](#azure-linux-package-only-follow-up) |
| `desktop.clean-inventory` | Ubuntu 24.04.5 x64 | blocked | Shell inventory is recorded, but a desktop baseline without development runtimes remains unverified |
| `desktop.path-lifecycle` | Linux GNU x64 | blocked | Shell lifecycle passed; a new desktop terminal and desktop settings still need evidence |
| `desktop.clean-install` | macOS ARM64 | deferred | Tracked in #62; retained CI evidence is not a clean desktop |
| `desktop.real-upgrade` | Windows x64, Linux GNU x64 | blocked | Two distinct identified candidates and an approved environment are required |
| `desktop.real-upgrade` | macOS ARM64 | deferred | Tracked in #62 |
| `desktop.trust` | Windows x64, Linux GNU x64 | blocked | Normal browser-download and OS trust behavior has no complete evidence |
| `desktop.trust` | macOS ARM64 | deferred | Tracked in #62 |
| `desktop.minimum-runtime` | Windows x64, Linux GNU x64 | blocked | Tested combinations are known; minimum supported versions are not established |
| `desktop.minimum-runtime` | macOS ARM64 | deferred | Tracked in #62 |
| `native.synthetic-lifecycle` | Windows Credential Manager | passed | [Synthetic native-store result](#synthetic-native-store-result); feature build only |
| `native.release-lifecycle` | Windows Credential Manager, Linux Secret Service | blocked | Packaged-binary store evidence needs disposable identities and exact approvals |
| `native.release-lifecycle` | macOS login Keychain | deferred | Tracked in #62 |
| `native.synthetic-entry` | Linux Secret Service | passed | [Synthetic write, status, and removal](#linux-secret-service-synthetic-follow-up); feature build and user-observed results only |
| `native.synthetic-lifecycle` | Linux Secret Service | passed | [Synthetic lifecycle](#linux-secret-service-complete-synthetic-run), including repeated CLI logout; feature build only |
| `native.synthetic-lifecycle` | macOS login Keychain | deferred | Tracked in #62 |
| `native.locked-unavailable` | Linux Secret Service | passed | [Locked and isolated unavailable service checks](#linux-secret-service-complete-synthetic-run); feature build only |
| `native.locked-unavailable` | Windows Credential Manager | blocked | The [unavailable-store attempt](#windows-unavailable-store-attempt) stopped before entry operations; a new run needs approval |
| `native.locked-unavailable` | macOS login Keychain | deferred | Tracked in #62 |
| `client.registration` | Windows x64, stable VS Code | passed | [Registration follow-up](#stable-vs-code-registration-follow-up) and [client retry](#stable-vs-code-client-retry) |
| `client.discovery` | Windows x64, stable VS Code | passed | [Client retry](#stable-vs-code-client-retry); all eight tools appeared in the real client |
| `client.discovery-lifecycle` | Windows x64, stable VS Code | blocked | Stop, restart, and removal have evidence, but a fresh tool-list fetch after restart was not verified |
| `client.registration` | Windows x64, Copilot CLI | passed | [Offline client](#local-copilot-cli-offline-client); test configuration and synthetic setting only |
| `client.discovery` | Windows x64, Copilot CLI offline | passed | [Offline client](#local-copilot-cli-offline-client); all eight tools before and after restart |
| `client.discovery-lifecycle` | Windows x64, Copilot CLI | blocked | Offline stop, restart, and removal passed; safe-error and full settings isolation remain unverified |
| `client.discovery-lifecycle` | Claude Code | deferred | Tracked in #62 |
| `browser.manual-setup-removal` | Windows regular Chrome, stable Edge | passed | [Browser versions, setup/removal, and cleanup](#manual-browser-installation-result) |
| `browser.host-no-pending` | Windows regular Chrome, stable Edge | passed | [Direct browser-to-host negative checks](#browser-to-native-host-negative-checks); no login or cookie handoff |
| `browser.package-lifecycle` | Windows regular Chrome, stable Edge | blocked | Direct host launch has evidence; fallback, foreign preservation, mismatch/conflict cases, and distinct-candidate updates remain untested |
| `account.client-read` | Windows x64, stable VS Code `book_list` | passed | [One approved Web-library read](#local-vs-code-web-library-read); no model-driven routing |
| `account.client-reads` | Other supported VS Code and Copilot CLI reads | blocked | Each live case needs a new exact account approval and client prerequisites |
| `account.client-reads` | Claude Code | deferred | Tracked in #62 |
| `account.production-login` | Linux and macOS | not_applicable | Not supported by this acceptance scope; Linux implementation is tracked in #52 |

The separate stable VS Code approval permitted at most three MCP server starts,
one unknown-field rejection check, and one model request. The observer recorded
two starts and no model request. D&D Beyond requests and product credential
access were excluded. The separate Copilot CLI test also excluded account reads,
sign-in, and model requests. Other client and account tests need new approval.
The USD 10 total planning ceiling and one-VM limit applied to this run.
This approval does not authorize the other gates below.

## Gate 1: clean desktop

The gates below retain procedures for future separately approved work.
Implement #52 before its new authentication acceptance. Desktop-specific,
browser, and protected-store checks are separate from terminal-only environment
use. macOS desktop and Keychain checks and Claude Code client checks belong to
issue #62, not #34 completion. This revision grants no test approval.

Use a disposable desktop VM or a separate test OS account. A clean-machine claim
also requires no development runtimes installed, not just another account on a
developer machine. Start from a recorded image with no Rust, Python, Node.js,
PowerShell 7, or Visual Studio Build Tools. Use the OS shell for product commands.

An Azure VM can supply a disposable desktop environment. Windows client images
require [eligible dev/test licensing][azure-client]. Select a desktop image,
not Server Core. Record remote desktop access as part of the tested environment;
it does not establish physical-desktop or browser-download trust behavior.
Keep Beyond Gate local in the VM. SSH alone does not establish a Linux desktop
session or an available Secret Service collection.

1. Verify the archive checksum through the installation guide. Record the OS,
   CPU, candidate hash, installed native runtimes, and image origin.
2. Observe the normal download, extraction, and OS trust prompts. Stop on a
   protection denial. Do not remove quarantine, disable Gatekeeper or SmartScreen,
   or change organization policy to get a pass.
3. Extract into an empty stable user-local path with spaces. Run help and version
   for both binaries. Check stdout, stderr, exit codes, and executable permission.
4. On Windows, verify the x64 Visual C++ Redistributable is installed before
   package startup. Record its version. If it is absent, record the startup
   failure, install the supported runtime, and repeat on the same recorded
   image. Do not install Visual Studio or individual downloaded DLLs.
5. Add only the test installation to the test account's PATH. Start a new terminal
   and run both binary names. Record current-terminal and new-terminal behavior.
6. Reinstall the same candidate. For an upgrade, use two separately identified
   real candidates. Stop the test's server, replace the complete package, update
   the extension when applicable, and check versions and registration paths.
   If only one candidate exists, mark the real upgrade blocked.
7. Remove only the test registration, installation, and added PATH entry.
   Check that unrelated settings remain unchanged. Deleting binaries does not
   remove a credential or revoke an upstream session.

Record the lowest OS/runtime combination actually exercised. Dependency logs
and a CI runner name do not establish a minimum OS, glibc, or runtime version.
Older Windows, Intel macOS, other Linux distributions, and unlisted CPUs remain
unverified unless a separate supported matrix and evidence exist.

## Gate 2: native credential services

**Separate explicit approval is required.** Define the exact test identity,
service/entry, read/write/delete operations, operation limits, deadline, and
cleanup before a run. Permit synthetic material only. Do not inspect, replace,
delete, or lock the normal product entry.

| OS | Prerequisite | Isolated test and limit |
|----|--------------|-------------------------|
| Windows | Windows Credential Manager under the approved test account | The existing [isolated native-storage harness](testing.md#isolated-native-storage) uses feature-built binaries and a random synthetic entry. It does not prove release-binary storage behavior. |
| macOS | Available login Keychain in a disposable desktop account | Use only an approved test entry. No portable isolated native-store harness is implemented. Mark execution blocked until the test method and identity are approved. |
| Linux | User session D-Bus and a Secret Service provider with a default collection | Use a disposable desktop account and test collection. The session-only follow-up is not a shipped native-store harness. Production login/session use remains tracked in [#52](https://github.com/PlagueHO/beyond-gate/issues/52). |

Check absent state, synthetic write/read, stored-unverified status, logout,
repeated logout, and final absence. A stored entry is not current upstream
validation. Check locked and unavailable service states only in the disposable
identity. Never lock the user's normal store or stop a shared service.
Use normal OS controls. An unavailable or locked service must return an explicit
error, not an absent-session result or a plaintext fallback.

Do not force a Windows lock-state test by locking the desktop or changing shared
services. If the OS cannot provide a safe isolated failure state, record that
case as blocked. In-memory failure tests are synthetic evidence only.
Check cleanup after both success and failure. If forced termination prevents
cleanup, record the owned test identity and use its approved cleanup procedure.
Never enumerate credentials to find a replacement identity.

## Gate 3: real MCP clients

**Separate explicit approval is required.** Prefer a disposable OS account.
A client-specific profile is suitable only when current official documentation
shows that it isolates all relevant settings and state. A temporary `HOME` alone
is not proof of isolation. Do not overwrite or import normal client settings,
sessions, tokens, or approvals. If isolation is unavailable, mark the run blocked.

Check instructions against the exact installed version:
[VS Code configuration][vscode], [Claude Code MCP][claude], and
[Copilot CLI MCP][copilot]. Client accounts, policy access, and sign-in are
separate prerequisites, not Beyond Gate runtime requirements.
Do not start sign-in or a model request without an approval that covers it.

For each client, preview `mcp install`, confirm the absolute packaged sibling
path, and register it in the isolated account. Check that unrelated synthetic
settings remain intact. Observe the actual client, not only its JSON file:

| Client | Discovery surface | Configuration to verify |
|--------|-------------------|-------------------------|
| VS Code | MCP server list in the approved Copilot session | Copilot Global portable `mcpServers` settings |
| Claude Code | `claude mcp list` and its MCP tools view | Isolated user-level `.claude.json` |
| Copilot CLI | `/mcp` dashboard | Isolated `COPILOT_HOME` `mcp-config.json` |

Confirm all twenty tools: `auth_validate`, `character_list`, `character_show`,
`campaign_list`, `campaign_show`, `book_list`, `book_outline`, `book_read`,
`class_list`, `class_show`, `monster_list`, `monster_show`, `background_list`,
`background_show`, `species_list`, `species_show`, `spell_list`, `spell_show`,
`feat_list`, and `feat_show`. Earlier retained
eight-tool evidence does not establish real-client acceptance for the later
class, monster, background, species, spell, and feat tools.
Discovery alone does not permit calling `auth_validate` or valid resource operations.
If the client supports an exact tool call without a model request, check one
unknown-field rejection only after verifying that the selected candidate rejects
that input before credential or network access. For current resource tools,
`character_list` with `{"unexpected": true}` is one such input.
Do not use a valid character, campaign, class, monster, background, book, or
`auth_validate` call as an account-free check. Synthetic identifiers do not make
valid reads account-free.
If an approved exact-call method or verified rejection contract is unavailable,
record that tool-call case as blocked.
Verify the explicit error and absence of account data.

Stop the server through the client. Verify its owned child exits. Restart and
check discovery again. Remove only the `beyond-gate` entry and check that it
disappears without changing other servers. Record shutdown, restart, and removal
as separate results. Synthetic marker commands do not count as real clients.

## Gate 4: manual browser and native-host setup

**Separate explicit approval is required for browser and host changes.**
Run Chrome and Edge as separate Windows cases in an isolated OS account.
Use a fresh regular profile. Native-host registration is per OS user, not per
browser profile. A fresh profile alone does not isolate registry changes.
Do not use the development worktree as the stable installation.

Record the exact Windows build, browser flavor/version, package hash, CLI/native
host version, extension version, and extension ID. Use the
[manual installation guide](installation.md#package-matrix) and
[setup commands](cli.md#auth-install-and-auth-uninstall).

1. Extract and manually load the packaged extension. Check its displayed
   version and ID `gnhdconfgjfdbbgkemadegjcjnnodefh`.
2. Preview and confirm setup for the selected browser with a stable path with
   spaces. Check matching host executable, allowed origin, and manifest path.
   Confirm that another install or conflicting mapping is not overwritten.
3. Repeat setup and confirm that it makes no change. Record mismatched
   extension/host or stale-path refusal using isolated test files only.
4. Update the complete candidate, reload the unpacked extension, and recheck all
   versions and mappings. Do not infer update coverage from a single candidate.
5. Remove matching host setup with the same directory. Check unrelated browser,
   machine-wide, and host settings. Explicit Edge removal can still leave Chrome
   fallback discovery; record that separately instead of claiming total removal.
6. Remove the test extension and owned files. Verify cleanup without inspecting
   unrelated profiles or registry entries.

These are installation checks, not login tests. Do not call the cookie API or
start account login under this gate. Stable Chrome, stable Edge, and Edge Dev
are distinct evidence rows. Windows browser results do not establish macOS or
Linux browser support. No Store listing or automatic update service is required.

## Gate 5: account login and supported reads

**A new exact approval is required.** Previous approvals are consumed.
Define permitted hosts, resource IDs, operations through each adapter/client,
credential-load limit, total HTTP attempt limit, deadlines, response bounds,
stop conditions, and cleanup. Do not put credential values in the approval.
Stop on access denial, throttling, unexpected content, or a protection prompt.
Do not retry or select another resource unless the approval permits it.

Current product support includes Windows library discovery and bounded book
reads, plus fixed, bounded character and campaign read operations. Character and
campaign behavior has synthetic process coverage, but no live acceptance.
Existing one-chapter
initial-chunk evidence does not prove general compatibility, live continuation,
or real sharing. Do not automatically follow continuation tokens.
Keep live results separate from package checks, synthetic fixtures, and
native-store-only results. Retain safe counts and parity flags, not content.

[vscode]: https://code.visualstudio.com/docs/agents/reference/mcp-configuration
[claude]: https://code.claude.com/docs/en/mcp
[copilot]: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers
[azure-client]: https://learn.microsoft.com/en-us/azure/virtual-machines/windows/client-images
