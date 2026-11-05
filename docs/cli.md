# CLI command reference

**Current status:** `beyond-gate` supports help, version output, shell completions,
local session status and logout, Windows session validation, and local MCP start and registration. Windows
login, explicit browser setup, and the native host are implemented.
Approved Chrome and Edge Dev login/save/restart/logout checks passed.
Windows `book list` has bounded live metadata transport through the shared stored session.
Web-catalog scope, category selection, and title cleanup are implemented and verified natively.
Other live resource reads remain unsupported.

Other valid resource reads currently exit with code 1, write an unsupported-operation
diagnostic to stderr, and leave stdout empty. They do not return sample data or
empty success results. The success contracts below are tested with synthetic
inputs in offline tests. They are not evidence of live access.

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
`character`, `campaign`, `book`, or `mcp`.

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
| [`character list`](#character-list) | List character summaries | Live read unsupported |
| [`character show`](#character-show) | Select one character | Live read unsupported |
| [`campaign list`](#campaign-list) | List campaigns and associations | Live read unsupported |
| [`campaign show`](#campaign-show) | Select one campaign | Live read unsupported |
| [`book list`](#book-list) | List Web-catalog book metadata and access evidence | Windows discovery verified |
| [`book outline`](#book-outline) | Read complete bounded navigation | Live read unsupported |
| [`book read`](#book-read) | Read one bounded section chunk | Live read unsupported |
| [`mcp start`](#mcp-start) | Start the sibling stdio server | Implemented |
| [`mcp install`](#mcp-install) | Register the server with a local client | Implemented; confirms before writing |
| [`completions`](#completions) | Generate a shell completion script | Implemented |

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

`book list` reads the fixed protected session and two bounded metadata routes.
The other six actions fail as unsupported after valid input is accepted.
Their success outputs are offline-tested contracts.

### List options

`character list`, `campaign list`, `book list`, and `book outline` share these
options:

| Option | Values | Default | Effect |
|--------|--------|---------|--------|
| `--format <FORMAT>` | `table`, `json`, `tsv` | `table` | Select result presentation |
| `--no-headers` | Flag; no value | Headers included | Omit table or TSV column names |

`--no-headers` with `--format json` is invalid usage, with exit code 2.
`show` and `book read` produce JSON only. They reject format and header options.
There are no filter, automatic pagination, account, URL, or fixture options.

### character list

```text
beyond-gate character list [--format <FORMAT>] [--no-headers]
```

Lists character summaries. No positional arguments are required.
Table and TSV columns are `id`, `name`, and `source_url`, in that order.
JSON is an array of [character objects](#character-and-campaign-objects), including
the structured `raw` field. Source order is preserved.

Current result: unsupported-operation diagnostic, exit code 1, and empty stdout.
Invalid list options fail with code 2. Offline assembly errors include invalid
metadata, conflicting records, incomplete or invalid pagination, and exceeded
page or record limits. They do not produce a partial successful list.

### character show

```text
beyond-gate character show <id>
```

Requires one character identifier. No action-specific options are accepted.
The success contract is one JSON
[character object](#character-and-campaign-objects), not an array.
It preserves the supplied data without interpreting game rules.

Current result: unsupported-operation diagnostic, exit code 1, and empty stdout.
Missing or invalid identifiers fail with code 2. Shared character operation
failures use code 1. No upstream not-found behavior is implemented.

### campaign list

```text
beyond-gate campaign list [--format <FORMAT>] [--no-headers]
```

Lists campaigns and their opaque character associations. No positional arguments
are required. Table and TSV columns are `id`, `name`, `source_url`, and
`character_ids`, in that order. JSON is an array of
[campaign objects](#character-and-campaign-objects), including `raw`.

Current result: unsupported-operation diagnostic, exit code 1, and empty stdout.
Invalid list options fail with code 2. Shared metadata, record-conflict,
pagination, and limit failures use code 1 without a partial successful list.

### campaign show

```text
beyond-gate campaign show <id>
```

Requires one campaign identifier. No action-specific options are accepted.
The success contract is one JSON
[campaign object](#character-and-campaign-objects), not an array.
Associated character identifiers remain in their supplied order. They do not
prove that those characters are visible in a separate character list.

Current result: unsupported-operation diagnostic, exit code 1, and empty stdout.
Missing or invalid identifiers fail with code 2. Shared campaign operation
failures use code 1. No upstream not-found behavior is implemented.

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

Current result: unsupported-operation diagnostic, exit code 1, and empty stdout.
Invalid identifiers or list options fail with code 2. The offline contract rejects
denied or unknown access, invalid metadata, repeated entry identifiers, missing
parents, cycles, and more than 1,000 raw entries. It never silently truncates an
outline. See [book bounds and continuation](#book-bounds-and-continuation).

### book read

```text
beyond-gate book read <book-id> <section-id> [--continuation <TOKEN>]
```

Requires explicit book and section identifiers. The only action-specific option
is `--continuation`, an optional string. Omit it for an initial read.
The success contract is one JSON
[section object](#outline-and-section-objects) with at most 4,096 UTF-8 text bytes.
There are no format, header, whole-book, or automatic continuation options.

Current result: unsupported-operation diagnostic, exit code 1, and empty stdout.
Invalid identifiers, missing arguments, and malformed or oversized input tokens
fail with code 2 before the live operation. A syntactically valid token does not
enable live access.

In the offline contract, denied or unknown access, unavailable content,
unsupported sections, source failures, invalid metadata, wrong-scope or
invalid-position continuation, and changed sources fail with code 1.
Failure to encode a required output token also fails with code 1, not a result
that hides remaining text.

## Output contracts

The following describes successful resource output tested offline, not current
live results. JSON uses standard escaping and preserves structured values.
CLI JSON is pretty-printed with a final newline.

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

The offline core checks fresh supplied access before each outline or section
load, including each continuation. Unknown access is not a grant. Neither a prior
library result nor a continuation authorizes another read. Current product
entry points return unsupported errors; no live access check occurs yet.

Outlines allow at most 1,000 raw entries, including duplicates. Section output
contains at most 4,096 UTF-8 text bytes. This cap excludes JSON escaping, source
attribution, and continuation bytes. It is not a whole-response or source-memory
limit. Original text and line breaks are preserved. A chunk can end before the
cap to preserve a UTF-8 boundary or a supplied line.

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

## Contract sources

The reference follows the [clap definition](../src/beyond-gate-cli/src/command.rs),
[runner](../src/beyond-gate-cli/src/lib.rs),
[serializers](../src/beyond-gate-cli/src/output.rs), and
[registration code](../src/beyond-gate-cli/src/mcp.rs).
The [CLI process tests](../src/beyond-gate-cli/tests/cli.rs) and
[synthetic adapter tests](../src/beyond-gate-cli/tests/adapter_process.rs)
check help, usage, output, errors, and explicit continuation without live accounts.
