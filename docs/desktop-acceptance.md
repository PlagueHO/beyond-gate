# Desktop acceptance

**Status: offline checks and manual procedures, not full desktop acceptance.**
Issue [#34](https://github.com/PlagueHO/beyond-gate/issues/34) stays open until
the required desktop evidence exists. A package check, synthetic registration,
or development-machine run is not a clean desktop result.

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
and outside the temporary installation. Without `-ReportPath`, existing command
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
Linux/macOS execution, hosted CI, clean desktops, minimum runtimes, real upgrades,
and all separately gated checks remain unverified for this change.

## Gate 1: clean desktop

Use a disposable desktop VM or a separate test OS account. A clean-machine claim
also requires no development runtimes installed, not just another account on a
developer machine. Start from a recorded image with no Rust, Python, Node.js,
PowerShell 7, or Visual Studio Build Tools. Use the OS shell for product commands.

1. Verify the archive checksum through the installation guide. Record the OS,
   CPU, candidate hash, installed native runtimes, and image origin.
2. Observe the normal download, extraction, and OS trust prompts. Stop on a
   protection denial. Do not remove quarantine, disable Gatekeeper or SmartScreen,
   or change organization policy to get a pass.
3. Extract into an empty stable user-local path with spaces. Run help and version
   for both binaries. Check stdout, stderr, exit codes, and executable permission.
4. On Windows, record the installed x64 Visual C++ Redistributable version.
   If required, use Microsoft's supported installer and repeat on the same
   recorded image. Do not install Visual Studio or individual downloaded DLLs.
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
| Linux | User session D-Bus and a Secret Service provider with a default collection | Use a disposable desktop account and test collection. No portable isolated native-store harness is implemented. Production login/session use remains tracked in [#52](https://github.com/PlagueHO/beyond-gate/issues/52). |

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

Confirm all eight tools: `auth_validate`, `character_list`, `character_show`,
`campaign_list`, `campaign_show`, `book_list`, `book_outline`, and `book_read`.
Discovery alone does not permit calling `auth_validate` or valid book operations.
If the client supports an exact tool call without a model request, check one
unsupported character/campaign operation or a book call with an unknown field.
Otherwise record that tool-call case as blocked pending an approved method.
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

Current product support is Windows library discovery and bounded book reads.
Character and campaign live reads remain unsupported. Existing one-chapter
initial-chunk evidence does not prove general compatibility, live continuation,
or real sharing. Do not automatically follow continuation tokens.
Keep live results separate from package checks, synthetic fixtures, and
native-store-only results. Retain safe counts and parity flags, not content.

[vscode]: https://code.visualstudio.com/docs/agents/reference/mcp-configuration
[claude]: https://code.claude.com/docs/en/mcp
[copilot]: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers
