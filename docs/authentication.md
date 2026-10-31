# Authentication design

**Local storage implemented; browser login blocked.** Simple local login is a core requirement.
The [2026-10-27 feasibility assessment](architecture.md#feasibility-assessment)
did not establish a permitted path. Its blanket development stop gate is now
removed under the [current policy](roadmap.md#development-policy-and-technical-status).
This is not upstream approval or a working login implementation. Live account
tests and credential access still need separate explicit approval. An ignored,
developer-only
[session-exchange probe](#opt-in-session-exchange-probe) is now available.
The user reported a successful manual exchange on 2026-10-27. The probe does not
implement product login.
The [existing-browser review for #29](#existing-browser-decision-for-issue-29)
records the remaining return-path blocker and the evidence needed to resume.
The later [extension feasibility evaluation](#extension-feasibility-evaluation)
adds a candidate transport, not a working account login.

The [browser handoff contract checkpoint](browser-handoff.md) now records two
version 1 protocol proposals, a shared core API proposal for #30, and canonical
synthetic test cases for #29/#46. It is **blocked for product integration**:
application-peer authentication, the production cookie profile, and the core
validation/persistence contract remain unresolved. The cases are not yet consumed
by Rust or browser tests. This documentation does not enable login or credential
access.

## Local session commands

The CLI now exposes `auth login`, `auth status`, and `auth logout`. This is partial
offline work for [#3](https://github.com/PlagueHO/beyond-gate/issues/3), not a
completed browser-login flow.

| Command | Current result |
|---------|----------------|
| `beyond-gate auth login` | Exits with code 1. Reports the missing working browser handoff on stderr. No browser, secret input, credential-store access, or request occurs. |
| `beyond-gate auth status` | Exits with code 0 for `absent` or `stored-unverified`. Reports only local presence on stdout. Stored material has not been validated upstream; expiry is unknown. |
| `beyond-gate auth logout` | Exits with code 0 after local deletion or when the entry is already absent. States that upstream revocation is not implemented. |

Storage errors exit with code 1 and use stderr. Invalid command usage exits with
code 2. A locked store or a failed service is not an absent session. Status does
not modify a credential. Logout deletes its entry without first reading its
secret, so it can also remove invalid stored data.

There is no product command to create or import a credential. Do not create one
manually to work around blocked login. The core write boundary exists for a future
approved flow and uses synthetic values in tests. No credential argument,
environment input, password prompt, or MCP input is added. Neither adapter uses
stored material for an upstream request. The MCP server exposes only read tools,
which currently return the core's explicit unsupported-operation errors.

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

There is no token cache in this implementation. Logout does not revoke an
upstream session, log the browser out, or clear credentials held by another
process. No upstream revocation contract has been established.

### Offline checks and remaining work

Unit tests use isolated in-memory credentials. They check shared entry identity,
write/read/delete, exact size bounds, invalid stored material, denied or locked
storage, missing backends, native failures, ambiguous entries, repeated logout,
and redacted error chains. CLI tests check local output and failures with injected
operations. Process tests exercise blocked login, help, and rejected input without
accessing real credentials. MCP process tests check local stdio serving and
explicit unsupported live-read errors.

No native credential-store round trip or live account test is part of these
checks. Platform CI checks build compatibility, not real credential-service access.
A native-store smoke test requires separate explicit credential-access approval.

Usable, expired, and rejected upstream session states remain deferred. So do
browser cancellation, cookie exchange failures, timed expiry, bounded refresh,
and actual CLI/MCP authenticated-session parity. They need a working browser
handoff and tested exchange, expiry, refresh, and host contracts first.
The manual developer probe below remains separate and unchanged.

## Required user experience

1. The user starts `beyond-gate auth login` after the browser handoff is implemented.
2. Beyond Gate opens the user's installed Edge or Chrome browser for
   human-controlled login.
3. Beyond Gate obtains only the session material needed for authorized reads.
4. It stores persistent session material in protected OS credential storage.
5. The CLI and MCP server use that same local session without repeated setup.

These steps describe the desired future experience, not the current blocked command
or a verified capture mechanism.
D&D Beyond is not assumed to expose an OAuth callback for this project.
Do not promise one. Evaluate a working browser-mediated handoff before choosing
a browser integration. Do not read unrelated browser profiles or cookies.
Beyond Gate must never ask for the account password.

### Requested fallback

The user requested a fallback when browser handoff is unavailable. Direct
credential values in CLI arguments or MCP tool inputs are not part of the
design. Those inputs can expose a session through process information or client
records. MCP tools must not accept credentials.

One option to evaluate is a user-initiated, one-shot import from a temporary
process environment into the shared core, followed by validation and protected
OS storage. The value must not be placed in command arguments, shell history,
logs, or MCP client configuration. The CLI and MCP server would then use the
same stored session. This is a proposal, not an implemented fallback.

An environment variable is not equivalent to protected OS storage. Parent or
child processes and diagnostics can retain or expose its value. Define the
trusted injection method, user consent, lifetime, failure behavior, and cleanup
before selecting this option. Do not assume that the application can erase
copies held by another process. Any test with a real credential still needs
separate approval.

## Browser handoff assessment

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

Implementation stops at this missing contract. `auth login` retains its existing
explicit failure before browser, input, storage, or network activity. No callback
listener, product token exchange, cookie import, or simulated successful login
is added. Cancellation, timeout, rejected login, and successful protected handoff
remain unimplemented; synthetic storage and probe tests do not satisfy them.

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

Issue #29 remains blocked, not complete. A verified contract can permit offline
implementation and synthetic flow/process tests. It does not approve a live
experiment. Any live check still needs separate approval for exact hosts,
operations, request budgets, stop conditions, and cleanup. Expanding the browser
integration beyond the approved synthetic extension evaluation also needs a new
user decision.

### Extension feasibility evaluation

**Scope update: 2026-10-31. Result: candidate transport; account login unverified.**
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
differ between stores. The synthetic browser test passed in Edge only; Chrome
still needs its own synthetic registration, handoff, failure, and cleanup tests.
Neither browser has a product login implementation yet.

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

#### Remaining validation gates

Continue with synthetic data. Verify permission denial, rejection of a different
extension, cancellation, timeout, and test-registration cleanup. On 2026-10-31,
the user reported removing the test host. Cleanup of the Edge extension and
local test site has not been confirmed.

A production design must also bind the separately launched native host to one
pending `auth login` in the same OS user context. Native messaging alone does not
connect it to an already running CLI process. Define authenticated local IPC,
its local-process threat model, one-use transactions, deadlines, duplicate and
replay rejection, and consent before reading or saving a session. Do not trust
a caller-supplied origin or challenge without that pending-login check. Do not
accept credentials from arbitrary local callers.

Only after the synthetic browser test and implementation contracts are established,
request separate approval for a bounded live experiment. Identify the exact
cookie scope and minimum material; verify any required exchange and successful
protected save. Stop on access denial or bot protection. Define hosts, operations,
request budgets, stop conditions, and cleanup before approval. Do not upload
cookie values, account responses, or browser data.

Issue #29 therefore remains incomplete, but the Edge test confirms one synthetic
transport path. Issue #30 still owns token use, expiry evidence, bounded refresh,
and authenticated adapter parity. Do not substitute the test host's receipt for
any of those results.

## Opt-in session-exchange probe

The user requested implementation of live testing on 2026-10-27. The initial scope
is authentication only: one manually initiated exchange, with no character,
campaign, or book reads. The test is in
`src/beyond-gate-core/tests/auth_probe.rs` and is ignored by default.
Blocking reqwest and rpassword are development requirements of this probe.
Async reqwest and serde_json are also normal dependencies of the separate
[core HTTP boundary](architecture.md#http-boundary).
Neither product binary invokes the probe or sends requests.

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
On Windows, initialize the
[Visual Studio developer environment](../CONTRIBUTING.md#local-development)
first. Missing C headers in `aws-lc-sys` are a build setup failure, not a rejected
session. The linked instructions include a compile-only check.
Do not run the command through an agent or a recorded terminal session.
Proceed only with your own session and separate explicit approval for the
experiment.

```shell
cargo test -p beyond-gate-core --test auth_probe --locked live_authentication -- --ignored --exact --nocapture
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
result for the exact command above: one live test passed. The probe reported that
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
is added. Browser login, exchange, expiry, and live adapter parity stay blocked.

If the selected upstream flow requires a session cookie to short-lived token
exchange, the future session design must support it. Determine actual cookie
names, hosts, token fields, expiry, and refresh behavior in the feasibility work.
Do not hard-code guessed endpoints or assume that cookies never expire.

The local storage boundary above uses Windows Credential Manager, macOS Keychain,
or Linux Secret Service. A future approved login must use that boundary. Keep short-lived
tokens in memory when possible. Store non-secret settings separately in the
user's OS configuration directory. Do not silently fall back to plaintext when a
credential store is locked or missing.

Future `auth status` output must report usability and expiry only when supported
evidence exists. Current status reports local presence only. It never displays
secrets. Current `auth logout` removes the local entry and states that upstream
revocation is unavailable. Future token caches must also participate in logout.
Local removal alone is not upstream revocation.

Handle expiry explicitly. Bound token refresh and do not repeatedly retry a
rejected session. Give the user a clear login action. MCP tools must report that
human login is needed, not ask an agent to collect a cookie or password.

## Early proof and fallback

A manually imported session cookie may test a candidate technical path or provide
an explicit fallback. It must not become the primary login experience.
If needed, read it from a no-echo interactive terminal prompt. Never accept a
secret as a CLI argument, copied MCP prompt, environment example, or logged value.
Define secure noninteractive input separately if a real need appears.

Tests must use synthetic cookie and token values. Test protected-store failure,
expired cookies, token expiry, denied login, cancelled login, and logout.
Do not commit account snapshots, HAR files, browser profiles, or content samples.

## Unresolved questions

- Does upstream permit a suitable local authentication flow?
- Can a simple browser handoff work without an extension or credential scraping?
- What minimal session material and host allowlist does the flow require?
- What expiry and token-exchange rules does upstream actually support?
- Which Linux environments provide the protected session service needed at runtime?

These remain open after the assessment. Resolve them through the
[integration questions](architecture.md#integration-evidence-and-live-tests) and
linked [roadmap](roadmap.md). Do not weaken protected storage or bypass bot
protection to make login appear successful.
