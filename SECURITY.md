# Security policy

## Report a vulnerability privately

Use this repository's
[private vulnerability reporting form](https://github.com/PlagueHO/beyond-gate/security/advisories/new).
Private reporting is enabled. A GitHub account is required.
See GitHub's [reporting instructions](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately).

Do not disclose a suspected vulnerability in a public issue or pull request.
Use public issues for ordinary bugs and development questions only.
If the private form is unavailable, open an issue that asks for a private
reporting route. Do not include vulnerability details in that issue.

Include the affected version, operating system, expected result, observed
result, and safe reproduction steps. Use synthetic data and a local test case.
Describe the possible impact and any known workaround.
Never include credentials, cookies, tokens, private URLs, real account snapshots,
or proprietary D&D Beyond content, even in a private report.
Do not bypass access controls or test accounts that you do not control.

## Supported versions

Security fixes target the latest published release only. Older releases do not
receive backports. Upgrade to the latest release before reporting a problem
that might already be fixed.

There are no published releases at the time this policy is introduced.
The default branch is development code, not a supported binary release.
Reports about development code are still welcome through the private route.

## Response and disclosure

The [maintainer](MAINTAINERS.md) reviews reports through GitHub's private advisory
tools. There is no guaranteed response time, fix date, or support period.
A report does not promise a fix or a release.

Keep report details private while the maintainer assesses the report.
Use the private thread to discuss impact, a fix, and coordinated disclosure.
Do not put sensitive evidence in a public advisory or changelog.

## Dependency checks and limits

The repository checks Rust dependencies against RustSec advisories and a
reviewed license policy. See the
[dependency-check commands and limits](CONTRIBUTING.md#rust-dependency-advisory-and-license-checks).
The same policy runs locally and in CI.

CodeQL checks source code. Dependabot reports known dependency problems.
Secret scanning and push protection check for exposed secrets, not dependency
licenses. These controls have different roles. No check proves that the project
is free of vulnerabilities or that a dependency is safe in every use.
