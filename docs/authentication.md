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
2. Beyond Gate opens the user's installed browser for human-controlled login.
3. Beyond Gate obtains only the session material needed for authorized reads.
4. It stores persistent session material in protected OS credential storage.
5. The CLI and MCP server use that same local session without repeated setup.

These steps describe the desired future experience, not the current blocked command
or a verified capture mechanism.
D&D Beyond is not assumed to expose an OAuth callback for this project.
Do not promise one. Evaluate a working browser-mediated handoff before choosing
a browser integration. Do not read unrelated browser profiles or cookies.
Beyond Gate must never ask for the account password.

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
