# Browser login contracts

**Status: Windows product login verified with Chrome and Edge Dev.**
This document defines the version 1 contracts for
[#29](https://github.com/PlagueHO/beyond-gate/issues/29).
The user approved the implemented shared core boundary for
[#30](https://github.com/PlagueHO/beyond-gate/issues/30).
The agreement is recorded on
[#29](https://github.com/PlagueHO/beyond-gate/issues/29#issuecomment-6554690324)
and [#30](https://github.com/PlagueHO/beyond-gate/issues/30#issuecomment-6554690285).
The user accepted the [Windows pairing boundary](#accepted-windows-pairing-boundary)
and the [direct-service rule](#accepted-windows-service-rule). Shared Windows
validation and saving have offline tests. The
[Edge API observation](#approved-one-use-edge-api-observation) supplies the
profile fields. Windows parsers and the paired controller now bind the selected
`cobalt-session-v1` profile. The
[Windows runtime integration](#windows-runtime-integration) now connects the
native relay and CLI to actual shared completion. Browser selection, explicit
Windows setup, and real-browser synthetic completion checks are implemented.
Production terminal, direct validation, protected save, restart/status, and logout
passed in the [approved local scope](testing.md#approved-windows-product-login-verification).
Hosted CI also passed. Message examples alone are
not an authenticated channel.

The [canonical synthetic cases](fixtures/browser-handoff-v1.json) now have
[Rust consistency checks](../src/beyond-gate-core/tests/browser_handoff_fixture.rs)
and [executable protocol tests](../src/beyond-gate-core/tests/browser_handoff_protocol.rs).
The shared core parses the proposed message shapes and bounded frames with the
invented profile and, on Windows, the bounded production profile. Selected
message and frame recipes execute this code.
Encrypted peer and process checks now run; Chrome and Edge synthetic native
messaging passed locally. Real Windows login now has separate approved evidence.
A fixture pass is
not proof of account authentication.
Redirected `beyond-gate auth login` still fails before runtime, storage, or
network activity. Interactive Windows login now starts the pending pairing flow;
macOS and Linux product browser login remain deferred.

## Windows runtime integration

The actual `beyond-gate-native-host` executable accepts only
`chrome-extension://gnhdconfgjfdbbgkemadegjcjnnodefh/`, plus an optional decimal
`--parent-window` argument. The argument is syntax, not browser-launch proof.
The host name is `io.github.plagueho.beyond_gate`. The retained public manifest
key pins a manual package identity. Both browsers verified that ID in isolated
synthetic checks. Product installation remains a user action.
Origin rejection occurs before runtime or stdin reading.

Both JSON protocols retain version 1. The production profile adds an input-only
browser `pair` preface and a bound final `ack`; the synthetic fixture sequence
does not change. The preface requires exact fields, explicit consent, selected
browser, `cobalt-session-v1`, and a canonical 43-character key. Keys cannot be
serialized through the shared message encoder.

```json
{
  "protocol": "bg-browser-handoff",
  "version": 1,
  "type": "pair",
  "browser": "edge",
  "profile": "cobalt-session-v1",
  "consent": true,
  "key": "KioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKio"
}
```

The key above is invented test data, not a generated product key. After Noise
pairing, the host sends a consented production `open`. The CLI supplies bound
authorization, and the host forwards browser `pending`. Only then can a browser
submit the selected cookie. The host does not query cookies, contact the service,
or open credential storage.

The CLI monitors peer control while the real core validates and saves. A pending
encrypted read remains alive across result delivery; partial reads are never
cancelled and resumed. Before-write cancellation preserves storage. A started
write retains the reservation and reports uncertainty. Browser cancellation
after submit is forwarded to the CLI. The relay waits for the CLI's outcome;
missing confirmation cannot prove unchanged storage.

Validation has a 15-second bound. Store setup and save waiting share a fixed
five-second bound within the original pending deadline. The host waits at most
20 seconds for this outcome. After submit, a failed delivery, stalled outcome,
or lost acknowledgement reports `save_indeterminate`, not ordinary timeout.
The blocking store worker checks the same five-second deadline before setup
and write admission. This does not depend on the async timeout being polled.

Only a genuine shared `SavedSession` permits `result`. The browser must then
send the following bound acknowledgement. The host forwards it encrypted to the
CLI, waits for CLI confirmation, and forwards confirmation to the browser.
The acknowledgement phase shares one five-second deadline.

```json
{
  "protocol": "bg-browser-handoff",
  "version": 1,
  "type": "ack",
  "transaction": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

The host-to-CLI acknowledgement has the same fields with protocol `bg-host-cli`.
An early, wrong, or missing acknowledgement cannot produce confirmed success.
The native result projects `service-validated`; the CLI/core result uses
`service_validated`. Both retain unknown cookie expiry. Stored presence remains
`stored-unverified`, not a claim of future usability.

These runtime paths have offline relay, actual native-process refusal, and
paired local-service/in-memory-store checks. Both real browsers passed synthetic
success, service rejection, cancellation, lost acknowledgement, and unavailable
storage. Separate approved Chrome and Edge Dev product login/save checks passed.
The manual browser form now uses
[shared production recipes](fixtures/windows-login-v1.json) in browser and Rust
tests. Its consent, single cookie query, cancellation, and acknowledgement
controller is implemented. Real Chrome and Edge process checks now exercise
the product controller and native/core completion with invented material.
They use private fixture pipes, a local mock service, and in-memory storage.
The production interactive CLI, live validation, native save, restart/status,
logout, and owned cleanup passed in the separate approved local scope.

The user accepted
[native registration as trusted installed configuration](authentication.md#browser-login-window).
The browser passes its pairing key to the selected native host. The flow does
not protect against a change that substitutes another host through registry
entries, manifests, or launch mapping. Windows setup commands now check conflicts
and require confirmation. Registry ownership checks and selected per-user
mapping removal share one transaction. See the
[process evidence and remaining limits](testing.md#windows-product-flow-process-checks).

## Evidence and ownership

The [approved Windows product checks](testing.md#approved-windows-product-login-verification)
record real Chrome and Edge Dev login, protected save, and cleanup.
The [earlier feasibility evidence](authentication.md#extension-feasibility-evaluation)
remains separate from that product result.

| Work | Owner | Current boundary |
|------|-------|------------------|
| Native host, browser registration, pending CLI login, protected handoff | [#29](https://github.com/PlagueHO/beyond-gate/issues/29) | Windows implementation and product checks complete; review and merge remain separate |
| Session exchange, validation, persistent format, expiry and refresh | [#30](https://github.com/PlagueHO/beyond-gate/issues/30) | Establishment boundary agreed and implemented; reusable session lifecycle remains separate |
| Chrome and Edge extension, manual package, real synthetic browser CI | [#46](https://github.com/PlagueHO/beyond-gate/issues/46) | Windows product integration and hosted synthetic checks passed using the shared contracts |

Issue #30 does not require completion of all of #29 before lifecycle development.
macOS and Linux browser login remain unsupported; their existing local status
and logout behavior stays unchanged.

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
host's possession of the one-use key before allowing a cookie read. The latest
Windows decision below replaces the earlier independent browser-launch proof.
Browser origin checks remain required but are not application authorization.
Authenticate the connection before sending a transaction authorization or cookie.

| Attack | Required defense or current limit |
|--------|-----------------------------------|
| Local process supplies an allowed origin argument | Reject absent or incorrect one-use keys; argument comparison alone fails. Independent extension-launch proof is not claimed under the accepted Windows boundary. |
| Another process creates the expected IPC endpoint first | Detect endpoint impersonation; fail closed; never send a cookie to an unverified endpoint |
| Another OS user connects | Explicit local logon/user access controls and remote-client rejection |
| Unapproved same-user process connects | Application-peer authentication, not only an OS-user check |
| A pending-login identifier is copied | Identifier is correlation only; authorization must be bound to the authenticated connection |
| A valid submit is copied to another connection | Connection binding, one-use state, and replay rejection |
| Two CLI logins race | One pending login per OS logon; fail the second with `busy`; do not select a transaction by enumeration |
| Trusted memory or installed files are modified | Outside this threat boundary; do not claim protection |

### IPC decision and blocker

This subsection records the original stronger launch-proof requirement.
The [accepted Windows pairing boundary](#accepted-windows-pairing-boundary)
supersedes that requirement, not the access, one-use, deadline, or cleanup limits.

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
The user has since approved one-use encrypted pairing. The
[approved follow-up direction](#approved-pairing-and-evidence-direction) separates
that decision from the remaining mechanism and execution checks.

### Windows candidate review

**Reviewed: 2026-10-31. Result: IPC-1 remains unresolved.**
The full staged implementation plan starts with the existing contract gates.
Approval of that plan does not establish a working authentication mechanism,
freeze the proposed contracts, or authorize account or credential-store tests.

The review checked the Windows and safe Rust interfaces below. It did not
install a dependency, launch a browser, register a host, or access credentials.

| Candidate | Verified capability | Missing proof |
|-----------|---------------------|---------------|
| Named-pipe access controls | Explicit permissions and logon isolation can restrict callers. | A permitted OS user is not an approved application or browser launch. |
| Named-pipe peer process IDs | Windows exposes both client and server process ID queries. | A process ID alone does not prove application identity, extension consent, or a pending login. |
| Browser parent-process check | Windows allows a creator to specify another process as the new process's parent, subject to access rights. | Parent identity alone cannot prove that the browser created the host for the approved extension. |
| Tokio 1.54.1 named pipes | Provides first-instance creation and remote-client rejection. | These transport controls do not authenticate both applications or the browser launch. |
| Interprocess 2.4.7 named pipes | Provides safe client/server peer process and session ID queries. | Safe access to peer metadata is not a complete authentication mechanism. |

[GetNamedPipeClientProcessId](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeclientprocessid)
and
[GetNamedPipeServerProcessId](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeserverprocessid)
provide OS-reported process identifiers rather than JSON assertions. A design
using them must still establish trusted executable identity, process lifetime,
connection binding, and authorized browser launch. Checking the installed host
binary alone would also accept a direct launch of that same binary.

[UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)
documents `PROC_THREAD_ATTRIBUTE_PARENT_PROCESS`. It lets a creator select
another parent using a handle with `PROCESS_CREATE_PROCESS` access. Do not
assume those rights are available for every browser process. The documented
option is enough to reject an unconditional claim that a parent PID proves
the creator or extension. No parent-spoofing experiment was run.

[Tokio's options](https://docs.rs/tokio/1.54.1/tokio/net/windows/named_pipe/struct.ServerOptions.html)
remain useful transport controls.
[Interprocess's Windows API](https://docs.rs/interprocess/2.4.7/x86_64-pc-windows-msvc/interprocess/os/windows/named_pipe/struct.PipeStream.html)
shows that peer metadata need not require repository `unsafe` code.
Neither library is selected by this review. A library that transports bytes
and exposes process IDs does not itself resolve IPC-1.

[CredReadW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credreadw)
reads the credential set associated with the caller's logon token.
[Generic credentials](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw)
do not identify an authenticated application peer. Do not use the existing
service/account entry as an exclusive application-authentication key without
an additional reviewed protection boundary. This review used documentation
only; it did not open or read the native store.

PROFILE-1 and CORE-1 also remain unresolved. The browser cookie API describes
fields, not the actual `CobaltSession` profile. The core still has only opaque
local storage; it has no implemented `establish_session` operation or accepted
service-validation result. The developer probe is not a replacement.

Stop dependent product work at these gates. Do not enable a host, register it,
or authorize a cookie query using an origin, parent PID, peer PID, or challenge
alone. A reviewed launch-proof mechanism or an explicitly approved alternative
is still required. This review does not prove that no viable mechanism exists.
The version 1 message proposal and synthetic fixture remain unchanged.

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

`synthetic-v1` is the test-data profile. It is not enabled through a product
adapter. Its invented attributes are:

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

### Selected Windows cookie contract

`cobalt-session-v1` uses the separately approved
[Edge API observation](#approved-one-use-edge-api-observation) and the
[accepted Windows service rule](#accepted-windows-service-rule). This is a
compiled contract, not a product browser support claim.

| Attribute | Required production value |
|-----------|---------------------------|
| Query | `cookies.getAll({url: "https://www.dndbeyond.com/", name: "CobaltSession", path: "/"})` |
| Name / domain / path | `CobaltSession` / `.dndbeyond.com` / `/` |
| Host-only | `false` |
| Secure / HttpOnly | Both `true` |
| SameSite | `unspecified`; do not substitute `lax` |
| Partition | Absent; omit `partitionKey` from the query |
| Store | Current regular context; omit `storeId`; never enumerate profiles |
| Expiry | Persistent cookie with finite expiry strictly after the browser clock |
| Value | 1 through 2,552 ASCII cookie-octet bytes; no truncation or synthetic-prefix requirement |
| Host permissions | Only `https://dndbeyond.com/*` and `https://www.dndbeyond.com/*` |

Reject missing or multiple candidates and every wrong attribute before submit.
Do not query another profile, broaden host permissions, or select the first
result as a fallback. The root-host permission covers the domain cookie; it
does not permit arbitrary subdomains.
The [Chrome cookies API](https://developer.chrome.com/docs/extensions/reference/api/cookies)
documents the current-context store, unpartitioned default, and `unspecified`
SameSite value. Approved Chrome and Edge Dev product selection passed separately
from the earlier metadata observation.

Windows message parsing enforces the production value limit before receipt.
`PendingHandoff::for_profile` fixes the profile before opening and rejects
changes in either `open` or `submit`. Its existing `new` constructor remains
synthetic-only. `SessionEstablishmentInput::from_handoff` consumes a
production-bound receipt without copying the credential allocation; synthetic
receipts cannot enter this conversion. Receipt still produces no final success.
The actual extension must enforce metadata and consent; these are not wire
claims that the native host can independently verify.

**PROFILE-1 result:** the exact browser selection, including malformed
and ambiguous results, has offline tests. Only one real Edge profile was observed;
actual Chrome and Edge Dev selection and production-terminal login now passed.
The diagnostic was confirmed removed. No new cookie query is authorized.
**CORE-1 result:** the Windows operation is integrated with the relay and
completion controller. Real protected save passed, and the user agreed to this
shared boundary for #30. Later lifecycle/adapter use remains #30 work.

The native host name is `io.github.plagueho.beyond_gate`.
The sibling binary is `beyond-gate-native-host` (`.exe` on Windows).
These names reserve no installation or publication action.
Use browser-specific manifests with only the exact approved extension origins.
The production package pins `gnhdconfgjfdbbgkemadegjcjnnodefh`; both browsers
checked this identity. The `a` and `b` identities in older fixtures are invented.

Windows setup uses explicit per-user registration beneath
`HKCU\Software\Google\Chrome\NativeMessagingHosts` for Chrome and
`HKCU\Software\Microsoft\Edge\NativeMessagingHosts` for Edge. Do not rely on Edge's
fallback lookup. Check conflicting existing registrations and both registry
views. Preview changes and obtain confirmation before writing. Removal must
match this installation's exact path and identity; never delete another entry.
Validate the manifest, binary path, extension identity, and protocol version
before enabling the bridge. Do not use a shell to launch the native host.

| Browser/OS | Evidence | Required before a support claim |
|------------|----------|---------------------------------|
| Edge / Windows | Five real-browser synthetic completion cases and cleanup passed locally and in hosted CI; isolated setup tests passed | Stable Edge real-account behavior is not claimed from Edge Dev |
| Edge Dev / Windows | Approved real product login/save, restart/status, logout and cleanup passed on 158.0.4357.0 | Retest changed versions under a fresh exact approval |
| Chrome / Windows | Five synthetic completion cases passed locally and in CI; approved real login/save/restart/logout passed on 156.0.8078.74 | Retest changed versions under a fresh exact approval |
| Chrome or Edge / macOS | Untested | OS-specific secure IPC, install paths, browser and cleanup evidence |
| Chrome or Edge / Linux | Untested | OS-specific secure IPC, install paths, browser and cleanup evidence |

No Store publication, automatic update service, or new product runtime is needed.
Manual installation guidance and packages are implemented under #46.
Browser-specific CI must identify the actual browser flavor and
versions; Chromium is not an unlabelled Chrome or Edge pass.

## Shared core boundary

The Windows implementation now exports the following API through
`beyond-gate-core::auth::establishment`. It replaces the earlier borrowed-store
proposal. Windows CLI completion now calls it. Windows browser setup is
implemented; MCP cannot accept credentials or start this ingress.

```rust
async fn establish_session(
    input: SessionEstablishmentInput,
    control: LoginControl,
) -> Result<SavedSession, EstablishmentError>;
```

| Type | Responsibility |
|------|----------------|
| `SessionEstablishmentInput` | Own one `BrowserSessionCookie` for the fixed service rule; no arbitrary URL or runtime cookie name |
| `BrowserSessionCookie` | Own a nonempty RFC 6265 value of at most 2,552 bytes; zeroize owned allocations and redact Debug; browser attributes are a separate ingress check |
| `LoginControl` | Own a monotonic deadline of at most five minutes, cancellation state, and the pending-slot reservation; adapters cannot extend it |
| `LoginCancellation` | Cancel the owning operation without retaining the reservation after completion |
| `SavedSession` | Expose only `validation: "service_validated"` and `expiry: "unknown"` after confirmed save; adapters format the result without a cookie or token |
| `EstablishmentError` | Expose a safe typed category and redacted typed cause; no raw credential, URL input, response body, or native error text |

The core owns exchange, validation, and one protected save. The host and extension
do not exchange cookies for bearer tokens. Adapters do not implement session
logic. The derived bearer remains transient process memory and is dropped after
validation. It is not persisted. Browser profile selection is implemented in
the extension; the cookie constructor does not prove consent, scope, or provenance.

The user accepted the [strict direct-service rule](#accepted-windows-service-rule).
Token presence alone remains insufficient. The rule trusts the direct TLS
response, not an independently verified JWT signature. A future bearer expiry
does not establish reusable-cookie expiry or content access. HTTP 401/403 is
`session_rejected`, not guessed expiry.

Before save, check cancellation and the deadline again. Do not change the
existing credential on a pre-save failure. Keep native blocking storage calls
off executor threads. Use isolated synthetic stores and services in tests,
not the real `SessionStore::open` backend. The implementation reuses the keyring
test seam. Production store setup and saving run in one blocking worker, after
validation. Cancellation and write admission use one atomic state transition.
A worker retains the reservation even if its caller is dropped. A queued write
cannot start after cancellation wins admission. A started or late write is
`save_indeterminate` unless the caller receives confirmed completion.

Persistent encoding is exactly `BGSE`, version byte `1`, service-profile byte
`1`, a two-byte little-endian cookie length, and the unchanged cookie bytes.
The eight-byte header and value must fit the native 2,560-byte limit.
`BrowserSessionCookie::from_persisted` rejects unknown magic/version/profile,
invalid lengths, trailing data, and invalid cookie syntax. It does not migrate
old opaque entries, truncate values, split entries, or use files.
Local status remains `stored-unverified`; creation-time validation is not a
permanent access grant. Loading and refresh for live operations remain #30 work.

CLI login success requires `SavedSession`; a handoff receipt or
`StoredUnverified` cannot substitute for it. MCP must later consume #30's shared
session lifecycle using the same fixed entry, not accept a cookie in a tool.
This boundary adds neither a second credential protocol for MCP nor new tools.
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
| `session_expired` | `SessionExpired`; service-issued bearer expired, not proof of reusable-cookie expiry | Unchanged |
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

The Rust consistency checks read the canonical messages and every case. They
check unique case IDs, message references, proposed limits, and expected error
effects. The protocol tests execute all canonical messages, selected message
mutations, and malformed-frame recipes against the shared core. They also check
frame bounds, both prefix encodings, duplicate keys, nullable field presence,
deadlines, cancellation, frame counts, and redacted errors.

The parsers do not authenticate peers or establish a session. The frame reader
and writer use a caller-supplied phase deadline with an additional 15-second
per-frame cap. A failed or cancelled operation prevents reuse of that reader or
writer. The eventual transport must support bounded cancellation and cleanup;
these async tests do not establish that browser standard handles do so.

Future Rust and browser suites must execute the remaining state and operation
cases and add authenticated OS peer and product browser/CLI integration tests.
Do not satisfy the issue's process/browser acceptance criteria with JSON parsing
or mocked `authenticated_peer: true`.

## Isolated browser feasibility

The [development browser suite](testing.md#isolated-browser-feasibility) now
loads a synthetic extension in regular installed Chrome and Edge on Windows.
Both actual cookie APIs and actual browser-launched Rust hosts passed locally.
Tests reject missing host registration and another allowed origin, gate the
handoff query on consent and pending authorization, and check native receipt.
The started hosts exited and their owned registrations and profiles were cleaned
up. Separate Windows browser CI jobs are defined but have not run.

The host uses the shared framing/parser implementation and canonical fixture.
The 14 canonical messages and 63 cases remain synthetic. Its pending reply is
injected, not authenticated CLI authorization. The caller-origin comparison is
test metadata only. The final `unsupported_contract` reply confirms receipt
without reporting validation or protected save.

No product host, extension, host-to-CLI channel, account operation, or native-store
access is enabled. These passes resolve local automation feasibility, not IPC-1,
PROFILE-1, CORE-1, or hosted-runner acceptance.

## Approved pairing and evidence direction

The user approved all three follow-up decisions: one-use encrypted pairing,
production cookie-metadata verification, and separately scoped validation and
synthetic credential-store checks. Product implementation may continue.
This approval does not establish cookie attributes, service behavior, or storage
results. Exact live operations must still be defined before execution.

### Pairing implementation boundary

The [pairing module](../src/beyond-gate-core/src/auth/handoff/pairing.rs) now uses
pinned `snow` 0.10.0 with the fixed
`Noise_NNpsk0_25519_ChaChaPoly_SHA256` pattern and a versioned application
prologue. See the [Noise specification](https://noiseprotocol.org/noise.html#pre-shared-symmetric-keys)
and [`snow` API](https://docs.rs/snow/0.10.0/snow/struct.Builder.html).
The selected features enable Curve25519, ChaChaPoly, SHA-256, and OS randomness.
Neither caller nor peer can select a different protocol or weaker algorithm.
No repository unsafe code or new product runtime is added.

The primitive consumes a 32-byte pairing secret and permits only empty handshake
payloads. A completed handshake establishes possession of that secret and
derives encrypted directional transport keys. It does not attest an executable
or browser origin. The final controller must generate a full-entropy secret,
deliver it through explicit user action, enforce one pending login and one
claim, and bind the resulting channel to that login.
Short passwords and public transaction IDs must not be used as pairing secrets.

`PendingPairing` owns one ticket with a monotonic deadline of at most five
minutes. `claim` consumes the ticket, including on failure, and rejects a claim
at or after expiry. Dropping an unclaimed ticket clears its owned waiting secret.
A compile-fail doctest checks that Rust ownership rejects a second claim.
The application must still provide OS-wide slot exclusion, user transfer,
expiry-driven cleanup, and product handshake cancellation.

`PairingKey::generate` now fills exactly 32 zeroizing bytes using pinned
`getrandom` 0.4.3. Its [system-source API](https://docs.rs/getrandom/0.4.3/getrandom/fn.fill.html)
reports any failure, including partial output, without a fallback. The local
typed error hides dependency and OS details. Key diagnostics are redacted, and
`into_bytes` consumes the key. `Transaction::generate` separately generates
128-bit public correlation data; it is not a pairing credential.
The system randomness call can block and cannot be interrupted by an async
timer. Product code must not claim a hard time bound for that synchronous call.
Two unit checks passed for key size and safe key/error formatting. A compile-fail
doctest rejects a second transfer. These checks do not measure entropy or
implement a protected user transfer surface.

The user-mediated transfer is the proposed authorization anchor, rather than
an automatic claim that OS process metadata proves the browser launch.
The exact interactive transfer surface and OS isolation remain unresolved.
Do not silently remove those requirements or report IPC-1 as complete.
No pairing value may enter chat, process arguments, environment variables,
ordinary redirected output, discovery files, URLs, or logs.

The encrypted channel rejects tampering, replay, reordering, and another
handshake's keys. Every failed operation closes its cryptographic state.
Plaintext remains bounded at 16,384 bytes; encrypted messages add a 16-byte tag.
The existing JSON frame reader is not an encrypted-envelope reader. The
separate encrypted stream implementation below enforces that outer envelope,
handshake deadlines, frame counts, cancellation, and stream ownership. Product
adapter integration and OS-isolated endpoint selection remain required.

Thirteen offline tests use real Noise with invented pairing keys. They cover
bidirectional encryption, unfinished-handshake rejection, wrong keys, different
application context, early application payload, size limits, tampering, replay,
cross-channel isolation, safe typed error chains, and exact ticket deadlines.
These tests do not prove one-use user delivery, OS-wide pending-login exclusion,
native IPC, or product login.
Owned input and plaintext buffers are zeroizing. Dependency-internal copies
are not guaranteed to clear.

### Encrypted stream and separate-process checks

The [encrypted stream module](../src/beyond-gate-core/src/auth/handoff/transport.rs)
owns the stream and cryptographic state together. Its nine-byte outer header
has four fixed bytes `BGP1`, a one-byte kind, and a little-endian `u32` payload
length. Kind 1 carries 1 through 256 Noise handshake bytes. Kind 2 carries 17
through 16,400 encrypted bytes. The decrypted payload is one `bg-host-cli`
version 1 JSON message, without the old plaintext length prefix.
Other versions, kinds, and lengths are rejected. There is no plaintext fallback.

The host initiates pairing. The CLI consumes its pending ticket before accepting
the handshake. Each direction sends one empty-payload handshake message. One
non-resetting ten-second deadline includes handshake construction and I/O and
cannot exceed the original pending expiry. Each direction then permits eight
application messages. An application operation cannot extend pending expiry
or exceed its caller deadline or fifteen seconds.

A started operation takes ownership of active stream and key state. Failure
or future cancellation drops both and prevents reuse in either direction.
This closes streams whose `Drop` implementation closes their handles. The
generic API does not establish OS isolation or interrupt blocking standard I/O.
Synchronous cryptographic work cannot be interrupted by an async timer; deadline
checks reject late completion.

Sixteen [stream tests](../src/beyond-gate-core/tests/browser_handoff_transport.rs)
passed locally on Windows. They check real encryption, wrong keys, invalid
headers, exact byte and frame limits, tampering, replay, invalid JSON, expired
tickets, stalled headers and ciphertext, and cancellation during handshake,
read, and write.

The [two-process fixture](../src/beyond-gate-core/examples/paired-handoff-fixture.rs)
uses one loopback listener and compiled invented keys. Both processes have a
ten-second lifetime. Three
[process tests](../src/beyond-gate-core/tests/paired_process.rs) passed locally:
synthetic encrypted receipt and explicit `unauthorized_peer` rejection for the
wrong key, plus `transaction_mismatch` rejection for an authenticated submission
to another transaction. The core controller now generates authorization for
a system-generated public transaction instead of injecting the fixture reply.
Receipt reports `validated=false` and `saved=false`. This synthetic controller
is not a pending product login or an OS-wide slot.

The helper requires `browser-fixture` and an explicit example build before
process tests. A missing helper fails those tests. The Rust CI matrix includes
that build and stream, flow, and process targets on Linux, macOS, and Windows; hosted execution
remains unverified. No browser, account request, or credential-store access is
needed. These checks do not complete user key delivery, OS-wide exclusion,
native-host integration, production cookie selection, validation, or save.

### Bound application exchange

The [application controller](../src/beyond-gate-core/src/auth/handoff/flow.rs)
owns one `PendingHandoff` transaction and its original five-minute maximum
expiry. It receives an already paired connection. The caller must bind that
connection to the correct ticket and supply OS-wide ownership. This controller
does not decide how an unauthenticated attempt affects the waiting slot.

It permits one consented `open` within ten seconds before sending authorization.
It then permits one matching synthetic `submit` within the existing fifteen-second
operation cap and pending expiry. Wrong correlation, duplicate `open`, early
results, cancellation, malformed input, and late input close the connection.
Peer claims of service rejection or storage failure are not trusted as core
evidence. Only fixed, bound pre-save host errors are accepted.

Successful receipt returns `ReceivedSubmission`, not a validated or saved
session. The owned controller cannot receive again. No account validation,
protected write, or final success result occurs in this operation.
Eleven [flow tests](../src/beyond-gate-core/tests/browser_handoff_flow.rs) passed
locally, including real generated-key pairing, wrong transactions, consent,
sequencing, cancellation, exact pending limits, and stalled operations.
A compile-fail doctest rejects a second receive on the same controller.

### Selected terminal pairing display

The user selected an interactive terminal instead of a native desktop dialog.
The [CLI display](../src/beyond-gate-cli/src/pairing.rs) requires terminal stdin,
stdout, and stderr before it encodes or writes a key. It then opens the OS
terminal device, checks its type, and requires at least 60 columns and 10 rows.
The display uses pinned `crossterm` 0.29.0 with event and clipboard features
disabled. It adds no GUI or required Node runtime.

Keys use canonical unpadded base64url: exactly 43 characters for 32 bytes.
This encoding is not a URL and must not enter commands, URLs, chat, files, or
logs. The key is printed once on an owned alternate screen. Owned raw and
encoded buffers are zeroizing and are not retained by the display guard.
Close clears that screen and restores the previous screen; cleanup failures
produce fixed errors. A process-local lease prevents two displays from replacing
each other. It is not the required OS-wide pending-login slot.

The caller must retain the separate ticket and close the display on claim,
cancellation, expiry, or disconnect. This API does not run an expiry worker or
coordinate the waiting endpoint. Terminal I/O is synchronous. Terminal recording,
screen capture, formatter/OS copies, crashes, and forced termination are outside
the buffer-erasure guarantee. No browser key-input field or product ingress is
enabled by the display.

Six unit checks and one real redirected-output process check passed on Windows.
The app terminal refused the invented-key preview because its native standard
streams were not all terminals. The refusal was preserved. An owned native
console preview later completed rendering and explicit close with exit code 0.
The first native attempt exposed missing read access on the owned console handle;
the corrected handle permits the
[Windows console-mode check](https://learn.microsoft.com/en-us/windows/console/getconsolemode)
without reading any console content. Only an invented key was used.
Native console exit confirms the display operations, not a pixel-level visual
review or a product browser handoff. Linux and macOS display execution remains
unverified.

### Windows pending-login reservation

The [core reservation](../src/beyond-gate-core/src/auth/handoff/slot.rs) now
reserves one pending login per Windows logon. It is separate from the terminal
display lease and from the separate native-host endpoint.
The caller must acquire it before displaying a key and retain it through
submission, validation, save, cancellation, or expiry. That product controller
integration remains incomplete.

The namespace uses exactly one logon SID from the current process token.
Windows defines [logon SIDs](https://learn.microsoft.com/en-us/windows/win32/secauthz/well-known-sids)
as distinct for each logon. The code does not substitute a user SID, user name,
PID, or environment value. Actual OS identity and pipe paths stay out of
diagnostics. This name provides exclusion, not application authentication.
Development checks use a disjoint namespace with a bounded public transaction.

Pinned `interprocess` 2.4.7 creates the server with
[`FILE_FLAG_FIRST_PIPE_INSTANCE`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createnamedpipew),
one instance, remote clients rejected, and inheritance disabled.
An existing object makes acquisition fail. It is never adopted, overwritten,
or treated as proof of a trusted pending CLI.
The reservation never calls `accept`, reads, or writes.
The library's
[single-instance accept limitation](https://docs.rs/interprocess/2.4.7/x86_64-pc-windows-msvc/interprocess/os/windows/named_pipe/struct.PipeListenerOptions.html)
therefore does not affect this reservation. The endpoint below avoids that
accept path through safe owned-handle transfer.

The protected DACL is created atomically as `D:P(D;;GA;;;OW)`.
The [Owner Rights SID](https://learn.microsoft.com/en-us/windows/win32/secauthz/sid-strings)
deny ACE explicitly denies all owner access, including permission to change the DACL.
No client receives data, metadata, pipe-instance creation, or permission-change
rights. The reservation stores no secret.
Its owned handle releases the namespace on drop or process termination.
Administrators, OS compromise, and trusted-process memory tampering remain
outside the stated threat boundary. Pre-creation can deny availability but
cannot become an authenticated handoff.

Seven Windows unit checks and three new separate-process checks passed locally.
Actual handle opens rejected each selected client and owner-control right with
Windows error 5. Two competing fixture processes produced exactly one owner.
A fresh process acquired the namespace after normal exit and after termination
of only the owned holder process. Foreign-object rejection and drop release
also passed. No product slot, browser, account, credential store, or network
request was used.

Other-user/logon execution and hosted Windows CI remain unverified.
Non-Windows platforms fail explicitly; their reservation implementation remains
absent. This component does not close IPC-1 or enable product login.

### Windows single-instance handoff endpoint

The [endpoint module](../src/beyond-gate-core/src/auth/handoff/endpoint.rs) creates
one local, non-inheritable server instance while borrowing the held reservation.
Safe owned-handle conversions move the initial synchronous listener into a
synchronous stream, then into a Tokio stream. No normal listener `accept` call
creates a replacement instance. A compile-fail example prevents releasing the
reservation before its connected server stream.

The endpoint has a separate logon-specific namespace. Its protected DACL denies
Owner Rights `FILE_CREATE_PIPE_INSTANCE`, `WRITE_DAC`, and `WRITE_OWNER` with
mask `0xc0004`. It grants only current-logon client data and metadata access with
mask `0x12019b`. The client requests these individual rights, not `GENERIC_WRITE`.
It uses overlapped I/O and anonymous security QoS. Remote clients are rejected.
[Microsoft's pipe access rules](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)
require an additional instance-creation check for another server.

Actual client opens reject both owner-control rights with Windows error 5.
An actual second server creation also returns error 5 when the first server
allows two instances and leaves a spare slot. This checks the DACL, not the
one-instance limit. A successful client open requesting `GENERIC_WRITE` is not
used as a proxy for server-instance creation or permission-change authority.

Connection readiness uses the client process ID query only as transport
metadata. Error 1168 means no client yet. Other failures stop the wait.
The wait is capped at ten seconds and the original pending deadline. No PID,
name, SID, or handle authenticates an application peer. The connected stream
must complete the existing one-use Noise pairing before application messages.

The stream has no user-space output buffer. Flush does not prove peer receipt.
Drop and shutdown close without starting the library's blocking flush or linger
workers. Unread output can be discarded. Cancelled kernel I/O can retain the
object briefly; local cleanup checks require release within one second.
Product terminal-result acknowledgement and delivery semantics remain unresolved.
The process fixture uses a bound cancellation exchange and waits for close;
this cleanup exchange is not an accepted product success protocol.

Eight endpoint checks passed locally. They cover real bidirectional bytes,
owner-control and extra-instance rejection, missing endpoints, invalid expiry,
deadline expiry, cancelled waits, unread-output cleanup, and fixed redacted Debug.
The safe-transfer reservation proof and all nine paired-process checks also
passed. Three process cases use actual Windows pipes for receipt, wrong-key
rejection, and wrong-transaction rejection. They never claim validation or save.

This is offline Windows transport evidence, not product login completion.
Other-logon execution, hosted Windows CI, browser integration, PROFILE-1,
CORE-1, and the complete IPC-1 application mechanism remain unverified.

### Exact negative-authentication check scope

The [negative probe](../src/beyond-gate-core/tests/auth_negative.rs) has two
ignored live tests and requires the exact matching scope flag. Nine offline
tests verify fixed input, request bounds, stop statuses, exact approval,
TLS-only live use, JSON shape inspection, and bounded response bodies.
It is development-only and supplies no real account material.

| Field | Scope |
|-------|-------|
| Host and operation | Direct HTTPS POST to `https://auth-service.dndbeyond.com/v1/cobalt-token` only. Normal TLS verification remains enabled. |
| Input | First request has no Cookie header. Second uses only the compiled invented `CobaltSession=beyond-gate-invalid-synthetic-v1` value. No user input, account profile, bearer token, or credential store. |
| Budget | At most two sequential requests. Five-second connection limit and ten-second total limit per request. At most twenty seconds of request time after build. No retry, redirect, proxy, refresh, or resource read. |
| Results | Record only fixed case names, HTTP statuses, and stop/pass classification. Do not read response bodies or record headers, tokens, URLs from errors, or account data. |
| Stop conditions | Continue to the second case only after the expected negative 401. Stop the entire check on 200 or any other status, including 403, redirect, rate limit, server failure, timeout, or transport failure. Never bypass an access denial or challenge. |
| Cleanup | Drop each response and the HTTP client. Clear the exact non-secret scope flag from the shell. No credential, registration, browser profile, or secret-bearing file is created. |

Two 401 observations would establish negative status behavior only. They would
not prove authenticated/guest distinction for a successful response, accepted
token schema, account validation, refresh, expiry, or persistent material.
The precise execution question must be answered before the ignored live test
runs. General product approval must not set its scope flag automatically.

The separate metadata-only packet below remains one manual observation in the
user's selected signed-in browser. The user must inspect the cookie attributes
locally, never send the value or a screenshot containing it.
The separate synthetic-store packet uses only the existing `beyond-gate-e2e`
identity and one random context. The user approved its exact local scope:
synthetic save, status, logout, and cleanup of that one entry, with no product
entry access or network requests. Its execution results must be recorded
separately from the negative-authentication result.

### Approved negative check result

The user approved the exact negative-only scope. One missing-cookie POST ran
and returned HTTP 200. The check correctly failed and stopped immediately.
The invented-cookie case did not run. No response body, token, or account
metadata was read. The exact scope flag was removed from the execution shell.

This observation disproves the proposed assumption that HTTP status alone
will reject absent credentials at this candidate endpoint. It does not prove
that the response contains a guest token, an authenticated token, a challenge,
or any usable credential. Those remain unknown because the approved check
deliberately did not read the body.

Do not treat HTTP 200 or a nonempty token as `ServiceValidated`.
A new narrowly scoped response-shape observation would need separate exact
approval before another request or body read. No expanded experiment, retry,
or second request was performed after this stop.

The user then approved one new absent-cookie JSON-shape request with a ten-second
limit and at most 64 KiB of body inspection. It ran separately and returned
HTTP 200, `token_present=false`, and `ttl_positive=true`.
These Booleans describe only the `token` and `ttl` fields. No token text, raw body,
headers, claims, or account identifiers were printed or saved. The owned body
was zeroizing, and token text was borrowed during parsing. Parser-internal
scratch copies are outside that allocation guarantee.

The shape check passed as an observation, not as authentication. It establishes
that this absent-cookie JSON response had no nonempty `token`; a positive `ttl`
alone is not usability or expiry evidence. It does not establish the meaning
of another response field or an authenticated response. The invented-cookie
case and a positive real-session check have not run.
There were two approved requests in total, one in each separately approved scope,
with no retry, redirect, or resource read.

### User-reported Edge cookie metadata

The user completed the bounded manual inspection and confirmed exactly one
candidate in the selected Edge site's cookie table. No cookie value, screenshot,
account identity, browser export, or other cookie was provided.

| Attribute | User-reported observation |
|-----------|---------------------------|
| Name | `CobaltSession` |
| Domain | `.dndbeyond.com` |
| Path | `/` |
| Secure | Checked; explicitly confirmed |
| HttpOnly | Checked |
| SameSite | Blank in DevTools; do not substitute `lax` |
| Partition column | Blank in DevTools; exact API partition metadata not observed |
| Expiry | A future persistent expiry, not `Session` |
| Size | 172 bytes as displayed by DevTools |
| Candidate count | Exactly one in the selected site's table |

This is a non-secret profile observation from one Edge session, not a fixture
or account snapshot. It supports the candidate domain/path and flags; it does
not establish every session's size, Chrome behavior, `hostOnly` API value,
partition-store behavior, expiry guarantees, or session usability.
See the official [DevTools cookie fields](https://developer.chrome.com/docs/devtools/application/cookies#fields).
No real credential is copied into code, fixtures, or artifacts.

PROFILE-1 now has concrete user-reported evidence rather than community hints
alone. The compiled selection must still reject ambiguity, enforce the actual
browser API scope, and distinguish unobserved fields from verified attributes.
Do not broaden permissions. The later API observation below supplies the
previously unobserved fields; these manual DevTools facts remain distinct.

### Approved one-use Edge API observation

**User-reported: 2026-11-01. Result: `observed`.**
The user approved and ran one local metadata diagnostic in the signed-in regular
Edge profile. The narrow `cookies.getAll` query had a five-second wait limit.
The diagnostic marked the observation used before querying and reported only
fixed metadata and a native-size Boolean.

| Reported field | Value |
|----------------|-------|
| `name_matches`, `domain_matches`, `path_matches` | All `true` |
| `host_only` | `false` |
| `secure`, `http_only` | Both `true` |
| `same_site` | `unspecified` |
| `partitioned` | `false` |
| `regular_store` | `true` |
| `persistent`, `expiry_present`, `expiry_future` | All `true` |
| `fits_native` | `true` |

The `observed` classification records one candidate from the exact query.
No value, exact length or expiry, account field, raw cookie object, or export was
provided. The result supports the selected contract, not every account,
Chrome behavior, future usability, account validation, or protected saving.
The query scope is consumed. No new app-issued network request or native-store
operation was part of this observation. Existing tabs could make background
requests; network isolation is not claimed.

Removal of the temporary diagnostic and closure of its inspector were requested.
The user has not yet confirmed removal. Do not reload or reinstall to repeat
the observation. A new cookie read, service request, or native-store test needs
fresh, exact approval.

### Approved isolated-store result

The separately approved existing harness passed `native.auth.absent`,
`native.auth.stored-unverified`, `native.auth.logout`, and
`native.auth.repeated-logout` in Windows Credential Manager. The harness used
only synthetic bytes under `beyond-gate-e2e` and one random context. Its final
absence assertion and cleanup completed without a native/file-cleanup failure.
It did not open the product `beyond-gate` entry or make network requests.

The full manual suite returned exit 1: 102 cases passed, 47 remained blocked,
and five installation-harness cases failed. The failed cases were
`install.claude-code.preview`, `install.claude-code.conflict`,
`install.copilot-cli.preview`, `install.copilot-cli.malformed`, and
`install.vscode.conflict`, all with the safe `harness_error` reason.
Do not describe the full suite as passed or infer a cause from that reason.
These failures are separate from the successful native-store assertions.

The store result proves isolated synthetic local behavior only. It does not
select persistent production material, prove an actual-cookie save, validate
an account, or complete #29/#30.

### Accepted Windows service rule

The user approved this rule after a separately scoped current-cookie observation:
one direct POST to `https://auth-service.dndbeyond.com/v1/cobalt-token`, HTTP 200,
a nonempty token of at most 8,192 bytes, a positive numeric or decimal-string
`http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier`, and a
required future JWT `exp`. Missing, zero, malformed, and expired evidence fails
closed. The response must be JSON with no non-identity content encoding.
Connect time is at most five seconds; the entire request is at most fifteen
seconds, within the original login deadline. The body is at most 64 KiB,
including streamed bodies without a declared length.

No proxy, redirect, automatic retry, alternate origin, second validation request,
content read, or independent signature-verification claim is added.
A service-issued expired bearer yields `session_expired`; this does not prove
the reusable cookie expired. Store opening starts only after accepted evidence.
This rule was first checked with field observations and offline fixtures.
Later approved Windows product checks exercised the real API and native store.

The evidence sequence used four service requests under separate approvals:
absent cookie returned HTTP 200 with no token; invented cookie returned HTTP 401;
an older user-supplied cookie returned HTTP 401; a current cookie after human
sign-in returned HTTP 200 with a token, positive nameidentifier, and future exp.
The initial real-cookie console received no input, as the user confirmed, and
did not spend a request. All reports kept `account_validated=false` and
`saved=false`. They were field observations, not product login or protected-save
results. The older rejection does not prove expiry. All request scopes are
consumed. No retry or new live test is implicitly approved.

### Accepted Windows pairing boundary

The user explicitly selected **Accept the stated Windows pairing boundary**.
Windows v1 may use possession of the one-use, user-entered pairing key as
application authorization. It does not independently prove that the approved
extension launched the native host. The terminal-to-extension transfer is the
user's authorization step, not a claim of OS or publisher attestation.

Trust the installed extension, native host, CLI, and selected browser.
Processes that can read application memory or replace the extension or native
host are outside this boundary. Do not claim protection against installation
tampering or a compromised Windows session. Ordinary same-user connections
without the correct key remain unauthorized.

Keep explicit consent before cookie access, exact extension-origin checks,
logon-restricted local pipes, remote rejection, first-instance ownership,
one-use encrypted connection binding, replay rejection, original deadlines,
and cancellation cleanup. Public transaction identifiers and caller-origin
strings are not grants. No password input or credential-bearing CLI argument,
environment variable, MCP tool, URL, configuration, or log is permitted.

This decision allows product integration against that boundary. It is not
permission for a new account request, browser-profile read, or native-store test.
The earlier independent launch-proof requirements are historical. The Windows
runtime, synthetic browser/CLI tests, and approved product checks are implemented.

## Acceptance and remaining gates

The [contract-blocker decision](#contract-blocker-decision) is historical.
The accepted Windows contracts have offline, hosted synthetic, and approved
product evidence. macOS/Linux login and reusable authenticated reads remain deferred.

| Decision | Status |
|----------|--------|
| Contract-first scope and stated threat boundary | User selected |
| Both version 1 message contracts and limits | Implemented and checked through the product controller and native/core flow |
| Application-peer authentication and discovery | Accepted one-use key boundary; Windows encrypted channel and logon-restricted endpoint implemented and tested |
| Actual cookie profile | Selected Windows contract implemented; Chrome and Edge Dev product selection passed; diagnostic cleanup confirmed |
| Shared core validation and persistence | Windows service rule and protected save implemented; establishment boundary agreed with #30 |
| Rust fixture consistency checks | Implemented; data checks only |
| Rust message parsers and bounded async framing | Implemented; production CLI/native ingress and separate synthetic fixtures |
| Product host-to-CLI coverage | Synthetic success, rejection, cancellation, lost acknowledgement, and unavailable storage passed |
| Real Chrome and Edge synthetic browser/native-host tests | Passed locally and in hosted Windows jobs; no account or native store used |
| Chrome and Edge synthetic CI | Separate hosted jobs passed on the recorded implementation commit |
| Live-account and native-store checks | Approved Chrome and Edge Dev product checks passed; test entries, extensions, and owned installation removed |

The existing product evidence does not cover later source changes. Review and
merge remain separate from implementation and test completion. All private-test
approval budgets are consumed. New account or credential-store tests need exact
hosts, operations, request budgets, stop conditions, and cleanup before approval.
No raw-cookie CLI arguments, MCP credential parameters,
plaintext fallback, Store publication, or access-control workaround is added.

## Public source follow-up

**Reviewed: 2026-10-31. PROFILE-1 and CORE-1 remain unresolved.**
This is a historical review. The later accepted Windows rules and product
evidence resolve these implementation blockers.

The pinned
[DDBI cookie selector](https://github.com/MrPrimate/ddbi-utilities-extension/blob/40c7f29af86ba0588080aa9792befac5a682c38c/src/popup/cobalt.js)
filters `CobaltSession` by `.dndbeyond.com` and selects the first result.
This is evidence of a third-party selection rule, not an observation of the
actual cookie's path, flags, partitioning, expiry, or ambiguity.
Its [tests](https://github.com/MrPrimate/ddbi-utilities-extension/blob/40c7f29af86ba0588080aa9792befac5a682c38c/test/cobalt.test.js)
use mocked cookie callbacks without those attributes. Do not treat these
tests as a verified production profile or copy their first-result selection.

The pinned
[DDB proxy auth helper](https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/auth.js)
checks the returned token field. With no supplied cookie, it returns locally
without sending an exchange request. This does not establish the upstream
response for absent or invalid credentials, or distinguish guest access.
The [setup guide](https://docs.ddb.mrprimate.co.uk/docs/ddb-importer/initial-setup)
confirms the candidate cookie name and login context, not a complete service
validation contract. Token receipt alone remains insufficient.

A separate credential-free negative check was proposed: at most two bounded
POSTs to the candidate exchange endpoint, with absent and invented invalid
cookie input. The approval request received no explicit approval. No
authentication request, account access, browser-profile access, or native-store
operation was performed. Do not infer permission from the request to work
autonomously.

## Contract-blocker decision

**Result: IPC-1, PROFILE-1, and CORE-1 remain blocked.** This milestone narrows
the earlier full implementation plan to the missing contract evidence.
This section records the earlier decision, not the current Windows status.
Its proposed observation packets are not active approvals.
It does not add a host, browser registration, transport, core API, or dependency.
No browser, account, or credential store was used. A session-only synthetic
standard-handle experiment is recorded below.

The review read #29's four artifacts and the bodies and comments of #29/#30/#46.
Those issues returned no comments. Artifact lists for #30 and #46 were empty.
There is no new owner agreement in those sources. The recorded Edge prototype
pass and manual exchange retain their existing limited meanings.

### IPC-1 missing launch and connection proof

The earlier Windows review covers named-pipe permissions and process metadata.
Two additional interface checks do not close its missing trust chain:

| Candidate | Source-backed behavior | Decision under the current boundary |
|-----------|------------------------|-------------------------------------|
| Inherited anonymous pipe from CLI to host | [Windows anonymous pipes](https://learn.microsoft.com/en-us/windows/win32/ipc/anonymous-pipe-operations) require a handle to reach the peer, usually by child inheritance. Transfer to an unrelated process needs a separate handle-delivery path. | The browser starts its own native host. A CLI-created child is not that host. No reviewed authenticated path delivers a CLI handle to the browser-launched host. This moves the discovery problem; it does not solve it. |
| Browser native window handle | [Chrome native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging#native-messaging-protocol) supplies `--parent-window` to parent native UI. It can be zero for a service worker. | This is a command argument, not launch attestation or connection authorization. Reject it as a replacement for IPC-1 proof. |

Safe Rust access to pipe peer metadata is available through the reviewed
[Interprocess API](https://docs.rs/interprocess/2.4.7/x86_64-pc-windows-msvc/interprocess/os/windows/named_pipe/struct.PipeStream.html).
That capability is not the missing browser-launch proof. No dependency is
selected. A parser or child-process test cannot establish a trust mechanism
that these candidates do not specify. No experiment was run for those two
candidates. The later standard-handle test checks a different capability.

To clear IPC-1, #29 must supply one complete mechanism that meets the
[five existing requirements](#ipc-decision-and-blocker), with actual transport
tests. It must bind OS-verified peer evidence to live process instances and
the connection, reject endpoint replacement, and prove the authorized extension
launch. It must also define logon isolation, concurrent-login exclusion,
deadline enforcement, and restart cleanup. These remain requirements, not
implemented checks. A direct launch of the installed host must still fail.

If that needs a pairing step, a new installation trust anchor, or a narrower
threat boundary, request a separate user decision with its exact mechanism and
trade-offs. None is selected here. The result is a missing reviewed mechanism,
not proof that no secure mechanism can exist. Do not start product integration
by substituting a caller origin, parent window, peer PID, or challenge.

### Synthetic standard-handle follow-up

The pinned
[Chromium Windows launcher](https://chromium.googlesource.com/chromium/src/+/b7205b2952d33d03a3188f57df44dc2bc6d1215c/chrome/browser/extensions/api/messaging/launch_context_win.cc)
creates **named pipes** for native messaging. Do not assume that `stdio` means
anonymous pipes. In its direct `.exe` launch path, the browser opens the client
ends before launching the host and passes them as inherited standard handles.
Its other path uses command-shell redirection. This is reviewed Chromium source,
not evidence of the exact Chrome or Edge versions installed on a user's system.

A session-only PowerShell 7 experiment used two named pipes and one owned child
process. It assigned the child's standard input and output to its test pipe
handles. Both
[`GetNamedPipeServerProcessId`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeserverprocessid)
and
[`GetNamedPipeClientProcessId`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeclientprocessid)
succeeded on both standard handles. The reported server IDs matched the test
parent; connector IDs matched the child that opened those client ends.
The test required all four matches and successful child exit. Connection,
response, and process waits were bounded. It closed its streams and process
handles and created no registration, credential entry, or account request.

**Result: OS pipe metadata is available through synthetic standard handles.**
This experiment did not launch a browser, use an extension, test the browser's
inherited-client path, verify executable identity, or authenticate a CLI endpoint.
The test used managed native-API calls from PowerShell, not a selected Rust
product dependency. No adapter ingress or native host is enabled.

The direct-launch source also matters when interpreting the IDs: its browser,
not the eventual host, opens those client handles. A connector process ID must
not be treated as the identity of every process that later inherits or receives
the handle. Windows
[handle duplication](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-duplicatehandle)
can transfer access to the same object to another process. A complete mechanism
must address that transfer under the existing local-process threat boundary.
No handle-transfer or browser-launch attack was tested here.

IPC-1 therefore remains blocked. A later mechanism must establish live trusted
process identity, connection binding, the approved extension launch, false-CLI
rejection, and compatible safe Rust APIs. OS-reported pipe ownership is a useful
input to that review, not sufficient authorization to read a cookie.

### PROFILE-1 evidence needed from the browser

The current [Chrome cookies API](https://developer.chrome.com/docs/extensions/reference/api/cookies)
defines cookie metadata and host permissions. It does not define D&D Beyond's
cookie attributes. The official
[authentication guidance](https://dndbeyond-support.wizards.com/hc/en-us/articles/7747237593108-Authentication-Information)
describes sign-in methods, not a native-client credential profile.
The successful manual probe does not establish that profile either.

| Required evidence | Current result |
|-------------------|----------------|
| Exact query URL, name, domain, path, and host-only status | `CobaltSession` is a community-evidenced candidate name. Its production scope remains unverified. |
| Secure, HttpOnly, SameSite, partition, and current regular store | The API exposes these fields. Actual required values are unknown; synthetic attributes are not evidence. |
| Missing or multiple candidates | `cookies.get` selects by path length and creation time. It does not prove uniqueness. #46 must define a narrowly filtered selection that rejects ambiguity without unrelated cookie or profile enumeration. |
| Expiry and value size | Cookie metadata can establish browser expiry, not bearer-token lifetime or service usability. Required production size remains unknown. |
| Minimum material | One manual exchange supports a candidate input only. It does not prove a reusable persistent session, bounded refresh, or service validation. |

PROFILE-1 stays blocked until #29/#46 record evidence for each profile field
and #30 establishes which material its operation needs. Do not broaden host
permissions to compensate for an unknown scope.

The following is a **proposed approval packet**, not permission to run it:

| Field | Proposed metadata-only observation |
|-------|-----------------------------------|
| Host and context | `https://www.dndbeyond.com/`, already open after the user's own login, in one regular Chrome or Edge profile. This is a candidate observation site, not an accepted query URL. |
| Operation | The user inspects metadata for the candidate `CobaltSession` cookie locally. Record only the profile attributes above and bounded length information. Do not copy its value, other cookies, account fields, screenshots, or browser exports. |
| Budget | One manual inspection, at most 300 seconds. No app-issued network request, navigation, reload, exchange, refresh, browser automation, or OS-store operation. An existing page can make background requests; this is not a network-isolation claim. |
| Stop conditions | Stop on missing or ambiguous material, a new host or login requirement, access denial, bot protection, or any need to expose the value. Do not search other profiles or expand the operation. |
| Cleanup | Close the inspection view. Keep only the non-secret profile observations. No registration, extension, credential entry, or secret-bearing file may be created. |

Separate explicit approval is required before this observation. If its context
or limits cannot be met, revise the packet first. Metadata cannot establish
minimum usable material. Any later exchange or validation experiment needs
its own exact host, method, path, request count, response/time bounds, stop
conditions, and cleanup. No service-validation endpoint is selected here.

### CORE-1 decisions required from the shared operation

The existing [shared core API](#shared-core-boundary) remains a proposal.
Do not treat this review as #30 owner agreement. The current opaque
`SessionSecret` cannot supply a verified cookie type or a persistent format.

| Boundary | Required decision before acceptance | Evidence still missing |
|----------|-------------------------------------|------------------------|
| Input | Own one redacted, bounded cookie and a compiled evidence-backed profile; no arbitrary URL, cookie name, or credential bag | Accepted PROFILE-1 and the profile's operation requirements |
| Validation | Fix the HTTPS host, method, path, request/body limits, response schema, and meaning of one bounded non-content validation operation; include any exchange in the total deadline | No accepted operation proves `ServiceValidated`; a nonempty exchange token is insufficient |
| Persistent material | Choose the minimum reusable credential kind, portable versioned byte encoding, exact encoded maximum, and handling of unknown versions and existing opaque entries | No accepted format or evidence of which secret must survive restart |
| Result | Return `SavedSession` only after service validation and a confirmed protected save; keep credentials and derived tokens out of the result | Accepted validation and persistence contracts plus #30 agreement |
| Failure/control | Reuse the proposed safe typed errors and monotonic deadline; preserve pre-save state and report uncertainty after a write starts | Executable core lifecycle evidence under #30; no adapter may supply a success fallback |

Two existing limits must be reconciled before choosing the profile and format:

- The current `SessionHeaders::cookie` check in
  `src/beyond-gate-core/src/http/mod.rs` caps the **sum of name and value** at
  8,192 bytes. For the 13-byte candidate name `CobaltSession`, the value cap is
  8,179 bytes, not the wire proposal's 8,192. An accepted profile must fit each
  boundary or require a separately reviewed implementation change.
- The selected [Windows keyring backend](https://docs.rs/windows-native-keyring-store/1.1.0/windows_native_keyring_store/)
  maps an entry to a generic Windows credential.
  [`CREDENTIALW`](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw)
  caps its secret blob at 2,560 bytes. A versioned encoding and all its overhead
  must fit that limit as well as the core's 8 KiB bound. An in-memory mock's
  acceptance of 8 KiB does not prove a Windows save can succeed.

These are constraints on a future design, not new runtime limits or an approved
encoding. Do not truncate a credential, split it across entries, store a bearer
token instead without evidence, or add a file fallback to make it fit.
A predictable local encoding/size failure must be detected before a native write;
an unconfirmed result after a write starts remains `save_indeterminate`.

The shared examples already distinguish receipt, validation, and save:

| Existing synthetic case | Meaning both adapters must preserve |
|-------------------------|-------------------------------------|
| `receipt-is-not-validated-login` | Missing validation contract yields `unsupported_contract`; no save |
| `rejected-not-guessed-expired` / `verified-expiry` | Rejection and evidence-backed expiry are different errors |
| `store-unavailable-before-write` / `store-denied-before-write` | Storage failure is not an absent session; no write has started |
| `unknown-expiry-does-not-block-validated-save` | Unknown expiry is not a guessed failure when validation and save are otherwise established |
| `failed-save-not-unchanged` / `lost-final-reply` | Unknown completion cannot become success or a claim that storage is unchanged |

These are test expectations, not executable core evidence. MCP consumes #30's
shared lifecycle later; it does not receive a cookie through a tool.

### Checkpoint disposition

| Item | Disposition and next owner |
|------|----------------------------|
| IPC-1 | Blocked; synthetic stdio metadata is available, but #29 still needs authenticated discovery, live trusted peer identities, browser/extension-launch proof, and actual transport evidence |
| PROFILE-1 | Blocked; #29/#46 need approved evidence for exact cookie selection, and #30 must establish minimum usable material |
| CORE-1 | Blocked; #30/#29 need the validation operation, persistent encoding within backend limits, and explicit agreement |
| Protocol and fixture versions | Unchanged: both protocols remain proposed version 1; fixture version 1 remains `proposed-blocked` |
| Proposed bounds | Still proposed: 16 KiB frames, 8 KiB wire values, nesting/frame-count caps, five-minute pending deadline, and phase budgets are not frozen product defaults |
| Browser evidence | #46 still owes separate real Chrome and Edge synthetic CI and actual manual-package extension identities |

No wire shape, state transition, error code, or synthetic expectation changed.
No fixture edit or version bump is needed for this review. In particular,
`exact-value-bound` tests handoff admission, not successful HTTP use or storage.
Freezing either protocol requires agreement on the bounds, trust mechanism,
profile, and core boundary together. A future incompatible wire change needs a
new protocol version; a fixture-format change needs its own version review.

Keep #29 open and product login blocked. The next product milestone needs a
separate implementation plan after the required evidence and agreements exist.
This review adds no reciprocal dependency on completion of #29/#30/#46 and no
approval for live tests, credential access, pairing, publication, or policy changes.
