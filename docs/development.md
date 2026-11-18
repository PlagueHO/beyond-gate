# Development

Beyond Gate is a native Rust workspace. Node.js and pnpm are only needed to work
on this documentation site.

Start with [Contributing](../CONTRIBUTING.md) for native setup, editor tasks, and
the Windows Visual Studio 2022 Build Tools environment.
Use the [repository commands](../AGENTS.md) for Rust checks.

## Documentation site

Use Node.js 24 or 26 and pnpm 12.15.0. Run these commands from the repository root.
The docs package pins pnpm. Keep the machine's default package registry.

The previews use stable VitePress 1.6.4 with a narrow Vite 6.4.6 override.
VitePress still requests Vite 5, which no longer receives security fixes.
[Vite 6.4 receives security patches](https://vite.dev/releases#supported-versions).
The override and the esbuild installation-script permission are in
`docs/pnpm-workspace.yaml`. Recheck the override when upgrading VitePress.

```powershell
pnpm --dir docs install --frozen-lockfile
pnpm --dir docs dev
pnpm --dir docs build
pnpm --dir docs preview
pnpm --dir docs test
```

Open the address printed by the server, including `/beyond-gate/`.
The preview serves the production build. It does not publish the site.
Press **Ctrl+C** in the terminal to stop either server.

The normal build uses Gatehouse with light and dark states.
It does not include theme comparison controls or unused theme styles.
The local preview binds only to `127.0.0.1`. Set `DOCS_PREVIEW_PORT` to select
another port. The server fails if that port is in use.

The browser checks use the built output. Install Chrome or Edge, or set
`CHROME_PATH` to an installed browser executable. The tests use a fresh,
temporary browser profile. They do not read account credentials or browse
D&D Beyond. They start and stop their own loopback preview server.

## Five theme samples

The separate comparison build keeps five working themes available for review:
Gatehouse, Crimson folio, Iron atlas, Ember archive, and Ivory sigil.
Use the theme selector to compare the overview and the same CLI reference.
The selector preserves the study when you open another page.

```powershell
pnpm --dir docs build:themes
pnpm --dir docs preview:themes
```

Stop an existing preview first, or select a different `DOCS_PREVIEW_PORT`.
Comparison output is in `docs/.vitepress/studies` and is not committed.
To check that output, set `DOCS_TEST_THEMES=1` before running the browser tests:

```powershell
$env:DOCS_TEST_THEMES = '1'
pnpm --dir docs test
Remove-Item Env:DOCS_TEST_THEMES
```

Gatehouse was selected during autonomous implementation, not through an explicit
visual approval. The samples remain available for further discussion and refinement.

## Design and source contracts

- Read [Architecture](architecture.md) for shared core and adapter boundaries.
- Read [Authentication](authentication.md) for session selection and platform limits.
- Read [Browser login contracts](browser-handoff.md) for the human-controlled handoff.
- Read [Testing](testing.md) for offline tests and separately approved live checks.
- Read the [Roadmap](roadmap.md) for current priorities and development policy.
- Read [Desktop acceptance](desktop-acceptance.md) for native evidence and remaining gates.

## Source research

The [API call catalog](api-catalog.md) describes the source contracts.
The [API survey](api-survey.md) separates observations from native acceptance.
Neither document grants permission to access an account or automate a service.

## Markdown checks

The existing offline checks still apply to source Markdown.
See [Markdown and local links](../CONTRIBUTING.md#markdown-and-local-links).
Do not replace them with network link checks.

```powershell
pwsh -NoProfile -File .\scripts\check-docs.ps1
pwsh -NoProfile -File .\scripts\test-doc-checks.ps1
```
