# Beyond Gate - Copilot instructions

See AGENTS.md for layout and commands.

## Purpose and safety

Beyond Gate is a native Rust CLI and local MCP server project for authorized
D&D Beyond reads. Product operations are not yet implemented.

- Keep v1 read-only and local; do not add remote hosting or multiuser services.
- Do not bypass access controls, paywalls, or bot protection.
- Do not treat account access or content ownership as automation permission.
- Never put credentials in CLI arguments, prompts, logs, fixtures, or client configuration.
- Never add proprietary book text or real account snapshots to the repository.
- Keep login human-controlled and secrets in protected OS credential storage.
- Do not request D&D Beyond passwords or accept credentials through MCP tools.
- Do not add a required Python, Node.js, or cloud runtime.

## Code style and documentation

- Use simple, short functions with one clear responsibility.
- Apply SOLID only where it improves actual behavior or testability.
- Do not add traits, layers, generic frameworks, or configuration for hypothetical needs.
- Write useful `///` documentation for every function, including private helpers and tests.
- Describe purpose, inputs, outputs, and non-obvious Rust concepts where useful.
- Add runnable examples for reusable APIs when they clarify use.
- Add `# Errors`, `# Panics`, and `# Safety` sections only when applicable.
- Do not repeat the function name as its only documentation.
- Use `//!` documentation to explain each crate or module.
- Avoid redundant inline comments; let clear names explain ordinary code.
- Use `Result` for expected failures and preserve typed causes in shared operations.
- Do not use broad error suppression, secret-bearing Debug output, or fake-success stubs.
- Do not use `unwrap` or `expect` in production code.

## Naming

| Element | Convention | Example |
|---------|------------|---------|
| Packages and commands | kebab-case | `beyond-gate-core` |
| Modules and functions | snake_case | `list_characters` |
| Types and traits | UpperCamelCase | `CharacterSummary` |
| Constants | SCREAMING_SNAKE_CASE | `FOUNDATION_NOTICE` |

## Adapter patterns

- Keep shared operations in the core; do not duplicate access logic in adapters.
- Use resource-first clap commands when real operations are added.
- Keep CLI result data on stdout and diagnostics on stderr.
- Reserve MCP stdout for protocol messages once serving exists.
- Prefer the official rmcp SDK over a custom MCP protocol implementation.
- Preserve upstream character fields rather than inventing a rules engine.
- Use bounded book reads with source URLs; do not add bulk content extraction.
- Follow the roadmap's development policy. The historical permission stop gate
  is removed; technical contracts and separate live-test approval are still needed.

## Testing

- Keep unit tests beside the responsible module and process tests in crate `tests`.
- Use synthetic fixtures and local mock servers, not live accounts.
- Test denied access, expiry, malformed input, and output contracts with successful reads.
- Keep CLI and MCP error and data semantics consistent for each shared operation.
