BeforeAll {
    Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking
    Import-Module (Join-Path $PSScriptRoot 'pester.psm1') -DisableNameChecking
    $pwsh = (Get-Command pwsh -CommandType Application | Select-Object -First 1).Source

    function New-SyntheticPesterResult {
        $cases = Get-E2eCases (Get-E2eInventory)
        $tests = @($cases | ForEach-Object {
            [pscustomobject]@{
                ExpandedName = $_.id
                Result = $(if ($_.class -in @('live', 'native')) { 'Skipped' } else { 'Passed' })
                ErrorRecord = @()
            }
        })
        return [pscustomobject]@{ Tests = $tests; FailedContainersCount = 0; FailedBlocksCount = 0 }
    }
}
Describe 'Pester result mapping' -Tag 'Harness' {
    BeforeEach { $result = New-SyntheticPesterResult }
    It 'preserves the exact safe schema and contracts counts' {
        $report = ConvertTo-E2ePesterReport $result 'contracts' $false
        ($report.Keys | Sort-Object) -join ',' |
            Should -BeExactly 'cases,counts,exit_code,full_coverage,live_request_budget,schema,selection'
        $report.exit_code | Should -Be 0
        $report.full_coverage | Should -BeFalse
        $report.live_request_budget | Should -Be 0
        $report.counts.passed | Should -Be 224
        $report.counts.blocked | Should -Be 130
        $report.counts.not_applicable | Should -Be 4
        $report.counts.failed | Should -Be 0
        foreach ($row in $report.cases) {
            ($row.Keys | Sort-Object) -join ',' | Should -BeExactly 'case,reason,status'
        }
    }
    It 'maps synthetic full results to a nonzero incomplete report' {
        $report = ConvertTo-E2ePesterReport $result 'full' $false
        $report.exit_code | Should -Be 1
        $report.full_coverage | Should -BeFalse
        $report.counts.blocked | Should -Be 134
        @($report.cases | Where-Object reason -EQ 'native_approval_required').Count | Should -Be 4
    }
    It 'does not count unexpected <_> as coverage' -ForEach @('Skipped', 'Inconclusive', 'NotRun') {
        $result.Tests[0].Result = $_
        $report = ConvertTo-E2ePesterReport $result 'contracts' $false
        $report.exit_code | Should -Be 1
        $report.cases[0].status | Should -BeExactly 'failed'
        $report.cases[0].reason | Should -BeExactly 'untested'
    }
    It 'maps typed failures without publishing raw diagnostics' {
        $result.Tests[0].Result = 'Failed'
        try { throw [InvalidOperationException]::new('process_timeout') }
        catch { $result.Tests[0].ErrorRecord = @($_) }
        $report = ConvertTo-E2ePesterReport $result 'contracts' $false
        $report.cases[0].reason | Should -BeExactly 'process_timeout'
        try { throw 'cookie-canary bearer-canary private-id https://example.invalid/private' }
        catch { $result.Tests[0].ErrorRecord = @($_) }
        $report = ConvertTo-E2ePesterReport $result 'contracts' $false
        ($report | ConvertTo-Json -Depth 10) | Should -Not -Match 'canary|private-id|example.invalid'
        $report.cases[0].status | Should -BeExactly 'failed'
        $report.cases[0].reason | Should -BeExactly 'harness_error'
    }
    It 'retains the fixed <Kind> cleanup infrastructure case' -ForEach @(
        @{ Kind = 'native' }
        @{ Kind = 'file' }
    ) {
        $result.Tests[0].Result = 'Failed'
        try {
            $failure = [InvalidOperationException]::new('cleanup_failed')
            $failure.Data['E2eCleanup'] = $Kind
            throw $failure
        } catch { $result.Tests[0].ErrorRecord = @($_) }
        $report = ConvertTo-E2ePesterReport $result 'contracts' $false
        $report.exit_code | Should -Be 1
        @($report.cases | Where-Object case -EQ "suite.$Kind-cleanup").Count | Should -Be 1
        $report.cases[0].reason | Should -BeExactly 'cleanup_failed'
    }
    It 'retains both cleanup failures from one context' {
        $result.Tests[0].Result = 'Failed'
        try {
            $failure = [InvalidOperationException]::new('cleanup_failed')
            $failure.Data['E2eCleanup'] = @('native', 'file')
            throw $failure
        } catch { $result.Tests[0].ErrorRecord = @($_) }
        $report = ConvertTo-E2ePesterReport $result 'contracts' $false
        @($report.cases | Where-Object case -IN @('suite.native-cleanup', 'suite.file-cleanup')).Count | Should -Be 2
        $report.exit_code | Should -Be 1
    }
    It 'fails missing duplicate unknown and live-passed results' {
        $result.Tests = @($result.Tests | Select-Object -Skip 1)
        $report = ConvertTo-E2ePesterReport $result 'contracts' $false
        $report.exit_code | Should -Be 1
        @($report.cases | Where-Object case -EQ 'suite.infrastructure').Count | Should -Be 1
        $result = New-SyntheticPesterResult
        $result.Tests += $result.Tests[0]
        { ConvertTo-E2ePesterReport $result 'contracts' $false } | Should -Throw -ExpectedMessage 'inventory_duplicate'
        $result = New-SyntheticPesterResult
        $result.Tests[0].ExpandedName = 'unknown.case'
        { ConvertTo-E2ePesterReport $result 'contracts' $false } | Should -Throw -ExpectedMessage 'inventory_invalid'
        $result = New-SyntheticPesterResult
        ($result.Tests | Where-Object ExpandedName -LIKE 'live.*' | Select-Object -First 1).Result = 'Passed'
        { ConvertTo-E2ePesterReport $result 'contracts' $false } | Should -Throw -ExpectedMessage 'live_prerequisites'
        $result = New-SyntheticPesterResult
        ($result.Tests | Where-Object ExpandedName -LIKE 'native.*' | Select-Object -First 1).Result = 'Passed'
        { ConvertTo-E2ePesterReport $result 'contracts' $false } | Should -Throw -ExpectedMessage 'isolation_required'
    }

    Context 'Synthetic native isolation guards' {
        BeforeAll {
            Import-Module (Join-Path $PSScriptRoot 'fixtures.psm1') -DisableNameChecking
        }
        BeforeEach {
            $context = @{
                Environment = @{ BEYOND_GATE_E2E_NATIVE_APPROVED = '0' }
                Identity = 'synthetic-isolation-context'; NativeStarted = $false
            }
        }
        It 'rejects a missing isolation handshake before storage access' {
            Mock Invoke-E2eContextProcess -ModuleName fixtures {
                @{ Code = 0; Err = ''; Out = 'not-an-isolated-binary' }
            }
            { Start-E2eNativeContext $context } | Should -Throw -ExpectedMessage 'isolation_required'
            $context.NativeStarted | Should -BeFalse
            $context.Environment.BEYOND_GATE_E2E_NATIVE_APPROVED | Should -BeExactly '0'
            Should -Invoke Invoke-E2eContextProcess -ModuleName fixtures -Exactly 1
        }
        It 'reports both native and file cleanup failures without raw errors' {
            $context.NativeStarted = $true
            $context.Binaries = @{ helper = 'synthetic-helper' }
            $context.Directory = $TestDrive
            Mock Invoke-E2eProcess -ModuleName fixtures { throw 'cookie-canary' }
            Mock Remove-Item -ModuleName fixtures { throw 'private-path-canary' }
            $caught = $null
            try { Close-E2eContext $context }
            catch { $caught = $_ }
            $caught | Should -Not -BeNullOrEmpty
            $caught.Exception.Message | Should -BeExactly 'cleanup_failed'
            ($caught.Exception.Data['E2eCleanup'] | Sort-Object) -join ',' | Should -BeExactly 'file,native'
            Should -Invoke Invoke-E2eProcess -ModuleName fixtures -Exactly 1
            Should -Invoke Remove-Item -ModuleName fixtures -Exactly 1
        }
        It 'checks all three binaries before synthetic cleanup' {
            Mock Invoke-E2eContextProcess -ModuleName fixtures {
                param($Context, $Binary, $Arguments)
                if ($Arguments[0] -eq 'cleanup') {
                    if ($Context.Environment.BEYOND_GATE_E2E_NATIVE_APPROVED -ne '1') { throw 'isolation_required' }
                    return @{ Code = 0; Err = ''; Out = '' }
                }
                if ($Context.Environment.BEYOND_GATE_E2E_NATIVE_APPROVED -ne '0') { throw 'isolation_required' }
                return @{ Code = 0; Err = ''; Out = 'beyond-gate-e2e-isolation-v1' }
            }
            Start-E2eNativeContext $context
            $context.NativeStarted | Should -BeTrue
            Should -Invoke Invoke-E2eContextProcess -ModuleName fixtures -Exactly 3 -ParameterFilter {
                $Arguments[0] -in @('check', '--e2e-context-check')
            }
            Should -Invoke Invoke-E2eContextProcess -ModuleName fixtures -Exactly 1 -ParameterFilter {
                $Arguments[0] -eq 'cleanup'
            }
        }
    }
    It 'fails container and block errors even when test rows pass' {
        $result.FailedContainersCount = 1
        (ConvertTo-E2ePesterReport $result 'contracts' $false).exit_code | Should -Be 1
        $result.FailedContainersCount = 0
        $result.FailedBlocksCount = 1
        (ConvertTo-E2ePesterReport $result 'contracts' $false).exit_code | Should -Be 1
    }
}

Describe 'Safe Pester failure execution' -Tag 'Harness' {
    It 'suppresses canary diagnostics for <Phase> failures' -ForEach @(
        @{ Phase = 'discovery'; Body = "BeforeDiscovery { throw 'cookie-canary' }" }
        @{ Phase = 'setup'; Body = "Describe 'synthetic' { BeforeAll { throw 'cookie-canary' }; It 'cli.version' { } }" }
        @{ Phase = 'test'; Body = "Describe 'synthetic' { It 'cli.version' { 'cookie-canary' | Should -Be 'safe' } }" }
        @{ Phase = 'teardown'; Body = "Describe 'synthetic' { AfterAll { throw 'cookie-canary' }; It 'cli.version' { } }" }
    ) {
        $path = Join-Path $TestDrive 'Failure.Tests.ps1'
        [IO.File]::WriteAllText($path, $Body)
        $module = (Join-Path $PSScriptRoot 'pester.psm1').Replace("'", "''")
        $escapedPath = $path.Replace("'", "''")
        $code = @"
Import-Module Pester -RequiredVersion 6.2.0 -DisableNameChecking
Import-Module '$module' -DisableNameChecking
`$result = Invoke-Pester -Configuration (New-E2ePesterConfiguration @('$escapedPath'))
ConvertTo-E2ePesterReport `$result 'contracts' `$false | ConvertTo-Json -Depth 10 -Compress
"@
        $arguments = @('-NoProfile', '-EncodedCommand', [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($code)))
        $process = Invoke-E2eProcess $pwsh $arguments
        $process.Code | Should -Be 0
        $process.Err | Should -BeExactly ''
        $process.Out | Should -Not -Match 'cookie-canary'
        $report = $process.Out | ConvertFrom-Json -AsHashtable
        $report.exit_code | Should -Be 1
        $report.full_coverage | Should -BeFalse
    }
}
