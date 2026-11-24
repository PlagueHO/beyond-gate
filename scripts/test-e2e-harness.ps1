#Requires -Version 7.4
# Deterministic harness tests: synthetic values and child processes only.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
& (Join-Path $PSScriptRoot 'install-test-tools.ps1') -Check
Import-Module Pester -RequiredVersion 6.2.0 -DisableNameChecking
Import-Module (Join-Path $PSScriptRoot 'e2e\pester.psm1') -DisableNameChecking
$files = @('Harness.Tests.ps1', 'Dispatch.Tests.ps1', 'Reporting.Tests.ps1',
    'LiveApi.Harness.Tests.ps1', 'LiveEntities.Harness.Tests.ps1') |
    ForEach-Object { Join-Path $PSScriptRoot "e2e\$_" }
$result = Invoke-Pester -Configuration (New-E2ePesterConfiguration $files)
foreach ($test in $result.Tests) {
    # Names come only from these fixed synthetic tests; never print ErrorRecord.
    "$($test.ExpandedName): $($test.Result)"
}
if ($result.Result -ne 'Passed') {
    [Console]::Error.WriteLine('Offline Pester harness failed. Raw diagnostics are suppressed.')
    exit 1
}
"Passed $($result.PassedCount) offline Pester tests. No native credentials or live accounts were accessed."
exit 0
