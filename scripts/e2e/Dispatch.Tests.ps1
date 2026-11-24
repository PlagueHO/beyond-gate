BeforeDiscovery {
    $scenarios = @('success', 'success-again', 'list', 'slot', 'capacity', 'upload', 'dispatch', 'run', 'lookup', 'queue', 'cancel', 'cancel-throw', 'cleanup', 'cleanup-throw')
}
BeforeAll {
    Import-Module (Join-Path $PSScriptRoot 'dispatch.psm1') -Force
    $seenNames = [Collections.Generic.HashSet[string]]::new()
}

Describe 'Synthetic submission lifecycle' -Tag 'Harness' {
    BeforeEach {
        $state = @{ Scenario = $_; Calls = [Collections.Generic.List[hashtable]]::new(); Submission = '' }
        $fakeGh = {
            param([string[]]$Arguments, [string]$InputText)
            $state.Calls.Add(@{ Arguments = $Arguments; Input = $InputText })
            $operation = "$($Arguments[0]) $($Arguments[1])"
            $response = @{ Code = 0; Out = ''; Err = '' }
            switch ($operation) {
                'secret list' {
                    $response.Out = '[]'
                    if ($state.Scenario -eq 'list') { $response.Code = 1 }
                    if ($state.Scenario -eq 'slot') {
                        $response.Out = '[{"name":"BEYOND_GATE_E2E_0123456789ABCDEF0123456789ABCDEF"}]'
                    }
                    if ($state.Scenario -eq 'capacity') {
                        $response.Out = ConvertTo-Json -Compress @(1..100 | ForEach-Object { @{ name = "SYNTHETIC_$_" } })
                    }
                }
                'secret set' {
                    if ($state.Scenario -eq 'upload') { $response.Code = 1; $response.Err = 'cookie-canary' }
                }
                'workflow run' {
                    $field = $Arguments | Where-Object { $_.StartsWith('submission_id=') }
                    $state.Submission = $field.Substring('submission_id='.Length)
                    if ($state.Scenario -eq 'dispatch') { $response.Code = 1; $response.Err = 'private-id' }
                }
                'run list' {
                    if ($state.Scenario -eq 'lookup') { $response.Code = 1 }
                    else {
                        $conclusion = if ($state.Scenario -eq 'run') { 'failure' } else { 'success' }
                        $status = if ($state.Scenario -in @('queue', 'cancel', 'cancel-throw')) { 'queued' } else { 'completed' }
                        $response.Out = ConvertTo-Json -Compress @(@{
                            databaseId = 123; displayTitle = "live-e2e-$($state.Submission)"
                            status = $status; conclusion = $conclusion
                        })
                    }
                }
                'run cancel' {
                    if ($state.Scenario -eq 'cancel') { $response.Code = 1 }
                    if ($state.Scenario -eq 'cancel-throw') { throw 'cookie-canary' }
                }
                'secret delete' {
                    if ($state.Scenario -eq 'cleanup') { $response.Code = 1 }
                    if ($state.Scenario -eq 'cleanup-throw') { throw 'private-url-canary' }
                }
                default { throw 'Unexpected synthetic gh operation.' }
            }
            return $response
        }.GetNewClosure()
        $secret = ConvertTo-SecureString 'synthetic-cookie-canary' -AsPlainText -Force
        $options = @{ Cookie = $secret; InvokeGh = $fakeGh; QueueTimeoutSeconds = 1; TimeoutSeconds = 10 }
        if ($state.Scenario -eq 'slot') { $options.SubmissionId = '0123456789abcdef0123456789abcdef' }
    }
    AfterEach { $secret.Dispose() }
    It 'handles <_>' -ForEach $scenarios {
        $scenario = $_
        if ($scenario -in @('success', 'success-again')) {
            $result = Invoke-E2eSecretSubmission @options
            $result.run_id | Should -Be 123
        } else {
            $expected = switch ($scenario) {
                'list' { '^secret_list_failed$' }
                'slot' { '^secret_slot_exists$' }
                'capacity' { '^secret_capacity$' }
                'upload' { '^secret_upload_failed$' }
                'dispatch' { '^dispatch_failed$' }
                'run' { '^run_failed$' }
                'lookup' { '^run_lookup_failed$' }
                'cleanup' { '^secret_cleanup_failed:BEYOND_GATE_E2E_[A-F0-9]{32}$' }
                'queue' { '^queue_timeout$' }
                'cancel' { '^run_cancel_failed$' }
                'cancel-throw' { '^run_cancel_failed$' }
                'cleanup-throw' { '^secret_cleanup_failed:BEYOND_GATE_E2E_[A-F0-9]{32}$' }
            }
            $caught = $null
            try { Invoke-E2eSecretSubmission @options | Out-Null }
            catch { $caught = $_ }
            $caught | Should -Not -BeNullOrEmpty
            $caught.Exception.Message | Should -Match $expected
            $caught.Exception.Message.Contains('canary') | Should -BeFalse
        }
        $sets = @($state.Calls | Where-Object { $_.Arguments[0] -eq 'secret' -and $_.Arguments[1] -eq 'set' })
        $deletes = @($state.Calls | Where-Object { $_.Arguments[0] -eq 'secret' -and $_.Arguments[1] -eq 'delete' })
        if ($scenario -in @('list', 'slot', 'capacity')) {
            $sets.Count | Should -Be 0
            $deletes.Count | Should -Be 0
        } else {
            $sets.Count | Should -Be 1
            $deletes.Count | Should -Be 1
            $sets[0].Arguments[2] | Should -BeExactly $deletes[0].Arguments[2]
            $sets[0].Input | Should -BeExactly 'synthetic-cookie-canary'
            $seenNames.Add($sets[0].Arguments[2]) | Should -BeTrue
        }
        ($state.Calls.Arguments -join ' ') | Should -Not -Match 'synthetic-cookie-canary|--body'
        @($state.Calls | Where-Object { $_.Arguments[0] -eq 'workflow' }).Count | Should -BeLessOrEqual 1
        if ($scenario -in @('queue', 'cancel', 'cancel-throw')) {
            @($state.Calls | Where-Object { $_.Arguments[0] -eq 'run' -and $_.Arguments[1] -eq 'cancel' }).Count | Should -Be 1
        }
        if ($scenario -eq 'upload') {
            @($state.Calls | Where-Object { $_.Arguments[0] -eq 'workflow' }).Count | Should -Be 0
        }
    }
}
