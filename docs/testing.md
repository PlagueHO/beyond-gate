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
Testing redirected-login refusal does not prove a working browser handoff.

`full` returns a nonzero exit while any required case is not passed.
`contracts` selects offline command, protocol, and registration contracts. It
excludes native storage and live reads. It can pass but always reports
`full_coverage: false`. A missing inventory case, duplicate case, or failed
selected contract cannot produce full success.

## Browser handoff offline checks

### Shared session lifecycle checks

Portable core tests use isolated in-memory credential entries, actual loopback
exchanges, and controlled clocks. They cover fresh validation, cached reuse,
exact bearer expiry, rejection without retry, invalid formats, storage failures,
concurrent acquisitions, cancellation, deadlines, logout, replacement, and
redacted error chains. A synthetic resource server checks bearer-only headers
and distinct HTTP 401/403 meanings through the bounded HTTP client.

The `session_process` test starts actual CLI and rmcp stdio child processes.
Both use the same compiled invented scenarios and shared core implementation.
It checks equal validation data and diagnostics, exit codes, strict empty tool
input, MCP cancellation, and EOF cleanup. Protocol waits are bounded. The
`session-fixture` development feature has no product argument, environment
selector, or MCP input. It cannot accept a live cookie or service URL.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib auth::session --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-mcp --test session_process --locked
```

No native credential-store entry or real account is accessed. Windows
production validation and reuse are implemented; Linux/macOS calls fail before
credential or service access. Linux implementation is tracked in
[#52](https://github.com/PlagueHO/beyond-gate/issues/52).
These checks do not prove live resource access or upstream revocation.
The manual suite discovers `auth_validate` and tests rejected extra fields.
It does not call valid production validation within its zero live request budget.

### Windows runtime completion checks

The actual native host now has
[process refusal tests](../src/beyond-gate-cli/tests/native_host.rs). They check
origin rejection before input, missing consent, invalid key/fields, legacy hello
rejection, oversized framing, and the fixed ten-second stalled-input cap.
All process inputs fail before a product-pipe connection. A valid pairing is
not tested against another user's pending product login.

The core relay tests use actual Noise and mock CLI claims. They check both
declared browsers, final acknowledgement, cancellation forwarding, a late-save
claim after cancellation, and uncertain stalled outcomes. These claims do not
prove service validation or native save.

The establishment tests use actual paired receipts, a local HTTP service, and
in-memory keyring entries. They check real validation/save completion,
rejection, lost/wrong acknowledgements, buffered cancellation before any request,
disconnect, early acknowledgement, cancellation before and during writes,
reservation retention, and the exact five-second save wait. A delayed-worker
regression checks write admission without polling the async timeout. No native credential
entry or live account is used. CLI process tests require redirected login to
fail before runtime, key generation, endpoint work, or account access.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib auth:: --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-cli --test cli --test native_host --locked
```

These checks alone do not prove real browser integration, hosted CI evidence,
or approved live/save behavior.

The manual browser form now has twelve
[offline product-controller tests](../beyond-gate-browser-extension/tests/flow.test.mjs).
The [production recipes](fixtures/windows-login-v1.json) are shared with
[Rust codec checks](../src/beyond-gate-core/tests/windows_login_contract.rs) and
the actual Noise relay tests. The package's public manifest key derives the
expected extension ID. Tests require consent, regular context, exact native
cookie limits, one query, final acknowledgement, safe cancellation, and fixed
phase deadlines. Existing synthetic-browser tests remain separate.

```powershell
npm test --prefix .\scripts\browser-tests
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --test windows_login_contract --locked
```

Windows setup tests use private synthetic registry roots and invented files.
They cover preview, cancellation, idempotent install/removal, foreign setup,
malformed manifests, and transactional removal with concurrent changes.
They do not register a product host or open protected credentials.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-cli --lib browser_setup --locked
```

Synthetic UI rendering was checked in a fresh headless Chrome profile at desktop
and narrow widths, in light and dark modes, and with unavailable native access.
The key cleared before connection, failure text omitted the raw API error, and
neither layout had horizontal overflow. The temporary profile and browser were
removed. This check used a mocked native API and no cookie API, registration,
account request, or native credential entry. It is not end-to-end browser login.

#### Windows product-flow process checks

The [product-flow runner](../scripts/browser-tests/product-flow.mjs) loads the
pinned browser package into a fresh regular Chrome or Edge profile. It calls the
actual browser controller through real native messaging. Two Rust processes use
the actual relay, private Windows pipe, Noise pairing, pending authorization,
shared validation/save controller, and final acknowledgement sequence.

The runner replaces the cookie API with the shared invented recipe. It never
calls the real cookie API. It redirects only the native-host name to one unique
synthetic registration. The fixture connects only to a random fixture namespace,
not the product endpoint. The
[feature-gated Rust fixture](../src/beyond-gate-core/src/auth/windows_login_fixture.rs)
rejects other cookie values and uses a local mock service and explicit in-memory
store. The normal CLI and native host have no fixture argument or runtime mode.
Do not register this fixture under the product host name.

```powershell
node .\scripts\browser-tests\product-flow.mjs --browser chrome
node .\scripts\browser-tests\product-flow.mjs --browser edge
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib fixture::non_fixture_cookie --features windows-login-fixture --locked
```

Local Chrome `156.0.8078.74` and Edge `156.0.4314.50` passed synthetic success,
service rejection, cancellation, lost acknowledgement, and unavailable storage.
Cancellation
did not save. A lost acknowledgement reported `save_indeterminate` after a
confirmed in-memory write. Each run removed its owned registration and profile.
Unavailable storage did not write an entry.
The Windows CI browser matrix now runs this fixture separately from the earlier
synthetic receipt-only tests. Hosted results are not yet recorded.

These checks do not prove the production CLI's interactive terminal path, real
account validation, native protected saving, or restart/session use.
Those checks still need a fresh approved scope.

#### Windows native-host setup checks

Six Windows unit checks use unique private registry roots and invented files.
They cover preview/cancellation, both-browser repeat installation, owned removal,
foreign entries in both hives/views and Edge fallback locations, changed files,
wrong registry shape/types, missing binaries, source-tree refusal, and registry
read failure. They do not create a product native-host registration.
The CLI process checks also reject source-tree setup before registry access.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-cli --lib browser_setup --test cli --locked
```

The [Rust fixture checks](../src/beyond-gate-core/tests/browser_handoff_fixture.rs)
read the [shared synthetic data](fixtures/browser-handoff-v1.json). They check
canonical message shapes, proposed bounds, unique case IDs, message references,
and expected error effects.

The [protocol tests](../src/beyond-gate-core/tests/browser_handoff_protocol.rs)
execute the real shared message parsers, frame readers, and frame writers.
They cover selected fixture recipes, exact and excess byte bounds, both prefix
encodings, duplicate and unknown fields, null transaction presence, frame counts,
monotonic deadlines, cancelled writes, and safe error chains.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --test browser_handoff_fixture --test browser_handoff_protocol --locked
```

These checks do not test authenticated process communication, login state
transitions, browser permissions, service validation, or protected writes.
They require no browser, native credentials, or account requests. A pass does
not complete [issue #29](https://github.com/PlagueHO/beyond-gate/issues/29).

### Encrypted pairing checks

The [pairing tests](../src/beyond-gate-core/tests/browser_handoff_pairing.rs)
exercise actual Noise handshakes and encrypted transport with invented 32-byte
keys. The fixed pattern requires PSK possession and uses a versioned application
context. Tests reject unfinished pairing, wrong keys, early handshake payload,
tampering, replay, reordering, and cross-channel messages. They also check exact
plaintext/tag bounds and redacted cryptographic causes.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --test browser_handoff_pairing --locked
```

All thirteen checks passed locally. An owned pending ticket rejects an expired
claim and is consumed by one claim; a compile-fail doctest rejects reuse.
The primitive does not implement user pairing delivery, OS-wide pending-login
control, OS identity, IPC, or adapter ingress.
It does not complete the authenticated handoff requirement.

### Encrypted stream and process checks

The [stream tests](../src/beyond-gate-core/tests/browser_handoff_transport.rs)
check the fixed encrypted envelope, real Noise pairing, independent frame
budgets, expiry, malformed and tampered input, replay, and cancellation.
All sixteen passed locally on Windows.

Build the [synthetic helper](../src/beyond-gate-core/examples/paired-handoff-fixture.rs)
before running the
[separate-process tests](../src/beyond-gate-core/tests/paired_process.rs):

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 build -p beyond-gate-core --example paired-handoff-fixture --example pending-slot-fixture --example browser-handoff-fixture --features browser-fixture --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --test browser_handoff_transport --test browser_handoff_flow --test paired_process --features browser-fixture --locked
```

All three process checks passed locally. They require synthetic receipt without
validation/save, explicit wrong-key rejection, and wrong-transaction rejection.
Authorization now comes from the core controller with a generated public
transaction, not an injected fixture reply. The helper uses compiled
invented keys, IPv4 loopback only, and a ten-second process lifetime. Missing
helper binaries fail feature-enabled process tests; binaries are resolved from
the test executable's target directory, not `PATH`.

The feature is development-only. These checks make no account requests and
use no browser or credential store. The Rust CI matrix defines the explicit
helper build and test selection on all three operating systems. Hosted results
remain unverified. Neither a generic stream nor loopback fixture establishes
OS-isolated product IPC, real pending-login authorization, or product login.

The latest local Windows run also passed workspace formatting, all-targets
Clippy, serial workspace tests, doctests, private-item rustdoc, and the normal
workspace build. Clippy, tests, doctests, and rustdoc included
`beyond-gate-core/browser-fixture`. Live authentication tests stayed ignored.
Three existing offline authentication checks first failed because Windows
accepted sockets inherited nonblocking mode. Resetting the synthetic mock
stream to blocking mode preserved its five-second I/O limits. All nine
offline probe checks and the workspace rerun then passed.

### Generated keys and bound application flow

`PairingKey::generate` uses the system randomness source for exactly 32 bytes
and has no fallback. Two unit checks cover size and redacted key/error output.
`Transaction::generate` creates a separate 128-bit public correlation value.
The tests do not measure entropy or implement user key delivery.

The [flow tests](../src/beyond-gate-core/tests/browser_handoff_flow.rs) use real
paired streams and generated keys. All eleven passed locally. They cover
consent, core-generated authorization, one bound receipt without login success,
wrong correlation, invalid sequencing, forged storage/success claims, expiry,
hello/submission limits, and cancellation. Compile-fail doctests reject a second
key transfer and a second receive on the same controller.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --test browser_handoff_flow --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib randomness_tests --locked
```

The controller requires an already paired connection tied to the correct ticket.
It does not own an OS-wide slot, process unauthenticated connection attempts,
query a browser, validate an account, save material, or send a success result.

### Interactive terminal pairing checks

The [display module](../src/beyond-gate-cli/src/pairing.rs) has six unit checks for
all standard-stream terminal gates, process-local ownership, canonical encoding,
key bounds, readable plain text, safe error causes, and deadlines.
The [process check](../src/beyond-gate-cli/tests/pairing_terminal.rs) requires
the explicitly built synthetic helper and checks nonzero exit, empty stdout,
and the exact refusal diagnostic under redirection.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 build -p beyond-gate-cli --example pairing-terminal-fixture --features pairing-fixture --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-cli --lib --test pairing_terminal --features pairing-fixture --locked
```

All six display checks and the process check passed locally on Windows.
The Rust CI matrix defines the helper build and refusal test on all three
operating systems. Hosted results remain unverified.

The app terminal correctly refused the preview because the native process
streams were not all terminals. A separate owned Windows console completed
the five-second invented-key preview, rendering and explicit close, with exit
code 0. The first native attempt failed because the owned handle lacked read
access for Windows console-mode checks; the corrected handle passed.
No generated pairing key, account input, browser query, or credential-store
operation was used. Linux/macOS rendering and pixel-level visual review remain
unverified. Terminal recordings and OS copies have no erasure guarantee.

### Windows pending-login reservation checks

The [reservation module](../src/beyond-gate-core/src/auth/handoff/slot.rs) has
seven Windows unit checks. They use separate synthetic namespaces, not the
product slot. Actual handle opens must reject read, write, pipe-instance
creation, `READ_CONTROL`, `WRITE_DAC`, `WRITE_OWNER`, and generic read/write
access with Windows error 5. Other checks cover logon SID syntax, separate
namespaces, exclusivity, drop cleanup, foreign-object refusal, and safe errors.

Three Windows checks in the existing
[process suite](../src/beyond-gate-core/tests/paired_process.rs) require a
feature-built helper. They test two competing processes, normal holder exit,
and termination of only the owned holder process. A fresh process must acquire
the released namespace. Fixed metadata never claims validation or save.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib auth::handoff::slot --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 build -p beyond-gate-core --example paired-handoff-fixture --example pending-slot-fixture --example browser-handoff-fixture --features browser-fixture --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --test paired_process --features browser-fixture --locked
```

The reservation has no account, browser, credential-store, or network request.
All seven unit checks and all three new process checks passed locally on Windows.
Its pipe never accepts a client and is not the native-host endpoint.
Non-Windows platforms return an explicit unsupported error and have a unit
check for that behavior. CI builds the reservation helper only on Windows.
Other-user/logon execution and hosted CI results remain unverified.

### Windows single-instance endpoint checks

The [endpoint module](../src/beyond-gate-core/src/auth/handoff/endpoint.rs) has
eight offline Windows checks. They use only isolated synthetic namespaces.
They check denied owner-control opens and actual extra-server creation with a
spare instance slot. They also check real data exchange, missing endpoints,
deadlines, cancellation, bounded drop cleanup, and redacted Debug output.
The reservation suite adds a safe one-instance owned-handle transfer proof.
A compile-fail endpoint example checks the reservation lifetime.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib auth::handoff --locked -- --test-threads=1
pwsh -NoProfile -File .\scripts\cargo.ps1 build -p beyond-gate-core --example paired-handoff-fixture --example pending-slot-fixture --example browser-handoff-fixture --features browser-fixture --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --test paired_process --features browser-fixture --locked -- --test-threads=1
```

All 18 selected handoff unit checks and all nine process checks passed locally.
Three process cases use actual Windows pipes for receipt, wrong-key rejection,
and wrong-transaction rejection. Receipt always reports no validation or save.
No browser, account, product credential store, or network request is used.
Hosted CI and other-logon execution remain unverified.

### Negative-authentication evidence

The [negative-only probe](../src/beyond-gate-core/tests/auth_negative.rs) has nine
offline checks and two ignored live tests. Run the default offline selection:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --test auth_negative --locked
```

All nine offline checks passed. Both live tests remain ignored by default.
It has no real-cookie input or account access. It sends at most two fixed
negative cases, records HTTP statuses only, and never reads response bodies.
An unexpected status or transport failure stops execution without retry.

The [exact scope](browser-handoff.md#exact-negative-authentication-check-scope)
must be approved before using the following command. It is not an ordinary
offline check and must not be added to CI:

```powershell
$env:BEYOND_GATE_NEGATIVE_AUTH_APPROVED = 'absent-and-synthetic-v1'
$checkExitCode = 1
try {
    pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --test auth_negative --locked -- --ignored --exact live_negative_authentication --nocapture
    $checkExitCode = $LASTEXITCODE
} finally {
    Remove-Item Env:\BEYOND_GATE_NEGATIVE_AUTH_APPROVED
}
if ($checkExitCode -ne 0) { throw 'Negative-authentication check stopped or failed.' }
```

Do not enable `live_authentication` as a side effect. Negative status evidence
does not prove that a real session is valid or that storage works.

After exact scope approval, the single absent-cookie request returned HTTP 200.
The live test failed and stopped as designed. It did not send the invented-cookie
request or read any response body. The result does not establish whether the
response is a guest, a challenge, or an authenticated response. Another request
or body inspection needs a new exact scope; do not rerun the failed check.

The user subsequently approved one separate absent-cookie JSON-shape observation.
That one request returned HTTP 200 with `token_present=false` and
`ttl_positive=true`. The observation passed, but authenticated usability remains
unverified. Only those fixed Booleans and the status were reported. No token,
raw body, claims, or account data was printed or saved.
The owned body was bounded at 64 KiB and zeroizing.

The separate test is `live_unauthenticated_response_shape` and its matching
scope flag is `absent-json-shape-v1`. Never substitute the status-only scope
flag, run all ignored tests, or repeat either live scope automatically.

### Scoped validation-field comparison

The developer-only
[probe module](../src/beyond-gate-core/tests/support/auth_validation_evidence.rs)
adds seven offline checks and three ignored live entries to `auth_probe`.
All 16 offline probe checks passed. All four live probe entries remain ignored
by default. The new entries need a distinct exact-scope approval flag and
terminal stdin/stderr before a prompt or request. They are not product
cookie-import APIs.

The user approved at most three direct POST requests to
`https://auth-service.dndbeyond.com/v1/cobalt-token`: absent cookie, invented
invalid cookie, then one real `CobaltSession` through a hidden local terminal.
Each request has a fifteen-second deadline and a 64 KiB response limit.
No retry, redirect, proxy, content read, other host, or store access is allowed.
Denial, throttling, unexpected content, malformed token encoding, or invalid
approved fields stop all subsequent cases. Empty hidden input stops the third
request. The owned console has a four-minute bound, including human input.

Only fixed classifications are reported for approved login/anonymous and expiry
fields. Identifier values, cookies, tokens, unknown fields, and raw responses
never enter reports or files. Owned cookie, response, and decoded payload buffers
are zeroizing. Transport and parser internals have no complete memory-erasure
guarantee. The test decodes JWT payloads only for field evidence; it does not
verify signatures or declare service validation. Every report explicitly states
`account_validated=false` and `saved=false`.

The first run stopped after one absent-cookie HTTP 200 JSON response without
a token. The probe incorrectly required a token from that negative case.
This was a probe error: the earlier recorded negative shape also had no token.
Missing or null tokens are now negative observations, never usable real login.
The remaining entry skips the already spent absent-cookie request and permits
only the two unspent cases. Do not run both entries as fresh full scopes or
repeat the absent-cookie case.

The invented-cookie request returned HTTP 401. The probe stopped on that denial,
as required by the scope, before asking for a real cookie. Exactly two requests
were spent across both runs. Neither negative request will be repeated.

The user then approved one separate real-cookie POST with the same bounds,
restricted fields, and stop conditions. Its distinct scope flag is
`one-real-cookie-fields-v1` and its entry is `live_positive_validation_evidence`.
It has no negative requests. Its owned console timed out after four minutes
without recording a positive result. The child was terminated by its specific
owned process ID. No real-case HTTP status or field evidence was recorded.
The report cannot establish whether hidden input completed or a real request
started. Do not repeat that scope automatically. Later runs report fixed input
and request-attempt phases to distinguish waiting from an in-flight request.
Forced process termination does not run Rust destructors and has no complete
memory-erasure guarantee.

This evidence cannot close PROFILE-1 or CORE-1 without an accepted validation
rule and persistent-material contract. Product login remains unsupported.

The user reported that the first real-cookie prompt was visible but no cookie
was entered. The user then requested reopening the unused request.
The next run recorded hidden input received and exactly one request attempt.
It returned HTTP 401 and stopped without retry, redirect, response-body
inspection, validation, or save. Neither negative case was repeated.
The response does not establish whether the input was stale, incorrectly copied,
or unsuitable for the proposed exchange. Do not label this as verified expiry.
Further live work needs human recovery or accepted exchange evidence and a new
exact scope; do not retry the rejected credential automatically.

The user identified that rejected value as older or saved material.
After human browser sign-in, a new separately approved current-cookie request
returned HTTP 200 with a nonempty token, positive numeric nameidentifier, and
future JWT expiry. The positive TTL observation was also present.
Exactly four requests were used in this comparison sequence. No retry, content
read, product validation, or protected save occurred. Identifier and expiry
values were not retained. All reports still state `account_validated=false`
and `saved=false`. The temporary console launcher was removed.
The user then accepted the
[strict Windows service rule](browser-handoff.md#accepted-windows-service-rule).
All live scopes are consumed; new live or native-store checks need fresh approval.

### Windows shared establishment checks

The shared [core operation](../src/beyond-gate-core/src/auth/establishment.rs)
uses one bounded service exchange and one protected save. Its private local
service and in-memory keyring seams are not configurable product inputs.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib auth::establishment --locked -- --test-threads=1
```

Nine offline tests cover exact persistent bounds and versions, positive versus
missing/zero/malformed/expired claims, real HTTP request headers, rejection,
redirects, throttling, invalid content, and declared or streamed oversized
responses. They also cover safe setup/write failures, cancellation, timeout,
and caller drop before and during a real blocking synthetic `set_secret` call.
The worker retains the pending reservation until it exits. Pre-write stop
preserves the old entry; an unconfirmed started write is `save_indeterminate`.
Cancellation handles do not retain a completed reservation.

The tests use real isolated Windows reservation objects, but no native
credential store or live account. Windows `auth login` now starts pairing;
browser setup remains incomplete. These are core lifecycle checks, not Chrome/Edge product login
or hosted CI evidence. macOS and Linux establishment are deferred.

## Isolated browser feasibility

The development-only [browser runner](../scripts/browser-tests/run.mjs) uses
regular installed Chrome and Edge on Windows. It loads the
[synthetic extension](../scripts/browser-tests/extension/manifest.json) through
`Extensions.loadUnpacked` over a DevTools pipe. See the official
[Puppeteer extension guide](https://pptr.dev/guides/chrome-extensions) and
[Chrome testing guide](https://developer.chrome.com/docs/extensions/how-to/test/end-to-end-testing).
It does not use the removed unpacked-extension command-line loading method.

Use Node.js 24 or later and PowerShell 7. The lockfile pins `puppeteer-core`
25.16.0. No browser is downloaded. Dependencies use the machine's default npm
registry. Node.js is development tooling, not a product runtime.

```powershell
npm ci --prefix .\scripts\browser-tests --ignore-scripts --no-audit --no-fund
npm test --prefix .\scripts\browser-tests
npm run feasibility --prefix .\scripts\browser-tests -- --browser chrome
npm run feasibility --prefix .\scripts\browser-tests -- --browser edge
```

The runner builds the [Rust fixture host](../src/beyond-gate-core/examples/browser-handoff-fixture.rs)
through `scripts\cargo.ps1`, with Visual Studio 2022 Build Tools on Windows:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 build -p beyond-gate-core --example browser-handoff-fixture --features browser-fixture --locked --quiet
```

The example requires `browser-fixture`. Normal product builds do not enable it.
Its pending reply comes from the shared fixture, not an authenticated CLI.
Its final reply is `unsupported_contract`, never a validated or saved result.
Neither the runner nor the host opens a credential store.

Each run owns a fresh directory under `target\browser-feasibility`, a fresh
browser profile, and a unique native-host key under that browser's HKCU
`NativeMessagingHosts` namespace. It does not replace existing keys or edit
client settings. The manifest permits only the actual test extension origin,
except in the deliberate wrong-origin rejection case.

Only `BeyondGateSyntheticSession` at `login.beyond-gate.invalid` is read.
These attributes are invented. The test permission grants cookie capability
for that fake host; code further restricts the name and exact metadata.
An explicit test consent action and a valid synthetic pending reply must precede
the handoff query. The separate cookie-API setup check seeds and reads only that
invented cookie; it is not a product consent flow or popup test.
A local proxy rejects HTTP and CONNECT traffic and never forwards requests.
The test does not navigate to an account site or use a personal profile.

### Local evidence and limits

Local checks passed on Windows kernel 10.0.26100 with Node.js 26.12.1 and
`puppeteer-core` 25.16.0.
Both browsers were regular installed builds, not bundled Chromium or Chrome for
Testing. Each browser passed these five native cases: missing registration,
wrong allowed origin, no consent, no pending login, and native receipt.
The host report confirmed hello and submit receipt. The two started hosts in
each browser exited without forced termination. The owned keys, profiles, and
proxy were removed or closed. All 14 extension unit tests passed.

| Browser | Actual version | Extension loading | Cookie API | Native cases | Cleanup |
|---------|----------------|-------------------|------------|--------------|---------|
| Chrome | 156.0.8078.74 | DevTools pipe | Passed | Passed | Passed |
| Edge | 156.0.4314.50 | DevTools pipe | Passed | Passed | Passed |

The runner also reports the actual Windows kernel, Node.js, and browser versions.
The [CI workflow](../.github/workflows/ci.yml) defines a separate Windows job for
each browser. It pins Node.js 26.12.1 and
[`actions/setup-node` v7.1.0](https://github.com/actions/setup-node/releases/tag/v7.1.0).
It retains all existing Rust OS jobs and uses no account secret or uploaded
transcript. Missing required infrastructure fails the job. These new Actions
jobs have passed on earlier revisions. A later repeated Chrome startup failure
is tracked in [#55](https://github.com/PlagueHO/beyond-gate/issues/55).
Local passes do not prove that a hosted failure is fixed.

### Safe startup diagnostics

Both browser runners emit a `starting` JSON record on stderr before the launch attempt.
It reports the installed executable's version without starting another browser,
plus the browser flavor, OS, Node.js, and pinned driver versions. This record
does not indicate a successful launch or a passed test.

A launch failure adds a bounded `launch` object to the failed JSON record.
It includes the launch stage, elapsed milliseconds, a fixed cause category,
and whether the driver observed process start or exit. Known protocol errors
can include an allowed startup method name. At most eight allowed startup
method names and a received-message count identify pipe progress without
recording protocol payloads. A process exit code is included only when the
driver's launch error provides it. Unclassified errors remain failed results.

The runner does not print raw errors, stacks, arbitrary cause chains, browser
output, full executable or profile paths, command lines, or environment data.
It does not enable Puppeteer's `dumpio` option. Chrome logging goes to an owned
`startup.log` inside the test directory. On launch failure, the runner reads
at most 16 KiB and reports only allowed source filenames and fixed error
categories. Raw log contents are never printed or uploaded. Cleanup removes
the log with the owned directory. Missing and unreadable logs are explicit.
Cleanup still attempts every
owned resource. If both an operation and cleanup fail, the operation remains
the primary error and `cleanup_failures` reports fixed resource labels.
Cleanup failure alone also fails the test.

The shared launcher keeps the fifteen-second launch timeout and the existing
per-runner protocol deadlines. It does not retry, skip a browser, change browser
flavor, or change any native-messaging or product-flow assertion.
CI requires three separate feasibility runs with fresh profiles. These are
mandatory cold-launch cases, not success-on-retry behavior. Any failed run
fails the step even if a later run passes. Each run must complete its cleanup.

The [first diagnostic hosted run](https://github.com/PlagueHO/beyond-gate/actions/runs/41420080293)
passed both browsers' feasibility and product-flow checks at `001a59e`.
It reported regular Chrome `156.0.8078.48` with Puppeteer `25.16.0`.
The original startup failure did not recur. This pass does not establish a
startup fix or a root cause. The earlier failed jobs do not contain the missing
launch evidence.

The [next diagnostic run](https://github.com/PlagueHO/beyond-gate/actions/runs/41421876546/job/125199095766)
reproduced the launch failure with regular Chrome `156.0.8078.80` on image
`20261030.294`. `Target.setDiscoverTargets` timed out after 15,375 milliseconds
with no protocol messages received. The process started but had not reported
exit when the error was captured. Edge passed. This identifies the failed
startup boundary, not the reason Chrome did not answer.

Cleanup closes the native port, verifies an owned host's executable path and
start time before stopping its specific PID if necessary, removes only its
marked native-host key, closes the browser and proxy, and verifies the test
directory's parent before removal. Cleanup failures produce a failed result
with fixed diagnostics, not raw profile paths or peer messages.
A forced runner termination can prevent cleanup. Inspect the exact owned
directory and key before removal; never delete a browser namespace or temporary
root. The runner does not clean up the user's earlier session-only prototype.

This is transport feasibility only. It does not authenticate application peers,
connect to `auth login`, execute all 63 state recipes, prove production cookie
attributes, implement a user popup, validate an account, or save a session.
Linux and macOS browser/native registration remain untested.

### Windows cookie-profile admission checks

The selected
[Windows contract](browser-handoff.md#selected-windows-cookie-contract) has three
parser checks beside the message module and three additional paired-flow checks.
All six passed locally on Windows with invented cookie bytes. The complete
default and feature-enabled workspace tests also passed.

Parser checks cover both protocols, exact admission at 2,552 bytes, rejection
at 2,553 bytes, invalid cookie syntax, exact profile names, and unchanged synthetic
prefix checks. Paired tests reject changing the profile before authorization or
after binding, including a submit with the correct transaction. Receipt transfer
accepts only the production profile and emits no saved-login result. No browser,
real account, live request, or native credential entry is used.

One additional native-process check rejects production profiles at both fixture
ingress points and preserves successful synthetic receipt with
`unsupported_contract`, never saved success. All ten native-process checks
passed. The test copies only known synthetic setup and the feature-built binary
into a newly owned build directory; cleanup removes only those fixed files.

The guarded non-Windows test requires `unsupported_profile`. Local Windows
checks cannot execute that platform branch; later hosted Linux/macOS Rust jobs
passed. The user-reported Edge metadata remains separate from these offline
tests. Diagnostic removal was confirmed during the approved product check below.

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

## Approved Windows product login verification

The user separately approved this local product test after hosted CI passed.
It is not the manual live-resource suite and does not enable that suite.
The user selected Edge Dev, not the separately installed stable Edge build.

| Browser | Version | Human pairing and saved result | New CLI status | Local logout |
|---------|---------|--------------------------------|----------------|--------------|
| Chrome | 156.0.8078.74 | Passed; actual product CLI exit 0 | `stored-unverified` | Passed |
| Edge Dev | 158.0.4357.0 | Passed; actual product CLI exit 0 | `stored-unverified` | Passed |

The OS was Windows kernel `10.0.26100`. Default-feature product binaries were
built with the repository wrapper and Visual Studio 2022 Build Tools.
They ran from an explicit stable local installation outside source and `target`.
Both browsers confirmed the pinned extension ID before handoff.
The user controlled sign-in and entered each generated key only in the browser.
The agent did not capture the console key, password, cookie, bearer, or claims.
A session-only runner recorded only browser labels and product CLI exit codes.

Each browser completed one product login. Success required the real fixed
direct-service validation, native protected save, and final acknowledgement.
There was no fixture service, cookie API replacement, memory store, proxy, retry,
or resource read. The approval allowed one exact `CobaltSession` query and one
POST to `https://auth-service.dndbeyond.com/v1/cobalt-token` per browser.
No other handoff was started. Request counts follow the tested single-request
product contract; no private network transcript was captured.

Four native status calls covered initial absence, persistence after each login
in a new CLI process, and final absence. Three logout calls covered both saved
entries and repeated deletion. The first restart assertion used incorrect
wording. The observed output matched the source contract; the assertion was
corrected without another credential read.

The user removed both test extensions and closed their login windows.
Matching native-host removal passed for both browsers. Only the owned manifests,
extension files, binaries, and installation directory were removed.
The earlier temporary Edge metadata diagnostic was also confirmed removed.
The protected product entry was absent at the end. All private-test approval
budgets are consumed; another test requires a new exact approval.

[Hosted CI run 41282317197](https://github.com/PlagueHO/beyond-gate/actions/runs/41282317197)
passed separate stable Chrome and Edge synthetic jobs and all three Rust OS jobs.
The earlier run passed browser jobs but found non-Windows unused helpers and a
missing Windows fixture build; those failures were fixed before this successful run.
Stable Edge has synthetic CI evidence, not a separate real-account pass.
macOS and Linux product browser login remain deferred.

Saved evidence describes validation at login time. Local status does not prove
current upstream usability or expiry. Reusable authenticated CLI/MCP sessions,
refresh, live resource reads, and upstream revocation remain separate work.
No PR, release, or merge occurred during this verification.

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

The separately approved local Windows run passed all four `native.auth.*`
assertions: absent, stored-unverified, logout, and repeated logout. The final
absence assertion and owned entry/file cleanup completed without a reported
cleanup failure. Only the random synthetic `beyond-gate-e2e` entry was accessed;
no product entry or account was used.

The full command returned exit 1, not full success: 102 cases passed, 47 live
cases remained blocked, and five installation-harness cases failed with
`harness_error`. The failed cases were Claude Code preview/conflict, Copilot CLI
preview/malformed settings, and VS Code conflict. These results must remain
separate from the passed native assertions. No failure cause or product-login
success is established by the safe harness classification.

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
