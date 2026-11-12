# MCP tool reference

**Current status:** `beyond-gate-mcp` is a local read-only stdio server using the
official Rust MCP SDK, rmcp. Startup, initialization, tool discovery, validation,
and protocol errors are implemented. Windows `book_list` has bounded live
metadata transport with Web-catalog scope, category selection, and title cleanup.
Native CLI/MCP acceptance passed with matching Web-catalog results.
Windows `book_outline` and `book_read` use fresh library evidence and checked
canonical cookie pages. Synthetic process checks cover both adapters.
Native book adapter acceptance passed for one selected chapter with matching
outlines and initial chunks. Live continuation and sharing remain unchecked.
Character and campaign reads use fixed, bounded operations through the stored
Windows session. Synthetic process checks cover successful reads and matching
CLI/MCP data. No live character or campaign acceptance has been run.
Windows `auth_validate` makes one fresh session check
through the shared core without credential input.
`class_list` and `class_show` provide bounded class/subclass metadata only.
Browser catalog contracts were inspected; native compatibility, real sharing,
and live global traversal remain unverified.
`monster_list` and `monster_show` return one explicit page or unchanged selected
detail for observed access code 1. Code 4 is denied; unverified numeric codes,
including 2 and 3, are unsupported. Native monster acceptance has not run.
`background_list` returns website-filtered metadata. `background_show` selects
one raw definition from a fresh complete collection. The observed live collection
exceeds the unchanged 1 MiB cap and fails explicitly; native acceptance has not run.
`feat_list` returns bounded website metadata. `feat_show` selects one checked
raw definition from a fresh bounded bearer collection.

The four character and campaign tools perform read-only source calls. Failures
return MCP tool errors with `isError: true`; they do not return sample data or
empty success results. Success shapes are offline-tested adapter contracts, not
evidence of successful live access.

See [native installation](installation.md) for setup and
[authentication limits](authentication.md#local-session-commands) for
human-controlled Windows browser login and protected local session commands. The server does not provide
a separate login or account-selection mechanism.

## Startup and lifecycle

A client can launch the server directly:

```shell
beyond-gate-mcp
```

Or it can use the CLI launcher when both binaries are in the same directory:

```shell
beyond-gate mcp start
```

No arguments are needed for direct stdio serving. There is no `--transport`,
HTTP listener, port, remote endpoint, or remote-hosting mode. The CLI launcher
starts the sibling executable without a shell, inherits its streams, and passes
through its exit status.

**Planned startup input:** [#52](https://github.com/PlagueHO/beyond-gate/issues/52)
will let a client supply `CobaltSession` through the server environment.
The shared core will use it only in process memory, without a browser or native
credential store. An explicit environment value will override saved credentials
and fail safely rather than silently fall back. Client secret-input references
must not embed literal secrets in configuration. MCP will not prompt on its
streams, accept credential-bearing tool arguments, or open a browser implicitly.
This input path is not implemented yet. See the
[agreed authentication follow-up](authentication.md#agreed-authentication-follow-up).

The MCP client owns the process and connects its stdin and stdout. It initializes
the session with `initialize`, sends `notifications/initialized`, discovers tools
with `tools/list`, and invokes a tool with `tools/call`. Messages use JSON-RPC
over stdio. The offline process test sends one JSON message per line.
The server waits for transport closure or failure; it is not a terminal prompt.
The process test covers cancellation notification handling and exit after stdin
closes. There is no product shutdown tool.

During serving, stdout contains MCP protocol messages only. Startup and transport
diagnostics use stderr. Direct `--help`/`-h` and `--version`/`-V` instead produce
ordinary CLI output and exit with code 0. Unknown command-line arguments exit
with code 2 without protocol output. Successful server closure uses code 0;
initialization or transport failures use code 1. A tool error is a protocol
result, not a request to stop the server.

## Client registration

**Warning:** Registration changes user-level client settings after confirmation.
Preview first:

```shell
beyond-gate mcp install vscode --dry-run
beyond-gate mcp install claude-code --dry-run
beyond-gate mcp install copilot-cli --dry-run
```

Remove `--dry-run` only when you intend to review and confirm the change.
The [CLI installer reference](cli.md#mcp-install) specifies client commands,
configuration paths, confirmation, conflicts, and repeat installation.
It registers the absolute `beyond-gate-mcp` path with no arguments, not an HTTP
URL. Do not add credentials to client configuration.

VS Code uses Copilot Global configuration. It shares the Copilot MCP settings
file with Copilot CLI. Claude Code uses its user-level settings. For discovery,
open the MCP server list in VS Code, enter `/mcp` in Copilot CLI, or run
`claude mcp list` in Claude Code. Discovery does not establish live data access.

To remove registration, remove only `beyond-gate` from the configuration file's
`mcpServers` object. Keep unrelated settings. Moving the binaries changes their
absolute path; remove the old registration before installing the new path.

## Shared input rules

Every tool takes a JSON object and rejects unknown fields. Character, book, and
species list tools require an empty object `{}`. Campaign, class, monster, and
background, feat, and spell lists accept their documented filters. There are no passwords,
cookies, authentication tokens, arbitrary URL inputs, or account identities.
The only token input is `book_read.continuation`, which represents a bounded
section position, not a credential.

Resource identifiers are opaque strings. Their schemas declare `minLength: 1`.
Runtime validation also rejects leading or trailing whitespace and all control
characters. Numeric identifiers must still be strings. Identifiers are selections,
not URL-fetch instructions.

`book_read.continuation` is optional and accepts a string or `null`; omission or
`null` starts without continuation. Its schema declares `maxLength: 16384`.
The core independently enforces the 16,384-byte encoded-token limit and validates
the wire format. Do not treat a schema length check as a substitute for runtime
token validation.

All tools advertise `readOnlyHint: true`, `destructiveHint: false`,
and `openWorldHint: true`. Resource tools use `idempotentHint: true`.
`auth_validate` uses `idempotentHint: false` because it makes a fresh service
exchange and updates transient evidence. These annotations do not grant
access or make live reads available. Tool results are data, not instructions.

## Tool index and CLI mapping

| Tool | CLI equivalent | Success data shape |
|------|----------------|--------------------|
| [`auth_validate`](#auth_validate) | `beyond-gate auth validate` | Safe validation evidence |
| [`character_list`](#character_list) | `beyond-gate character list --format json` | Array of character objects |
| [`character_show`](#character_show) | `beyond-gate character show <id> --output json` | One character object |
| [`campaign_list`](#campaign_list) | `beyond-gate campaign list --format json [--active-only]` | Array of campaign objects |
| [`campaign_show`](#campaign_show) | `beyond-gate campaign show <id> --output json` | One campaign object |
| [`class_list`](#class_list-and-class_show) | `beyond-gate class/subclass list --output json` | Compact metadata array |
| [`class_show`](#class_list-and-class_show) | `beyond-gate class/subclass show <id>` | Selected metadata object |
| [`species_list`](#species_list-and-species_show) | `beyond-gate species list --output json` | Metadata array |
| [`species_show`](#species_list-and-species_show) | `beyond-gate species show <id> --output json` | Selected metadata and unchanged raw record |
| [`monster_list`](#monster_list-and-monster_show) | `beyond-gate monster list --output json` | One entries/page envelope |
| [`monster_show`](#monster_list-and-monster_show) | `beyond-gate monster show <id>` | Selected identity and unchanged raw detail |
| [`background_list`](#background_list-and-background_show) | `beyond-gate background list --output json` | Metadata array |
| [`background_show`](#background_list-and-background_show) | `beyond-gate background show <id>` | Selected identity, sources, and unchanged raw definition |
| [`feat_list`](#feat_list-and-feat_show) | `beyond-gate feat list --output json` | Metadata array |
| [`feat_show`](#feat_list-and-feat_show) | `beyond-gate feat show <id>` | Unsupported protected detail |
| [`spell_list`](#spell_list-and-spell_show) | `beyond-gate spell list --output json` | Entries and explicit page/next_page |
| [`spell_show`](#spell_list-and-spell_show) | `beyond-gate spell show <slug>` | Selected identity and untrusted raw_html |
| [`book_list`](#book_list) | `beyond-gate book list --format json` | Array of book objects |
| [`book_outline`](#book_outline) | `beyond-gate book outline <book-id> --format json` | Outline envelope |
| [`book_read`](#book_read) | `beyond-gate book read <book-id> <section-id> [--continuation <token>]` | One section chunk |

The data shapes below match CLI JSON, not CLI table or TSV presentation.
Examples are synthetic except the two observed Fighter IDs in the class section.

## Tools

### feat_list and feat_show

```json
{"name":"feat_list","arguments":{}}
{"name":"feat_list","arguments":{"name":"invented","partnered_content":"any"}}
{"name":"feat_show","arguments":{"id":"17"}}
```

List accepts optional `name`, one `prerequisite_subtype_id`, `tag_ids`,
`source_ids`, `source_category_ids`, and `partnered_content`.
The [shared input table](cli.md#feat-metadata-and-detail-status) defines their
limits. Unknown fields, explicit null, invalid IDs, duplicate array values,
and page/skip/take are rejected. Website IDs are not access grants.
List returns `id`, `name`, `source`, and `source_url`, without rules text.
Requested options and form echoes must agree. Valid empty results require
the observed marker. The source/result cap is 1 MiB, the row cap is 1,000,
and the deadline is 30 seconds.

Show requires a string canonical positive decimal `u64` ID. It reads public
source configuration and one fixed protected collection, then returns only the
selected row after the complete response passes validation. A tool error uses
`isError: true`; no sample or empty success is returned. The selected record
preserves unknown `raw` and source fields, repeatable flags, and parent IDs.
No rules engine, parent traversal, ownership inference, or detail fallback is
implemented. See the [source status](api-catalog.md#feat-browse-and-detail-status).

### background_list and background_show

```json
{"name":"background_list","arguments":{}}
{"name":"background_list","arguments":{"name":"invented","skill_ids":[240,241],"partnered_content":"any"}}
{"name":"background_show","arguments":{"id":"17"}}
```

List accepts optional `name`, `skill_ids`, `tag_ids`, `source_ids`,
`source_category_ids`, and `partnered_content`. Arrays default to empty and
permit at most 50 unique positive `u32` website option IDs each.
Name is nonempty, unpadded, control-free literal text within 256 UTF-8 bytes.
Partnered content is `yes`, `no`, or `any`, defaulting to `no`.
Explicit null, unknown fields, and page/skip/take inputs are rejected.
Requested IDs must exist in the same response's website form.
The [shared filter table](cli.md#background-metadata-and-selected-detail)
maps these fields to the actual website controls.

List returns an array with `id`, `name`, `source`, and `source_url`.
It excludes rules text and preserves duplicate names. A verified empty marker
returns `[]`; missing structure or unexpected paging does not.
Show requires one canonical positive decimal `id` string that fits `u64`.
It returns `id`, `name`, `source_url`, `sources`, and unchanged `raw`.
Unknown definition and source-reference fields survive.
The fixed complete collection is checked before selection, including every ID,
display link, and configured source reference. Links are never fetched.
A missing ID is absent from that collection, not necessarily from the website.

Both operations pin the session identity. List uses one cookie-only website
read. Show uses credential-free public configuration and one bearer-only
collection read with `sharingSetting=2`. It does not guess a detail route or
selected-ID API filter. The first-party reader returns definitions directly
without a monster access map. Unexpected access fields are unsupported.
No ownership, source-book access, or later read grant is inferred.

Bounds are 1,000 raw rows/definitions, 1,000 references per definition, 1 MiB
response/result, and 30/60 seconds for list/show. **The observed live collection
was 1,160,568 bytes, so show rejects it with `BodyTooLarge` under the unchanged
production cap.** There is no truncation, automatic continuation, cache,
partial output, or metadata-only show fallback.
Denied, rejected/expired, missing, malformed, incomplete, unsupported, and
limit failures produce tool errors, not empty success results.
Cancellation drops the active shared future.

Synthetic process checks compare CLI and MCP data, errors, schemas, and
cancellation. Native compatibility, native account acceptance, website/API
identity coverage, and real sharing remain unverified. See the
[source contract](api-catalog.md#background-read-contract).

### monster_list and monster_show

```json
{"name":"monster_list","arguments":{}}
{"name":"monster_list","arguments":{"name":"wolf","cr_max":"2","size_ids":[4,5],"legendary":false}}
{"name":"monster_list","arguments":{"name":"wolf","cr_max":"2","size_ids":[4,5],"legendary":false,"page":2}}
{"name":"monster_show","arguments":{"id":"17"}}
```

The selected ID example is synthetic; use an ID returned by a successful page.
List accepts optional string `name`, positive `u32` `page` (default 1), and the
filter fields in the [shared input table](cli.md#monster-pages-and-detail).
`skip` and `take` are no longer accepted. CR bounds are literal strings,
not website option IDs. ID arrays permit at most 50 unique positive `u32`
values per field and 256 total; the returned public form must contain the
selected options. Numeric AC/average-HP bounds must be nonnegative and ordered.
Legendary, mythic, and lair selectors are optional booleans. Partnered content
is `yes`, `no`, or `any`, default `no`; `any` sends an empty website value.
Names permit at most 256 UTF-8
bytes without padding or controls. Null selectors, wrong types, overflow, and
unknown fields fail before session access. Show accepts only required string
`id`, a canonical positive decimal integer within `u64`. Numeric IDs must still
be strings.

List returns `entries` and `pagination` with `page` and `next_page`.
Each entry has string `id`, `name`, fixed-website `source_url`, and
numeric `access_code: 1`. Null `next_page` is terminal. Continuation requires a
new explicit call with the same filters; it does not pin a snapshot or grant
access. Duplicate names remain separate IDs. Empty pages retain metadata.
Show adds unchanged `raw` from the selected response, preserving unknown fields,
statistics, source references, and description fields. Neither missing fields
nor rules versions are inferred.

List makes one cookie-only website GET, then one bearer-only selected-ID
collection GET for a nonempty page. It uses the official website listing,
not the homebrew route. Empty pages require the explicit source marker and no
collection read. The website controls page size; observed full pages had 20
rows, with a local ceiling of 100. No requested page size or total is invented.
Collection IDs and grants must match the candidate set exactly; output keeps
website order. Show makes one selected service GET. A required session exchange
can precede reads. All reads use one process-local session identity.
Detail checks its own scalar
access evidence. Only code 1 permits output. Code 4 denies output; unverified
numeric codes, including 2 and 3, are explicitly unsupported. Missing or invalid
access, malformed records, duplicate IDs, inconsistent pages, and limits return
tool errors. Any failed row fails the whole page. HTTP 404, HTTP denial,
rejected sessions, and expiry remain distinct. No metadata-only detail fallback,
automatic continuation, retries, redirects, display-link fetches, or cache is
added.

Bodies and compact core results are capped at 1 MiB. The whole operation is
capped at 30 seconds, including session work; queries are capped at 16 KiB.
MCP cancellation drops the read.
These are local limits. Browser checks observed filtered pages, an explicit
empty marker, a two-ID collection, one selected detail, and a denied listing.
They do not prove native cookie compatibility, larger batches, native
account acceptance, owned/shared access, complete stat-block coverage, or full
source-book text. See [evidence](api-survey.md#monster-list-and-detail-research)
and the [shared CLI contract](cli.md#monster-pages-and-detail).

### class_list and class_show

`kind` is optional and defaults to `class`; `subclass` selects the parent-aware
catalog. `class_list` accepts optional `name`. `class_show` requires string `id`.
Only subclass operations accept optional string `class_id`. Supplied selectors
cannot be null. Reject unknown fields, wrong types/kinds, IDs on list, names on
show, and parent selectors on class reads before storage or network access.
IDs must be canonical positive decimal integers that fit `u64`.
Names must be nonempty, unpadded, control-free, and within 256 UTF-8 bytes.

These are exact `tools/call` parameter examples:

```json
{"name":"class_list","arguments":{}}
{"name":"class_list","arguments":{"name":"Fighter"}}
{"name":"class_list","arguments":{"name":"fight"}}
{"name":"class_show","arguments":{"id":"2190879"}}
{"name":"class_show","arguments":{"id":"10"}}
{"name":"class_list","arguments":{"kind":"subclass"}}
{"name":"class_list","arguments":{"kind":"subclass","name":"Champion"}}
{"name":"class_list","arguments":{"kind":"subclass","class_id":"2190879"}}
{"name":"class_list","arguments":{"kind":"subclass","class_id":"10","name":"Champion"}}
{"name":"class_show","arguments":{"kind":"subclass","id":"7001","class_id":"2190879"}}
{"name":"class_show","arguments":{"kind":"subclass","id":"7001"}}
```

Fighter IDs were observed; subclass `7001` and Champion are illustrative.
Names use local lowercase substring matching, not an API search parameter.
Duplicate names remain separate. An unscoped subclass show rejects matching IDs
under several parents instead of selecting the first.

Synthetic selected metadata:

```json
{
  "kind": "subclass",
  "id": "7001",
  "name": "Invented Champion",
  "parent_class_id": "2190879",
  "source_ids": [7],
  "is_homebrew": false,
  "slug": "invented",
  "more_details_url": "https://www.dndbeyond.com/classes/invented#Example",
  "sources": [{"sourceId": 7, "future_reference_flag": true}]
}
```

Lists return an array with only `kind`, `id`, `name`, `parent_class_id`, and
`source_ids`. Show adds the other fields above. A base class has a null parent.
Missing optional slug/link metadata is null. Invalid supplied metadata fails.
Source-reference extensions survive, but raw definition fields, descriptions,
features, equipment text, and spell rules do not. This is metadata detail, not
lossless raw-definition output or a content grant. No rules-year/legacy labels
are inferred. Links are display metadata and never choose a request.

The same core enforces [CLI bounds and errors](cli.md#class-and-subclass-metadata):
32 global parents, 1,000 raw definitions before filtering, 1 MiB per response,
8 MiB aggregate resource bodies, 1 MiB canonical result, and a 120-second whole
deadline. Reads are sequential with single attempts and no automatic continuation.
Class and scoped reads are not subject to the global parent count.
Any failed parent prevents a partial result. Empty success requires complete
checked discovery; missing IDs refer only to the default catalog scope.
MCP cancellation drops pending work. Production session use remains Windows-only.

### species_list and species_show

`species_list` takes the empty object `{}`. `species_show` requires string `id`.
The ID must be a positive canonical decimal integer that fits `u64`. Unknown
fields, non-string IDs, and leading zeroes fail before source access.

```json
{"name":"species_list","arguments":{}}
{"name":"species_show","arguments":{"id":"17"}}
```

List returns `id`, `name`, `source`, `source_url`, and `is_legacy` from the
fixed public species page. The site search is client-side, so the tool does
not infer a server-side filter or paging request. Show adds the unchanged
`raw` source record, including unknown fields, from a fresh
`/character/v5/game-data/races` collection read. It does not request the
displayed website link.

Missing or malformed page structure, duplicate IDs, denied reads, unsupported
paging, and exceeded limits return tool errors without partial data. Browser
evidence confirms the public page structure only. Detail access, complete
collection coverage, native compatibility, and the required null pagination
field remain unverified. The shared core caps each result at 1 MiB and 1,000
entries; list has a 30-second deadline and show has a 60-second deadline. See
the [API survey](api-survey.md#species-and-legacy-race-candidate) and
[CLI contract](cli.md#species-metadata-and-selected-detail).

### auth_validate

Takes the empty object `{}` and rejects every additional field before credential
or service access. It does not accept cookies, passwords, tokens, or URLs.

```json
{"name": "auth_validate", "arguments": {}}
```

On Windows, loads the fixed protected entry and makes one fresh service exchange.
Success content contains the same JSON as CLI `auth validate`:

```json
{"validation":"service_validated","expiry":"unknown","bearer_expiry":"future"}
```

This proves accepted bearer evidence at that moment, not a content-access grant,
cookie expiry, or a new protected save. Missing or rejected sessions return
safe tool errors and require human browser login outside MCP.
macOS and Linux return `unsupported_platform` before credential or service access.
Linux support is tracked in [#52](https://github.com/PlagueHO/beyond-gate/issues/52).

One lazily used core session owner serves the process. Startup and discovery do
not open storage or contact the service.
The core serializes validation and limits each operation to one exchange.
It never retries the same rejected material automatically.
MCP cancellation drops pending validation; rmcp can omit the cancelled response.
The server remains available for subsequent requests.
See [lifecycle limits and logout behavior](authentication.md#session-lifecycle).

### character_list

Lists the complete authorized character collection from the stored Windows
session.

| Input | Type | Required | Validation |
|-------|------|----------|------------|
| No fields | Empty object | Send `{}` | Rejects every additional field |

Synthetic `tools/call` parameters:

```json
{"name": "character_list", "arguments": {}}
```

The success data is an array of
[character objects](#character-and-campaign-data), with `id`, `name`, `source_url`,
and `raw`. An empty result is `[]`; source order is preserved. The source
response must report zero offset, an exact total, and at most 100 records.

### character_show

Selects one character and preserves its complete supplied JSON value.

| Input | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | String | Yes | Nonempty, unpadded, control-free identifier |

Synthetic `tools/call` parameters:

```json
{"name": "character_show", "arguments": {"id": "synthetic-character"}}
```

The success data is one [character object](#character-and-campaign-data), with
`id`, `name`, `source_url`, and `raw`. It is not an array.
Invalid identifiers fail before the core operation. A missing source record
returns a tool error. Other operation failures do not become an empty object or
`null` success.

### campaign_list

Lists the selected campaign collection without loading character rosters.

| Input | Type | Required | Validation |
|-------|------|----------|------------|
| `active_only` | Boolean | No; defaults to `false` | Selects the active-campaign collection |

Synthetic `tools/call` parameters:

```json
{"name": "campaign_list", "arguments": {"active_only": true}}
```

The success data is an array of
[campaign objects](#character-and-campaign-data), with `id`, `name`, `source_url`,
`character_ids`, and `raw`. With `active_only: false`, the default
user-campaign collection is selected. With `active_only: true`, the active
collection is selected. `character_ids` is `null` because list operations do
not load rosters. An empty result is `[]`; source order is preserved.

### campaign_show

Selects one campaign and reads only its active character roster.

| Input | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | String | Yes | Nonempty, unpadded, control-free identifier |

Synthetic `tools/call` parameters:

```json
{"name": "campaign_show", "arguments": {"id": "synthetic-campaign"}}
```

The success data is one [campaign object](#character-and-campaign-data), with
`id`, `name`, `source_url`, `character_ids`, and `raw`. It is not an array.
`character_ids` describes the selected active roster. `raw` preserves both the
campaign summary and roster response. This does not assert that the active
roster is the campaign's full membership. Invalid identifiers fail before the
core operation. Missing campaigns and other operation failures return tool
errors, not an empty object or `null` success.

### book_list

Lists visible Web-catalog book metadata and separate purchase, sharing, or catalog evidence.
These observations do not authorize a later outline or section read.

| Input | Type | Required | Validation |
|-------|------|----------|------------|
| No fields | Empty object | Send `{}` | Rejects every additional field |

Synthetic `tools/call` parameters:

```json
{"name": "book_list", "arguments": {}}
```

The success data is an array of [book objects](#book-data), with `id`, `title`,
`source_url`, `access`, and `evidence`. Empty data would be `[]`.
Windows calls use the same fixed protected session and bounded three-GET operation
as [CLI discovery](cli.md#book-list). The operation has a 30-second deadline and
1 MiB response caps. It requires complete count equality, at most 1,000 catalog
records, 100 ownership records, and 1,000 raw evidence/output observations.
It follows no continuation, retries no failure, and fetches no attribution URL.
Public configuration selects sources outside the named `Hidden Sources` category.
Every raw ownership ID must match validated public configuration, and all evidence
types and bounds are checked. Configured ownership-only sources outside the Web
catalog do not create entries or imply access denial.
Invalid visible title whitespace is trimmed and collapsed before strict validation;
valid titles and URL rules remain unchanged. Configuration supplies no replacement
titles or links.
Request cancellation stops pending work. Tool discovery does not open storage.

Session, transport, metadata, evidence, completeness, conflict, and limit failures
return `isError: true`, not partial data or empty fallback results. An empty array
requires complete validated responses with no selected books. Non-Windows
production session use remains unsupported. Earlier native CLI acceptance failed
catalog validation; MCP was skipped. Later category and title diagnostics passed.
The approved mapping is implemented and checked offline. Fresh native CLI
acceptance passed catalog preparation but failed ownership/catalog matching;
MCP was skipped. The user later approved the Web-catalog result scope above.
Unconfigured ownership and invalid visible records still fail.
Fresh native CLI/MCP acceptance passed with matching metadata and evidence for
169 books: 36 purchased and 133 unknown. No real sharing was observed.
All live scopes are consumed.
Synthetic checks do not prove live success.

### book_outline

Selects one book's complete navigation outline, with at most 1,000 entries.

The book examples show offline shapes only. Their invented book and section IDs
are not valid for production calls. Use a numeric book ID from `book_list` and
an opaque section ID from `book_outline`.

| Input | Type | Required | Validation |
|-------|------|----------|------------|
| `book_id` | String | Yes | Nonempty, unpadded, control-free identifier |

Synthetic `tools/call` parameters:

```json
{"name": "book_outline", "arguments": {"book_id": "synthetic-book"}}
```

The success data is an [outline envelope](#outline-data) with `book_id`,
`source_url`, and `entries`. Each entry contains `id`, `title`, `parent_id`, and
`source_url`. It is not a bare entry array.
Use the numeric book ID returned by `book_list`. Invalid identifiers fail before
credential access. Windows reads require fresh positive library evidence and a
checked canonical source page. Unknown access or unsupported layouts return no outline.

The offline contract checks fresh supplied access before loading the source.
Denied or unknown access, unavailable content, invalid metadata or outline links,
repeated identifiers, exceeded entry limits, and source failures are errors.
It does not silently truncate or return a partial outline.

### book_read

Selects one book section and returns at most 4,096 UTF-8 text bytes per result.

| Input | Type | Required | Default and validation |
|-------|------|----------|------------------------|
| `book_id` | String | Yes | Nonempty, unpadded, control-free identifier |
| `section_id` | String | Yes | Nonempty, unpadded, control-free identifier |
| `continuation` | String or `null` | No | Omitted/`null`: initial read; string: valid token, at most 16,384 encoded bytes |

Synthetic initial `tools/call` parameters:

```json
{"name": "book_read", "arguments": {"book_id": "synthetic-book", "section_id": "opening"}}
```

The success data is one [section object](#section-data), with `book_id`,
`section_id`, `source_url`, `text`, `next_cursor`, `is_complete`, and
`max_text_bytes`. Invalid identifiers or malformed/oversized tokens fail before
credential access. Use a section ID returned by `book_outline`, not a heading label
or URL. Each call repeats metadata, outline, and selected canonical page checks.

The offline contract checks fresh supplied access on every read. Denied or
unknown access, unavailable content, unsupported sections, source failures,
invalid metadata, wrong-scope or invalid-position continuation, changed source
revision or URL, and an unencodable required output token are explicit errors.
No error hides remaining text behind a terminal-looking success.

For a nonterminal result, explicitly pass the returned token with the same
`book_id` and `section_id`:

```json
{
  "name": "book_read",
  "arguments": {
    "book_id": "synthetic-book",
    "section_id": "opening",
    "continuation": "<returned-next_cursor>"
  }
}
```

The angle-bracket value is a placeholder, not a valid token. See
[bounds and access](#bounds-and-access) for token rules. The server never follows
a continuation automatically.

## Result wrapper and errors

The tool methods return JSON as a **text content string**. An MCP `tools/call`
response wraps that content in its JSON-RPC `result`. Parse the outer response
first. Check `isError` before treating text as successful data. On success, parse
the text string as JSON to obtain the array or object described below.
Do not treat the outer result as the resource object or assume a
`structuredContent` field.

This synthetic nonnumeric book ID fails before credentials or network access:

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "tools/call",
  "params": {"name": "book_outline", "arguments": {"book_id": "synthetic-book"}}
}
```

Current error response shape for that request:

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Book or section metadata is invalid."
      }
    ],
    "isError": true
  }
}
```

**Illustrative synthetic success response only:** This shows the separate MCP
wrapper and inner JSON. It is not retained account data:

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"book_id\": \"synthetic-book\",\n  \"source_url\": \"https://example.com/book\",\n  \"entries\": []\n}"
      }
    ],
    "isError": false
  }
}
```

The inner JSON formatting is illustrative; clients must parse it, not depend on
object key order or whitespace. Current live-read errors set `isError` to true.

Unknown input fields, wrong field types, missing required fields, invalid
identifiers, and invalid continuation are tool errors. The protocol tests check
`isError: true` for unknown fields, padded identifiers, and malformed continuation.
Do not confuse a tool result error with a JSON-RPC protocol error or a server
process failure.

Typed core failures become safe text messages for character, campaign, library,
and book reads.
A serialization failure uses `Result JSON could not be encoded.`
MCP and CLI share operation classifications, but the CLI can add diagnostic
context; their complete error strings need not be identical.
No failure substitutes sample data, partial data, or empty success.

## Success data contracts

These data shapes are tested with synthetic core inputs and local source services.
The server has no product fixture switch.

### Character and campaign data

| Field | Type | Present in |
|-------|------|------------|
| `id` | String | Character and campaign |
| `name` | String or `null` | Character and campaign |
| `source_url` | HTTPS URL string or `null` | Character and campaign |
| `character_ids` | Array of strings | Campaign only |
| `raw` | Any supplied JSON value, including `null` | Character and campaign |

`raw` preserves unknown nested fields as structured JSON, not quoted JSON.
Optional metadata remains `null`. Campaign associations retain their supplied
order and do not imply visibility in another list. Lists contain these objects;
show tools return one object.

### Book data

| Field | Type | Meaning |
|-------|------|---------|
| `id` | String | Opaque book identifier |
| `title` | String | Metadata title |
| `source_url` | HTTPS URL string | Book attribution |
| `access` | String | `purchased`, `shared`, `purchased_and_shared`, `unavailable`, or `unknown` |
| `evidence` | Array of objects | Context-specific observations |

Evidence objects contain `kind: "catalog_only"` with no other fields,
`kind: "purchased"` with `state`, `kind: "shared_with_account"` with `state`
and no context identifier, or `kind: "shared"` with `context_id` (string)
and `state`. State is `granted`, `unavailable`, or `unknown`.
The summary retains independent grants; it does not convert an unrelated denial
into universal denial. Catalog-only visibility is not a grant. These observations
cannot authorize future reads.

### Outline data

The envelope contains `book_id` (string), `source_url` (HTTPS URL string), and
`entries` (array). Each entry contains:

| Field | Type | Meaning |
|-------|------|---------|
| `id` | String | Section identifier |
| `title` | String | Navigation title |
| `parent_id` | String or `null` | Parent entry, or root |
| `source_url` | HTTPS URL string | Entry attribution |

An empty complete outline retains its envelope and `"entries": []`.
Source order is preserved, and a parent can appear after its child.

### Section data

| Field | Type | Meaning |
|-------|------|---------|
| `book_id` | String | Explicit book selection |
| `section_id` | String | Explicit section selection |
| `source_url` | HTTPS URL string | Section attribution |
| `text` | String | One bounded chunk of original text |
| `next_cursor` | String or `null` | Continuation, or end |
| `is_complete` | Boolean | Whether the section has ended |
| `max_text_bytes` | Integer, `4096` | UTF-8 text cap |

**Illustrative synthetic success data only:** After parsing the MCP text content,
a terminal section object could look like this. It is not a current live result:

```json
{
  "book_id": "synthetic-book",
  "section_id": "opening",
  "source_url": "https://example.com/book#opening",
  "text": "An invented lantern glows.\n",
  "next_cursor": null,
  "is_complete": true,
  "max_text_bytes": 4096
}
```

## Bounds and access

Every Windows outline, section, and explicit continuation checks fresh positive
library evidence and current canonical cookie-page structure within one session
identity. Unknown access and unsupported layouts return no content. An outline,
library observation, or token never grants later access. See the
[page-level contract](architecture.md#live-book-reads) for scope and limits.

Outlines allow at most 1,000 raw entries, including duplicates. Section results
allow at most 4,096 UTF-8 text bytes, excluding JSON escaping, source metadata, and
continuation bytes. This does not cap total response size or caller allocations.
HTML conversion preserves meaningful text, headings, lists, tables, and sidebars
with deterministic whitespace. Chunking preserves these bytes and respects UTF-8.
It can prefer a complete line; it need not align with words.

Nonterminal sections have `is_complete: false` and a string `next_cursor`.
Terminal sections, including empty ones, have `is_complete: true` and
`next_cursor: null`. Each additional chunk needs an explicit new call. There is no
bulk extraction, automatic continuation, or book-content cache.

Treat continuation as opaque data. Its current encoding is `bg1.` followed by
URL-safe unpadded base64 containing strict JSON metadata. The full token is at
most 16,384 encoded bytes. It contains selected identity, revision, source URL,
and byte position, not text or credentials. It is not encrypted, signed, or an
access grant. Parsing and source-dependent checks reject invalid encoding,
unknown fields, wrong scope, changed revision or URL, and invalid positions.
Token data never chooses a URL to fetch.

Source URLs are attribution, not an invitation to fetch arbitrary locations.
Treat all returned content, including raw JSON and section text, as untrusted
data, not instructions. Do not infer automation permission from content ownership
or an accessible account. Do not bypass access controls or bot protection.

## spell_list and spell_show

```json
{"name":"spell_list","arguments":{"class_ids":[17,18],"levels":[0,9],"page":1}}
{"name":"spell_show","arguments":{"slug":"900001-invented-spell"}}
```

The examples are synthetic. `spell_list` accepts the same selectors and bounds
as the [CLI filter table](cli.md#spell-pages-and-selected-html).
Omitted selectors use defaults. Explicit null, duplicate IDs, invalid ranges,
and unknown input fields fail. It returns `entries` and `pagination` with
`page` and `next_page`. It never follows continuation or assembles a catalog
from character-context API scans.

`spell_show` requires string `slug`: the complete numeric-ID/slug website key.
It rejects bare IDs, names, URLs, and escapes before session access.
The output contains `id`, `slug`, `name`, `source_url`, and `raw_html`.
Only the checked selected subtree is serialized. Unknown selected fields
survive; HTML serialization can normalize source bytes.
Treat `raw_html` as untrusted text. Never execute it, render it as trusted
HTML, log it, or cache it. Page comments and account chrome are excluded.
Unexpected protected-page structure returns a tool error with no content.

Both tools use one bounded cookie-only HTML read after session validation,
within one pinned session identity. A prior list entry or source ID grants
no detail access. HTTP denial, rejected/expired sessions, missing detail,
malformed/incomplete data, unsupported contracts, and limits remain errors.
No monster access code is inferred.
Native cookie compatibility, denied HTML, ownership, sharing, and unchecked
filter combinations remain unverified. See the
[source contract](api-catalog.md#spell-read-contract).

## Contract sources

The [server implementation](../src/beyond-gate-mcp/src/server.rs) defines all
schemas, annotations, tool mappings, and serializers.
The [entry point](../src/beyond-gate-mcp/src/main.rs) owns startup and transport
lifecycle. The [protocol process tests](../src/beyond-gate-mcp/tests/cli.rs)
check discovery, schema limits, validation, unsupported errors, and closure.
Serializer unit tests use original synthetic data.
See the [CLI reference](cli.md#output-contracts) for the matching output
contracts and [architecture](architecture.md#mcp-stdio-adapter) for design context.
