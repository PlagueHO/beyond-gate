# Build and test the local documentation site, stopping on the first failure.
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    foreach ($command in @('build', 'test')) {
        & pnpm --dir docs $command
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
}
finally {
    Pop-Location
}
exit 0
