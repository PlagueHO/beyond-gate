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

## Pages validation and deployment

The [documentation workflow](https://github.com/PlagueHO/beyond-gate/actions/workflows/docs.yml)
checks every pull request, push to `main`, and explicit manual run.
Broad event coverage includes shared Markdown, assets, configuration, dependencies,
and workflow changes. No path filter can omit a required site check.

Site and Markdown jobs have read-only repository permissions.
They build and test production output, check all five themes, and check Markdown
style and offline links. Pull requests cannot upload a Pages artifact or run the
deployment job. This includes pull requests from branches in this repository.

Successful default-branch push and manual runs can publish only
`docs/.vitepress/dist`. Theme samples are not published.
Deployment requires both validation jobs to succeed. Its separate job has
`actions: read`, `contents: read`, `pages: write`, and `id-token: write`.
No documentation dependencies are installed or run in that job.
Actions use full commit SHA pins. Site dependencies use the frozen lockfile.

The deployment uses the existing `github-pages` environment and its `main` branch
policy. The workflow does not add a reviewer approval rule or change repository
policy. Do not treat the environment name as proof of human approval.
The first live deployment received explicit maintainer approval.
Successful pushes to `main` deploy automatically.

One `github-pages` concurrency group serializes deployments without cancelling an
active deployment. After any environment or concurrency wait, the preflight checks
the Pages host, base path, publishing type, and current default-branch commit.
An older run fails even if its build passed. Failed API reads stop deployment.
The deployment action consumes only the artifact from that workflow run.

### Published address and acceptance

The configured site uses `danielscottraynsford.com` and `/beyond-gate/`.
Do not replace it with a `github.io` address or remove the base path.
The workflow reads Pages metadata without enabling Pages or changing its settings.
An unexpected host, base path, or publishing type stops deployment.

The tested canonical link is
[https://danielscottraynsford.com/beyond-gate/](https://danielscottraynsford.com/beyond-gate/).
On 2026-11-18, both the approved
[automatic deployment](https://github.com/PlagueHO/beyond-gate/actions/runs/44280511769)
and [manual deployment](https://github.com/PlagueHO/beyond-gate/actions/runs/44281746032)
passed for commit `7b7b87204d4ac555e66a876ec17cbbb0d5ff894a`.

A fresh browser verified the home page, installation, CLI, MCP, and development
pages, images, CSS, JavaScript, and the direct `cli.html#auth-validate` fragment.
Local search for `auth validate` returned a working link to that fragment.
The final browser check reported no JavaScript errors or failed same-site responses.
Certificate verification remained enabled.

The Pages API still reports `http://danielscottraynsford.com/beyond-gate/`,
an approved TLS certificate, and `https_enforced: false`.
HTTP also returns 200. It does not redirect to HTTPS.
Use the verified HTTPS link above.
Domain, DNS, HTTPS-enforcement, and environment policy changes need separate approval.

### Manual runs

Use **Actions > Documentation checks and Pages > Run workflow**.
Select `main`, then select **Run workflow**.
The CLI equivalent, run from the repository root, is:

```powershell
gh workflow run docs.yml --repo PlagueHO/beyond-gate --ref main
gh run list --repo PlagueHO/beyond-gate --workflow docs.yml --limit 5
```

Open the new run to check both validation jobs and the deployment result.
The deployment environment links to the URL returned by the Pages action.
That URL does not replace public browser acceptance.
A manual feature-branch or tag run can validate but cannot publish.
An old run cannot publish after `main` advances.

### Deployment recovery

If a validation job fails, fix the failure before attempting deployment.
The existing site remains unchanged when no deployment starts.
If Pages deployment fails, inspect the deployment action and Pages service status.
Rerun the workflow only if its commit is still the current default-branch commit.
If `main` has advanced, start a fresh manual run instead.
Uploaded Pages artifacts expire after one day; a full new run rebuilds the artifact.

To restore earlier content, revert the responsible change on `main` through the
normal review process. Let the new revert commit pass all checks and deploy.
Do not rerun an older deployment to bypass the current-commit rule.
Pending deployments can be replaced by another pending run in the concurrency
group. If this leaves the current commit without a deployment, start a fresh run.
Do not cancel an active deployment merely to move another run ahead.

Refresh tabs opened before a deployment if search or navigation fails.
An open tab can retain an earlier application bundle that requests removed,
hashed JavaScript assets. This occurred between the automatic and manual
acceptance deployments. A fresh browser loaded the final assets and passed.
This workflow does not guarantee that open tabs survive deployment or that all
CDN caches update at the same instant.

Check deployment refusal cases locally without GitHub access:

```powershell
node --test .\docs\tests\pages-deployment.test.mjs
```

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
- Read [Authentication design](authentication-design.md) for session selection and platform limits.
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
