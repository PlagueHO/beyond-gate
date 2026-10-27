# Authentication design

**Planned, not implemented.** Simple local login is a core requirement.
The [2026-10-27 feasibility assessment](architecture.md#feasibility-assessment)
did not establish a permitted path. Live login and session experiments remain
subject to that gate. An ignored, developer-only
[session-exchange probe](#opt-in-session-exchange-probe) is now available.
The user reported a successful manual exchange on 2026-10-27. The probe does not
implement product login.

## Required user experience

1. The user starts the proposed `beyond-gate auth login` command.
2. Beyond Gate opens the user's installed browser for human-controlled login.
3. Beyond Gate obtains only the session material needed for authorized reads.
4. It stores persistent session material in protected OS credential storage.
5. The CLI and MCP server use that same local session without repeated setup.

These steps describe the desired experience, not a verified capture mechanism.
D&D Beyond is not assumed to expose an OAuth callback for this project.
Do not promise one. Evaluate a permitted browser-mediated handoff before choosing
a browser integration. Do not read unrelated browser profiles or cookies.
Beyond Gate must never ask for the account password.

## Browser handoff assessment

**Reviewed: 2026-10-27. Result: unproven; upstream clarification required.**
Opening the installed browser does not establish a way for this app to receive a
session. The missing step is an upstream-supported return path after human login.
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
token exchange from guesses. Manual cookie import also requires upstream
permission and does not satisfy the simple-browser-login requirement.

To establish a viable handoff, upstream must approve this native client and
document how it receives minimal session material after human login. It must also
define allowed hosts, expiry, refresh, and revocation. Only then select the
simplest supported return method. Check its success, cancellation, timeout,
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
not a claim of an officially supported flow. Further live use remains subject to the
[next gate](architecture.md#permission-questions-and-next-gate).

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
Proceed only with your own session and the required permission for the experiment.

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

If a permitted upstream flow requires a session cookie to short-lived token
exchange, the future session design must support it. Determine actual cookie
names, hosts, token fields, expiry, and refresh behavior in the feasibility work.
Do not hard-code guessed endpoints or assume that cookies never expire.

Store persistent secrets in the native credential store: Windows Credential
Manager, macOS Keychain, or a supported Linux secret service. Keep short-lived
tokens in memory when possible. Store non-secret settings separately in the
user's OS configuration directory. Do not silently fall back to plaintext when a
credential store is locked or missing.

Proposed `auth status` output reports only whether login is usable and when it
expires. It must not display secrets. Proposed `auth logout` removes local
credentials and memory caches. It must state whether upstream revocation is
available; local removal alone is not upstream revocation.

Handle expiry explicitly. Bound token refresh and do not repeatedly retry a
rejected session. Give the user a clear login action. MCP tools must report that
human login is needed, not ask an agent to collect a cookie or password.

## Early proof and fallback

A manually imported session cookie may prove a permitted technical path or provide
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
- How should Linux users recover when a protected credential service is unavailable?

These remain open after the assessment. Resolve them through the
[permission questions](architecture.md#permission-questions-and-next-gate) and
linked [roadmap](roadmap.md). Do not weaken protected storage or bypass bot
protection to make login appear successful.
