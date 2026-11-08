# Manual capability tests

**Full live capability coverage remains blocked.** The shared suite runs
the real CLI and MCP executables. It does not replace their read operations with
fixtures. Windows library discovery includes category selection and title cleanup;
the approved Web-catalog scope now handles configured ownership-only sources.
Fresh native Web-library CLI/MCP acceptance passed. Windows bounded book source
operations have synthetic process coverage. Separately approved native book
acceptance passed for one selected chapter with matching outlines and initial
chunks. Live continuation and sharing remain unchecked.
Character and campaign reads now have fixed, bounded Windows implementations
and synthetic process coverage. No live acceptance has been run. The suite
keeps a zero live-request budget. Its credential-free contracts reject extra
input for all three book commands and their MCP tools before session access.
They do not call character, campaign, or book operations with valid input.

The [issue #27 plan](https://github.com/PlagueHO/beyond-gate/issues/27) permits
this blocked-first stage. It does not authorize live tests. A supplied cookie
does not change this suite's zero live-request budget.

## Results and coverage

For archive-based checks and separate approval gates, use the
[desktop acceptance procedures](desktop-acceptance.md).
The package verifier's optional JSON report records offline case results,
candidate metadata, host versions, and cleanup. Manual acceptance cases remain
blocked, and `full_coverage` stays false. These reports do not replace the manual
suite or its separately approved native-store evidence.

The [Linux package-only follow-up](desktop-acceptance.md#azure-linux-package-only-follow-up)
records a disposable VM's command/runtime inventory, both binaries' startup,
new-login-shell PATH, reinstall, relocation, removal, and verified cleanup.
It used noninteractive SSH, not a desktop terminal. System Python 3 was present.
No credential, real-client, browser, or account operation was tested.
The passing shell report is not full desktop acceptance.

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

A `contract.*` pass proves unsupported errors or strict-input rejection only.
Corresponding `live.*` cases remain blocked. Help, successful authentication
probes, unsupported errors, and empty results do not prove live read coverage.
Testing redirected-login refusal does not prove a working browser handoff.

`full` returns a nonzero exit while any required case is not passed.
`contracts` selects offline command, protocol, and registration contracts. It
excludes native storage and live reads. It can pass but always reports
`full_coverage: false`. A missing inventory case, duplicate case, or failed
selected contract cannot produce full success.

## Browser handoff offline checks

### Class and subclass metadata checks

The [core catalog tests](../src/beyond-gate-core/src/class_catalog/tests.rs)
use invented definitions and real loopback transports. They check duplicate
names, local substring matching, parent context, source extensions, absence of
raw definition text, ambiguity, empty/missing results, invalid configuration,
malformed/continued envelopes, denial, expiry/rejection, and cancellation.
Threshold checks cover 32/33 global parents, scoped reads with 33 roots,
1,000/1,001 raw records, 1 MiB canonical JSON, and 8 MiB admitted resource bytes.
Padded valid JSON proves byte accounting uses the transport buffer, not
re-serialization. Failed parents stop subsequent requests.

The [CLI/MCP process suite](../src/beyond-gate-mcp/tests/session_process.rs)
checks every reviewed command/tool workflow, matching metadata JSON, schemas,
strict inputs, table/TSV defaults, failure categories, and cancellation.
The actual CLI parser checks help, completions, invalid IDs, forbidden options,
and JSON header rules. The capability inventory includes both new tools while
the manual suite retains its zero live-request budget.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --locked class_catalog
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-mcp --test session_process --locked
```

These fixtures do not open native storage or contact live accounts.
The five browser scopes for issue #65 are consumed. Native catalog compatibility,
value-level link validation, live global discovery, and real sharing remain
unverified. Another live check needs separate explicit approval.

### Live book source path checks

The [core source tests](../src/beyond-gate-core/src/book/source/tests.rs) use
original HTML and actual loopback metadata/page services. They cover canonical
identity, same-book aliases, nested hierarchy, duplicate roles, raw 1,000/1,001
anchor bounds, legal UTF-8 fragments, unsafe paths, hidden or ambiguous content,
empty sections, text structure, sidebars, stale text/URLs, sharing loss, failed
responses, and in-flight cancellation. Purchase-link cases cover relative and
absolute same-site marketplace paths and the dedicated marketplace host.
Structural-validation cases cover bounded depth and element counts before
heading-visibility checks. HTTP tests cover exact media, encoding,
body, retry, and status rules. Session tests cover account changes across a
request-local scope and another concurrent cache owner.

The [CLI/MCP process suite](../src/beyond-gate-mcp/tests/session_process.rs)
starts actual adapter processes through the production source parser. It compares
outline and section JSON, reconstructs exact normalized Unicode text through
explicit continuations, and checks stale content/URLs, access failures,
preview/malformed responses, strict input, cancellation, TSV, and output streams.
No product flag, endpoint argument, environment override, real credential, or
account snapshot selects these fixtures.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-mcp --test session_process --locked
```

These are offline behavior checks, not native adapter acceptance. The earlier
three native investigation scopes are consumed. More live requests require a
new exact resource, credential-read scope, total attempt budget, deadlines,
stop conditions, and cleanup. The manual capability suite retains its zero
live-request budget.

### Native book acceptance

The [separately approved native run](api-survey.md#native-book-adapter-acceptance)
completed one CLI `book list`, then one outline and one initial chapter read
through each adapter. Both returned matching 79-entry outlines and matching
3,929 UTF-8 byte chunks for the fixed selected chapter.
No continuation or other chapter was read.

The scope allowed at most 31 HTTP attempts and 16 protected-entry loads within
180 seconds. These are conservative operation bounds, not measured wire counts.
A temporary helper passed synthetic success, malformed-source, denied-response,
and missing-approval guards before its one native run. It captured responses
only in memory and reported safe counts and parity flags. The helper and consumed
guard were removed. The scope made no credential changes or installation.
Another live run needs new explicit approval.

This establishes native parity for that selected source and initial chunk only.
It does not establish live continuation, sharing, general book compatibility,
or a separate entitlement API. The manual capability suite retains its zero
live-request budget.

### Library discovery checks

Core tests use invented public configuration, catalog, and ownership JSON through actual loopback HTTP
and shared synthetic sessions. They cover purchase, account sharing, both,
catalog-only unknown access, source attribution, raw caps, complete counts,
continuation refusal, conflicts, unmatched IDs, sharing loss, no retry or redirect,
and safe failures. Real CLI/MCP child processes compare JSON evidence and errors,
TSV and table formatting, malformed and oversized data, session failures, strict
input rejection, and MCP cancellation. Product commands expose no fixture selector.

Production selection tests reject invalid or duplicate configuration IDs, unknown
categories, missing library-ID joins, malformed visible metadata, and conflicting
hidden records. Every ownership ID must match public configuration. All evidence,
including hidden and ownership-only records, validates before selection.
Raw limits apply before selection. Title tests
preserve valid internal spacing, clean invalid whitespace, and reject blank names,
non-whitespace controls, and unsafe links. Actual adapter process tests verify
normalized output, hidden-source exclusion, and safe configuration/title/evidence
failures. Configured hidden and visible IDs outside the Web catalog create no
books or grants for another ID. Unconfigured IDs still fail `UnmatchedOwnership`
through both real adapters. Invalid ownership types outside the Web catalog
also fail. Invalid public configuration prevents protected-session access.

Run the focused checks with the required Cargo wrapper:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core library:: --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-mcp --test session_process --locked
```

Live CLI/MCP acceptance needs a separate approved request budget. The two
[metadata probe scopes](#library-metadata-probe) are consumed. They do not
authorize valid production book-list calls. No shared ownership was observed
live; synthetic sharing checks do not establish real-account coverage.

Later approved native acceptance failed catalog validation and skipped MCP.
Two additional catalog diagnostics found missing attribution and rejected the
proposed released-only source rule. All scopes are consumed.
See [native rejection evidence](api-survey.md#native-adapter-rejection-and-catalog-diagnostics).
The ignored core tests `approved_live_catalog_diagnostic` and
`approved_live_released_catalog_contract` require separate acknowledgment strings.
An acknowledgment does not itself grant fresh permission. The second test's
native guard failure is not a passing discovery check. Normal workspace tests
ignore both entries.

The user later approved production category selection and title cleanup after
the [failure-class diagnostic](api-survey.md#approved-metadata-failure-class-diagnostic).
That implementation passes the synthetic discovery and actual adapter process
checks. It does not reuse any earlier live scope. A separately approved fresh
native CLI check passed catalog preparation but failed `UnmatchedOwnership`;
MCP was skipped. See [acceptance evidence](api-survey.md#native-selected-library-acceptance).
That scope is consumed. The manual capability suite was not rerun for this change; its earlier
unrelated `cli.auth.login-blocker` failure is not a successful full-suite result.

The separately approved ownership membership diagnostic then found 44 unique
ownership IDs: 37 matched the Web catalog, two unmatched IDs were hidden, and five
were visible. All seven unmatched IDs existed in public configuration.
The user approved Web-scoped results after the public-client comparison.
Fresh native acceptance for that scope then passed separately from synthetic success.
See [scope evidence](api-survey.md#approved-web-catalog-result-scope).

The [native Web-catalog acceptance](api-survey.md#native-web-catalog-adapter-acceptance)
used one newly approved CLI run and one MCP run. Both returned 169 books, with
matching IDs, titles, source URLs, access summaries, and evidence. The result had
36 purchased and 133 unknown books. No shared access was observed. This does not
replace real-sharing validation or claim full manual-suite coverage.
The one-use scope is consumed; no raw metadata was retained.

The ignored `approved_live_ownership_membership_diagnostic` uses the non-secret
acknowledgment `issue-32-ownership-membership-five-attempts`. Its native scope is
also consumed. The acknowledgment is a guard, not reusable permission.
Normal workspace tests ignore it.

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
Observed late replies and process exit add separate inspection fields.
These fields are absent when no such event was observed.
They do not replace the original failure snapshot or clear its failure.
An explicit flag records whether the driver had requested browser close when
the launch error was caught. A later stack can reflect that shutdown request.

The runner does not print raw errors, stacks, arbitrary cause chains, browser
output, full executable or profile paths, command lines, or environment data.
It does not enable Puppeteer's `dumpio` option. Chrome logging goes to an owned
`startup.log` inside the test directory. On launch failure, the runner reads
at most 16 KiB and reports only allowed source filenames and fixed error
categories. Raw log contents are never printed or uploaded. Cleanup removes
the log with the owned directory. Missing and unreadable logs are explicit.
The runner also bounds captured startup stderr to 16 KiB and returns only the
same safe categories. A failed launch can report fixed process counters,
known child-process types, Chrome module presence, and owned profile milestones.
At most five ordered thread-state/wait-reason tuples distinguish individual
waits without exposing thread IDs. These enumeration indices do not prove
debugger-thread identity.
During observation only, the collector can read public CodeView build IDs
from an already loaded browser executable or ELF module. These IDs select
public symbols, not accounts or profiles. At most two fixed module labels
and hexadecimal build IDs leave the collector. PDB paths, image bytes, and
process-memory values do not. This does not read browser files before launch.
These IDs describe backing files. A debugger must still verify that its
symbols match the loaded module before a function can be identified precisely.
In manual `startup-stacks` mode, each runner also reports these IDs on stderr
after all its browser cases succeed, before cleanup. This separate record
does not require another startup failure or run a stack probe. An unavailable
inspection is explicit. Default runs do not request this post-case inspection.
Fixed pipe byte counters, buffer lengths, and end/close flags distinguish
transport progress without reading or retaining a protocol payload.
The observer waits for Node.js's `spawn` event before checking the exact executable
and owned profile. It releases all observation listeners after each launch.
Process inspection has a separate five-second diagnostic limit;
it does not extend a browser or protocol deadline. No raw process paths,
command lines, module lists, or stderr leave the collector.
An offline Windows test checks this collector against an owned synthetic
Node.js process and invented profile markers. It checks actual process access
on the hosted runner without starting another browser or reading account data.

Cleanup still attempts every owned resource. If both an operation and cleanup fail, the operation remains
the primary error and `cleanup_failures` reports fixed resource labels.
Cleanup failure alone also fails the test.
After collecting failed-launch evidence, the runner aborts the owned browser
through Puppeteer's launch signal and waits for process exit before directory
cleanup. A five-second exit-wait limit reports a cleanup failure, not success.
The driver can return a launch error while its asynchronous browser close is
still pending. Removing the profile at that point can fail while files are open.
A controlled offline test also runs the real driver against an owned synthetic
process that never replies to its pipe. The original protocol timeout must
remain a failure. The process must exit before its directory is removed.

The shared launcher keeps the fifteen-second launch timeout and the existing
per-runner protocol deadlines. It does not retry, skip a browser, change browser
flavor, or change any native-messaging or product-flow assertion.
CI requires three separate feasibility runs with fresh profiles. These are
mandatory cold-launch cases, not success-on-retry behavior. Any failed run
fails the step even if a later run passes. Each run must complete its cleanup.

For startup investigation, a manual CI run can select `startup-sampling`.
This starts three separate Windows runners per browser. Normal push and PR
checks still use one runner per browser and retain their existing check names.
Each extra runner retains every required test and deadline. This mode increases
hosted-runner use; it does not convert an earlier failure into a pass.
A manual run can also select `startup-stacks` to prepare Microsoft's debugger
on the disposable runners. This option is separate from fresh-runner sampling.
The [pinned SDK installer](https://learn.microsoft.com/windows/apps/windows-sdk/downloads)
is checked against its SHA-256 checksum and Microsoft signature. Setup selects
only Debugging Tools for Windows. It refuses to run on a normal local machine.
Setup also uses [SymChk](https://learn.microsoft.com/windows-hardware/drivers/debugger/using-symchk)
to prepare matching `ntdll.dll` symbols from the
[Microsoft public symbol server](https://learn.microsoft.com/windows-hardware/drivers/debugger/microsoft-public-symbols).
This separate preparation has a thirty-second limit and retains only verified
public Windows symbols. It does not read or preload Chrome files.
If preparation fails, the failed setup verdict stays. A later probe reports
missing or ambiguous symbols and can still collect export-only hints.
It does not select an arbitrary PDB, download symbols during capture, or
report that missing symbols loaded.

On Chrome runners, this manual preparation also downloads two pinned public
PDBs from the [official Chrome symbol server](https://www.chromium.org/developers/how-tos/debugging-on-windows/#symbol-server).
Their public build IDs were observed after Chrome `156.0.8078.80` completed
its required cases. Each download has a thirty-second limit and a 128 MiB
size limit. SHA-256 checks verify the pinned bytes. This network-only step
does not read installed browser files or run Chrome. Other browser builds
can have different IDs. CDB must honor CodeView records and cannot use
mismatched-PDB overrides. Its normal export fallback remains available for
modules without PDBs. The stricter global `SYMOPT_EXACT_SYMBOLS` option is
disabled because it also removes useful Windows export hints.
See [symbol options](https://learn.microsoft.com/windows-hardware/drivers/debugger/symbol-options)
and [module reload matching](https://learn.microsoft.com/windows-hardware/drivers/debuggercmds/-reload--reload-module-).
Separate flags report actual PDB-loaded module rows, not just prepared files.
Missing or unmatched symbols are not a cause finding.
Fixed C++ hints cover early DLL initialization and file preread. Unknown C++
functions keep their frame position but leave only an `other` hint.

An optional [noninvasive stack probe](https://learn.microsoft.com/windows-hardware/drivers/debugger/noninvasive-debugging--user-mode-)
checks the exact executable and owned profile argument. It holds the process
handle open to prevent PID reuse. It reads at most 32 frames from debugger
thread zero and eight frames from each of the next four threads, then detaches.
The after-failure probe temporarily suspends the already failed target.
Process counters are collected first so this suspension cannot change them.
Network symbol lookup and debugger shell commands are disabled. The probe
does not create memory dumps, inspect values, or write raw debugger output.
Its fixed buffers hold at most 16 KiB of output and 4 KiB of stderr.
Both pipes continue to drain after these storage limits. Excess bytes are
discarded so they cannot block debugger exit. Quiet mode reduces startup noise.
Only allowed module names and fixed function hints leave the collector.
Ordered frame hints preserve the call sequence. Windows loader hints use the
prepared local symbols. The probe reports whether those symbols loaded.
Export hints can be the nearest export, not the exact waiting function.
They do not establish a cause without other evidence.
The debugger has a three-second limit. Its wrapper has an eight-second limit.
Neither limit changes a browser or protocol deadline. Probe failure remains
explicit and does not clear the launch failure. Browser abort and cleanup
still follow the probe. An owned synthetic Node.js process checks the actual
probe on the hosted runners and must respond after debugger detach.
In this explicit investigation mode, a failed probe check does not skip the
remaining browser checks. The job still fails. Default checks keep their
normal step conditions.
Fixed Boolean fields identify debugger gallery, engine, symbol-path, and
stack-marker progress, known attach failures, and target exit during the probe.
No raw debugger output leaves the collector. A nonzero debugger exit retains
these fixed progress fields and its exit code.

In `startup-stacks` mode only, a launch that still has no protocol replies
after ten seconds starts one early observation. The early probe uses
[CDB's `-pvr` option](https://learn.microsoft.com/windows-hardware/drivers/debugger/cdb-command-line-options)
to leave the target running. The fifteen-second launch timeout and each
protocol timeout remain unchanged. The timer is canceled when launch ends.
An observed reply prevents this early capture. This trigger is not a sleep,
retry, browser prewarm, or startup workaround. Diagnostic work can affect
timing, and running-thread stacks are less reliable than suspended stacks.

The early record identifies its pending-launch phase and its actual start
and completion times. It must not be treated as an atomic snapshot or assumed
to have completed before shutdown. On launch failure, this observation is
joined and reported instead of starting another stack probe. The original
failure count and protocol-method snapshot stay separate. If launch succeeds,
the observation is a separate stderr record, not another passed test.
Both launch outcomes join any pending collector before removing listeners or
allowing profile cleanup. Unexpected inspection errors keep a typed cause
and report only its safe category; they do not replace the original launch
failure. Separate owned-process tests check both debugger modes.

```powershell
gh workflow run ci.yml --ref <branch> -f startup-sampling=true -f startup-stacks=true
```

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

The [repeated hosted run](https://github.com/PlagueHO/beyond-gate/actions/runs/41425928582/job/125218197284)
failed its first two mandatory launches with the same silent protocol boundary
and no startup log. Its third launch passed all five native cases, but the
step correctly remained failed. A local initialized Chrome created its owned
startup log, and twelve separate local cold launches passed. This narrows the
hosted investigation to early startup without establishing its cause.

The [next instrumented run](https://github.com/PlagueHO/beyond-gate/actions/runs/41436385092/job/125247052790)
passed Chrome but exposed this cleanup race in Edge. The first Edge launch
had zero protocol replies, an active process with renderer and GPU children,
and a failed directory cleanup. Its later cases passed but did not clear the
failure. The driver's `profile-in-use` category does not prove that a foreign
browser used the unique profile. The startup cause remains unknown.

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

The later [retained Windows package check](desktop-acceptance.md#retained-ci-candidate-verification)
passed all 18 synthetic installation cases without native storage approval.
That run found and fixed a separate stale Windows redirected-login expectation.
Its credential-free selection then passed with 108 passed cases, 50 blocked live
cases, and four unselected native cases. This does not explain the earlier five
installation failures or add new native-store evidence.

See the [desktop acceptance matrix](desktop-acceptance.md#remaining-acceptance-matrix)
for later native-store evidence and remaining gaps.

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

## Library metadata probe

The ignored Windows unit tests in `auth::session::library_probe` are separate
from the #27 suite and its zero request budget.
It requires separate explicit approval for protected credential access and the exact
three-attempt scope. Ordinary tests and CI skip it.
A non-secret `BEYOND_GATE_LIBRARY_PROBE_APPROVAL` acknowledgment is an execution guard,
not independent permission. The schema-only test `approved_live_library_metadata`
requires `issue-32-metadata-three-attempts`. The separately approved follow-up test
`approved_live_library_contract_checks` requires
`issue-32-metadata-contract-three-attempts`; the earlier acknowledgment cannot admit it.
Do not run the test just because that acknowledgment is known or set.
Isolated `e2e-isolation` builds are rejected before native storage access.

The fixed sequence is one `AUTH-01` exchange, GET `LIB-01`, then GET `LIB-02`.
The probe makes no storage writes or deletes. It keeps secrets and raw responses
in process memory only. Transport has no redirects, retries, proxies, or compression.
It uses a 5-second connection cap, 15-second request cap, and 60-second complete
probe cap. The existing exchange caps its body at 64 KiB; each metadata body is
capped at 1 MiB. Structural inspection is capped at 1,000 records per collection.
Any failure stops the sequence. No source URL, continuation, or book text is fetched.

Output is restricted to fixed schema names and JSON types, response byte counts,
and safe ownership-type presence classifications. Unknown keys become a marker.
It never prints scalar values, headers, account or source records, or raw responses.
The acknowledgment must be removed from the launching process after the run.
The fixed credential entry is left unchanged.

The separately approved schema-only run on 2026-11-04 succeeded in exactly three attempts.
The separately approved follow-up stopped after two attempts with a failed contract
check: the source response exceeded the existing 100-record page cap.
It also reported that not every source matched the conservative book-path rule.
It skipped the ownership request instead of continuing after failure.
Both scopes are consumed. See the [safe results](api-survey.md#approved-native-library-metadata-check).
The result does not prove complete library discovery, sharing contexts, or
successful production CLI/MCP reads.

Run only the synthetic checks without approval or credentials:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib library_probe --locked
```

These checks use invented responses and loopback HTTP. They verify approval gating,
redaction, malformed shapes, count equality, source joins, source paths, record caps,
byte and attempt caps, and no retry or redirect behavior.
They do not load protected storage or contact live services.

## Mobile book metadata probe

Issue #33 has a test-only Windows probe in
`auth::session::library_probe::mobile_probe`.
One separately approved run completed on 2026-11-06. That scope is consumed;
another live run needs new approval.
At that diagnostic stage, the native book contract gate was blocked and
production outline and section reads remained unsupported. The later
[approved page-level contract](architecture.md#live-book-reads) permits the
implemented website reader. This mobile probe does not establish that reader.

The fixed scope is one read of the protected session entry and one POST
to `https://www.dndbeyond.com/mobile/api/v6/available-user-content`.
The pinned [candidate transport](api-survey.md#direct-mobile-routes-in-the-companion-tool)
sends the reusable cookie in the form field `token`, not a bearer header.
This different credential transport needs separate explicit approval.
It is not a fallback for the website's purchase redirect.
No live request was made during implementation or offline checks.

The ignored `live_mobile_metadata` test requires the non-secret
`BEYOND_GATE_BOOK_PROBE_APPROVAL` acknowledgment
`issue-33-mobile-metadata-one-attempt`. This is an execution guard, not permission.
Missing or earlier acknowledgments and `e2e-isolation` builds fail before protected
storage access. Do not select ignored tests or set the acknowledgment without
separate approval for the exact scope.

The probe makes no bearer exchange, credential write/delete, retry, redirect,
proxy request, source-page request, book-code request, or book download.
It uses a 5-second connection cap, 15-second request cap, 30-second complete
deadline, and 1 MiB response cap. Only identity-encoded JSON with HTTP 200 is
accepted. It caps raw license records and total nested entities at 1,000 each.
Any failure stops the sequence.

Output contains fixed schema names and JSON types, aggregate counts, and
`content_access=unverified`. Unknown field names and all scalar values are omitted.
License types and ownership fields never become access grants. No account data,
book identifiers, download URLs, credentials, or raw responses are logged or saved.
Owned cookie and form buffers are cleared on drop. Dependency, parsed JSON, and
transport-owned copies cannot be guaranteed to be erased.
After a separately approved run, remove the acknowledgment from the launcher and
stop its process. The fixed protected entry must remain unchanged.

Run only the synthetic checks without approval or credentials:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib mobile_probe --locked
```

These Windows checks use invented data and loopback HTTP. They check acknowledgment
and target guards, form encoding, record and byte bounds, safe summaries, media
types, the actual POST body/headers, request timeout, and refusal to retry or
follow redirects. They do not prove the candidate's
current availability, read-only semantics, ownership meaning, or content access.
The #27 suite's live request budget remains zero.

### Approved metadata result

The approved run made one POST and accepted HTTP 200 with a 12,596-byte JSON
response. Structural inspection found 8 license records and 478 nested entities.
It confirmed `Licenses`, `EntityTypeID`, `Entities`, numeric `id`, and boolean
`isOwned` fields in this response. It did not establish their access meaning.
The result remained `content_access=unverified`.

The exact selected test passed:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib auth::session::library_probe::mobile_probe::live_mobile_metadata --locked -- --ignored --exact --nocapture
```

Only that ignored test ran. There was no bearer exchange, retry, redirect,
book-code request, or content read. The protected entry was not changed.
The launcher removed its acknowledgment and the test process exited.
Only structural output and aggregate counts were retained.
This result is not CLI/MCP acceptance or approval for another request.

## Website cookie probe

Issue #33 has a second ignored Windows test in
`auth::session::library_probe::website_probe`.
The user separately approved a cookie-only investigation after the earlier
bearer-only purchase redirect. This is not a production credential fallback.
Its one approved request completed on 2026-11-06. That scope is consumed.

The exact scope is one read of the fixed protected session entry and one GET to
`https://www.dndbeyond.com/sources/dnd/cos/introduction`, using only the existing
`CobaltSession` cookie. It makes no bearer exchange and uses no other credentials,
browser impersonation, proxy, retry, redirect, other page, or credential write/delete.
Connection, request, and complete deadlines remain 5, 15, and 30 seconds.
The response cap is 1 MiB. Only HTTP 200, identity encoding, and one `text/html`
media-type header are accepted. Missing fixed structure or known challenge markers
stop the check. The probe does not extract text or parse a DOM.

The ignored `live_website_cookie` entry requires
`BEYOND_GATE_BOOK_PROBE_APPROVAL=issue-33-website-cookie-one-attempt`.
This non-secret guard is not permission. Earlier acknowledgments and
`e2e-isolation` builds fail before protected storage access.
Each live execution needs separate approval for its exact scope.

The approved run passed:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib auth::session::library_probe::website_probe::live_website_cookie --locked -- --ignored --exact --nocapture
```

Only that ignored test ran. Its single HTTP 200 response had 257,524 HTML bytes.
Fixed article, section, and canonical-URL literals were present. Output remained
`content_access=unverified`. These observations do not prove DOM structure,
preview status, ownership, or full-book access.

No raw HTML, headers, text, identifiers, or credentials were retained.
The cookie and response buffers are zeroizing. Dependency-owned credential and
transport copies cannot be guaranteed to be erased. The protected entry was
unchanged. The launcher removed the acknowledgment and the process exited.
No further request is approved. The #27 suite's live budget remains zero.

Run the five synthetic website checks without credentials:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib website_probe --locked
```

They check separate approval and target guards, redacted literal observations,
the actual cookie-only GET, terminal failures, exact byte limits, and no retry
or redirect. Shared probe tests cover the same fixed transport deadlines.
The shared body reader rejects duplicate media-type headers for JSON and HTML.
These checks do not enable production book reads.

## Protected chapter structure probe

The user selected Chapter 1 at
`https://www.dndbeyond.com/sources/dnd/cos/into-the-mists`.
A separate approval permitted one protected-entry read and three ordered GETs:
the selected source with the existing cookie, this chapter with the same cookie,
and this chapter without credentials as the final control.
The anonymous client has no credential headers. No request follows its result.

The Windows-only `auth::session::library_probe::structure_probe` module uses
the test-only `scraper` dependency with default features disabled.
It does not execute scripts, serialize HTML, return text, or cache content.
Output keys are static labels; values are counts. Canonical links and unique
containers are required. The source and chapter paths are fixed, not token inputs
or automatically followed outline links.

The ignored `live_protected_structure` test requires the separate non-secret
`BEYOND_GATE_BOOK_PROBE_APPROVAL=issue-33-protected-structure-three-attempts`
guard. Earlier guards and `e2e-isolation` builds fail before protected reads.
Knowing a guard is not approval. The scope has no bearer exchange, other identity,
other credentials, browser impersonation, proxy, retry, redirect following,
other page, credential write/delete, or content cache.

Limits remain 5 seconds to connect, 15 seconds per request, 30 seconds total,
1 MiB per body, and 1,000 raw outline anchors before classification.
Unexpected responses, denied authenticated requests, challenges, ambiguity,
and limits stop the remaining sequence. A final anonymous redirect or refusal is
reported as a control outcome with an unknown reason; its body is not read.
Neither it nor a successful page becomes a general access grant.

The separately approved run on 2026-11-06 passed:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib auth::session::library_probe::structure_probe::live_protected_structure --locked -- --ignored --exact --nocapture
```

Only that ignored test ran. Source and chapter HTTP 200 responses had 246,103 and
356,402 HTML bytes. The parsed outline had 81 raw links, including one duplicate,
and three fragment targets for the selected chapter.
All three matched IDs in its single article. The final anonymous GET returned
HTTP 307; no redirect or retry followed. There were exactly three attempts.
See the [survey result](api-survey.md#approved-protected-chapter-structure-and-control)
for the remaining safe counts and their limits.

Only classifications, booleans, and counts were retained. Raw HTML, text, private
fragment sets, headers, and credentials were not retained. Cookie and raw body
buffers are zeroizing. Parsed DOM and dependency-owned copies cannot be
guaranteed to be erased. The fixed entry was unchanged. The launcher removed
the guard and the process exited. This scope is consumed; another live request
needs new approval. The #27 suite's live budget remains zero.

Run its five synthetic checks without credentials:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib structure_probe --locked
```

They check counts and redaction, raw bounds, canonical paths, ambiguity, hidden
attributes, known challenges, separate approval, exact authentication order,
actual wire headers, the final anonymous redirect, and no fourth attempt.
They do not enable production reads or prove full-book authorization.

## Offline validation

The ignored Windows `library::discovery::tests::approved_live_metadata_comparison`
checks fixed `CFG-01` and `LIB-01` routes, not ownership or book content.
It requires separate approval for at most three HTTP attempts, including the
protected exchange, with a 60-second complete deadline and 1 MiB catalog caps.
Its current acknowledgment is `issue-32-corrected-config-comparison-three-attempts`;
knowing the value is not permission. The earlier
`issue-32-config-comparison-three-attempts` acknowledgment is rejected.
Both comparison scopes are consumed. The original run found complete ID matches,
but its diagnostic did not support a relative source-link form.
The separately approved corrected run validated metadata for 170 of 177 library
records. It did not resolve the four records lacking attribution. See the
[comparison evidence](api-survey.md#alternative-configuration-catalog-comparison).

Run its approval, synthetic metadata-join, and safe-link guards without credentials:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib metadata_comparison_guards_use_synthetic_records --locked
```

The separate ignored `library::discovery::tests::approved_live_category_comparison`
checks named-category membership before strict metadata validity. Its earlier acknowledgment
was `issue-32-hidden-category-three-attempts`; that scope is consumed, not reusable
permission. The user approved at most three attempts on 2026-11-06: public
configuration, protected exchange, then bearer-only catalog. It kept the 60-second
total deadline, existing response caps, and no retries, redirects, ownership calls,
book reads, credential changes, or raw retention.

The check found eight hidden records among 177 library records, including all
four without attribution. Two of the remaining 169 records still failed both
metadata mappings. It stopped with `InvalidMetadata`. No production behavior
changed. See the [category comparison evidence](api-survey.md#approved-hidden-category-comparison).
Run its synthetic approval, category-join, completeness, and metadata guards:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib category_comparison --locked
```

The same ignored entry now rejects that acknowledgment. Its separately approved
failure-class scope uses `issue-32-category-failure-classes-three-attempts`.
That scope is also consumed. It reports fixed failure classes and tests only the
proposed whitespace and supplied legacy-link trials in memory. It does not enable
production changes. All 169 non-hidden records passed the trial mappings; the two
remaining current failures were title formatting. See the
[failure-class evidence](api-survey.md#approved-metadata-failure-class-diagnostic).
Run all discovery and failure-class guards offline, with live entries ignored:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --lib library::discovery::tests --locked
```

```powershell
pwsh -NoProfile -File .\scripts\test-e2e-harness.ps1
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-cli --test inventory --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test --workspace --locked
pwsh -NoProfile -File .\scripts\cargo.ps1 test --workspace --all-features --locked
```

Ordinary Rust tests use synthetic stores and loopback fixtures, not native
storage or live accounts. Library missing-session and cancellation checks also
use local public-classification fixtures. The script checks bounded output and input, protocol
errors, process and suite deadlines, cleanup, report redaction, inventory
omissions, failure propagation, readiness, and submission lifecycle failures.
It does not dispatch a workflow or upload a secret.
Ordinary CI runs these synthetic harness checks on its platform matrix. It does
not run native-store cases or the manual account suite.

Run the other [workspace checks](../CONTRIBUTING.md#workspace-checks) and
[Markdown checks](../CONTRIBUTING.md#markdown-and-local-links) after changes.
