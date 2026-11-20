# Repository protections

This policy addresses [#39](https://github.com/PlagueHO/beyond-gate/issues/39).
Keep design choices, exact approved settings, and applied settings separate.
A proposal is not an active protection.
Policy changes, release tags, and release publication need explicit approval.

## Proposed settings

The maintainer selected the following design during planning.
This section preserves the proposal approved on 2026-11-20.
See the separate approval and applied-evidence records below.

### Default branch

Update the existing `main` ruleset, ID `27710613`.
Use `target: branch`, `enforcement: active`, and
`conditions.ref_name.include: ["~DEFAULT_BRANCH"]`, with no exclusions.
Use an empty bypass list. Administrators cannot bypass these merge rules normally.

| Rule or parameter | Proposed value |
|-------------------|----------------|
| `deletion` | Block default-branch deletion |
| `non_fast_forward` | Block force pushes |
| `required_approving_review_count` | `0`; a PR is still required |
| `required_review_thread_resolution` | `true` |
| `require_code_owner_review` | `false` |
| `require_last_push_approval` | `false` |
| `dismiss_stale_reviews_on_push` | `false` |
| `required_reviewers` | `[]` |
| `require_extra_approval_for_unattributed_changes` | `false` |
| `allowed_merge_methods` | `["merge", "squash", "rebase"]` |
| `strict_required_status_checks_policy` | `true` |
| `do_not_enforce_on_create` | `true`, unchanged from the existing rule |
| CodeQL `security_alerts_threshold` | `high_or_higher` |
| CodeQL `alerts_threshold` | `errors` |
| Copilot `review_on_push` | `true` |
| Copilot `review_draft_pull_requests` | `false` |

Zero required approvals avoid blocking the sole maintainer's own PRs.
They do not remove maintainer review duties.
Automatic Copilot review does not replace human release approval.
Do not activate GitHub Code Quality blocking until its results are verified.
Do not add signed-commit, linear-history, merge-queue, or deployment requirements.

### Required checks

Require these eleven exact check names.
Set `integration_id: 15368` for each check, the observed GitHub Actions app.
This binds the source app, not a particular workflow file.
Review workflow changes as part of maintainer review.

| Workflow file | Check names |
|---------------|-------------|
| `.github/workflows/ci.yml` | `Rust (ubuntu-latest)`, `Rust (macos-latest)`, `Rust (windows-latest)` |
| `.github/workflows/ci.yml` | `Synthetic native messaging (chrome, Windows)`, `Synthetic native messaging (edge, Windows)` |
| `.github/workflows/docs.yml` | `site`, `markdown` |
| `.github/workflows/dependency-checks.yml` | `dependencies` |
| `.github/workflows/codeql.yml` | `Analyze (actions)`, `Analyze (javascript-typescript)`, `Analyze (rust)` |

These workflows run on PRs targeting `main` and on main pushes.
They have no path filters that exclude these required checks.
Strict checks require a PR to be up to date with its base.
If a check name or matrix changes, approve its matching rule change before
removing the old check. A missing required check can block merges.

Do not require `Deploy checked documentation`, which is skipped on PRs.
Do not require the path-filtered package jobs, tag provenance, or manual live tests.
Ordinary checks use synthetic data and do not need account credentials.

### Version tags

Use two separate active rulesets with `target: tag`.
Both target `conditions.ref_name.include: ["refs/tags/v*"]`, with no exclusions.
Other tags are outside this policy.

| Ruleset name | Rules | Bypass |
|--------------|-------|--------|
| `Version tag creation` | `creation` | `RepositoryRole`, actor ID `5`, mode `always` |
| `Version tag integrity` | `update`, `deletion`, `non_fast_forward` | None |

Actor ID `5` is the repository administrator role.
The approved request included `update_allows_fetch_and_merge: false`.
GitHub omits this branch-only parameter from tag-rule responses.
The applied tag rule is `{"type":"update"}` with no bypass.
It still restricts all matching tag updates.
Creation permission must not also permit tag replacement or deletion.
Do not combine these rulesets or grant a bypass to GitHub Actions or Copilot.
The release job uses an existing tag with `--verify-tag`.
It does not need tag-creation permission.

### Releases and permissions

Enable repository immutable releases for future publications.
Prepare and review all assets in a draft before publication.
Published assets and the associated tag then become immutable.
The title, release notes, pre-release marker, and latest marker remain editable.
Use a new version to correct locked assets. Do not move an existing version tag.

Keep these permissions and behaviors unchanged:

| Area | Required behavior |
|------|-------------------|
| Default Actions token | `read`; `can_approve_pull_request_reviews: false` |
| Package build | `contents: read` |
| Provenance job | `contents: read`, `actions: read`, `id-token: write`, `attestations: write` |
| Draft job | `contents: write`; only a version-tag push, after build and provenance |
| PR packaging | Read-only checks; no draft or publication |
| Manual packaging | No release; only a default-branch manual run can attest a candidate |
| Pages | Existing `github-pages` environment and main-only deployment policy |

Do not widen action allowlists, token permissions, or workflow triggers.
The existing workflow creates drafts only; it does not publish releases or crates.
GitHub users with write access can manage releases.
Tag restrictions and immutability do not enforce administrator-only publication.
Repository administrators can edit repository rules.
These are control limits, not permission to bypass the documented review process.

## Approved settings

On 2026-11-20, the maintainer explicitly approved the exact settings in this
session: eleven checks, zero approvals, no main bypass, separate version-tag
creation and integrity rules, and future release immutability.
The earlier unavailable response did not authorize writes.
Application began only after the later explicit approval.

The approved ruleset requests match the proposal above.
The only response representation difference is the branch-only tag parameter
described in the applied record. No protection was weakened.
Future policy changes still need their own explicit approval.

## Applied settings and evidence

All approved policy changes were applied and read back on 2026-11-20.

### Before application

The baseline below was inspected before application on 2026-11-20.

| Area | Baseline |
|------|----------|
| Repository | Public, default branch `main` |
| Classic main protection | HTTP 404, `Branch not protected` |
| Effective main rules | Empty |
| Rulesets | Disabled `main`, ID `27710613`; no other or inherited rulesets returned |
| Disabled main rule | One approval; three Rust checks and `Analyze (rust)`; administrator `always` bypass |
| Other disabled rules | CodeQL alert blocking, Code Quality errors, and Copilot review |
| Immutable releases | `enabled: false`, `enforced_by_owner: false`; no releases returned |
| Actions | Read-only token; PR approval disabled; all actions allowed; global SHA pinning not required |
| Pages | `github-pages` environment permits branch `main` only |

At main commit `c56260ffc4499d3e8318576bce86f60fe4165628`, all eleven proposed checks
returned `success` from app ID `15368` during the refreshed inspection.
Successful PR runs also show the same names:
[CI](https://github.com/PlagueHO/beyond-gate/actions/runs/44700081873),
[documentation](https://github.com/PlagueHO/beyond-gate/actions/runs/44700081914),
[dependencies](https://github.com/PlagueHO/beyond-gate/actions/runs/44700081869),
and [CodeQL](https://github.com/PlagueHO/beyond-gate/actions/runs/44700081853).

### Final applied configuration

| Setting | Verified result |
|---------|-----------------|
| [Main ruleset](https://github.com/PlagueHO/beyond-gate/rules/27710613) | ID `27710613`, active, default branch, no bypass, six approved rules |
| Main reviews | PR required, zero approvals, resolved review threads, no code-owner or last-push approval |
| Main checks | All eleven exact names above, app ID `15368`, strict, `do_not_enforce_on_create: true` |
| CodeQL and Copilot | Approved alert thresholds and push review; Code Quality blocking removed |
| [Version tag creation](https://github.com/PlagueHO/beyond-gate/rules/27715153) | ID `27715153`, active, `refs/tags/v*`, creation rule, administrator-role bypass only |
| [Version tag integrity](https://github.com/PlagueHO/beyond-gate/rules/27715160) | ID `27715160`, active, `refs/tags/v*`, update/deletion/force-push restrictions, no bypass |
| Immutable releases | `enabled: true`, `enforced_by_owner: false` |
| Effective main rules | Six rules, all from ruleset `27710613`; branch reports `protected: true` |
| Actions permissions | Unchanged from the baseline: read-only token, PR approval disabled, all actions allowed, no global SHA-pinning requirement |
| Pages | Existing `github-pages` environment remains restricted to branch `main` |

Every returned rule field was compared with its approved request.
At main commit `6e3d7114be530ee8a68c351ebf1973494771cf28`, all eleven check names
had successful evidence from GitHub Actions app ID `15368` before activation.
No workflow or product code changed.
The original requests and final API responses are retained with the issue
session evidence.

### API response differences

GitHub accepted the tag-integrity request but omitted
`update_allows_fetch_and_merge`. The official
[OpenAPI schema](https://github.com/github/rest-api-description/blob/main/descriptions/api.github.com/api.github.com.json)
defines this optional parameter as "Branch can pull changes from its upstream
repository". It does not apply to a tag.
Verification stopped until that distinction was checked.
The final tag response contains an explicit `update` restriction, no parameters,
and no bypass. This preserves the approved tag-update restriction.

The release-immutability enable request succeeded, but the first immediate
read-back still reported `enabled: false`.
A later read with `Cache-Control: no-cache` reported `enabled: true`.
Final inspection also returned `enabled: true`. No second enable write was needed.
The initial read was not treated as evidence of success.

### Local documentation checks

`pwsh -NoProfile -File .\scripts\check-docs.ps1` passed after the repository
installer supplied pinned `rumdl 0.3.2`. `git diff --check` also passed.

## Normal operation

For a main-branch change, open a PR and review its content and check results.
Resolve review threads and update the branch when strict checks require it.
Wait for every required check. Do not replace a failed or missing check with
an empty success result. Merge only when explicitly requested.
Zero required approvals are not an automatic merge authorization.

Before a version tag, review a checked main-branch commit, matching workspace
version and lockfile, dated changelog, native assets, and provenance.
Get explicit approval for the selected version tag.
Review the resulting draft and its exact assets before requesting publication.
Follow [release preparation](../CONTRIBUTING.md#packaging-and-future-releases)
and [maintainer review](../MAINTAINERS.md#changelog-and-release-review).

## Exceptions and recovery

There is no routine bypass for main merges or version-tag updates and deletion.
For a blocked merge, first resolve failed checks, missing checks, stale branches,
or unresolved review threads.

An emergency rule edit needs explicit approval.
Record its reason, exact temporary change, and original API payload.
Change only the affected rule. Do not disable all protections or add a permanent
bypass as a shortcut. Restore the approved settings after recovery and inspect
the effective rules again. Report any restoration failure.

Before each policy write, reread the affected settings.
Stop if they differ from the approved input.
An API denial or failed write must stop further changes.
Record any successful writes separately, and get approval for a rollback.
Do not overwrite another administrator's concurrent change.

Changing a ruleset or disabling future immutability does not unlock an already
immutable release. Deleting an immutable release does not permit reuse of its
tag name. Use a new version instead.

## Verification limits

Use non-destructive API inspection for this issue.
Compare every applied target, rule, check, threshold, permission, and bypass with
the approved payload. Inspect effective default-branch rules separately.
Read back immutable-release settings and current check evidence.

Do not create disposable refs, merge a PR, create a version tag, or publish a
release as a test. Inspection proves configuration, not an observed rejected
push, successful merge, or immutable publication.
Do not claim those behaviors were tested.

## References

- [GitHub rules and bypass behavior](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets)
- [Ruleset REST API](https://docs.github.com/en/rest/repos/rules)
- [Immutable release behavior](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)
- [Enable future release immutability](https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/establish-provenance-and-integrity/prevent-release-changes)
- [Release management permissions](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
