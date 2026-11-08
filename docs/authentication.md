# Authentication design

**Windows human-controlled login implemented and verified with Chrome and Edge Dev.**
The manual extension, native host, and interactive CLI use the
[version 1 browser handoff](browser-handoff.md). Approved product checks covered
human pairing, direct-service validation, protected save, restart/status, logout,
and cleanup. See the [scope and evidence](testing.md#approved-windows-product-login-verification).
Windows session validation and process-local reuse are implemented and tested
with synthetic services and stores. Windows library metadata discovery now uses
that lifecycle for bounded Web-catalog reads. Approved native CLI/MCP checks
returned matching metadata and evidence. Other live resource reads remain unsupported.
macOS/Linux browser login and production session use remain deferred.
Linux implementation is tracked in [#52](https://github.com/PlagueHO/beyond-gate/issues/52).

The user accepted the
[Windows pairing boundary](browser-handoff.md#accepted-windows-pairing-boundary).
The one-use, user-entered key authorizes the application connection. Independent
extension-launch proof is not claimed. Installed configuration, code, and process
memory remain trusted. The accepted
[service rule](browser-handoff.md#accepted-windows-service-rule) and
[cookie contract](browser-handoff.md#selected-windows-cookie-contract) define the
shared core boundary agreed with #30. Only a versioned cookie is persisted.
The bearer is transient, and cookie expiry remains unknown.

Live account tests and credential access still need separate explicit approval
under the [current policy](roadmap.md#development-policy-and-technical-status).
The earlier browser assessments and the
[developer probe](#opt-in-session-exchange-probe) below retain dated evidence.
They do not describe the current Windows implementation or authorize new tests.

## Browser login window

The [manual package](../beyond-gate-browser-extension/manifest.json) opens a separate browser
window after an explicit toolbar action. It is not an action popup that closes
when focus moves. The user selected this form so the terminal and browser can
remain visible together.

The form provides a fixed D&D Beyond sign-in link. Navigation and password entry
remain user actions. Loading the extension or opening its window does not query
cookies, connect a native host, or send a service request. The user must enter
the terminal's one-use key and approve the cookie read, validation, and protected
session replacement.

The [browser controller](../beyond-gate-browser-extension/flow.mjs) requires the pinned
extension ID and a regular context before pairing. It uses exactly the selected
cookie query after bound pending authorization. Missing, ambiguous, partitioned,
expired-metadata, or malformed candidates fail; no alternate profile or store
is enumerated. No key or cookie is written to browser storage or logs.
Browser/JavaScript-owned memory copies have no erasure guarantee.

The window waits for final acknowledgement before showing saved-session
evidence. Cancellation during cookie selection prevents later submission.
Cancellation after submission waits for the native outcome; disconnect or
unconfirmed completion cannot prove unchanged storage. Closing the window stops
its handoff. A started write can remain uncertain.

The [shared production recipes](fixtures/windows-login-v1.json) contain invented
keys, cookies, and results. Browser tests and Rust codec/relay tests use them.
The package's public SPKI key derives the same ID as the native host.
Offline browser logic and synthetic UI rendering checks have passed. These
checks do not prove real native registration, account access, or protected save.

**Accepted setup boundary:** the user approved native-host registration as trusted
installed configuration. This includes registry entries, manifests, and the
mapping that selects the launched executable. The browser preface passes the
pairing key to that trusted host. A same-user registry change can redirect the
launch without changing the original executable. The current flow does not
protect the key, cookie, or browser result from a substituted host. Origin checks
and host-to-CLI encryption do not remove this limit.

Windows setup now has explicit
[install and uninstall commands](cli.md#auth-install-and-auth-uninstall).
They check conflicts before confirmation and do not overwrite foreign setup.
This trust approval does not authorize new account or credential-store tests.

## Local session commands

The CLI exposes `auth login`, `auth status`, `auth validate`, and `auth logout`.
Local presence and deletion started in
[#3](https://github.com/PlagueHO/beyond-gate/issues/3).
Windows browser login now has approved product verification.

| Command | Current result |
|---------|----------------|
| `beyond-gate auth login` | Windows: requires three interactive streams, starts one-use pairing, and calls shared completion only after a bound submit. Real Chrome and Edge Dev login/save checks passed. Other platforms retain the unsupported error. |
| `beyond-gate auth status` | Exits with code 0 for `absent` or `stored-unverified`. Reports only local presence on stdout. It does not check current upstream usability; expiry is unknown. |
| `beyond-gate auth validate` | Windows: loads the fixed protected entry and makes one fresh service exchange. Success writes safe validation JSON with code 0. Other platforms fail before credential or service access. |
| `beyond-gate auth logout` | Exits with code 0 after local deletion or when the entry is already absent. States that upstream revocation is not implemented. |

Storage errors exit with code 1 and use stderr. Invalid command usage exits with
code 2. A locked store or a failed service is not an absent session. Status does
not modify a credential. Logout deletes its entry without first reading its
secret, so it can also remove invalid stored data.

There is no credential import command. Do not create a credential manually to
work around failed browser setup. The Windows CLI calls the core write
boundary after paired submission. Offline tests use synthetic values only.
No credential argument,
environment input, password prompt, or MCP credential input is added.
The MCP `auth_validate` tool uses the same core validation as the CLI.
Both explicit validation operations can contact the fixed authentication service.
Resource read tools use the protected stored session for supported reads.

### Protected-store selection and recovery

The core selects the backend explicitly through
[`keyring-core`](https://docs.rs/keyring-core/latest/keyring_core/).
It does not install a global default store, enumerate credentials, or select a
mock, sample, file, kernel-keyring fallback, or plaintext backend.

| Platform | Selected backend | Requirements and recovery |
|----------|------------------|---------------------------|
| Windows | [windows-native-keyring-store](https://docs.rs/windows-native-keyring-store/latest/windows_native_keyring_store/) | Uses Windows Credential Manager. Writes request `Local` persistence, not roaming `Enterprise` persistence. Use the same Windows account and an available credential service. Resolve access errors in the OS. |
| macOS | [apple-native-keyring-store](https://docs.rs/apple-native-keyring-store/latest/apple_native_keyring_store/) with `keychain` | Uses the user's login Keychain, not the sandbox-only protected-data backend. Unlock the login keychain and review OS access prompts. |
| Linux | [zbus-secret-service-keyring-store](https://docs.rs/zbus-secret-service-keyring-store/latest/zbus_secret_service_keyring_store/) with `crypto-rust` | Requires a user session D-Bus service and a Secret Service provider with an available default collection, such as GNOME Keyring or a compatible KDE Wallet. Start or unlock it through the desktop's normal tools, then run the command in that user session. No OpenSSL or libdbus build dependency is added. |

The service identity is `beyond-gate`; the account identity is `local-session`.
Both adapters must use these same core operations. No actual username is part of
the entry identity. On Windows the default target name is
`local-session.beyond-gate`. Only this entry can be read, written, or removed.
An ambiguous match is an error; no duplicate is selected or deleted automatically.

Headless Linux and WSL do not always provide a session bus or default collection.
If protected storage cannot work in that environment, the command fails. Do not
use a file or environment variable as a replacement. No password collection or
headless unlock script is supplied. Other operating systems fail explicitly.

Backends do not expose identical errors. Access denied and locked storage share
one category when they cannot be distinguished. A service connection failure can
be a general storage failure. Diagnostics omit raw native error messages and
credential values, but retain a typed cause behind a redacted error wrapper.

Opaque session values must be nonempty and at most 8 KiB. Native stores can impose
a smaller limit; such failures are explicit. This bound is a local storage rule,
not an upstream cookie or token contract. Core-owned secret allocations are cleared
on drop. This does not guarantee removal of copies held by OS APIs or callers.

The selected Windows backend uses generic credentials.
[`CREDENTIALW`](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw)
limits their secret blob to 2,560 bytes. The Windows establishment encoding
uses eight bytes of overhead and at most 2,552 cookie bytes. The original 8 KiB
opaque storage bound remains unchanged for local presence checks.
Unknown persistent versions and malformed envelopes fail safely; no migration,
truncation, split-entry storage, or plaintext fallback is selected.

The shared lifecycle holds derived bearers only in a process-local cache.
Logout clears its owner's evidence before local deletion. Other owners detect
the deleted entry at their next protected load. Logout does not revoke an
upstream session, log the browser out, or stop another process's in-flight
request. No upstream revocation contract has been established.

### Offline checks and remaining work

Unit tests use isolated in-memory credentials. They check shared entry identity,
write/read/delete, exact size bounds, invalid stored material, denied or locked
storage, missing backends, native failures, ambiguous entries, repeated logout,
and redacted error chains. CLI tests check local output and failures with injected
operations. Process tests exercise redirected-login refusal, help, and rejected input without
accessing real credentials. MCP process tests check local stdio serving and
library discovery parity through invented local services and unsupported errors
for other live reads.

No native credential-store round trip or live account test is part of these
checks. Platform CI checks build compatibility, not real credential-service access.
A native-store smoke test requires separate explicit credential-access approval.

Windows establishment tests cover cancellation, cookie exchange failures,
deadlines, service rejection, expired bearer evidence, and uncertain writes.
Portable lifecycle checks cover reuse, fresh validation, exact bearer expiry,
bounded exchange, rejection, store failures, and cache cleanup. A process test
uses the actual CLI runner and rmcp stdio server with the same invented sessions
and local services. Their validation data and safe diagnostics agree.
Live library adapter acceptance and approved real lifecycle checks remain separate.
The developer probe below remains separate.

## Required user experience

1. The user installs the manual extension and registers its Windows native host.
2. The user starts `beyond-gate auth login` in an interactive terminal.
3. The user opens the extension window and signs in in the regular browser.
4. The user enters the one-use terminal key and approves validation and save.
5. Beyond Gate queries the selected cookie, validates it, and confirms protected
   storage and final receipt before reporting success.

The CLI does not open or control the browser. Password entry stays in the
browser. No OAuth callback, credential import, or environment-based fallback
is implemented. Explicit CLI/MCP validation is available on Windows.
Production resource reads remain unsupported.

## Browser handoff assessment

**Historical assessment.** The Windows implementation and approved evidence
above supersede the return-path blockers in this section.

**Reviewed: 2026-10-27. Result: no working product handoff established.**
Opening the installed browser does not establish a way for this app to receive a
session. The missing step is a working return path after human login.
The [architecture evidence table](architecture.md#official-evidence) records the
sources and their dates.

D&D Beyond Terms & Conditions section 2.1 describes account setup and changing
login methods. Avrae's Getting Started guide describes linking D&D Beyond and
Discord accounts. Neither establishes a native-app authorization flow for Beyond
Gate. A supported flow for Avrae is not a transferable authorization grant.

| Flow detail | Evidence result for Beyond Gate |
|-------------|---------------------------------|
| Runtime login and API hosts | The user reported a successful probe exchange with `auth-service.dndbeyond.com`. Other runtime hosts remain unverified; documentation hosts are not a runtime allowlist. |
| Client registration and access scopes | No supported contract established. |
| Browser callback or other session handoff | No supported loopback callback, custom URI callback, device flow, or other return contract established. |
| Minimal persistent session material | The manual proof used `CobaltSession`; no persistent storage format or minimum product credential set is selected. |
| Cookie-to-token exchange | The user reported one successful live probe below. This verifies the candidate response shape for that attempt, not an official or stable contract. |
| Token lifetime and refresh | Unknown; no expiry values or refresh rules established. |
| Logout and upstream revocation | No upstream revocation contract established. Local removal alone cannot prove revocation. |
| Endpoint stability | No supported version or compatibility commitment established. |

Do not select a callback protocol, browser extension, cookie capture method, or
token exchange from guesses. Manual cookie input does not satisfy the
simple-browser-login requirement and is limited to the separate developer probe.

To establish a viable handoff, demonstrate how the app receives minimal session
material after human login. Establish the selected hosts, expiry, refresh, and
revocation behavior. Select the simplest working return method without bypassing
access controls or reading unrelated browser data. Check success, cancellation,
timeout,
rejected login, and expiry behavior with synthetic offline cases first.

### Technical evaluation for interactive use

The [interactive-use assumption](architecture.md#interactive-use-and-evaluation-assumption)
does not remove the need for a session return path. These general browser
properties were reviewed on 2026-10-27. They are not observations of a D&D Beyond
account or its actual cookies.

[RFC 8252](https://www.rfc-editor.org/rfc/rfc8252.html#section-4.1), published in
October 2017, describes native OAuth login through an external browser. The
authorization server returns a code through a redirect, and the app exchanges it
for tokens. Section 6 requires PKCE, which binds the code exchange to the app that
started it. Section 7 describes possible return methods. This is a candidate
design only if upstream supports it; a local listener alone cannot make upstream
issue a code or accept a new redirect URI.

[MDN's cookie guide](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Cookies)
explains that browsers send cookies within their domain and path scope.
`HttpOnly` prevents page JavaScript from reading a cookie. A website session is
not automatically a session in a separate native HTTP client. A browser redirect
to a local app does not by itself transfer the website's cookies.

| Candidate | Technical finding | Remaining evidence needed |
|-----------|-------------------|---------------------------|
| Open the normal website for manual browser lookup | Keeps authentication and reading in the browser; it does not return structured results to CLI or MCP. | A separate supported data handoff would be needed to meet the current product goals. |
| External-browser OAuth with a native return path | A standard can support human login without exposing the password to the app. It requires server support and client configuration. | Upstream authorization endpoint, client registration, redirect rules, PKCE support, token exchange, and read scopes. |
| Reuse the browser's session in a native client | The user reported a successful exchange after manual hidden-prompt input. No automatic browser profile or cookie extraction was attempted. | A simple browser handoff, the minimum product credential set, allowed data-service hosts, and expiry rules. |

The manual session exchange now has a user-reported live success. The simple
automatic native handoff remains unproven: no supported browser return contract
has been established. A manual cookie exchange does not establish that return
path, token expiry, or access to a data service.

The initial official-source review found no session response contract. Later
community evidence and the user's probe result support the candidate exchange,
not a claim of an officially supported flow. Further live account tests need
separate explicit approval under the
[live-test guidance](architecture.md#integration-evidence-and-live-tests).

### Existing-browser decision for issue #29

**Initial review: 2026-10-31. Result: no verified return path established.**
The later accepted extension design supersedes this existing-browser-only scope.
For [#29](https://github.com/PlagueHO/beyond-gate/issues/29), the user initially selected
the existing browser only. This scope excludes an extension, separate browser
profile, embedded browser, and browser debugging connection. It does not approve
live account tests or native credential access.

The review read the issue, its comments and artifact list, the original #3, and
the separate session-lifecycle issue #30. No comments or artifacts were returned
for #29. The review also checked the public sources below. A targeted public
search found no additional documented return contract for Beyond Gate.
This limited finding does not prove that no private partner integration exists.

| Source | Finding | Limit |
|--------|---------|-------|
| [D&D Beyond Authentication Information](https://dndbeyond-support.wizards.com/hc/en-us/articles/7747237593108-Authentication-Information) | Describes login providers and mobile account linking. | Does not specify third-party client registration or a return path for this app. |
| [D&D Beyond Terms & Conditions](https://www.dndbeyond.com/en/terms-conditions) | Provides access and account terms. | Is not an authentication protocol or callback specification. |
| [RFC 8252, section 7](https://www.rfc-editor.org/rfc/rfc8252.html#section-7) | Defines native OAuth return methods that require authorization-server support. | Does not establish D&D Beyond support for any method. |
| [MDN cookie security guidance](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Cookies#security) | Explains cookie scope and the `HttpOnly` restriction on page JavaScript. | Does not establish the attributes of an actual D&D Beyond cookie or transfer one to this app. |

Opening the website in the existing browser does not return session material to
Beyond Gate. A local listener cannot make upstream accept a callback. The manual
probe starts with an already supplied cookie, so its successful exchange does
not fill that gap. Do not copy another application's client identity, assume a
redirect is supported, or weaken browser protections.

At that checkpoint, implementation stopped at the missing contract. `auth login`
returned an explicit failure before browser, input, storage, or network activity. No callback
listener, product token exchange, cookie import, or simulated successful login
was added. Synthetic storage and probe tests did not satisfy the product criteria.

To resume within this scope, establish these facts from protocol evidence:

1. A login entry point and return method intended for this app, including any
   client registration, redirect rules, scopes, and transaction binding.
2. The minimum returned session material, exact credential destinations, and any
   required exchange host, operation, and response contract.
3. How to verify the result before saving it, with bounded waiting, request and
   response limits, cancellation, rejection, and cleanup rules.

Agree on the stored-session boundary with
[#30](https://github.com/PlagueHO/beyond-gate/issues/30) before changing its format.
That issue owns shared session validation, expiry evidence, bounded refresh,
and authenticated CLI/MCP use. The current opaque storage limit is not a
verified upstream format, and local presence is not proof of usable login.

That review left #29 blocked. Later user decisions and Windows product evidence
resolved this return-path blocker. Any new live check still needs separate
approval for exact hosts, operations, request budgets, stop conditions, and cleanup.

### Extension feasibility evaluation

**Scope update: 2026-10-31. Result: candidate transport; account login unverified.**
This is dated feasibility evidence, not the current product status.
The user then approved evaluation of a narrowly scoped extension with synthetic
tests. This replaces the earlier exclusion of extension evaluation. The user
subsequently ran a test extension and native host in Edge. This does not approve
live account tests or native credential-store access.

The browser platform offers a possible return path without an upstream OAuth
callback. [Chrome's cookies API](https://developer.chrome.com/docs/extensions/reference/api/cookies)
can query cookie objects with a name and URL. It requires the `cookies`
permission and host permissions. Cookie objects include `HttpOnly` values through
this privileged API; page JavaScript restrictions are a different boundary.
[Mozilla documents the same cookie fields](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/cookies/Cookie).
This is not evidence that a particular D&D Beyond cookie is readable, sufficient,
or usable outside its browser session.

**Permission limit:** host permissions cannot restrict the extension to one
cookie name. The extension code must enforce that limit. The granted capability
is broader than the intended single-cookie operation, even with an exact host.
Review this risk before selecting the product design. Do not claim that a
manifest makes access to other cookies on that host impossible.

[Chrome native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)
can send JSON between an extension page or service worker and a registered
native process over stdio. Each frame has a native-endian 32-bit byte length.
It does not require an HTTP listener or sending credentials in a URL.
The native host manifest restricts allowed extension origins without wildcards.
[Edge](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging)
uses the same transport with its own registration.
[Firefox](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging)
requires different host-manifest fields and caller-argument handling.
Browser support is a platform contract, not upstream permission.

The user requested both Edge and Chrome as target browsers. The same candidate
design can use their Chromium extension APIs, but each browser needs appropriate
native-host registration and allowed extension IDs. A published extension ID can
differ between stores. The later
[isolated browser suite](testing.md#isolated-browser-feasibility) passed synthetic
registration, handoff, rejection, and cleanup tests in both Chrome and Edge.
Those feasibility checks alone did not establish product login.

#### Synthetic evidence and limits

A session-only prototype uses `login.beyond-gate.invalid`, an invented cookie
name, and values restricted to the `synthetic-` prefix. It has no production
host, cookie name, network requests, OS-store access, or installation code.
Its manifest is a permission fixture, not an installed or complete extension.
Node.js runs these development tests only; neither product binary gains a
Node.js runtime dependency.

The prototype passed 14 synthetic tests. They cover consent gating; one
URL/name/path-filtered query; absent, ambiguous, expired, wrong-scope, partitioned,
and malformed cookie rejection; the exact 8 KiB value bound; safe error output;
cancellation and timeout; late-result suppression; port cleanup; request binding;
and native-frame size, UTF-8, JSON, and truncation checks. A separate child process
uses real framed stdio to check receipt, incorrect binding, extra fields, and
replay rejection. Replies confirm only synthetic receipt, never authentication
or a saved session.

The original prototype mocks the browser APIs and consent action. It does not
test a real extension runtime or browser-to-host launch. It does not connect to
a pending CLI login or implement
protected storage, exchange, or #30's lifecycle. Its random challenge binds test
messages only; it is not proof of an authorized pending login. JavaScript
strings and browser-owned copies cannot be guaranteed to clear from memory.
The prototype's cookie attributes are invented, not product validation rules.
The test sources remain session artifacts, not a shipped login path.

#### Edge synthetic browser test

On 2026-10-31, the user reported a pass from a separate session-only test
extension in Edge and shared a screenshot of the popup result. The screenshot
shows the consent box selected and the message: `PASS: Edge sent the fake cookie
to the local test host. No login was validated or saved.`

The test package serves an invented `HttpOnly` cookie from `127.0.0.1`. Its
extension checks the exact synthetic cookie scope and sends the fake value to a
local Rust native-messaging host. The host acknowledges the request before the
popup reports a pass. This user-reported result exercises the Edge extension
runtime, cookie API, native-host registration, and host launch on the successful
synthetic path. It is not evidence of a D&D Beyond session or product login.

The screenshot does not establish permission-denial behavior, rejection of
another extension, cancellation, timeout, or registration cleanup. The native
host checks the format of its caller-origin argument; it does not authenticate
an arbitrary local process that could supply such an argument. The browser
registration restricts which extension Edge may launch it for, but that
restriction alone does not bind the host to a pending CLI login.

## Opt-in session-exchange probe

The user requested implementation of live testing on 2026-10-27. The initial scope
is authentication only: one manually initiated exchange, with no character,
campaign, or book reads. The test is in
`src/beyond-gate-core/tests/auth_probe.rs` and is ignored by default.
Blocking reqwest and rpassword are development requirements of this probe.
Async reqwest and serde_json are also normal dependencies of the separate
[core HTTP boundary](architecture.md#http-boundary).
Neither product binary invokes the probe. Windows product login uses the
separate shared establishment exchange.

### Community evidence, not an official API contract

[DDB Importer's setup guide](https://docs.ddb.mrprimate.co.uk/docs/ddb-importer/initial-setup#cobalt-cookie)
describes using a `CobaltSession` cookie for authenticated requests. Its proxy's
[configuration](https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/config.js)
and [authentication code](https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/auth.js),
reviewed on 2026-10-27, identify this candidate:

- Send one `POST` to `https://auth-service.dndbeyond.com/v1/cobalt-token`.
- Supply `CobaltSession` as a cookie and request JSON.
- Expect HTTP 200 with a nonempty string field named `token`.

These are community implementation observations, not an upstream permission
grant, a stable contract, or proof that this account can read a particular resource.
The cookie and the returned bearer token are different credentials. This probe
accepts only the cookie. It does not infer a token lifetime or try a refresh.

### Run manually

Run this exact command in an interactive terminal from the repository root.
Use the repository wrapper. On Windows, it selects
[Visual Studio 2022 Build Tools](../CONTRIBUTING.md#windows-build-environment)
and runs Cargo in the same process. Missing C headers in `aws-lc-sys` are a build
setup failure, not a rejected session. The linked instructions include a
compile-only check.
Do not run the command through an agent or a recorded terminal session.
Proceed only with your own session and separate explicit approval for the
experiment.

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 test -p beyond-gate-core --test auth_probe --locked live_authentication -- --ignored --exact --nocapture
```

The test requests the cookie through a hidden terminal prompt. Do not put it in
the command, an environment variable, a file, chat, logs, or MCP configuration.
Empty input cancels without sending a request. Noninteractive input is rejected.
The returned token is not displayed, returned to another operation, or saved.
The probe makes no account-data read and stores no credentials or content.

The request uses certificate-validated HTTPS to the fixed host above. It disables
system and environment proxies, redirects, and retries. Connection setup has a
5-second timeout; the exchange has a 15-second timeout. The response is limited
to 64 KiB, and the input cookie to 8 KiB. These are probe limits, not upstream limits.

Success means only that HTTP 200 returned the candidate token shape. It does not
validate the token at another service, prove book access, establish expiry, or
solve automatic browser handoff. Non-200 responses, including redirects, denied
access, rate limits, and service errors, stop the test. Malformed or oversized
responses also fail. Error output omits response bodies and credentials.
Do not add a workaround after a denial.

### Offline coverage and current result

The ordinary workspace tests use synthetic values and loopback HTTP mocks only.
They cover request headers, candidate success and malformed response shapes,
401/403 session rejection, rate limits, service errors, redirect rejection,
HTTPS-only live configuration, exact size limits, input validation, and redacted
diagnostics. Rejection cases do not prove a specific upstream expiry policy.

**Live result: passed, user-reported on 2026-10-27.** The user supplied the terminal
result for the original direct Cargo invocation: one live test passed. The probe reported that
the token exchange succeeded and the token was discarded. By the test's checks,
the response was HTTP 200 with a nonempty string `token` field. Right-click paste
worked at the hidden prompt in the user's terminal.

This result was reported by the user, not independently rerun by the agent.
No token or raw response was supplied or recorded. It establishes one successful
manual candidate exchange only. Character, campaign, and book reads, token use at
another service, expiry, refresh, revocation, and automatic browser handoff remain
untested. No further live operation is approved by this result. Record later
outcomes and dates without credentials, token payloads, or raw responses.

## Session lifecycle

### Validation and evidence

`beyond-gate-core::auth::session::Session` owns the shared lifecycle.
Its construction does not open storage or contact the service. Production
validation and reuse are Windows-only. The policy and synthetic tests are
portable so [Linux support](https://github.com/PlagueHO/beyond-gate/issues/52)
can use the same rules after its platform contract is implemented.

CLI `auth validate` and MCP `auth_validate` always make one fresh exchange,
unless loading fails or the owner has already rejected the same material.
Neither operation accepts credentials or writes the stored cookie again.
Success is the same JSON object in both adapters:

```json
{"validation":"service_validated","expiry":"unknown","bearer_expiry":"future"}
```

This proves accepted direct-service bearer evidence at validation time.
It is not a content-access grant, independently verified JWT signature,
protected-save result, or guarantee of future upstream acceptance.
`auth status` remains local-only and reports `stored-unverified` for presence.
The browser receipt and `SavedSession` keep their existing distinct meanings.

### Reuse and request bounds

Before each use, the core rereads the fixed protected entry off the async
executor. It binds cached evidence to the exact cookie envelope. An absent or
changed entry invalidates the prior bearer. A failed read, failed fresh
validation, or cancelled exchange cannot restore old bearer evidence.

The core uses both the service-issued wall-clock expiry and an original
monotonic deadline. A backward clock change cannot extend bearer reuse.
Exact expiry stops reuse. The next operation can make one new cookie exchange;
there is no separate refresh endpoint or background refresh worker.

| Boundary | Limit |
|----------|-------|
| Whole operation, including queue, storage, exchange, and any read | 30 seconds |
| Protected-store waiting | 5 seconds |
| Service connection | 5 seconds |
| One service request and its body | 15 seconds |
| Service exchanges per operation | At most one; no automatic retry |
| Service response | At most 64 KiB; JSON with identity encoding |
| Derived bearer | At most 8 KiB; memory only |
| Persistent cookie envelope | At most 2,560 bytes; unchanged BGSE v1 format |

The service transport forbids redirects, proxies, and transport retries.
`Session::get_json` applies memory-only bearer headers to the existing bounded
HTTP client after exact origin checks. Its read deadline cannot exceed bearer
expiry. It adds no production resource host or endpoint. Issues #31, #32, and #33
still own live source and entitlement contracts.

Core-owned cookie and bearer allocations and temporary header strings are
cleared on drop. OS, HTTP-header, and transport-owned copies have no complete
erasure guarantee.

### Failures, cancellation, and logout

| Category | Meaning |
|----------|---------|
| `session_absent` | The fixed native entry is missing; human browser login is required |
| `session_rejected` | The service or an authenticated read rejected credentials; human browser login is required |
| `session_expired` | Service-issued bearer evidence expired; cookie expiry remains unknown |
| `invalid_material` / `unsupported_contract` | Invalid bytes or unknown persistent format; no migration or fallback |
| `storage_access` / `storage_unavailable` / `storage_failure` | Protected storage failed; not absence |
| `ambiguous_entry` | Multiple matching credentials; no automatic selection |
| `validation_unavailable` | Transport or service response cannot establish usability |
| `timed_out` | The operation or protected-store wait reached its bound |
| `unsupported_platform` | Production validation and reuse are not implemented on this platform |

The owner clears bearer evidence after rejection. It does not exchange the
same rejected cookie again or delete it automatically. Different material
saved through human login can begin a new attempt. A new process has no durable
rejection history and can make one explicit bounded attempt. HTTP 401 does not
prove cookie expiry. HTTP 403 from a resource read remains content denial and
does not trigger credential refresh.

CLI Ctrl+C and MCP request cancellation drop pending validation work.
The [rmcp request context](https://docs.rs/rmcp/3.8.0/rmcp/service/struct.RequestContext.html)
supplies the MCP cancellation token. The protocol can omit a response to a
cancelled request. Cancellation never installs a late bearer result.
A started blocking native read can finish, but cannot install evidence.
CLI and MCP runtime shutdown do not wait for that worker. This preserves the
operation timeout, cancellation result, and MCP exit on transport closure even
if the native credential service stays blocked.

Local logout clears its owner's cache and deletes only the fixed protected
entry without reading the secret. Missing entries remain successful.
Failed deletion is an error, not confirmed removal. A started native deletion
can finish after a timeout or caller cancellation. Another process observes
removal at its next load, not during an in-flight request.
Local removal does not revoke upstream credentials or log the browser out.

### Manual-suite test exception

The [manual capability suite](testing.md) adds an off-by-default
`e2e-isolation` feature. It requires a random non-secret context and separate
native-storage approval. It uses only `beyond-gate-e2e` test entries, never the
product `beyond-gate` / `local-session` identity. A feature-required helper can
seed synthetic bytes and remove its own entry. It cannot import a live cookie.

The suite's live budget is zero. Its dispatch entry point stops before secret
input or upload while production session use and endpoint contracts are absent.
The separately tested future Actions lifecycle uses a unique repository secret
per submission and `gh secret set` stdin. This defines a development-only
exception to the local-probe rule, not an enabled product or CI login flow.
No general product credential environment variable or MCP credential parameter
is added. This manual live-resource suite remains blocked; the separately
verified Windows browser login does not enable it.

Windows establishment fixes the selected cookie, direct exchange host, response
rule, and persistent encoding. It keeps the derived bearer transient and saves
only the cookie. Shared bounded reuse does not assume that the cookie never
expires. Other-platform login must use the protected storage
boundary above, not plaintext when a store is locked or missing.

The manual suite discovers the new tool and rejects its invalid inputs without
credential access. Valid production validation stays outside its zero live
request budget. Synthetic process tests cover validation success and failure.

## Unresolved questions

- Does upstream permit a suitable local authentication flow?
- Which resource hosts and access contracts can support authenticated CLI/MCP reads?
- What additional real lifecycle evidence and upstream revocation contract can be verified?
- Which secure browser handoff can macOS and Linux support?

Windows login does not resolve these remaining questions. Use the
[integration questions](architecture.md#integration-evidence-and-live-tests) and
linked [roadmap](roadmap.md). Do not weaken protected storage or bypass bot
protection to make login appear successful.
