#Requires -Version 7.4
[CmdletBinding()]
param(
    [string]$SelectionFile,
    [ValidateSet('all', 'book', 'character', 'campaign', 'class', 'subclass', 'monster',
        'background', 'feat', 'species', 'spell', 'equipment', 'magic_item')]
    [string]$Entity = 'all',
    [switch]$ApproveLive, [switch]$UseEnvironment, [string]$ReportPath
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (!$ApproveLive) {
    [Console]::Error.WriteLine('Live entity checks require -ApproveLive. No selections, credentials, or requests were used.')
    exit 1
}
if (!$IsWindows -and !$IsLinux) {
    [Console]::Error.WriteLine('Live entity checks support Windows and Linux only.')
    exit 1
}
try {
    if (!$SelectionFile) { throw 'selection_invalid' }
    $path = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($SelectionFile)
    $file = [IO.FileInfo]::new($path)
    if (!$file.Exists -or $file.Length -gt 16384) { throw 'selection_invalid' }
    $selection = [IO.File]::ReadAllText($path, [Text.UTF8Encoding]::new($false, $true))
} catch {
    [Console]::Error.WriteLine('Selection file is missing, invalid, or larger than 16 KiB. No credentials or requests were used.')
    exit 1
}
& (Join-Path $PSScriptRoot 'install-test-tools.ps1') -Check
Import-Module (Join-Path $PSScriptRoot 'e2e\pester.psm1') -DisableNameChecking
Import-Module (Join-Path $PSScriptRoot 'e2e\live-api.psm1') -DisableNameChecking
$scope = @(Get-LiveEntityScope $Entity)
$result = Invoke-Pester -Configuration (New-E2ePesterConfiguration (Join-Path $PSScriptRoot 'e2e\LiveEntities.Tests.ps1') @{
    SelectionJson = $selection; Entity = $Entity; ApproveLive = $true; UseEnvironment = [bool]$UseEnvironment
})
$safeReasons = @(
    'approval_required', 'unsupported_platform', 'selection_invalid', 'input_missing', 'input_invalid',
    'terminal_required', 'build_failed', 'build_contract_failed', 'process_start', 'process_timeout',
    'suite_timeout', 'output_limit', 'cleanup_failed', 'read_failed', 'timed_out', 'rate_limited',
    'access_denied', 'redirect_rejected', 'body_limit', 'session_absent', 'session_rejected',
    'session_expired', 'invalid_material', 'unsupported_contract', 'storage_access',
    'storage_unavailable', 'ambiguous_entry', 'storage_failure', 'validation_unavailable',
    'authenticated_read_failed', 'record_not_found', 'session_changed'
)
$rows = @(foreach ($case in $scope) {
    $tests = @($result.Tests | Where-Object ExpandedName -CEQ $case.id)
    $status = 'failed'; $reason = 'harness_error'
    if ($tests.Count -eq 1) {
        if ($tests[0].Result -ceq 'Passed') { $status = 'passed'; $reason = 'verified' }
        elseif ($tests[0].Result -ceq 'Skipped') { $status = 'blocked'; $reason = 'stopped_after_failure' }
        else {
            foreach ($errorRecord in $tests[0].ErrorRecord) {
                if ($errorRecord.Exception.Message -cin $safeReasons) { $reason = $errorRecord.Exception.Message }
            }
        }
    }
    @{ case = $case.id; status = $status; reason = $reason }
})
$passed = $result.Result -ceq 'Passed' -and $result.TotalCount -eq $scope.Count -and
    $result.PassedCount -eq $scope.Count -and @($rows | Where-Object status -CNE 'passed').Count -eq 0
$report = @{
    schema = 1; suite = 'live-entities'; scope = $Entity
    full_coverage = ($Entity -ceq 'all' -and $passed)
    status = $(if ($passed) { 'passed' } else { 'failed' })
    live_request_budget = ($scope | Group-Object family | ForEach-Object { $_.Group[0].budget } | Measure-Object -Sum).Sum
    cases = $rows
}
if ($ReportPath) {
    $json = ConvertTo-Json $report -Depth 5
    [IO.File]::WriteAllText($ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($ReportPath),
        "$json`n", [Text.UTF8Encoding]::new($false))
}
foreach ($row in $rows) { "$($row.case): $($row.status) ($($row.reason))" }
if (!$passed) {
    [Console]::Error.WriteLine('Live entity coverage is incomplete. Raw diagnostics are suppressed.')
    exit 1
}
"Selected live entity scope passed: $Entity."
exit 0
