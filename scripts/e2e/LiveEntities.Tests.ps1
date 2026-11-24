param([string]$SelectionJson = '{}', [string]$Entity = 'all', [bool]$ApproveLive = $false,
    [bool]$UseEnvironment = $false)

BeforeDiscovery {
    Import-Module (Join-Path $PSScriptRoot 'live-api.psm1') -DisableNameChecking
    $cases = @(Get-LiveEntityScope $Entity | ForEach-Object { @{ id = $_.id } })
}
BeforeAll {
    Import-Module (Join-Path $PSScriptRoot 'live-api.psm1') -DisableNameChecking
    $context = @{ Rows = $null }
}
Describe 'Selected live entity contracts' {
    It '<id>' -ForEach $cases -Skip:(!$ApproveLive) {
        if ($null -eq $context.Rows) {
            try {
                $context.Rows = @(Invoke-LiveEntitiesProcess $SelectionJson $Entity -ApproveLive -UseEnvironment:$UseEnvironment)
            } catch {
                $reason = if ($_.Exception.Message -cin @(
                    'approval_required', 'unsupported_platform', 'selection_invalid', 'input_missing',
                    'input_invalid', 'terminal_required', 'build_failed', 'build_contract_failed',
                    'process_start', 'process_timeout', 'suite_timeout', 'output_limit', 'cleanup_failed'
                )) { $_.Exception.Message } else { 'harness_error' }
                $context.Rows = @(New-FailedEntityRows $Entity $reason)
            }
        }
        $row = @($context.Rows | Where-Object id -CEQ $id)
        if ($row.Count -ne 1) { throw [InvalidOperationException]::new('harness_error') }
        if ($row[0].status -ceq 'blocked') {
            Set-ItResult -Skipped -Because 'stopped_after_failure'
        } elseif ($row[0].status -cne 'passed') {
            throw [InvalidOperationException]::new($row[0].reason)
        } else {
            $row[0].reason | Should -BeExactly 'verified'
        }
    }
}
