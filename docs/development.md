# Development

Beyond Gate is a native Rust workspace. Node.js and pnpm are only needed to work
on this documentation site.

Start with [Contributing](../CONTRIBUTING.md) for native setup, editor tasks, and
the Windows Visual Studio 2022 Build Tools environment.
Use the [repository commands](../AGENTS.md) for Rust checks.

## Developer task index

Copilot App and VS Code use the same subject-first names.
Select a task manually. No task runs on session creation or archive.
Review and accept `.github/github-app.yml` in Copilot App after it changes.
See the [official repository configuration reference](https://docs.github.com/copilot/reference/github-copilot-app-reference/repository-configuration).

| Subject | Tasks | Prerequisites |
|---------|-------|---------------|
| Rust | Build debug; Build release; Check formatting; Apply formatting; Check Clippy; Test workspace; Test examples; Build API docs | Repository Rust toolchain; native build tools; PowerShell 7 |
| CLI | Run; Help; Test process; Test adapters | Native Rust setup |
| MCP | Serve stdio; Help; Test process | Native Rust setup; an MCP client for tool calls |
| Workspace | Fetch dependencies; Test developer workflow; Validate | Native Rust setup; pinned Markdown checker for Validate |
| Docs | Install checker; Check Markdown; Check links; Test checker | Native Rust setup to install the pinned checker |
| Docs | Install site dependencies; Run site; Build site; Test site; Preview site; Build themes; Preview themes; Validate site | Node.js, pinned pnpm; Chrome or Edge for site tests |
| Dependencies | Install checker; Check; Check offline; Test checker | Native Rust setup; pinned cargo-deny; fresh advisory data or a valid offline cache |
| Docker | Build image; CLI help; MCP help; Test runtime | Docker with a running Linux engine; built image for run/check tasks |

Task labels include the subject and colon, such as **Rust: Build debug**.
**Workspace: Validate** stops on the first failed check.
It does not install tools, run Docker, test the site, or contact a live account.
Use separate Docker, site, and dependency tasks for those checks.
**Rust: Apply formatting** changes source files.
**CLI: Run** and **MCP: Serve stdio** do not perform login or tool calls.
**CLI: Run** shows the no-command overview and exits with the existing usage code 2.
The CLI has no interactive command loop. Use **CLI: Help** for a successful help
command, or pass subcommands through the native run helper in a terminal.
**MCP: Test process** builds both sibling binaries before its focused checks.
No task authorizes credential access or authenticated operations.
Live tests, credential changes, release publication, and site deployment remain
separate manual workflows.

## Native build and run

Follow [native setup](../CONTRIBUTING.md#local-development) from a clean checkout.
Run these commands from the checkout root in PowerShell 7:

```powershell
pwsh -NoProfile -File .\scripts\cargo.ps1 build --workspace --locked
pwsh -NoProfile -File .\scripts\run.ps1 cli
pwsh -NoProfile -File .\scripts\run.ps1 cli --help
pwsh -NoProfile -File .\scripts\run.ps1 cli character list --help
pwsh -NoProfile -File .\scripts\run.ps1 mcp --help
```

On Linux and macOS, use `/` path separators.
The run helper builds both binaries before each run with locked dependencies.
Cargo reuses unchanged build output. Its artifact messages select the executable,
including a configured Cargo target directory.
Build failures stop the helper before the product starts.
Build diagnostics use stderr. Product stdin, stdout, stderr, and exit status are
preserved. CLI options after `cli` go to the product, not to Cargo.
No application data folder or persistent content cache is created.

For repeated runs without a build, use the built binaries directly:

```powershell
.\target\debug\beyond-gate.exe --help
.\target\debug\beyond-gate-mcp.exe --help
```

These paths assume the default target directory and debug profile.
Linux and macOS binaries have no `.exe` suffix.
Use the release build task for optimized binaries under `target\release`.
Do not put credentials in arguments, tasks, transcripts, or command output.
Authenticated manual exploration requires separate approval and the existing
[authentication workflow](authentication.md).

### MCP client connection and restart

An MCP client owns stdin and stdout and starts the server when it connects.
Use the absolute built server path in the client's stdio configuration.
For example, replace this placeholder with your checkout path:

```json
{
  "mcpServers": {
    "beyond-gate": {
      "command": "C:\\path\\to\\checkout\\target\\debug\\beyond-gate-mcp.exe",
      "args": []
    }
  }
}
```

Keep client-specific fields from the [MCP reference](mcp.md#client-registration).
No URL or port is needed. Do not register an app terminal as an MCP connection.
Build first, then connect through the client.
The server responds to initialization and tool discovery without a live read.
It does not prompt for credentials.

To observe manual startup without making tool calls:

```powershell
pwsh -NoProfile -File .\scripts\run.ps1 mcp
```

The server waits for protocol input. It is not an interactive command prompt.
Do not enter ordinary text. Press **Ctrl+C** to stop terminal execution.
A normal client stops its child process by closing stdin.
To restart, disconnect the client, rebuild if needed, and reconnect.
Stop running binaries before replacing them on Windows.
The CLI exits when its command finishes. Run the command again to restart it.

Saved credentials remain in Windows Credential Manager, macOS Keychain, or
Linux Secret Service. They do not reside in the checkout or build output.
Process-only input and derived session evidence do not persist after exit.
A restarted process must receive its input again or select the existing OS entry.
No logout, OS-store read, or live validation is part of the checks below.

## Local runtime container

The root `Dockerfile` builds a separate Linux runtime image with both release
binaries, CA certificates, and a non-root user.
It uses the checkout's Rust toolchain and `Cargo.lock`.
The final image has no compiler, source fixtures, host credential mounts,
published ports, or product cache.
`.dockerignore` includes only the build source, manifests, and required logo.
The [development container](../CONTRIBUTING.md#development-container) instead
provides an editor and build tools.

Install Docker with a running Linux engine.
Build and check the image from the checkout root:

```powershell
docker build --tag beyond-gate:local .
docker run --rm --network none beyond-gate:local --help
docker run --rm --network none --entrypoint beyond-gate-mcp beyond-gate:local --help
pwsh -NoProfile -File .\scripts\test-runtime-container.ps1
```

Build requires network access for base images, Rust, and locked dependencies.
The runtime checks disable network access and use no account data.
They check both binaries, CLI errors, direct and sibling MCP startup, discovery,
invalid input, stdin closure, and removal of each owned container.
Missing Docker, a stopped engine, or a missing image fails explicitly.
The checker accepts `-Image` when you use a different local image tag.

### Container stdio and lifecycle

For an MCP client, set `command` to `docker` and use these arguments:

```json
{
  "mcpServers": {
    "beyond-gate": {
      "command": "docker",
      "args": [
        "run", "--rm", "-i", "--network", "none",
        "--entrypoint", "beyond-gate-mcp", "beyond-gate:local"
      ]
    }
  }
}
```

This example permits offline initialization and discovery only.
`-i` keeps stdin open. Do not add `-t`: a TTY changes protocol streams.
Do not add `-d`: the client must own the attached stdio connection.
No HTTP listener or host port is used.
See the [Docker run reference](https://docs.docker.com/reference/cli/docker/container/run/).

`--rm` removes the container after it stops. Reconnect to start a fresh process.
For a manual named CLI run, use:

```powershell
docker run --rm -i --name beyond-gate-manual --network none beyond-gate:local --help
```

If a named manual container remains running, stop only that container:

```powershell
docker stop beyond-gate-manual
docker ps -a --filter name=beyond-gate-manual
```

The image remains after containers exit.
After disconnecting clients and stopping containers, remove only this local image:

```powershell
docker image rm beyond-gate:local
```

Do not use broad Docker prune commands for cleanup.
No new application files or session cache persist across these runs.
No host directory or volume is required.

### Container authentication limits

Windows browser login cannot run inside this Linux image.
The image has no desktop, user-session D-Bus, or Secret Service provider.
Saved-session login, status, and logout are not supported runtime workflows here.
Do not mount host credential stores or add a plaintext substitute.

Linux process-only input is implemented by the shared core without an OS store.
Live container account/client acceptance is not verified.
Any authenticated container operation requires separate approval and a defined
request budget. It also requires network access; the examples above deny it.
Never place a credential value in an image, Dockerfile, argument, `.env` file,
client configuration, log, or repository.
Docker can retain explicitly supplied environment values in container metadata
while a container exists. `--rm` is cleanup, not a protected credential store.
Do not treat native account acceptance as container acceptance.

## Workflow checks and environment evidence

Run the native workflow checks without accessing a live account:

```powershell
pwsh -NoProfile -File .\scripts\test-developer-workflow.ps1
pwsh -NoProfile -File .\scripts\validate-workspace.ps1
```

Synthetic checks cover paths with spaces, input and argument forwarding,
separate output streams, build/product failures, missing artifacts, task-name
parity, and first-failure behavior.
Native checks exercise the actual run helper and MCP discovery, invalid tool
input, and shutdown after stdin closes.
CI runs native workflow checks on Windows, Linux, and macOS.
A separate Linux CI job builds and checks the runtime image.
CI configuration is not evidence that a run passed.

Windows native helper checks passed locally.
The [CI run for commit d10c552](https://github.com/PlagueHO/beyond-gate/actions/runs/45559225466)
passed native workflow checks on Windows, Linux, and macOS.
Its Linux runtime job built the documented Docker image and passed the
network-disabled container help, error, MCP discovery, stdin closure, and cleanup
checks. The local Windows Docker engine was unavailable; container acceptance
uses the Linux CI evidence, not a local Docker run.

On 2026-11-25, the developer confirmed that **Workspace: Test developer workflow**
reported 38 matching tasks in Copilot App.
The developer also confirmed **MCP: Serve stdio** stopped with **Ctrl+C** and
could start and stop again. This is developer-reported manual acceptance,
not an automated UI check. Copilot App blocks automated UI control.
Use a normal terminal if an app task cannot supply the required streams.
No live account or native credential-store check was run.

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
