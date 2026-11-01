# Changelog

This project uses [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Completed Windows human-controlled browser login and protected handoff.
  Approved Chrome and Edge Dev tests passed actual product-console login,
  direct-service validation, native protected save, restart/status, logout,
  repeated logout, and final absence. Hosted Chrome/Edge synthetic jobs and
  three-OS Rust checks passed. The shared core contract is agreed with #30.
  macOS/Linux browser login and reusable live-session handling remain separate.
  Earlier authentication entries below record intermediate milestones; their
  incomplete-state notes are historical, not the current Windows login status.
- Real Chrome/Edge process checks for the product browser controller, native
  relay, private Windows pipe, Noise pairing, and shared completion. The fixture
  uses only invented cookies, local mock responses, and in-memory storage.
  Cancellation does not save; lost acknowledgement remains indeterminate.
  Windows CI runs this separately from receipt-only feasibility checks.
  Windows archives now include the native host and bounded manual browser ZIP,
  with package identity, permission, and rejection checks.
- Windows native-host setup and owned removal with explicit browser and stable
  directory selection, public manifest preview, and confirmation. Conflict
  checks cover both registry views and hives, including Edge fallback locations.
  Existing foreign setup is not overwritten. Registry creation uses a Windows
  transaction. The user accepted registry entries, manifests, and launch mapping
  as trusted installed configuration. Actual product browser integration,
  packaging, and fresh approved live/save evidence remain incomplete.
- Manual Chrome/Edge login form in a user-selected separate browser window.
  Explicit consent and bound native authorization gate the single selected
  cookie query. Keys are cleared from the form before connection. Final
  acknowledgement gates saved-session evidence; failed or cancelled pending
  queries cannot submit later. Shared invented production recipes now drive
  browser and Rust codec/relay checks. Offline logic and synthetic UI rendering
  checks passed. Real installation and end-to-end evidence
  remain incomplete.
- Actual Windows native-host executable and terminal-gated CLI login
  orchestration. One-use pairing now reaches real shared validation and save,
  with concurrent cancellation and final browser acknowledgement. A fixed
  five-second save wait prevents late write admission. Post-submit failures
  cannot report unchanged storage without the CLI's confirmation. Offline
  process and paired local-service/in-memory-store checks cover refusal,
  cancellation, rejection, timeout, uncertain writes, and lost/wrong receipts.
  Browser setup and end-to-end product evidence remain incomplete.
- User-reported one-use Edge API metadata evidence and the selected Windows
  `cobalt-session-v1` contract. Windows parsers enforce its 2,552-byte value
  limit. The paired controller fixes the profile before authorization and
  rejects changes at open or submit. Production-bound receipt conversion moves
  material into the shared core without a credential copy; synthetic receipts
  are rejected. Offline tests use invented bytes. The metadata scope is
  consumed; diagnostic removal is unconfirmed. Product login remains unsupported.
- User-accepted Windows pairing boundary: the one-use, user-entered key
  authorizes the application connection without independent extension-launch
  proof. Installed code and process memory remain trusted. Origin checks,
  consent, local pipe controls, deadlines, and replay rejection remain required.
- Shared Windows session establishment with one fixed direct TLS exchange,
  the user-accepted positive account and future bearer-expiry rule, and one
  confirmed protected save. Versioned cookie bytes fit the 2,560-byte native
  limit; derived bearers and account claims are not persisted. Atomic
  cancellation/write admission and worker-owned reservations preserve pre-write
  state and report uncertain started saves. Offline tests use local services
  and in-memory keyring entries. Product browser login remains unsupported.
- Separately approved fresh-cookie field evidence: HTTP 200 with a nonempty
  token, positive nameidentifier, and future expiry. Negative and older-cookie
  rejection evidence remains distinct. No product validation, content read,
  or protected save occurred; all live request scopes are consumed.
- Separately gated developer validation-field comparison with a maximum of
  three approved direct exchanges, hidden real-cookie input, fixed anonymous
  and expiry facts, zeroizing owned buffers, and no storage or content access.
  Seven new offline checks cover field redaction, bounds, approval, terminal
  refusal, missing-token negative evidence, and actual mock HTTP behavior.
  JWT decoding does not validate a
  product session. The live comparison remains ignored by default.
- Windows single-instance handoff endpoint with safe owned-handle transfer,
  reservation-bound server lifetime, logon-only client access, explicit owner
  control denial, anonymous client security QoS, and bounded connection waiting.
  Drop closes without blocking flush workers. Eight endpoint checks and actual
  paired-pipe process cases cover access, receipt, rejection, and cleanup.
  Application pairing is still required; product browser login and session
  validation remain incomplete.
- Windows pending-login reservation with a logon-specific namespace, first-instance
  ownership, one non-inheritable server handle, and a protected deny-all Owner
  Rights DACL. The reservation accepts no client and grants no data, pipe-instance,
  or permission-change rights. Offline checks cover competing processes, normal
  exit, owned-process termination, rejected client rights, and a pre-existing
  foreign object. The complete application handoff and product login remain incomplete.
- Interactive terminal pairing display with redirected-output refusal, canonical
  43-character base64url keys, an owned alternate screen, explicit cleanup,
  process-local display exclusion, and redacted errors. Six unit checks and one
  process refusal check passed. An invented-key Windows native-console preview
  completed rendering and close. Product endpoint and browser integration remain
  incomplete; terminal/OS copies have no erasure guarantee.
- System-generated, redacted 256-bit pairing keys and separate 128-bit public
  transactions using pinned `getrandom` 0.4.3, without a randomness fallback.
  A consuming application controller now sends authorization only after a
  consented open on an already paired connection and returns one bound receipt,
  never login success. Eleven flow checks and two key/error checks passed
  locally. Ownership doctests reject repeated transfer and receipt.
- Bounded encrypted host-to-CLI streams with a fixed binary envelope, consuming
  pairing tickets, shared deadlines, directional frame budgets, and failure or
  cancellation cleanup. Sixteen stream checks and three separate-process checks
  passed locally on Windows. The helper now uses core-generated authorization
  and rejects wrong transactions as well as wrong keys. It requires an explicit
  feature build; the Rust CI matrix includes these checks. Product key delivery,
  OS-isolated IPC, browser integration, validation, and save remain incomplete.
- Encrypted pairing primitive using pinned `snow` 0.10.0 and a fixed
  Noise PSK handshake and a consuming, expiring pairing ticket. Thirteen offline
  checks cover real encryption, proof failures, tampering, replay, bounds, safe
  causes, and exact ticket deadlines. User-mediated secret delivery, OS-wide
  pending-login control, and native IPC are not yet implemented.
- Separately gated, ignored negative-authentication probe with fixed absent
  and invented-cookie requests, HTTP-status-only reporting, and stop conditions.
  Nine offline checks passed. No real account, credential-store operation, or
  live request is enabled by the default tests.
- Development-only Chrome and Edge synthetic native-messaging checks with a
  feature-required Rust fixture host, isolated profiles, narrow invented cookie
  scope, and a rejecting local proxy. Local checks cover real cookie APIs,
  missing registration, wrong extension origin, absent consent, no pending
  authorization, native receipt, and owned cleanup. Separate Windows browser CI
  jobs are defined but have not run. No account validation, protected save,
  authenticated CLI channel, or product extension is added.
- Shared version 1 handoff message parsers and bounded async frame readers and
  writers for the invented profile. Owned input and credential allocations are
  zeroizing; error chains hide peer and dependency text. Tests execute shared
  message and frame recipes, exact bounds, duplicate fields, frame budgets,
  deadlines, and cancellation. No authenticated transport, native-host ingress,
  product browser integration, or product login is enabled.
- Offline Rust consistency checks for the shared browser handoff fixture.
  Checks cover canonical message shapes, proposed bounds, unique case IDs,
  message references, and safe expected error effects. They do not execute
  event recipes, authenticate peers, or implement product login.
- Proposed version 1 browser-to-host and host-to-CLI contracts, a shared core
  session-establishment API proposal, and canonical synthetic conformance data.
  The contract checkpoint remains blocked on application-peer authentication,
  the production cookie profile, and core validation/persistence agreement.
  No product login, host registration, or credential access is added.
- Dedicated CLI command and local stdio MCP tool references, with exact inputs,
  output contracts, errors, safety bounds, and clearly labeled synthetic examples.
  README interface sections now link to these references. Live reads remain unsupported.
- Manual CLI/MCP capability inventory and shared bounded process suite, with
  separate passed, failed, blocked, and not-applicable results. Required missing
  live reads prevent full success. A manual-only `windows-latest` workflow still
  selects Visual Studio 2022 Build Tools. An off-by-default test feature isolates
  native session entries. Offline harness checks cover safe reports and a
  per-submission Actions secret lifecycle. Live credential input and account
  requests remain disabled; no successful live coverage is claimed.
- Non-root Rust development container that installs the selected toolchain,
  portable VS Code tasks and settings, focused Copilot App tasks, GitHub Actions
  Copilot setup, and pinned rumdl Markdown style and offline local-link checks.
- Monthly Cargo and GitHub Actions dependency updates with separate security
  groups, confirmed maintainer and code-owner guidance, and contributor setup
  and maintenance instructions. No automatic merging or repository rules change.
- Chore and Documentation issue forms with purpose-specific fields and safe-data
  guidance. The existing Implementation form now covers feature requests and
  roadmap work. Contributor guidance explains how to select all four forms.
- Extracted-package verification for fixed Linux x86_64, Windows x86_64, and
  macOS Apple Silicon runners. Checks cover SHA-256, exact archive contents,
  runtime-independent startup, offline CLI/MCP failures, local stdio discovery,
  and isolated synthetic client registration. Rejection tests cover damaged
  packages and wrong package contracts. No live credentials or clients are used.
- Packaged native installation guide with checksum and extraction commands,
  OS/runtime and trust limits, troubleshooting, and explicit v1 release blockers.
- Resource-first `mcp start` and `mcp install` commands. Start launches the
  sibling native MCP server with inherited stdio and exit status. Install
  supports user-level VS Code Copilot Global, Claude Code, and Copilot CLI
  settings with preview, confirmation, safe JSON merging, and offline tests.
- Local stdio MCP server using rmcp 3.8.0 with seven read-only tools, strict
  schemas, explicit bounded book continuation, safe typed core error messages,
  and process tests for protocol discovery and tool calls. Live reads remain
  unsupported and return explicit tool errors.
- Resource-first `character list/show`, `campaign list/show`, and
  `book list/outline/read` CLI adapter with offline process tests, table/JSON/TSV
  list output, header control, preserved structured JSON, safe typed diagnostics,
  and stable 0/1/2 exits. Production reads fail explicitly as unsupported.
- Runtime Bash, Elvish, Fish, PowerShell, and Zsh completions generated from the
  actual clap command definition without credential or account access.
- Shared versioned section continuation tokens with a 16 KiB encoded cap,
  strict URL-safe parsing, redacted failures, and synthetic cross-process tests.
  Tokens do not grant access; reads retain the 4,096-byte UTF-8 text bound and
  fresh access/source checks. No automatic continuation or content cache is added.
- Offline core book outlines with a 1,000-entry cap and selected-section reads
  with a 4,096-byte UTF-8 text cap, fresh supplied access checks, source URLs,
  opaque revision-bound cursors, explicit completion, typed redacted failures,
  synthetic tests, and a runnable navigation example. No live book reads,
  automatic continuation, content cache, or CLI/MCP exposure is added.
- GitHub Copilot App tasks and a PowerShell Cargo wrapper that load Visual Studio
  2022 Build Tools on Windows for run, build, test, lint, and documentation checks.
- Offline core character and campaign records with unknown JSON preservation,
  opaque identifiers, campaign associations, optional HTTPS source metadata,
  duplicate conflict checks, bounded complete pagination, typed failures, and
  synthetic tests. Live reads, endpoints, and CLI/MCP operations remain blocked.
- Offline core library metadata assembly with explicit purchase and sharing
  evidence, source URLs, duplicate conflict checks, bounded complete pagination,
  redacted typed failures, and synthetic tests. No cached grants, live discovery,
  account reads, or CLI/MCP book interfaces are enabled.
- Shared protected local session operations with explicit Windows Credential
  Manager, macOS login Keychain, and Linux Secret Service backends, bounded opaque
  secrets, redacted typed failures, and synthetic store tests. No plaintext fallback
  or product credential import is available.
- Resource-first `auth status` and `auth logout` commands for local presence and
  removal only. `auth login` reports the browser-handoff blocker without collecting
  secrets or starting a browser. Stored sessions remain unverified; upstream
  exchange, expiry, refresh, revocation, and MCP session use remain unavailable.
- Offline-tested async core HTTP boundary with explicit HTTPS origins, GET-only
  typed JSON reads, bounded time, size, concurrency, retries, Retry-After handling,
  redirect rejection, sensitive session headers, and redacted typed error causes.
  No D&D Beyond hosts or endpoints are enabled, and neither binary sends requests.
- Three-crate Rust foundation with help, version output, and offline process tests.
- Development and agent guidance, architecture, authentication design, and roadmap.
- CI checks, issue and PR templates, and native binary packaging.
- Dated access feasibility assessment with official sources, per-read permission
  questions, browser-login limits, and a blocked live integration gate.
- Personal interactive lookup scope and a browser-handoff comparison based on
  native OAuth and cookie documentation.
- Ignored, authentication-only developer probe for a manual Cobalt session
  exchange, with hidden input, bounded requests, redacted errors, and synthetic
  offline tests. One successful live exchange was reported by the user on
  2026-10-27; data reads, expiry, and automatic login remain untested.

### Changed

- Recorded the bounded validation comparison: absent-cookie HTTP 200 without a
  token, invented-cookie HTTP 401, and a separately approved real-cookie HTTP
  401. Both denials stopped their scopes. No authenticated session or protected
  save is established; rejected input is not guessed to be expired.
- Recorded user-reported non-secret Edge cookie metadata and the separately
  approved Windows synthetic-store results. All four isolated native assertions
  passed with cleanup. The full manual suite still returned exit 1, with five
  failed installation-harness cases and 47 blocked live cases. No real-cookie
  save, product session, or complete validation contract is established.
- Recorded the separately approved negative-authentication result: the
  absent-cookie request returned HTTP 200. The bounded check stopped before
  its second request and read no body. A successful status cannot establish
  service validation; response meaning remains unverified.
- A second, separately approved absent-cookie JSON-shape observation returned
  HTTP 200 with no nonempty `token` and a positive `ttl` field. Only status and
  field-presence Booleans were reported; the bounded owned body was zeroizing.
  No authenticated-session or token-lifetime claim is made.
- Recorded pinned public cookie-selection and token-exchange source evidence.
  Mocked cookie callbacks and a token-field check do not establish the production
  cookie attributes or upstream negative-authentication behavior. Documented
  that generic credential storage does not itself authenticate an application
  peer. No live authentication request or credential-store access was performed.
- Refined the browser-login contract checkpoint with the remaining launch-proof,
  cookie-profile, and shared-core decisions. Recorded the Windows 2,560-byte
  credential-blob limit and the existing HTTP cookie name/value bound.
  A session-only synthetic test confirmed pipe metadata on standard handles;
  reviewed Chromium named-pipe launch behavior does not prove extension identity.
  All three gates remain blocked. Protocols, synthetic fixtures, product login,
  and credential access are unchanged.
- Record the Windows-first browser-handoff gate review, including OS-reported
  pipe peer IDs, safe Rust transport candidates, and the limits of parent-process
  checks. Chrome and Edge remain targets, not verified product support.
  Login stays blocked pending authenticated launch, cookie-profile evidence,
  and the shared core session contract. No runtime behavior changes.
- Record the existing-browser review for #29 and the later user-approved
  extension evaluation. Document cookie-permission limits, a native-messaging
  candidate with 14 session-only synthetic tests, the user-reported successful
  synthetic Edge handoff, requested Chrome support and fallback, and remaining
  browser and account checks. Keep login blocked and #30 lifecycle work separate.
  No product browser, credential, or network behavior changes.
- Update repository AI guidance to reflect local auth and MCP features while
  keeping live reads unsupported. Separate operational instructions from
  authoring rules and check commands, paths, and CI details against the repository.
- Remove the blanket upstream-permission development stop gate at the
  maintainer's request. Keep dated evidence, access-control protections, technical
  contract requirements, and separate approval for live account tests and
  credential access. Unsupported operations still fail; diagnostics now state
  the missing technical implementation instead of the removed gate.

### Fixed

- Reset accepted synthetic authentication mock sockets to blocking mode before
  applying I/O timeouts. This prevents inherited nonblocking mode from causing
  Windows `WouldBlock` failures in offline exchange checks.
- Documentation checker tests now return success after all assertions pass,
  instead of returning the last expected rejection's exit code to GitHub Actions.
- Use the Visual Studio 2022 Cargo wrapper in contributor command examples on
  Windows.
- Use PowerShell 7 and the existing Visual Studio 2022 Build Tools wrapper for
  Windows Rust CI, consistent with native release packaging.
- Correct stale CLI help, README, and changelog statements about local MCP
  serving. Browser login and live reads remain unimplemented.
- Keep Visual Studio setup messages out of the Cargo wrapper's success stream
  so in-process callers can parse metadata.
- Stop before starting an HTTP attempt when a queued permit becomes available
  after the total deadline. Accept case-insensitive identity content encoding.

Account login and live D&D Beyond reads are not available. Local stdio MCP
serving is available, but its live read tools return explicit unsupported errors.
