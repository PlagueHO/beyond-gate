# Maintainers

[@PlagueHO](https://github.com/PlagueHO) is the sole maintainer and code owner.
Use repository issues for bugs, development questions, and maintenance proposals.
No response time or support period is promised.
Use [Support and compatibility](docs/support.md) for pre-1.0 interface review and
upstream failure handling. Check [Privacy](docs/privacy.md) when reviewing changes
to session input, storage, network destinations, or browser/client behavior.
Use the [private reporting route](SECURITY.md) for suspected vulnerabilities,
not public issues. The security policy covers the latest published release only.

## Review responsibilities

Review changes for correctness, useful tests, clear documentation, and the
project's local, read-only scope. Check that offline tests use synthetic data.
Do not approve access-control bypasses, real account fixtures, or credentials.
Follow [Contributing](CONTRIBUTING.md) for commands and pull request requirements.

[CODEOWNERS](.github/CODEOWNERS) assigns all paths to @PlagueHO. GitHub uses the
file on the pull request's base branch to request owner review on ready pull
requests. Draft pull requests do not request owner review automatically.
The owner must have write access. Ownership does not itself require approval;
that needs a separate repository rule.
See the [repository protection policy](docs/repository-protections.md) for
proposed, approved, and applied settings. The default-branch rule requires
PRs and passing checks, but no approving review or normal administrator bypass.
Maintainer review is still required before a requested merge.
See GitHub's [code-owner reference](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners).

## Dependency maintenance

Follow the [dependency policy](CONTRIBUTING.md#dependency-maintenance).
Review upstream release notes, compatibility, changed features, lockfile changes,
and security fixes. Keep GitHub Actions pinned to full commit SHAs.
Run the relevant checks before approval. Major updates need their own review.
Do not automatically approve or merge updates.

Review tools outside Dependabot's scope manually. Check the Rust toolchain,
container image and features, native packages, PowerShell, editor extensions, and
the pinned Markdown and dependency checkers. Keep checker versions consistent
across local setup and CI.

Review [Rust advisory and license results](CONTRIBUTING.md#rust-dependency-advisory-and-license-checks)
before dependency approval. Resolve failures or approve a narrow, justified,
time-bounded exception with a tracking issue. Do not approve blanket ignores.
Confirm that [dependency license duties](CONTRIBUTING.md#dependency-license-review)
are met before binary distribution. A passing checker does not prove compliance.

## Changelog and release review

The project uses [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Add each delivered change under `Unreleased` in `CHANGELOG.md`.
Use the existing Added, Changed, Fixed, or Security categories as applicable.
Do not describe planned behavior as available.

Before a release, move the entries into a dated version section. Keep a new
`Unreleased` section for later changes. Update the shared workspace version and
all affected `Cargo.lock` package versions together. Review compatibility before
selecting the version.

Follow the existing [packaging and release instructions](CONTRIBUTING.md#packaging-and-future-releases).
Review the version, lockfile, dated changelog, native assets, checksums, and
validation evidence before approving a tag or draft publication.
A tag must match the workspace version. Release actions need explicit approval.
This setup does not publish releases or Cargo crates.
Follow the [version-tag and release protection policy](docs/repository-protections.md#version-tags).
Review all final assets in a draft before publication.
Future published assets are immutable. This policy does not publish releases.

## Code signing policy

Signing must cost nothing, as agreed in
[#35](https://github.com/PlagueHO/beyond-gate/issues/35). Do not buy certificates,
enroll in paid programs, or provision paid signing services. Native publisher
signing is not a release requirement without an approved free route.
Any future signing service needs separate maintainer approval.

Use [GitHub artifact attestations](https://github.com/actions/attest), available
for public repositories on all current GitHub plans. Existing Actions usage
limits still apply. GitHub Actions OIDC and Sigstore need no purchased certificate
or stored private signing key. Public attestations enter a public transparency
log; never include private account data.

Require all final archives and checksum sidecars to pass the
[download verification policy](docs/installation.md#verify-build-provenance).
Missing or failed provenance blocks draft preparation.
See [Contributing](CONTRIBUTING.md#release-provenance-checks) for CI checks.

Trust depends on GitHub, Sigstore, and control of the repository and workflow.
See [GitHub's trust limits](https://docs.github.com/en/actions/concepts/security/artifact-attestations).
This policy does not waive
[OS download trust checks](docs/installation.md#system-services-and-trust-limits)
or authorize a protection bypass, release tag, or publication.
