# Maintainers

[@PlagueHO](https://github.com/PlagueHO) is the sole maintainer and code owner.
Use repository issues for bugs, development questions, and maintenance proposals.
No response time or support period is promised.

## Review responsibilities

Review changes for correctness, useful tests, clear documentation, and the
project's local, read-only scope. Check that offline tests use synthetic data.
Do not approve access-control bypasses, real account fixtures, or credentials.
Follow [Contributing](CONTRIBUTING.md) for commands and pull request requirements.

[CODEOWNERS](.github/CODEOWNERS) assigns all paths to @PlagueHO. GitHub uses the
file on the pull request's base branch to request owner review on ready pull
requests. Draft pull requests do not request owner review automatically.
The owner must have write access. Ownership does not itself require approval;
that needs a separate repository rule. This setup does not change those rules.
See GitHub's [code-owner reference](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners).

## Dependency maintenance

Follow the [dependency policy](CONTRIBUTING.md#dependency-maintenance).
Review upstream release notes, compatibility, changed features, lockfile changes,
and security fixes. Keep GitHub Actions pinned to full commit SHAs.
Run the relevant checks before approval. Major updates need their own review.
Do not automatically approve or merge updates.

Review tools outside Dependabot's scope manually. Check the Rust toolchain,
container image and features, native packages, PowerShell, editor extensions, and
the pinned Markdown checker. Keep checker versions consistent across setup and CI.

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

## Code signing policy

The maintainer approved free GitHub build provenance for issue
[#35](https://github.com/PlagueHO/beyond-gate/issues/35). Signing must cost nothing.
Do not buy certificates, enroll in paid programs, or provision paid signing
services. Native publisher signing is not a release requirement when no approved
free route is available.

[GitHub artifact attestations](https://github.com/actions/attest) are available
for public repositories on all current GitHub plans. They use GitHub Actions
OIDC and Sigstore without a purchased certificate or a stored private signing key.
Existing Actions usage limits still apply. Public attestations enter a public
transparency log; never include private account data.

Require the final archive and checksum bytes to match an attestation from
`PlagueHO/beyond-gate/.github/workflows/release.yml`, the reviewed source commit,
and the expected tag or default-branch ref. Only version-tag pushes and manual
default-branch runs can create attestations. PR packages and other-ref manual
packages are unattested test outputs. A missing or failed attestation blocks the
verified-download path and draft release preparation; do not fall back to a
checksum-only claim.

Provenance links bytes to a build identity. It does not prove that the software
is safe, reproducible, or approved by an OS publisher program. Trust depends on
GitHub, Sigstore, and control of the repository and workflow. See
[GitHub's trust limits](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
and the [download verification guide](docs/installation.md#verify-the-download).

Windows binaries have no Authenticode publisher signing. macOS packages have no
Developer ID signing or notarization. Ad hoc and self-signed signatures do not
provide those guarantees. [SignPath Foundation](https://signpath.org/) offers
free OSS signing, but its [conditions](https://signpath.org/terms) require service
approval and project controls. No approval or integration is established here.
It is an optional future free route, not a release blocker.

Apple requires a suitable distribution identity for
[notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).
Its [fee waiver](https://developer.apple.com/help/account/membership/fee-waivers/)
has specific organization requirements; no eligible approved arrangement is
established here. Paid membership is not a release prerequisite.

These decisions do not waive native download trust checks. Never disable
SmartScreen, Gatekeeper, quarantine, or organization policy to pass them.
Stop on an OS protection denial. Changes to this policy, release tags, publication,
and applications to external signing services need separate maintainer approval.
