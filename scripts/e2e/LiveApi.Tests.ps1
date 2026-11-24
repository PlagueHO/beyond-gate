param([bool]$ApproveLive = $false, [bool]$UseEnvironment = $false)

Describe 'Live public and authenticated API integration' {
    It 'Validates public configuration, exchange, Web catalog, and ownership' -Skip:(!$ApproveLive) {
        Import-Module (Join-Path $PSScriptRoot 'live-api.psm1') -DisableNameChecking
        $executable = New-LiveApiExecutable
        $outcome = Invoke-LiveApiProcess $executable -ApproveLive -UseEnvironment:$UseEnvironment
        if ($outcome.status -cne 'passed') { throw [InvalidOperationException]::new($outcome.reason) }
        $outcome.reason | Should -BeExactly 'passed'
    }
}
