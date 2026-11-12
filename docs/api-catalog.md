# D&D Beyond API call catalog

Last reviewed: 2026-11-11. Research issues: [#31][issue-31], [#32][issue-32],
[#65](https://github.com/PlagueHO/beyond-gate/issues/65), and
[#66](https://github.com/PlagueHO/beyond-gate/issues/66) and
[#69](https://github.com/PlagueHO/beyond-gate/issues/69).

Use this shared index when planning or implementing another issue.
It catalogs all calls identified in the [API survey](api-survey.md), including candidates
and known failures. It is not a complete list of D&D Beyond's internal APIs.
**Cataloged does not mean implemented, supported upstream, or approved for a live test.**
Windows Web-library and bounded book reads are implemented. Character and campaign
live reads remain unsupported. Species listing metadata comes from the public
website page; selected detail uses a candidate route with live access and
completeness still unverified. Native book adapter acceptance passed for one selected
chapter; live continuation and sharing remain unchecked.

## Spell read contract

Issue [#69](https://github.com/PlagueHO/beyond-gate/issues/69) uses GET
`https://www.dndbeyond.com/spells` and the selected canonical
`/spells/{numeric-id}-{slug}` HTML route. Both use the same validated cookie
identity in a new operation scope. The session exchange is the existing fixed
authentication-service call. No bearer is sent to spell HTML routes.
There is one resource request and at most one exchange, without redirects,
retries, alternate routes, or automatic page/detail fan-out.

List uses the [observed website controls](cli.md#spell-pages-and-selected-html),
not character `classLevel`. The returned GET form, option membership, echoed
selections, direct rows, footer, and explicit pagination must agree.
Selection checks include implicit first-option defaults in single-select controls.
Empty success requires `li.alert.no-results` inside
`ul.listing.listing-rpgspell.rpgspell-listing`, with a footer and no paging.
Nonempty rows are direct `div.info` children with `data-type=spells`,
a canonical `data-slug`, and a matching `.spell-name a.link`.
Every row is checked; one invalid or duplicate identity fails the whole page.
List returns entries and explicit page/next-page metadata in website order.
Complete means one requested filtered page, not every available spell.

Show requires the selected ID/slug class, canonical URL, spell stat block,
description, available-class marker, and source marker.
Only `section.detail-content` is returned as serialized `raw_html`.
Unknown selected fields survive. No JSON or JSON-LD contract was observed.
This user-selected HTML scope replaces the unverified raw JSON candidate.
Unexpected scripts, forms, or account/comment containers inside the subtree
fail rather than being silently removed. HTML remains untrusted text.

Limits are 100 raw listing rows, unchanged 1 MiB response bytes, 1 MiB compact
result JSON, 30 seconds for the whole operation, 16 KiB query bytes,
256-byte name and detail-key inputs, and bounded unique option arrays.
HTTP 403 denies access; HTTP 401 rejects the session without proving expiry.
HTTP 404 on show means missing. Unverified HTTP-200 access structures return no
content. Metadata, source IDs, visible options, and monster access codes grant
nothing. The book URL guard is unchanged.

`RULE-06`, `RULE-07`, and `RULE-08` remain character-context candidates.
First-party client construction uses `classId`, `classLevel`, and
`spellListIds` for ordinary and always-known reads, and `classId`/`classLevel`
for always-prepared reads. One always-known flow adds `backgroundId`.
These routes do not establish complete standalone website coverage and are
not fetched or merged by this implementation.

Browser evidence is [recorded separately](api-survey.md#spell-browser-evidence).
Native cookie compatibility, denied HTML, API responses, ownership, sharing,
and unchecked filter combinations remain unverified. No native account test
is part of implementation validation.

## How to use and maintain this catalog

Reference an entry's stable ID and link to its section from the implementation issue.
For example, reference `CHAR-01` and
[character and campaign contracts](#characters-and-campaigns).
Keep IDs stable when evidence changes. Mark a removed route as retired instead of reusing its ID.
Record new evidence and failed checks in the survey; update the matching catalog row.

Each section states its service, authentication evidence, source, and common limits.
Each row adds the method, path, known inputs or output, and unresolved conditions.
Braced names are placeholders, not actual account or resource identifiers.
Do not infer missing parameters, access rules, methods, or service bindings.

Before implementing a row, verify its current contract and record the owning issue,
exact request, response shape, access rules, paging, limits, and error behavior.
Use synthetic fixtures and local mock services for tests.
Keep this catalog and the survey consistent when implementation status changes.
Do not store credentials, signed download URLs, account snapshots, or proprietary text here.

| Evidence | Meaning | Not established |
|----------|---------|-----------------|
| Live | A separately approved native request returned the stated result | General access, all failures, or complete production support |
| Browser | Official browser navigation or safe structural metadata was inspected | Native authentication equivalence |
| Client | First-party public code declares the call | Current response, access, or complete parameters |
| Page | A public page or navigation link was inspected | A JSON API contract |
| Candidate | Pinned third-party source uses the call | Current first-party behavior |
| Unresolved | Method, service, or read semantics remain unknown | A usable request |

## Services and shared limits

Append each listed path to the complete service base, preserving its version prefix.
The leading slash is a separator, not an instruction to replace the base's path.
For example, `CHAR` plus `/characters/list` means
`https://character-service.dndbeyond.com/character/v5/characters/list`.
A service ID is documentation shorthand, not a production allowlist entry.
Do not bind a route to an origin from its name alone.

| Service | Base | Known role |
|---------|------|------------|
| AUTH | `https://auth-service.dndbeyond.com` | Fixed protected-session exchange |
| CHAR | `https://character-service.dndbeyond.com/character/v5` | Character records and builder data |
| CHAR51 | `https://character-service.dndbeyond.com/character/v5.1` | Explicit item route version |
| WEB | `https://www.dndbeyond.com` | Campaign APIs, website HTML, config, mobile candidates |
| GAME | `https://gamedata-service.dndbeyond.com` | Vehicle and infusion definitions |
| MONSTER | `https://monster-service.dndbeyond.com` | Separate monster candidates |
| Unbound | No verified base | Client route with an unresolved injected service |

`https://api.dndbeyond.com` and `https://dice-service.dndbeyond.com` appear in the client.
Their presence does not resolve the Unbound rows.
First-party library helpers separately bind `LIB-01` and `LIB-02` to
`https://api.dndbeyond.com`. Approved native CLI/MCP checks verified bounded
Web-catalog discovery. Real shared-access observations and book-content access
remain unverified.
`https://media.dndbeyond.com` serves inspected scripts and assets, not an access grant.
Staging hosts, telemetry, and message services are not resource API candidates.

The existing HTTP defaults remain 1 MiB per response, a 5-second connection deadline,
a 15-second attempt deadline, a 30-second logical-read deadline, concurrency four,
and at most two retries. Redirects and proxies remain disabled.
Research measurements do not change these limits.
Core resource transport supports GET; fixed authentication exchange is separate.
Candidate POST reads need their own proven read-only contract and bounded body.

Character/campaign assembly permits at most 100 raw records per page, 10 pages, and
1,000 raw records in total. Count duplicates before removal. Return complete results or fail.
Book outlines allow at most 1,000 raw entries. Section results allow at most 4,096 UTF-8 bytes.
Continuation is explicit, within 16 KiB, and never an access grant or fetch URL.
Check fresh access before each outline or section read.
Do not crawl chapters, download full books, or add a book-content cache.

## Authentication

Source: existing product session handling and earlier approved probes in
[the survey](api-survey.md#character-and-campaign-contracts).
Browser login remains the agreed credential-entry flow.

| ID | Service | Method and path | Authentication and result | Status |
|----|---------|-----------------|---------------------------|--------|
| AUTH-01 | AUTH | POST `/v1/cobalt-token` | Protected `CobaltSession` cookie; accepted response supplies a process-local bearer | Live; implemented for Windows login, validation, and session reuse |

Do not persist the derived bearer. Local credential presence is not upstream validation.
Do not add pasted credentials, credential environment variables, or credential-bearing MCP inputs.
This catalog does not authorize credential access.

## Characters and campaigns

Authentication: bearer-only requests succeeded in the approved native probes.
Evidence: Live, with additional Client and Browser list evidence.
Sources: [survey contracts](api-survey.md#character-and-campaign-contracts), C1, and L1.
Implementation issue: [#31][issue-31]. All five resource calls remain unimplemented.

| ID | Service | Method and path | Known inputs and result | Required condition |
|----|---------|-----------------|-------------------------|--------------------|
| CHAR-01 | CHAR | GET `/characters/list` | Query `userId={currentAccountId}`; boolean `success`, array `data.characters`, numeric `pagination.skip`, `take`, `total` | Use accepted session identity; require the complete default response within bounds |
| CHAR-02 | CHAR | GET `/character/{id}` | Query `includeCustomItems=true`; object `data`, null pagination, campaign association | Preserve unknown fields; validate selected ID and failure behavior |
| CAMP-01 | WEB | GET `/api/campaign/stt/active-campaigns` | String `status`; array `data`; summary `id`, `name`, `playerCount` | No roster fan-out from a list |
| CAMP-02 | WEB | GET `/api/campaign/stt/user-campaigns` | Same observed summary envelope | Distinction from active campaigns still needs a complete semantic contract |
| CAMP-03 | WEB | GET `/api/campaign/stt/active-short-characters/{campaignId}` | Array `data`; roster `id`, `name`, `userId` | Fetch only the selected roster |

The `CHAR-01` trial with `skip=0&take=1` failed a combined contract check.
Do not claim that paging parameters work. The accepted plan requires a complete default
response or explicit failure. `character_ids: null` means not loaded; `[]` means loaded empty.
These association decisions still need implementation and tests.

## Character-builder game data

All rows use GET. Evidence: Client. Source: C1.
Authentication and complete per-route inputs remain unverified.
The source can add `campaignId`, `sharingSetting`, and `characterId` from current state;
this does not prove that each row accepts or requires every parameter.
Except for the metadata-only RULE-01/RULE-02 contract below, shapes, access,
paging, size, and complete standalone catalog coverage remain unresolved.
These are not approved bulk-content extraction contracts.

| ID | Service | Path | Purpose and remaining contract |
|----|---------|------|--------------------------------|
| RULE-01 | CHAR | `/game-data/classes` | Implemented metadata-only GET with `sharingSetting=2`; Browser evidence; no rules-text grant |
| RULE-02 | CHAR | `/game-data/subclasses` | Implemented metadata-only GET with `sharingSetting=2&baseClassId={id}`; parent-scoped; Browser evidence |
| RULE-03 | CHAR | `/game-data/backgrounds` | Fresh bearer GET with `sharingSetting=2`; show selects a checked raw definition; observed 85-record response exceeds unchanged 1 MiB production cap; [contract](#background-read-contract) |
| RULE-04 | CHAR | `/game-data/races` | Candidate collection for selected species detail; see [species contract](#species-browse-and-detail-contract) |
| RULE-05 | CHAR | `/game-data/feats` | Feats; selection, prerequisites, and access |
| RULE-06 | CHAR | `/game-data/spells` | Character-eligible spells; standalone coverage not proven |
| RULE-07 | CHAR | `/game-data/always-known-spells` | Class spell support; class, level, and completeness |
| RULE-08 | CHAR | `/game-data/always-prepared-spells` | Class spell support; class, level, and completeness |
| RULE-09 | CHAR51 | `/game-data/items` | Equipment and magic items; explicit `v5.1`, not default `v5` |
| RULE-10 | CHAR | `/game-data/background-starting-equipment` | Background equipment; selection |
| RULE-11 | CHAR | `/game-data/class-starting-equipment` | Class equipment; selection |
| RULE-12 | CHAR | `/game-data/monsters` | Builder monster definitions; separate-service coverage not proven |
| RULE-13 | CHAR | `/game-data/racial-trait/collection` | Species traits; selected IDs and parameters |
| RULE-14 | CHAR | `/game-data/class-feature/collection` | Class features; selected IDs and parameters |
| RULE-15 | CHAR | `/game-data/backdrops` | Appearance metadata; access and limits |
| RULE-16 | CHAR | `/game-data/frames` | Appearance metadata; access and limits |
| RULE-17 | CHAR | `/game-data/portraits` | Appearance metadata; access and limits |
| RULE-18 | CHAR | `/game-data/themes` | Appearance metadata; access and limits |
| RULE-19 | CHAR | `/name/random` | Name support; read semantics not proven |
| RULE-20 | CHAR | `/name/random/list` | Name support; semantics and limits not proven |

Do not download a whole eligible dataset and describe local filtering as bounded API search.

### Class and subclass metadata contract

Issue [#65](https://github.com/PlagueHO/beyond-gate/issues/65) selected metadata-only
list/show. [Browser evidence](api-survey.md#class-and-subclass-metadata-evidence)
verified the class collection and two selected parent collections. Native
compatibility, value-level link validation, real sharing, and live global
coverage remain unverified. No direct detail route or remote name filter was
established. Show selects locally by ID; names use local lowercase substring
matching. Keep duplicate names and parent context.

Require numeric `id`, true `success`, string `message`, array `data`, and explicit
null `pagination`. Reject unknown/nonterminal envelope shapes. A supplied
`totalCount` must equal the raw count. Validate positive IDs, names, parents,
source references against CFG-01, and duplicate identities before filtering.
Source configuration is identity metadata, not ownership or rules-text permission.
Book-specific Hidden Sources selection does not apply to these metadata catalogs.

Global subclass traversal uses sequential per-parent GETs. Bound it to 32 parents
and 1,000 combined raw definitions. Class/scoped operations are not subject to
the global parent count. Responses are at most 1 MiB each; actual admitted
resource bodies total at most 8 MiB including configuration. Canonical JSON
results are at most 1 MiB before adapter formatting. The whole deadline is
120 seconds including session waits. Every request has one attempt, with no
redirect, retry, or continuation. Conservative explicit HTTP maxima including
exchanges are 3 for class, 5 for scoped subclass, and 67 for global subclass.
Complete means the default checked catalog scope, not all D&D content.

Metadata show preserves source-reference extensions but excludes raw definition
fields, descriptions, features, equipment/spell text, and inferred version/access
labels. Missing optional slug/link fields become null; malformed supplied fields
fail. Links resolve only to HTTPS at the fixed website origin, without credentials
or query. Display fragments are allowed; links never select fetches.
Denied, expired/rejected, missing, ambiguous, malformed, incomplete, and over-limit
results stay distinct. Never emit partial results after a failed parent.

### Background read contract

Issue [#66](https://github.com/PlagueHO/beyond-gate/issues/66) requires metadata
browsing and full selected detail where access is verified. Its
[public browser checks](api-survey.md#background-filter-evidence) confirmed
GET `/backgrounds`, repeated website skill selectors, partnered-content values,
and an explicit empty-result marker. The unfiltered page exposed 171 distinct
numeric background links without observed pagination controls.

An initial browser-only GET exceeded a 1 MiB probe cap and was cancelled before
JSON parsing. A later structural read used an 8 MiB browser-only cap and
returned HTTP 200, 85 records, and 1,160,568 bytes. The envelope had numeric
`id`, true `success`, string `message`, array `data`, and null `pagination`.
Source references had `sourceId`, `pageNumber`, and `sourceType`.
No content-access field or access map was present; `grantedFeats` is rules
data, not a grant. See the [structural evidence](api-survey.md#authenticated-background-response-limit).

Public client inspection traced direct background definition rendering.
Its local restricted-definition filter checks preferences and active sources,
not ownership. Show follows this returned-source contract: one fresh bearer
GET to the fixed collection, preceded by credential-free public configuration.
The complete envelope must be terminal, with unique positive IDs, valid names,
and source references that exist in the checked configuration. Every row must
pass before selection. Display links must use the exact website origin and
match the raw ID. They never select a fetch. Unexpected access maps or fields
are unsupported; HTTP denial remains denied. Unknown raw fields and source
extensions survive unchanged. A missing ID means absent from this checked
collection, not absent from the whole website.

List uses a cookie-only fixed website GET with the observed controls. It returns
`id`, `name`, source label, and `source_url`, not rules text. Requested option
IDs must exist in the same response's form. Unknown paging is unsupported.
No background page, skip, take, or API filter is invented.

Local bounds are 1,000 raw rows/definitions, 1,000 references per definition,
1 MiB response/result, and 30/60 seconds for list/show. Both adapters share
these bounds and typed failures. The observed 1,160,568-byte collection
**fails** the production cap with `BodyTooLarge`, even if the selected record
would fit. No limit increase, truncation, automatic continuation, rules engine,
bulk output, or cache is added. No metadata-only show fallback is used.
The [CLI contract](cli.md#background-metadata-and-selected-detail) and
[MCP contract](mcp.md#background_list-and-background_show) specify the inputs.

Website links/options and source IDs are not access grants. One checked website
detail page does not establish general collection access or full website text.
Native compatibility, native account acceptance, website/API identity coverage,
and real sharing remain unverified.

### Species browse and detail contract

Issue [#67](https://github.com/PlagueHO/beyond-gate/issues/67) adds
`species list/show`. List reads the fixed public `/species` page with the
stored cookie and parses card IDs, names, source titles, canonical links, and
legacy labels. It does not submit the page's client-side search input or infer
pagination. Empty or malformed page structure, duplicate IDs, observed paging,
and limits fail without partial output. List is limited to 1,000 entries,
1 MiB, and 30 seconds.

Show makes one fresh bearer request to the candidate character-service route
`GET /character/v5/game-data/races`. The first-party character application
contains the route family. A pinned third-party implementation uses the same
route without a query and reads race IDs and variant metadata. Its generic
client unwraps a character-service `data` envelope. Show requires the `id`,
`success`, `message`, array `data`, and null `pagination` fields. If present,
`totalCount` must match the array length. It returns the unchanged selected
record, including unknown fields, and does not request the displayed website
link. Denial, malformed or incomplete collections, unsupported access maps,
missing IDs, and limits fail without partial output.

The detail collection and canonical result are each limited to 1 MiB, the
collection to 1,000 records, and show to 60 seconds. Browser evidence confirms
the public page structure only. The detail route and basic field names have
client/candidate evidence; live response shape, completeness, access
semantics, native compatibility, and account acceptance remain unverified.
See the [API survey](api-survey.md#species-and-legacy-race-candidate),
[CLI contract](cli.md#species-metadata-and-selected-detail), and
[MCP contract](mcp.md#species_list-and-species_show).

## Definition collections and vehicles

Evidence: Client. Source: C1. Authentication, inputs, shapes, bounds, and access remain
unverified. Generic factories are templates, not evidence for every possible substitution.

| ID | Service | Method and path | Known distinction or remaining contract |
|----|---------|-----------------|-----------------------------------------|
| DEF-01 | GAME | GET `/vehicles/v3/rule-data` | Explicit rule-data declaration; not vehicle definition `v4` |
| DEF-02 | GAME | GET `/vehicle/v4/collection` | Vehicle collection; parameters and complete bounds |
| DEF-03 | GAME | GET `/vehicle/v4/{id}` | Selected vehicle definition |
| DEF-04 | GAME | GET `/infusion/v1/collection` | Infusion collection; parameters and complete bounds |
| DEF-05 | GAME | GET `/infusion/v1/{id}` | Selected infusion definition |
| DEF-06 | GAME | POST `/{entity}/{version}/collection` | Generic factory; body and read-only semantics unresolved |
| DEF-07 | CHAR | POST `/game-data/{entity}/collection` | Separate factory; body and read-only semantics unresolved |

The inspected separate-service version map contains vehicle `4` and infusion `1`.
Do not invent versions for `racial-trait` or `class-feature` from the entity-name helper.
Do not add a generic POST escape hatch.

## Monsters and configuration

Evidence: Approved browser monster probes; native metadata comparison for configuration.
Sources: T1/T2; the public proxy also constructs MON-01, and config forwarding
appears in T6.
Approved bearer-only browser probes checked MON-01 and MON-02. Native monster
CLI/MCP account acceptance has not run. A separately approved credential-free
`CFG-01` read succeeded.

| ID | Service | Method and path | Candidate inputs and result | Remaining contract |
|----|---------|-----------------|-----------------------------|--------------------|
| MON-01 | MONSTER | GET `/v1/Monster` | Implemented repeated `ids` for website candidates; array `data`, ID-keyed `accessType`, null pagination | Browser checked one/two selected IDs; native acceptance and larger batches unverified; codes 2/3 unsupported |
| MON-02 | MONSTER | GET `/v1/Monster/{id}` | Implemented selected object `data`, scalar `accessType`; preserves raw detail | Browser checked one code-1 detail; code 4 denied, codes 2/3 unsupported; native acceptance unverified |
| CFG-01 | WEB | GET `/api/config/json` | Credential-free JSON accepted within 1 MiB; 268 sources and 50 categories; all 177 `LIB-01` IDs matched; named `Hidden Sources` category selects eight library records for exclusion | Source/category identity only; configured ownership-only IDs are valid outside Web results, not replacement books or ownership grants; native adapter parity passed |

The [monster follow-up research](api-survey.md#monster-list-and-detail-research)
traces selected IDs to the same MON-01 collection in the public proxy. Its
release/homebrew filter is not an access-map check. Importer detail fields are
proxy evidence, not a direct-detail contract. The first-party builder
uses RULE-12; that does not establish standalone service coverage or permission.
Later explicitly approved account probes checked both routes and numeric paging.
The user selected support for observed code 1, rejection of code 4, and explicit
unsupported errors for all other numeric codes. Missing or malformed access
evidence fails. A page with any denied, unsupported, duplicate, or malformed row
fails as a whole.

Production list makes one cookie-only website GET with validated filters
and an explicit page. A nonempty page adds one bearer-only MON-01 request
for exactly the candidate IDs. Empty pages require the explicit marker
and no collection request. One session scope pins the protected identity
across both transports. Detail makes one direct selected GET. Neither
operation retries, follows redirects, fetches display links, follows another
page, uses a proxy, or caches content. A session exchange can precede reads.
Website candidate IDs, collection IDs, and one-to-one access keys must agree.
Output preserves website order. Display links require the fixed HTTPS website origin and
matching selected ID without credentials, queries, or fragments.

Local policy, not verified upstream maxima: maximum 100 raw records, name
limit 256 UTF-8 bytes, 1 MiB per body and compact result, a 30-second
whole-operation deadline, and an 8 MiB padded-table limit. IDs are canonical
positive decimal strings within `u64`. Result entries contain `id`, `name`,
`source_url`, and `access_code: 1`; list adds `page` (default 1) and explicit
`next_page`, while detail adds unchanged `raw`. The website controls page size;
no total is inferred. ID arrays permit 50 unique positive values each and 256
overall. Encoded queries are capped at 16 KiB. No rules year, source ownership,
complete stat-block coverage, or owned/shared case is inferred.

The later [approved monster filter trace](api-survey.md#approved-monster-filter-submission-trace)
confirmed real `GET /monsters?filter-*` submissions. A bounded read of one
selected response contained six filtered rows in the initial HTML. Repeated
keys encode multiple selections. CR selectors use option IDs, not literal CR
values. Boolean controls have empty, `t`, and `f` values; partnered content
defaults to `f` on the site.

This website route is not MON-01. No monster-service browser request or
equivalent JSON filter mapping was observed. The user chose website-filtered lists with explicit
page-based pagination, replacing the earlier `skip`/`take` list interface.
Detail reads and access checks stay unchanged.

A separately approved [paging and selected-ID check](api-survey.md#approved-website-paging-and-selected-id-check)
confirmed two twenty-row website pages, matching next-page links, the
`li.alert.no-results` empty marker, and one two-ID MON-01 collection read.
That collection returned exactly the selected records with code 1, an
ID-keyed access map, and null pagination. Nonempty filtered pages can therefore
use a website candidate read followed by one exact selected-ID collection
read. Restore website order and check every returned identity and grant
before summary output. Neither a listing link nor prior access is a grant.

The shared query and parser are implemented with synthetic CLI/MCP process
coverage. Unsupported option IDs, malformed HTML, missing empty markers, and
inconsistent pages fail explicitly. The source-book cookie guard stays unchanged.
Native cookie compatibility, native
CLI/MCP account acceptance, larger batches, and owned/shared cases remain
unverified. Do not infer an upstream batch maximum from the two-ID check.
Do not fetch per-row details, scan pages to emulate filters, or widen the
existing book-page URL guard. Both filter research scopes are closed.

The [configuration comparison](api-survey.md#alternative-configuration-catalog-comparison)
stopped at failed diagnostics. Its initial URL mapping rejected a relative form
used by the importer. A separately approved corrected comparison validated 170 of
177 library records, but did not resolve the four records lacking attribution.
Neither run establishes complete live discovery. Do not use the initial
zero-valid-record count as evidence that the configuration catalog lacks links.

The later [public category check](api-survey.md#independent-source-picker-and-category-evidence)
used one separate credential-free `CFG-01` request. It confirmed one `Hidden Sources`
category and the expected description phrases. Of 57 sources without links, 51
belonged to that category and six did not. This does not establish category
membership for the four unresolved `LIB-01` records or a complete book-selection
rule. No production category filter was enabled. Sources: C5 and T8-T10.

The separately approved [hidden-category comparison](api-survey.md#approved-hidden-category-comparison)
on 2026-11-06 matched all 177 library IDs. Eight were hidden, including all four
records without attribution. Two of the remaining 169 records still failed both
metadata mappings. The check returned `InvalidMetadata` after three attempts.
The scope is consumed; no production filter or ownership request followed.
The two remaining failure classes and complete ownership handling remain unresolved.

A subsequent separately approved
[failure-class diagnostic](api-survey.md#approved-metadata-failure-class-diagnostic)
found padded titles on both remaining records and a control character on one.
No link-validation failure class was reported for those records. The proposed
library mapping and configuration-title whitespace cleanup each validated all
169 non-hidden records. The three-request diagnostic passed and its scope is
consumed. The user subsequently approved production category selection and title
cleanup. That initial mapping used strict URLs and full raw-catalog
ownership matching. Fresh native CLI acceptance later passed metadata preparation
but failed ownership/catalog matching; MCP was skipped. No wider URL allowlist is justified.
The later [approved Web scope](api-survey.md#approved-web-catalog-result-scope)
keeps full raw ownership validation against configuration without emitting
ownership-only sources.

Earlier monster-service pagination observations included `take`, `skip`,
`currentPage`, `pages`, and `total`. Current filtered list output uses website
`page`/`next_page`, not that earlier offset interface.
A visible name, partial record, or HTTP success is not a full-content grant.
Pinned proxy item code uses `v5`; first-party `RULE-09` explicitly selects `v5.1`.
Do not replace stronger version evidence with a third-party assumption.

## Website library, outlines, and sections

These are HTML routes, not JSON section APIs.
Sources: B1 and the [book evidence](api-survey.md#books-sources-and-access-checks).
Browser requests used the signed-in page context; credential headers were not collected.
Earlier native probes used only the process-local bearer. A separately approved
cookie-only check later accepted one chapter response. Neither transport is a
production book reader.
Book implementation remains separate from [#31][issue-31]; see the
[library and book work](roadmap.md#implementation-issues).

| ID | Service | Method and path | Evidence and result | Limit |
|----|---------|-----------------|---------------------|-------|
| BOOK-01 | WEB | GET `/sources` | Browser redirects to `/en/library`; earlier credential-free fetch failed | Do not enable automatic production redirects |
| BOOK-02 | WEB | GET `/en/library` | Browser HTML includes selected-source metadata | 2,654,622 decoded bytes exceeds the 1 MiB limit; bounded transport unresolved |
| BOOK-03 | WEB | GET `/sources/dnd/cos` | Browser HTML 246,103 bytes; native bearer HTML 243,123 bytes; later cookie-only native HTML 246,103 bytes with one parsed outline container and 81 raw links | One selected example, not a general book map or access grant |
| BOOK-04 | WEB | GET `/sources/cos/introduction` | Listed chapter route; browser redirected to the canonical route | Alias applies only to observed navigation |
| BOOK-05 | WEB | GET `/sources/dnd/cos/introduction` | Browser HTML 257,524 bytes; earlier bearer-only native HTTP 307 to a purchase path; separately approved cookie-only native HTTP 200 with 257,524 HTML bytes and expected literal markers | Preview status, full-book access, fresh authorization, and DOM parsing remain unverified |
| BOOK-06 | WEB | GET `/sources/dnd/cos/into-the-mists` | User-selected non-preview chapter; cookie-only native HTTP 200 with 356,402 HTML bytes; one parsed article and all three selected outline fragments matched; final anonymous control HTTP 307 | The control reason and general entitlement contract remain unverified; no redirect was followed |

The listed section target was `/sources/cos/introduction#RunningtheAdventure`.
The fragment `#RunningtheAdventure` is local HTML selection, not an HTTP API input.
The outline selector is `.compendium-toc-full-text`; the article container is
`.p-article-content`. See the survey for heading boundaries and raw link counts.
Those investigations did not extract text or establish a separate entitlement API.
The later [page-level contract](architecture.md#live-book-reads) now implements
bounded text conversion and fresh checks, with synthetic CLI/MCP process coverage.
It requires positive fresh library evidence and a checked canonical cookie page
for each operation. [Native book adapter acceptance](api-survey.md#native-book-adapter-acceptance)
passed for the selected BOOK-06 chapter, with matching outlines and initial chunks.
Live continuation and sharing remain unchecked.
Other source paths must not be generated from this one example.
The [cookie-only result](api-survey.md#approved-native-website-cookie-check)
does not authorize an automatic fallback from a denied bearer request.
The [protected-chapter check](api-survey.md#approved-protected-chapter-structure-and-control)
adds parsed structure and a final anonymous control. It does not grant access
to other chapters or authorize another request.

## Library JSON client routes

Evidence: Client and approved native metadata checks on 2026-11-04.
Sources: C2/C4/L2.
Implementation issue: [#32][issue-32].
See the [source inspection](api-survey.md#first-party-library-json-routes).
Fixed-route production discovery includes the approved category selection and
title cleanup and the user-approved Web-catalog result scope. Fresh native
acceptance passed through both adapters with matching 169-book results. All live
scopes are consumed; another live run needs separate approval.
The production client explicitly selects `https://api.dndbeyond.com`, not WEB.
Its signed-in search flow obtains authentication through `AUTH-01`.
Bearer-only native requests returned JSON within 1 MiB in that check.
The follow-up proved source count equality but rejected the source response at
the existing per-page record cap. At that checkpoint, ownership completeness
and paging remained unverified. Later diagnostic and native adapter checks
accepted count equality and terminal collections under the implemented caps.

| ID | Service base | Method and path | Client evidence and remaining contract |
|----|--------------|-----------------|----------------------------------------|
| LIB-01 | `https://api.dndbeyond.com` | GET `/library/sources?platform=Web` | Native 200; 123,198 bytes; `items` count equals `totalCount`; within 1,000 records but exceeds 100-record page cap; some source paths fail the conservative rule |
| LIB-02 | `https://api.dndbeyond.com` | GET `/library/sources/me` | Native 200; 2,104 bytes; `items`, numeric `totalCount`, `sourceId`, `ownership[].type`; `owned` observed but no `shared`; contexts, absence, and revocation unresolved |

The search client combines ownership and free-source observations into one filter flag.
Do not use that flag as purchase evidence or a content-access grant.
Do not copy fallback source IDs, error-to-empty behavior, or bot-protection integration.
Keep the 1 MiB response cap. The user approved a separate 1,000-record complete
catalog cap, while ownership retains 100 records. Runtime discovery requires
count equality for both routes and rejects continuation or ownership IDs absent
from validated public configuration. Configured ownership-only sources do not
create result entries outside the Web catalog.
Missing ownership remains unknown. `shared` becomes account-level sharing
without an invented context. A supplied validated marketplace link is permitted
only when a book path is absent. See [live discovery](architecture.md#live-library-discovery).
See the [native check](api-survey.md#approved-native-library-metadata-check).
The first structural check did not verify count equality. The approved follow-up
verified it only for sources, then stopped at the page cap before the ownership read.
Production implementation has synthetic transport and process checks.
Do not turn partial probe evidence into live adapter acceptance or a fresh-content
access claim. Neither consumed scope authorizes another request.
Later native acceptance failed catalog metadata validation. Catalog-only
diagnostics found missing attribution and rejected released-only filtering.
The earlier proposed legacy-link and title-normalization changes were not applied
at that checkpoint.
See [native rejection evidence](api-survey.md#native-adapter-rejection-and-catalog-diagnostics).
Later diagnostics established the category/title mapping now implemented without
wider URL rules. Fresh native CLI acceptance passed catalog preparation, then
failed `UnmatchedOwnership`; MCP was skipped. See the
[acceptance evidence](api-survey.md#native-selected-library-acceptance).
A later diagnostic found seven configured ownership-only IDs: two hidden and
five visible. The user approved the Web result scope rather than inventing book
metadata for them. That change is checked offline; fresh native acceptance is
complete: both adapters returned matching metadata and evidence for 169 books,
with 36 purchased, zero shared, and 133 unknown. See the
[fresh native acceptance](api-survey.md#native-web-catalog-adapter-acceptance).
All live scopes are consumed.

The [public library client trace](api-survey.md#public-library-rendering-boundary)
on 2026-11-05 found a prepared `sources` prop, user filters, and cards that consume
supplied `relativePath` values. It did not expose the raw-source book-selection
or link-mapping implementation. The menu's path-skipping helper serves bounded
favorites and recent lists, not complete discovery. No account request or
production behavior change resulted from this trace.

## Mobile book API candidates

All rows use WEB, POST, and `application/x-www-form-urlencoded`.
Evidence: Candidate, except the limited Live metadata result for MOB-01.
Source: T7 and the [approved metadata result](testing.md#approved-metadata-result).
Each sends the stored Cobalt value in the form field `token`, not the derived bearer.
Beyond Gate has no production mobile credential transport.
An ignored, test-only Windows `MOB-01` probe has
[offline safety checks](testing.md#mobile-book-metadata-probe).
Its one approved metadata request is consumed. It does not establish content access.

| ID | Path | Additional form fields | Source-code interpretation and limit |
|----|------|------------------------|--------------------------------------|
| MOB-01 | `/mobile/api/v6/available-user-content` | None | Live: one bounded HTTP 200 JSON response confirmed license/entity structure and field types. Book `EntityTypeID` equal to `496802664` remains a T7 interpretation; source mapping and access meaning remain unverified |
| MOB-02 | `/mobile/api/v6/get-book-url/{bookId}` | None | Whole-book download URL; outside current bounded-read design |
| MOB-03 | `/mobile/api/v6/book-codes` | `sources`, JSON array with `sourceID` and `versionID` | Database access material; not a selected-section endpoint or approved test |
| MOB-04 | `/mobile/api/v6/do-higher-versions-exist` | `sourceVersions`, `manifestVersion` | Downloaded-source version check; client reads `sourceUpdatesAvailable` |
| MOB-05 | `/mobile/api/v6/user-data` | None | Account information; privacy-sensitive and not needed to prove section access |

The third-party helper checks `status: "success"` and unwraps `data` when present.
That is parser behavior; the approved MOB-01 run confirmed one successful envelope only.
Free, owned, shared, denied, expired, oversized, and malformed outcomes remain unresolved.
An entity without `isOwned` does not prove shared access.
Do not copy the helper's redirects, full-result logging, or whole-book processing.
Any metadata-only `MOB-01` test needs separate explicit scope and approval.

## Public listing and navigation pages

These are WEB HTML paths, not JSON API routes.
Evidence: Page. Sources: P1/P2 and C2.
The first eight returned HTML without credentials. The last three were found in navigation
only; their content was not fetched. GET describes normal page navigation, not a tested API.
See the [public filter inventory](api-survey.md#public-listing-filters) for exact form names.
No filter selection, next page, or selected record was requested in the initial survey.
The later [monster trace](api-survey.md#approved-monster-filter-submission-trace)
checked filter submissions and server HTML, but not pagination.

| ID | Path | Purpose and limit |
|----|------|-------------------|
| PAGE-01 | `/classes` | Public class listing; no JSON contract |
| PAGE-02 | `/backgrounds` | Public background filters; paging not observed |
| PAGE-03 | `/species` | Public species listing; bounded HTML metadata; no server-side search or paging contract observed |
| PAGE-04 | `/feats` | Public feat filters; paging not observed |
| PAGE-05 | `/spells` | Explicit cookie-only filtered pages; [spell contract](#spell-read-contract) |
| PAGE-05-D | `/spells/{numeric-id}-{slug}` | Selected cookie-only HTML detail; no raw JSON route |
| PAGE-06 | `/monsters` | Implemented filtered candidate pages and explicit next-page checks; browser HTML evidence does not establish native account acceptance |
| PAGE-07 | `/magic-items` | Public item filters and `page` links |
| PAGE-08 | `/equipment` | Public equipment filters and `page` links |
| PAGE-09 | `/sources/dnd/br-2024` | Basic Rules navigation only; no content fetched |
| PAGE-10 | `/sources/dnd/br-2024/rules-glossary` | Rules Glossary navigation only; no content fetched |
| PAGE-11 | `/sources/dnd/ua` | Unearthed Arcana navigation only; no content fetched |

## Other client routes with incomplete contracts

Source: C1, except the browser timing rows from B1.
Rows are Client evidence unless marked Browser or Unresolved.
No row is implemented or approved for a live test by its inclusion here.
Authentication for CHAR rows and complete parameters remain unverified.

| ID | Service | Method and path | Remaining condition |
|----|---------|-----------------|---------------------|
| AUX-01 | CHAR | GET `/party/inventory/{campaignId}` | Campaign access, privacy, and shape |
| AUX-02 | CHAR | GET `/premade/{characterId}` | Access and read semantics |
| AUX-03 | CHAR | GET `/rule-data` | Coverage; not a proven full rules catalog |
| AUX-04 | CHAR | GET `/vehicles` | Selection, paging, and access |
| AUX-05 | CHAR | GET `/vehicle/components` | Selection, paging, and access |
| AUX-06 | CHAR | GET `/infusion/items` | Selection and access |
| AUX-07 | CHAR | GET `/known-infusions` | Selection and access |
| AUX-08 | CHAR | GET `/roll-result-groups` | Privacy, scope, and paging |
| AUX-09 | CHAR | GET `/share/url` | Sharing semantics; excluded from v1 until reviewed |
| AUX-10 | CHAR | GET `/character/rest/short` | Read-only semantics unproven; rest actions excluded |
| AUX-11 | CHAR | GET `/character/rest/long` | Read-only semantics unproven; rest actions excluded |
| AUX-12 | Unbound | GET `/campaigns/v1/details/{gameId}` | Client uses bearer and `data`; service base unresolved |
| AUX-13 | Unbound | GET `/campaigns/v1/{gameId}/characters` | Client uses bearer and `data`; service base unresolved |
| AUX-14 | Unbound | GET `/dice/v1/getallfamilysets` | No explicit bearer in inspected call; service and access unresolved |
| AUX-15 | Unbound | GET `/diceuserconfig/v1/get` | Client uses bearer; service and privacy unresolved |
| AUX-16 | WEB | Unknown `/api/campaign/{campaignId}/character/{characterId}` | Unresolved method and read/write semantics |
| AUX-17 | WEB | Unknown `/api/vcm/33-15` | Browser timing only; method and purpose unknown |
| AUX-18 | WEB | Unknown `/api/vcm/41-348` | Browser timing only; method and purpose unknown |

## Third-party intermediary calls

These are integration references, not direct D&D Beyond API candidates.
Do not send Beyond Gate credentials to a hosted proxy or add a required proxy runtime.
Sources: T5/T6 and the [importer research](api-survey.md#mrprimate-importer-and-mobile-api-candidates).
Paths use the selected importer proxy, normally `https://proxy.ddb.mrprimate.co.uk`.
Public self-hosted source does not prove that hosted book routes are available locally.

| ID | Method and path | Known role and boundary |
|----|-----------------|-------------------------|
| PXY-01 | POST `/proxy/auth` | Checks supplied cookie through bearer exchange; not product credential entry |
| PXY-02 | POST `/proxy/user-data` | Account lookup; not a book-access grant |
| PXY-03 | GET `/proxy/api/config/json` | Public proxy forwards to `CFG-01` |
| PXY-04 | POST `/proxy/items` | Eligible item extraction, not a proven bounded search |
| PXY-05 | POST `/proxy/class/spells` | Class spell extraction, not a complete standalone catalog |
| PXY-06 | POST `/proxy/character` or `/proxy/v5/character` | Character processing and additional lookups; not the direct record contract |
| PXY-07 | POST `/proxy/monster` or `/proxy/monsters` | Monster extraction; client filters are not a server-side bounded contract |
| PXY-08 | POST `/proxy/monstersById` or `/proxy/monsters/ids` | Selected-ID batch extraction; limits unverified |
| PXY-09 | POST `/proxy/campaigns` | Public proxy campaign lookup |
| PXY-10 | POST `/proxy/adventure/available-user-content` | Hosted ownership lookup; absent from pinned public proxy |
| PXY-11 | POST `/proxy/library` | Hosted fallback lookup with `ownedOnly`; absent from pinned public proxy |
| PXY-12 | POST `/proxy/adventure/book-codes` | Hosted database access material; absent from pinned public proxy |
| PXY-13 | POST `/proxy/adventure/book-url/{bookId}` | Hosted signed URL and book code; absent from pinned public proxy |
| PXY-14 | GET `/proxy/maps/metadata/summary` | Foundry enhancement metadata, not D&D Beyond access evidence |

Book acquisition and ownership helpers send `cobalt` and `betaKey` to their selected proxy.
The importer also uses a configurable CORS proxy, normally
`https://images.ddb.mrprimate.co.uk/`, for book ZIP delivery.
The signed URL is runtime data, not a catalog entry or approved fetch target.
Its dynamic-service origin is configuration, not evidence for another resource route.
Avrae's DynamoDB entitlements and Discord signing are also integration-specific;
no reusable personal-account HTTP access endpoint was established from them.

## Exclusions and evidence sources

Character creation, copy, activation, deletion, build, rest, levels, scores, choices,
preferences, inventory changes, spell changes, vehicle state, campaign joining,
and dice settings writes remain outside v1.
Do not infer permission or read-only behavior from an HTTP method.
Unresolved families are retained to prevent repeated guessing, not to authorize them.

The [survey source index](api-survey.md#sources) supplies the exact source links and builds.
The [survey](api-survey.md) retains request scopes, failed probes, metadata measurements,
HTML filter names, and interpretation limits.

| Source ID | Evidence source |
|-----------|-----------------|
| C1 | First-party character application build `main.4ccf1602.js` |
| C2 | First-party mega-menu build `index-CVrwI34f.js`; navigation and library JSON helpers, not verified native contracts |
| C3 | Official My Library announcement; user-facing filters, not an API schema |
| C4 | First-party library page build `page-794b0c736763f2f7.js`; consumes prepared records, not a raw-source contract |
| C5 | Credential-free `CFG-01` category metadata checked on 2026-11-05; no account-library join |
| P1/P2 | Eight public listing pages; HTML filters, not API parameters |
| T1/T2 | `AlexWorland/dndbeyond-mcp` at `61d6f3caae27b539d5a1ba7b0668074c27672702` |
| T3/T4 | `avrae/avrae` at `15258cb884c1085582a710a9791f890371afb6c4` |
| T5 | `MrPrimate/ddb-importer` at `a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5` |
| T6 | `MrPrimate/ddb-proxy` at `a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a` |
| T7 | `MrPrimate/ddb-adventure-muncher` at `5d4f8e80554078ad907084885e8d2f73bf7a4355` |
| T8 | `theunschut/ddb-scraper` at `54cf2d411ad4533b45c9f0cf7871dfc0cdf38c8b` |
| T9 | `astermoline/dndbeyond-parser` at `3a667c78ba832cd87f76817fad6407a8a5b05e9a` |
| T10 | `petrgon/beyond-my-sources` at `7dcfa621323d354e661ce8c6de7b915bde809170` |
| B1 | Separately approved three-page selected-book browser inspection |
| L1 | Separately approved native probes and accepted decisions on [#31][issue-31] |
| L2 | Separately approved three-attempt native library metadata check on [#32][issue-32] |

Recheck version-sensitive decisions against current first-party documentation when available.
If no public contract exists, record that gap and obtain approval for a bounded live check.
Stop on access denial. Never treat another issue's old test scope as reusable approval.

[issue-31]: https://github.com/PlagueHO/beyond-gate/issues/31
[issue-32]: https://github.com/PlagueHO/beyond-gate/issues/32
