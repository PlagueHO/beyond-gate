# Architecture

## Current boundary

The workspace contains three crates. The core exports one shared foundation status
message. Both adapters use clap for help and version output. There is no product
network client, account model, credential store, or MCP protocol implementation.

The workspace uses Rust 2024 and resolver 3. The
[Cargo edition guide](https://doc.rust-lang.org/edition-guide/rust-2024/cargo-resolver.html)
recommends this resolver for Rust 2024. The pinned toolchain is
[Rust 1.99.1](https://github.com/rust-lang/rust/releases/tag/1.99.1).
The only direct external product dependency is
[clap 4.6.7](https://docs.rs/clap/4.6.7/clap/). The core's developer-only
[session probe](authentication.md#opt-in-session-exchange-probe) uses reqwest,
rpassword, and serde_json as development dependencies.
`Cargo.lock` records exact resolved versions, including development dependencies.

## Intended boundaries

| Crate | Owns | Does not own |
|-------|------|--------------|
| `beyond-gate-core` | Authorized reads, session use, models, typed errors | CLI formatting or MCP protocol |
| `beyond-gate-cli` | Resource-first commands, output, human login flow | Duplicate endpoint or access logic |
| `beyond-gate-mcp` | Stdio lifecycle, bounded read tools, error mapping | Browser login or credentials from an agent |

Prefer direct functions and concrete types. Introduce a trait only when an actual
test boundary or multiple behaviors need it. Do not create empty model, service,
repository, or transport layers before the first operation needs them.

The planned product network stack is async reqwest with Tokio. It is not
implemented yet. The developer probe uses reqwest's blocking interface only.
The MCP implementation should use the
[official Rust SDK, rmcp](https://github.com/modelcontextprotocol/rust-sdk),
with stdio only. Select current compatible versions when those issues start.
Do not copy ArcadeJanitor's custom HTTP/WebSocket MCP layer.

## Data and output contracts

- v1 is read-only. Local session storage changes do not authorize upstream writes.
- CLI resources are proposed: `auth`, `character`, `campaign`, `book`, `mcp`.
- List operations will support table, JSON, and TSV with consistent field names.
- Keep structured values and typed failures in the core. Format at the adapter edge.
- Preserve unknown upstream character fields without interpreting game rules.
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
- Which OS credential-store crates work on each supported desktop.
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
