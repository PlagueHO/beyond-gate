# Support and compatibility

**Beyond Gate is an unofficial, pre-1.0 project with best-effort support.**
There is no published release yet. Development builds and reviewed test packages
are not stable releases. No response time, fix date, deprecation period, or support
period is promised. Read [Privacy](privacy.md) before supplying a session or
connecting an MCP client.

## Platforms and tested scope

Build, package, synthetic test, native client, and live account results prove
different things. A passing build does not prove authentication or account access.
The [package matrix](installation.md#package-matrix) uses Windows Server 2022 x64,
Ubuntu 24.04 x64, and macOS 15 ARM64 as CI baselines. These are not minimum OS
versions. Other CPUs, older systems, emulation, and WSL need separate evidence.

| Path | Current implementation | Verified scope and limits |
|------|------------------------|---------------------------|
| Windows CLI/MCP | Shared process-only or saved authentication and bounded reads | Synthetic adapter checks cover shared input and read contracts. Native Web-library and one selected book chapter passed separate account checks. Other resource and sharing acceptance remains incomplete. |
| Linux CLI/MCP | Shared process-only or saved authentication and bounded reads | Portable synthetic checks cover source selection and reads. Retained Linux package startup and isolated synthetic Secret Service checks do not prove native account acceptance. |
| macOS | Native packages and protected-store backend exist | Production reads remain gated. Native authentication and desktop acceptance are deferred in #62. Common process-only design is a target, not verified macOS production use. |
| Windows Chrome login | Manual extension and native host | Chrome `156.0.8078.74` passed real pairing, validation, save, restart/status, and logout. |
| Windows Edge login | Manual extension and native host | Edge Dev `158.0.4357.0` passed those real login checks. Stable Edge has separate synthetic and setup evidence, not equivalent real-account acceptance. |
| VS Code client | Local stdio registration and tools | Stable VS Code `1.145.2` has retained registration, discovery, stop/restart, and removal evidence. A separate `1.143.1` check returned one approved `book_list` result. A fresh discovery fetch after restart and model-driven routing were not verified. |
| Copilot CLI client | Local stdio registration and tools | Copilot CLI `1.0.106` has retained offline discovery, restart, and removal evidence. It does not prove live reads or all current tool/client combinations. |
| Claude Code client | Registration path exists | Native client validation is deferred in #62. |

See [shared input tests](testing.md#shared-process-only-authentication-checks),
[approved Windows login](testing.md#approved-windows-product-login-verification),
and [desktop acceptance](desktop-acceptance.md#remaining-acceptance-matrix)
for candidate identities, exact results, and gaps. Older eight-tool discovery
results do not prove discovery of the current larger tool set.
See the [CLI](cli.md) and [MCP](mcp.md) references for each resource's current limits.

Windows and Linux terminal-only environment input needs no browser, desktop,
D-Bus, or Secret Service. Hidden CLI input also needs no native store.
Protected save has separate [OS service requirements](installation.md#system-services-and-trust-limits).
Browser login is optional and Windows-only. Linux/macOS browser login, Firefox,
Safari, private windows, and other browser combinations are not supported by this
extension. Organization policy can prevent manual installation or native messaging.
Do not bypass that policy or OS download protection.

## Interface compatibility and versioning

The project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Its `0.y.z` development interfaces can change. Do not assume that a pre-1.0 patch
or minor update preserves scripts, client settings, or stored formats.
There is no fixed warning period or promise to retain deprecated interfaces.

Compatibility review covers these public surfaces:

- CLI command names, flags, defaults, validation, exit codes, and stdout/stderr.
- JSON fields, TSV columns and escaping, output envelopes, and source attribution.
- MCP tool names, required/optional fields, strict schemas, and result/error envelopes.
- Explicit paging, book section bounds, and continuation input and output.
- Session-source labels, stored-session formats, and versioned browser handoff.

The [CLI output contract](cli.md#output-contracts) and [MCP reference](mcp.md)
define current behavior. Use structured output for scripts, not visual table layout.
Unknown MCP input fields are rejected. Resource failures use CLI exit code 1;
usage failures use 2. MCP tool errors use a result with `isError`, not a fabricated
empty success or an automatic server shutdown.

Unknown upstream fields can survive in selected raw records or HTML. Their
presence is not a stable project schema or an access grant. Upstream raw fields
can change without a Beyond Gate version change.
Continuation tokens are opaque positions, not credentials or durable bookmarks.
Do not assume that an old token remains valid after an update or source change.

Delivered changes belong in [CHANGELOG.md](../CHANGELOG.md). Breaking interface
changes need a clear description and recovery instructions there and in the
affected reference. A future 1.0 release must define its stable public contract;
this page does not declare that release. The existing
[release review](../MAINTAINERS.md#changelog-and-release-review) still applies.
Security fixes target only the latest published release once one exists.
See [Security policy](../SECURITY.md#supported-versions) for this release limit.

## Upstream failures and recovery

D&D Beyond can change endpoints, schemas, layouts, access rules, or service
availability without notice. Beyond Gate does not promise upstream API stability.
It rejects unsupported or invalid responses rather than inventing fields,
silently dropping invalid rows, or treating metadata as permission.

| Failure | Safe action |
|---------|-------------|
| Explicit input is empty or malformed | Correct the selected input locally. Do not expect fallback to a saved session. Keep the value out of reports. |
| Session is rejected | Stop the failed attempt. Use a new explicit authentication action after reviewing the service result. A browser sign-in alone does not prove native compatibility. |
| Service-issued bearer expired | The core has bounded renewal. A reported failure is not proof of reusable-cookie expiry. Do not add a blind retry loop. |
| Access denied or bot protection | Stop. Do not bypass the denial, infer access from source IDs, or switch to an unverified route. |
| Unsupported schema/layout or size limit | Stop the affected operation. Report the version, command/tool, and safe error category. Do not raise limits, scrape alternate pages, or submit raw content as a workaround. |
| Timeout or rate limit | Stop repeated attempts. Respect service limits. A later user-triggered attempt is not a promise that the route will work. |
| Protected store is locked/unavailable | Use normal OS unlock controls or explicit process-only input. There is no plaintext fallback. |
| Save is indeterminate | Do not assume unchanged storage or replay the transaction. Status can check presence but cannot prove service usability. Review before a new explicit save or removal. |
| Component mismatch or missing native host | Use matching files from one reviewed package. Preview registration changes. Do not edit protocol versions or extension identity to force a connection. |

Book reads keep fresh access checks, at most 1,000 outline entries, at most
4,096 UTF-8 section-text bytes, and explicit continuation bounded to 16 KiB.
The application does not follow continuation automatically or cache book content.
These limits do not grant extraction or redistribution rights.

## Report a problem

Use [repository issues](https://github.com/PlagueHO/beyond-gate/issues) for ordinary
bugs, questions, and proposals. Include both binary versions, OS/CPU, client or
browser version where relevant, command/tool name, expected result, safe error
category, and a synthetic reproduction if possible.
Read [contributor reporting guidance](../CONTRIBUTING.md#issues).

Do not include cookies, tokens, pairing keys, account identifiers, private URLs,
real settings, raw responses, or proprietary content. Use
[private vulnerability reporting](../SECURITY.md#report-a-vulnerability-privately)
for suspected security defects. Private reports must also exclude secrets.
The [sole maintainer](../MAINTAINERS.md) reviews reports on a best-effort basis.
A report does not promise a fix or release.

## Unofficial integration, attribution, and rights

Beyond Gate is not an official Wizards of the Coast or D&D Beyond integration.
Account authentication, purchased content, and sharing access do not establish
permission to automate access. The
[Wizards terms](https://company.wizards.com/en/legal/terms), reviewed on
2026-11-21, restrict unauthorized automated access and unauthorized third-party
connections. Applicable terms can change; review them before use.

The earlier [feasibility assessment](architecture.md#feasibility-assessment)
did not establish a permitted integration path. Removing the historical
development stop gate was not upstream approval. Whether a particular use or
distribution is permitted remains unresolved. This page is not legal advice
or a substitute for qualified advice or upstream authorization.

[MIT](../LICENSE) covers Beyond Gate code and documentation, not upstream
content, trademarks, or services. Keep its notice when distributing project
copies. Dependencies retain their own licenses; follow
[dependency license review](../CONTRIBUTING.md#dependency-license-review).
Source URLs attribute returned material but do not grant reuse rights.
Do not distribute proprietary content or real account fixtures.

The intended use is personal, user-triggered lookup. Remote MCP hosting,
multiuser services, account sharing, bulk extraction, content redistribution,
and write operations remain outside v1 scope. Manual extension distribution
does not imply Store review, upstream approval, or automatic updates.
