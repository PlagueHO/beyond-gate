BeforeAll {
    Import-Module (Join-Path $PSScriptRoot 'live-api.psm1') -DisableNameChecking
    Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking
    $pwsh = Join-Path $PSHOME 'pwsh'
}

Describe 'Live API safe result mapping' -Tag 'Harness' {
    It 'returns only safe results after successful real-process framing' {
        $outcome = ConvertTo-LiveApiOutcome @{
            Code = 0; Out = "running 1 test`ntest public_and_authenticated_library ... beyond_gate_live_api=passed`ntest result: ok`n"
            Err = 'synthetic-cookie-canary synthetic-bearer-canary https://example.invalid/private'
        }
        $outcome.status | Should -BeExactly 'passed'
        $outcome.reason | Should -BeExactly 'passed'
        ($outcome.Keys | Sort-Object) -join ',' | Should -BeExactly 'reason,status'
        ($outcome | ConvertTo-Json) | Should -Not -Match 'canary|example.invalid'
    }
    It 'preserves the fixed <_> failure without raw diagnostics' -ForEach @(
        'session_rejected', 'session_expired', 'public_request_failed', 'private_request_failed',
        'contract_mismatch', 'timed_out', 'access_denied'
    ) {
        $outcome = ConvertTo-LiveApiOutcome @{ Code = 101; Out = "beyond_gate_live_api=$_`n"; Err = 'private-canary' }
        $outcome.status | Should -BeExactly 'failed'
        $outcome.reason | Should -BeExactly $_
    }
    It 'rejects missing, duplicate, unknown, or inconsistent success markers' -ForEach @(
        @{ Code = 0; Out = '' }
        @{ Code = 0; Out = "beyond_gate_live_api=passed`nbeyond_gate_live_api=passed`n" }
        @{ Code = 0; Out = "beyond_gate_live_api=private_canary`n" }
        @{ Code = 101; Out = "beyond_gate_live_api=passed`n" }
        @{ Code = 0; Out = "beyond_gate_live_api=session_rejected`n" }
    ) {
        $outcome = ConvertTo-LiveApiOutcome @{ Code = $Code; Out = $Out; Err = 'private-canary' }
        $outcome.status | Should -BeExactly 'failed'
        $outcome.reason | Should -BeExactly 'harness_error'
    }
}

Context 'Live API credential and process guards' -Tag 'Harness' {
    It 'requires approval before reading credentials or starting a child' {
        InModuleScope live-api {
            Mock Get-LiveApiSecret { throw 'unexpected credential access' }
            Mock Invoke-E2eProcess { throw 'unexpected process' }
            { Invoke-LiveApiProcess 'synthetic-binary' } | Should -Throw 'approval_required'
            Should -Invoke Get-LiveApiSecret -Times 0 -Exactly
            Should -Invoke Invoke-E2eProcess -Times 0 -Exactly
        }

    }
    It 'rejects <Label> input before process startup' -ForEach @(
        @{ Label = 'empty'; Value = '' }
        @{ Label = 'cookie-list'; Value = 'synthetic;invalid' }
        @{ Label = 'multiline'; Value = "synthetic`ninvalid" }
        @{ Label = 'oversized'; Value = ('a' * 2553) }
        @{ Label = 'cookie assignment'; Value = 'CobaltSession=synthetic' }
        @{ Label = 'cookie header'; Value = 'cookie:synthetic' }
    ) {
        if (!$IsWindows -and !$IsLinux) {
            Set-ItResult -Skipped -Because 'Live reads are unsupported on this platform.'
            return
        }
        InModuleScope live-api -Parameters @{ SyntheticValue = $Value } {
            Mock Get-LiveApiSecret { $SyntheticValue }
            Mock Invoke-E2eProcess { throw 'unexpected process' }
            { Invoke-LiveApiProcess 'synthetic-binary' -ApproveLive } | Should -Throw 'input_*'
            Should -Invoke Invoke-E2eProcess -Times 0 -Exactly
        }
    }
    It 'accepts the product cookie size boundary before starting the child' -Skip:(!$IsWindows -and !$IsLinux) {
        InModuleScope live-api {
            Mock Get-LiveApiSecret { 'a' * 2552 }
            Mock Invoke-E2eProcess { @{ Code = 0; Out = "beyond_gate_live_api=passed`n"; Err = '' } }
            (Invoke-LiveApiProcess 'synthetic-binary' -ApproveLive).status | Should -BeExactly 'passed'
            Should -Invoke Invoke-E2eProcess -Times 1 -Exactly
        }
    }
    It 'limits the child scope and clears launcher-owned credential references' -Skip:(!$IsWindows -and !$IsLinux) {
        InModuleScope live-api {
            Mock Get-LiveApiSecret { 'synthetic-cookie-canary' }
            Mock Invoke-E2eProcess {
                param($Executable, $Arguments, $Environment, $TimeoutSeconds, $MaxBytes)
                $script:childEnvironment = $Environment
                $Executable | Should -BeExactly 'synthetic-binary'
                $Arguments -join ' ' | Should -BeExactly '--ignored --exact public_and_authenticated_library --nocapture --test-threads=1'
                $Arguments -join ' ' | Should -Not -Match 'canary'
                $Environment.BEYOND_GATE_COBALT_SESSION | Should -BeExactly 'synthetic-cookie-canary'
                $Environment.BEYOND_GATE_LIVE_API_APPROVED |
                    Should -BeExactly 'public-and-authenticated-library-five-attempts'
                $TimeoutSeconds | Should -Be 40
                $MaxBytes | Should -Be 16384
                @{ Code = 0; Out = "beyond_gate_live_api=passed`n"; Err = 'private-canary' }
            }
            $outcome = Invoke-LiveApiProcess 'synthetic-binary' -ApproveLive
            $outcome.status | Should -BeExactly 'passed'
            $script:childEnvironment.Count | Should -Be 0
        }
    }
    It 'clears credential references after a subprocess timeout' -Skip:(!$IsWindows -and !$IsLinux) {
        InModuleScope live-api {
            Mock Get-LiveApiSecret { 'synthetic-cookie-canary' }
            Mock Invoke-E2eProcess {
                param($Environment)
                $script:childEnvironment = $Environment
                throw [InvalidOperationException]::new('process_timeout')
            }
            { Invoke-LiveApiProcess 'synthetic-binary' -ApproveLive } | Should -Throw 'process_timeout'
            $script:childEnvironment.Count | Should -Be 0
        }
    }
    It 'fails the command without approval even with invented inherited input' {
        $run = Invoke-E2eProcess $pwsh @(
            '-NoProfile', '-File', (Join-Path (Split-Path $PSScriptRoot -Parent) 'test-live-api.ps1')
        ) -Environment @{ BEYOND_GATE_COBALT_SESSION = 'synthetic-cookie-canary' }
        $run.Code | Should -Be 1
        $run.Err | Should -Match 'require -ApproveLive'
        "$($run.Out)$($run.Err)" | Should -Not -Match 'canary'
    }
}

Context 'Live API build artifact guards' -Tag 'Harness' {
    It 'selects only the exact integration test artifact' {
        InModuleScope live-api {
            Mock Invoke-E2eProcess {
                $artifact = @{
                    reason = 'compiler-artifact'; target = @{ name = 'live_api'; kind = @('test') }
                    executable = (Join-Path $PSHOME $(if ($IsWindows) { 'pwsh.exe' } else { 'pwsh' }))
                } | ConvertTo-Json -Compress -Depth 4
                @{ Code = 0; Out = "$artifact`n"; Err = 'synthetic-build-canary' }
            }
            New-LiveApiExecutable | Should -Exist
        }
    }
    It 'stops build errors before credential input' {
        InModuleScope live-api {
            Mock Invoke-E2eProcess { @{ Code = 1; Out = ''; Err = 'synthetic-build-canary' } }
            { New-LiveApiExecutable } | Should -Throw 'build_failed'
        }
    }
    It 'rejects malformed or missing build evidence' -ForEach @(
        @{ Out = '{synthetic-build-canary' }
        @{ Out = '{"reason":"build-finished","success":true}' }
    ) {
        InModuleScope live-api -Parameters @{ SyntheticOutput = $Out } {
            Mock Invoke-E2eProcess { @{ Code = 0; Out = $SyntheticOutput; Err = '' } }
            { New-LiveApiExecutable } | Should -Throw 'build_contract_failed'
        }
    }
}
