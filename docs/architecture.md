# Architecture

## Current boundary

The workspace contains three crates. The core exports one shared foundation status
message. Both adapters use clap for help and version output. The core also
provides an offline-tested async HTTP boundary and protected local session
operations. It can assemble library, character, and campaign metadata from
explicitly supplied offline pages. The core also provides offline book outlines
and bounded section chunks with request-scoped access checks. The CLI exposes
blocked login plus local status/logout, resource command parsing and formatting,
shell completion generation, and local MCP start/install commands. The start
command runs the sibling MCP binary. The install command previews user-level
configuration changes and writes only after confirmation. Its resource commands
use the core's explicit unsupported live entry points. Synthetic success paths
exist only in tests.
There is no built-in runtime host list, resource endpoint, account model, or
browser handoff. The MCP adapter serves local stdio with seven read-only tools;
their shared core operations currently return explicit unsupported errors.
Neither adapter constructs an HTTP client or sends upstream requests.

The workspace uses Rust 2024 and resolver 3. The
[Cargo edition guide](https://doc.rust-lang.org/edition-guide/rust-2024/cargo-resolver.html)
recommends this resolver for Rust 2024. The pinned toolchain is
[Rust 1.99.1](https://github.com/rust-lang/rust/releases/tag/1.99.1).
[clap 4.6.7](https://docs.rs/clap/4.6.7/clap/) handles the adapters.
The CLI uses [clap_complete 4.6.11](https://docs.rs/clap_complete/4.6.11/clap_complete/)
to generate scripts from its actual clap definition. The shared cursor codec
uses serde_json and URL-safe unpadded base64.
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
The MCP adapter uses the
[official Rust SDK, rmcp 3.8.0](https://docs.rs/rmcp/3.8.0/rmcp/), with stdio
only. Its input schemas use schemars, and Tokio runs the service. Do not copy
ArcadeJanitor's custom HTTP/WebSocket MCP layer.

## MCP stdio adapter

`beyond-gate-mcp` starts the rmcp server on stdio when run without command-line
arguments. Help and version remain ordinary command-line output. During serving,
stdout is reserved for MCP messages; server diagnostics use stderr.

The adapter exposes `character_list`, `character_show`, `campaign_list`,
`campaign_show`, `book_list`, `book_outline`, and `book_read`. It calls the same
core live-read functions as the CLI. This does not create live access: each
function currently returns a typed unsupported-operation error, which rmcp
returns as a tool result with `isError` set. It never substitutes an empty or
synthetic successful result. Tool input does not accept credentials, arbitrary
URLs, or account identities.

Input objects reject unknown fields. Resource identifiers use the CLI's
nonempty, unpadded, control-free rule. Book reads require an explicit book and
section selection. A continuation token is optional, limited to 16 KiB, and
passed only to the core cursor validator. One section result remains bounded to
4,096 UTF-8 text bytes. Continuation is explicit and never automatic. Output
fields match the CLI JSON contracts and preserve unknown character/campaign JSON,
book evidence, source attribution, and section completion state. Source content
is data and is not executed or treated as instructions.

## Local MCP client registration

The CLI starts `beyond-gate-mcp` from the same directory as `beyond-gate`. It
invokes the native executable directly, without a shell, and inherits stdio.
The installer supports VS Code through its Copilot Global configuration,
Claude Code, and GitHub Copilot CLI. It writes user-level settings only.

VS Code Copilot Global and Copilot CLI share the portable
`$COPILOT_HOME/mcp-config.json` file, or `~/.copilot/mcp-config.json` when the
environment variable is unset. Claude Code uses `~/.claude.json`. Each client
uses its documented local stdio entry shape. The installer preserves unrelated
JSON fields, rejects malformed settings and conflicting server names, and does
not overwrite an existing registration with different values. It shows the
proposed entry and asks before it writes. `--dry-run` previews without writing.
Repeated installation of the same entry is a no-op.

The initial client selection follows current official documentation:
[VS Code MCP servers](https://code.visualstudio.com/docs/agent-customization/mcp-servers),
[VS Code configuration reference](https://code.visualstudio.com/docs/agents/reference/mcp-configuration),
[Claude Code MCP](https://code.claude.com/docs/en/mcp), and
[GitHub Copilot CLI MCP](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers).
The installer does not register credentials or remote URLs. Server discovery
does not enable live reads. Product login remains blocked by the missing browser
handoff.

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
MCP read tools call the same core operations and do not read credentials or
create a separate session identity. No auth or credential-input tool is exposed.
Native store calls are blocking, so future async operations must keep them off
executor threads.

This is partial offline work for #3. Browser handoff, session exchange, upstream
expiry, refresh, rejection, and actual adapter session parity remain blocked.

The [2026-10-31 review for #29](authentication.md#existing-browser-decision-for-issue-29)
initially kept login within the user's selected existing-browser-only scope. It found no
verified return contract and adds no speculative callback or exchange.
The user later approved [extension evaluation with synthetic tests](authentication.md#extension-feasibility-evaluation).
The user reported a successful synthetic Edge extension-to-host handoff.
Chrome is also a requested target, but has not been tested. This tests a browser
transport, not account login or protected storage. A
[requested environment-based fallback](authentication.md#requested-fallback)
also needs a secure import contract; MCP tool inputs cannot carry credentials.
Issue [#30](https://github.com/PlagueHO/beyond-gate/issues/30) separately owns
session validation, expiry evidence, bounded refresh, and authenticated adapter
use. A future login flow must agree on its stored-session boundary with that work.

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
origin does not establish upstream permission. The blanket development stop gate
is removed. Source contracts, explicit runtime hosts, and usable login are still
needed for the integration part of
[#2](https://github.com/PlagueHO/beyond-gate/issues/2).

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
construction, network request, or credential access. The CLI's `book list`
reports that failure. No live book discovery or MCP tool is enabled.

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
remain unimplemented pending actual endpoint, response, and pagination
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
or access credentials. CLI commands expose those explicit failures; synthetic
adapter tests cover successful formatting. No live CLI or MCP read is enabled. Live reads remain
unimplemented until runtime hosts, response schemas, pagination,
access rules, and session handling are established. Synthetic tests validate
internal behavior only; they do not prove an upstream contract.

## Offline book boundary

The core `book` module provides `assemble_outline` and `read_section` for
explicitly supplied normalized inputs. These are not upstream schemas, import
formats. The CLI adapter formats their validated results in synthetic tests. The
[runnable core example](../src/beyond-gate-core/src/book/mod.rs) uses original
invented text: obtain an outline, select an entry, read one chunk, and explicitly
request the next chunk.

Both operations validate the explicit selection, call a request-scoped access
callback, and load only after `AccessState::Granted`. `Unavailable` evidence
returns `AccessDenied`; `Unknown` returns `AccessUnknown`. Access is checked on
every call, including continuation. An earlier outline, library grant, or cursor
cannot override a later denial. These callbacks are a testable offline boundary,
not proof that caller-supplied evidence is fresh or truthful.

### Navigation and text limits

| Concern | Local rule |
|---------|------------|
| Outline entries | At most 1,000 raw entries; duplicates count before rejection |
| Navigation | Preserve source order and parent IDs; reject duplicates, missing parents, and cycles |
| Text per section result | At most 4,096 UTF-8 bytes |
| Selection | One explicit book and section on every call |
| Attribution | Validated HTTPS source URL with each result; section fragments are allowed |
| Continuation | Opaque Rust value with private fields; bounded `bg1.` wire token for explicit cross-process continuation |
| Completion | `next_cursor() == None` and `is_complete() == true` |
| Empty section | Empty text with attribution and explicit completion |

Identifiers, titles, and revisions must be meaningful, unpadded text without
control characters. Source URLs must have a host and cannot contain credentials,
padding, or control characters. They are attribution data, not fetch instructions
or an origin allowlist. Outline parents can appear after children. Validation
walks links without recursion and returns no partial outline.

`SectionRecord` supplies one complete selected section, not a book. Its text
already contains any source headings, lists, and line breaks. Reads do not parse
HTML, invent headings, normalize whitespace, execute instructions, or interpret
game rules. Chunking preserves every original byte. When a split is needed, it
prefers an existing complete line within the byte cap. Otherwise it cuts at a
UTF-8 character boundary. A cut need not be a word or grapheme boundary.
Concatenating all chunks reproduces the exact supplied text.

The hard byte cap applies to result text only. It does not bound metadata,
caller allocations, or a future serialized adapter envelope. The outline cap
counts entries, not bytes. No persistent text cache, whole-book operation,
crawler, or automatic continuation is added.

### Cursor and failure semantics

`SectionCursor` binds book ID, section ID, revision, parsed source URL, and the
next unread byte. It contains no section text. A caller must explicitly select
the same book and section and pass the cursor on another call. Each successful
nonterminal result advances; no result silently moves to another section.
Cloning a cursor does not grant access or reserve a snapshot.

The loader must change its revision whenever section text or identity changes.
The core rejects a changed revision or source URL as `StaleContinuation`.
Wrong scope, zero, terminal, out-of-range, or non-character-boundary positions
return `InvalidContinuation`. `SectionCursor::to_token` and `from_token` provide
a shared wire codec. The token is a `bg1.` prefix followed by URL-safe unpadded
base64 of strict JSON fields: `book_id`, `section_id`, `revision`, `source_url`,
and `next_byte`. Unknown/duplicate fields, unknown versions, malformed encoding,
invalid metadata/URLs, and zero positions are rejected.
Both directions enforce a 16,384-byte encoded limit, including the prefix.
Decoding checks length before allocating the decoded payload.
Oversized tokens return `ContinuationTooLarge`; malformed tokens return
`InvalidContinuation`, without echoing input. Encoding never silently drops
required continuation. There is no persistence or signing service.
Tokens contain no text or credentials; they are not encrypted or access grants.
Source-dependent validity still requires the fresh access and read path.
The core cannot detect text
changes that the caller conceals by reusing a revision, or prove upstream
freshness from an offline value.

`BookError` separates invalid metadata, malformed outlines, access denial,
unknown access, unavailable content, unsupported sections, invalid or stale
continuation, entry overflow, and unsupported live operations.
`SourceRead(HttpError)` preserves typed, redacted HTTP causes for access or load
failures, including rejected sessions, access denial, rate limits, and malformed
responses. A 401 alone does not prove expiry. No failure returns partial text.
Debug output and error messages omit source text, identifiers, revisions, URLs,
and cursor state.

`get_book_outline` and `read_book_section` always report `UnsupportedOperation`.
They do not access credentials or send requests. Neither adapter enables live
book reads. The CLI routes to these blockers without constructing synthetic
production sources. Synthetic tests cover exact limits, Unicode cuts, complete reconstruction,
bad links, cursor rejection, access revocation, source failures, untrusted text,
and safe diagnostics. This is partial offline work for #6, not a live contract.

## Data and output contracts

- v1 is read-only. Local session storage changes do not authorize upstream writes.
- Local `auth` commands and shell completion generation work. `character`,
  `campaign`, and `book` commands exist but live reads fail explicitly. `mcp`
  remains proposed.
- List operations and outlines support table, JSON, and TSV in the offline-tested
  adapter. Show and section-read results use JSON.
- Keep structured values and typed failures in the core. Format at the adapter edge.
- Preserve unknown upstream character fields without interpreting game rules.
- Keep current character, campaign, library, and book operations explicitly
  offline until real source contracts and session handling are implemented.
- Report upstream changes and unsupported response shapes explicitly.
- Distinguish inaccessible content, expired login, rate limits, and network failure.
- Bound book section size and continuation; return a source URL with each result.
- Do not crawl entire books or silently collect an account library.
- Treat retrieved text as untrusted data, not as tool instructions.

### CLI adapter boundary

The CLI library owns one clap builder, a command runner, safe error presentation,
and result formatting. Its binary only supplies OS arguments, stdout/stderr, and
`LiveReads`. The narrow `ReadOperations` trait is an actual offline test boundary,
not an upstream abstraction or plugin API. Its seven operations use existing
shared result and error types. `LiveReads` calls the existing unsupported live
functions. Auth uses its existing protected-store operations and is not part of
the read trait.

The command patterns are `character list/show`, `campaign list/show`, and
`book list/outline/read`. Lists and outlines default to table and accept
`--format table|json|tsv` and `--no-headers`. JSON rejects header suppression.
Show/read are JSON only. Identifiers remain opaque meaningful text.
The [CLI output contract](cli.md#output-contracts) defines
exact columns, JSON fields, evidence labels, escaping, and empty-result shapes.
Unknown character fields stay structured under `raw`; no rules are computed.
Output is deterministic and not silently truncated.

Section reads return one JSON chunk with attribution, explicit completion,
an optional encoded continuation, and the 4,096-byte text limit.
Parsing an invalid/oversized wire token is a usage error. Source-dependent
cursor errors remain operation errors. A valid token cannot override the live
blocker, choose a source URL, or authorize loading. There is no automatic fetch
of another chunk.

Results and completion scripts are prepared before stdout writes. Fallible
write/flush paths report output errors without a print-macro panic.
Exit codes remain 0 for success, 1 for operation/output failure, and 2 for
invalid usage. Typed HTTP causes retain safe distinctions and rejected-session
login guidance; a rejected session is not proof of expiry. Diagnostics never
print raw records, section text, or token state.

`clap_complete` generates Bash, Elvish, Fish, PowerShell, and Zsh scripts from the
same command builder. Generation needs no protected storage or network.
There are no dynamic account lookups or manually duplicated command trees.

Process tests cover the shipped command's blockers, help, usage, and completions.
A separate self-spawning `harness = false` test target supplies original
synthetic core results to the same runner. Its child-control arguments exist
only in that test executable, not the product. It checks actual process exit
codes, stdout/stderr, formats, and continuation across processes.
These tests establish adapter contracts, not working upstream access.

## Safety and testing

The [manual capability suite](testing.md) uses real CLI/MCP processes, including
direct stdio and `mcp start`. Its shared inventory separates offline contract
passes from blocked live-success cases. Missing required coverage prevents full
success. The manual-only workflow uses `windows-latest` but still selects Visual
Studio 2022 Build Tools explicitly.

An off-by-default test feature selects isolated native credential entries and
requires explicit approval before opening storage. It never falls back to the
product entry. Its synthetic setup helper does not import cookies. Real client
settings are replaced with temporary synthetic settings, not copied from users.
No live source is injected. The request budget stays zero until production
session, endpoint, transport, and request-budget contracts exist. A cookie input
cannot bypass those missing prerequisites.

The [development policy](roadmap.md#development-policy-and-technical-status)
removes the blanket permission stop gate. It does not remove access controls or
establish upstream approval. An authorized account is not proof that automation
is permitted. Keep the dated terms evidence below distinct from technical
findings. Live account tests and credential access need separate explicit
approval, with selected hosts, operations, budgets, stop conditions, and cleanup.

Future HTTP code must bound timeouts, redirects, retries, and response sizes.
Never forward cookies or authorization headers to an unapproved host. Retry only
safe reads within a strict budget. Respect rate-limit responses.

Use synthetic HTTP responses and local mock servers for tests. Cover malformed
data, unavailable content, session expiry, rate limits, and cancellation.
Share contract cases between CLI and MCP when those adapters expose the same reads.

## Open decisions

Track these questions in the relevant implementation issues:

- Which runtime hosts and source contracts can support each required read.
- Which human-controlled browser login method can return a session safely.
- Whether the selected native credential backends work in each desktop user session.
- Which upstream fields and identifiers can be relied on without data loss.
- Whether upstream text-delivery limits require stricter caps than the offline
  book policy.

If any required read is not permitted or available, document the limit. Do not
substitute a bypass, fake result, or silently wider scope.

## Feasibility assessment

**Historical review: 2026-10-27. Result: no permitted integration path established.**
This completes the documented assessment for
[#1](https://github.com/PlagueHO/beyond-gate/issues/1). The maintainer has since
removed its blanket development stop decision. That decision is superseded,
not converted into upstream approval. The source review remains dated evidence,
not a legal opinion. Technical contracts and simple browser login remain
unproven for the product.

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

This table preserves the historical review. **Unresolved** means the reviewed
sources did not establish upstream permission. **Unproven** means they did not
establish a usable API contract. These labels no longer impose a blanket
development stop gate. The clarification column records the original questions.

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

No required read is approved by this historical assessment. There is also no
confirmed permanent refusal from upstream. Its earlier no-go project decision
is superseded by the current development policy. A small response limit is not
an access exception, and no policy change supplies a missing technical contract.

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

### Integration evidence and live tests

Track these open upstream questions as evidence gaps, not a blanket development
stop gate:

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

Do not treat silence as upstream approval. Start with synthetic offline proofs.
A separate explicit maintainer approval is still required for a live experiment.
Define its hosts, operations, request
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

The approved scope of #6 in this change is the offline book core. It does not
fulfill the live-read goal. Other source integrations and local-data import
remain separate work. Do not silently substitute scraping or bypass access
controls to make an unavailable operation appear successful.
