# Implementation roadmap

Both binaries provide help and version output. The CLI also has blocked login,
local auth status/logout, and shell completions. Resource command parsing and
formatting have offline process tests; live reads fail explicitly.
Protected-store operations have synthetic offline
tests; no product credential-creation flow exists. The core HTTP boundary has
offline policy tests, but no live endpoints or adapter wiring. Authenticated operations
below remain pending. The core can also assemble explicitly supplied offline
library metadata with context-specific evidence and bounded pagination. This is
not live discovery. The core also validates supplied book outlines and reads
bounded sections with fresh supplied access checks. The first feasibility
assessment remains historical evidence, not a development stop gate.
Each issue includes acceptance criteria,
dependencies, tests, and documentation requirements.

## Implementation issues

| Issue | Scope | Prerequisites |
|-------|-------|---------------|
| [#1](https://github.com/PlagueHO/beyond-gate/issues/1) | Access and technical feasibility assessment | None; assessment recorded; blanket permission stop gate removed |
| [#2](https://github.com/PlagueHO/beyond-gate/issues/2) | Typed HTTP client and bounded request policy; offline boundary implemented | #1; approved hosts and endpoint contracts still unresolved |
| [#3](https://github.com/PlagueHO/beyond-gate/issues/3) | Simple human login and protected sessions; local storage/status/logout implemented, browser login blocked | #1, #2 |
| [#4](https://github.com/PlagueHO/beyond-gate/issues/4) | Offline library metadata, evidence, and page assembly implemented; live discovery blocked | #1, #2, #3 |
| [#5](https://github.com/PlagueHO/beyond-gate/issues/5) | Authorized character and campaign reads | #1, #2, #3 |
| [#6](https://github.com/PlagueHO/beyond-gate/issues/6) | Offline outlines and bounded section reads implemented; live reads not implemented | #4 |
| [#7](https://github.com/PlagueHO/beyond-gate/issues/7) | Offline CLI adapter, output formats, bounded wire continuation, and completions implemented; live reads blocked | #5, #6 |
| [#8](https://github.com/PlagueHO/beyond-gate/issues/8) | Local stdio MCP and read parity | #5, #6; coordinate with #7 |
| [#9](https://github.com/PlagueHO/beyond-gate/issues/9) | Local MCP start and install | #7, #8 |
| [#10](https://github.com/PlagueHO/beyond-gate/issues/10) | Native packages and first-run usability | #9 and its prerequisites |

Authentication follow-up work remains open:

| Issue | Scope | Current dependency |
|-------|-------|--------------------|
| [#29](https://github.com/PlagueHO/beyond-gate/issues/29) | Native host, browser registration, pending CLI login and protected handoff | [Contract checkpoint](browser-handoff.md) proposed but blocked on authenticated IPC, the cookie profile, and the shared core boundary. #46 blocks end-to-end completion. |
| [#30](https://github.com/PlagueHO/beyond-gate/issues/30) | Shared authenticated session validation and lifecycle for CLI and MCP | Agree on the checkpoint's core boundary with #29; opaque local presence is not upstream validation. No reciprocal completion dependency. |
| [#46](https://github.com/PlagueHO/beyond-gate/issues/46) | Manually installed Chrome/Edge extension and real synthetic browser CI | Synthetic feasibility can start independently. Product integration needs the accepted #29 contracts, not completion of all of #29. |

The user reported one successful
[synthetic Edge handoff](authentication.md#edge-synthetic-browser-test).
Chrome, authenticated pending-CLI binding, account handoff, protected save, and
the requested separate environment fallback remain unverified. The checkpoint's
synthetic JSON cases are not yet executed by Rust or browser tests.

Implement the HTTP boundary before full session exchange. Simple login remains
an early required milestone, not an optional fallback. Library discovery and
character/campaign reads can proceed in parallel after session handling.
The adapters can follow each completed core read without waiting for all reads.

Use feasibility evidence to identify actual technical and access limits, not as
a blanket development stop gate. Stop on access denial. Never bypass a
restriction or describe synthetic offline inputs as a verified upstream contract.

## Development policy and technical status

**The blanket permission stop gate is removed at the maintainer's request.**
The 2026-10-27 [assessment](architecture.md#feasibility-assessment) remains dated
evidence. Its earlier project-wide stop decision is superseded. This change
does not establish upstream approval, resolve terms questions, or grant access.

Development can proceed within the read-only, personal, user-triggered scope.
Keep access checks, origin restrictions, bounded requests, protected credentials,
and the rules against crawling or bulk extraction. Live account tests and
credential access still need separate explicit approval. Define selected hosts,
operations, request budgets, stop conditions, and cleanup before such tests.

The [browser-login handoff](authentication.md#browser-handoff-assessment) remains
unproven. Live resource endpoints, schemas, entitlement checks, and product
session handling are not implemented. Those technical limits still prevent
working live reads in #4 through #6, dependent adapters, and a usable release.
Offline work does not resolve them.
The [HTTP boundary](architecture.md#http-boundary) implements offline request
policy with no built-in upstream hosts or resource endpoints.
The [interactive-use evaluation](architecture.md#interactive-use-and-evaluation-assumption)
assumes user-triggered lookups, not background collection.

Track missing contracts and test approval in the
[integration evidence and live-test guidance](architecture.md#integration-evidence-and-live-tests).
The initial official-source review found no API schema. A later user-requested
[developer probe](authentication.md#opt-in-session-exchange-probe) adds synthetic
tests and one ignored manual token-exchange test based on community evidence.
The user reported one successful manual exchange on 2026-10-27. This narrow
technical result does not add product operations, verify data reads or expiry,
or authorize another live account test. Offline-only work is the approved scope
for #6 in this change, not a claim that the live-read goal has been fulfilled.

## Release boundary

Issue #4 remains partial. The [offline library boundary](architecture.md#offline-library-boundary)
validates supplied metadata, access evidence, duplicate handling, and bounded
complete pagination. It retains no prior grants or persistent cache.
`discover_library` explicitly reports unsupported discovery; the CLI's `book list`
reports that failure instead of returning live books. Runtime endpoints, actual metadata and entitlement
schemas, source-URL rules, upstream pagination, usable sessions, and approved live
validation remain unresolved. Offline tests cannot prove upstream freshness,
detect unreported revocation, or establish schema-change compatibility.

Issue #5 remains offline-only. The core validates explicitly supplied character
and campaign records, preserves unknown JSON, and assembles bounded complete
pages. Live list and read functions explicitly report unsupported operations.
The CLI exposes their explicit unsupported failures and tests successful output
with synthetic core results. No upstream schema, access rule, or endpoint is assumed.

Issue #6 remains offline-only. The [book boundary](architecture.md#offline-book-boundary)
returns validated outlines with at most 1,000 raw entries and selected-section
chunks with at most 4,096 UTF-8 text bytes. Every call checks fresh supplied
access before loading. Opaque in-process cursors bind book, section, source
revision, URL, and byte position. Shared `bg1.` wire tokens allow explicit
cross-process continuation with a 16 KiB encoded cap. The CLI tests bounded
output and continuation offline, but its production commands fail as unsupported.
No live book contract, MCP read, automatic continuation, or content cache is
added. Real access freshness and
source revisions still depend on a future source implementation.

Issue #7 remains partial for live-read criteria. Its CLI adapter defines
`character list/show`, `campaign list/show`, and `book list/outline/read`.
Lists/outlines support table, JSON, and TSV with header control. Show/read
preserve structured JSON. The CLI uses the actual clap definition to generate
Bash, Elvish, Fish, PowerShell, and Zsh completions. Offline process tests do not
make the core's unsupported live operations usable. There is no product fixture
input, cached content, or account snapshot.

Issue #8 now has a local rmcp stdio server and seven matching read tools. The
tools call the shared core functions and preserve their explicit unsupported
errors; they do not enable account reads. Tool schemas, read-only annotations,
bounded continuation input, and protocol process tests are implemented. The
current MCP output contract is documented alongside the CLI contract.

Issue #3 remains partial. `auth login` fails with the browser-handoff blocker.
`auth status` can establish only `absent` or `stored-unverified`, not upstream
usability or expiry. `auth logout` removes the local entry but cannot revoke
upstream sessions. Native-store runtime checks need separate approval; automated
tests use synthetic stores. Exchange, expiry, bounded refresh, browser cancellation,
and real CLI/MCP session parity remain deferred.

v1 is local-only and read-only. It needs simple authentication, consistent CLI/MCP
results, source URLs, bounded book reads, and clear errors before a usable release.
There is no requirement for a cloud service, Python, Node.js, or crate publication.

Issue #10 adds offline extracted-package verification and the
[native installation guide](installation.md). The package matrix is Linux x86_64,
Windows x86_64, and macOS Apple Silicon on fixed native CI runners. Packaging
checks hashes, archive contents, executable paths, startup, honest unsupported
errors, MCP protocol behavior, and isolated synthetic client registration.
Each platform result applies only after its packaging workflow succeeds.

This is not completion of #10 or proof of a usable v1. Missing-session and expiry
errors for live operations remain blocked, not replaced by unsupported errors.
Simple login, native credential-store checks, actual client discovery, and approved
live CLI/MCP checks for access, attribution, and book bounds remain release blockers.
See the guide's [release-readiness table](installation.md#release-readiness).
Maintainers must review version, lockfile, dated changelog, assets, evidence, and
the draft before publication. This issue authorizes no release tag or publication.
