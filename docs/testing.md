# Manual capability tests

**Infrastructure only: live account reads remain blocked.** The shared suite runs
the real CLI and MCP executables. It does not replace their read operations with
fixtures. Current resource commands and tools return unsupported errors.

The [issue #27 plan](https://github.com/PlagueHO/beyond-gate/issues/27) permits this
blocked-first stage. It does not implement production authentication or live
reads. A supplied cookie cannot make those missing operations work.

## Results and coverage

[`capabilities.json`](../scripts/e2e/capabilities.json) lists commands, argument
names, output formats, completion shells, installation clients, MCP tools, and
live assertions. The suite expands these into individually reported cases.
CLI parser tests and real MCP discovery detect inventory changes.

| Result | Meaning |
|--------|---------|
| `passed` | The selected assertion ran and passed. |
| `failed` | An assertion or required infrastructure operation failed. |
| `blocked` | A prerequisite is absent, or the case did not run. |
| `not_applicable` | A partial selection explicitly excludes the case. |

A `contract.*` pass proves the current unsupported-error contract only.
Corresponding `live.*` cases remain blocked. Help, successful authentication
probes, unsupported errors, and empty results do not prove live read coverage.
Testing the browser-login blocker does not prove a working browser handoff.

`full` returns a nonzero exit while any required case is not passed.
`contracts` selects offline command, protocol, and registration contracts. It
excludes native storage and live reads. It can pass but always reports
`full_coverage: false`. A missing inventory case, duplicate case, or failed
selected contract cannot produce full success.

## Run locally without credentials

Use PowerShell 7.4 or later. Start at the repository root.

```powershell
pwsh -NoProfile -File .\scripts\start-live-e2e.ps1 -Selection contracts
pwsh -NoProfile -File .\scripts\start-live-e2e.ps1 -Selection full
```

The first command can pass without credentials. The second must currently fail
with explicit blocked live cases. Neither command requests a cookie, exchanges
a token, opens the native credential store, or reads real client settings.
Build failure stops execution.

The launcher uses `scripts\cargo.ps1` for every Cargo invocation. On Windows,
it selects Visual Studio 2022 Build Tools in the same process as Cargo.
Executables built with `e2e-isolation` go into `target\manual-e2e`, not the normal
binary directory. PowerShell remains development tooling, not a product runtime.

To run already built binaries and save only safe case results:

```powershell
pwsh -NoProfile -File .\scripts\test-e2e.ps1 `
    -BinaryDirectory .\target\manual-e2e\debug -Selection contracts `
    -ReportPath .\target\capability-summary.json
```

Raw stdout and stderr stay in bounded memory. Reports contain fixed case names,
classifications, and a zero live request budget. They do not contain child
arguments, exception details, credentials, account names, resource identifiers,
private URLs, response bodies, or book text. No raw transcript is uploaded.

## Isolated native storage

Native storage tests require separate explicit credential-access approval.
Do not run the following command merely because a plan or PR was approved:

```powershell
pwsh -NoProfile -File .\scripts\start-live-e2e.ps1 `
    -Selection full -ApproveNativeStorage
```

This tests only synthetic bytes in Windows Credential Manager. It does not
perform live authentication. Other platforms report native cases as blocked.
Windows-hosted execution itself is not proof that its credential service works.
Only a successful approved native run supports that claim.

The off-by-default `e2e-isolation` feature changes only credential identity.
The fixed service is `beyond-gate-e2e`. The account is a random 32-character
lowercase hexadecimal run context. `BEYOND_GATE_E2E_CONTEXT` carries that
non-secret context. `BEYOND_GATE_E2E_NATIVE_APPROVED=1` is also required before a
test build opens storage. Missing or invalid context fails before native access.
There is no fallback to the product `beyond-gate` / `local-session` entry.

The suite checks both adapters and the feature-required `e2e-session` helper
before enabling storage. The hidden `--e2e-context-check` handshake exists only
in feature builds. The helper accepts `check`, `seed`, or `cleanup`; it has no
credential-import input and seeds only synthetic bytes. Normal builds retain
their current credential identity and have no test handshake.

Tests check absent status, stored-unverified status, logout, and repeated logout.
They delete only their own isolated entry. This proves feature-built process
behavior, not native-store behavior of an unmodified release binary.

If abrupt termination prevents cleanup, use the context printed before native
access or in the cleanup warning. In the same approved Windows account, set the two non-secret
test variables and run only the feature helper:

```powershell
$env:BEYOND_GATE_E2E_CONTEXT = '<the 32-character test context>'
$env:BEYOND_GATE_E2E_NATIVE_APPROVED = '1'
.\target\manual-e2e\debug\examples\e2e-session.exe cleanup
Remove-Item Env:\BEYOND_GATE_E2E_CONTEXT
Remove-Item Env:\BEYOND_GATE_E2E_NATIVE_APPROVED
```

Never delete the product entry as test cleanup. A forced OS shutdown can also
leave a uniquely named `Beyond Gate E2E <context>` temporary directory. Inspect
that exact directory before removing it. Do not remove a broad temporary root.

## Manual GitHub Actions

The [manual workflow](../.github/workflows/live-e2e.yml) has only
`workflow_dispatch`. It has no push, PR, schedule, or reusable-workflow trigger.
The default selection is `full`, so missing live coverage cannot look successful.

Run reviewed `main` code only. The workflow rejects other repository/ref
combinations and repeated run attempts. It checks out the dispatched commit.
A new attempt needs a new explicit dispatch and submission identifier.
The workflow must first exist on `main`; a local workflow file does not register
it for dispatch. See GitHub's
[`workflow_dispatch` reference](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#onworkflow_dispatch).

The workflow uses `windows-latest`, as requested. This currently selects Windows
Server 2025, but the label can move. It records the actual OS/image separately.
It checks for or installs Visual Studio 2022 Build Tools before Cargo.
It does not use a newer Visual Studio installation as a replacement.
See the [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

To dispatch the current credential-free subset from an authenticated GitHub CLI:

```powershell
pwsh -NoProfile -File .\scripts\dispatch-live-e2e.ps1 -ContractsOnly
```

You need permission to dispatch Actions on `PlagueHO/beyond-gate`.
The helper fixes the repository and `main` ref. It generates a unique non-secret
submission identifier. It does not read or upload an account cookie.
Dispatch acceptance is not a successful test result.

For the GitHub UI path, select **Actions > Manual capability tests > Run
workflow**. Select `main`. Generate an identifier with
`[guid]::NewGuid().ToString('N')`. Enter it as `submission_id`.
Select `contracts` for offline-only execution. Read and select the acknowledgment.
Leave native approval false unless that separate store test is approved.
Updating a secret and clicking **Run workflow** are separate operations; the
current blocked workflow requires no account secret.

Only case names and classifications enter the job summary. The workflow uploads
no account artifacts and uses no account secret. The job limit is 30 minutes.
Each process has a 20-second limit, each MCP session has a 30-second limit, and
the suite has a 180-second execution limit after compilation.

Child-tree cleanup is best effort. A forced OS shutdown, process termination,
or parent crash can prevent cleanup. The harness kills only its own active
process trees. It does not search for or stop other processes by name.

## Fresh credentials: disabled until production prerequisites exist

`dispatch-live-e2e.ps1` without `-ContractsOnly` exits nonzero before hidden input
or GitHub secret access. There is deliberately no readiness override.
The workflow does not resolve any account secret. Do not create one for this
stage.

The tested future submission boundary uses this design:

1. Verify trusted code, production session use, selected resources, exact
   hosts/endpoints, supported transport, and enforced request limits.
2. Request fresh `CobaltSession` through hidden input in an unrecorded interactive
   terminal. Never collect it in chat, arguments, source, fixtures, workflow
   inputs, or MCP input.
3. Generate one secret name, `BEYOND_GATE_E2E_<submission ID>`. Do not reuse a
   slot. Stop if it exists. Upload with `gh secret set` stdin, never `--body`
   or a plaintext file.
4. Dispatch with only the non-secret identifier. Stop if upload or dispatch
   fails. Do not retry an uncertain dispatch.
5. Match only that identifier's run. Bound queue waiting to 120 seconds and
   total helper waiting to 600 seconds. On failure, cancel the identified run
   where possible. Delete only the submission's own secret.

The lifecycle functions are tested with stubbed `gh` responses and synthetic
cookies. They are not an enabled live path. No product cookie or bearer format
is selected here. The cookie and exchanged bearer token remain different
credentials. Do not assume either lifetime.

Future UI use must create a new uniquely named repository Actions secret
immediately before a separate dispatch. A repository writer needs the
permissions required by GitHub to manage Actions secrets and dispatch runs.
The workflow token remains read-only; it does not gain secret-management rights.
No cleanup PAT or repository policy change is added.

The local helper owns cleanup. After an interrupted helper, inspect the matching
run and delete only its recorded secret:

```powershell
gh secret delete BEYOND_GATE_E2E_<SUBMISSION_ID> --repo PlagueHO/beyond-gate
```

Repository secrets are available to eligible workflows during their retention
window. Unique names prevent accidental replacement by later submissions, not
access by trusted repository writers. GitHub reads repository secrets when a run
is queued. Deleting a secret does not revoke a queued run's copy or revoke the
upstream cookie. Concurrency alone does not bind a mutable secret to a dispatch.
See [Actions secrets](https://docs.github.com/en/actions/reference/security/secrets)
and [GitHub CLI secret input](https://cli.github.com/manual/gh_secret_set).

Secret values stay in memory where possible. The BSTR used by the helper is
cleared. .NET strings and OS or tool copies cannot be guaranteed to be erased.
No plaintext file, product environment credential input, public import command,
or MCP credential parameter is added.

## Live prerequisites and limits

The live request budget is **zero**. No authentication host, resource host, or
live operation is enabled in this suite. Current unsupported-contract processes
do not make account requests. The ignored authentication probe remains separate
and requires its own approval.

Future activation needs verified production exchange and session use in both
adapters, exact HTTPS hosts and paths, schemas, and a narrow test-only transport.
Define an approved total budget across CLI and MCP, not merely a count of child
processes. Enforce actual requests, including fresh access checks and retries,
at the production request boundary. Synthetic budget tests cannot prove
enforcement in a production path that does not exist.

Require one explicitly selected character, campaign, book, and section from an
approved test account. Define the required book access and continuation data
before a run. Keep selections in memory; do not commit snapshots or identifiers.
Missing required resources must be blocked or failed, never passed.

Retain the 1,000-entry outline, 4,096 UTF-8 byte section, and 16 KiB continuation
limits. Continue only the explicitly selected section within the total budget.
Do not enumerate books or cache their content.

Stop on rejected or expired credentials, access denial, rate limiting, unexpected
redirects, or bot protection. Do not change permissions or provoke upstream
failures. Keep rejection, unavailable-service, malformed-data, and expiry tests
deterministic and offline. Do not automatically refresh or retry authentication.

## Offline validation

```powershell
pwsh -NoProfile -File .\scripts\test-e2e-harness.ps1
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-cli --test inventory --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test --workspace --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test --workspace --all-features --locked
```

Ordinary Rust tests use synthetic stores and loopback fixtures, not native
storage or live accounts. The script checks bounded output and input, protocol
errors, process and suite deadlines, cleanup, report redaction, inventory
omissions, failure propagation, readiness, and submission lifecycle failures.
It does not dispatch a workflow or upload a secret.
Ordinary CI runs these synthetic harness checks on its platform matrix. It does
not run native-store cases or the manual account suite.

Run the other [workspace checks](../CONTRIBUTING.md#workspace-checks) and
[Markdown checks](../CONTRIBUTING.md#markdown-and-local-links) after changes.
