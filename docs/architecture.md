# Architecture

## Current boundary

The workspace contains three crates. The core exports one shared foundation status
message. Both adapters use clap for help and version output. The core also
provides an offline-tested async HTTP boundary and protected local session
operations. It can assemble library, character, and campaign metadata from
explicitly supplied offline pages. The CLI exposes blocked login plus local
status and logout.
There is no built-in runtime host list, resource endpoint, account model,
browser handoff, or MCP implementation. Neither adapter constructs an HTTP client
or sends upstream requests.

The workspace uses Rust 2024 and resolver 3. The
[Cargo edition guide](https://doc.rust-lang.org/edition-guide/rust-2024/cargo-resolver.html)
recommends this resolver for Rust 2024. The pinned toolchain is
[Rust 1.99.1](https://github.com/rust-lang/rust/releases/tag/1.99.1).
[clap 4.6.7](https://docs.rs/clap/4.6.7/clap/) handles the adapters.
The core HTTP boundary uses async
[reqwest 0.13.5](https://docs.rs/reqwest/0.13.5/reqwest/), Tokio, serde,
serde_json, and httpdate. The developer-only
[session probe](authentication.md#opt-in-session-exchange-probe) additionally uses
blocking reqwest and rpassword through development dependencies.
`Cargo.lock` records exact resolved versions, including development dependencies.
Local auth uses `keyring-core`, explicit target-specific credential-store crates,
and `zeroize`. These operations do not use the probe or the HTTP boundary.

## Intended boundaries

| Crate | Owns | Does not own |
|-------|------|--------------|
| `beyond-gate-core` | Authorized reads, session use, models, typed errors | CLI formatting or MCP protocol |
| `beyond-gate-cli` | Resource-first commands, output, human login flow | Duplicate endpoint or access logic |
| `beyond-gate-mcp` | Stdio lifecycle, bounded read tools, error mapping | Browser login or credentials from an agent |

Prefer direct functions and concrete types. Introduce a trait only when an actual
test boundary or multiple behaviors need it. Do not create empty model, service,
repository, or transport layers before the first operation needs them.

The HTTP boundary uses async reqwest with Tokio. Its synthetic tests do not
establish an upstream response contract. The separate developer probe uses
reqwest's blocking interface only.
The MCP implementation should use the
[official Rust SDK, rmcp](https://github.com/modelcontextprotocol/rust-sdk),
with stdio only. Select current compatible versions when those issues start.
Do not copy ArcadeJanitor's custom HTTP/WebSocket MCP layer.

## Local session boundary

The core `auth` module owns one entry: service `beyond-gate`, account
`local-session`. It selects Windows Credential Manager, macOS login Keychain, or
Linux Secret Service explicitly. Windows writes use local-machine persistence.
No global default, credential enumeration, browser-profile access, or plaintext
fallback is used. See [backend requirements](authentication.md#protected-store-selection-and-recovery).

`SessionStore` wraps a concrete keyring entry. The existing keyring store interface
provides the real test boundary, so no additional storage trait is needed.
Unit tests supply isolated in-memory entries, never native credentials.
`SessionSecret` owns bounded opaque bytes and clears its allocation on drop.
The format is not an upstream contract. No product command currently writes one.

Status reports `Absent` or `StoredUnverified`. An entry is not proof of usable,
expired, or rejected authentication. Only the native missing-entry result means
absence; other errors remain failures. Invalid local values are not silently
deleted. Logout removes the fixed entry without reading the secret and treats
an already missing entry as success. It does not revoke an upstream session.
There is no in-memory token cache or long-running authenticated adapter yet.

Errors retain typed native causes behind a redacted wrapper. Display, Debug, and
the standard error chain omit credential bytes and raw native messages. The safe
classification distinguishes invalid local data, denied or locked storage,
unavailable backends, ambiguous entries, unsupported platforms, and other native
failures where the backend enum permits it. Unknown failures do not become absence.

The CLI parses and presents these operations. Its login command returns an
explicit blocker before any storage, browser, input, or network activity.
Future MCP session use must call the same core operations and identity; no MCP
tools or credential input are implemented here. Native store calls are blocking,
so future async adapters must keep them off executor threads.

This is partial offline work for #3. Browser handoff, session exchange, upstream
expiry, refresh, rejection, and actual adapter session parity remain blocked.

## HTTP boundary

The core's `http` module supplies one concrete `HttpClient` and `get_json<T>`
operation. It accepts a caller-selected serde response type, not a resource model
invented from an unverified endpoint. No adapter exposes this operation yet.

`HttpClient::new` requires an explicit list of exact HTTPS origins. Each origin
contains a scheme, host, and effective port. An empty list denies all requests.
Paths, queries, fragments, and userinfo are not allowed in origin configuration.
Request URLs cannot contain userinfo or fragments. A host suffix or different
port does not match an allowed origin. The production client validates TLS
certificates and has no insecure HTTP option. Only private unit tests can use
literal loopback HTTP.

This boundary ships no D&D Beyond host list or resource endpoint. Supplying an
origin does not establish upstream permission. The [live gate](#feasibility-assessment)
remains blocked. Approved hosts and endpoint contracts are still needed to meet
the integration part of [#2](https://github.com/PlagueHO/beyond-gate/issues/2).

### Request limits

These are fixed local safety limits, not verified D&D Beyond limits.

| Concern | Limit |
|---------|-------|
| Method | GET only |
| Connect timeout | 5 seconds |
| Each attempt | 15 seconds, including body reads |
| Total operation | 30 seconds, including permit waiting and retry delays |
| Response body | 1 MiB |
| Concurrent operations | 4 across all clones of one client |
| Retry budget | 2 retries after the first attempt |
| Retry delay | 250 ms, then 500 ms; never shorter than a valid Retry-After |
| Redirects | None, including redirects to another allowed origin |
| Session input | At most 8 KiB for a bearer token or a cookie name plus value |

The client disables reqwest's automatic retries, proxies, automatic Referer, and
automatic decompression. It does not enable a cookie jar. It requests identity
encoding and rejects other content encodings. It checks a known oversized length
before reading the body, then checks each chunk against the remaining byte budget.
Responses without Content-Length have the same limit. It does not read error
bodies for diagnostics.

Only HTTP 429/502/503/504 and conservative transient transport failures are
retried. Transport timeouts and connection-refused/reset/aborted, broken-pipe,
unexpected-EOF, interrupted, or timed-out I/O causes can be transient. TLS and DNS
failures without such a cause are terminal. Other statuses, rejected sessions,
denied access, redirects, invalid destinations, oversized bodies, and malformed
responses are not retried. There is no POST or token-exchange retry path.

Retry-After accepts delay-seconds and HTTP-date. A past date permits the local
backoff. Missing or malformed values use the local backoff. When multiple valid
values arrive, the longest wait wins. Numeric overflow is treated as an excessive
wait, not as permission to retry early. A wait that cannot fit in the remaining
total budget returns the last typed failure immediately. Exhaustion retains its
status and attempt count; HTTP 429 remains a rate-limit failure.

Each logical operation holds a shared concurrency permit through its attempts
and retry waits. Queue waiting has the same total deadline. Cancellation drops
the active request and releases the permit. Cloning a client does not create
another retry worker or a separate permit pool.

### Session and error boundary

`SessionHeaders` accepts an in-memory bearer token or one cookie name and value.
It validates the input, marks its header sensitive, and omits credentials from
Debug. The caller supplies it per read, after session handling outside this
module. There are no global credential headers, browser access, password input,
session exchanges, refresh operations, or persistent secrets in this boundary.
An origin is checked before any credential is attached. Redirects never forward
credentials.

`HttpError` distinguishes invalid input, unapproved origins, client setup,
network failures, timeouts, rate limits, unusable sessions, denied access,
redirects, oversized bodies, malformed responses, and other upstream statuses.
HTTP 401 means the session was rejected and human login is needed. It is not
proof of a known expiry interval. HTTP 403 means access was denied; no bypass or
retry follows.

Failures expose their category, received status when relevant, attempt count,
and valid Retry-After delay. A `FailureCause` retains a concrete transport, JSON,
or Tokio deadline cause. Its safe accessors report cause type, timeout/connect/DNS
classification, and JSON line and column. Its Display, Debug, and error source
chain do not expose raw transport URLs, headers, private body values, or serde
messages. The boundary emits no request or response logs.

Synthetic loopback tests check typed output, exact byte limits, terminal failures,
redirect non-forwarding, timeout and retry budgets, Retry-After, shared concurrency,
queue waiting, cancellation, session validation, and error-chain redaction.
These tests establish policy behavior only, not a D&D Beyond data contract.

## Offline library boundary

The core `library` module implements partial offline work for
[#4](https://github.com/PlagueHO/beyond-gate/issues/4).
`assemble_library` validates explicitly supplied normalized pages and returns a
`LibrarySnapshot` only when the complete sequence succeeds. `discover_library`
always returns `LibraryError::UnsupportedDiscovery` before any HTTP client
construction, network request, or credential access. Neither adapter exposes a
book command or tool.

These are Beyond Gate's internal types, not an upstream response schema or an
import-file format. They do not establish that the caller's observations are
fresh, truthful, or authorized. No runtime host, discovery endpoint, JSON parser,
credential input, automatic page fetch, or persistent library is added.

### Metadata and access evidence

Each `BookRecord` contains an opaque identifier, a title, an HTTPS source URL,
and one explicit `AccessEvidence` observation. Identifiers, titles, and sharing
context identifiers must be nonempty, unpadded, and free of control characters.
Source URLs must have a host and cannot contain credentials, fragments, padding,
or control characters. A source URL is metadata, not permission to request its
origin. The normalized result returns the parsed URL with each book.

| Observation | Meaning |
|-------------|---------|
| `CatalogOnly` | The book is visible; entitlement is unknown. |
| `Purchased { state }` | Evidence about purchase access for this account. |
| `Shared { context_id, state }` | Evidence about one opaque sharing context. No campaign schema is assumed. |

`AccessState` is `Granted`, `Unavailable`, or `Unknown`. A grant must come from
explicit access evidence, never from a catalog record, title, URL, or stored
session. `Unavailable` records explicit denial or revocation for that context.
Unknown evidence is not a grant. The types have no default positive state.

Books retain distinct observations in their first-seen order. `Book::access`
derives a summary rather than storing a permanent usable-access flag:

- A purchase grant gives `Purchased`; a sharing grant gives `Shared`.
- Both give `PurchasedAndShared`.
- Without a grant, all observations must be explicitly unavailable to give
  `Unavailable`. Otherwise the result is `Unknown`.

Removal of one sharing grant cannot cancel an independent purchase or another
sharing grant. Two different states for the same purchase or sharing context
are a conflict, including an unknown state paired with a definite state.
The entire discovery fails instead of selecting the last observation.

Each call starts with an empty result. It does not merge an earlier snapshot,
cache a grant, or prove that pages were observed atomically. An older returned
snapshot remains a historical value, not authority for a later read. Future
book reads must check current access again. This offline boundary cannot detect
an upstream revocation that the caller does not report.

### Pagination and failure semantics

These are fixed local limits, not verified D&D Beyond limits.

| Concern | Limit or rule |
|---------|---------------|
| Raw entries per page | 100 |
| Pages per discovery | 10, including empty pages |
| Total raw entries | 1,000, including duplicates and unavailable or unknown records |
| First page | `cursor` must be `None` |
| Following page | `cursor` must match the preceding `next_cursor` |
| Completion | A terminal page must have `next_cursor: None` |
| Continuation | Opaque nonempty, unpadded text without control characters; never a fetch URL |

Pages must form one complete chain. Empty intermediate pages are allowed.
Repeated or cyclic continuation markers, missing or out-of-order pages, input
after a terminal page, and limits are explicit failures. Even an empty library
needs a terminal page. Assembly stops on its first failure and never returns a
successful partial library. Iteration consumes at most one page beyond the page
cap to detect overflow; that page is not processed or retained. The caller
supplies already-materialized pages, so the caps do not bound caller allocations.

Duplicate detection uses book identifiers and preserves first-seen book order.
Exact observations are collapsed. Independent access contexts are combined.
Different titles or parsed source URLs for the same identifier fail with
`ConflictingMetadata`. Entries count toward limits before duplicate removal.
Input data never chooses a source URL or access state by last-write-wins.

`LibraryError` distinguishes invalid metadata or evidence, conflicting metadata
or evidence, invalid pagination, incomplete discovery, exceeded limits, and
unsupported live discovery. A supplied failed page produces `PageRead(HttpError)`
and retains the original typed, redacted HTTP cause. Rejected sessions remain
distinct from denied access; a 401 is not reclassified as proven expiry.
Failures contain no partial snapshot. Error formatting and metadata Debug
implementations omit raw records, private identifiers, continuation values,
source URLs, and credentials.

Synthetic tests cover the access matrix, revocation and fresh assemblies,
duplicate conflicts, normalized-input validation, complete and incomplete
pagination, exact limits, duplicate-heavy overflow, and safe diagnostics.
Tests reuse the HTTP module's loopback mock to confirm that real 401, 403, 429,
and malformed-JSON failures keep their types through page assembly. They do not
pretend that this mock is a library response contract.

Live schema-change tests, current entitlement checks, and discovery integration
remain blocked pending a permitted endpoint, actual response and pagination
contracts, and usable session handling. No upstream book response or real
account has been read for this offline work.

## Offline character and campaign boundary

The core `character_campaign` module validates and assembles explicitly supplied
normalized pages. These are internal offline records, not an upstream response
schema or import format. The module does not fetch pages, access credentials,
retain earlier results, or interpret game rules.

Character and campaign records use opaque identifiers, optional validated names,
optional HTTPS source URLs as metadata, and the original JSON value. The original
value preserves unknown fields. Campaign character associations remain opaque
identifiers and are not required to appear in a separate character result.
Duplicate identifiers collapse only when the complete normalized record matches;
conflicting records fail rather than choosing a last value. Record and snapshot
debug output omits account metadata.

Assembly uses local limits of 100 raw records per page, 10 pages, and 1,000 raw
records total. It requires a complete opaque cursor chain and returns no partial
snapshot. A terminal empty result is different from missing pages or an
unsupported live operation. A missing identifier in a completed supplied
snapshot means only that the supplied data has no matching record. It does not
prove that an upstream resource is missing or inaccessible. Supplied HTTP errors
retain their typed classification, including access denial.
These record-count limits do not bound memory already used by the caller to
build a page or its JSON values. Future adapters must apply response-byte limits
before parsing or constructing records.

`list_characters`, `get_character`, `list_campaigns`, and `get_campaign` always
return an explicit unsupported-operation error. They do not construct a client
or access credentials. No CLI or MCP operation is enabled. Live reads remain
blocked until permission and supported hosts, response schemas, pagination,
access rules, and session handling are established. Synthetic tests validate
internal behavior only; they do not prove an upstream contract.

## Data and output contracts

- v1 is read-only. Local session storage changes do not authorize upstream writes.
- Local `auth` commands exist. `character`, `campaign`, `book`, and `mcp` remain proposed.
- List operations will support table, JSON, and TSV with consistent field names.
- Keep structured values and typed failures in the core. Format at the adapter edge.
- Preserve unknown upstream character fields without interpreting game rules.
- Keep character and campaign page assembly offline until the live feasibility
  gate opens.
- Report upstream changes and unsupported response shapes explicitly.
- Distinguish inaccessible content, expired login, rate limits, and network failure.
- Bound book section size and continuation; return a source URL with each result.
- Do not crawl entire books or silently collect an account library.
- Treat retrieved text as untrusted data, not as tool instructions.

## Safety and testing

The [feasibility gate](roadmap.md) comes before live integration.
An authorized account is not proof that automation is permitted. Review the
[current Wizards terms](https://company.wizards.com/en/legal/terms) and get
appropriate permission or clarification where needed.

Future HTTP code must bound timeouts, redirects, retries, and response sizes.
Never forward cookies or authorization headers to an unapproved host. Retry only
safe reads within a strict budget. Respect rate-limit responses.

Use synthetic HTTP responses and local mock servers for tests. Cover malformed
data, unavailable content, session expiry, rate limits, and cancellation.
Share contract cases between CLI and MCP when those adapters expose the same reads.

## Open decisions

The implementation issues must settle these before the relevant behavior ships:

- Whether each required upstream operation has a permitted, stable access path.
- Which human-controlled browser login method can return a session safely.
- Whether the selected native credential backends work in each desktop user session.
- Which upstream fields and identifiers can be relied on without data loss.
- Which book section and continuation limits balance usability and access terms.
- Which MCP clients need native install support first.

If any required read is not permitted or available, document the limit. Do not
substitute a bypass, fake result, or silently wider scope.

## Feasibility assessment

**Reviewed: 2026-10-27. Result: no permitted integration path established.**
This completes the documented assessment for
[#1](https://github.com/PlagueHO/beyond-gate/issues/1), not the live integration
gate. Permission remains unresolved for every required read. Technical feasibility
and simple browser login remain unproven. Do not start dependent live integration.
This is a project access decision, not a legal opinion or a claim that upstream
cannot approve a future integration.

### Interactive use and evaluation assumption

The intended workflow is personal, interactive lookup of data the user can
already access in the browser. It is not unattended collection. For continued
technical evaluation, assume each CLI or MCP lookup follows a user request, with
no scheduled reads, background synchronization, crawling, or bulk extraction.
The user requested continued evaluation on the assumption that this personal use
is allowed. This is a working assumption for technical analysis, not new upstream
permission evidence.

Permission and technical feasibility are separate questions. Even under that
assumption, browser access does not establish a native session handoff or an API
response contract. Public documentation review and offline foundation checks can
continue without either. The unresolved upstream questions below do not prohibit
that offline work. They must not be reported as a confirmed refusal of personal
interactive use.

### Official evidence

All sources below were read on the review date. Document dates are those displayed
by the source, not a promise that its interface or policy will remain unchanged.

| Source | Document date | Relevant evidence and limit |
|--------|---------------|-----------------------------|
| [E1: Wizards General Terms](https://company.wizards.com/en/legal/terms) | December 10, 2025 | Sections 2.2(i) and 2.2(iii) restrict unauthorized automated reads and third-party tools. Section 2.2(iv) prohibits security and authentication breaches. Sections 6.1 and 6.2 reserve content rights; section 10 gives no service continuity guarantee. |
| [E2: D&D Beyond Terms & Conditions](https://www.dndbeyond.com/en/terms-conditions) | June 25, 2026 | Incorporates the General Terms. Sections 1.4 and 2.2 limit content and account rights. Section 5.2(III) invites permission requests and describes opening client protocols as a future aim, not a current API grant. Section 5.2(V) prohibits unauthorized automation and extraction tools. |
| [E3: D&D Beyond subscriptions](https://www.dndbeyond.com/en/subscribe) | No update date displayed | Describes purchased content sharing through Master Tier campaigns. This is a platform feature, not permission for Beyond Gate to discover or extract shared content. |
| [E4: D&D Beyond home page](https://www.dndbeyond.com/en) | No update date displayed | Links to My Characters and My Campaigns. These are user interfaces, not documented response contracts for this app. |
| [E5: Play-by-Post guide](https://www.dndbeyond.com/posts/1668-play-by-post-playing-dungeons-dragons-in-pbp) | February 24, 2024 | Its Avrae section identifies an integration supported by D&D Beyond and links to Avrae documentation. That support does not authorize another app. |
| [E6: Avrae Getting Started](https://avrae.readthedocs.io/en/stable/cheatsheets/get_started.html) | No update date displayed | Describes public character import by sharing URL and D&D Beyond/Discord linking for campaign rolls. It does not publish an API or native login handoff for Beyond Gate. |
| [E7: Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy) | November 15, 2017 | Rule 7 preserves the Wizards Terms. The FAQ excludes verbatim copying and reposting of books from Fan Content. This policy does not establish an automated access exception. |

E2 applies specifically to D&D Beyond and incorporates E1. Personal use, a public
character URL, a subscription, and content sharing do not establish authorization
for this app under those rules. E5 and E6 show that supported integrations exist;
they do not establish general developer access or reusable partner permission.

### Required reads and decisions

**Unresolved** means upstream permission or clarification is required before a
live proof. **Unproven** means no usable API contract was established from the
reviewed sources. Every row below is blocked for live integration.

| Required read | Permission result | Technical result and evidence | Required clarification |
|---------------|-------------------|-------------------------------|------------------------|
| List the user's characters | Unresolved; E1, E2 | Unproven. E4 exposes a user interface, not a list API contract. | Approve account-scoped discovery and its supported interface. |
| Read an accessible character | Unresolved; E1, E2 | Unproven for Beyond Gate. E6 documents Avrae import, not a reusable character API. | Approve individual reads, including private and public character access rules. |
| List the user's campaigns | Unresolved; E1, E2 | Unproven. E4 links to campaign management, not a list API contract. | Approve campaign discovery and define membership visibility. |
| Read a campaign and its accessible character references | Unresolved; E1, E2 | Unproven. E6 describes linked campaign rolls, not a campaign response schema. | Define which campaign fields and participant references this app may read. |
| Discover purchased books and accessible book metadata | Unresolved; E1, E2 | Unproven. E2 describes licensed access, not an entitlement or library API. | Approve library discovery and define identifiers and entitlement checks. |
| Discover campaign-shared books and accessible book metadata | Unresolved; E1, E2 | Unproven. E3 establishes platform sharing, not an external entitlement API. | Define campaign-specific access, sharing removal, and the supported discovery interface. |
| Read a purchased or shared book outline | Unresolved; E1, E2, E7 | Unproven. No outline schema or supported retrieval interface was established. | Approve outline reads and define the permitted metadata and access checks. |
| Read a bounded section of a purchased or shared book | Unresolved; E1, E2, E7 | Unproven. No section schema, continuation contract, or permitted size was established. | Approve bounded text delivery through CLI and local MCP, with source URLs and explicit limits. |

No required read is approved by this assessment. There is also no confirmed
permanent refusal from upstream. The current decision is **no-go for live work
until permission and a usable contract are established**, not a claim that all
future access is impossible. A small response limit is not an access exception.

### Technical evidence boundary

The verified public pages use `company.wizards.com`, `www.dndbeyond.com`, and
`avrae.readthedocs.io`. Subscription discovery redirected from
`marketplace.dndbeyond.com` to `www.dndbeyond.com/en/subscribe`. These are evidence
locations, not an approved runtime host allowlist.

No officially supported runtime API host, endpoint, cookie name, token field,
exchange, expiry interval, refresh method, rate limit, or stable response schema
is established for Beyond Gate. A later
[community-based probe](authentication.md#opt-in-session-exchange-probe) tests a
candidate exchange offline and can be run manually. On 2026-10-27, the user
reported one successful live exchange. This is a narrow technical result, not an
official API contract or a general data-access result.
No versioning or endpoint support commitment was found in the reviewed official
sources. E1 section 10 and E2 section 4.7 allow service or availability changes;
neither is an API stability contract. See the
[login assessment](authentication.md#browser-handoff-assessment) for flow details
that remain unknown.

The review searched public official terms, D&D Beyond feature and subscription
guidance, and developer/API, OAuth, account-linking, content-sharing, and export
guidance. It followed the official Avrae documentation link in E5. Search summaries,
forum posts, and guessed endpoints were not used as permission or protocol
evidence. The later community-based probe is separate from the official-source
review and does not establish upstream permission. No supported export contract
was verified.

The [Tools and Features support page](https://dndbeyond-support.wizards.com/hc/en-us/categories/6712747082260-Tools-and-Features)
returned HTTP 403. Requests to search-supplied legacy `help.dndbeyond.com` article
URLs failed DNS resolution. The `/avrae` path on `www.dndbeyond.com` returned HTTP
404. Those pages were not evidence, and no access denial was bypassed. The issue
had no comments; its artifact list API returned HTTP 404. These limits prevent a
claim that every possible official or private partner path was reviewed.

### Permission questions and next gate

Before any live proof, obtain upstream authorization or official guidance that
answers these questions:

1. May this native local app perform each read above through both CLI and local
   MCP, including delivery of bounded licensed text to an MCP client?
2. Which supported API hosts, paths, response schemas, and access scopes may it
   use? What versioning, request limits, and change notices apply?
3. How must it check purchased and campaign-shared access, including removal of
   sharing, membership changes, regional restrictions, and expired access?
4. Which human-controlled native login handoff is permitted? What client
   registration, session exchange, expiry, refresh, and revocation rules apply?
5. What text size, continuation, caching, and retention limits apply to outlines
   and sections? Are there additional terms for partnered content?

Do not treat silence as approval. After permission and contracts are established,
start with synthetic offline proofs. A separate explicit maintainer approval is
still required for a live experiment. Define its hosts, operations, request
budget, stop conditions, and cleanup first. Do not persist credentials or content
during that experiment or attach account responses, HAR files, or browser data.
Persistent protected session storage belongs to later approved product work.

The initial official-source review did not establish a response contract to test.
Later community implementation evidence supports an isolated candidate token
exchange. The ignored live test and its synthetic offline cases are documented in
the [authentication design](authentication.md#opt-in-session-exchange-probe).
The user reported one passing live token-exchange test on 2026-10-27. No character,
campaign, or book read, expiry check, or automatic login handoff was demonstrated.
The agent did not access the account or rerun the live request. The result does
not approve other live work.

Continued offline validation on 2026-10-27 passed the workspace build, formatting,
clippy, tests, doctests, and rustdoc checks from [Contributing](../CONTRIBUTING.md).
The foundation tests cover seven CLI/MCP process cases and one core doctest.
They verify the existing help, version, failure, output-stream, and
foundation-notice behavior. Additional synthetic probe tests check the isolated
exchange boundary. Neither set proves account access, automatic browser handoff,
or any required data read.

If permission cannot be obtained, propose a separate scope decision: retain an
offline-only foundation, or assess local user-supplied data with independently
verified rights and a supported export format. Neither option fulfills the
current live-read scope. Do not adopt it silently or substitute scraping.
