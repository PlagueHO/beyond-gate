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
