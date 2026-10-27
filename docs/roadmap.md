# Implementation roadmap

Both binaries provide help and version output. The CLI also has blocked login
and local auth status/logout. Protected-store operations have synthetic offline
tests; no product credential-creation flow exists. The core HTTP boundary has
offline policy tests, but no live endpoints or adapter wiring. Authenticated operations
below remain pending. The first feasibility assessment is documented, but it does not
open the live integration gate. Each issue includes acceptance criteria,
dependencies, tests, and documentation requirements.

## Implementation issues

| Issue | Scope | Prerequisites |
|-------|-------|---------------|
| [#1](https://github.com/PlagueHO/beyond-gate/issues/1) | Permitted access and technical feasibility | None; assessment recorded; live gate remains blocked |
| [#2](https://github.com/PlagueHO/beyond-gate/issues/2) | Typed HTTP client and bounded request policy; offline boundary implemented | #1; approved hosts and endpoint contracts still unresolved |
| [#3](https://github.com/PlagueHO/beyond-gate/issues/3) | Simple human login and protected sessions; local storage/status/logout implemented, browser login blocked | #1, #2 |
| [#4](https://github.com/PlagueHO/beyond-gate/issues/4) | Accessible book library discovery | #1, #2, #3 |
| [#5](https://github.com/PlagueHO/beyond-gate/issues/5) | Authorized character and campaign reads | #1, #2, #3 |
| [#6](https://github.com/PlagueHO/beyond-gate/issues/6) | Book outlines and bounded section reads | #4 |
| [#7](https://github.com/PlagueHO/beyond-gate/issues/7) | Resource-first CLI, output formats, completions | #5, #6 |
| [#8](https://github.com/PlagueHO/beyond-gate/issues/8) | Local stdio MCP and read parity | #5, #6; coordinate with #7 |
| [#9](https://github.com/PlagueHO/beyond-gate/issues/9) | Local MCP start and install | #7, #8 |
| [#10](https://github.com/PlagueHO/beyond-gate/issues/10) | Native packages and first-run usability | #9 and its prerequisites |

Implement the HTTP boundary before full session exchange. Simple login remains
an early required milestone, not an optional fallback. Library discovery and
character/campaign reads can proceed in parallel after session handling.
The adapters can follow each completed core read without waiting for all reads.

The feasibility work is a stop gate, not permission to bypass a restriction.
If a path is not permitted or viable, record that result and revise scope before
dependent live integration. Synthetic offline tests can still be developed.

## Current gate result

**Reviewed: 2026-10-27. No permitted integration path established.** The
[operation assessment](architecture.md#required-reads-and-decisions) records
unresolved permission for every required read. The
[browser-login handoff](authentication.md#browser-handoff-assessment) remains
unproven. This is not a permanent upstream refusal or approval for experiments.

Live work in #2 through #6 remains blocked, including endpoint probes, session
exchange, browser capture, and purchased or shared content reads. Dependent live
CLI and MCP operations in #7 through #9 and a usable release in #10 cannot pass
this gate either. Offline adapter and policy tests may proceed without an account,
but their models must not be presented as verified upstream contracts.
The [HTTP boundary](architecture.md#http-boundary) implements offline request
policy with no built-in upstream hosts or resource endpoints. This does not
complete the approved-endpoint evidence requirement in #2 or open the live gate.
The [interactive-use evaluation](architecture.md#interactive-use-and-evaluation-assumption)
assumes user-triggered lookups, not background collection. Technical research and
offline checks continue under that assumption; it does not establish a native
login handoff or change the upstream evidence.

Reopen the gate only after the
[upstream questions](architecture.md#permission-questions-and-next-gate) have
supported answers. Use synthetic offline proofs before seeking separate explicit
approval for a bounded live experiment. The initial official-source review found
no API schema. A later user-requested
[developer probe](authentication.md#opt-in-session-exchange-probe) adds synthetic
tests and one ignored manual token-exchange test based on community evidence.
The user reported one successful manual exchange on 2026-10-27. This narrow
technical result does not add product operations, verify data reads or expiry,
or approve dependent live integration.

If permission cannot be obtained, the maintainer must choose a documented scope
change instead of an access bypass. Offline-only work or a separately assessed
local-data import are proposals, not approved replacements for the current goals.

## Release boundary

Issue #3 remains partial. `auth login` fails with the browser-handoff blocker.
`auth status` can establish only `absent` or `stored-unverified`, not upstream
usability or expiry. `auth logout` removes the local entry but cannot revoke
upstream sessions. Native-store runtime checks need separate approval; automated
tests use synthetic stores. Exchange, expiry, bounded refresh, browser cancellation,
and real CLI/MCP session parity remain deferred.

v1 is local-only and read-only. It needs simple authentication, consistent CLI/MCP
results, source URLs, bounded book reads, and clear errors before a usable release.
There is no requirement for a cloud service, Python, Node.js, or crate publication.
