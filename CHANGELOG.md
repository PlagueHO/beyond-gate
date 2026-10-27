# Changelog

This project uses [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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

### Fixed

- Stop before starting an HTTP attempt when a queued permit becomes available
  after the total deadline. Accept case-insensitive identity content encoding.

No account login, D&D Beyond reads, or MCP serving is available yet.
