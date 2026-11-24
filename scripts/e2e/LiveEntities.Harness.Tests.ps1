BeforeAll {
    Import-Module (Join-Path $PSScriptRoot 'live-api.psm1') -DisableNameChecking
    Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking
    $pwsh = Join-Path $PSHOME 'pwsh'
    $root = Split-Path $PSScriptRoot -Parent
}

Describe 'Entity protocol and stop guards' -Tag 'Harness' {
    It 'accepts all fixed cases with the reviewed budget and no raw diagnostics' {
        $scope = @(Get-LiveEntityScope)
        $scope.Count | Should -Be 24
        $lines = @('test public_and_authenticated_entities ... beyond_gate_live_entities_budget=74')
        $lines += $scope | ForEach-Object { "beyond_gate_live_entity=$($_.id):passed:verified" }
        $rows = @(ConvertTo-LiveEntityOutcome @{ Code = 0; Out = $lines -join "`n"; Err = 'private-canary' })
        @($rows | Where-Object status -CNE 'passed').Count | Should -Be 0
        ($rows | ConvertTo-Json) | Should -Not -Match 'canary'
    }
    It 'keeps a single selected family and a stopped detail failure' {
        $out = "beyond_gate_live_entities_budget=5`nbeyond_gate_live_entity=character.list:passed:verified`nbeyond_gate_live_entity=character.detail:failed:access_denied"
        $rows = @(ConvertTo-LiveEntityOutcome @{ Code = 101; Out = $out; Err = 'private-canary' } character)
        $rows.Count | Should -Be 2
        $rows[0].status | Should -BeExactly 'passed'
        $rows[1].reason | Should -BeExactly 'access_denied'
    }
    It 'rejects <Label> evidence without publishing supplied data' -ForEach @(
        @{ Label = 'missing'; Budget = '5'; List = ''; Detail = ''; Code = 0 }
        @{ Label = 'wrong budget'; Budget = '74'; List = 'character.list:passed:verified'; Detail = 'character.detail:passed:verified'; Code = 0 }
        @{ Label = 'duplicate'; Budget = '5'; List = 'character.list:passed:verified'; Detail = 'character.list:passed:verified'; Code = 0 }
        @{ Label = 'unknown case'; Budget = '5'; List = 'private_canary.list:passed:verified'; Detail = 'character.detail:passed:verified'; Code = 0 }
        @{ Label = 'reordered'; Budget = '5'; List = 'character.detail:passed:verified'; Detail = 'character.list:passed:verified'; Code = 0 }
        @{ Label = 'success after failure'; Budget = '5'; List = 'character.list:failed:read_failed'; Detail = 'character.detail:passed:verified'; Code = 101 }
        @{ Label = 'initial block'; Budget = '5'; List = 'character.list:blocked:stopped_after_failure'; Detail = 'character.detail:blocked:stopped_after_failure'; Code = 101 }
        @{ Label = 'unknown reason'; Budget = '5'; List = 'character.list:failed:private_canary'; Detail = 'character.detail:blocked:stopped_after_failure'; Code = 101 }
        @{ Label = 'exit disagreement'; Budget = '5'; List = 'character.list:passed:verified'; Detail = 'character.detail:passed:verified'; Code = 101 }
    ) {
        $out = "beyond_gate_live_entities_budget=$Budget`nbeyond_gate_live_entity=$List`nbeyond_gate_live_entity=$Detail"
        $rows = @(ConvertTo-LiveEntityOutcome @{ Code = $Code; Out = $out; Err = 'private-canary' } character)
        $rows.Count | Should -Be 2
        @($rows | Where-Object reason -CNE 'harness_error').Count | Should -Be 0
        ($rows | ConvertTo-Json) | Should -Not -Match 'canary'
    }
}

Context 'Entity launcher input and cleanup guards' -Tag 'Harness' {
    It 'requires approval before build, selection preflight, or input' {
        InModuleScope live-api {
            Mock New-LiveApiExecutable { throw 'unexpected build' }
            Mock Get-LiveApiSecret { throw 'unexpected input' }
            Mock Invoke-E2eProcess { throw 'unexpected child' }
            { Invoke-LiveEntitiesProcess '{}' } | Should -Throw 'approval_required'
            Should -Invoke New-LiveApiExecutable -Times 0 -Exactly
            Should -Invoke Get-LiveApiSecret -Times 0 -Exactly
            Should -Invoke Invoke-E2eProcess -Times 0 -Exactly
        }
    }
    It 'rejects <Label> selection evidence before input' -ForEach @(
        @{ Label = 'invalid'; Code = 101; Out = '' }
        @{ Label = 'missing'; Code = 0; Out = '' }
        @{ Label = 'duplicate'; Code = 0; Out = "beyond_gate_live_entities_selection=valid`nbeyond_gate_live_entities_selection=valid" }
    ) -Skip:(!$IsWindows -and !$IsLinux) {
        InModuleScope live-api -Parameters @{ Code = $Code; Out = $Out } {
            Mock New-LiveApiExecutable { 'synthetic-binary' }
            Mock Get-LiveApiSecret { throw 'unexpected input' }
            Mock Invoke-E2eProcess { @{ Code = $Code; Out = $Out; Err = 'private-canary' } }
            { Invoke-LiveEntitiesProcess '{}' -ApproveLive } | Should -Throw 'selection_invalid'
            Should -Invoke Get-LiveApiSecret -Times 0 -Exactly
            Should -Invoke Invoke-E2eProcess -Times 1 -Exactly
        }
    }
    It 'bounds UTF-8 selection input before build' -Skip:(!$IsWindows -and !$IsLinux) {
        InModuleScope live-api {
            Mock New-LiveApiExecutable { throw 'unexpected build' }
            { Invoke-LiveEntitiesProcess ('a' * 16385) -ApproveLive } | Should -Throw 'selection_invalid'
            Should -Invoke New-LiveApiExecutable -Times 0 -Exactly
        }
    }
    It 'rejects oversized or copied cookie assignments before a live child' -ForEach @(
        @{ Value = ('a' * 2553) }
        @{ Value = 'CobaltSession=synthetic' }
    ) -Skip:(!$IsWindows -and !$IsLinux) {
        InModuleScope live-api -Parameters @{ SyntheticValue = $Value } {
            Mock New-LiveApiExecutable { 'synthetic-binary' }
            Mock Get-LiveApiSecret { $SyntheticValue }
            Mock Invoke-E2eProcess {
                @{ Code = 0; Out = 'test validate_entity_selections ... beyond_gate_live_entities_selection=valid'; Err = '' }
            }
            { Invoke-LiveEntitiesProcess '{}' character -ApproveLive } | Should -Throw 'input_invalid'
            Should -Invoke Invoke-E2eProcess -Times 1 -Exactly
        }
    }
    It 'clears references after <Ending> and never sends a credential to preflight' -ForEach @(
        @{ Ending = 'success' }
        @{ Ending = 'timeout' }
    ) -Skip:(!$IsWindows -and !$IsLinux) {
        InModuleScope live-api -Parameters @{ Ending = $Ending } {
            Mock New-LiveApiExecutable { 'synthetic-binary' }
            Mock Get-LiveApiSecret { 'synthetic-cookie-canary' }
            Mock Invoke-E2eProcess {
                param($Arguments, $Environment, $TimeoutSeconds, $Deadline, $MaxBytes)
                $script:entityEnvironment = $Environment
                $Arguments -join ' ' | Should -Not -Match 'canary'
                if ($Arguments -contains 'validate_entity_selections') {
                    $Environment.ContainsKey('BEYOND_GATE_COBALT_SESSION') | Should -BeFalse
                    $Environment.ContainsKey('BEYOND_GATE_LIVE_API_APPROVED') | Should -BeFalse
                    return @{ Code = 0; Out = 'test validate_entity_selections ... beyond_gate_live_entities_selection=valid'; Err = '' }
                }
                $Arguments -join ' ' | Should -BeExactly '--ignored --exact public_and_authenticated_entities --nocapture --test-threads=1'
                $Environment.BEYOND_GATE_COBALT_SESSION | Should -BeExactly 'synthetic-cookie-canary'
                $Environment.BEYOND_GATE_LIVE_API_APPROVED | Should -BeExactly 'selected-entity-readers-74-attempts'
                $TimeoutSeconds | Should -Be 610
                $MaxBytes | Should -Be 16384
                ($Deadline - [datetime]::UtcNow).TotalSeconds | Should -BeLessOrEqual 620
                if ($Ending -ceq 'timeout') { throw [InvalidOperationException]::new('process_timeout') }
                @{ Code = 0; Out = "beyond_gate_live_entities_budget=5`nbeyond_gate_live_entity=character.list:passed:verified`nbeyond_gate_live_entity=character.detail:passed:verified"; Err = 'private-canary' }
            }
            if ($Ending -ceq 'timeout') {
                { Invoke-LiveEntitiesProcess '{}' character -ApproveLive } | Should -Throw 'process_timeout'
            } else {
                $rows = @(Invoke-LiveEntitiesProcess '{}' character -ApproveLive)
                @($rows | Where-Object status -CNE 'passed').Count | Should -Be 0
            }
            $script:entityEnvironment.Count | Should -Be 0
            Should -Invoke Get-LiveApiSecret -Times 1 -Exactly
            Should -Invoke Invoke-E2eProcess -Times 2 -Exactly
        }
    }
    It 'refuses unapproved command execution without selection access' {
        $run = Invoke-E2eProcess $pwsh @('-NoProfile', '-File', (Join-Path $root 'test-live-entities.ps1'),
            '-SelectionFile', 'missing-synthetic-selection.json')
        $run.Code | Should -Be 1
        $run.Err | Should -Match 'require -ApproveLive'
    }
}

Context 'Entity Pester single-run guards' -Tag 'Harness' {
    It 'invokes the full selected scope exactly once for <Ending>' -ForEach @(
        @{ Ending = 'success'; ExpectedPassed = 24; ExpectedFailed = 0; ExpectedSkipped = 0 }
        @{ Ending = 'failure'; ExpectedPassed = 0; ExpectedFailed = 24; ExpectedSkipped = 0 }
        @{ Ending = 'stopped'; ExpectedPassed = 1; ExpectedFailed = 1; ExpectedSkipped = 22 }
    ) {
        $suite = (Join-Path $PSScriptRoot 'LiveEntities.Tests.ps1').Replace("'", "''")
        $module = (Join-Path $PSScriptRoot 'pester.psm1').Replace("'", "''")
        $shim = Join-Path $TestDrive "entity-$Ending.Tests.ps1"
        $body = @"
. '$suite' -ApproveLive `$true
BeforeAll {
    `$global:syntheticEntityFixture = @{ Calls = 0; Rows = @(foreach (`$case in (Get-LiveEntityScope)) {
        @{ id = `$case.id; status = 'passed'; reason = 'verified' }
    }) }
    if ('$Ending' -ceq 'stopped') {
        `$global:syntheticEntityFixture.Rows[1].status = 'failed'
        `$global:syntheticEntityFixture.Rows[1].reason = 'access_denied'
        foreach (`$row in `$global:syntheticEntityFixture.Rows[2..23]) {
            `$row.status = 'blocked'; `$row.reason = 'stopped_after_failure'
        }
    }
    Mock Invoke-LiveEntitiesProcess {
        `$global:syntheticEntityFixture.Calls++
        if ('$Ending' -ceq 'failure') { throw [InvalidOperationException]::new('private-canary') }
        `$global:syntheticEntityFixture.Rows.Count | Should -Be 24
        `$global:syntheticEntityFixture.Rows
    }
}
AfterAll {
    `$global:syntheticEntityFixture.Calls | Should -Be 1
    `$global:syntheticEntityFixture.Rows.Count | Should -Be 24
}
"@
        [IO.File]::WriteAllText($shim, $body)
        $escapedShim = $shim.Replace("'", "''")
        $code = @"
Import-Module '$module' -DisableNameChecking
`$r = Invoke-Pester -Configuration (New-E2ePesterConfiguration '$escapedShim')
@{ passed = `$r.PassedCount; failed = `$r.FailedCount; skipped = `$r.SkippedCount; blocks = @(`$r.FailedBlocks).Count } | ConvertTo-Json -Compress
"@
        $run = Invoke-E2eProcess $pwsh @('-NoProfile', '-EncodedCommand',
            [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($code))) -TimeoutSeconds 60
        $run.Code | Should -Be 0
        "$($run.Out)$($run.Err)" | Should -Not -Match 'canary'
        $counts = ConvertFrom-Json $run.Out
        $counts.passed | Should -Be $ExpectedPassed
        $counts.failed | Should -Be $ExpectedFailed
        $counts.skipped | Should -Be $ExpectedSkipped
        $counts.blocks | Should -Be 0
    }
}
