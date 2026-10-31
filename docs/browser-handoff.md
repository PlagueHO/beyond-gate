# Browser login contract checkpoint

**Status: proposed; blocked for product integration.** This document defines a
version 1 proposal for [#29](https://github.com/PlagueHO/beyond-gate/issues/29).
It does not implement login or establish agreement with
[#30](https://github.com/PlagueHO/beyond-gate/issues/30).
The local-process threat boundary below was selected by the user. The IPC
authentication mechanism, production cookie profile, and core validation contract
remain unresolved. Do not treat the message examples as an authenticated channel.

The [canonical synthetic cases](fixtures/browser-handoff-v1.json) are data for
future Rust and browser tests. No product parser, process test, or browser test
consumes them yet. A fixture pass is not proof of account authentication.
`beyond-gate auth login` still fails before browser, storage, or network activity.

## Evidence and ownership

The issue body, all comments, and artifact 1 of #29 were read for this checkpoint.
Issues #29, #30, and #46 had no comments at the time of review.
Artifact 1 records the user-reported successful synthetic Edge test and host
removal. It does not record a pending CLI connection or a protected save.
The [authentication evidence](authentication.md#extension-feasibility-evaluation)
retains the original observations and their limits.

| Work | Owner | Checkpoint boundary |
|------|-------|---------------------|
| Native host, browser registration, pending CLI login, protected handoff | [#29](https://github.com/PlagueHO/beyond-gate/issues/29) | This proposal; no runtime implementation |
| Session exchange, validation, persistent format, expiry and refresh | [#30](https://github.com/PlagueHO/beyond-gate/issues/30) | Proposed shared core API below; agreement still required |
| Chrome and Edge extension, manual package, real synthetic browser CI | [#46](https://github.com/PlagueHO/beyond-gate/issues/46) | Reuse both contracts and the same fixtures after acceptance |

Issue #46 blocks end-to-end completion of #29. Its synthetic browser feasibility work
can start independently. Do not make #46 depend on completion of its parent.
Issue #30 can develop synthetic lifecycle cases independently; neither issue depends
on completion of the other.

### Verified platform facts

[Chrome native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)
starts a separate host process and uses framed stdio. Its manifest has exact
`allowed_origins`, without wildcards. Browser-to-host frames can be much larger
than the local cap proposed here. These browser limits do not select product
limits or authenticate host-to-CLI communication.

[Edge native messaging](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging)
uses browser-specific registration. Edge can fall back to Chrome or Chromium
registrations. An origin argument identifies the reported caller; an arbitrary
local process can also supply that argument.

[Windows pipe security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)
allows explicit access controls and logon-session isolation. Default permissions
can grant read access beyond the intended user. Broad generic write access can
also grant permission to create pipe instances. Neither user identity nor a pipe
name proves that the peer is Beyond Gate.

## Local-process threat boundary

Trust the installed Beyond Gate binaries, selected browser, approved extension,
OS security services, and their protected memory. Protect against other OS users,
remote callers, endpoint impersonation, accidental callers, and unauthorized
direct launches of the native host, including launches under the same OS user.

Exclude administrators, OS compromise, and malware that can read or change the
trusted processes' memory or installed files under the current user's account.
Do not claim isolation from that malware. This exclusion does not make every
same-user process trusted.

The native host must authenticate the waiting CLI. The CLI must authenticate the
host and the authorized browser launch before allowing a cookie read. Browser
origin checks remain required but are not enough for this second boundary.
Authenticate the connection before sending a transaction authorization or cookie.

| Attack | Required defense or current limit |
|--------|-----------------------------------|
| Local process supplies an allowed origin argument | Independently verify the browser launch and host peer; argument comparison alone fails |
| Another process creates the expected IPC endpoint first | Detect endpoint impersonation; fail closed; never send a cookie to an unverified endpoint |
| Another OS user connects | Explicit local logon/user access controls and remote-client rejection |
| Unapproved same-user process connects | Application-peer authentication, not only an OS-user check |
| A pending-login identifier is copied | Identifier is correlation only; authorization must be bound to the authenticated connection |
| A valid submit is copied to another connection | Connection binding, one-use state, and replay rejection |
| Two CLI logins race | One pending login per OS logon; fail the second with `busy`; do not select a transaction by enumeration |
| Trusted memory or installed files are modified | Outside this threat boundary; do not claim protection |

### IPC decision and blocker

A Windows named pipe is a candidate transport, not a selected authentication
scheme. [Tokio's pipe API](https://docs.rs/tokio/1.54.1/tokio/net/windows/named_pipe/struct.ServerOptions.html)
provides transport options such as first-instance creation and remote-client
rejection. A first instance helps detect an existing endpoint; it does not prove
the identity of that endpoint to a client. A logon-restricted access list does
not reject all unauthorized same-user applications.

**Blocker IPC-1:** no reviewed mechanism currently proves both application peers,
the browser launch, and the secure delivery of one-use authorization under this
threat boundary. The session-only prototype's challenge does not supply that
proof. Do not silently substitute default pipe permissions, a PID supplied in
JSON, parent-process names, a caller origin, or a token in a discovery file.

Before selecting IPC, document and test all of the following:

1. How each peer obtains OS-verified peer evidence and checks trusted executable
   identity without relying on caller-supplied values or a race-prone name check.
2. How the host proves the authorized browser launch, not merely that the host
   binary is installed or running under the same user.
3. How discovery rejects a false CLI endpoint and avoids secret-bearing files,
   process arguments, environment input, URLs, ordinary output, or logs.
4. How one-use authorization is delivered confidentially to the intended peer,
   bound to the connection, and invalidated after submit, disconnect, or expiry.
5. How explicit access controls, remote rejection, endpoint ownership, concurrent
   login exclusion, and crash recovery work through reviewed safe Rust APIs.

No dependency is selected. Tokio transport alone does not meet these requirements.
Repository `unsafe` code and a new required runtime remain forbidden. A safe
dependency for OS checks or authenticated transport needs review before use.
If a pairing action or a different trust boundary is necessary, obtain a user
decision before changing this proposal. No such action is approved here.

The fixture condition `authenticated_peer` is an injected test precondition.
It is **not a wire field**, an implemented check, or permission to enable a host.
Cases with this condition can test state logic later, but cannot prove IPC-1.

## Shared protocol rules

These are proposed local limits, not upstream limits or accepted UX defaults.
Maintainer acceptance is required before freezing them.

| Rule | Version 1 proposal |
|------|--------------------|
| Encoding | Strict UTF-8 JSON object; no BOM, duplicate keys, trailing data, or non-finite numbers |
| Fields | Exact fields for the message type; reject unknown or missing fields and wrong types |
| Version | Integer `1`; no negotiation, downgrade, or best-effort parsing |
| Frame payload | 1 through 16,384 UTF-8 bytes, excluding the length prefix |
| JSON nesting | At most 8 object/array levels, counting the root as level 1 |
| Session value | 1 through 8,192 ASCII cookie-octet bytes; the complete message must also fit its frame |
| Transaction | 32 lowercase hexadecimal characters from a secure 128-bit random source |
| Pending login | At most 300,000 ms from CLI creation; includes all phases |
| Peer/hello wait | At most 10,000 ms; never extends the pending deadline |
| Cookie-query wait | At most 15,000 ms after `pending`; never extends the pending deadline |
| Validation budget | At most 15,000 ms, including response reads; connect at most 5,000 ms |
| Save-result wait | At most 5,000 ms; an OS write can outlive this wait |
| Connections | One host claim per pending login; one active pending login per OS logon |
| Message count | At most 8 frames in each direction per connection, including errors and cancellation |
| Retries | No automatic reconnect, credential resubmit, exchange retry, or save retry |

A cookie octet is one of `0x21`, `0x23..0x2b`, `0x2d..0x3a`, `0x3c..0x5b`,
or `0x5d..0x7e`. Quoted cookie syntax, whitespace, separators, and header injection
are not accepted. These syntax checks do not prove upstream validity.

Use monotonic deadlines. Each phase gets the smaller of its budget and the
remaining pending time. `expires_in_ms` is an integer from 1 through 300,000.
Receivers start a local timer when they receive it. They cannot extend the CLI's
authoritative deadline. An event at or after the deadline is late.
An observed cancellation wins over a simultaneous pre-save completion.

Reject an invalid frame length before allocating its payload. Bound reads even
when the sender stops halfway through a prefix or payload. On framing failure,
close the connection; do not try to resynchronize. Do not reflect malformed bytes
in an error. When a safe reply is still possible, use a fixed error code only.

Transactions are public correlation identifiers, not credentials or capabilities.
An exact identifier match is necessary after binding, but never sufficient.
Only one submit can enter validation. Consume its authorization atomically before
validation starts. An unauthorized connection must not cancel or consume another
connection's pending login.

## Browser-to-host contract

Protocol name: `bg-browser-handoff`. Use `runtime.connectNative` for one port per
user action. The transport prefix is a native-endian unsigned 32-bit byte count,
as required by Chrome and Edge. The 16 KiB local cap applies in both directions.
Only framed messages go to host stdout. Diagnostics go to stderr and use fixed,
redacted messages. Never log JSON payloads.

The following tables list **all fields**, including the common fields.
String literals in the tables are exact. `browser` is `chrome` or `edge`.
`profile` is an accepted, compiled credential profile identifier, not a URL.
`value` contains the single agreed cookie value, not a cookie jar.

| Type and direction | Required fields |
|--------------------|-----------------|
| `hello`, extension to host | `protocol`, `version`, `type`, `browser`, `profile`, `consent` |
| `pending`, host to extension | `protocol`, `version`, `type`, `transaction`, `profile`, `expires_in_ms` |
| `submit`, extension to host | `protocol`, `version`, `type`, `transaction`, `profile`, `value` |
| `cancel`, extension to host | `protocol`, `version`, `type`, `transaction` |
| `result`, host to extension | `protocol`, `version`, `type`, `transaction`, `outcome`, `validation`, `expiry` |
| `error`, either direction | `protocol`, `version`, `type`, `transaction`, `code` |

`protocol` is always `bg-browser-handoff`; `version` is always integer `1`.
`consent` must be Boolean `true`, after an explicit user action and consent in the
extension. `pending` and `submit` must have the same accepted `profile`.
For `cancel` or `error`, `transaction` is `null` before binding and the exact
identifier after binding. All other transaction fields are non-null strings.
`result` permits only `outcome: "saved"`, `validation: "service-validated"`, and
`expiry: "unknown"` in this proposal. A future expiry representation needs an
explicit contract revision; it must not be inferred from token shape.

Example using invented data:

```json
{
  "protocol": "bg-browser-handoff",
  "version": 1,
  "type": "submit",
  "transaction": "11111111111111111111111111111111",
  "profile": "synthetic-v1",
  "value": "synthetic-session"
}
```

### Browser state sequence

1. `idle`: do not connect or query cookies without a user action and consent.
2. `connecting`: send `hello`. The host checks configured browser/origin identity
   and obtains an authenticated pending CLI authorization through IPC.
3. `authorized`: receive `pending` only after that authorization. Check its
   version, profile, deadline, and transaction before the one cookie query.
4. `awaiting-result`: send one `submit` after exact profile validation. Drop
   extension-owned references to the value. Do not retry on disconnect.
5. `saved`, `failed`, or `indeterminate`: accept only the matching final result
   or safe error. A lost result after submit is `indeterminate`.
   Close the port, clear timers and references, and require a new user action.

Missing consent never connects. Missing pending authorization never reads a
cookie. A permission failure, missing or ambiguous cookie, wrong scope, malformed
value, cancellation, or timeout never submits a value.
Before submit, extension errors may report only the safe codes below.
After submit, only `cancel` may be sent while waiting.

Accept `result` only in `awaiting-result`, with the bound transaction and exact
result fields. A result before submit is a protocol error and cannot report
success. A malformed, mismatched, or missing reply after submit cannot prove
whether a save occurred. Enter `indeterminate`, report `save_indeterminate`,
and close the port without retrying. A valid matching pre-save error can
establish failure with unchanged storage; a lost connection cannot.

Receipt is not a success state. Do not send an intermediate successful receipt,
display a login-success message on submit, or reinterpret a lost reply as success.
Closing the popup or port cancels its transaction; late callbacks must not read,
submit, or update UI. JavaScript and browser-owned copies cannot be guaranteed to
clear from memory. Do not claim that dropping a reference erases those copies.

## Host-to-CLI contract

Protocol name: `bg-host-cli`. The proposed payload encoding is the same strict
JSON subset. Its separate framing is a **little-endian** unsigned 32-bit byte
count. It is not browser native-endian framing.
Framing does not supply encryption or authentication. Apply the finally selected
authenticated local transport before any of these application messages.

| Type and direction | Required fields |
|--------------------|-----------------|
| `open`, host to CLI | `protocol`, `version`, `type`, `browser`, `profile`, `consent` |
| `authorized`, CLI to host | `protocol`, `version`, `type`, `transaction`, `profile`, `expires_in_ms` |
| `submit`, host to CLI | `protocol`, `version`, `type`, `transaction`, `profile`, `value` |
| `cancel`, host to CLI | `protocol`, `version`, `type`, `transaction` |
| `result`, CLI to host | `protocol`, `version`, `type`, `transaction`, `outcome`, `validation`, `expiry` |
| `error`, either direction | `protocol`, `version`, `type`, `transaction`, `code` |

`protocol` is always `bg-host-cli`. Other field types and result literals match
the browser contract. Translate only these known messages between protocols.
Do not forward unknown objects or expose arbitrary host commands through the CLI.

### CLI state sequence

| State and event | Next state and permitted work |
|-----------------|-------------------------------|
| `waiting` plus authenticated, consented `open` | `authorized`; claim the one pending transaction; send `authorized` |
| `waiting` plus unauthenticated caller | Reject that connection; retain `waiting`; no authorization or cookie access |
| `authorized` plus first valid, bound `submit` | `validating`; consume authorization before one core establishment call |
| `validating` plus supported service validation | `saving`; attempt one protected save if not cancelled or late |
| `saving` plus confirmed save before deadline/cancellation | `saved`; return the final `result` |
| Any pre-save state plus cancellation/timeout/disconnect | `failed`; invalidate authorization; no new query, validation, or save |
| `saving` plus cancellation/timeout/disconnect or uncertain write | `indeterminate`; never claim success or unchanged storage |
| Terminal state plus another submit | Reject as replay; no new side effects |

Only a matching authenticated connection can advance its claimed transaction.
Reject a duplicate while validating without starting another operation.
An unexpected message terminates that claimed transaction but must not stop
another login or reveal its state.
Malformed messages before a claim reject only that connection and retain the
pending transaction's `waiting` state. The same rule applies to an unauthorized
attempt against an already claimed transaction.

On normal termination, close owned handles and ports, clear timers and
core-owned secret buffers, and release only this transaction's endpoint/lock.
Crash recovery must not reuse authorization or erase another process's endpoint.
No authorization survives a restart. The discovery and lock mechanism is part of
IPC-1 and remains unselected.

### Save and cancellation boundary

The protected-store call is the commit boundary. Before it starts, cancellation,
rejection, or timeout leaves any existing credential unchanged. No proactive
delete or replacement happens during receipt or validation.

Native storage is blocking and does not provide a general cancellation or
transactional rollback contract. Once a write starts, a failed call or lost
completion can leave storage uncertain. A 5-second wait is not a promise that
the OS call stops after 5 seconds.

Keep the worker's ownership explicit until the OS call finishes or the process
ends. Keep the pending-login slot reserved while that worker runs, even after
the transport closes. Do not start a second write, send a late success,
automatically delete an entry, restore an old secret, or retry after uncertainty. Return
`save_indeterminate` when a final reply is still possible. If the connection is
gone, the CLI must report the same safe diagnostic locally.
Release worker-owned secrets when the worker actually finishes. Forced process
termination can prevent application cleanup; the OS closes process handles but
does not undo a completed credential write.

A confirmed save followed by a lost reply is also indeterminate to the receiver.
Local status can establish presence only, not whether that login was validated.
Recovery requires a new explicit user action. Do not replay the old cookie.

## Credential profiles and host registration

`synthetic-v1` is the **only defined profile**, for test data only. It is not
enabled in a product binary. Its invented attributes are:

| Attribute | Synthetic value |
|-----------|-----------------|
| Query URL | `https://login.beyond-gate.invalid/` |
| Name | `BeyondGateSyntheticSession` |
| Domain | `login.beyond-gate.invalid`, host-only |
| Path | `/` |
| Secure / HttpOnly | Both `true` |
| SameSite | `lax` |
| Partition | None |
| Store | Current regular profile's default cookie store; no profile enumeration |
| Value | Cookie-octet syntax, `synthetic-` prefix, 10 through 8,192 bytes total |
| Expiry | Session cookie, or a finite expiry strictly after the test clock |

The extension must reject zero or multiple matching candidates and all
wrong-scope, expired, partitioned, or malformed candidates. Do not use a query
that silently picks one of several applicable cookies. The exact bounded
selection algorithm belongs to #46 and needs real browser tests.
The wire contains only the value. The host cannot independently prove cookie
metadata from that value; it relies on the authenticated, trusted extension path.

**Blocker PROFILE-1:** the real `CobaltSession` cookie scope, attributes, and minimum
usable material are not verified. Do not copy the synthetic attributes into a
production profile. **Blocker CORE-1:** the required upstream validation operation,
evidence, persistent format, and #30 agreement are not established. The
[manual probe](authentication.md#community-evidence-not-an-official-api-contract)
is evidence of one candidate exchange, not a service-validation contract.

The proposed native host name is `io.github.plagueho.beyond_gate`.
The proposed sibling binary is `beyond-gate-native-host` (`.exe` on Windows).
These names reserve no installation or publication action.
Use browser-specific manifests with only the exact approved extension origins.
Actual manual-package identities must be established by #46; the `a` and `b`
identities in fixtures are invented.

On Windows, plan explicit per-user registration beneath
`HKCU\Software\Google\Chrome\NativeMessagingHosts` for Chrome and
`HKCU\Software\Microsoft\Edge\NativeMessagingHosts` for Edge. Do not rely on Edge's
fallback lookup. Check conflicting existing registrations and both registry
views. Preview changes and obtain confirmation before writing. Removal must
match this installation's exact path and identity; never delete another entry.
Validate the manifest, binary path, extension identity, and protocol version
before enabling the bridge. Do not use a shell to launch the native host.

| Browser/OS | Evidence | Required before a support claim |
|------------|----------|---------------------------------|
| Edge / Windows | User-reported synthetic prototype success only | Real #46 PR CI, failure cases, actual identities, registration and cleanup tests |
| Chrome / Windows | No browser pass recorded | Separate real #46 PR CI and registration/cleanup evidence |
| Chrome or Edge / macOS | Untested | OS-specific secure IPC, install paths, browser and cleanup evidence |
| Chrome or Edge / Linux | Untested | OS-specific secure IPC, install paths, browser and cleanup evidence |

No Store publication, automatic update service, or new product runtime is needed.
Manual installation guidance and packages belong to #46 after the contract
checkpoint. Browser-specific CI must identify the actual browser flavor and
versions; Chromium is not an unlabelled Chrome or Edge pass.

## Proposed shared core boundary

The following is an API proposal for `beyond-gate-core::auth`, not compilable
repository API or an implemented lifecycle:

```rust
async fn establish_session(
    input: SessionEstablishmentInput,
    store: &SessionStore,
    control: LoginControl,
) -> Result<SavedSession, EstablishmentError>;
```

| Proposed type | Responsibility |
|---------------|----------------|
| `SessionEstablishmentInput` | Own one `BrowserSessionCookie` and a compiled `VerifiedSessionProfile`; no arbitrary URL or runtime cookie name |
| `BrowserSessionCookie` | Own validated bounded bytes, zeroize owned allocations, and redact all formatters; not the existing opaque `SessionSecret` |
| `VerifiedSessionProfile` | Fix exchange, validation and persistent-material rules from accepted evidence; no production variant until CORE-1/PROFILE-1 are resolved |
| `LoginControl` | Own the monotonic deadline and cancellation state; adapters cannot extend it |
| `SavedSession` | Contain only `ValidationEvidence::ServiceValidated` and `ExpiryEvidence::Unknown` in this proposal; never a cookie or token |
| `EstablishmentError` | Expose a safe typed category and redacted typed cause; no raw credential, URL input, response body, or native error text |

The core owns exchange, validation, and one protected save. The host and extension
do not exchange cookies for bearer tokens. Adapters do not implement session
logic. Keep a derived bearer token in protected process memory only; do not
persist it merely because it was returned. The accepted profile must select the
minimum persistent material and its encoded size, within the existing 8 KiB
storage bound. No persistent format is selected by this document.

An exchange response containing a nonempty `token` is not
`ValidationEvidence::ServiceValidated`. #30 must specify the bounded,
non-content validation operation and what it proves. Unknown expiry is not a
failure or a guessed expiry date. A verified expired state needs upstream
evidence; do not infer it from every 401/403 response.

Before save, check cancellation and the deadline again. Do not change the
existing credential on a pre-save failure. Keep native blocking storage calls
off executor threads. Use isolated synthetic stores and services in tests,
not the real `SessionStore::open` backend. Reuse the existing keyring test seam.

CLI login success requires `SavedSession`; a handoff receipt or
`StoredUnverified` cannot substitute for it. MCP must later consume #30's shared
session lifecycle using the same fixed entry, not accept a cookie in a tool.
This proposal adds neither a second credential protocol for MCP nor new tools.
Refresh, authenticated resource reads, and upstream revocation remain #30 work.

### Safe errors

Every wire `code` comes from this table. Receivers map codes to fixed local
messages, not messages supplied by the peer. Unknown codes are protocol errors.
CLI operation failures remain exit 1 on stderr with no success on stdout.
Protocol errors never include reflected fields. Existing invalid CLI usage
remains exit 2. A closed or corrupt connection need not receive a reply.

| Code | Meaning / core correspondence | Storage effect |
|------|-------------------------------|----------------|
| `consent_required` | No explicit extension consent | Unchanged |
| `unauthorized_peer` | Required application-peer proof failed | Unchanged |
| `origin_rejected` | Browser or extension identity is not configured | Unchanged |
| `no_pending_login` | No pending CLI transaction | Unchanged |
| `busy` | Another transaction/connection already owns the slot | Unchanged |
| `version_mismatch` | Unsupported protocol version | Unchanged |
| `unsupported_profile` | Profile not compiled and accepted | Unchanged |
| `protocol_error` | Invalid fields, encoding, framing, sequence, or size | Unchanged before save; indeterminate if save started |
| `transaction_mismatch` | Message does not match the bound transaction | Unchanged before save; indeterminate if save started |
| `replay` | Duplicate submission or consumed authorization | No additional write |
| `cookie_unavailable` | Missing, ambiguous, wrong-scope, expired, or denied cookie query | Unchanged |
| `invalid_material` | `InvalidMaterial`; invalid value or persistent encoding | Unchanged |
| `cancelled` | `Cancelled`; cancelled before save | Unchanged |
| `timed_out` | `TimedOut`; deadline reached before save | Unchanged |
| `disconnected` | Peer closed before save or no final result was received | Unchanged before save; otherwise unknown |
| `session_rejected` | `SessionRejected`; upstream rejects material without verified expiry | Unchanged |
| `session_expired` | `SessionExpired`; accepted evidence verifies expiry | Unchanged |
| `validation_unavailable` | `ValidationUnavailable`; bounded transport/service failure | Unchanged |
| `unsupported_contract` | `UnsupportedContract`; no accepted validation/persistence contract | Unchanged |
| `storage_unavailable` | `StorageUnavailable` | Unchanged if no write started |
| `storage_access` | `StorageAccess`; denied/locked backend | Unchanged if no write started |
| `storage_failure` | `StorageFailure`; backend/setup failure before write | Unchanged |
| `save_indeterminate` | `SaveIndeterminate`; failed/late write or lost final acknowledgement | Unknown; never auto-delete or retry |

After a save starts, use `save_indeterminate` for an unconfirmed outcome even if
the underlying failure would otherwise map to cancellation, timeout, or storage
failure. After confirmed success, a duplicate remains `replay`, not a new save.
Redact `Display`, `Debug`, and standard error chains consistently with the
existing `AuthError` wrapper. Do not expose a raw transport or native source
through a supposedly safe cause.

## Fixture interpretation

`fixture_version: 1` versions the test-data format separately from both protocols.
`messages` contains complete canonical messages. Each case names a boundary,
starting state, input, conditions, and expected state/code/effects.
`code: "ok"` is a fixture expectation, not a wire error code.
`open`/`hello` use the exact message definitions above.

An input with `message` copies that named message. Optional `set` replaces
top-level fields, `add` adds fields, and `remove` removes fields.
For `sequence`, apply the message names or event names in order.
The synthetic `clock_unix_seconds` is the cookie-expiry clock.
`cookie_query_completed` starts with `cookie_fixture` and one matching candidate;
`cookie_set` replaces named fields, and `matching_candidates` replaces the count.
Its query started before the case, so completion does not count as another read.
`origin` conditions override the configured origin selected by `browser`.
`frames_seen` is the number of frames already read in that direction.
`value_bytes` replaces `value` with the ASCII string `synthetic-` followed by
enough `s` bytes to reach the given total length. Zero means the empty value.
For lengths 1 through 9, use that many `s` bytes. Such values fail the synthetic
profile's required prefix, even though they fit the general wire size bound.
Frame recipes construct invalid or boundary inputs without allocating large
checked-in files: `declared_length`, `payload_hex`, and `prefix_hex` are literal;
`pad_json_to_bytes` adds trailing JSON spaces to the named message.
`json_depth` constructs nested arrays around an empty object, with the given
total level count. Reject excess depth before message-shape validation.
`event` is an injected test event, never a message a caller can use as proof.
`elapsed_ms` is measured on the synthetic monotonic clock.

`authenticated_peer`, `trusted_browser_launch`, `pending_login`, and
`profile_accepted` are synthetic harness conditions, not caller assertions.
The latter means accepted **by the test harness**, not product acceptance.
The production default for an unproved condition is failure.
Cases inherit `default_conditions`; per-case conditions override those values.
Positive cases require all four conditions unless stated otherwise.

Effect Booleans identify work allowed **during that case**, not cumulative
effects before its starting state. `cookie_read` authorizes one bounded query,
not disclosure to the host before cookie validation. `save` means one attempt,
not proof that a native write can be undone. `validate` means one bounded
synthetic core operation. Storage uncertainty is recorded separately.

Future Rust and browser suites must consume these cases and add actual framing,
timing, OS peer, browser permission, cookie-selection, process, and cleanup tests.
Do not satisfy the issue's process/browser acceptance criteria with JSON parsing
or mocked `authenticated_peer: true`.

## Acceptance and remaining gates

| Decision | Status |
|----------|--------|
| Contract-first scope and stated threat boundary | User selected |
| Both version 1 message proposals and limits | Proposed; not frozen |
| Application-peer authentication and discovery | Blocked: IPC-1 |
| Actual cookie profile | Blocked: PROFILE-1 |
| Shared core validation, persistence, and #30 agreement | Blocked: CORE-1 |
| Rust/browser fixture consumers and process tests | Not implemented |
| Chrome and Edge synthetic PR CI | Required under #46; not established here |
| Live-account or native-store experiment | Not authorized by this milestone |

Maintainers must accept both contracts and the shared core boundary before
product host/extension integration. Record the accepted revision and fixture
version in #29 and coordinate #30/#46. Do not freeze or enable the blocked
portions merely because documentation and JSON checks pass.

Keep #29 open. Separate approval is required for account or credential-store
tests, with exact hosts, operations, request budgets, stop conditions, and
cleanup. The environment-based fallback remains an unimplemented separate
ingress design. No raw-cookie CLI arguments, MCP credential parameters,
plaintext fallback, Store publication, or access-control workaround is added.
