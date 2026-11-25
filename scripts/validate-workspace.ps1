# Run ordinary offline workspace checks in order and preserve the first failure.
$ErrorActionPreference = 'Stop'
$checks = @(
    @('cargo.ps1', 'fmt', '--all', '--', '--check'),
    @('cargo.ps1', 'clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings'),
    @('cargo.ps1', 'test', '--workspace', '--locked'),
    @('cargo.ps1', 'test', '--workspace', '--doc', '--locked'),
    @('cargo.ps1', 'doc', '--workspace', '--no-deps', '--document-private-items', '--locked'),
    @('cargo.ps1', 'build', '--workspace', '--locked'),
    @('cargo.ps1', 'build', '--release', '--workspace', '--locked'),
    @('test-developer-workflow.ps1'),
    @('check-docs.ps1'),
    @('test-doc-checks.ps1')
)
foreach ($check in $checks) {
    $script = Join-Path $PSScriptRoot $check[0]
    $arguments = @($check | Select-Object -Skip 1)
    & pwsh -NoProfile -File $script @arguments
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
exit 0
