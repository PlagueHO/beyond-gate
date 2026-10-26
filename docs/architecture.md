# Architecture

## Current boundary

The workspace contains three crates. The core exports one shared foundation status
message. Both adapters use clap for help and version output. There is no network
client, account model, credential store, or MCP protocol implementation.

The workspace uses Rust 2024 and resolver 3. The
[Cargo edition guide](https://doc.rust-lang.org/edition-guide/rust-2024/cargo-resolver.html)
recommends this resolver for Rust 2024. The pinned toolchain is
[Rust 1.99.1](https://github.com/rust-lang/rust/releases/tag/1.99.1).
The only direct external dependency is [clap 4.6.7](https://docs.rs/clap/4.6.7/clap/).
`Cargo.lock` records exact resolved versions.

## Intended boundaries

| Crate | Owns | Does not own |
|-------|------|--------------|
| `beyond-gate-core` | Authorized reads, session use, models, typed errors | CLI formatting or MCP protocol |
| `beyond-gate-cli` | Resource-first commands, output, human login flow | Duplicate endpoint or access logic |
| `beyond-gate-mcp` | Stdio lifecycle, bounded read tools, error mapping | Browser login or credentials from an agent |

Prefer direct functions and concrete types. Introduce a trait only when an actual
test boundary or multiple behaviors need it. Do not create empty model, service,
repository, or transport layers before the first operation needs them.

The planned network stack is async reqwest with Tokio. It is not installed yet.
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
