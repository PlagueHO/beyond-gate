# Pester is development tooling. Never publish its raw failure records.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking

function New-E2ePesterConfiguration {
    param([string[]]$Path, [hashtable]$Data = @{})
    Import-Module Pester -RequiredVersion 6.2.0 -DisableNameChecking -ErrorAction Stop
    $configuration = New-PesterConfiguration
    $configuration.Run.Container = @(foreach ($file in $Path) {
        New-PesterContainer -Path $file -Data $Data
    })
    $configuration.Run.PassThru = $true
    $configuration.Run.Exit = $false
    $configuration.Output.Verbosity = 'None'
    $configuration.Output.StackTraceVerbosity = 'None'
    $configuration.Output.CIFormat = 'None'
    $configuration.Output.CIDebugOutput = 'None'
    $configuration.TestResult.Enabled = $false
    $configuration.CodeCoverage.Enabled = $false
    $configuration.Should.ErrorAction = 'Stop'
    return $configuration
}

function ConvertTo-E2ePesterReport {
    param($Result, [string]$Selection, [bool]$ApproveNativeStorage)
    $cases = Get-E2eCases (Get-E2eInventory)
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $cleanupCases = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($test in $Result.Tests) {
        $id = $test.ExpandedName
        Assert-E2e ($seen.Add($id)) 'inventory_duplicate'
        $matches = @($cases | Where-Object id -CEQ $id)
        Assert-E2e ($matches.Count -eq 1) 'inventory_invalid'
        $case = $matches[0]
        if ($test.Result -eq 'Passed') {
            Assert-E2e ($case.class -ne 'live') 'live_prerequisites'
            if ($case.class -eq 'native') {
                Assert-E2e ($Selection -eq 'full' -and $ApproveNativeStorage -and $IsWindows) 'isolation_required'
            }
            Set-E2eResult $case 'passed' 'verified'
        } elseif ($test.Result -eq 'Skipped' -and $case.class -eq 'live') {
            Set-E2eResult $case 'blocked' 'live_prerequisites'
        } elseif ($test.Result -eq 'Skipped' -and $case.class -eq 'native') {
            if ($Selection -eq 'contracts') { Set-E2eResult $case 'not_applicable' 'not_selected' }
            elseif (!$ApproveNativeStorage) { Set-E2eResult $case 'blocked' 'native_approval_required' }
            elseif (!$IsWindows) { Set-E2eResult $case 'blocked' 'native_platform' }
            else { Set-E2eResult $case 'failed' 'untested' }
        } else {
            $reason = 'untested'
            if ($test.Result -eq 'Failed') {
                $reason = 'harness_error'
                foreach ($record in $test.ErrorRecord) {
                    if ($record.FullyQualifiedErrorId -eq 'PesterAssertionFailed') { $reason = 'contract_mismatch' }
                    $candidate = Get-E2eReason $record
                    if ($candidate -ne 'harness_error') { $reason = $candidate }
                    if ($record.Exception.Data.Contains('E2eCleanup')) {
                        foreach ($kind in $record.Exception.Data['E2eCleanup']) {
                            Assert-E2e ($kind -cin @('native', 'file')) 'result_invalid'
                            $null = $cleanupCases.Add("suite.$kind-cleanup")
                        }
                    }
                }
            }
            Set-E2eResult $case 'failed' $reason
        }
    }
    foreach ($id in $cleanupCases) {
        $cases.Add((New-E2eCase $id 'infrastructure'))
        Set-E2eResult $cases[-1] 'failed' 'cleanup_failed'
    }
    if ($seen.Count -ne $cases.Count - $cleanupCases.Count -or $Result.FailedContainersCount -gt 0 -or $Result.FailedBlocksCount -gt 0) {
        $cases.Add((New-E2eCase 'suite.infrastructure' 'infrastructure'))
        Set-E2eResult $cases[-1] 'failed' 'harness_error'
    }
    return Get-E2eReport $cases $Selection
}

Export-ModuleMember -Function New-E2ePesterConfiguration, ConvertTo-E2ePesterReport
