# CLI command reference

**Current status:** `beyond-gate` supports help, version output, shell completions,
local session status and logout, Windows session validation, and local MCP start and registration. Windows
login, explicit browser setup, and the native host are implemented.
Approved Chrome and Edge Dev login/save/restart/logout checks passed.
Windows `book list` has bounded live metadata transport through the shared stored session.
Web-catalog scope, category selection, and title cleanup are implemented and verified natively.
Windows `book outline/read` now use fresh library evidence and checked canonical
cookie pages. Their production source path has synthetic process checks.
Native adapter acceptance passed for one selected chapter with matching outlines
and initial chunks. Live continuation and sharing remain unchecked.
Character and campaign reads use fixed, bounded operations through the stored
Windows session. Synthetic process checks cover successful reads and matching
CLI/MCP data. No live character or campaign acceptance has been run.
Class and subclass commands return metadata only through bounded Windows
catalog reads. Browser metadata contracts were inspected under separate
approvals. Native class/subclass acceptance has not been run.
Monster commands return one explicit page or unchanged selected detail, only
for observed access code 1. Code 4 is denied; unverified codes, including 2 and
3, are unsupported. Browser route probes passed; native acceptance has not run.

See [native installation](installation.md) for binary setup and
[authentication](authentication.md#local-session-commands) for protected storage
requirements. See the [MCP reference](mcp.md) for matching tools and stdio usage.

## Syntax, help, and version

```text
beyond-gate <command> [arguments] [options]
beyond-gate --help
beyond-gate --version
beyond-gate help <command> [subcommand]
beyond-gate <command> --help
beyond-gate <command> <subcommand> --help
```

`-h` and `--help` show help. `-V` and `--version` show the root executable version.
Version is a root option, not an option on every subcommand. Help and version
requests write to stdout and exit with code 0. No credentials or network are used.
The generated `help` command also supports nested paths, such as
`beyond-gate help book read`.

No command, a resource without its action, an unknown option, or invalid arguments
produce usage information on stderr and exit with code 2. A resource is `auth`,
`character`, `campaign`, `class`, `subclass`, `species`, `spell`, `monster`,
`background`, `feat`, `equipment`, `book`, or `mcp`.

In the syntax below, angle brackets mark required values. Square brackets mark
optional values. Replace placeholders before use; do not type the brackets.
Identifiers are opaque strings, not necessarily numbers. Empty identifiers,
leading or trailing whitespace, and control characters are rejected before the
operation. The CLI does not infer a source URL from an identifier.

## Command index

| Command | Purpose | Current behavior |
|---------|---------|------------------|
| [`auth login`](#auth-login) | Complete human-controlled login | Windows; approved Chrome and Edge Dev checks passed |
| [`auth install`](#auth-install-and-auth-uninstall) | Register a native browser host | Windows; explicit directory, preview, and confirmation |
| [`auth uninstall`](#auth-install-and-auth-uninstall) | Remove matching native setup | Windows; preserves other installations |
| [`auth status`](#auth-status) | Inspect local credential presence | Implemented; no upstream validation |
| [`auth validate`](#auth-validate) | Make one fresh session check | Windows; safe JSON, no credential input |
| [`auth logout`](#auth-logout) | Remove the local credential | Implemented; no upstream revocation |
| [`character list`](#character-list) | List character summaries | Windows; bounded complete response |
| [`character show`](#character-show) | Select one character | Windows; fixed source route |
| [`campaign list`](#campaign-list) | List campaigns and associations | Windows; `--active-only` selects the active collection |
| [`campaign show`](#campaign-show) | Select one campaign and active roster | Windows; selected roster only |
| [`class list/show`](#class-and-subclass-metadata) | Browse class identity and attribution | Windows; metadata only |
| [`subclass list/show`](#class-and-subclass-metadata) | Browse subclasses with parent context | Windows; scoped or bounded global metadata |
| [`species list/show`](#species-metadata-and-selected-detail) | Browse website metadata and selected raw detail | Windows; detail route is a candidate; live access unverified |
| [`monster list/show`](#monster-pages-and-detail) | Read one page or selected raw detail | Windows; verified access code 1 only |
| [`background list/show`](#background-metadata-and-selected-detail) | Browse metadata or select one raw definition | Windows; complete bounded source reads; live acceptance unverified |
| [`feat list/show`](#feat-metadata-and-detail-status) | Browse feat metadata or select one raw definition | Windows; bounded protected collection |
| [`equipment list/show`](#equipment-source-pages-and-raw-detail) | Read equipment from one mixed source page | Windows; explicit page and full definition key |
| [`spell list/show`](#spell-pages-and-selected-html) | Browse one explicit page or selected HTML detail | Windows; browser structure checked; native acceptance unverified |
| [`book list`](#book-list) | List Web-catalog book metadata and access evidence | Windows discovery verified |
| [`book outline`](#book-outline) | Read complete bounded navigation | Windows checked source pages |
| [`book read`](#book-read) | Read one bounded section chunk | Windows checked source pages |
| [`mcp start`](#mcp-start) | Start the sibling stdio server | Implemented |
| [`mcp install`](#mcp-install) | Register the server with a local client | Implemented; confirms before writing |
| [`completions`](#completions) | Generate a shell completion script | Implemented |

## Equipment source pages and raw detail

```text
beyond-gate equipment list [--page <PAGE>] [--output <FORMAT>] [--no-headers]
beyond-gate equipment show <id> --page <PAGE> [--output <FORMAT>] [--no-headers]
```

List defaults to page 1 and table output. Show requires a page and defaults
to JSON. Use `--output json` to retain pagination even when no equipment rows
exist. Table and TSV output add page/count fields to each returned row;
an empty table has no rows in which to show continuation. `--format` and
`-o` are aliases for `--output`. JSON rejects `--no-headers`.

Pages are one-based integers from 1 through 1,000,000. Each operation reads
fresh public configuration and exactly one bearer-only item page, with at
most one session exchange. The fixed API query is
`sharingSetting=2&page=<PAGE-1>&pageSize=100`. No cookies go to the item
service. No extra page is fetched to fill equipment results.
Website filters, `skip`, `take`, `page_size`, and global search are not inputs.

Only raw boolean `magic=false` counts as equipment. `magic=true` counts as a
magic item, even in Weapon or Armor categories. Rarity and derived character
inventory flags do not change classification. Every row must pass identity,
classification, metadata, and configured-source checks before output.
One invalid unselected row fails the whole page.

| List field | Meaning |
|------------|---------|
| `entries` | Equipment metadata from this mixed page, in source order |
| `entries[].id` | Full `entityTypeId:id` definition key |
| `entries[].source_id`, `entity_type_id` | Raw item ID and entity type, not book source IDs |
| `entries[].name`, `filter_type`, `cost`, `weight` | Source metadata; cost can be null |
| `entries[].sources` | Unchanged attribution references, including extensions |
| `source_url`, `entries[].source_url` | Fixed item API URL with the checked page |
| `pagination.page`, `next_page` | Checked page and explicit next page, or null |
| `pagination.source_page_size` | Fixed mixed-source page size of 100 |
| `pagination.source_count`, `source_total` | Raw mixed-page count and mixed-catalog total, not equipment totals |

A magic-only page can have empty `entries` and a non-null `next_page`.
A terminal or beyond-terminal empty page requires consistent numeric
pagination. The CLI never follows continuation automatically.

Show copies the full `id` from list and requires that list's page. Both key
components must be canonical positive decimal `u64` values. Bare numeric IDs,
leading zeroes, URLs, and padded keys fail before session access.
Show validates the entire fresh page before selecting one record. It returns
`id`, `source_id`, `entity_type_id`, `name`, `page`, `source_url`, `sources`,
and unchanged `raw` JSON. Unknown nested fields survive. No rules engine
interprets the record.

Missing-record errors mean absent from the checked page, not absent globally.
A selected magic record gives a wrong-kind error. Denied, rejected, expired,
malformed, inconsistent, duplicate, unsupported, and over-limit responses
give no partial output. Unverified access maps are unsupported, not interpreted
with monster access codes. Attribution and metadata are not access grants.

Limits are 100 raw rows, 1,000 references per row, 1 MiB per HTTP body and
compact result, and a 60-second operation deadline. Padded table output has
an 8 MiB limit. No retry, arbitrary URL, global lookup, HTML fallback, cache,
or account write is added.
Browser research verified paging and one repeat raw identity. Native CLI/MCP
compatibility, account acceptance, and real owned/shared behavior remain
unverified. See the [source evidence](api-survey.md#equipment-and-magic-item-research)
and [matching MCP tools](mcp.md#equipment_list-and-equipment_show).

## Feat metadata and detail status

```text
beyond-gate feat list --output json
beyond-gate feat list --name invented --partnered-content any --format tsv
beyond-gate feat list --prerequisite-subtype-id <option-id> --tag-id <option-id>
beyond-gate feat show <id>
```

List reads the fixed `/feats` website GET form through the stored Windows
session. It returns `id`, `name`, `source`, and `source_url` in website order,
not descriptions, snippets, prerequisites text, or rules. Duplicate names
remain separate when their IDs differ. Returned links never select a fetch.

| Option | Shared input and limit |
|--------|------------------------|
| `--name` / `-n` | Literal unpadded text, at most 256 UTF-8 bytes |
| `--prerequisite-subtype-id` | One positive `u32` website option ID |
| `--tag-id` | Up to 50 distinct positive website tag IDs |
| `--source-id` | Up to 50 distinct positive website source IDs |
| `--source-category-id` | Up to 50 distinct positive website category IDs |
| `--partnered-content` | `yes`, `no`, or `any`; default `no` |

Repeat only the three multi-select ID flags. The prerequisite control is
single-select; its IDs are not feat record IDs. IDs must exist in the same
response's form, and requested values must be echoed. None of these inputs
is an access grant. There is no page, skip, take, or automatic continuation.
An empty result requires the observed explicit marker. Missing structure,
unknown paging, conflicting IDs, and incomplete results fail without output.
List is bounded to 1,000 rows, 1 MiB response/result, and 30 seconds.

Show validates a canonical positive decimal `u64` ID, reads public source
configuration, then makes one fixed bearer GET to the checked feat collection.
It validates the terminal envelope and every row before returning one selected
definition with `id`, `name`, `source_url`, `sources`, `is_repeatable`,
`repeatable_parent_id`, and unchanged `raw`. Unknown source extensions,
prerequisites, and child fields survive. A missing selected ID means that it is
absent from this checked collection. No ownership or sharing is inferred.

The collection and selected result are limited to 1,000 definitions/references
and 1 MiB; show has a 60-second deadline. An oversized collection fails even
when the selected definition would fit. No selected-ID request, parent
traversal, grouping, eligibility calculation, rules engine, cache, or
metadata-only fallback is added. Invalid IDs exit with code 2.

See the [MCP tools](mcp.md#feat_list-and-feat_show) and
[source contract](api-catalog.md#feat-browse-and-detail-status).
The approved structural pass confirmed the collection shape and size.
Native CLI/MCP acceptance and broader owned/shared coverage remain unverified.

## Background metadata and selected detail

```text
beyond-gate background list --output json
beyond-gate background list -n acolyte --skill-id 240 --skill-id 241 --partnered-content any -o tsv
beyond-gate background show <id>
```

List reads one filtered website result. Show selects one raw source definition
from a fresh complete authenticated collection, not a guessed detail route.
Both use the shared Windows session and fail without partial output.

| CLI option | MCP field | Website control |
|------------|-----------|-----------------|
| `--name`, `-n` | `name` | `filter-name` |
| Repeated `--skill-id` | `skill_ids` | `filter-skills` |
| Repeated `--tag-id` | `tag_ids` | `filter-tags` |
| Repeated `--source-id` | `source_ids` | `filter-source` |
| Repeated `--source-category-id` | `source_category_ids` | `filter-source-category` |
| `--partnered-content yes/no/any` | `partnered_content` | `filter-partnered-content` |

Name is literal, nonempty, unpadded, control-free text within 256 UTF-8 bytes.
Each ID filter permits at most 50 unique positive `u32` values. IDs must be
options in the returned website form, not assumed API IDs or access grants.
Partnered content defaults to `no`, as on the website. `any` leaves this filter
unspecified. There is no `page`, `skip`, or `take`; paging was not verified.
The encoded query is at most 16 KiB.

List JSON is an array of metadata objects with `id`, `name`, `source`, and
`source_url`. `source` is the displayed source label, not ownership evidence.
Duplicate names survive. Duplicate IDs fail. Empty results require the known
website marker. Missing structure, challenge pages, and paging fail.
No rules text or per-row detail request is part of list.

Show requires one canonical positive decimal ID that fits `u64`, without
leading zeroes. It reads public configuration, then the fixed authenticated
background collection with `sharingSetting=2`. It checks every record and
source reference before selection. JSON contains `id`, `name`, `source_url`,
`sources`, and the unchanged selected `raw` definition. Unknown raw fields
and source-reference extensions survive. A missing ID means absent from that
complete checked collection. Website and API coverage are not assumed equal.
No full website page or source-book text coverage is claimed.

Both actions support `--output` / `-o` with `json`, `table`, or `tsv` and
`--format` as an alias. List defaults to table; show defaults to JSON.
`--no-headers` works only with table/TSV. Columns use the JSON field order
above; structured show cells use compact JSON with the shared escaping rules.
Padded tables are preflighted against 8 MiB before output.

Local bounds are 1,000 raw rows/definitions, 1,000 references per definition,
1 MiB per response and compact result, and 30/60 seconds for list/show.
**The observed live collection was 1,160,568 bytes. Show rejects that response
with `BodyTooLarge` under the unchanged 1 MiB cap.** It does not truncate,
increase limits, cache content, or invent a server-side ID filter.
Denial, rejected/expired sessions, missing records, invalid input, malformed
data, incomplete results, unsupported operations, and limits remain distinct.
Local input errors exit 2; source/read errors exit 1. Errors leave stdout empty.
Ctrl-C cancels the shared read.

The first-party reader returns definitions directly without the monster access
map. No numeric access code or ownership claim is inferred. Unexpected access
fields are unsupported. Native compatibility, native account acceptance, and
real sharing remain unverified. See the [source contract](api-catalog.md#background-read-contract)
and [matching MCP tools](mcp.md#background_list-and-background_show).

## Monster pages and detail

```text
beyond-gate monster list
beyond-gate monster list --name wolf --cr-max 2 --size-id 4 --size-id 5 --output json
beyond-gate monster list -n wolf --cr-max 2 --size-id 4 --size-id 5 --page 2 --format tsv
beyond-gate monster show <id>
```

`list` reads one website-filtered page using the stored Windows session cookie.
For a nonempty page, it makes one selected-ID monster-service request and checks
fresh access for every record before output. It does not filter an already-fetched
page or traverse the catalog. A required session exchange can precede these reads.
`--name` / `-n` is a literal server-side search, not local filtering.
Names permit at most 256 UTF-8 bytes without padding or control characters.

`--page` is a positive `u32`, default 1. It replaces `--skip` and `--take`,
which are no longer accepted. The website controls page size; the browser
check observed 20 rows per full page. The local ceiling is 100 records,
not a verified upstream maximum.

| CLI options | Matching MCP fields | Values |
|-------------|---------------------|--------|
| `--name`, `-n`; `--page` | `name`; `page` | Literal text; positive page |
| `--cr-min`, `--cr-max` | `cr_min`, `cr_max` | Literal `0`, `1/8`, `1/4`, `1/2`, or integer 1 through 30 |
| `--type-id`, `--subtype-id` | `type_ids`, `subtype_ids` | Public website option IDs |
| `--size-id`, `--environment-id`, `--alignment-id` | `size_ids`, `environment_ids`, `alignment_ids` | Public website option IDs |
| `--sense-id`, `--save-proficiency-id`, `--skill-proficiency-id` | `sense_ids`, `save_proficiency_ids`, `skill_proficiency_ids` | Public website option IDs |
| `--resistance-id`, `--damage-immunity-id`, `--condition-immunity-id`, `--vulnerability-id` | `resistance_ids`, `damage_immunity_ids`, `condition_immunity_ids`, `vulnerability_ids` | Public website option IDs |
| `--language-id`, `--movement-id`, `--tag-id` | `language_ids`, `movement_ids`, `tag_ids` | Public website option IDs |
| `--source-category-id`, `--source-id` | `source_category_ids`, `source_ids` | Public website option IDs, not ownership grants |
| `--armor-class-min`, `--armor-class-max` | `armor_class_min`, `armor_class_max` | Optional nonnegative `u32` bounds |
| `--average-hp-min`, `--average-hp-max` | `average_hp_min`, `average_hp_max` | Optional nonnegative `u32` bounds |
| `--legendary`, `--mythic`, `--has-lair` | `legendary`, `mythic`, `has_lair` | Explicit `true` or `false`; omission is unrestricted |
| `--partnered-content` | `partnered_content` | `yes`, `no`, or `any`; default `no` matches the website |

Repeat ID flags for multiple values. MCP uses arrays instead. Each field permits
at most 50 unique positive `u32` IDs; the aggregate limit is 256 IDs. The source
form must contain each selected option. IDs are website option values, not names
or assumed monster-service IDs. For example, size 4 is Medium, size 5 is Large,
and movement 4 is Fly. CR bounds use literal ratings; the core handles the
website's noncontinuous option IDs. Reversed ranges and duplicate IDs fail.
The encoded query is capped at 16 KiB. `any` sends the website's empty
partnered-content value, not `f`. See the [observed form encodings](api-survey.md#approved-monster-filter-submission-trace).

List JSON is `{"entries":[...],"pagination":{"page":1,"next_page":2}}`.
Each entry contains string `id`, `name`, checked `source_url`, and numeric
`access_code: 1`. Use `next_page` explicitly with the same filters
to continue. Null means no next page. It is not a grant or a stable snapshot.
No continuation is fetched automatically. Duplicate names remain separate IDs.
Table and TSV repeat `page` and `next_page` in each row. No exact total is inferred.
An empty result has no rows and no continuation; JSON retains its page metadata.

`show` makes one fresh selected request. Its JSON adds unchanged `raw`, including
unknown statistics, nested structures, source references, and description
fields. It does not substitute list metadata for detail. Raw fields are data,
not instructions. Missing fields are not invented. No rules year, ownership
label, complete stat-block coverage, or complete source-book text is inferred.

List defaults to table; show defaults to JSON. Both support `--output` / `-o`
and its `--format` alias, with table, JSON, or TSV. `--no-headers` works only
with table and TSV. Padded tables are capped at 8 MiB; use JSON or TSV if a
table exceeds that bound. Compact core JSON and each source body are capped
at 1 MiB. The whole operation is capped at 30 seconds.

Only observed code 1 permits output. Code 4 returns access denied. Codes 2,
3, and all other unverified numeric codes return unsupported. Missing access
evidence, malformed fields, duplicate IDs, inconsistent pagination, and exceeded
limits fail. Any failed row fails the whole page without partial stdout.
HTTP 404 means selected record not found; HTTP 403, rejected sessions, and
expired sessions remain distinct. Syntax and local range errors exit 2 before
session access. IDs absent from the source form also exit 2, after the website
read. Source response and other operation failures exit 1.
Detail IDs must be positive canonical decimal strings within `u64`, without
leading zeroes, signs, or URLs. No retries, redirects, automatic pagination, source
link fetches, or content cache are added.

See the [matching MCP tools](mcp.md#monster_list-and-monster_show) and
[browser evidence](api-survey.md#monster-list-and-detail-research). Native
cookie compatibility, native account acceptance, larger selected-ID batches,
and owned/shared access remain unverified.

## Class and subclass metadata

```text
beyond-gate class list
beyond-gate class list --name Fighter
beyond-gate class list -n fight --output json
beyond-gate class show 2190879
beyond-gate class show 10
beyond-gate subclass list
beyond-gate subclass list --name Champion
beyond-gate subclass list --class-id 2190879
beyond-gate subclass list --class-id 10 --name Champion --output json
beyond-gate subclass show 7001 --class-id 2190879
beyond-gate subclass show 7001
```

The two Fighter IDs were observed. Subclass `7001` and Champion are illustrative,
not retained live subclass observations. Names find candidates; IDs select records.
`--name` / `-n` uses local case-insensitive substring matching. `fight` and `ght`
both match Fighter. Keep duplicate names. There is no query language, wildcard,
fuzzy search, remote name filter, or implicit current/legacy preference.
Filters must be nonempty, unpadded, control-free, and within 256 UTF-8 bytes.
Class IDs must be positive canonical decimal integers that fit `u64`.

Lists default to table. Show defaults to JSON. Both support `--output` / `-o`,
the `--format` alias, and `table`, `json`, or `tsv`. `--no-headers` is invalid
with JSON. Table/TSV uses identity, name, parent, and source-ID columns.
JSON lists contain `kind`, `id`, `name`, `parent_class_id`, and `source_ids`.
JSON show also contains `is_homebrew`, `slug`, `more_details_url`, and `sources`.
Optional metadata is null when absent. Invalid supplied metadata fails.
The [MCP reference](mcp.md#class_list-and-class_show) gives the shared JSON shape.

Class/subclass tables are limited to 8 MiB (8,388,608 bytes). Before allocating
padded output, the CLI counts escaped UTF-8 cells, character-based column padding,
separators, newlines, and headers when selected. A table above this limit fails
with exit code 1, an explicit stderr diagnostic, and empty stdout. Metadata is
never truncated. Use JSON or TSV for a bounded result that exceeds the table
limit; these formats do not add column padding and keep their existing behavior.

Show is metadata-only. It does not return descriptions, feature text, equipment
text, spell rules, or raw definitions. Unknown source-reference metadata is
preserved in `sources`; arbitrary unknown definition fields are excluded.
This intentionally narrows lossless raw-definition detail to metadata.
Source IDs, names, and `is_homebrew` do not establish year, legacy/current state,
ownership, sharing, or permission to read rules text.

Only subclasses accept `--class-id`. Without it, discovery reads every checked
parent sequentially. Equal subclass IDs under different parents remain distinct.
An unscoped ambiguous show fails and requires a parent. A missing selected parent
fails before any subclass request. No direct detail route or metadata link is fetched.
Results sort by lowercase name, original name, numeric parent, then numeric ID.

Each operation checks public source configuration and fresh protected catalogs.
The global parent cap is 32. The combined raw class/subclass cap is 1,000 before
filtering. Each response is at most 1 MiB; all resource bodies, including
configuration, total at most 8 MiB. Compact canonical JSON results are at most
1 MiB before formatting. The whole operation is at most 120 seconds, including
session waits. There are no retries, redirects, or automatic continuation.
Class/scoped operations can accept more than 32 roots within the raw-record cap.
Conservative maximum explicit HTTP counts, including possible bearer exchanges,
are 3 for class, 5 for scoped subclass, and 67 for global subclass operations.

Empty lists require complete successful discovery. Missing IDs mean absent from
the checked default catalog scope, not globally nonexistent. Denial, session
expiry/rejection, invalid metadata, incomplete discovery, ambiguity, and exceeded
bounds fail without partial results. Linux/macOS production session use remains
unsupported. Native compatibility, live global traversal, and real sharing are
unverified. More live account tests require separate approval.

## Species metadata and selected detail

```text
beyond-gate species list
beyond-gate species list --output json
beyond-gate species show 17
```

List returns an array with `id`, `name`, `source`, `source_url`, and
`is_legacy`. It parses the fixed public page. The site search is client-side,
so no search filter or paging input is inferred. Show defaults to JSON and
adds `raw`, which preserves the selected source record and unknown fields.
IDs are positive canonical decimal integers that fit `u64`. List defaults to
a table. Both commands support `--output` / `-o`, the `--format` alias, and
`table`, `json`, or `tsv`.

Show reads the fixed `/character/v5/game-data/races` collection once and
selects an ID from that fresh response. It does not request the displayed
website link. The response must have a successful character-service envelope,
an array `data`, and null `pagination`. Missing or malformed page structure,
duplicate IDs, denied reads, unsupported paging, and over-limit results fail
without partial output. The list has a 1,000-entry, 1 MiB, and 30-second limit.
Show has a 1,000-record, 1 MiB response/result, and 60-second limit.

Browser evidence confirms the public page structure, not detail access. The
candidate detail route still lacks live response and access evidence. See the
[API survey](api-survey.md#species-and-legacy-race-candidate) and
[matching MCP tools](mcp.md#species_list-and-species_show).

## Authentication commands

`auth login`, `auth status`, `auth validate`, and `auth logout` have no positional arguments or
action-specific options.
They do not accept passwords, cookies, tokens, or account identities.

### auth login

```shell
beyond-gate auth login
```

On Windows, all three standard streams must be interactive terminals. Redirected
streams fail before runtime, key generation, endpoint creation, network, or storage.
The command reserves one logon-local pending slot and displays a one-use key on
an owned alternate screen. The pending lifetime is at most five minutes.
Ctrl+C cancels the operation. A second pending login fails with `busy`.

An authorized browser host must prove key possession before cookie authorization.
The terminal display closes after pairing. The shared core then validates the
production-bound cookie and confirms a protected save. Only final browser receipt
confirmation permits a successful command result on stdout. Failure uses stderr
and exit code 1; an unconfirmed started save reports `save_indeterminate`.
Success uses exit code 0 and the fixed text contract:

```text
saved: Beyond Gate session; validation=service_validated; expiry=unknown.
```

The native executable is `beyond-gate-native-host.exe`.
Install it and the manual browser package through the
[explicit setup procedure](installation.md).
The command does not open a website or install components automatically.
Approved real Chrome and Edge Dev login, protected save, restart/status, and
logout checks passed. Browser versions and remaining limits are in the
[verification record](testing.md#approved-windows-product-login-verification).
See the [Windows runtime contract](browser-handoff.md#windows-runtime-integration).

macOS and Linux retain the unsupported-login error. No credential argument,
password prompt, cookie import, environment ingress, or MCP input is added.
The developer authentication probe is not product login.

### auth install and auth uninstall

Windows host setup requires an explicit stable local installation directory.
That directory must be outside source and `target` trees. Installation requires
`beyond-gate.exe` and `beyond-gate-native-host.exe` there. Windows test archives
include the native host and the separate `beyond-gate-browser.zip`.

Preview the selected browser without writing:

```powershell
beyond-gate auth install edge --directory C:\Tools\BeyondGate --dry-run
beyond-gate auth install chrome --directory C:\Tools\BeyondGate --dry-run
```

Remove `--dry-run` to request confirmation. The command writes a browser-specific
public JSON manifest in that directory. It registers the exact pinned extension
origin in the selected browser's per-user native-host key, in the 32-bit view.
The registry leaf and its default value are created in one Windows transaction.
An existing leaf must match; the command does not overwrite it.

Conflict checks cover user and machine keys in both registry views. Edge checks
its Chromium and Chrome fallback locations too. A foreign path, malformed
manifest, extra registry value, or child key stops setup. Machine-wide and
fallback keys are not modified. A failed operation can leave public setup files
or registrations; inspect the previewed locations before retrying.

Preview removal with the original installation directory:

```powershell
beyond-gate auth uninstall edge --directory C:\Tools\BeyondGate --dry-run
beyond-gate auth uninstall chrome --directory C:\Tools\BeyondGate --dry-run
```

Removal requires confirmation. It removes only matching selected-browser
per-user leaves. Ownership checks and registry deletion share one Windows
transaction. It preserves binaries, other browsers, and machine-wide keys.
It retains a manifest if another checked registration still uses it. Edge can
still use a Chrome fallback after its explicit registration is removed.
A moved installation requires removal with its old directory before registration
of the new one.

Registry entries, manifests, and launch mapping are
[trusted installed configuration](authentication.md#browser-login-window).
Setup does not read cookies, make account requests, or access the credential
store. Neither setup nor a preview proves successful browser login.
macOS and Linux setup fail explicitly before filesystem or registry access.

### auth status

```shell
beyond-gate auth status
```

Inspects only Beyond Gate's protected local credential entry. On success it
writes one text line to stdout and exits with code 0:

| State prefix | Meaning |
|--------------|---------|
| `absent:` | No local Beyond Gate session is stored |
| `stored-unverified:` | Local session material exists; upstream usability is not checked and expiry is unknown |

Status does not modify the credential. A locked store or other storage failure
is an error, not an absent session. Storage errors exit with code 1 and use stderr.
This command can trigger an OS access or unlock dialog.

### auth validate

```shell
beyond-gate auth validate
```

On Windows, reads the fixed protected entry and makes one fresh exchange
through the shared core. No credential, URL, or account argument is accepted.
Success writes this JSON object on stdout and exits with code 0:

```json
{"validation":"service_validated","expiry":"unknown","bearer_expiry":"future"}
```

Stored presence or earlier cached evidence cannot substitute for fresh validation.
The result does not establish cookie expiry, content access, or a protected save.
Errors use safe shared categories on stderr with exit code 1 and no result on
stdout. Missing or rejected sessions require human `auth login`.
Ctrl+C cancels pending validation. macOS and Linux return `unsupported_platform`
before credential or service access. Linux support is tracked in
[#52](https://github.com/PlagueHO/beyond-gate/issues/52).
See [lifecycle bounds and failures](authentication.md#session-lifecycle).

### auth logout

**Warning:** This command deletes Beyond Gate's local credential entry.

```shell
beyond-gate auth logout
```

Deletes only that entry, without first reading its secret. An already absent entry
is also success. It writes one text line to stdout and exits with code 0 after
successful deletion or absence. The line states that upstream revocation is not
implemented and the browser session can remain active.

Logout clears its core owner's cached evidence. Other processes detect deletion
at their next protected load; an in-flight request is not recalled.
A started native deletion can finish after a timeout or caller cancellation.
Logout does not log the browser out or revoke the upstream session. It can remove
invalid stored local data. Storage failures exit with code 1 and use stderr.
This command can trigger an OS access or unlock dialog.

Status and logout use Windows Credential Manager, macOS login Keychain, or Linux
Secret Service. No plaintext fallback exists. See
[protected-store selection and recovery](authentication.md#protected-store-selection-and-recovery)
for the fixed entry identity, backend requirements, and failure recovery.

## Resource commands

All resource reads use fixed origins and the protected stored session.
Book outline/read also repeat fresh library and page-level access checks.
Character and campaign success outputs are offline-tested. No live character
or campaign acceptance has been run.

### List options

`character list`, `character show`, `campaign list`, `campaign show`,
`book list`, and `book outline` share these options:

| Option | Values | Default | Effect |
|--------|--------|---------|--------|
| `-o, --output <FORMAT>` | `table`, `json`, `tsv` | `table` for lists; `json` for shows | Select result presentation |
| `--format <FORMAT>` | `table`, `json`, `tsv` | Same as `--output` | Alias for `--output` |
| `--no-headers` | Flag; no value | Headers included | Omit table or TSV column names |

`--no-headers` with JSON is invalid usage, with exit code 2.
There are no account, URL, or fixture options.

### character list

```text
beyond-gate character list [-o, --output <FORMAT>] [--no-headers]
```

Lists character summaries. No positional arguments are required.
Table and TSV columns are `id`, `name`, and `source_url`, in that order.
JSON is an array of [character objects](#character-and-campaign-objects), including
the structured `raw` field. Source order is preserved.

The operation requests the account's default character collection. It accepts
only a complete response with zero offset, a total matching the returned count,
and no more than 100 records. It does not infer or follow pagination.
Invalid options fail with code 2. Session, transport, response, completeness,
and limit failures use code 1 with empty stdout.

### character show

```text
beyond-gate character show <id> [-o, --output <FORMAT>]
```

Requires one character identifier and accepts `table`, `json`, or `tsv` output.
The default is JSON. The success contract is one
[character object](#character-and-campaign-objects), not an array.
It preserves the supplied data without interpreting game rules. A missing
source record returns an explicit error.

Invalid identifiers fail with code 2. Session, transport, and response failures
use code 1.

### campaign list

```text
beyond-gate campaign list [--active-only] [-o, --output <FORMAT>] [--no-headers]
```

Lists campaigns without loading rosters. `--active-only` selects the active
campaign collection directly; without it, the default user-campaign collection
is used. No positional arguments are required. Table and TSV columns are
`id`, `name`, `source_url`, and
`character_ids`, in that order. JSON is an array of
[campaign objects](#character-and-campaign-objects), including `raw`.
The `character_ids` value is `null` because the list does not load a roster.

The source response is bounded to 100 records. Invalid options fail with code 2.
Session, transport, malformed-response, and limit failures use code 1 without a
partial successful list.

### campaign show

```text
beyond-gate campaign show <id> [-o, --output <FORMAT>]
```

Requires one campaign identifier and accepts `table`, `json`, or `tsv` output.
The default is JSON. The operation reads the default user-campaign collection,
then requests only the selected campaign's active roster. The success contract
is one
[campaign object](#character-and-campaign-objects), not an array.
The `character_ids` field describes that active roster. `raw` preserves both
the campaign summary and roster response; this does not assert that the roster
is the full campaign membership.

Invalid identifiers fail with code 2. A campaign absent from the returned
collection is an explicit not-found error. Session, transport, and response
failures use code 1.

### book list

```text
beyond-gate book list [--format <FORMAT>] [--no-headers]
```

Lists Web-catalog book metadata and separate access observations. No positional arguments are
required. Table and TSV columns are `id`, `title`, `access`, and `source_url`, in
that order. JSON is an array of [book objects](#book-objects), including `evidence`.
Catalog visibility and prior purchase or sharing observations do not grant access
to a later read.

Windows discovery uses fixed public configuration, source, and ownership GETs without retries or
automatic continuation. The complete operation has a 30-second deadline. Each
response has a 1 MiB cap. The unpaged catalog has a 1,000-record cap; ownership
has a 100-record cap. Raw evidence and output observations each have a 1,000-entry
cap before duplicate removal. `totalCount` must equal the returned collection
size. Known continuation markers, malformed data, conflicting metadata,
ownership IDs absent from public configuration, or exceeded limits fail the entire operation.

Public configuration excludes only the named `Hidden Sources` category.
All raw ownership IDs must match validated public configuration. Configured
ownership-only sources outside the Web catalog do not create entries.
They are not reported as unavailable. Unknown evidence on any source still fails.
Invalid visible title
whitespace is trimmed and collapsed; valid titles remain unchanged.
Empty titles, non-whitespace controls, and invalid links still fail.
Configuration titles and links are not fallback metadata.

Purchase comes only from `ownership[].type: "owned"`; account sharing comes from
`"shared"`. Free and catalog flags are not grants. Missing or empty ownership
remains `unknown`, not `unavailable`. Source attribution uses a validated book
path, or a supplied validated D&D Beyond marketplace URL when the path is absent.
Metadata links are never fetched.

Success returns complete data with code 0. Discovery and session failures use
code 1 and empty stdout. Invalid options use code 2. Ctrl+C cancels the operation.
Non-Windows production session use remains unsupported.
Earlier native CLI acceptance failed `InvalidMetadata`; MCP was skipped.
Later diagnostics validated category selection and title cleanup without broader
URL rules. That mapping is now implemented and checked offline. Fresh native CLI
acceptance passed catalog preparation but failed `UnmatchedOwnership`: ownership
referenced a source absent from the full raw catalog. MCP was skipped.
The user then approved Web-scoped results after a diagnostic classified seven
ownership-only IDs, all present in configuration. Fresh native acceptance passed:
CLI and MCP returned matching metadata and evidence for 169 books, including
36 purchased and 133 unknown. All live scopes are consumed.
Real shared-access observations remain unverified.

### book outline

```text
beyond-gate book outline <book-id> [--format <FORMAT>] [--no-headers]
```

Requires one book identifier. The success contract is a complete outline with at
most 1,000 entries. Table and TSV columns are `id`, `title`, `parent_id`, and
`source_url`, in that order. JSON uses an
[outline envelope](#outline-and-section-objects), not a bare entry array.

Use the numeric book ID returned by `book list`. Windows reads require fresh
positive library evidence and one checked canonical source page. Unknown access,
unsafe links, unsupported layouts, and malformed sources return no outline.
At most 1,000 raw anchors are allowed before filtering or duplicate-role handling.
The result preserves nested list hierarchy; fragment links otherwise belong to
their chapter. Repeated navigation targets have distinct stable opaque IDs.
Invalid parser input or list options use code 2. Invalid numeric IDs and shared
operation failures use code 1. See [book bounds and continuation](#book-bounds-and-continuation).

### book read

```text
beyond-gate book read <book-id> <section-id> [--continuation <TOKEN>]
```

Requires explicit book and section identifiers. The only action-specific option
is `--continuation`, an optional string. Omit it for an initial read.
The success contract is one JSON
[section object](#outline-and-section-objects) with at most 4,096 UTF-8 text bytes.
There are no format, header, whole-book, or automatic continuation options.

Use a section ID from `book outline`, not a heading label or URL.
Each Windows read repeats fresh metadata, outline, and selected-page checks.
Native adapter acceptance passed for one selected chapter. Synthetic local
services cover continuation and failure cases; live continuation remains unchecked.
Parser-invalid identifiers, missing arguments, and malformed or oversized input
tokens fail with code 2 before the live operation. Noncanonical book IDs and
invalid opaque section IDs fail with code 1 before credential access.
A syntactically valid token does not enable live access.

In the offline contract, denied or unknown access, unavailable content,
unsupported sections, source failures, invalid metadata, wrong-scope or
invalid-position continuation, and changed sources fail with code 1.
Failure to encode a required output token also fails with code 1, not a result
that hides remaining text.

## Output contracts

Offline checks verify the following output contracts. Examples use invented
data, not live account results. JSON uses standard escaping and preserves
structured values. CLI JSON is pretty-printed with a final newline.

### Character and campaign objects

| Field | JSON type | Included in |
|-------|-----------|-------------|
| `id` | String | Character and campaign |
| `name` | String or `null` | Character and campaign |
| `source_url` | HTTPS URL string or `null` | Character and campaign |
| `character_ids` | Array of strings | Campaign only |
| `raw` | Any supplied JSON value, including `null` | Character and campaign |

Lists return arrays. Show commands return one object with the same fields.
`raw` retains the entire supplied JSON value, including unknown nested fields.
It is not a quoted JSON string. Optional metadata is `null`, not invented from an
identifier. Campaign associations are not resolved or rewritten.

### Book objects

| Field | JSON type | Meaning |
|-------|-----------|---------|
| `id` | String | Opaque book identifier |
| `title` | String | Book metadata, not content |
| `access` | String | Summary label from the observations below |
| `source_url` | HTTPS URL string | Book attribution |
| `evidence` | Array of objects | Separate catalog, purchase, and sharing observations |

Access labels are `purchased`, `shared`, `purchased_and_shared`, `unavailable`,
and `unknown`. A purchase or sharing grant can remain valid in one observation
even if a separate context denies access. Without a grant, catalog-only or unknown
evidence prevents an `unavailable` summary.

| Evidence `kind` | Other fields |
|-----------------|--------------|
| `catalog_only` | None |
| `purchased` | `state`: `granted`, `unavailable`, or `unknown` |
| `shared_with_account` | `state`: `granted`, `unavailable`, or `unknown`; no context identifier |
| `shared` | `context_id`: string; `state`: `granted`, `unavailable`, or `unknown` |

These fields describe supplied observations. They are not permanent permissions.

### Outline and section objects

An outline contains `book_id` (string), `source_url` (HTTPS URL string), and
`entries` (array). Each entry has these fields:

| Field | JSON type | Meaning |
|-------|-----------|---------|
| `id` | String | Section identifier |
| `title` | String | Navigation title |
| `parent_id` | String or `null` | Parent entry; `null` for a root |
| `source_url` | HTTPS URL string | Entry attribution |

Entries retain source order; a parent can appear later in the array.
A section result has these fields:

| Field | JSON type | Meaning |
|-------|-----------|---------|
| `book_id` | String | Selected book |
| `section_id` | String | Selected section |
| `source_url` | HTTPS URL string | Section attribution |
| `text` | String | One bounded chunk of original text |
| `next_cursor` | String or `null` | Explicit continuation token, or end |
| `is_complete` | Boolean | `true` when no text remains |
| `max_text_bytes` | Integer, `4096` | Text-byte cap, not total JSON size |

### Table and TSV rules

Table output aligns columns with spaces. TSV separates cells with tabs and uses
one physical line per row. Both include headers unless `--no-headers` is set.
Both escape cell contents:

| Original value | Cell representation |
|----------------|---------------------|
| Backslash | `\\` |
| Tab | `\t` |
| Newline | `\n` |
| Carriage return | `\r` |
| Other control character | `\u{hex}`, such as `\u{1b}` |

Missing optional metadata becomes an empty cell. `character_ids` is compact JSON
inside one escaped cell. Undo cell escaping before parsing that cell as JSON.
JSON output instead keeps original values with standard JSON escaping.
There is no automatic color or terminal-dependent format change.
No rows or fields are silently truncated.

Empty JSON lists are `[]`. Empty outlines retain `book_id`, `source_url`, and
`"entries": []`. Empty table or TSV results contain only headers, or no bytes when
`--no-headers` is set. Operation failures are not empty successful results.

### Book bounds and continuation

Every live outline, section, and explicit continuation checks fresh positive
library evidence and current canonical cookie-page structure. Unknown or
catalog-only evidence cannot enable a read, including public/free-looking sources.
The same session identity must supply metadata and content. A session change
returns `session_changed`; retry the operation. Neither prior results nor tokens grant access.
See the [page-level contract](architecture.md#live-book-reads) for supported scope.

Outlines allow at most 1,000 raw entries, including duplicates. Section output
contains at most 4,096 UTF-8 text bytes. This cap excludes JSON escaping, source
attribution, and continuation bytes. It is not a whole-response or source-memory
limit. HTML conversion preserves meaningful text, headings, lists, tables, and
sidebars with deterministic whitespace. Chunking preserves those normalized bytes.
A chunk can end before the cap to preserve a UTF-8 boundary or a complete line.

A nonterminal result has a string `next_cursor` and `is_complete: false`.
Request the next chunk explicitly with the same book and section identifiers:

```text
beyond-gate book read <book-id> <section-id> --continuation <returned-token>
```

This is syntax, not a successful live-read example. The CLI never follows a token
automatically. At the end, including an empty section, `next_cursor` is `null` and
`is_complete` is `true`. There is no bulk export or book-content cache.

Treat tokens as opaque. The current wire format uses a `bg1.` prefix and URL-safe,
unpadded base64 containing strict JSON metadata. The complete token is limited
to 16,384 encoded bytes (16 KiB), including the prefix. It contains book and
section identity, revision, source URL, and byte position, not text or credentials.
It is not encrypted, signed, or an access grant.

Parsing rejects invalid encoding, versions, fields, and metadata without echoing
the token. Source-dependent checks reject wrong scope, changed revision or URL,
and invalid byte positions after fresh access is checked. A token never selects
a URL to fetch. An oversized required output token is an explicit failure.

## Local MCP commands

### mcp start

```shell
beyond-gate mcp start
```

Starts the sibling `beyond-gate-mcp` executable directly, without a shell.
No positional arguments or action-specific flags are accepted. Install both
native binaries in the same directory; Windows uses `.exe` filenames.
The command inherits stdin, stdout, and stderr and passes through the server's
exit status. A missing companion or launch failure produces a diagnostic on
stderr and code 1.

The server waits for MCP protocol messages. It is not an interactive terminal
prompt. During serving, stdout contains protocol messages only. See
[startup and lifecycle](mcp.md#startup-and-lifecycle) for direct startup,
initialization, and transport closure.

### mcp install

**Warning:** Without `--dry-run`, confirmation permits changes to user-level
client settings. Review the proposed entry before answering.

```text
beyond-gate mcp install <client> [--dry-run]
```

Requires one client value from this table:

| `client` | Command required on `PATH` | User-level settings |
|----------|----------------------------|---------------------|
| `vscode` | `code` | Copilot Global: `mcp-config.json` under `COPILOT_HOME`, otherwise `~/.copilot` |
| `claude-code` | `claude` | `~/.claude.json` |
| `copilot-cli` | `copilot` | `mcp-config.json` under `COPILOT_HOME`, otherwise `~/.copilot` |

`~` means the current user's home directory. An explicitly empty `COPILOT_HOME`
is an error, not a request for the default. VS Code and Copilot CLI share this
configuration. Project settings are not changed.

The optional `--dry-run` flag defaults to off. It previews without a prompt or
write. It still requires the companion binary, selected client, and valid
existing settings. To preview, for example:

```shell
beyond-gate mcp install vscode --dry-run
```

The installer registers `mcpServers.beyond-gate` with the absolute sibling MCP
executable path and `args: []`. VS Code and Copilot CLI also receive
`type: "local"`. It stores no credentials or remote URL.

For a new entry, stdout shows the target path and proposed entry. Without
`--dry-run`, the prompt is `Apply this change? [y/N]`. Only `y` or `yes`, after
trimming whitespace and ignoring case, permits a write. Any other response,
including empty input or EOF, cancels successfully with no settings change.
There is no `--yes` flag.

An identical existing entry is a successful no-op, without a prompt.
A different entry with the same name is a conflict, not an overwrite.
The installer preserves unrelated JSON values and checks settings again after
confirmation. Missing settings can be created. Malformed JSON, a non-object root
or `mcpServers` value, unresolved paths, a missing client or companion, permission
failures, and input/output failures are explicit errors with code 1.
Preview, cancellation, repeat installation, and successful registration use code 0.
Invalid client names or options use code 2.

Registration does not enable live reads. For discovery and removal, see
[client registration](mcp.md#client-registration).

## completions

```text
beyond-gate completions <shell>
```

Requires one of `bash`, `elvish`, `fish`, `powershell`, or `zsh`.
No action-specific options are accepted. The command writes a completion script
to stdout and exits with code 0. A missing or unknown shell is usage error 2.
Output failures use code 1.

Scripts come from the same clap definition as parsing and help. They contain
commands and options, not dynamic account identifiers. Generation does not access
credentials or the network.

For Bash with bash-completion installed:

```bash
mkdir -p "$HOME/.local/share/bash-completion/completions"
beyond-gate completions bash > "$HOME/.local/share/bash-completion/completions/beyond-gate"
```

For PowerShell 7:

```powershell
$completionDirectory = Split-Path -Parent $PROFILE
New-Item -ItemType Directory -Force -Path $completionDirectory | Out-Null
$completionPath = Join-Path $completionDirectory 'beyond-gate-completions.ps1'
beyond-gate completions powershell | Set-Content -LiteralPath $completionPath -Encoding utf8
. $completionPath
```

Review the generated script before adding this line to the PowerShell profile
for later sessions:

```powershell
. (Join-Path (Split-Path -Parent $PROFILE) 'beyond-gate-completions.ps1')
```

## Exit codes and streams

| Exit code | Meaning |
|-----------|---------|
| `0` | Successful command, help, version, or canceled/no-op installation |
| `1` | Operation, storage, unsupported live operation, serialization, or output failure |
| `2` | Invalid usage, including malformed or oversized input continuation |

`mcp start` passes through the child process status rather than replacing it with
these adapter codes.

Result data, local session messages, completion scripts, and installer previews
and prompts go to stdout. Diagnostics go to stderr. During MCP serving, stdout
is reserved for protocol messages. Resource operations validate and serialize
before writing results, but a stream failure can leave partially written bytes.
Output failures, including a failed diagnostic write, do not report success.

Offline operation errors preserve distinctions such as denied access, unknown
access, rejected sessions, malformed sources, and invalid continuation.
A rejected session does not prove expiry. Browser login remains unavailable;
never supply a password, cookie, or token as a workaround.

## Spell pages and selected HTML

```shell
beyond-gate spell list --output json
beyond-gate spell list --class-id 17 --class-id 18 --level 0 --level 9 --page 2 -o json
beyond-gate spell show 900001-invented-spell
```

These IDs and names are synthetic. Show requires the complete numeric-ID/slug
key from a website link. Bare IDs, names, URLs, escapes, and padded text fail
before session access. The key permits at most 256 ASCII bytes.

| CLI flag | MCP field | Website control |
|----------|-----------|-----------------|
| `--name`, `-n` | `name` | `filter-search` |
| `--page` | `page` | `page` |
| `--class-id` | `class_ids` | `filter-class` |
| `--level` | `levels` | `filter-level` |
| `--school-id` | `school_ids` | `filter-school` |
| `--casting-time-id` | `casting_time_ids` | `filter-casting-time` |
| `--attack-type-id` | `attack_type_ids` | `filter-attack-type` |
| `--save-id` | `save_ids` | `filter-save-required` |
| `--damage-type-id` | `damage_type_ids` | `filter-damage-type` |
| `--condition-id` | `condition_ids` | `filter-condition` |
| `--tag-id` | `tag_ids` | `filter-tags` |
| `--source-id` | `source_ids` | `filter-source` |
| `--source-category-id` | `source_category_ids` | `filter-source-category` |
| `--verbal` | `verbal` | `filter-verbal` |
| `--somatic` | `somatic` | `filter-somatic` |
| `--material` | `material` | `filter-material` |
| `--concentration` | `concentration` | `filter-concentration` |
| `--ritual` | `ritual` | `filter-ritual` |
| `--partnered-content` | `partnered_content` | `filter-partnered-content` |

Repeat option flags for distinct IDs. Current and legacy class IDs stay distinct.
Levels are 0 through 9; zero means cantrips. Schools use option IDs 3 through 10.
Each other option filter allows at most 50 unique positive IDs. All option
filters together allow at most 256 IDs. The returned GET form must contain each
requested option and must echo consistent selections. IDs are not access grants.
Name allows 256 UTF-8 bytes. Page is a positive `u32`, default 1.
Encoded queries allow at most 16 KiB.

Boolean flags take `true` or `false`. Omission leaves that restriction unset.
The website uses `1`/`2` for concentration and `t`/`f` for other booleans.
Partnered content takes `no` (default), `any`, or `yes`.

List JSON returns `entries` and `pagination` with `page` and `next_page`.
Each entry contains `id`, `slug`, `name`, `level`, `school`, and `source_url`.
Website order is preserved. Table and TSV add page fields to each row.
The explicit empty marker returns no entries, not an error. Missing or
conflicting rows and paging fail the whole page. There is no automatic
continuation, selected-detail fan-out, or class/level API scan.

Show returns `id`, `slug`, `name`, `source_url`, and `raw_html`.
Only the checked `detail-content` subtree is serialized. Unknown elements and
stat fields survive. Serialization can normalize HTML.
Treat `raw_html` as untrusted text. Do not execute it, render it as trusted HTML,
log it, or cache it. Unexpected scripts, forms, and account/comment containers
inside that subtree fail the operation. Surrounding page chrome is not copied.

Both operations validate the session and selected response on every call.
They permit one cookie-only HTML read and at most one session exchange within
30 seconds. Limits are 100 raw listing rows, 1 MiB per response, 1 MiB compact
JSON, and the existing 8 MiB table presentation cap. Excess output fails; fields
are not truncated. HTTP 403 is denied, HTTP 401 is rejected rather than proven
expired, and HTTP 404 on detail means missing. Unsupported access structures
return no content. Monster access codes do not apply.

Browser structure checks are not native acceptance. Native cookie compatibility,
denied HTML, ownership, sharing, and unchecked filter combinations remain
unverified. See the [spell source contract](api-catalog.md#spell-read-contract)
and [MCP tools](mcp.md#spell_list-and-spell_show).

## Contract sources

The reference follows the [clap definition](../src/beyond-gate-cli/src/command.rs),
[runner](../src/beyond-gate-cli/src/lib.rs),
[serializers](../src/beyond-gate-cli/src/output.rs), and
[registration code](../src/beyond-gate-cli/src/mcp.rs).
The [CLI process tests](../src/beyond-gate-cli/tests/cli.rs) and
[synthetic adapter tests](../src/beyond-gate-cli/tests/adapter_process.rs)
check help, usage, output, errors, and explicit continuation without live accounts.
