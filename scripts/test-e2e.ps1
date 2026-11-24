#Requires -Version 7.4
# Run built executables only. This stage never requests or exchanges credentials.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$BinaryDirectory,
    [ValidateSet('full', 'contracts')][string]$Selection = 'full',
    [switch]$ApproveNativeStorage,
    [string]$ReportPath,
    [ValidateRange(1, 600)][int]$SuiteTimeoutSeconds = 180
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Import-Module (Join-Path $PSScriptRoot 'e2e\common.psm1') -Force -DisableNameChecking
& (Join-Path $PSScriptRoot 'install-test-tools.ps1') -Check
Import-Module Pester -RequiredVersion 6.2.0 -DisableNameChecking
Import-Module (Join-Path $PSScriptRoot 'e2e\pester.psm1') -DisableNameChecking
$files = @('Cli.Tests.ps1', 'Mcp.Tests.ps1', 'Prerequisites.Tests.ps1') |
    ForEach-Object { Join-Path $PSScriptRoot "e2e\$_" }
$configuration = New-E2ePesterConfiguration $files @{
    BinaryDirectory = [IO.Path]::GetFullPath($BinaryDirectory)
    Selection = $Selection; ApproveNativeStorage = [bool]$ApproveNativeStorage
    Deadline = [datetime]::UtcNow.AddSeconds($SuiteTimeoutSeconds)
}
try {
    $result = Invoke-Pester -Configuration $configuration
    $report = ConvertTo-E2ePesterReport $result $Selection ([bool]$ApproveNativeStorage)
} catch {
    $cases = Get-E2eCases (Get-E2eInventory)
    $cases.Add((New-E2eCase 'suite.infrastructure' 'infrastructure'))
    Set-E2eResult $cases[-1] 'failed' (Get-E2eReason $_)
    $report = Get-E2eReport $cases $Selection
}
$json = $report | ConvertTo-Json -Depth 10
if ($ReportPath) { [IO.File]::WriteAllText([IO.Path]::GetFullPath($ReportPath), "$json`n") }
$json
exit $report.exit_code
