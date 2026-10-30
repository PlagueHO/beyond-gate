# MCP tool reference

**Current status:** `beyond-gate-mcp` is a local read-only stdio server using the
official Rust MCP SDK, rmcp. Startup, initialization, tool discovery, validation,
and protocol errors are implemented. Live character, campaign, and book reads
are not implemented.

Valid calls to all seven read tools currently return unsupported-operation tool
errors with `isError: true`. They do not return sample data or empty success
results. Success shapes in this reference are offline-tested adapter contracts,
not evidence of successful live access.

See [native installation](installation.md) for setup and
[authentication limits](authentication.md#local-session-commands) for the blocked
browser login and protected local session commands. The server does not provide
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

Every tool takes a JSON object and rejects unknown fields. List tools require
an empty object `{}`, not filters. There are no passwords, cookies, tokens for
authentication, arbitrary URL inputs, or account identities. The only token input
is `book_read.continuation`, which represents a bounded section position, not a
credential.

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
`idempotentHint: true`, and `openWorldHint: true`. These annotations do not grant
access or make live reads available. Tool results are data, not instructions.

## Tool index and CLI mapping

| Tool | CLI equivalent | Success data shape |
|------|----------------|--------------------|
| [`character_list`](#character_list) | `beyond-gate character list --format json` | Array of character objects |
| [`character_show`](#character_show) | `beyond-gate character show <id>` | One character object |
| [`campaign_list`](#campaign_list) | `beyond-gate campaign list --format json` | Array of campaign objects |
| [`campaign_show`](#campaign_show) | `beyond-gate campaign show <id>` | One campaign object |
| [`book_list`](#book_list) | `beyond-gate book list --format json` | Array of book objects |
| [`book_outline`](#book_outline) | `beyond-gate book outline <book-id> --format json` | Outline envelope |
| [`book_read`](#book_read) | `beyond-gate book read <book-id> <section-id> [--continuation <token>]` | One section chunk |

The data shapes below match CLI JSON, not CLI table or TSV presentation.
All example identifiers and source URLs are synthetic.

## Tools

### character_list

Lists authorized character summaries when a live source becomes available.

| Input | Type | Required | Validation |
|-------|------|----------|------------|
| No fields | Empty object | Send `{}` | Rejects every additional field |

Synthetic `tools/call` parameters:

```json
{"name": "character_list", "arguments": {}}
```

The success data is an array of
[character objects](#character-and-campaign-data), with `id`, `name`, `source_url`,
and `raw`. Empty data would be `[]`; source order is preserved.
Current calls return the character/campaign unsupported-operation tool error,
not that array. Shared offline failures include invalid or conflicting metadata,
incomplete or invalid pagination, exceeded limits, and page-read failures.

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
Invalid identifiers fail before the core operation. Valid calls currently return
the character/campaign unsupported-operation tool error.
Shared operation failures do not become an empty object or `null` success.
No upstream not-found contract is implemented.

### campaign_list

Lists campaigns and preserves opaque character associations.

| Input | Type | Required | Validation |
|-------|------|----------|------------|
| No fields | Empty object | Send `{}` | Rejects every additional field |

Synthetic `tools/call` parameters:

```json
{"name": "campaign_list", "arguments": {}}
```

The success data is an array of
[campaign objects](#character-and-campaign-data), with `id`, `name`, `source_url`,
`character_ids`, and `raw`. Empty data would be `[]`; source order is preserved.
Current calls return the character/campaign unsupported-operation tool error.
Shared offline metadata, conflict, pagination, limit, or source failures do not
return a partial successful list.

### campaign_show

Selects one campaign without resolving its character associations.

| Input | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | String | Yes | Nonempty, unpadded, control-free identifier |

Synthetic `tools/call` parameters:

```json
{"name": "campaign_show", "arguments": {"id": "synthetic-campaign"}}
```

The success data is one [campaign object](#character-and-campaign-data), with
`id`, `name`, `source_url`, `character_ids`, and `raw`. It is not an array.
Invalid identifiers fail before the core operation. Valid calls currently return
the character/campaign unsupported-operation tool error.
Shared operation failures do not become an empty object or `null` success.
No upstream not-found contract is implemented.

### book_list

Lists visible book metadata and separate purchase, sharing, or catalog evidence.
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
Current calls return the unsupported live-library discovery tool error.
Offline metadata, evidence, conflict, pagination, limit, and page-read failures
are errors, not partial successful results.

### book_outline

Selects one book's complete navigation outline, with at most 1,000 entries.

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
Invalid identifiers fail before the core operation. Valid calls currently return
the unsupported live-book-read tool error.

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
the core live operation. Valid calls currently return the unsupported
live-book-read tool error.

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

This synthetic request is valid, but its current result is an unsupported error:

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
        "text": "Live book reads are unsupported: source contracts and usable login are not implemented."
      }
    ],
    "isError": true
  }
}
```

**Illustrative synthetic success response only:** The current product does not
return this successful outline. It shows the separate MCP wrapper and inner JSON:

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

The current unsupported messages distinguish character/campaign reads, library
discovery, and book reads. Typed core failures become safe text messages.
A serialization failure uses `Result JSON could not be encoded.`
MCP and CLI share operation classifications, but the CLI can add diagnostic
context; their complete error strings need not be identical.
No failure substitutes sample data, partial data, or empty success.

## Success data contracts

These data shapes are tested with synthetic core inputs. The server has no
fixture switch or live success mode.

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
`kind: "purchased"` with `state`, or `kind: "shared"` with `context_id` (string)
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

The offline book operations check fresh supplied access before every outline or
section load, including continuation. Unknown access and explicit denial both
stop the load. A prior library observation or token is never an access grant.
Current product calls stop at the unsupported-operation boundary, not a live
access check.

Outlines allow at most 1,000 raw entries, including duplicates. Section results
allow at most 4,096 UTF-8 text bytes, excluding JSON escaping, source metadata, and
continuation bytes. This does not cap total response size or caller allocations.
Text and line breaks remain unchanged. Chunk boundaries respect UTF-8 and can
prefer a complete supplied line; they need not align with words.

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

## Contract sources

The [server implementation](../src/beyond-gate-mcp/src/server.rs) defines all
schemas, annotations, tool mappings, and serializers.
The [entry point](../src/beyond-gate-mcp/src/main.rs) owns startup and transport
lifecycle. The [protocol process tests](../src/beyond-gate-mcp/tests/cli.rs)
check discovery, schema limits, validation, unsupported errors, and closure.
Serializer unit tests use original synthetic data.
See the [CLI reference](cli.md#output-contracts) for the matching output
contracts and [architecture](architecture.md#mcp-stdio-adapter) for design context.
