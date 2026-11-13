# D&D Beyond API survey

Recorded on 2026-11-02 for [issue #31][issue-31].
Library JSON client evidence was added on 2026-11-04 for [issue #32][issue-32].
The public library rendering boundary was traced on 2026-11-05.
Independent source-picker implementations and public category metadata were
checked on the same date.
A separately approved hidden-category comparison ran on 2026-11-06.
A new failure-class diagnostic ran later that day.

This survey covers every area observed in the Rules menu and the related API families found
in the site's public client. It includes earlier approved character and campaign inspections.
It does not establish a complete list of internal APIs or a supported public API specification.
It does not add production reads to Beyond Gate.

Use the [shared API call catalog](api-catalog.md) to find known calls by stable ID.
This survey holds their detailed evidence, measurements, and failed checks.
Keep both documents current so other issues can reuse findings without repeating probes.

## Equipment and magic-item research

Issue [#70](https://github.com/PlagueHO/beyond-gate/issues/70) requires raw JSON
equipment detail and a shared source with magic-item issue
[#71](https://github.com/PlagueHO/beyond-gate/issues/71).
The initial approved built-in Browser research ran on 2026-11-14.
The user performed login. No native credential access, live CLI/MCP test,
account write, or bulk catalog traversal ran. No credentials, account snapshots,
or proprietary rules text were retained.

### Observed item collection

The [public character client][character-client] explicitly selects
`GET https://character-service.dndbeyond.com/character/v5.1/game-data/items`.
It does not use the default `v5` version for items.
Two browser-only exchanges at the existing fixed authentication service succeeded.
Four bearer-only item reads returned HTTP 200:

| Query | Rows | Pagination | Response bytes |
|-------|-----:|------------|---------------:|
| `sharingSetting=2` | 100 | `skip=0`, `take=100`, `total=2796` | 250,444 |
| `sharingSetting=2&skip=0&take=1` | 100 | Same | 250,444 |
| `sharingSetting=2&skip=1&take=1` | 100 | Same | 250,442 |
| `sharingSetting=2&ids=<selected-id>` | 100 | Same | 250,444 |

All three selector trials returned the same first-page identities.
These queries do not establish working paging or selected-ID reads.
The response has numeric `id`, true `success`, string `message`, array `data`,
and a nonterminal pagination object. There was no envelope access map.
Do not treat this partial response as a complete catalog, infer global absence
from it, or describe local selection from it as a verified detail operation.

Observed row fields include numeric `id`, `entityTypeId`, and `baseTypeId`,
string `definitionKey`, boolean `magic`, string `name`, `filterType`, and
array `sources`. Raw fields also include descriptions, item properties, costs,
weights, equipment-specific fields, and flags. Unknown selected fields must
survive any eventual raw JSON implementation.
Source references contain numeric `sourceId` and `sourceType`, with nullable
numeric `pageNumber`. The inspected page had 100 unique IDs and definition keys.
Global uniqueness and website/API identity equivalence remain unverified.

### Classification and access

The inspected page had 21 records with boolean `magic=false` and 79 with
boolean `magic=true`. Magical records occurred in Weapon and Armor categories,
as well as Wondrous item. Category or rarity alone cannot establish the split.
The first-party item helper reads `definition.magic`; derived character items
can also become magical through infusion state.
Use raw catalog classification, not character inventory state.
Missing or malformed classification must not silently become equipment.
These observations do not verify every equipment category or all catalog pages.

The first-party item-loading path also filters definitions through homebrew
preferences and `activeSourceLookup`. This client restriction is not a verified
content-access contract. Configuration membership, source IDs, website links,
`sharingSetting=2`, and the absence of an access map grant nothing.
Owned/shared behavior and native adapter acceptance still need evidence.

### Website candidates

The [equipment page][equipment-page] exposed a GET form with `filter-search`,
`filter-cost-min`, `filter-cost-max`, `filter-weight-min`, `filter-weight-max`,
`filter-source-category`, and `filter-source`.
The source controls are multi-selects.
Its `ul.listing.listing-igear.igear-listing` contained 30 rows.
`b-pagination` had an explicit next link selecting `page=2`.
The credential-free `/js/waterdeep` read confirmed the cost and weight names.
Filter effects, empty-result markers, and API identity mapping remain unchecked.

One selected equipment detail exposed a small `details-container-content`
subtree. This is HTML evidence only. The user selected raw JSON detail rather
than the existing spell HTML approach. Do not substitute HTML or metadata-only
detail to complete the equipment feature.

### Verified paging consumer and page-scoped selection

A later credential-free read of the same first-party character script found
`getInventoryShoppe` calling `makeLoadAvailableItems` with query parameters
`page` and `pageSize`. The initial values are page 0 and page size 1,000.
The client uses `pagination.total` to determine further pages and combines them.
This supplied Client evidence for different parameter names before live testing.
Do not copy its full-catalog loop or cache.
Do not infer that page size 1,000 fits Beyond Gate's unchanged limits.

The user then approved six item GETs, one browser-only session exchange, and
two public script/configuration reads. All six GETs used bearer only and
omitted cookies. Public reads omitted credentials. All returned HTTP 200.

| API page | Page size | Rows | Pagination | Response bytes |
|---------:|----------:|-----:|------------|---------------:|
| 0 | 1 | 1 | `skip=0`, `take=1`, `total=2796` | 1,331 |
| 1 | 1 | 1 | `skip=1`, `take=1`, `total=2796` | 3,162 |
| 0 | 100 | 100 | `skip=0`, `take=100`, `total=2796` | 250,442 |
| 27 | 100 | 96 | `skip=2700`, `take=100`, `total=2796` | 245,943 |
| 28 | 100 | 0 | `skip=2800`, `take=100`, `total=2796` | 125 |
| 0, repeated | 1 | 1 | Same as first request | 1,331 |

The first two requests returned different identities. The repeat matched
the initial selected identity and full raw JSON. Every inspected
`definitionKey` matched positive decimal `entityTypeId:id`. Complete keys
were unique within inspected pages. Numeric IDs alone are not established
as globally unique.

The fresh public script was 5,800,062 bytes. Its active-source lookup came
from sources filtered by character source-category preferences. Homebrew
preference also affects client filtering. This is preference evidence, not
an ownership or access-code mapping. Public configuration was 715,197 bytes
with 270 sources. Every inspected item source ID existed in it.
Configuration membership checks attribution, not content permission.

The user approved explicit one-based source page plus full `definitionKey`
for detail. The shared implementation requests exactly one zero-based API
page with fixed `pageSize=100`. It validates all rows before filtering
equipment or selecting raw detail. It reports absence only within that
page. It never scans the catalog, fills filtered pages, substitutes HTML,
or treats unverified access maps as monster access codes.
Native compatibility and owned/shared behavior remain unverified.

The approved scope used two of eight equipment/detail page operations, three
of four public script/configuration reads, all four item API reads, and both
permitted exchanges. DOM inspections made no extra explicit resource requests.
The first item body had an 8 MiB streamed research ceiling; later trials had
a 1 MiB ceiling. Research ceilings do not change production policy.
Temporary probe state was removed.
A new explicit approval is required for further account API reads or exchanges.
The later six-GET scope is also exhausted. Temporary Browser probe state
was removed. No native credential read, live CLI/MCP test, global traversal,
or account write ran. Implementation and synthetic checks are documented
in the [equipment contract](api-catalog.md#equipment-and-magic-item-contract-status).

## Spell browser evidence

The approved issue #69 browser phase is complete and closed.
It used nine bounded page navigations/submissions and four public script
reads, below the approved 16 MiB combined script cap.
No explicit authenticated API request, native credential read, token exchange,
native acceptance test, or account write ran.
No denial or bot-protection stop occurred. No real snapshots or spell rules
text were retained.

The [standalone spell catalog][spells-page] submitted GET forms.
Checks covered repeated `filter-class`, `filter-level`, and `filter-school`,
including level 0 cantrips. A Cleric/cantrip result had nine rows and no paging.
A two-class/two-level result had twenty rows and page links.
The selected page-2 link preserved filters and selected page 2.
A submitted name/school/component/concentration/ritual combination returned
two rows. Concentration used `1`/`2`; ritual and components used `t`/`f`.
The submitted form included `filter-partnered-content=f`.
The form exposed other controls in the
[shared filter table](cli.md#spell-pages-and-selected-html), but those
combinations did not receive submission acceptance checks.

A synthetic no-match search supplied `li.alert.no-results` inside
`ul.listing.listing-rpgspell.rpgspell-listing`, with a footer and no paging.
Nonempty rows were direct `div.info` children, not `li` children.
Each had `data-type=spells`, a numeric-ID/slug `data-slug`, and a matching
`.spell-name a.link`. The default page had twenty rows.
Separate current, legacy, and partnered class IDs must not collapse by name.
The school controls used IDs 3 through 10. Rows identified the school through
an empty `.spell-school .school` element with a school-name CSS class.
Page-2 controls included links to page 1 without a `page` query parameter.

Two selected detail navigations supplied `.spell-details.<id-slug>-details`
with `section.more-info` and `section.detail-content`.
The selected content included `.ddb-statblock.ddb-statblock-spell`,
`.more-info-content`, `.available-for`, and `.spell-source`.
The checked current page had a canonical link matching its URL.
Observed stat items were level, casting time, range/area, components, duration,
school, attack/save, and damage/effect, each with a label and value.
Neither page supplied application JSON or JSON-LD.
The user selected complete numeric-ID/slug HTML detail rather than a guessed
numeric-only or JSON route.

The page's first-party `waterdeep` script and the public
[character client][character-client] were inspected for request construction.
Ordinary and always-known spell reads used `classId`, `classLevel`, and
`spellListIds`. Always-prepared reads used `classId` and `classLevel`.
One always-known flow added `backgroundId`. These character-context sources
do not establish a complete standalone website catalog.
No class/level scanning or catalog merge is implemented.

This evidence establishes browser structure, not native HTTP compatibility.
Native cookie behavior, denied HTML, authenticated API responses, ownership,
sharing, and unchecked filter combinations remain unverified.
See the [implemented contract](api-catalog.md#spell-read-contract).

## Scope and evidence

The signed-in browser was used for safe request metadata and public client inspection.
No account identifiers, credentials, account records, or book sections are included here.
The initial broader Rules survey made no new authenticated JSON API request.
The later selected-book inspection used three separately approved signed-in page opens.
The site ran its normal background requests. Inspection code made no direct API requests
and returned only structural metadata, field types, and safe classifications.

The public listing inspection used nine requests: eight succeeded and `/sources` failed during
fetching or redirect handling. Requests omitted credentials, rejected redirects, had a
15-second deadline, and limited each HTML response to 2 MiB. The inspection stopped on that
failure. It did not open record details or follow pagination.

The public character and mega-menu scripts were also inspected. Public script reads omitted
credentials and used a 15-second deadline and an 8 MiB limit. No staging service was queried.
These research limits do not replace the product's stricter HTTP limits.

The later MrPrimate research inspected public source in three pinned repositories.
It made no live D&D Beyond API request, used no stored credentials, and did not call a
hosted proxy. The project's documentation site returned HTTP 403; it was not retried.
Repository source, not that unavailable page, supports the findings below.

| Evidence | Meaning | Limit |
|----------|---------|-------|
| Live | An earlier approved request returned the stated shape | Does not establish every account, error case, or page |
| Browser | The official page request or safe cache metadata was observed | Does not prove an endpoint's complete contract |
| Client | The route or method exists in the first-party public source | Does not prove live success, permission, or response shape |
| Page | Public navigation, form names, or page links were inspected | HTML routes and filters are not JSON API contracts |
| Candidate | A pinned third-party implementation uses the route or shape | Must be checked before production use |
| Unresolved | This survey did not establish the contract | Do not guess an endpoint or return invented data |

## Rules menu coverage

The first eight listing pages below returned public HTML. The Basic Rules, Rules Glossary,
and Unearthed Arcana paths were observed in navigation only. Their content was not fetched.

| Area | Website path | Related API family | Proposed read surface |
|------|--------------|--------------------|-----------------------|
| Classes | `/classes` | Character `game-data/classes` | Selected class metadata |
| Backgrounds | `/backgrounds` | Character `game-data/backgrounds` | Selected background metadata |
| Species | `/species` | Public HTML; candidate Character `game-data/races` for selected detail | Bounded website metadata; selected source record, live contract unverified |
| Feats | `/feats` | Character `game-data/feats` | Bounded search and selected feat |
| Spells | `/spells` | Character spell game data | Bounded search and selected spell |
| Equipment | `/equipment` | Character item and starting-equipment game data | Equipment metadata |
| Magic Items | `/magic-items` | Character `game-data/items` | Bounded search and selected item |
| Monsters | `/monsters` | Character monster game data; separate monster-service candidate | Bounded search and selected monster |
| D&D Beyond Basic Rules | `/sources/dnd/br-2024` | Book transport unresolved | Source metadata, outline, selected section |
| Rules Glossary | `/sources/dnd/br-2024/rules-glossary` | Book transport unresolved | Selected glossary entry or bounded section |
| Unearthed Arcana | `/sources/dnd/ua` | Source discovery and delivery unresolved | Selected source metadata and bounded read |
| Other books and sources | `/sources` redirects to `/en/library` in the later browser inspection | Library metadata and selected-book HTML observed; native transport and fresh access unresolved | Existing `book list`, `book outline`, and `book read` |

These proposed surfaces are not implemented commands. Character-builder game data can depend
on a character, campaign, sharing context, or rules version. It must not be presented as a
complete standalone rules catalog without evidence.

### Public listing filters

The following names were found on the public forms. `page` was also found in links on spells,
monsters, magic items, and equipment. Query names, accepted values, server-side behavior, and
JSON service parameters are separate contracts. No filter or next page was requested in this
initial survey. A later [approved monster filter trace](#approved-monster-filter-submission-trace)
checked real submissions and filtered server HTML, but not a next page or equivalent JSON
service parameters.

The initial survey found general `search` inputs on all eight pages. A later
issue #67 browser pass found the species search as an unnamed client-side input,
with no GET form. No server-side species query was tested or inferred. Classes
and species had no additional matching filter names. Backgrounds and feats
had no observed paging parameter.

| Listing | Additional observed input names |
|---------|---------------------------------|
| Spells | `filter-search`, `filter-level`, `filter-class`, `filter-school`, `filter-casting-time`, `filter-attack-type`, `filter-save-required`, `filter-damage-type`, `filter-condition`, `filter-concentration`, `filter-ritual`, `filter-verbal`, `filter-somatic`, `filter-material`, `filter-tags`, `filter-source`, `filter-source-category`, `filter-partnered-content` |
| Monsters | `filter-search`, `filter-cr-min`, `filter-cr-max`, `filter-type`, `filter-sub-types`, `filter-size`, `filter-environment`, `filter-alignment`, `filter-armor-class-min`, `filter-armor-class-max`, `filter-average-hp-min`, `filter-average-hp-max`, `filter-movement`, `filter-language`, `filter-senses`, `filter-save-proficiency`, `filter-skill-proficiency`, `filter-resistance`, `filter-vulnerability`, `filter-damage-immunity`, `filter-condition-immunity`, `filter-has-lair`, `filter-is-legendary`, `filter-is-mythic`, `filter-tags`, `filter-source`, `filter-source-category`, `filter-partnered-content` |
| Magic Items | `filter-search`, `filter-type`, `filter-rarity`, `filter-requires-attunement`, `filter-magic-bonus`, `filter-has-charges`, `filter-attached-spells`, `filter-effect-type`, `filter-effect-subtype`, `filter-conditions`, `filter-armor-subtypes`, `filter-weapon-subtypes`, `filter-tags`, `filter-source`, `filter-source-category`, `filter-partnered-content` |
| Equipment | `filter-search`, `filter-cost-min`, `filter-cost-max`, `filter-weight-min`, `filter-weight-max`, `filter-source`, `filter-source-category` |
| Backgrounds | `filter-name`, `filter-skills`, `filter-tags`, `filter-source`, `filter-source-category`, `filter-partnered-content` |
| Feats | `filter-name`, `filter-prereq-subtype`, `filter-tags`, `filter-source`, `filter-source-category`, `filter-partnered-content` |

## Service origins

The [first-party character client][character-client] contains the production character,
game-data, website, API, dice, media, and authentication origin strings below. The monster
origin comes from the [third-party endpoint definitions][candidate-endpoints].

| Origin | Role and evidence | Beyond Gate status |
|--------|-------------------|--------------------|
| `https://auth-service.dndbeyond.com` | Protected-session exchange; Live and Client | Existing fixed authentication service |
| `https://character-service.dndbeyond.com` | Characters and character-builder game data; Live and Client | Resource reads not implemented |
| `https://www.dndbeyond.com` | Campaign endpoints and HTML listings; Live, Client, and Page | Resource reads not implemented |
| `https://gamedata-service.dndbeyond.com` | Separate definition and vehicle rule-data service; Client | No live contract |
| `https://monster-service.dndbeyond.com` | Separate monster service | Later approved code-1 page/detail and code-4 listing probes; native acceptance unverified |
| `https://api.dndbeyond.com` | Production API configuration string; Client | Individual route bindings still need verification |
| `https://dice-service.dndbeyond.com` | Production dice configuration string; Client | Individual route bindings still need verification |
| `https://media.dndbeyond.com` | Public scripts and assets; Client and Page | Research source, not an access grant |

An origin string is not permission to call every route on that host. Staging strings were
also present in public source. They were not inspected as services and must not enter a
production allowlist from this survey alone.

## Character and campaign contracts

These shapes come from the earlier approved live inspections in this session. The official
character page and public client provide additional list evidence.

| Method | URL or path | Observed contract |
|--------|-------------|-------------------|
| POST | `https://auth-service.dndbeyond.com/v1/cobalt-token` | Accepted protected cookie exchange; bearer remains process-local |
| GET | `https://character-service.dndbeyond.com/character/v5/characters/list?userId={currentAccountId}` | Boolean `success`; array `data.characters`; numeric `pagination.skip`, `take`, and `total` |
| GET | `https://character-service.dndbeyond.com/character/v5/character/{id}?includeCustomItems=true` | Object `data`; null pagination; campaign object and association data |
| GET | `https://www.dndbeyond.com/api/campaign/stt/active-campaigns` | String `status`; array `data`; summary fields include numeric `id`, string `name`, and numeric `playerCount` |
| GET | `https://www.dndbeyond.com/api/campaign/stt/user-campaigns` | Same observed summary envelope |
| GET | `https://www.dndbeyond.com/api/campaign/stt/active-short-characters/{campaignId}` | Array `data`; records include numeric `id`, string `name`, and numeric `userId` |

The website campaign endpoints accepted bearer-only authentication in the approved probes.
Do not copy an unrestricted browser cookie bag into resource requests.

The character list client sends only `userId`. Safe browser cache inspection showed a zero
initial offset, a total equal to the returned record count, and a count within the 100-record
limit. A separate `skip=0&take=1` trial failed a combined page-contract check and stopped.
It did not establish which check failed. Do not invent working paging parameters from it.

The accepted implementation decision is to use the default character list request and
require a complete response within the existing limits. An incomplete or oversized response
must fail explicitly. Current-account identity must come from accepted session evidence,
not a user-selected account ID.

Campaign listing will return summaries without a roster request for each campaign.
`character_ids: null` means the roster was not loaded. A selected campaign show operation
will load one roster and return an array, including `[]` for a known empty roster.
These decisions are not yet production behavior.

Missing records, denied access, malformed responses, expiry, and exact size limits still
need synthetic tests. The successful live probes do not establish those failure contracts.

## Class and subclass metadata evidence

Issue [#65](https://github.com/PlagueHO/beyond-gate/issues/65) selected metadata-only
browsing. Five separately approved browser scopes used 13 explicit requests.
All scopes are consumed. They do not authorize native credential access or
another account request.

The first-party asset
[`main.4ccf1602.js`](https://media.dndbeyond.com/character-app/static/js/main.4ccf1602.js)
had SHA-256
`84B06F44B9FD7D2F5318E289984EB24D62372C585CC4AE5C4A9D023631D815A4`.
It declares class GET and subclass GET with `baseClassId`. Its chooser applies
local lowercase substring matching and preserves duplicate names. No direct
definition-detail route or upstream name filter was established.

| Browser observation | Result |
|---------------------|--------|
| Classes GET with `sharingSetting=2` | 26 records; 578,185 decoded bytes in the corrected probe |
| Fighter `2190879` and Fighter `10` | Same name; source IDs `[145]` and `[1]`; null parents |
| Subclasses under `2190879` | 9 records; 183,454 decoded bytes |
| Subclasses under `10` | 11 records; 204,990 decoded bytes |
| Parent relations | All observed child parents matched |
| Cross-parent name overlap | One case-insensitive overlap |
| Public configuration | 269 sources, 50 categories; 713,412 decoded bytes |

Envelopes had numeric `id`, boolean `success`, string `message`, array `data`,
and null `pagination`. `totalCount` and additional envelope fields were absent.
Records had numeric IDs, names, parent IDs, source references, `isHomebrew`,
`slug`, `moreDetailsUrl`, and raw content fields. Top-level `sourceId`,
`isLegacy`, `rulesVersion`, and `version` were absent. Source labels and link
values were not retained. Cross-parent ID uniqueness, native compatibility,
real sharing, and live global traversal remain unverified.
Do not infer year/version/ownership from IDs, sources, names, or order.

The pinned importer at
[`a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5`](https://github.com/MrPrimate/ddb-importer/tree/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5)
uses proxy class/subclass workflows and locally inferred rules years.
Its subclass request uses name and year rather than proving a direct upstream
name-search contract. The public proxy at
[`a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a`](https://github.com/MrPrimate/ddb-proxy/tree/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a)
has no matching classes/subclass handlers. No proxy runtime, cache, or inferred
year mapping is adopted.

The public renderer gates feature descriptions through a per-definition access
lookup: FREE 1, OWNED 2, SHARED 3, NO_ACCESS 4. Missing entries are inaccessible.
Character-context simulated OWNED is not a standalone grant.
An initial selected-feature scope stopped before POST because an upstream typed
key predicate did not match. The later public trace showed that optional-feature
keys can be derived locally from numeric IDs, then parsed back into numbers for
the selected-ID request. Response access lookup keys are decimal IDs.

The corrected approved scope made exchange, classes GET, and one selected-feature
collection POST. Fighter `2190879` had 24 positive numeric feature IDs; all 24
lacked `definitionKey`. The selected POST returned HTTP 200 with 146 decoded
bytes, zero `definitionData` entries, and zero `accessTypes` entries.
The stop category was `missing_numeric_access_evidence`. No access code was
observed. This is not content permission, explicit account denial, or proof
of global nonexistence. No alternative ID/version or GET was tried.
The selected-ID factory uses default character-service `v5`. A separate
`makeLoadAvailableOptionalClassFeatures` GET helper uses an active definition ID,
but does not establish whole-class authorization.

The user then selected metadata-only list/show. The implementation validates
fixed collections and source identities, not raw-content permission.
Unknown source-reference metadata survives; arbitrary raw definition fields
are intentionally excluded. No class-feature request is part of production
browsing. See [RULE-01/RULE-02 limits](api-catalog.md#class-and-subclass-metadata-contract).
Synthetic adapter checks are not native/live acceptance.

## Background filter evidence

Issue [#66](https://github.com/PlagueHO/beyond-gate/issues/66) selected catalog
metadata plus full selected detail where access is verified. Its separately
approved browser scope permits bounded page and API inspection with
human-controlled login. During the initial public pass, the browser was not
signed in, and the user was not available to log in. That pass made no
authenticated API request, token exchange, native credential read, or
selected-detail request.

The public [background listing][backgrounds-page] had one GET filter form
with action `/backgrounds`. Its controls were:

| Control | Observed shape |
|---------|----------------|
| `filter-name` | Text input; initially empty |
| `filter-source-category` | Multiple select; 33 options |
| `filter-source` | Multiple select; 186 options |
| `filter-skills` | Multiple select; 18 options |
| `filter-tags` | Multiple select; 18 options |
| `filter-partnered-content` | Single select; empty unspecified, `t` Yes, `f` No; initially `f` |

These are website option IDs, not inferred API IDs or access grants. For example,
the skill options included Arcana `240` and History `241`. A real form submission
with those two selections produced:

```text
/backgrounds?filter-name=&filter-skills=240&filter-skills=241&filter-partnered-content=f
```

The unfiltered page had 171 distinct numeric background links. The filtered
response had 38 listing rows and 38 numeric background links. No
`.b-pagination-list` controls were observed on either page. This is not proof of
global completeness, a page API, or the absence of other result limits.
The monster implementation's 100-record page cap cannot be copied unchanged
and described as a complete unfiltered background result.

A second real form submission cleared the multiple selections and used an
invented no-match name. The response had zero numeric background links and
one `li.alert.no-results` child inside
`ul.listing.listing-rpgbackground.rpgbackground-listing`.
That marker distinguishes the observed empty result from a missing listing.
No bot-protection marker was observed during these three page operations.

This initial evidence established public form behavior only. Implementation
paused until the authenticated structural and first-party reader checks below.
Public links and website option IDs still provide no content-access grant.
The website's 171 links do not define the authenticated API's 85-record scope.

The initial pass consumed three page operations and made no explicit API
request or public script read. No probe data was stored in the browser, and no
account responses or proprietary detail were retained.

### Authenticated background response limit

On resume, the browser showed a sign-out control instead of sign-in controls.
Two bounded credential-free reads checked the same first-party character
asset used by the class research. Its SHA-256 was unchanged. It still declares
the background GET and applies a local restricted-definition filter in the
available-background adapter. No separate background detail declaration was
found in the inspected matches. These observations do not establish content
access.

One browser-only token exchange succeeded. A bearer-only
`GET https://character-service.dndbeyond.com/character/v5/game-data/backgrounds?sharingSetting=2`
then returned HTTP 200. Its body crossed the 1 MiB probe limit. The streaming
reader cancelled the body at 1,100,304 received bytes, before JSON parsing.
This is a lower bound, not the complete response size. No response fields,
record count, source identities, or access values were returned or retained.
The bearer stayed inside the browser operation and was not persisted.

That probe stopped at its byte limit. The approved scope bounded research
request counts but did not set a 1 MiB research response cap; the 1 MiB value
is the production transport limit. A later structural probe used an 8 MiB
browser-only cap without changing production limits.

The later probe used one fresh exchange and one background GET. It returned
HTTP 200, 1,160,568 bytes, and 85 records. Its envelope had numeric `id`,
boolean `success`, string `message`, array `data`, and null `pagination`.
`success` was true. All observed record IDs were positive safe integers, and
all records had source arrays. Source-reference keys were `sourceId`,
`pageNumber`, and `sourceType`.

Observed identity fields included `id`, `entityTypeId`, `definitionKey`,
`name`, `slug`, `moreDetailsUrl`, `isHomebrew`, and `sources`.
Records also had descriptions, proficiency/language/equipment descriptions,
feature name and description, `featureIsFeat`, card text, appearance fields,
ability data, organization data, characteristics arrays, spell-list IDs,
`featList`, and `grantedFeats`. No raw values or rules text were retained.
There was no content-access field or access map in the envelope or records.
`grantedFeats` is rules data, not a content-access grant.

A separate selected website navigation to `/backgrounds/7-acolyte` had a
matching canonical path and a 440,153-byte decoded HTML body. It contained
the normal background detail container and description structure without an
observed bot-protection marker. This one selected page is not evidence that
every collection record grants full standalone detail.

A third public client read traced background description/feature rendering
and the local restricted-definition filter. That filter applies homebrew
preferences and active source selection. It is not a standalone content-access
check. The background helper had no observed access-type getter. The inspected
chooser can also include a definition from an existing character, which
cannot establish standalone access. No separate detail declaration was found
in the inspected matches. These observations do not prove that no such route
exists.

The implementation follows that direct authenticated source-reader contract.
Show selects one unchanged definition only after a fresh successful collection
read and complete-envelope validation. It does not claim that public metadata,
source IDs, or a character-attached definition grants access. No monster-style
access map is invented. An unexpected access field is unsupported; HTTP denial
returns no content. This is selection from the returned source collection,
not a claim of full website detail or source-book coverage.

The whole observed collection exceeds the unchanged production 1 MiB transport
limit. Production show therefore fails explicitly with `BodyTooLarge` for that
response. It does not raise the limit, truncate, cache, or return partial data.
No selected-ID API filter, separate detail route, or paging contract was
verified. Native cookie compatibility, native show acceptance, website/API
identity coverage, and owned/shared behavior remain unverified.

List reads only metadata from the filtered website result. It requires the
checked form, listing, footer, and explicit empty marker, and rejects paging
controls rather than inventing continuation. Local limits are 1,000 raw rows
or definitions, 1,000 source references per definition, and 1 MiB compact
result. List has a 30-second deadline; show has a 60-second deadline for its
configuration and collection reads. Neither operation retries or fans out.

Total research consumption is five of eight page operations, four of six
explicit API requests including two exchanges, and three of four public script
reads. Temporary public-source probe data was removed from the browser.
No native credential access or live CLI/MCP check ran.
Unused operations do not authorize another host, native account tests, or
an automatic retry after an access denial or bot-protection stop.

## Character-builder game data

All routes in this section are Client evidence from the
[public character application][character-client].
Their response shapes, access rules, paging, and record counts were not tested live.

The source URL builder maps `HACK__CHARACTER_SERVICE_GAME_DATA` to the character-service
base. It constructs `character/{version}/{route}`. The default version is `v5`.
The `game-data/items` declaration explicitly selects `v5.1`.
This differs from the third-party client's `v5` item route.

The request builder can add current-state parameters. Public helpers show `campaignId`,
`sharingSetting`, and `characterId` construction. Their required values and use for each
endpoint still need verification. A sharing parameter is not an access grant.

The following paths use GET and the base
`https://character-service.dndbeyond.com/character/v5/`, except the stated item override.

| Relative path | Area | Contract still required |
|---------------|------|-------------------------|
| `game-data/classes` | Classes | Rules version, source coverage, and access |
| `game-data/subclasses` | Subclasses | Parent-class selection and access |
| `game-data/backgrounds` | Backgrounds | Rules version and access |
| `game-data/races` | Species and legacy race data | Complete coverage, access, and rules version; see [candidate review](#species-and-legacy-race-candidate) |
| `game-data/feats` | Feats | Search or selection, prerequisites, and access |
| `game-data/spells` | Character-eligible spells | Selection context and standalone catalog coverage |
| `game-data/always-known-spells` | Class spell support | Class and level parameters, completeness, and access |
| `game-data/always-prepared-spells` | Class spell support | Class and level parameters, completeness, and access |
| `game-data/items` at `v5.1` | Equipment and magic items | Selected reads, source coverage, and response size |
| `game-data/background-starting-equipment` | Background equipment | Background selection and rules version |
| `game-data/class-starting-equipment` | Class equipment | Class selection and rules version |
| `game-data/monsters` | Character-builder monster definitions | Coverage versus the separate monster-service candidate |
| `game-data/racial-trait/collection` | Species traits | Selected IDs, parameters, and access |
| `game-data/class-feature/collection` | Class features | Selected IDs, parameters, and access |
| `game-data/backdrops` | Character appearance | Access and metadata limits |
| `game-data/frames` | Character appearance | Access and metadata limits |
| `game-data/portraits` | Character appearance | Access and metadata limits |
| `game-data/themes` | Character appearance | Access and metadata limits |
| `name/random` | Name generation support | Read semantics and selection parameters |
| `name/random/list` | Name generation support | Read semantics and result limits |

Downloading a whole eligible catalog and filtering it locally is not a proven bounded search.
Limiting displayed results after a large download does not meet the product's request limit.

### Species and legacy-race candidate

The first-party character application contains the `game-data/races` route
builder. A pinned third-party endpoint definition names
`GET /character/v5/game-data/races` without a query. Its race search uses the
authenticated character-service client and reads `entityRaceId`, `fullName`,
`baseName`, `baseRaceName`, `isLegacy`, and `isSubRace`. The candidate client
unwraps character-service `data` responses and sends bearer authentication.
See [T11](#sources) and [T12](#sources).

This evidence does not establish the current response envelope, pagination,
complete standalone coverage, or per-record access rules. No protected route
or live account was tested. The selected-detail implementation uses one fixed
collection read, without a guessed detail route or selected-ID filter. It
rejects non-null pagination and returns no partial result. Its `pagination:
null` requirement, access behavior, native compatibility, and account
acceptance remain unverified.

The bounded issue #67 browser pass inspected the public `/species` page. It
found 242 cards in 42 source groups. Each card had one title, source label, and
link shaped as `/species/<id>-<slug>`. Each group had a footer. The page showed
no pagination links. Its search input had no `name` and no GET form, so the
implementation does not send a search query. The Browser canvas did not expose
response headers or bodies. One selected detail navigation redirected to
Marketplace; this did not establish a detail API or access contract. The pass
made no direct API request and read no account data.

## Definition collections and vehicles

The same first-party client has separate definition factories. Source helper `tm` uses GET,
`nm` uses GET with path substitution, `rm` uses POST, `im` uses PUT, and `am` uses DELETE.
These method assignments were inspected; the collection bodies were not sent.

| Method | Source-built route | Service base | Evidence and limit |
|--------|--------------------|--------------|--------------------|
| GET | `vehicles/v3/rule-data` | `https://gamedata-service.dndbeyond.com` | Explicit client declaration |
| GET | `vehicle/v4/collection` and `vehicle/v4/{id}` | Same game-data base | Definition factory and vehicle version constant |
| GET | `infusion/v1/collection` and `infusion/v1/{id}` | Same game-data base | Definition factory and infusion version constant |
| POST | `{entity}/{version}/collection` | Same game-data base | Generic definition factory; body and read-only semantics unresolved |
| POST | `game-data/{entity}/collection` | Character `character/v5` base | Separate generic factory; body and read-only semantics unresolved |

The entity-name helper includes `vehicle`, `infusion`, `racial-trait`, and `class-feature`.
The inspected separate-service version map explicitly contains only vehicle `4` and infusion
`1`. Do not invent separate-service versions for the other names.
Vehicle definition `v4` and vehicle rule-data `v3` are different routes.

A POST can perform a read, but that must be established from its actual operation.
The current core resource transport supports GET, not arbitrary POST.
Do not approve every collection route or add a generic POST escape hatch.
A selected-definition POST would need a fixed route, a bounded body, proven read-only
semantics, safe retry rules, and its own tests.

## Separate monster and configuration candidates

The following routes come from the pinned
[third-party endpoint definitions][candidate-endpoints] and
[reference implementation][candidate-reference]. Later approved monster probes
are recorded [below](#monster-list-and-detail-research).

| Candidate method | URL | Candidate parameters or shape |
|------------------|-----|-------------------------------|
| GET | `https://monster-service.dndbeyond.com/v1/Monster` | `search`, `skip`, `take`, `showHomebrew`, `sources`, or repeated `ids` |
| GET | `https://monster-service.dndbeyond.com/v1/Monster/{id}` | Selected monster |
| GET | `https://www.dndbeyond.com/api/config/json` | Rules lookup configuration |

The candidate monster search shape has array `data`, an `accessType` map, and pagination
fields `take`, `skip`, `currentPage`, `pages`, and `total`.
The candidate detail shape has object `data` and scalar `accessType`.
Only access cases 1 and 4 were observed in the later monster probes.
A visible name, partial record, or successful
HTTP response must not be treated as permission to return a full stat block.

The candidate config contains lookup arrays for challenge ratings, monster types,
environments, alignments, damage types, and senses. A source lookup is not a user library
or an entitlement check.

The third-party reference client also builds a spell catalog from many class and level
requests, caches large datasets, and can tolerate partial results.
It filters item and feat datasets locally. These are implementation choices, not evidence
of supported standalone bounded-search APIs. Beyond Gate must not copy these behaviors.
Hardcoded source maps and guessed access-type meanings are also unsuitable.

### Monster list and detail research

Issue [#72](https://github.com/PlagueHO/beyond-gate/issues/72) requires a verified
detail contract before implementation. The selected list behavior is one bounded
page with explicit continuation, not automatic catalog traversal.
The metadata-only scope selected for class issue #65 does not apply to monsters.

The pinned [importer monster factory][importer-monster-factory] selects
`POST /proxy/monsters/ids` when IDs are supplied and `POST /proxy/monster`
otherwise. Its [source types][importer-monster-types] describe proxy records with
statistics, source references, nested arrays, and description fields. These
types do not prove direct upstream field availability or content permission.
Beyond Gate does not use this proxy or send credentials to it.

The pinned [public proxy route builder][mrprimate-proxy-config] constructs
`GET /v1/Monster` with `search`, `skip`, `take`, `showHomebrew`, and repeated
`sources`. Selected IDs use repeated `ids` on that same collection route.
The [proxy monster reader][mrprimate-proxy-monsters] collects pages and filters
records with `isReleased === true` or homebrew state. That filter is not a
monster-specific access-map check. Its bulk traversal, image rewriting, and
partial-failure behavior are not adopted. It does not establish a separate
direct-detail operation or numeric access meanings.

The first-party character client also declares `game-data/monsters`.
`makeLoadAvailableMonsters` calls that collection through `Sy`, then `Sf`
applies `filterNonRestrictedEntityDefinitions`. The extras chooser consumes the
returned creature definitions. This is character-builder evidence, not proof
that a standalone service record has the same access contract. The class
feature access enum cannot establish monster-service access meanings.
The inspected asset is the same pinned `main.4ccf1602.js` described in the
[class evidence](#class-and-subclass-metadata-evidence). Its SHA-256 matched.

A separately approved built-in-browser scope opened the official `/monsters`
listing once and made three explicit credential-free public script reads.
The page loaded without an observed access-denial title. The inspected resource
routes included the site's normal authentication exchange but no monster-service
request. This is not a direct API probe or proof that no such route exists.
`/js/waterdeep` returned 134,636 bytes with SHA-256
`71327e8d6d49b7b268ddbebd3ae6ccf0c3559409130990e16141a2c98c367628`.
It contained no tested monster-service route or access-map identifier.
The second read inspected the pinned character client above.
The third read, `/js/cobalt`, returned 452,435 bytes with SHA-256
`92925618f6bf788af005a9665bc3e477e27df672415a4a6bb288a8a699e69eda`.
It contained no tested monster or access-map identifier. These negative matches
do not establish the absence of another client or route.

A later credential-free documentation check made one GET to
`https://monster-service.dndbeyond.com/swagger/v1/swagger.json`.
It returned HTTP 403 with content type `text/html`. The check stopped without
reading the body, following redirects, retrying, or trying another route.
No cookies, bearer, or default credentials were sent. This failure does not
establish that a public specification or the candidate resource routes are absent.

Two later account scopes were explicitly approved. The first made exactly five
requests, all HTTP 200, with no redirects or retries:

| Request | Decoded bytes | Structural result |
|---------|---------------|-------------------|
| Authentication exchange | 1,509 | Bearer accepted inside browser memory |
| MON-01, `search=wolf&skip=0&take=1&showHomebrew=f` | 2,550 | One record, total 18, ID `4775850`, code 1 |
| Same list with `skip=1` | 2,650 | One record, ID `17062`, code 1 |
| MON-01, `ids=4775850` | 4,161 | One selected array record, code 1 |
| MON-02, ID `4775850` | 4,113 | Selected object record, scalar code 1 |

List `accessType` is an object keyed by decimal ID. `pagination` contains numeric
`take`, `skip`, and `total`, with null `currentPage` and `pages`. List `stats`
and `metaData` are objects. Selected-ID collection pagination, stats, and metadata
are null. Direct detail has object `data`, numeric `accessType`, and null `stats`.
No class-style `success` or `message` discriminator was present.

The code-1 records exposed these raw fields. Types describe only the sampled
responses, not a complete schema or a guarantee for every monster:

| Raw fields | Observed shape |
|------------|----------------|
| `id`, `entityTypeId`, `collectionUserId`, `homebrewStatus` | Numbers; no account-related field values were retained |
| `name`, `url` | Strings; production validates identity and fixed-website attribution |
| `alignmentId`, `sizeId`, `typeId`, `challengeRatingId` | Numbers; no lookup labels or challenge-rating conversion inferred |
| `armorClass`, `averageHitPoints`, `passivePerception` | Numbers |
| `sourceId`, `sourcePageNumber` | Numbers; source IDs are not access grants |
| `initiativeBonus` | Number or null |
| `isReleased`, `isHomebrew`, `isLegendary`, `isMythic`, `hasLair`, `hideCr`, `isLegacy` | Booleans |
| `armorClassDescription`, `conditionImmunitiesHtml`, `sensesHtml`, `skillsHtml` | Strings |
| `specialTraitsDescription`, `actionsDescription`, `reactionsDescription`, `legendaryActionsDescription`, `lairDescription`, `languageNote` | Strings; content was not retained |
| `characteristicsDescription`, `languageDescription` | String or null |
| `avatarUrl`, `largeAvatarUrl`; `basicAvatarUrl` | Strings; basic avatar can be null; no images fetched |
| `version`, `swarm` | Null; non-null contracts not established |
| `stats` | Array; sampled entries have numeric `statId` and `value`, with null `name`; six entries observed |
| `skills` | Array; sampled entries have numeric `skillId` and `value`, with null `additionalBonus` |
| `senses` | Array; sampled entries have numeric `senseId` and string `notes` |
| `movements` | Array; sampled entries have numeric `movementId` and `speed`, with null `notes` |
| `sources` | Array; sampled entries have numeric `sourceId`, `pageNumber`, and `sourceType` |
| `hitPointDice` | Object; numeric `diceCount`, `diceValue`, `diceMultiplier`, `fixedValue`, and string `diceString` |
| `savingThrows`, `languages` | Empty arrays; element contracts not established |
| `subTypes`, `environments`, `tags`, `damageAdjustments`, `conditionImmunities` | Arrays; no complete element schema established |

Production validates required identity, name, URL, envelope, access, pagination,
and bounds. It does not impose a guessed schema on optional combat or source
fields. Unknown detail fields remain in `raw`. Preserving HTML strings does not
execute them, fetch embedded resources, or establish complete stat-block text.

The second scope made one exchange and one MON-01 request with
`search=Strahd&skip=0&take=1&showHomebrew=f`. Both returned HTTP 200, with 1,509
and 1,562 decoded bytes. The list total was 8. Selected ID `6491389` had code 4;
its inspected description fields were empty or null. The check stopped at that
denied case. No selected detail request or page open followed. Unused allowance
does not authorize a replacement target.
The inspected `mythicActionsDescription` and `bonusActionsDescription` fields
were null in that denied listing. This does not establish their non-null
detail contracts.

The user then chose implementation of the verified cases now. Shared core reads,
CLI `monster list/show`, and MCP `monster_list`/`monster_show` accept code 1,
reject code 4, and return explicit unsupported errors for all other numeric
codes, including 2 and 3. Missing or malformed access evidence fails. One denied
or unsupported row fails the whole page; no rows are silently removed.
This policy does not label code 1 as a universal ownership or content grant.

List and detail both contain description fields. Their presence does not prove
complete content. Detail preserves the selected authorized raw object, including
unknown statistics, source references, and future fields. No rules version,
full source-book coverage, owned access, or shared access is inferred.
The [implementation contract](api-catalog.md#monsters-and-configuration) separates
local limits from the two observed one-record pages.

Credentials and raw responses remained in browser memory. Only structural
metadata, public IDs, counts, sizes, and field types were retained. No raw account
snapshot or proprietary text was saved. Probe references were cleared without
claiming guaranteed JavaScript zeroization. The browser login stayed unchanged.
No native credential read, native CLI/MCP account acceptance, or detail-page open
occurred. Both account scopes are consumed; further live work requires a new
approval with fixed operations, budgets, stop conditions, and cleanup.

### Approved monster filter submission trace

The user later requested website filter support in both the CLI and MCP.
A separately approved browser trace submitted the real monster form three times.
The form uses `GET https://www.dndbeyond.com/monsters`. It sends website
`filter-*` query parameters, not an observed browser request to MON-01.
The trace establishes filtered HTML from the website backend. It does not
establish equivalent monster-service JSON parameters or the website backend's
internal service calls.

The approved scope allowed three form submissions and at most five additional
explicit reads at the website, monster-service, authentication, and media origins.
Each explicit read had a 15-second deadline. JSON/HTML reads were capped at
1 MiB and public scripts at 8 MiB. The active trace had a five-minute deadline.
Access denial, bot protection, an unexpected route, or a bound breach required
a stop. Detail reads, pagination, account writes, native credential access,
and raw account snapshots were excluded.

The form exposed these encodings. An observed control does not by itself
establish the independent effect of that filter.

| Controls | Observed encoding and limit of evidence |
|----------|----------------------------------------|
| `filter-search` | Text; the submissions used `wolf` |
| `filter-type` | Fifteen checkboxes; default `0` is All Monsters; other type labels were not established |
| `filter-cr-min`, `filter-cr-max` | Option IDs, not literal challenge ratings; ID `1` means CR 0 and ID `6` means CR 2 |
| `filter-size`, `filter-environment`, `filter-alignment`, `filter-sub-types` | Multiple selections use repeated query keys; two size values were submitted together |
| `filter-senses`, `filter-save-proficiency`, `filter-skill-proficiency` | Multiple-select controls |
| `filter-resistance`, `filter-damage-immunity`, `filter-condition-immunity`, `filter-vulnerability` | Multiple-select controls |
| `filter-language`, `filter-movement`, `filter-tags`, `filter-source-category`, `filter-source` | Multiple-select controls |
| `filter-armor-class-min`, `filter-armor-class-max`, `filter-average-hp-min`, `filter-average-hp-max` | Text controls with numeric-looking submitted values; accepted numeric ranges were not established |
| `filter-is-legendary`, `filter-is-mythic`, `filter-has-lair`, `filter-partnered-content` | Empty, `t` (Yes), or `f` (No); the site selected `filter-partnered-content=f` by default |

Other observed CR options were ID `2` for CR 1/8, `3` for CR 1/4,
`4` for CR 1/2, `5` for CR 1, and `7` for CR 3. The trace did not retain
the complete CR map. Size IDs were `2` Tiny, `3` Small, `4` Medium,
`5` Large, `6` Huge, `7` Gargantuan, and `10` Medium or Small.
Movement IDs were `1` Walk, `2` Burrow, `3` Climb, `4` Fly, and `5` Swim.
Sense IDs were `1` Blindsight, `2` Darkvision, `3` Tremorsense, and
`4` Truesight. Save IDs were `1` STR, `2` DEX, `3` CON, `4` INT,
`5` WIS, and `6` CHA. Only part of the skill map was inspected.
Do not infer missing option maps or equate these website IDs with JSON
service selectors without evidence.

The first submission sent these nonempty values:

```text
filter-type=0
filter-search=wolf
filter-cr-min=1
filter-cr-max=6
filter-size=4
filter-is-legendary=f
filter-partnered-content=f
```

The second kept those values and added a second `filter-size=5` key,
`filter-is-mythic=f`, and `filter-has-lair=f`. The third kept the second
query and added `filter-armor-class-min=99`, `filter-armor-class-max=99`,
`filter-average-hp-min=1`, `filter-average-hp-max=1000`, and
`filter-movement=1`. Blank form fields could also appear as empty query values.

| Submission | HTTP status | Decoded document bytes | Observed `.listing-body .info` rows |
|------------|-------------|------------------------|------------------------------------|
| 1 | 200 | 382,606 | 4 |
| 2 | 200 | 386,779 | 6 |
| 3 | 200 | 374,910 | 0 |

Several controls changed together. These observations do not isolate every
filter's effect. The third response had no observed rows, but a narrow check
did not establish an explicit empty-result marker. It is not yet a verified
successful-empty-page contract.

Three additional explicit reads completed:

| Read | HTTP status | Bytes | Structural result |
|------|-------------|-------|-------------------|
| Credential-free [`/js/cobalt`](https://www.dndbeyond.com/js/cobalt) | 200 | 452,435 | Listing and form submission code; no monster-service origin string |
| Credential-free [`/js/waterdeep`](https://www.dndbeyond.com/js/waterdeep) | 200 | 134,636 | Relevant `filter-*` names and form submission code; no monster-service origin string |
| Browser-authenticated GET of the exact submission-2 URL | 200 | 386,779 | `text/html`; six listing rows already present in the initial response |

The script SHA-256 values were:

- Cobalt: `92925618f6bf788af005a9665bc3e477e27df672415a4a6bb288a8a699e69eda`.
- Waterdeep: `71327e8d6d49b7b268ddbebd3ae6ccf0c3559409130990e16141a2c98c367628`.

The HTML read rejected redirects and used a streamed 1 MiB cap. It was parsed
in an inert template, not executed or attached to the page. Its six rows matched
the live DOM and had numeric monster links. Only the count, not the selected
record IDs or names, was retained. Row classes included `monster-icon`,
`monster-challenge`, `monster-name`, `monster-type`, `monster-size`,
`monster-alignment`, `monster-environment`, and `monster-tags`.
Their presence is not an access grant.

No monster-service request appeared in the inspected browser resource lists.
This observation is not exhaustive network capture and cannot rule out
internal service calls by the website backend. Normal page-generated traffic
was counted separately. No explicit reads targeted ancillary analytics or
asset origins.

The trace ended after 244,133 ms with three submissions and three explicit
reads. Temporary window and session-storage references were removed. Login
was unchanged. No credentials, account snapshot, record text, or raw page
was saved. The scope is closed; its unused read allowance cannot be reused.

At this stage, advanced filters were not implemented. A safe implementation had to
establish either equivalent JSON-service selectors or a website-page contract.
The latter still needs verified paging, empty-result detection, native cookie
transport, and fresh access checks for returned data. Do not send unverified
`filter-*` keys to MON-01, filter only one already-fetched JSON page, or scan
the whole catalog to emulate server filtering. Keep source selection, access
policy, and query validation in shared core code for both adapters.

### Approved website paging and selected-ID check

The user selected website-filtered lists with explicit page-based pagination
in both adapters. This replaces the earlier `skip`/`take` list interface.
Detail reads and access checks must keep their existing behavior.

A new approved browser scope allowed six explicit requests at the website,
authentication, and monster-service origins. Each response had a streamed
1 MiB cap and a 15-second deadline. The whole scope had a five-minute deadline.
No detail reads, native credential access, account writes, raw snapshots, or
saved credentials were allowed. The check completed all six requests in
55,681 ms and removed its temporary scope reference. Login stayed unchanged.
This scope is also closed.

| Request | HTTP status | Response bytes | Structural result |
|---------|-------------|----------------|-------------------|
| `/monsters?filter-type=0&filter-search=dragon&filter-partnered-content=f&page=1` | 200 | 418,926 | Twenty rows; active page 1; next link selects page 2 |
| Exact same-filter page-2 link | 200 | 418,061 | Twenty rows; next link selects page 3 |
| Earlier combined zero-row query | 200 | 374,910 | Explicit `li.alert.no-results` marker inside the monster listing |
| Same zero-row query with `page=2` | 200 | 374,910 | Same empty-result marker; no listing pagination |
| Browser-managed authentication exchange | 200 | 1,509 | Token remained in browser memory |
| `/v1/Monster?ids=4775850&ids=17062` | 200 | 8,127 | Exactly both selected records; both code 1; ID-keyed access map; null pagination |

The exact next link preserved the nonempty filter pairs, reordered them,
and advanced `page`. Top and bottom pagination controls repeated the same
links. Active-page elements used `span.b-pagination-item.s-active.active`.
Next links used `rel="next"` inside `.b-pagination-list`. Unrelated
fragment links also inherited the current query, so collecting every link
with a `page` parameter is not a valid pagination parser.

The listing container was `div.listing-body` with
`ul.listing.listing-rpgmonster.rpgmonster-listing`. Empty results had a child
`li.alert.no-results` with the fixed message
`You failed your Investigation check.` This resolves the earlier missing
empty-result marker. It does not establish how a nonempty result handles
a page beyond its final page; only page 2 of a known empty query was checked.

The complete CR option map showed a gap: CR 23 uses ID `27`, CR 24 uses
ID `29`, and CR 30 uses ID `35`. Do not derive all IDs by adding a constant
to the literal rating. Other option groups were present, but not every
individual filter value was submitted. Type checkbox IDs included `0`,
`1`, `2`, `3`, `4`, `6`, `7`, `8`, `9`, `10`, `11`, `13`, `14`,
`15`, and `16`; individual type labels were not established by this check.

The selected-ID collection returned `accessType`, `pagination`, `stats`,
`metaData`, and `data`. Pagination was null. This extends the earlier
single-ID observation to one two-ID request. It does not verify an upstream
maximum batch size or owned/shared cases. A future filtered-list implementation
must check the exact requested ID set, reject duplicate or extra records,
and apply the existing fresh code-1 policy before returning any row.
Website order must not depend on collection response order.

Only public filter metadata, page structure, counts, selected IDs, and
access classifications were retained. No raw HTML, raw record, account
snapshot, or credential was saved. No native CLI/MCP account acceptance
was performed.

### Shared filtered-list implementation

The user approved implementation of the website-page direction. Shared core
queries now expose all observed filter controls through both adapters.
CLI repeated ID flags and MCP arrays use the same validated website encodings.
Literal CR bounds use the complete observed map, including the missing ID 28.
Unknown MCP fields, explicit null selectors, invalid ranges, duplicate/zero IDs,
and local limit violations fail. Requested option IDs must exist in the
same response's public filter form.

List now reads one cookie-only website page, then one bearer-only selected-ID
collection for nonempty pages. Empty results require the observed marker and
no collection read. One session identity spans both reads. Every selected
identity and fresh code-1 grant must match before output; the core restores
website order. Detail and its independent fresh access check are unchanged.
Pagination is `page`/`next_page`, not the earlier `skip`/`take` interface.
The source controls page size; total counts are not inferred.

Synthetic checks cover filter serialization, CR mapping, strict input,
page/empty parsing, fixed URLs, exact selected sets, access errors, session
identity changes, size bounds, cancellation, and actual CLI/MCP process parity.
These checks do not replace native cookie compatibility, native account
acceptance, larger-batch evidence, or owned/shared evidence. No new account
calls followed the closed research scopes.

## Books, sources, and access checks

The initial survey did not establish book delivery. A later approved browser inspection
established a library route and HTML outline/section structure for one selected source.
The earlier native probe accepted the protected-session exchange and source HTML, but stopped
at a chapter redirect. A user-requested repeat identified a marketplace or purchase target.
A separately approved cookie-only native check later accepted the selected chapter HTML
with expected literal markers. It did not establish DOM structure or content authorization.
A further approved check parsed the source and user-selected non-preview chapter,
then stopped at the anonymous control's HTTP 307 without following it.
Fresh content access and general book support remain
unresolved. No book-content JSON endpoint or source-to-license mapping was established.
The earlier failed credential-free `/sources` fetch was not proof of an access denial.

### Selected book browser evidence

The user selected [Curse of Strahd][selected-book] and approved at most three normal page opens:
the library, the selected book, and one selected section.
The inspection consumed all three opens. It did not open another chapter, call an API
directly, collect request headers or bodies, return book text, or change login or protected
storage. The pages made their normal background requests.

| Page | Observed canonical path | Browser response | Decoded document bytes |
|------|-------------------------|------------------|------------------------|
| Library | `/en/library`, after one redirect from `/sources` | 200 | 2,654,622 |
| Selected source | `/sources/dnd/cos` | 200 | 246,103 |
| Selected chapter | `/sources/dnd/cos/introduction`, after one redirect from the listed `/sources/cos/introduction` link | 200; `text/html` | 257,524 |

These are browser navigation measurements, not native-client response measurements.
The library exceeds the current 1 MiB product response limit of 1,048,576 bytes.
Compression does not establish that the decoded response fits that limit.
Do not silently increase the limit or download the oversized page and filter it afterward.
A smaller metadata transport or an explicitly approved limit change is still needed.

The library uses Next.js script assets and exposes a `sources` array through the selected
card's component ancestry. The selected record was matched by its known source path.
Only field types, source identifier metadata, and selected access classifications were read.
No complete source or account record was returned.

Observed source fields include numeric `id`; string `name`, `relativePath`, `marketplaceUrl`,
`sku`, `publisher`, `type`, and `releaseDate`; object `image` and `ruleset`; and boolean
`isThirdParty`, `isFree`, `isReleased`, `isOwned`, `isSharedWithMe`, `isFavorite`,
`isOnWishlist`, and `isLegacy`.
The selected catalog source identifier was `6`. This is an example source ID, not an account
or license ID, and must not be hardcoded as a general mapping.
The component data does not establish a separate JSON endpoint, complete pagination, or
a fresh authorization result.

The selected source has an HTML table of contents in `.compendium-toc-full-text`.
There were 81 anchors, including 79 targets under the selected book's source paths.
Those selected-book targets included 25 distinct chapter paths and 53 fragment targets.
All 79 had nonempty labels; one target was repeated.
Chapter links were under headings, and section links used lists with observed depths zero
and one. Count raw entries before filtering or normalization.
Repeated targets and other link roles need explicit handling; do not silently discard
conflicting or invalid outline entries.

The listed introduction section used `/sources/cos/introduction#RunningtheAdventure`.
Normal browser navigation redirected to the `/sources/dnd/cos/introduction` path and retained
the fragment. The page also supplied that path in its canonical link.
The fragment matched an `h2` with ID `RunningtheAdventure` inside `.p-article-content`.
The next same-level heading provides a structural section boundary, with three lower-level
headings between them. The chapter container had 44 paragraphs and 13 headings with IDs.
No text extraction or 4,096-byte output check was performed.

This establishes a possible HTML parsing contract for the selected source, not a JSON
section API. The browser receives a chapter document and selects the fragment locally.
There was no observed server request for just that fragment or a byte-limited text window.
Native reads would need a reviewed HTML transport, content-container and heading validation,
safe text conversion, and explicit UTF-8 continuation within the existing output limit.
The current resource operation is `Session::get_json`; this inspection does not add HTML
support to it.

Resource timing showed `/api/vcm/33-15` on the source page and `/api/vcm/41-348` on the chapter.
Their purposes and methods were not established; they must not be called book-access APIs.
The chapter also made normal authentication, warning-acknowledgement, and communication
requests. Resource timing can be incomplete and does not prove that no other service exists.

The current browser could render the selected chapter without an observed content restriction.
This does not prove that the chapter requires ownership, that library flags are fresh grants,
or that a bearer-only native request has the same access.
Separate bounded checks are still needed for native authentication, expired or denied access,
preview-versus-protected content, and campaign sharing.
Do not treat an introduction or a successful HTML response as proof of full-book access.

Production redirects remain disabled. The observed canonical routes are evidence for this
source only, not permission to follow arbitrary redirects or guess aliases for other books.

### Approved native website cookie check

On 2026-11-06, the user explicitly approved a separate cookie-only investigation
after the earlier bearer-only purchase redirect. The scope was one protected-entry
read and one GET to `https://www.dndbeyond.com/sources/dnd/cos/introduction`.
It permitted this diagnostic transport, not an automatic production fallback.
No different identity, bearer exchange, other credential, browser impersonation,
proxy, other page, redirect, retry, or stored-session change was permitted.

The [ignored Windows probe](testing.md#website-cookie-probe) passed its one live
request. HTTP 200 returned identity-encoded `text/html` with 257,524 bytes,
within the 1 MiB cap. The fixed `p-article-content`, `RunningtheAdventure`,
and canonical-URL literals were present. Known challenge markers were absent.
The result remained `content_access=unverified`.

These are byte-literal observations, not DOM parsing or a comparison of complete
documents. No text was extracted or retained. The response byte count equals the
earlier browser measurement; that does not prove identical content.
The protected entry was unchanged, the acknowledgment was removed, and the
test process exited. The scope is consumed. Another request needs new approval.

This resolves whether the native client can receive HTML for this selected route
with the existing cookie. It does not show whether an introduction is a preview,
whether another chapter is protected, how to identify a content-access decision,
or how general outline and section parsing must work. Source mapping, fresh
authorization, revoked access, and full-book access remain separate contracts.
Do not treat HTTP success or marker presence as an access grant.

### Approved protected chapter structure and control

The user next selected `https://www.dndbeyond.com/sources/dnd/cos/into-the-mists`
as a non-preview chapter. On 2026-11-06, a separate approval permitted one
protected-entry read and at most three ordered GETs: the source with the existing
cookie, the selected chapter with that cookie, and the same chapter without
credentials as the final control. The scope allowed transient HTML parsing, not
text output, crawling, another chapter, redirects, retries, or credential changes.
The same 5-second connection, 15-second request, 30-second complete, 1 MiB body,
and 1,000-raw-link limits applied.

The [structure probe](testing.md#protected-chapter-structure-probe) completed
exactly three attempts. The source returned HTTP 200 with 246,103 HTML bytes.
Its canonical link matched the selected source. One parsed outline container
held 81 raw anchors: 26 chapter links, 53 fragment links, and 2 other links.
There was one duplicate target, no empty label, and no unsafe target under the
probe's classification. All 79 book links used the observed legacy prefix.
The selected chapter had three distinct outline fragment targets.

The authenticated chapter returned HTTP 200 with 356,402 HTML bytes and its
expected canonical link. One article container had 35 headings with IDs:
one `h1`, three `h2`, twelve `h3`, and nineteen `h4`.
All three selected outline fragments matched article IDs.
There were 41 unique IDs and no duplicates, 278 paragraphs, 7 list items,
and one table. Its article had no marketplace link and no tested hidden
attribute on the container or an ancestor. Known challenge elements were absent.

The article contained 56,814 raw text-node bytes and 96 elements in the diagnostic
`script`, `style`, `nav`, `aside`, and `form` group. These are counts, not extracted
or normalized reader text. That group is not an extraction exclusion list:
book sidebars can contain required content.

The anonymous control returned HTTP 307. Its body and redirect target were not
read or followed. That is a response difference, not proof of the restriction's
cause or a general entitlement API. Output remained `content_access=unverified`.
An allowed chapter must never grant a later chapter or continuation request.

Parsing used the test-only [scraper 0.27.0 API][scraper-docs] with default
features disabled. No external runtime was added. Raw HTML, text, identifiers,
headers, and credentials were not retained. Private fragment sets existed only
to compare targets within this process. Parsed DOM and dependency-owned copies
cannot be guaranteed to be erased. The fixed entry was unchanged.
The acknowledgment was removed and the process exited. This scope is consumed.

### Approved page-level book contract

The user later approved the [page-level book contract](architecture.md#live-book-reads)
for offline production implementation. Fresh positive Web-library evidence is
necessary but insufficient. The current native cookie response must also pass
canonical-source, structure, and selected-target checks. Each explicit
continuation repeats them within one session identity. Unknown access,
unowned previews, ambiguous layouts, and redirects return no content.
An unclassified redirect does not prove ownership denial. No separate
entitlement API is claimed. The separately approved native acceptance below
passed for one selected chapter and its initial chunk.
This decision does not approve more credential access or live requests.

### Native book adapter acceptance

On 2026-11-06, the user separately approved a fixed native CLI/MCP acceptance
scope for `/sources/dnd/cos` and the previously selected
`/sources/dnd/cos/into-the-mists` chapter. One CLI `book list` resolved the numeric
ID in memory. Each adapter then returned one outline and one initial chapter
chunk. All five operations passed. The outlines matched with 79 entries.
The chunks matched with 3,929 UTF-8 text bytes, source attribution, and the same
output envelope. No continuation or other chapter was read.

The maximum approved scope was 31 HTTP attempts and 16 protected-entry loads
within 180 seconds. These bounds cover five fixed operation sequences, including
at most two service exchanges per operation; they are not measured wire counts.
Allowed operations were POST `https://auth-service.dndbeyond.com/v1/cobalt-token`,
GET `https://www.dndbeyond.com/api/config/json`,
GET `https://api.dndbeyond.com/library/sources?platform=Web`,
GET `https://api.dndbeyond.com/library/sources/me`, and cookie-only GETs for the
fixed source and chapter. Redirects, retries, login, credential changes,
installation, and further content reads were excluded. Any failure would stop
the remaining sequence.

The temporary helper first passed synthetic success and stop-condition checks,
including native refusal without the approval guard. Its one native run used the
built product executables, not fixture entry points. JSON, text, tokens, and
account records stayed in process memory; output contained only safe counts and
comparison flags. No raw content or account snapshot was retained.
The helper and consumed guard were removed, and child processes exited.
This scope is consumed. Another live run needs new explicit approval.

This result establishes native adapter parity for the selected source and
initial chapter chunk. It does not establish live continuation, shared-access
behavior, compatibility with other books, or a separately verified entitlement
API. It does not expand the manual capability suite's zero live-request budget.

### First-party library JSON routes

Public source inspection for [issue #32][issue-32] on 2026-11-04 found library JSON
routes in the [first-party mega-menu application][menu-client].
The inspected script returned HTTP 200 and contained 629,971 bytes, within the
1 MiB inspection cap. No signed-in request, credential access, library API request,
or book-content request was made for this inspection.

The production host helper `Is` selects `api.dndbeyond.com`. The `Nf` helper uses
GET and parses JSON. The signed-in search flow obtains the authentication result
through `Rf`, then calls the source and ownership helpers.
These declarations are Client evidence. A later separately approved native check
is recorded below; it does not establish every response or access condition.

| Helper | Method and URL | Observed client use |
|--------|----------------|---------------------|
| `MR` | GET `https://api.dndbeyond.com/library/sources?platform=Web` | Source metadata candidate |
| `OR` | GET `https://api.dndbeyond.com/library/sources/me` | Account ownership candidate; the search flow reads `items` |

The `JD` search helper joins source metadata to ownership records by `sourceId`.
It reads `name`, `ruleset.label`, `publisher.isThirdParty`, and `isFree` from sources.
It tests each ownership record's `ownership` array for `type` equal to `owned`
or `shared`. It combines these states with free-source metadata into one
`isOwnedOrShared` search-filter flag.
That combined flag cannot preserve Beyond Gate's separate purchase and sharing evidence.
The helper also has fallback IDs and error-to-empty behavior; neither belongs in
production discovery.

The source-link helpers `NN` and `F_` use `relativeSlugPath` when present, otherwise
the `Hw` helper builds `/sources/dnd/{slug}` from `slug`.
This is first-party URL-building evidence, not proof that every source supplies a
valid path. The probe below did not inspect `relativeSlugPath`.
Do not silently discard sources with missing paths or substitute a homepage URL.

This resolves the service binding and identifies a JSON alternative to the oversized
`BOOK-02` document. Complete discovery, per-response record bounds, source-URL field
coverage, sharing-context fields, and revocation semantics remain unverified.
Do not infer unavailable access from an absent ownership record or invent a sharing
context from an account-level flag.

The browser request helper can use `AwsWafIntegration.fetch`. Its presence does not
establish that native access works or authorize a bot-protection workaround.
Stop on native denial or bot protection; do not copy browser fallback behavior.
No production host is enabled by this source inspection or the probe below.

The [official My Library announcement][library-announcement] confirms the user-facing
Owned, Owned & Shared, and All filters. It does not document these JSON schemas.
Issue #32 keeps the 1 MiB response limit. Bounded native responses and complete
evidence semantics must be established before implementation.
Prefer these first-party GET candidates over the unverified mobile credential-body
transport; do not call either route without the required live-check approval.

### Approved native library metadata check

The user separately approved the three-attempt scope on 2026-11-04 after autopilot
was disabled. The check used the fixed protected Windows entry, one existing
cookie-to-bearer exchange, and bearer-only GETs to the two exact library routes.
It did not save, replace, or delete credentials.
It made exactly three HTTP attempts, with no redirects, retries, proxies,
pagination, book reads, or other resource calls.
The one-use request scope is consumed; it does not approve another run.

| Request | Result | Safe observation |
|---------|--------|------------------|
| POST `AUTH-01` | Accepted | Existing shared exchange validation accepted the protected session |
| GET `LIB-01` | 200; 123,198 bytes | Root object with `items` array and numeric `totalCount` |
| GET `LIB-02` | 200; 2,104 bytes | Root object with `items` array and numeric `totalCount` |

Both metadata responses were uncompressed JSON within 1 MiB.
The metadata records had numeric `sourceId`, string `name`, boolean `isFree`,
object `publisher` and `ruleset`, and an `ownership` array.
`slug` and `marketplaceUrl` were either string or null.
Ownership records had numeric `sourceId` and an `ownership` array of objects with
string `type`. The check observed `owned`, no `shared`, and no unknown ownership
type in the account ownership response. This is not proof that sharing is unsupported
or that all shared access was enumerated.

Output contained only fixed structural paths, JSON types, response byte counts,
and safe presence classifications. Unknown field names were replaced with a marker.
No headers, identifiers, titles, URLs, response values, or raw responses were saved.
The probe did not compare `totalCount` with the returned record count, validate
terminal completeness, or establish paging parameters. It did not establish the
meaning of absent ownership, sharing-context identity, or sharing loss.
Those are still contract blockers for full production discovery.

The user then separately approved one follow-up with the same three-attempt maximum.
It compared counts, raw-record thresholds, and source paths using safe pass/fail
classifications only. It stopped after the exchange and source GET: two attempts.
The source response again contained 123,198 bytes. Its `totalCount` equaled the
returned record count, and source identifiers were unique positive integers.
The response stayed within 1,000 records but exceeded the existing 100-record page cap.
Not every record matched the probe's conservative canonical book-path rule.
This does not prove that the unmatched records have no valid website location;
their path mapping still needs a contract.

The follow-up reported a failed contract check and skipped the ownership GET.
It did not verify ownership completeness or the source-to-ownership join.
No retry or policy change occurred. Its scope is also consumed, including the
stop condition; the unused attempt does not authorize another request.
Do not silently raise the page cap, split the response into invented pages, or
discard unmatched sources to turn this result into successful discovery.

The user subsequently approved three explicit contract choices: a separate
1,000-record raw cap for the complete catalog, a supplied validated marketplace
URL when a book path is absent, and account-level sharing without an invented
context. Ownership keeps the 100-record cap. Missing ownership stays unknown,
not unavailable. These choices are implemented in
[shared discovery](architecture.md#live-library-discovery), with synthetic
transport and actual CLI/MCP process checks. Runtime count equality and source
joins must pass before any complete result. Live adapter acceptance, real shared
observations, and schema-change compatibility remain unverified.

The ignored [metadata probe](../src/beyond-gate-core/src/auth/session/library_probe.rs)
has synthetic checks for explicit approval, redaction, malformed shapes, byte and
attempt boundaries, count and source-join checks, and stopping without retries
or redirects. Both live entry points are ignored and separately acknowledgment-gated.
See [probe operation](testing.md#library-metadata-probe) for its safety limits.
Neither probe is CLI/MCP production acceptance.

### Native adapter rejection and catalog diagnostics

A later separately approved CLI/MCP acceptance scope stopped during the CLI
catalog validation with `InvalidMetadata`. MCP and ownership were not requested.
Only one exchange and one catalog GET occurred. Raw results stayed in memory.
The scope was consumed; no unused attempt was reused.

A separately approved catalog-only diagnostic then made two attempts. It found
invalid titles, missing book/marketplace attribution, and marketplace links on
`www.dndbeyond.com`. It printed fixed class flags only.

The user also approved public browser inspection. The signed-out library page
exposed released-source UI models, nonblank names with padding/control formatting,
and legacy WWW marketplace links. No login, source-content navigation, or
credential transfer was used. Client filtering and attribution were inspected in
the public library page and its
[page script](https://www.dndbeyond.com/_next/static/chunks/app/%5Blocale%5D/library/page-794b0c736763f2f7.js).
Rendered UI models do not prove a safe selection rule for the raw API catalog.

The user approved a conditional diagnostic/fix/acceptance scope with at most ten
native attempts. The two-attempt diagnostic verified Boolean `isReleased` on all
records, but rejected both required conditions: missing attribution was not
limited to unreleased records, and the proposed title/legacy-link mapping did not
validate every released record. The test failed explicitly. No conditional parser
changes or CLI/MCP rerun occurred. The whole scope is consumed.

There were 11 explicit native attempts across these first five scopes. Public asset and
browser requests are separate from this native count. No source values, names,
account snapshots, or credentials were saved. Complete live discovery remains
blocked by source selection and attribution contracts. Do not drop invalid
records, invent names or URLs, or treat the rejected release rule as verified.

### Alternative configuration catalog comparison

The user approved a separate three-attempt comparison on 2026-11-04 in interactive
mode. It requested `CFG-01` without credentials, used the existing protected
exchange, then requested `LIB-01` with the derived bearer. The overall deadline
was 60 seconds. Both catalogs retained the 1 MiB cap. There were no retries,
redirects, ownership requests, book-content reads, or credential changes.
Only counts and fixed flags were printed; raw records stayed in memory.

The configuration response contained 268 sources and 50 categories. All 177
library source IDs matched configuration IDs. Seven library records failed the
existing metadata mapping; four had no supplied path, slug, or marketplace link.
This proves ID coverage for this response, not valid attribution or book eligibility.

The comparison reported zero valid configuration records and failed
`InvalidMetadata`. Subsequent public-code inspection found a diagnostic defect:
it accepted absolute URLs and slash-prefixed paths, but rejected the relative
`sources/dnd/...` form used by the importer's
[page-path helper][importer-page-import]. The zero result does not establish that
the configuration catalog lacks usable links.

The test-only diagnostic now handles those supplied relative source paths through
the existing strict path validator. Synthetic guards cover relative, rooted, and
absolute source links and reject unsafe paths. No production metadata mapping
changed.

The user then approved one corrected comparison under a fresh three-attempt
scope with the same limits. It found 209 valid configuration metadata records,
266 valid titles, and 211 nonblank relative source paths among the 268 sources.
All 177 library IDs still matched configuration IDs, but only 170 had valid
configuration metadata. None of the four library records with missing attribution
obtained valid replacement metadata from this mapping. The diagnostic failed
`InvalidMetadata` and stopped without ownership or content requests.

The two comparisons add six native attempts: 17 total across seven consumed
scopes. The corrected mapping is tested live, but does not establish a complete
library. No further live request is approved. The configuration catalog does not
resolve the observed missing attribution through this mapping. The remaining
contract must establish which source records are books and where their links
come from. Complete CLI/MCP acceptance and ownership joins remain unverified.
Category exclusions from another application are not a complete-library contract.

### Public library rendering boundary

The user approved a public-code trace on 2026-11-05, not another account probe.
The [library page client][library-client] and [mega-menu client][menu-client]
were inspected without executing their code. No protected credentials, account
API calls, source pages, or book content were used. The oversized library HTML
document was not downloaded for this trace. The native attempt count remains 17;
the seven earlier account-request scopes remain consumed.

The page chunk's module `97398` exports `SourcesContents`. It accepts a prepared
`sources` array, copies it into local state, and passes those records to the list.
The following behavior is visible in that client boundary:

| Area | Verified client behavior | Limit |
|------|--------------------------|-------|
| Initial records | `SourcesContents` uses its supplied `sources` prop | No raw `LIB-01` deserialization or record-to-book mapping is shown |
| List filters | Ownership, ruleset, type, publisher, and title search act on prepared records | These user filters do not establish which raw source records belong in the library |
| Type | Selected filters test `type` for `adventure` or `sourcebook`; the default accepts either and other supplied types | No universal raw-source book predicate is established |
| Release state | The displayed list filter has no `isReleased` condition | Earlier released-only rendered models do not prove a release-only selection rule |
| Source link | The card uses the supplied `relativePath` as its link | This component does not derive that field from `slug` or `relativeSlugPath` |
| Marketplace button | Uses supplied `marketplaceUrl`, but hides the button for owned, legacy, or SKU-less records | Marketplace attribution is not a universal card fallback |
| Title | The card displays its supplied `name` | No title-whitespace normalization is shown in this component |
| Fallback status | An error banner can display `staticSourcesFallback` | The fallback loader and its selection rules are not shown; the flag's presence does not prove it was active |

The module also declares server references for favorites, wishlist, and recently
opened changes. None was invoked. Clicking source links can record recently
opened state; no such link was opened during this trace.

The raw-to-rendered mapping occurs before the inspected component. Its
implementation was not found in these public assets. This is a limit of the
inspected code, not proof that the implementation is unavailable everywhere.
In particular, this trace did not establish why the earlier rendered models
had attribution while some native catalog records did not.

The menu's `F_` helper serves a different purpose: it joins favorites and recently
opened entries to source metadata. It skips unmatched entries or entries without
a usable path. Its callers limit favorites to 15 and recent entries to three.
It is not the complete library renderer and does not justify discarding raw
catalog records. The separate search helper `JD` combines free and owned/shared
observations into a search flag; that flag remains unsuitable as purchase evidence.

The inspected menu asset was 629,971 bytes, within 1 MiB. Its SHA-256 was
`304e83268a7d349eef6d436f02ec25eba65fdc9ed03058c17dcb6c533f7906a0`.
No new production filter, title normalization, or URL fallback was enabled.
Complete library discovery remains blocked by the upstream selection and
attribution contract, not by an untested client-side filter found in this trace.

### Independent source-picker and category evidence

The user selected continued public contract research with strict output unchanged.
This bounded pass inspected three independent pinned implementations. It did not
repeat the library-page trace, execute third-party code, or read book content.

| Implementation | Verified behavior | Limit for Beyond Gate |
|----------------|-------------------|-----------------------|
| [Scraper source picker][scraper-source-filter] | Resolves category IDs from the case-insensitive name `Hidden Sources`, then excludes matching sources | This is its picker policy, not proof of the complete `LIB-01` selection rule |
| [Scraper configuration client][scraper-source-catalog] | Reads `CFG-01` source IDs, names, descriptions, and category IDs; does not retain source URLs | Cannot supply missing attribution; skips malformed IDs and keeps the first duplicate |
| [TypeScript source parser][sdk-source-parser] | Parses a selected source page's title, breadcrumbs, and article body from supplied DOM | Not library discovery, a JSON source model, or an account-access check |
| [Sources userscript][sources-userscript] | Reads old page CSS classes, derives title-based keys, and stores owned/shared selections in browser storage | No verified numeric source-ID join; local defaults and DOM heuristics do not meet the strict contract |

The scraper's [endpoint notes][scraper-endpoint-notes] report a first-party
`Hidden Sources` category intended to hide entries from source selections and
listings. This was a useful lead, not permission to discard the four unresolved
library records. Its unresolved entitlement implementation also does not negate
the stronger native evidence for `LIB-02`. Do not adopt its show-all fallback,
silent duplicate removal, or weaker metadata requirements.

A separate public-metadata check made exactly one credential-free GET to
[CFG-01][public-config] on 2026-11-05. It disabled cookies, proxies, redirects,
and retries, with a 5-second connection deadline, a 15-second overall deadline,
and a 1 MiB body cap. It stopped on non-200 status, unexpected content encoding,
invalid JSON, an invalid array shape, or more than 1,000 sources or categories.
Only fixed flags and counts were printed. No raw response was saved.

| Public configuration observation | Result |
|----------------------------------|--------|
| Response bytes | 713,202 |
| Sources / categories | 268 / 50 |
| Categories named `Hidden Sources` | One |
| Expected description phrases | Contains `hidden` and `source selections/listings` |
| Sources assigned to that category | 72 |
| Sources with a null or empty `sourceURL` | 57 |
| Such sources in `Hidden Sources` | 51 |
| Such sources outside `Hidden Sources` | Six |

This confirms the category and expected description phrases in current
first-party metadata. It does not prove that the earlier four library records
belong to that category. Those account-request records were not retained or
requested again. The six other missing links also disprove a general assumption
that every source without a link belongs to `Hidden Sources`.

The remaining specific hypothesis is a category-ID join between `CFG-01` and
`LIB-01`. A future separately approved check must establish which library records
the category covers and whether every remaining book has valid metadata.
It must also establish how excluded source observations interact with the complete
ownership join. Do not silently drop ownership evidence, use missing links as the
selection rule, or treat a hidden category as an access denial.
Neither current public evidence nor the inspected projects establishes that full
contract. No production filter, title normalization, or URL fallback was enabled.

The earlier seven protected/mixed scopes remain consumed at 17 attempts.
This separate one-request public scope is also complete: 18 direct native
authentication, library, and configuration attempts in total for issue #32.
Public script, browser, and GitHub requests are outside that count.
No new protected-storage access, exchange, account API call, or login occurred.
This research does not authorize another account check.

### Approved hidden-category comparison

The user separately approved a diagnostic on 2026-11-06 in interactive mode.
It made exactly three attempts: credential-free `CFG-01`, the existing protected
exchange, and bearer-only `LIB-01`. The overall deadline was 60 seconds, with
1 MiB catalog caps and the existing authentication response cap.
No retry, redirect, proxy, ownership call, book read, login, credential change,
or raw-response retention occurred.

Before protected-session access, the diagnostic required bounded configuration
arrays, unique positive source/category IDs, valid category names, exactly one
case-insensitive `Hidden Sources` category, and known category references for
every configuration source. It then required exact library counts and unique,
positive IDs that all joined to configuration. Synthetic tests cover those
guards and reject unresolved metadata even when all missing links are hidden.

| Observation | Result |
|-------------|--------|
| Configuration sources / hidden sources | 268 / 72 |
| Library records / hidden library records | 177 / eight |
| Library records without attribution | Four |
| Such records in `Hidden Sources` | Four |
| Remaining library records | 169 |
| Remaining records failing existing library metadata validation | Two |
| Remaining records failing configuration metadata validation | Two |
| Remaining records with neither valid mapping | Two |

This confirms category membership for all four currently missing-link records.
It does not establish their identity across earlier requests, because source IDs
were not retained. The category alone is not a complete fix: two other records
still fail both strict mappings. The diagnostic did not report their titles,
URLs, or specific failure classes. Do not infer the cause or assume that display
normalization would repair them.

The check returned `InvalidMetadata` and stopped. No production category filter,
metadata fallback, or adapter acceptance rerun was approved or performed.
The complete ownership join and real sharing coverage also remain unverified.
This scope is consumed. Its three attempts bring the issue #32 native
authentication/library/configuration total to 21, including the earlier separate
public-only configuration read. A further account request needs fresh approval.

### Approved metadata failure-class diagnostic

The user approved one new diagnostic-only scope on 2026-11-06 in interactive
mode. It reused the category comparison entry with a fresh acknowledgment and
rejected the consumed category-scope acknowledgment. Synthetic tests checked
fixed failure labels, redaction, whitespace-only title cleanup, unsafe links,
missing titles, and preservation of malformed-path rejection before live access.

The diagnostic made three attempts: public `CFG-01`, the existing protected
exchange, and bearer-only `LIB-01`. It kept the earlier 60-second total deadline,
catalog and authentication response caps, and no retry, redirect, proxy,
ownership call, book read, automatic login, credential change, or raw retention.
Only counts and fixed failure-class labels were printed.

| Observation | Result |
|-------------|--------|
| Library records / hidden library records | 177 / eight |
| Records without attribution / such records in `Hidden Sources` | Four / four |
| Remaining records failing both current metadata mappings | Two |
| Remaining library titles with padding | Two |
| Remaining library titles with control characters | One |
| Corresponding configuration title failures | Same padding and control-character counts |
| Link-validation failure classes for those remaining records | None reported |
| Remaining failures after the proposed library mapping | Zero |
| Remaining failures after configuration-title whitespace cleanup | Zero |

The proposed library mapping uses the earlier whitespace cleanup and supplied
legacy-marketplace-link trial. The configuration trial changes only its supplied
title's whitespace; it leaves the supplied URL unchanged. Both trials validated
metadata for all 169 non-hidden records. The library failure classes were limited
to title formatting; this result does not justify a broader URL allowlist.
Neither trial invents a title or link or removes an unresolved visible record.

The diagnostic passed. This establishes a concrete metadata-mapping candidate,
not complete native discovery. Production selection and title normalization were
not approved by this diagnostic scope and remain unchanged. The ownership join,
native CLI/MCP acceptance, and real sharing coverage still need verification.
This scope is consumed. Its three attempts bring the issue #32 native
authentication/library/configuration total to 24.

### Approved production category and title mapping

The user subsequently approved implementation on 2026-11-06. Production now
reads credential-free `CFG-01` before protected library requests. It uses only
source/category IDs and category names, not configuration titles or links.
It resolves exactly one `Hidden Sources` category by case-insensitive name and
excludes those sources only after validating their raw ownership observations.
Ownership IDs must match the full `LIB-01` catalog, not merely configuration.
Invalid visible title whitespace is trimmed and collapsed before strict
validation. Valid titles remain unchanged. URL rules are not widened.

Synthetic checks cover invalid classification, missing joins, hidden ownership,
unknown hidden evidence, hidden conflicts, raw limits, title cleanup, and strict
URL rejection. Actual CLI/MCP process checks cover the same selected output and
safe failures. Fresh native adapter acceptance is pending and requires separate
approval. No new account request is included in implementation approval.
The consumed native authentication/library/configuration total remains 24.

### Native selected-library acceptance

The user separately approved one CLI run followed by one MCP run only if CLI
passed. The one-use scope permitted at most ten HTTP attempts: five per adapter,
using only public `CFG-01`, the existing protected exchange, `LIB-01`, and
`LIB-02`. Each discovery retained its 30-second deadline; the overall check had
a 90-second deadline. It allowed no retry, automatic login, credential changes,
book-content reads, or raw retention. The harness passed an offline self-check
before native execution.

The native CLI passed public classification and catalog preparation, then returned
`UnmatchedOwnership`. This proves that at least one valid ownership ID was absent
from the full raw catalog, not merely absent from selected output. It does not
identify that record's category, title, access context, or reason for omission.
The failure is not a login rejection. No raw metadata or IDs were printed or
saved. MCP was skipped and the owned CLI process ended.

The scope is consumed. At this checkpoint, the production guard remained strict; neither configuration
membership nor a granted observation supplies missing catalog metadata. A new
synthetic scenario checks the same refusal through actual CLI/MCP processes,
including empty CLI stdout and MCP `isError: true`.
No further account request is approved.

The earlier 24 measured native attempts remain the exact historical count.
The latest CLI run used at most five further attempts; its exact HTTP count was
not instrumented. The cumulative ceiling is therefore 29, not an asserted exact
count. All unused attempts from the failed scope are discarded.

### Approved ownership membership diagnostic

The user approved one new diagnostic-only scope on 2026-11-06, in interactive
mode. It allowed public `CFG-01`, protected `LIB-01` and `LIB-02`, and at most
one existing exchange before each protected GET: five HTTP attempts at most.
The whole diagnostic had a 60-second limit. Existing body limits, no retries,
no redirects, and no content reads or credential changes remained in force.
The diagnostic printed fixed labels and counts only. Three synthetic tests
checked classification, strict failure handling, redaction, and a fresh approval
gate before native access. All older acknowledgments were rejected.

| Observation | Result |
|-------------|--------|
| Raw / unique Web catalog records | 177 / 177 |
| Visible Web catalog IDs | 169 |
| Raw ownership records / unique IDs / raw observations | 44 / 44 / 44 |
| Ownership IDs present in the raw Web catalog | 37 |
| Ownership-only IDs in configured hidden categories | Two |
| Ownership-only IDs in configured visible categories | Five |
| Ownership-only IDs absent from public configuration | Zero |
| Production snapshot valid under the then-current strict join | No |

The diagnostic passed as a classification check, not as discovery acceptance.
Production did not change during the run. No raw metadata, IDs, titles, URLs,
or credentials were printed or retained. The scope is consumed. Hidden-only
ownership exclusion is insufficient because five ownership-only IDs are visible.
This result does not establish why those sources are absent from the Web catalog.

The earlier cumulative ceiling was 29. This diagnostic adds at most five attempts;
the cumulative ceiling is now 34. Exact HTTP counts for these later shared-session
runs were not instrumented. The earlier measured count of 24 is unchanged.

### Approved Web-catalog result scope

A credential-free follow-up inspected the same [public menu client][menu-client]:
629,971 bytes, SHA-256
`304e83268a7d349eef6d436f02ec25eba65fdc9ed03058c17dcb6c533f7906a0`.
Two bounded public-script reads were used because the first lookup expected
function declarations; the actual helpers are variable assignments. No script
was executed or persisted. These public-script reads are outside native
authentication/library/configuration accounting.

The `JD` search helper builds an ownership-ID set, then maps the supplied source
records to result entries. Ownership-only IDs do not create entries. `MR` selects
the Web catalog while `OR` reads account ownership without a platform parameter.
This is first-party search behavior, not proof that the full-account library and
Web catalog have identical scope. No fallback IDs, free-source grants,
error-to-empty results, or bot-protection integration were adopted.

The user explicitly selected **Use the Web catalog as the result scope**.
Production now requires every raw ownership ID to exist in validated public
configuration, and validates all ownership types, counts, and limits.
Configured ownership-only sources, hidden or visible, do not create Web result
entries or imply access denial. Visible Web catalog records still require strict
metadata. All catalog IDs must match configuration. Hidden-category and title
rules remain unchanged; configuration supplies no replacement titles or URLs.
Ownership for one ID never grants access to another.

Synthetic and actual CLI/MCP process checks cover both configured ownership-only
classes, unconfigured IDs, invalid out-of-scope evidence, raw caps, complete
empty Web catalogs, and unchanged in-scope grants. Fresh native acceptance is
pending and needs separate approval. This scope decision authorized production
changes, not another account request.

### Native Web-catalog adapter acceptance

The user separately approved a fresh one-use CLI/MCP check on 2026-11-06, in
interactive mode. CLI ran first; MCP could run only if CLI passed. The scope
permitted at most ten attempts total, five per adapter, using only public
`CFG-01`, the existing protected exchange, `LIB-01`, and `LIB-02`.
Each discovery retained its 30-second limit; the full check had a 90-second limit.
The first access, expiry, redirect, rate-limit, bot-protection, data-validation,
timeout, or result-check failure would stop the run. No retry, automatic login,
credential change, content read, or raw retention was permitted.

The memory-only harness passed its offline self-check. Both real native binaries
then passed. The check compared all IDs, titles, source URLs, access summaries,
and evidence in memory. CLI returned valid JSON with empty stderr. MCP completed
initialization and `book_list`, then exited cleanly with no extra output.

| Result | Observation |
|--------|-------------|
| CLI / MCP books | 169 / 169 |
| Metadata and evidence parity | Passed |
| Purchased books | 36 |
| Shared books | Zero |
| Unknown books | 133 |
| Raw metadata, IDs, URLs, credentials, or responses retained | None |

This verifies the approved Web-catalog scope, not full-account discovery,
live sharing, other-platform session use, later content access, packaged-client
installation, or full manual-suite coverage. Sharing remains covered by
synthetic successful reads, loss-of-sharing checks, and adapter parity.

The owned processes ended and the temporary launcher was removed. The scope is
consumed, including unused attempts. No further account request is approved.
The earlier cumulative ceiling was 34; this run adds at most ten attempts,
for a ceiling of 44. Exact later HTTP counts were not instrumented.
The earlier measured total of 24 is unchanged.

### Native session probe

The user separately approved a native probe of the same selected source and chapter.
Its scope was one protected-storage read and at most three fixed HTTPS requests:
one cookie exchange at the existing authentication service, then bearer-only source and
chapter GETs on `www.dndbeyond.com`.
No cookie fallback, retries, redirects, proxy, storage write, or body logging was allowed.
The probe used a 5-second connection deadline, 15-second request deadlines, a 60-second
overall deadline, a 64 KiB exchange response cap, and 1 MiB HTML response caps.
Responses had to use the expected media and identity encoding.

| Request | Result | Safe observation |
|---------|--------|------------------|
| POST fixed `/v1/cobalt-token` | 200; 1,509 bytes | Shared service validation accepted the protected session |
| GET `/sources/dnd/cos` with bearer only | 200; 243,123 bytes; `text/html` | Expected article and table-of-contents markers were present |
| GET `/sources/dnd/cos/introduction` with bearer only | 307 | Probe stopped before reading the body or following the redirect |

All three approved requests were consumed. HTTP 307 is a redirect, not established denial.
The redirect target was not captured, so its host, path, and reason remain unknown.
The browser's earlier canonical link does not establish this native response's target.
Chapter content, protected access, and native section structure remain unverified.

The source result establishes that this exact native bearer-only request returned outline
HTML within the response cap. It does not prove that the outline requires authentication
or authorizes chapter reads.
The probe returned status, byte counts, accepted-media classifications, and structure checks
only. Bodies were held in zeroizing memory and not saved.
Protected storage and browser login were unchanged.

Two synthetic stop-condition tests passed before the live run.
The live test failed explicitly on HTTP 307 as required; this was not a successful chapter
read. The temporary test module was removed from the repository after the run.
No production HTML or book transport was added.
Further native requests need a new bounded scope. Do not retry or follow the redirect
automatically, widen product limits, or change authentication to get around this result.

### Separately approved redirect check

The user approved one further scope of at most three requests: a fixed exchange, the same
chapter request, and one follow-up only to the same chapter on `www.dndbeyond.com` without
query parameters. The guard also rejected fragments, user info, non-HTTPS URLs, nonstandard
ports, and recognized login/authentication paths. No cookies were sent to the chapter.
The same byte caps, deadlines, and no-retry policy applied.

The synthetic redirect guard test passed. The exchange returned HTTP 200 with a 1,509-byte
accepted response. The chapter again returned HTTP 307.
The target passed the HTTPS origin and user-info checks and was not a recognized login
route, but failed the combined path/query/fragment check.
The diagnostic does not establish which of those three conditions failed.
No raw `Location` value was returned or saved.
It is not evidence that the target is safe, that authentication is denied, or that browser
protection was encountered.

The probe stopped after two requests. No follow-up, alternate authentication, or other
chapter request was made. The unused third-request allowance was conditional and does not
permit another test. The scope ended at the failed guard.
The temporary module was removed. Its source is preserved only in session files.
Browser login and stored credentials remain unchanged.

Book source HTML is therefore observed natively, but native chapter delivery remains
unverified. Do not weaken the guard or enable redirects to claim a successful read.
A future check would need explicit approval for more precise redacted target classifications
before deciding whether any fixed target is acceptable.

### User-requested repeat after possible network disconnection

The user requested one repeat because the network might have been disconnected during the
earlier tests. This authorized the same bounded scope, not a wider target or authentication
fallback. The probe retained its caps, deadlines, fixed hosts, and redirect guard.
Its diagnostics were changed to report separate booleans and fixed route categories,
never a raw `Location` value.
The synthetic redirect guard test passed before the live repeat.

The exchange returned HTTP 200 with a 1,509-byte response accepted by the shared validator.
The chapter returned HTTP 307. The redirect used HTTPS on `www.dndbeyond.com`, the expected
port, no user info, no query, and no fragment. Its path matched the fixed classifier
for marketplace or purchase routes, not the approved chapter.
The guard stopped after two requests; no follow-up or body read was made.
The live test failed explicitly on the out-of-scope path.

This repeat reached the servers and received HTTP responses; it was not a network timeout.
It establishes a purchase-path redirect for this native bearer-only request.
It does not prove that the account lacks ownership, that another authentication method
would work, or that the browser's selected chapter was protected rather than a preview.
Do not treat accepted bearer exchange as proof of website chapter access.
The earlier browser and native observations used different request contexts.

No chapter text, raw redirect value, or credential was returned or saved.
The temporary module was removed again, and browser login and protected storage were
unchanged. This repeat scope ended at the failed guard.
At that stage, native book content remained unsupported. Do not follow the purchase route, change headers
to impersonate the browser, or add an automatic cookie fallback to overcome the result.
Any further comparison of website authentication needs a separately defined and approved
scope; the successful outline request is not sufficient access evidence.

The [Avrae entitlement models][avrae-entitlements] describe these third-party field names:

| Model | Field names | What they do not establish |
|-------|-------------|----------------------------|
| User entitlements | `acquiredLicenseIDs`, `sharedLicenses` | A local-user HTTP endpoint or current access |
| Sharing context | `campaignID`, `licenseIDs` | Permission outside the selected campaign |
| Entity entitlements | `entityType`, `entityID`, `isFree`, `licenseIDs` | Book delivery, source-ID equivalence, or automation permission |

The [Avrae client][avrae-client] obtains these records from integration-specific DynamoDB
tables, not a discovered personal-account D&D Beyond HTTP entitlement API.
Its Discord-token signing flow is also integration-specific.
Do not copy its database access, signing credentials, caches, or cloud runtime into Beyond Gate.

Before live book implementation, establish source identifiers, source URLs, metadata bounds,
and the difference between free, purchased, shared, denied, and unknown access.
Check access again before each outline or section read.
An earlier catalog entry, cached entitlement, outline, or continuation token is not a grant.

Keep the existing book limits: at most 1,000 outline entries, at most 4,096 UTF-8 text bytes
per section result, and a continuation token within 16 KiB.
Continuation must be explicit. Do not follow chapter links automatically, extract whole
books, retain a book-content cache, or include proprietary text in fixtures.
See the [existing bounded book design](architecture.md#offline-book-boundary).

### MrPrimate importer and mobile API candidates

The user suggested `MrPrimate/ddb-importer` because it imports book content into Foundry VTT.
The inspected repositories declare MIT licenses. Their source is useful Candidate evidence,
not an official API specification or proof of current account access.
No implementation was copied and no book archive, access material, or proprietary text was fetched.

| Repository | Inspected commit | Role |
|------------|------------------|------|
| `MrPrimate/ddb-importer` | `a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5` | Current Foundry client and proxy requests |
| `MrPrimate/ddb-proxy` | `a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a` | Public self-hosted proxy; no book or library handlers at this commit |
| `MrPrimate/ddb-adventure-muncher` | `5d4f8e80554078ad907084885e8d2f73bf7a4355` | Companion tool with direct mobile book API calls |

#### Current importer flow

The [book acquisition client][importer-book-data] reads the stored Cobalt cookie and service
key, then sends them to its selected proxy. The default proxy is hosted by MrPrimate;
custom proxy selection is supported. These are intermediary routes, not D&D Beyond URLs.

| Proxy request | Client use | Limit of the evidence |
|---------------|------------|-----------------------|
| POST `/proxy/adventure/available-user-content` | Obtain book IDs and enhancement book IDs | Hosted response contract, not the upstream access API |
| POST `/proxy/library` with `ownedOnly: false` | Fallback library lookup; select entries with `isOwned` and `hasEnhancement` | Does not establish an upstream library URL or shared-access semantics |
| POST `/proxy/adventure/book-codes` | Obtain database access material for selected sources | Client treats a successful response without usable material as no access |
| POST `/proxy/adventure/book-url/{bookId}` | Obtain a signed download URL and book code | Does not return a bounded section |

The [ownership helper][importer-adventures] falls back to the library lookup after failure.
Its comment names bot blocking as one possible cause. Beyond Gate must not copy this
fallback after an access denial or bot-protection response.
Enhancement metadata belongs to the Foundry integration and is not D&D Beyond access evidence.

The importer downloads a whole book ZIP through its CORS proxy, extracts an encrypted
`.db3` database, and can cache the ZIP. Its [native import client][importer-native-book]
reads `Content` fields `ID`, `CobaltID`, `ParentID`, `Slug`, `Title`, and `RenderedHTML`
to build journals and other Foundry records.
The hierarchy comes from database rows, not a demonstrated HTTP outline or section endpoint.
This whole-book flow does not fit Beyond Gate's current bounded-read and no-content-cache design.

The [public proxy route file][mrprimate-proxy-routes] has no handlers for the book,
available-content, or library routes above. Its [README][mrprimate-proxy-readme] calls it
a reduced individual-use implementation.
Do not claim that self-hosting this repository provides the current hosted book service.
The hosted book backend's implementation remains unavailable in the inspected source.

#### Direct mobile routes in the companion tool

The [companion transport][adventure-mobile-api] provides the missing upstream candidates.
Its helper sends POST requests with `application/x-www-form-urlencoded` bodies and follows
redirects. Each operation below sends the Cobalt value in the form field `token`.
This is different from the derived bearer used in the earlier native website probe.
Beyond Gate has no production mobile credential transport. Its test-only
MOB-01 probe has now completed one separately approved request.

All paths below use `https://www.dndbeyond.com`. Only MOB-01 has been called;
the other paths remain untested candidates.

| Method and path | Additional form fields | Source-code interpretation |
|-----------------|------------------------|----------------------------|
| POST `/mobile/api/v6/available-user-content` | None | Reads `Licenses`, filters `EntityTypeID` equal to `496802664`, and maps `Entities` by source ID; `isOwned` is an optional owned-only filter |
| POST `/mobile/api/v6/get-book-url/{bookId}` | None | Obtains the URL used for a whole-book download |
| POST `/mobile/api/v6/book-codes` | `sources`, a JSON array with `sourceID` and `versionID` | Obtains selected-book database access material |
| POST `/mobile/api/v6/do-higher-versions-exist` | `sourceVersions`, `manifestVersion` | Checks downloaded source versions; client reads `sourceUpdatesAvailable` |
| POST `/mobile/api/v6/user-data` | None | Obtains account information; not required evidence for a bounded book read |

The helper accepts `status: "success"` and returns `data` when present, otherwise the
outer result. This is a third-party parser, not a verified schema.
The book entity-type value and source-ID mapping need first-party or approved live evidence.
An entity without `isOwned` is not proof of shared access; campaign scope, sharing conditions,
expiry, and denial semantics remain unresolved.
The companion's source exclusions, category filters, duplicate removal, redirect following,
and error logging are not suitable production contracts for Beyond Gate.

#### Catalog discovery and remaining decisions

The current importer's [configuration loader][importer-config] uses
GET `/proxy/api/config/json` by default, also has a direct website-config path, and can
load bundled fallback metadata. The public proxy forwards the configuration lookup to
GET `https://www.dndbeyond.com/api/config/json`, as shown by its
[URL configuration][mrprimate-proxy-config] and [lookup helper][mrprimate-proxy-lookup].
The companion tool instead builds its catalog from bundled source metadata.
These catalogs identify sources; they do not prove ownership or fresh content access.
The public proxy also confirms the existing bearer exchange and the mobile `user-data`
candidate, but does not expose a book-section reader.

This research resolves where to investigate mobile book discovery and access metadata.
The approved MOB-01 run confirms current metadata availability for one account only.
It does not resolve bounded library size, general mobile availability, free versus
purchased versus shared access, or a fresh check for each requested outline or section.
It also explains why importer success cannot establish bearer-only website chapter access:
the companion sends a different credential form to a different delivery channel.
The cause of the earlier purchase redirect remains unresolved.

The metadata-only check of `available-user-content` used a separate approval,
fixed fields, body and record limits, and stop conditions.
It used protected local credentials directly with the fixed first-party service.
Do not send credentials to a hosted proxy, retrieve book access material, follow a signed
download URL, or adopt whole-book extraction as part of that metadata check.
A mobile license response alone cannot authorize an outline or section read.
At this research stage, live book reads remained unsupported. The later
[approved page-level contract](#approved-page-level-book-contract) uses website
pages, not mobile delivery.

Issue #33 now has an ignored, test-only Windows metadata probe for this candidate.
Its acknowledgment, request cap, redaction, and transport guards have synthetic
tests. See the [fixed probe scope](testing.md#mobile-book-metadata-probe).
The separately approved run on 2026-11-06 accepted one HTTP 200 response with
12,596 bytes, 8 license records, and 478 nested entities. It confirmed the
successful envelope and structural field types, not entity-type values, source
mapping, ownership meaning, or fresh content access. The result remained
`content_access=unverified`. The acknowledgment was removed and the process exited.
The protected entry was not changed. No retry or redirect occurred.
This scope is consumed. Another request needs new explicit approval.
The probe does not retrieve book
codes, signed URLs, or text, and it does not resolve the native content contract.

## Other client surfaces

These are Client evidence, not tested resource contracts or approved new tools.

| Surface | Source path or family | Limit |
|---------|-----------------------|-------|
| Party inventory | Character `party/inventory/{campaignId}`; GET | Campaign access and item shape unresolved |
| Premade character | Character `premade/{characterId}`; GET | Access and read semantics unresolved |
| Character rule data | Character `rule-data`; GET | Not proven to be a complete rules catalog |
| Character vehicles | Character `vehicles` and `vehicle/components`; GET | Selection, paging, and access unresolved |
| Character infusions | Character `infusion/items` and `known-infusions`; GET | Selection and access unresolved |
| Roll results | Character `roll-result-groups`; GET | Scope, privacy, and paging unresolved |
| Share URL | Character `share/url`; GET | Sharing semantics unresolved; exclude from v1 until reviewed |
| Rest support | Character `character/rest/short` and `character/rest/long`; GET declarations | GET helper alone does not approve a rest action |
| Campaign details | `/campaigns/v1/details/{gameId}`; GET | Client uses bearer and `data`; injected base URL not resolved here |
| Campaign characters | `/campaigns/v1/{gameId}/characters`; GET | Client uses bearer and `data`; injected base URL not resolved here |
| Dice families | `/dice/v1/getallfamilysets`; GET | Client call has no explicit bearer header; service binding and live access unresolved |
| Dice preferences | `/diceuserconfig/v1/get`; GET | Client uses bearer; service binding and privacy unresolved |
| Website campaign association | `/api/campaign/{campaignId}/character/{characterId}` | Method and read/write semantics unresolved |

The public mega-menu script contained website, marketplace, media, authentication, and
staging origin strings. The inspected string scan did not establish a book API.
Other observed script families include the message broker, event pipeline, and character-app
chunks. Asset and telemetry endpoints are not rules-reading tools.

## Excluded operations

The client also contains character creation, copy, activation, deletion, build, rest,
ability scores, preferences, descriptions, choices, class levels, inventory changes,
spell preparation, spell removal, spell slots, vehicle state, campaign joining, and dice
settings updates. They are outside the current read-only scope and were not called.

Do not infer safety from GET alone or mutation from POST alone.
Check the exact operation, method, body, access scope, and client use.
Do not expose credentials as CLI arguments, environment-variable inputs, pasted MCP values,
or client configuration. Browser login remains the agreed authentication flow.
Subsequent reads can use protected session storage without another interactive login.

## Implementation order and remaining contracts

The selected character and campaign core operations now use fixed read-only
routes and a protected Windows session. CLI and MCP share these operations.
Campaign list association state remains distinct from a loaded empty roster.
Synthetic process tests cover internal behavior and adapter parity. Live source
compatibility remains unvalidated; in particular, the campaign envelope's
`status: "success"` value is a conservative implementation assumption.

For later Rules tools, establish a selected-record or bounded-search contract for each
family before implementation. Verify identifiers, rules versions, source URLs, fresh access,
response media, schema, paging, completeness, and exact limits.
Test success, denied access, expiry, malformed responses, cancellation, and safe diagnostics
with synthetic fixtures and local mock servers.

Offline character/campaign assembly supports up to 100 raw records per page,
10 pages, and 1,000 raw records in total. Duplicates count before removal.
Current live list operations accept one unpaged response with at most 100 records.
Character list completeness is checked against its offset and total fields.
No live paging is inferred or followed.
The current HTTP default remains a 1 MiB response, a 5-second connection deadline,
a 15-second attempt deadline, a 30-second logical-read deadline, concurrency four, and
at most two retries. Redirects and proxies remain disabled.
Research response sizes do not authorize larger production limits.

New live checks need a selected operation, fixed hosts, a request budget, stop conditions,
and cleanup. Do not crawl categories, guess hidden endpoints, probe staging, or bypass
denied access. Leave unknown contracts explicitly unsupported.

## Feat public-source evidence

Issue [#68](https://github.com/PlagueHO/beyond-gate/issues/68) used
credential-free public page and client reads, then one approved bounded
structural pass. No account identifier or raw response content was retained.

The [first-party character client][character-client] declares `game-data/feats`
through its GET factory and character-service route builder. The available-feat
wrapper reads the returned `data` and applies a local restricted-definition
preference filter. This does not establish an ownership or content-access grant.
No separate feat detail request or bounded selected-ID filter was established.
The approved read below confirmed the response shape of
`/character/v5/game-data/feats?sharingSetting=2` for one session. It does not
establish broader account coverage or an ownership/access grant.

Definition helpers read `id`, `entityTypeId`, `name`, `sources`, prerequisites,
`isRepeatable`, and `repeatableParentId`. The character builder checks
prerequisites locally and groups repeatable options.
Beyond Gate preserves source fields rather than reproducing those rules.

The approved Windows check made four requests within the six-request and
60-second limits: one public configuration GET, one fixed session exchange,
one cookie-backed listing GET, and one bearer collection GET. The configuration
validated with 270 sources and 50 categories. The listing returned 329 rows.
The protected response was 388,729 bytes and had 145 rows, `success: true`, and
`pagination: null`. Its envelope had `data`, `id`, `message`, `pagination`, and
`success`; no `totalCount` field appeared. The union of row fields included
`id`, `name`, `sources`, `sourceId`, `isRepeatable`, `repeatableParentId`,
`prerequisites`, and definition fields. No top-level access-named field
appeared. The check printed only counts and field names, then discarded the
response.

This confirms one accepted protected collection response below the current
1 MiB limit. The envelope's null `pagination` matches the terminal-collection
contract used by Backgrounds; `totalCount`, when present, must equal the row
count. Feat show follows that contract and returns only a selected, checked
definition. No separate feat detail route or selected-ID filter was established.
The response had no access-named field, so the implementation does not infer
ownership or sharing from source IDs. Broader account coverage remains
unverified.

The [public feat page][feats-page] returned 950,054 UTF-8 bytes and 329 distinct
numeric links. It had one GET form with action `/feats`, the listing
`ul.listing.listing-rpgfeat.rpgfeat-listing`, and a listing footer.
No pagination control was observed.

| Control | Observed cardinality and encoding |
|---------|-----------------------------------|
| `filter-name` | One literal text input |
| `filter-prereq-subtype` | One prerequisite option; not a feat ID |
| `filter-tags` | Multi-select; repeated keys echoed both selected values |
| `filter-source` | Multi-select; repeated keys echoed both selected values |
| `filter-source-category` | Multi-select; repeated keys echoed both selected values |
| `filter-partnered-content` | Single select; existing website yes/no/any encodings |

A synthetic no-match name with partnered-content `f` returned 299,479 bytes
and one `li.alert.no-results` inside the listing. A second public GET combined
that name, one prerequisite option, and two options for each multi-select.
It returned 299,612 bytes, the same empty marker, and matching selected options.
These checks establish public form behavior, not general protected detail
access, account acceptance, or sharing.
The unfiltered page is close to the unchanged 1 MiB cap. Later growth must
fail explicitly rather than truncate or silently increase the limit.

Production `feat_show` uses the observed bounded collection route. Selected raw
records, repeatable relationships, source validation, and error parity have
synthetic coverage. Native CLI/MCP acceptance and broader account coverage
remain unverified.
See the [catalog status](api-catalog.md#feat-browse-and-detail-status).

## Sources

| ID | Source | Use |
|----|--------|-----|
| C1 | [First-party character application][character-client] | Source route builders, HTTP helpers, versions, and production origin strings |
| C2 | [First-party mega-menu application][menu-client] | Public navigation, origin strings, and library JSON request helpers |
| C3 | [Official My Library announcement][library-announcement] | User-facing library filters, not an API schema |
| C4 | [First-party library page client][library-client] | Prepared-record rendering, user filters, and source-link consumption; raw-source mapping not shown |
| C5 | [First-party public configuration][public-config] | Credential-free category definition and counts; not account-library eligibility or access evidence |
| P1 | [Classes][classes-page], [backgrounds][backgrounds-page], [species][species-page], [feats][feats-page] | Public listing metadata |
| P2 | [Spells][spells-page], [monsters][monsters-page], [magic items][items-page], [equipment][equipment-page] | Public filter names and paging links |
| T1 | [Pinned endpoint definitions][candidate-endpoints] | Third-party monster and config candidates; not authoritative |
| T2 | [Pinned reference implementation][candidate-reference] | Candidate shapes and unsuitable bulk/filtering behavior |
| T3 | [Pinned Avrae entitlement models][avrae-entitlements] | Candidate access field names |
| T4 | [Pinned Avrae client][avrae-client] | Integration-specific entitlement transport, not a local-user API |
| T5 | [Pinned importer book client][importer-book-data] and [ownership helper][importer-adventures] | Hosted proxy book flow, not direct upstream routes |
| T6 | [Pinned public proxy routes][mrprimate-proxy-routes] and [configuration][mrprimate-proxy-config] | Config forwarding and limits of the public self-hosted implementation |
| T7 | [Pinned adventure companion transport][adventure-mobile-api] | Direct mobile book API candidates and different credential transport; only MOB-01 has a separately approved structural metadata result |
| T8 | [Pinned scraper picker][scraper-source-filter] and [configuration client][scraper-source-catalog] | Hidden-category candidate; incomplete attribution and weaker validation |
| T9 | [Pinned TypeScript source parser][sdk-source-parser] | Selected-page DOM parsing, not complete library discovery |
| T10 | [Pinned sources userscript][sources-userscript] | Historical DOM ownership/sharing heuristics, not a numeric-ID contract |
| T11 | [Pinned race endpoint and reference tool][candidate-races] | Candidate route, bearer client use, and species field names |
| T12 | [Pinned character API client][candidate-api-client] | Candidate character-service envelope handling and bearer transport |
| B1 | [Library][library-page], [selected source][selected-book], and [selected chapter][selected-chapter] | Separately approved three-page browser inspection; structure and metadata only |
| L1 | [Issue #31][issue-31] | Earlier bounded approved probes and accepted character/campaign decisions, preserved in the issue artifacts |
| L2 | [Issue #32][issue-32] | Separately approved three-attempt native library metadata check; partial contract evidence only |

First-party script filenames identify the inspected builds. Their availability and contents
can change. Third-party links are pinned to exact commits.
Recheck important contracts against current first-party evidence before production work.
No search-result claim is used as endpoint evidence without source inspection.

[issue-31]: https://github.com/PlagueHO/beyond-gate/issues/31
[issue-32]: https://github.com/PlagueHO/beyond-gate/issues/32
[character-client]: https://media.dndbeyond.com/character-app/static/js/main.4ccf1602.js
[menu-client]: https://media.dndbeyond.com/mega-menu/assets/index-CVrwI34f.js
[library-client]: https://www.dndbeyond.com/_next/static/chunks/app/%5Blocale%5D/library/page-794b0c736763f2f7.js
[public-config]: https://www.dndbeyond.com/api/config/json
[classes-page]: https://www.dndbeyond.com/classes
[backgrounds-page]: https://www.dndbeyond.com/backgrounds
[species-page]: https://www.dndbeyond.com/species
[feats-page]: https://www.dndbeyond.com/feats
[spells-page]: https://www.dndbeyond.com/spells
[monsters-page]: https://www.dndbeyond.com/monsters
[items-page]: https://www.dndbeyond.com/magic-items
[equipment-page]: https://www.dndbeyond.com/equipment
[library-page]: https://www.dndbeyond.com/en/library
[library-announcement]: https://www.dndbeyond.com/posts/2122-find-filter-and-favorite-your-content-with-the-new
[selected-book]: https://www.dndbeyond.com/sources/dnd/cos
[selected-chapter]: https://www.dndbeyond.com/sources/dnd/cos/introduction#RunningtheAdventure
[candidate-endpoints]: https://github.com/AlexWorland/dndbeyond-mcp/blob/61d6f3caae27b539d5a1ba7b0668074c27672702/src/api/endpoints.ts
[candidate-reference]: https://github.com/AlexWorland/dndbeyond-mcp/blob/61d6f3caae27b539d5a1ba7b0668074c27672702/src/tools/reference.ts
[candidate-races]: https://github.com/AlexWorland/dndbeyond-mcp/blob/61d6f3caae27b539d5a1ba7b0668074c27672702/src/tools/reference.ts
[candidate-api-client]: https://github.com/AlexWorland/dndbeyond-mcp/blob/61d6f3caae27b539d5a1ba7b0668074c27672702/src/api/client.ts
[importer-monster-factory]: https://github.com/MrPrimate/ddb-importer/blob/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5/src/parser/DDBMonsterFactory.ts
[importer-monster-types]: https://github.com/MrPrimate/ddb-importer/blob/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5/src/types/ddb-monster-source.d.ts
[mrprimate-proxy-monsters]: https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/monsters.js
[avrae-entitlements]: https://github.com/avrae/avrae/blob/15258cb884c1085582a710a9791f890371afb6c4/ddb/entitlements.py
[avrae-client]: https://github.com/avrae/avrae/blob/15258cb884c1085582a710a9791f890371afb6c4/ddb/client.py
[importer-book-data]: https://github.com/MrPrimate/ddb-importer/blob/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5/src/muncher/adventure/native/BookData.ts
[importer-adventures]: https://github.com/MrPrimate/ddb-importer/blob/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5/src/muncher/DDBAdventures.ts
[importer-native-book]: https://github.com/MrPrimate/ddb-importer/blob/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5/src/muncher/adventure/native/NativeAdventureMunch.ts
[importer-config]: https://github.com/MrPrimate/ddb-importer/blob/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5/src/hooks/ready/ddbConfig.ts
[importer-page-import]: https://github.com/MrPrimate/ddb-importer/blob/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5/src/muncher/PageImport.ts
[mrprimate-proxy-routes]: https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/index.js
[mrprimate-proxy-readme]: https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/README.md
[mrprimate-proxy-config]: https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/config.js
[mrprimate-proxy-lookup]: https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/lookup.js
[adventure-mobile-api]: https://github.com/MrPrimate/ddb-adventure-muncher/blob/5d4f8e80554078ad907084885e8d2f73bf7a4355/munch/data/ddb.js
[scraper-docs]: https://docs.rs/scraper/0.27.0/scraper/
[scraper-source-filter]: https://github.com/theunschut/ddb-scraper/blob/54cf2d411ad4533b45c9f0cf7871dfc0cdf38c8b/DdbScraper.Repository/Ddb/SourcePickerFilter.cs
[scraper-source-catalog]: https://github.com/theunschut/ddb-scraper/blob/54cf2d411ad4533b45c9f0cf7871dfc0cdf38c8b/DdbScraper.Repository/Ddb/DdbSourceCatalogClient.cs
[scraper-endpoint-notes]: https://github.com/theunschut/ddb-scraper/blob/54cf2d411ad4533b45c9f0cf7871dfc0cdf38c8b/docs/ddb-api/endpoints.md
[sdk-source-parser]: https://github.com/astermoline/dndbeyond-parser/blob/3a667c78ba832cd87f76817fad6407a8a5b05e9a/src/parsers/source.ts
[sources-userscript]: https://github.com/petrgon/beyond-my-sources/blob/7dcfa621323d354e661ce8c6de7b915bde809170/script.js
