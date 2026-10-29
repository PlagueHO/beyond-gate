# Changelog

This project uses [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Resource-first `character list/show`, `campaign list/show`, and
  `book list/outline/read` CLI adapter with offline process tests, table/JSON/TSV
  list output, header control, preserved structured JSON, safe typed diagnostics,
  and stable 0/1/2 exits. Production reads fail explicitly as unsupported.
- Runtime Bash, Elvish, Fish, PowerShell, and Zsh completions generated from the
  actual clap command definition without credential or account access.
- Shared versioned section continuation tokens with a 16 KiB encoded cap,
  strict URL-safe parsing, redacted failures, and synthetic cross-process tests.
  Tokens do not grant access; reads retain the 4,096-byte UTF-8 text bound and
  fresh access/source checks. No automatic continuation or content cache is added.
- Offline core book outlines with a 1,000-entry cap and selected-section reads
  with a 4,096-byte UTF-8 text cap, fresh supplied access checks, source URLs,
  opaque revision-bound cursors, explicit completion, typed redacted failures,
  synthetic tests, and a runnable navigation example. No live book reads,
  automatic continuation, content cache, or CLI/MCP exposure is added.
- GitHub Copilot App tasks and a PowerShell Cargo wrapper that load Visual Studio
  2022 Build Tools on Windows for run, build, test, lint, and documentation checks.
- Offline core character and campaign records with unknown JSON preservation,
  opaque identifiers, campaign associations, optional HTTPS source metadata,
  duplicate conflict checks, bounded complete pagination, typed failures, and
  synthetic tests. Live reads, endpoints, and CLI/MCP operations remain blocked.
- Offline core library metadata assembly with explicit purchase and sharing
  evidence, source URLs, duplicate conflict checks, bounded complete pagination,
  redacted typed failures, and synthetic tests. No cached grants, live discovery,
  account reads, or CLI/MCP book interfaces are enabled.
- Shared protected local session operations with explicit Windows Credential
  Manager, macOS login Keychain, and Linux Secret Service backends, bounded opaque
  secrets, redacted typed failures, and synthetic store tests. No plaintext fallback
  or product credential import is available.
- Resource-first `auth status` and `auth logout` commands for local presence and
  removal only. `auth login` reports the browser-handoff blocker without collecting
  secrets or starting a browser. Stored sessions remain unverified; upstream
  exchange, expiry, refresh, revocation, and MCP session use remain unavailable.
- Offline-tested async core HTTP boundary with explicit HTTPS origins, GET-only
  typed JSON reads, bounded time, size, concurrency, retries, Retry-After handling,
  redirect rejection, sensitive session headers, and redacted typed error causes.
  No D&D Beyond hosts or endpoints are enabled, and neither binary sends requests.
- Three-crate Rust foundation with help, version output, and offline process tests.
- Development and agent guidance, architecture, authentication design, and roadmap.
- CI checks, issue and PR templates, and native binary packaging.
- Dated access feasibility assessment with official sources, per-read permission
  questions, browser-login limits, and a blocked live integration gate.
- Personal interactive lookup scope and a browser-handoff comparison based on
  native OAuth and cookie documentation.
- Ignored, authentication-only developer probe for a manual Cobalt session
  exchange, with hidden input, bounded requests, redacted errors, and synthetic
  offline tests. One successful live exchange was reported by the user on
  2026-10-27; data reads, expiry, and automatic login remain untested.

### Changed

- Remove the blanket upstream-permission development stop gate at the
  maintainer's request. Keep dated evidence, access-control protections, technical
  contract requirements, and separate approval for live account tests and
  credential access. Unsupported operations still fail; diagnostics now state
  the missing technical implementation instead of the removed gate.

### Fixed

- Stop before starting an HTTP attempt when a queued permit becomes available
  after the total deadline. Accept case-insensitive identity content encoding.

No account login, D&D Beyond reads, or MCP serving is available yet.
