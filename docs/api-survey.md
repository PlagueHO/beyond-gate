# D&D Beyond API survey

Recorded on 2026-11-02 for [issue #31][issue-31].

This survey covers every area observed in the Rules menu and the related API families found
in the site's public client. It includes earlier approved character and campaign inspections.
It does not establish a complete list of internal APIs or a supported public API specification.
It does not add production reads to Beyond Gate.

Use the [shared API call catalog](api-catalog.md) to find known calls by stable ID.
This survey holds their detailed evidence, measurements, and failed checks.
Keep both documents current so other issues can reuse findings without repeating probes.

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
| Species | `/species` | Character `game-data/races` | Species metadata, with upstream naming preserved |
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
JSON service parameters are separate contracts. No filter or next page was requested.

All eight pages include a general `search` input. Classes and species had no additional
matching filter names. Backgrounds and feats had no observed paging parameter.

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
| `https://monster-service.dndbeyond.com` | Separate monster service; Candidate | Not queried |
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
| `game-data/races` | Species and legacy race data | Upstream naming and rules version |
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
[reference implementation][candidate-reference]. They were not queried.

| Candidate method | URL | Candidate parameters or shape |
|------------------|-----|-------------------------------|
| GET | `https://monster-service.dndbeyond.com/v1/Monster` | `search`, `skip`, `take`, `showHomebrew`, `sources`, or repeated `ids` |
| GET | `https://monster-service.dndbeyond.com/v1/Monster/{id}` | Selected monster |
| GET | `https://www.dndbeyond.com/api/config/json` | Rules lookup configuration |

The candidate monster search shape has array `data`, an `accessType` map, and pagination
fields `take`, `skip`, `currentPage`, `pages`, and `total`.
The candidate detail shape has object `data` and scalar `accessType`.
Numeric access meanings are not verified. A visible name, partial record, or successful
HTTP response must not be treated as permission to return a full stat block.

The candidate config contains lookup arrays for challenge ratings, monster types,
environments, alignments, damage types, and senses. A source lookup is not a user library
or an entitlement check.

The third-party reference client also builds a spell catalog from many class and level
requests, caches large datasets, and can tolerate partial results.
It filters item and feat datasets locally. These are implementation choices, not evidence
of supported standalone bounded-search APIs. Beyond Gate must not copy these behaviors.
Hardcoded source maps and guessed access-type meanings are also unsuitable.

## Books, sources, and access checks

The initial survey did not establish book delivery. A later approved browser inspection
established a library route and HTML outline/section structure for one selected source.
The later native probe accepted the protected-session exchange and source HTML, but stopped
at a chapter redirect. A user-requested repeat identified a marketplace or purchase target.
Fresh access, bounded discovery, and general book support remain
unresolved. No book JSON endpoint or source-to-license mapping was established.
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
Native book content remains unsupported. Do not follow the purchase route, change headers
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
Beyond Gate has not implemented or tested this credential transport.

All paths below use `https://www.dndbeyond.com`. No request to these routes was made.

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
It does not resolve bounded library size, current mobile availability, free versus
purchased versus shared access, or a fresh check for each requested outline or section.
It also explains why importer success cannot establish bearer-only website chapter access:
the companion sends a different credential form to a different delivery channel.
The cause of the earlier purchase redirect remains unresolved.

The next useful live check would be a separately approved, metadata-only check of
`available-user-content`, with fixed fields, body and record limits, and stop conditions.
It must use protected local credentials directly with the fixed first-party service.
Do not send credentials to a hosted proxy, retrieve book access material, follow a signed
download URL, or adopt whole-book extraction as part of that metadata check.
A mobile license response alone cannot authorize an outline or section read.
Keep live book reads unsupported until a delivery method meets the existing design.

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

Complete the already selected character and campaign core operations first.
Connect CLI and MCP to the same core reads and preserve their error and output semantics.
Keep campaign list association state separate from a loaded empty roster.

For later Rules tools, establish a selected-record or bounded-search contract for each
family before implementation. Verify identifiers, rules versions, source URLs, fresh access,
response media, schema, paging, completeness, and exact limits.
Test success, denied access, expiry, malformed responses, cancellation, and safe diagnostics
with synthetic fixtures and local mock servers.

The current character/campaign assembly limits remain 100 raw records per page, 10 pages,
and 1,000 raw records in total. Duplicates count before removal.
The current HTTP default remains a 1 MiB response, a 5-second connection deadline,
a 15-second attempt deadline, a 30-second logical-read deadline, concurrency four, and
at most two retries. Redirects and proxies remain disabled.
Research response sizes do not authorize larger production limits.

New live checks need a selected operation, fixed hosts, a request budget, stop conditions,
and cleanup. Do not crawl categories, guess hidden endpoints, probe staging, or bypass
denied access. Leave unknown contracts explicitly unsupported.

## Sources

| ID | Source | Use |
|----|--------|-----|
| C1 | [First-party character application][character-client] | Source route builders, HTTP helpers, versions, and production origin strings |
| C2 | [First-party mega-menu application][menu-client] | Public navigation client and origin strings |
| P1 | [Classes][classes-page], [backgrounds][backgrounds-page], [species][species-page], [feats][feats-page] | Public listing metadata |
| P2 | [Spells][spells-page], [monsters][monsters-page], [magic items][items-page], [equipment][equipment-page] | Public filter names and paging links |
| T1 | [Pinned endpoint definitions][candidate-endpoints] | Third-party monster and config candidates; not authoritative |
| T2 | [Pinned reference implementation][candidate-reference] | Candidate shapes and unsuitable bulk/filtering behavior |
| T3 | [Pinned Avrae entitlement models][avrae-entitlements] | Candidate access field names |
| T4 | [Pinned Avrae client][avrae-client] | Integration-specific entitlement transport, not a local-user API |
| T5 | [Pinned importer book client][importer-book-data] and [ownership helper][importer-adventures] | Hosted proxy book flow, not direct upstream routes |
| T6 | [Pinned public proxy routes][mrprimate-proxy-routes] and [configuration][mrprimate-proxy-config] | Config forwarding and limits of the public self-hosted implementation |
| T7 | [Pinned adventure companion transport][adventure-mobile-api] | Direct mobile book API candidates and different credential transport; not live-tested |
| B1 | [Library][library-page], [selected source][selected-book], and [selected chapter][selected-chapter] | Separately approved three-page browser inspection; structure and metadata only |
| L1 | [Issue #31][issue-31] | Earlier bounded approved probes and accepted character/campaign decisions, preserved in the issue artifacts |

First-party script filenames identify the inspected builds. Their availability and contents
can change. Third-party links are pinned to exact commits.
Recheck important contracts against current first-party evidence before production work.
No search-result claim is used as endpoint evidence without source inspection.

[issue-31]: https://github.com/PlagueHO/beyond-gate/issues/31
[character-client]: https://media.dndbeyond.com/character-app/static/js/main.4ccf1602.js
[menu-client]: https://media.dndbeyond.com/mega-menu/assets/index-CVrwI34f.js
[classes-page]: https://www.dndbeyond.com/classes
[backgrounds-page]: https://www.dndbeyond.com/backgrounds
[species-page]: https://www.dndbeyond.com/species
[feats-page]: https://www.dndbeyond.com/feats
[spells-page]: https://www.dndbeyond.com/spells
[monsters-page]: https://www.dndbeyond.com/monsters
[items-page]: https://www.dndbeyond.com/magic-items
[equipment-page]: https://www.dndbeyond.com/equipment
[library-page]: https://www.dndbeyond.com/en/library
[selected-book]: https://www.dndbeyond.com/sources/dnd/cos
[selected-chapter]: https://www.dndbeyond.com/sources/dnd/cos/introduction#RunningtheAdventure
[candidate-endpoints]: https://github.com/AlexWorland/dndbeyond-mcp/blob/61d6f3caae27b539d5a1ba7b0668074c27672702/src/api/endpoints.ts
[candidate-reference]: https://github.com/AlexWorland/dndbeyond-mcp/blob/61d6f3caae27b539d5a1ba7b0668074c27672702/src/tools/reference.ts
[avrae-entitlements]: https://github.com/avrae/avrae/blob/15258cb884c1085582a710a9791f890371afb6c4/ddb/entitlements.py
[avrae-client]: https://github.com/avrae/avrae/blob/15258cb884c1085582a710a9791f890371afb6c4/ddb/client.py
[importer-book-data]: https://github.com/MrPrimate/ddb-importer/blob/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5/src/muncher/adventure/native/BookData.ts
[importer-adventures]: https://github.com/MrPrimate/ddb-importer/blob/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5/src/muncher/DDBAdventures.ts
[importer-native-book]: https://github.com/MrPrimate/ddb-importer/blob/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5/src/muncher/adventure/native/NativeAdventureMunch.ts
[importer-config]: https://github.com/MrPrimate/ddb-importer/blob/a6ea7cc50b4012cc805a8d5880a2a4378f8ec2f5/src/hooks/ready/ddbConfig.ts
[mrprimate-proxy-routes]: https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/index.js
[mrprimate-proxy-readme]: https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/README.md
[mrprimate-proxy-config]: https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/config.js
[mrprimate-proxy-lookup]: https://github.com/MrPrimate/ddb-proxy/blob/a181c5433df9f7a6765e38bc7fc1b4b7d9e0c17a/lookup.js
[adventure-mobile-api]: https://github.com/MrPrimate/ddb-adventure-muncher/blob/5d4f8e80554078ad907084885e8d2f73bf7a4355/munch/data/ddb.js
