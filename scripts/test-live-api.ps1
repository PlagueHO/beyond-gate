#Requires -Version 7.4
[CmdletBinding()]
param([switch]$ApproveLive, [switch]$UseEnvironment, [string]$ReportPath)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (!$ApproveLive) {
    [Console]::Error.WriteLine('Live API checks require -ApproveLive. No credentials or requests were used.')
    exit 1
}
if (!$IsWindows -and !$IsLinux) {
    [Console]::Error.WriteLine('Live API checks support Windows and Linux only. No requests were used.')
    exit 1
}
& (Join-Path $PSScriptRoot 'install-test-tools.ps1') -Check
Import-Module (Join-Path $PSScriptRoot 'e2e\pester.psm1') -DisableNameChecking
$configuration = New-E2ePesterConfiguration (Join-Path $PSScriptRoot 'e2e\LiveApi.Tests.ps1') @{
    ApproveLive = $true; UseEnvironment = [bool]$UseEnvironment
}
$result = Invoke-Pester -Configuration $configuration
$passed = $result.Result -eq 'Passed' -and $result.PassedCount -eq 1 -and $result.TotalCount -eq 1
$reason = if ($passed) { 'verified' } else { 'harness_error' }
$safeReasons = @(
    'approval_required', 'input_missing', 'input_invalid', 'terminal_required', 'unsupported_platform',
    'session_rejected', 'session_expired', 'timed_out', 'access_denied', 'private_request_failed',
    'public_request_failed', 'contract_mismatch', 'build_failed', 'build_contract_failed',
    'process_start', 'process_timeout', 'suite_timeout', 'output_limit', 'cleanup_failed'
)
foreach ($test in $result.Tests) {
    foreach ($record in $test.ErrorRecord) {
        if ($record.Exception.Message -cin $safeReasons) { $reason = $record.Exception.Message }
    }
}
$report = @{
    schema = 1; suite = 'live-api'; live_request_budget = 5
    status = $(if ($passed) { 'passed' } else { 'failed' }); reason = $reason
    scope = @('public-configuration', 'session-exchange', 'web-library', 'library-ownership')
}
if ($ReportPath) {
    $json = ConvertTo-Json $report -Depth 4
    [IO.File]::WriteAllText($ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($ReportPath),
        "$json`n", [Text.UTF8Encoding]::new($false))
}
if (!$passed) {
    [Console]::Error.WriteLine("Live API suite failed: $reason. Raw diagnostics are suppressed.")
    exit 1
}
'Live API suite passed: public configuration, session exchange, Web catalog, and ownership.'
exit 0
