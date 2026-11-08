# Implementation roadmap

The CLI and MCP binaries provide help and version output. The CLI has verified Windows
browser login, local auth status/logout, and shell completions. Resource command parsing and
formatting have offline process tests. Windows Web-library discovery has approved
native CLI/MCP parity. Windows book outline/read operations now use the approved
page-level contract, with synthetic process coverage. Native book adapter
acceptance passed for one selected chapter with matching outlines and initial
chunks. Live continuation and sharing remain unchecked.
Character and campaign reads now use fixed, bounded Windows operations through
the shared protected session. Synthetic process tests cover adapter parity;
live source compatibility remains unvalidated.
Issue #65 adds metadata-only Windows class/subclass list/show and two MCP tools.
Synthetic checks cover bounded discovery and adapter parity. Full-content access,
native compatibility, real sharing, and live global coverage remain unverified.
Protected-store operations have synthetic offline
tests; Windows login creates the protected entry after service validation.
The core HTTP read boundary has offline policy tests and fixed Web-library
routes through the shared protected session. The core can also assemble explicitly supplied offline
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
| [#3](https://github.com/PlagueHO/beyond-gate/issues/3) | Windows human login and local storage/status/logout implemented; reusable sessions and other-platform login remain deferred | #1, #2 |
| [#4](https://github.com/PlagueHO/beyond-gate/issues/4) | Offline library assembly implemented; #32 adds verified Windows Web-library discovery through both adapters | #1, #2, #3 |
| [#5](https://github.com/PlagueHO/beyond-gate/issues/5) | Authorized character and campaign reads | #1, #2, #3 |
| [#6](https://github.com/PlagueHO/beyond-gate/issues/6) | Offline outlines and bounded section reads; #33 adds Windows page-level source operations | #4 |
| [#7](https://github.com/PlagueHO/beyond-gate/issues/7) | CLI output, continuation, and completions; #32 adds Windows Web-library reads and #33 adds bounded book reads | #5, #6 |
| [#8](https://github.com/PlagueHO/beyond-gate/issues/8) | Local stdio MCP and read parity | #5, #6; coordinate with #7 |
| [#9](https://github.com/PlagueHO/beyond-gate/issues/9) | Local MCP start and install | #7, #8 |
| [#10](https://github.com/PlagueHO/beyond-gate/issues/10) | Native packages and first-run usability | #9 and its prerequisites |
| [#65](https://github.com/PlagueHO/beyond-gate/issues/65) | Class/subclass metadata-only list/show; no raw definitions or rules text | #2, #30, #7, #8; browser evidence and synthetic adapter checks, not native acceptance |

Authentication follow-up work remains open:

| Issue | Scope | Current dependency |
|-------|-------|--------------------|
| [#29](https://github.com/PlagueHO/beyond-gate/issues/29) | Native host, browser registration, pending CLI login and protected handoff | Windows implementation and #46 integration verified. Hosted CI and approved Chrome/Edge Dev login/save/restart/logout passed. Shared core contract agreed with #30. Code is on the feature branch, not merged. |
| [#30](https://github.com/PlagueHO/beyond-gate/issues/30) | Shared authenticated session validation and lifecycle for CLI and MCP | Windows fresh validation and bounded process-local reuse implemented locally with portable synthetic and CLI/MCP process checks. Uses the agreed #29 establishment contract. Local presence is not upstream validation; live reads remain separate. |
| [#52](https://github.com/PlagueHO/beyond-gate/issues/52) | Human-controlled Linux login and shared production session use | Reuse #30's portable core policy. Define Linux native-host and local-communication contracts; coordinate #34 and #46. Does not block #30's Windows work. |
| [#46](https://github.com/PlagueHO/beyond-gate/issues/46) | Manually installed Chrome/Edge extension and real synthetic browser CI | Windows product integration and separate browser CI passed against the accepted #29 contracts. Review and merge remain separate. |

The user reported one successful
[synthetic Edge handoff](authentication.md#edge-synthetic-browser-test).
The later [isolated browser suite](testing.md#isolated-browser-feasibility) passed
local Chrome and Edge cookie/native-host checks and owned cleanup. Rust tests
execute canonical messages and selected parsing/framing recipes. Full state
coverage and authenticated pending-CLI binding now have offline checks.
Both browsers passed five synthetic native/core completion cases.
Hosted-runner checks and approved real Chrome/Edge Dev handoff, protected save,
restart/status, and logout now passed.

The historical [contract-blocker milestone](browser-handoff.md#contract-blocker-decision)
records the earlier unresolved decisions. Later user decisions accepted Windows
pairing, the cookie profile, direct-service validation, and trusted installed
registration. The host, setup commands, and synthetic browser integration are
implemented. Account observations and credential-store tests still need separate
approval. The user agreed to the shared establishment boundary with #30;
shared lifecycle now has portable synthetic checks and Windows production entry
points under #30. Other-platform production login and session use remain deferred.

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

The [Windows browser-login handoff](testing.md#approved-windows-product-login-verification)
has approved product evidence. Live resource endpoints, schemas, entitlement
checks, and production resource adapters are not implemented. Shared Windows
session validation and reuse have synthetic coverage, not new live-account evidence.
Those technical limits still prevent
working live reads in #4 through #6, dependent adapters, and a usable release.
Offline work does not resolve them.
The [API survey](api-survey.md) records earlier approved character/campaign observations,
public Rules listing metadata, client-derived routes, and selected-book HTML structure.
The [shared API call catalog](api-catalog.md) indexes these calls with stable IDs,
authentication evidence, known contracts, limits, and gaps for reuse by other issues.
A separate native probe accepted source HTML with bearer-only authentication but stopped
at a chapter redirect. A user-requested repeat identified a marketplace or purchase target.
Native chapter reads, fresh access, and bounded library transport
remain unresolved.
Pinned MrPrimate importer and companion source identifies a separate mobile book-download
channel and candidate access-metadata routes. These were not live-tested and do not provide
a bounded section API. Whole-book download and content caching remain outside the design.
This evidence does not add production resource operations or approve bulk collection.
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
Issue #32 adds Windows Web-library discovery through public classification and
two bounded protected JSON routes, with matching CLI `book list` and MCP `book_list`.
It requires complete count equality and preserves separate purchase and
account-sharing evidence. Attribution permits a supplied validated marketplace
URL only when the book path is absent. It neither fetches those links nor grants
later content access. See [live discovery](architecture.md#live-library-discovery)
for caps and failures. Earlier native metadata and ownership checks failed.
The approved Web-catalog scope now handles configured ownership-only sources
without inventing book metadata. Fresh native CLI/MCP acceptance passed with
matching results: 169 books, 36 purchased and 133 unknown.
Real sharing observations remain unverified.
Offline tests cannot prove upstream freshness,
detect unreported revocation, or establish schema-change compatibility.

Issue #5 remains offline-only. The core validates explicitly supplied character
and campaign records, preserves unknown JSON, and assembles bounded complete
pages. Live list and read functions explicitly report unsupported operations.
The CLI exposes their explicit unsupported failures and tests successful output
with synthetic core results. No upstream schema, access rule, or endpoint is assumed.

Issue #6 supplied the offline validators. The [book boundary](architecture.md#offline-book-boundary)
returns validated outlines with at most 1,000 raw entries and selected-section
chunks with at most 4,096 UTF-8 text bytes. Every call checks fresh supplied
access before loading. Opaque in-process cursors bind book, section, source
revision, URL, and byte position. Shared `bg1.` wire tokens allow explicit
cross-process continuation with a 16 KiB encoded cap. The CLI tests bounded
output and continuation through the real shared source path against synthetic
local services. Issue #33 adds the user-approved
[page-level contract](architecture.md#live-book-reads) through both adapters.
Fresh positive library evidence and a checked canonical cookie page are required
for every call. Native book adapter acceptance passed for one selected chapter
with matching 79-entry outlines and 3,929 UTF-8 byte initial chunks.
Live continuation and sharing remain unchecked. No automatic continuation,
whole-book operation, or content cache is added.

Issue #7 provides `character list/show`, `campaign list/show`, and
`book list/outline/read`. Issue #65 adds `class list/show` and `subclass list/show`
for bounded metadata only. Character and campaign operations use fixed,
read-only Windows routes through the shared protected session. Lists and
outlines support table, JSON, and TSV; show/read preserve structured JSON.
Character and campaign show commands also support `--output` / `-o`, with
`--format` as an alias. There is no product fixture input, cached content, or
account snapshot.

Issue #8 provides a local rmcp stdio server. With issue #65, it has nine resource
read tools. The separate `auth_validate` tool brings the total to ten.
Character and campaign tools call the same bounded core operations as the CLI.
`class_list` and `class_show` expose class/subclass metadata only.
`campaign_list` can select the active collection with `active_only`; campaign
show returns the selected active roster. Tool schemas, read-only annotations,
bounded continuation input, and synthetic protocol process tests are
implemented. No live character or campaign acceptance has been run.

Issue #3 remains partial. Windows `auth login` now starts one-use pairing and
calls shared completion after bound submission. Windows setup and synthetic
browser integration are implemented. Real Chrome and Edge Dev login/save checks
and hosted CI passed. Other platforms retain unsupported login.
`auth status` can establish only `absent` or `stored-unverified`, not upstream
usability or expiry. `auth logout` removes the local entry but cannot revoke
upstream sessions. Native-store runtime checks need separate approval; automated
tests use synthetic stores. Windows establishment covers exchange failures,
expired bearer evidence, and browser cancellation.
Issue #30 adds fresh CLI/MCP validation and process-local bearer reuse with
one bounded exchange after bearer expiry. Cookie expiry and upstream revocation
remain unknown. Synthetic process checks prove validation parity, not live
resource reads. Linux support is tracked in #52.

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
Windows login and protected-store checks have approved product evidence.
Other-platform login, actual client discovery, real-sharing validation, and live
content access remain release blockers. Native Web-library metadata and evidence
parity passed separately from packaging.
See the guide's [release-readiness table](installation.md#release-readiness).
Maintainers must review version, lockfile, dated changelog, assets, evidence, and
the draft before publication. This issue authorizes no release tag or publication.
