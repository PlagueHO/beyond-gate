# Authentication design

**Planned, not implemented.** Simple local login is a core requirement.
The feasibility issue must establish a permitted path before any live experiment.

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

## Session lifecycle

The planned session design must support a session cookie to short-lived token
exchange where the permitted upstream flow requires it. Determine actual cookie
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

Resolve these in the linked [roadmap](roadmap.md). Do not weaken protected storage
or bypass bot protection to make login appear successful.
