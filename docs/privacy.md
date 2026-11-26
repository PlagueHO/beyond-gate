# Privacy and local data

**Beyond Gate runs locally, but authenticated reads contact D&D Beyond.**
Its CLI and stdio MCP server use the same native core. There is no project-hosted
account service, remote MCP listener, product telemetry, or automatic content cache.
An MCP client can retain results or send them to a model provider. Local stdio
does not guarantee that the client keeps account data on your computer.

This page describes the current implementation, not every OS, browser, or client.
See [Support and compatibility](support.md) for tested combinations and gaps.
See [Authentication](authentication.md) for safe input instructions.

## Session input and lifetime

For resource reads and validation, the CLI selects an explicit hidden prompt first,
then the presence of `BEYOND_GATE_COBALT_SESSION`, then the saved entry.
The MCP server does not prompt;
it selects environment input or the saved entry at startup. An empty environment
value is present, not absent. Empty, malformed, oversized, non-Unicode, or
rejected explicit input fails without falling back to saved credentials.
Supply only the cookie value, at most 2,552 bytes, not an assignment or header.

| Data | Location and lifetime | Removal and limits |
|------|-----------------------|--------------------|
| Environment session input | The server snapshots its source once. The core uses its selected copy in process memory only. | Process logout clears that session owner's state. It cannot erase the parent environment, another process, or OS copies. Restart can supply the same value again. |
| Hidden CLI input | One explicit command; stdin/stderr must be terminals. Input is not echoed. | Submit with Ctrl+D or cancel with Escape/Ctrl+C. Terminal mode is restored on normal exits. Forced termination and terminal recording are outside this guarantee. |
| Saved cookie | Explicit login/import validates before writing a versioned cookie envelope to one protected OS entry. | Replaced only by explicit save; retained across restarts until removal or replacement. No automatic expiry deletion is claimed. |
| Derived bearer and validation evidence | Shared within one process-local session lifecycle, with bounded expiry/renewal. | Not saved to the credential store or returned in results. Logout clears the owner's cache. |
| Resource responses | Bounded in-memory reads, then CLI output or MCP results. | No product book-content cache or account snapshot file. Shell redirection and client retention are outside the core's cleanup. |
| One-use browser pairing state | Pending interactive login, extension/native messages, and Windows local IPC. | Bounded deadlines, one-use authorization, cancellation, and connection cleanup. This is not a reusable account credential. |

The core uses secret-clearing containers for selected secret buffers. This does
not prove erasure of every allocation, environment copy, swap page, crash dump,
backup, terminal recording, or browser copy. Do not capture session or pairing
screens. Use the normal OS protections for your account.

Only `auth login` and explicit `auth import --prompt` / `--env` request protected
save. Login without options keeps Windows browser pairing. `auth login --prompt`
uses hidden input; `auth login --from-env` reads the fixed
`BEYOND_GATE_COBALT_SESSION` variable. These flags take no value and cannot be
combined. Failed selected input never falls back to another source or browser
pairing. Existing import commands remain compatible.
Ordinary environment or global `--auth-prompt` use never saves implicitly and never
opens the native store. It needs no browser, desktop, D-Bus, or Secret Service.
MCP never asks for a password or cookie through a tool or protocol-stream prompt.
Global `--auth-prompt` is not accepted for login or import.
Direct-value arguments such as `--auth-key` remain excluded: a value can appear
in process listings and shell history even when expanded from a shell variable.

Protected storage uses the fixed application identity `beyond-gate` /
`local-session`. Windows uses Credential Manager with local-machine persistence,
not roaming persistence. Linux uses a user D-Bus Secret Service provider.
The macOS backend uses the login Keychain; that backend does not establish native
macOS production-read support. Locked, denied, or unavailable storage remains an
explicit error. No plaintext fallback exists. See the
[store requirements](installation.md#system-services-and-trust-limits).

## Status, logout, and uncertain saves

`auth status` checks the selected source without contacting D&D Beyond.
Valid local environment syntax reports `source=environment` and
`provided-unverified`; saved presence reports `stored-unverified` or `absent`.
Neither proves usability, expiry, or resource access.
`auth status --stored` checks only the protected saved entry.
`auth validate` performs a fresh service check; it does not grant access to
every resource.

`auth logout` clears the selected process session or deletes the selected saved
entry. Environment selection preserves saved credentials.
`auth logout --stored` explicitly removes the saved entry.
Neither command revokes the upstream session, signs out the browser, clears
another process, or changes the parent's environment.
Remove caller-supplied environment input through that caller's normal controls
and restart its MCP server. Each new CLI process can inherit the value again.

A write failure or lost final acknowledgement can report `save_indeterminate`.
Storage may have changed. Do not replay, automatically delete, or assume rollback.
Check saved presence, then decide on a new explicit validation, save, or removal.
Presence alone is not proof that the saved value works.

## Environment input and MCP clients

Environment variables can pass to child processes. Supply secrets only through
the execution host's controlled environment or a client secret-input mechanism
that its current documentation supports. Do not place literal values in command
arguments, client JSON, shell profiles, scripts, chat, logs, or examples.
Do not assume that every client inherits arbitrary variables.

The [MCP startup example](mcp.md#startup-and-lifecycle) uses a
`${BEYOND_GATE_COBALT_SESSION}` reference, not a literal value.
The current [official Copilot reference](https://code.visualstudio.com/docs/agents/reference/mcp-configuration#_environment-variable-references),
checked on 2026-11-21, supports `${NAME}` in portable configuration.
That format does not support editor-only `${input:...}` prompts, `envFile`,
or a VS Code SecretStorage bridge. Do not copy an editor-only secret prompt
into portable Copilot settings.
Other clients have their own mechanisms and retention rules; their native
acceptance must be checked separately.

The installer writes a credential-free server entry only after confirmation.
It does not install a secret-input mechanism or verify live account access.
Before enabling tools, review the client's storage, model-provider destinations,
tool approvals, and data retention. Returned character/campaign fields can
contain private account data. Treat returned text and HTML as untrusted data,
not instructions or trusted executable markup.

## Network destinations

Production resource operations use fixed HTTPS origins and checked paths, not
caller-selected hosts. Credential exchange and resource requests have bounded
deadlines and response sizes. The shared transport disables system proxy use and
does not follow redirects; it does not provide an arbitrary proxy configuration.
The network still depends on normal OS DNS and TLS trust.

| Destination | Purpose |
|-------------|---------|
| `auth-service.dndbeyond.com` | Fixed cookie-to-bearer exchange at `/v1/cobalt-token`. |
| `www.dndbeyond.com` | Public configuration and checked website pages for catalog, record, and bounded book reads. Website session requests use the selected cookie. |
| `api.dndbeyond.com` | Web-library catalog and current-account ownership evidence. |
| `character-service.dndbeyond.com` | Bounded character/campaign and selected catalog/definition reads. |
| `monster-service.dndbeyond.com` | Bounded JSON monster search and selected access/detail checks. |

The shared core chooses cookie or bearer authentication for the fixed operation.
It does not expose those credentials in CLI/MCP results. Metadata and source URLs
are not access grants. See the [API call catalog](api-catalog.md) and
[HTTP boundary](architecture.md#http-boundary) for route-specific limits.
Public catalog requests can still disclose your network address to the service.
By default, after a valid monster detail grant, one additional public `/api/config/json`
request supplies readable lookup labels. This request carries no cookie or
bearer header. Lookup names are returned separately from unchanged raw detail.
No account or source ownership is inferred, and no lookup cache is written.
Explicit `monster show --raw` or MCP `raw: true` skips this lookup request.
It does not bypass the authenticated detail request or its access checks.
Authentication and reads expose necessary session/request data to D&D Beyond.
Its handling is governed by upstream policies, not this project's local cleanup.

The extension does not exchange tokens or fetch resource pages. It passes the
selected cookie through native messaging and local IPC to the pending CLI.
MCP communication is local stdio; the project provides no remote server.
Downloading packages or opening documentation uses GitHub or the documentation
host through your chosen tools. Those downloads are separate from product reads.

## Manual browser extension

The extension is installed manually in a regular Windows Chrome or Edge profile.
Its [manifest](../beyond-gate-browser-extension/manifest.json) requests `cookies`
and `nativeMessaging`, with host permissions for `https://dndbeyond.com/*` and
`https://www.dndbeyond.com/*`. Incognito is disallowed.

These host permissions are broader than the single `CobaltSession` cookie name.
The [Chrome cookie API](https://developer.chrome.com/docs/extensions/reference/api/cookies)
requires both cookie and host permissions. The current implementation makes one
bounded, exact cookie query only after explicit consent and pending-login
authorization. It rejects missing, ambiguous, wrong-scope, expired, or malformed
candidates. It does not query other profiles, read passwords, add content scripts,
log cookies, or store them in extension storage.

The data path is browser -> native host -> authorized pending CLI -> shared core
service validation -> protected save -> final acknowledgement.
The native host relays the handoff; it does not implement its own authentication
or persistence. Consent covers reading and saving the selected session.
A received message alone is not validated-login or saved-session success.
Installed code, configuration, and process memory remain trusted; pairing does
not prove every browser-launch property. See the
[accepted threat boundary](browser-handoff.md#accepted-windows-pairing-boundary).

Use the extension, native host, CLI, and MCP server from one reviewed package.
Their package versions must match; matching numbers alone do not prove protocol
compatibility. Updates are manual. Stop owned processes before replacing files,
then check the documented registration path and extension identity.
Do not force a mismatch by changing manifest keys or protocol fields.

There is no Chrome Web Store or Edge Add-ons publication, Store review, or
automatic update mechanism. Follow [Windows setup](authentication.md#use-the-browser-extension)
and [Edge's manual installation guidance](https://learn.microsoft.com/en-us/microsoft-edge/extensions/getting-started/extension-sideloading).
See [tested browser limits](support.md#platforms-and-tested-scope).

## Settings, logs, and removal

| Item | Current behavior and removal |
|------|------------------------------|
| Protected saved entry | Use explicit `auth logout --stored` before deleting the tools if removal is wanted. |
| MCP client settings | Confirmed installation writes the absolute server path. Stop the server and remove only its `beyond-gate` entry; preserve unrelated settings. |
| Browser native-host manifest and user registration | Use matching `auth uninstall` with preview/confirmation before deleting files. Other browsers and machine-wide entries are preserved; Edge can still find a Chrome fallback. |
| Unpacked extension and package files | Remove the extension through browser controls, then remove owned files and any PATH entry you added. This does not delete the browser's upstream cookie. |
| CLI/MCP output | CLI results use stdout and diagnostics use stderr; serving MCP reserves stdout for protocol. Caller-created output files and client transcripts need separate removal. |

The product has no file-log sink, telemetry collector, usage-report upload,
automatic updater, or persistent book-content cache. Secret-bearing dependency
errors are kept behind redacted error boundaries. This does not mean all result
data or setup paths are anonymous. Review diagnostics before reporting them.

Development probes and synthetic test harnesses can produce explicitly requested
reports and temporary artifacts. They are not enabled product logging.
OS/browser diagnostic logs, backups, crash reports, terminal capture, and
client/model-provider retention are outside Beyond Gate's control.
Removing the tools does not remove those copies or revoke upstream credentials.
See [installation removal](installation.md#additional-setup-details) and
[safe reporting](support.md#report-a-problem).

## Implementation evidence

The [source selector](../src/beyond-gate-core/src/auth/input.rs),
[session owner](../src/beyond-gate-core/src/auth/session.rs),
[protected store](../src/beyond-gate-core/src/auth/mod.rs), and
[HTTP client](../src/beyond-gate-core/src/http/mod.rs) define these boundaries.
The [extension flow](../beyond-gate-browser-extension/flow.mjs) implements consent
and handoff without extension storage. The
[shared input checks](testing.md#shared-process-only-authentication-checks) use
synthetic data; native acceptance remains limited to the recorded cases.
