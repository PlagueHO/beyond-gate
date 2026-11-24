BeforeAll {
    Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking
    Import-Module (Join-Path $PSScriptRoot 'dispatch.psm1') -Force
    $pwsh = (Get-Command pwsh -CommandType Application | Select-Object -First 1).Source

    function Get-ChildArguments {
        param([string]$Code)
        return @('-NoProfile', '-EncodedCommand', [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($Code)))
    }
}

Describe 'Inventory and safe reporting' -Tag 'Harness' {
    BeforeEach {
        $inventory = Get-E2eInventory
        $cases = Get-E2eCases $inventory
    }
    It 'preserves redirected login and read contracts' {
        $login = @($cases | Where-Object id -CEQ 'cli.auth.login-blocker')
        $login.Count | Should -Be 1
        $login[0].data.code | Should -Be 1
        ($login[0].data.arguments -join ' ') | Should -BeExactly 'auth login'
        $login[0].data.error | Should -BeExactly $(if ($IsWindows) {
            'pairing requires interactive stdin, stdout, and stderr'
        } else { 'no working session handoff' })
        $inventory.reads.Where({ $_.tool -eq 'book_list' })[0].requires_session | Should -BeTrue
        $spell = @($cases | Where-Object id -CEQ 'contract.cli.spell.show.json')
        $spell.Count | Should -Be 1
        ($spell[0].data.arguments -join ' ') | Should -BeExactly 'spell show 900001-invented-spell --unexpected'
        $feat = @($cases | Where-Object id -CEQ 'contract.cli.feat.show.json')
        $feat.Count | Should -Be 1
        $feat[0].data.code | Should -Be 2
        $feat[0].data.error | Should -BeExactly 'unexpected'
        ($feat[0].data.arguments -join ' ') | Should -BeExactly 'feat show 17 --unexpected'
        $featList = @($cases | Where-Object id -CEQ 'contract.cli.feat.list.json')
        $featList.Count | Should -Be 1
        $featList[0].data.code | Should -Be 2
        $featList[0].data.arguments | Should -Contain '--unexpected'
        foreach ($case in $cases.Where({ $_.id.StartsWith('contract.cli.book.list.') })) {
            $case.data.code | Should -Be 2
            $case.data.arguments | Should -Contain '--unexpected'
        }
    }
    It 'preserves classifications and full-coverage exit rules' {
        $cases.Count | Should -BeGreaterOrEqual 150
        (Get-E2eReport $cases).full_coverage | Should -BeFalse
        (Get-E2eReport $cases).exit_code | Should -Be 1
        foreach ($case in $cases) {
            if ($case.class -eq 'live') { Set-E2eResult $case 'blocked' 'live_prerequisites' }
            elseif ($case.class -eq 'native') { Set-E2eResult $case 'not_applicable' 'not_selected' }
            else { Set-E2eResult $case 'passed' 'verified' }
        }
        $partial = Get-E2eReport $cases 'contracts'
        $partial.exit_code | Should -Be 0
        $partial.full_coverage | Should -BeFalse
        (($partial.counts.Keys | Sort-Object) -join ',') | Should -BeExactly 'blocked,failed,not_applicable,passed'
        ($partial.counts.Values | Measure-Object -Sum).Sum | Should -Be $cases.Count
        (Get-E2eReport $cases 'full').exit_code | Should -Be 1
        Set-E2eResult $cases[0] 'failed' 'contract_mismatch'
        (Get-E2eReport $cases 'contracts').exit_code | Should -Be 1
    }
    It 'rejects missing duplicate unknown and invalid inventory results' {
        { Get-E2eReport @($cases | Select-Object -Skip 1) } | Should -Throw -ExpectedMessage 'inventory_invalid'
        { Get-E2eReport (@($cases) + @($cases[0])) } | Should -Throw -ExpectedMessage 'inventory_invalid'
        { Get-E2eReport (@($cases) + @(New-E2eCase 'unknown.case' 'cli')) } | Should -Throw -ExpectedMessage 'inventory_invalid'
        { Set-E2eResult $cases[0] 'passed' 'synthetic-cookie-canary' } | Should -Throw -ExpectedMessage 'result_invalid'
        { Set-E2eResult $cases[0] 'success' 'verified' } | Should -Throw -ExpectedMessage 'result_invalid'
        $inventory.clients += $inventory.clients[0]
        { Get-E2eCases $inventory } | Should -Throw -ExpectedMessage 'inventory_duplicate'
        { Assert-E2eLiveReady } | Should -Throw -ExpectedMessage 'live_prerequisites'
    }
    It 'never reports canary data or raw exception messages' {
        $canary = 'cookie-canary bearer-canary account-canary private-id https://example.invalid/private proprietary-canary'
        $cases[0].data['raw'] = $canary
        $report = Get-E2eReport $cases | ConvertTo-Json -Depth 10
        $report.Contains($canary) | Should -BeFalse
        $report.Contains('cookie-canary') | Should -BeFalse
        try { throw [InvalidOperationException]::new($canary) }
        catch { Get-E2eReason $_ | Should -BeExactly 'harness_error' }
    }
}

Describe 'Bounded child processes' -Tag 'Harness' {
    BeforeEach { $child = $null }
    AfterEach { if ($child) { Stop-E2eChild $child } }
    It 'captures separate streams and the real exit code' {
        $result = Invoke-E2eProcess $pwsh (Get-ChildArguments "[Console]::Out.Write('synthetic-out'); [Console]::Error.Write('synthetic-err'); exit 7")
        $result.Code | Should -Be 7
        $result.Out | Should -BeExactly 'synthetic-out'
        $result.Err | Should -BeExactly 'synthetic-err'
    }
    It 'removes inherited product and GitHub credentials from children' {
        $module = (Join-Path $PSScriptRoot 'common.psm1').Replace("'", "''")
        $escapedPwsh = $pwsh.Replace("'", "''")
        $inner = "[Console]::Out.Write([int](`$null -ne `$env:BEYOND_GATE_COBALT_SESSION -or `$null -ne `$env:BEYOND_GATE_LIVE_API_APPROVED -or `$null -ne `$env:BEYOND_GATE_LIVE_ENTITY_SELECTIONS -or `$null -ne `$env:BEYOND_GATE_LIVE_ENTITY -or `$null -ne `$env:GH_TOKEN))"
        $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($inner))
        $code = "Import-Module '$module' -DisableNameChecking; (Invoke-E2eProcess '$escapedPwsh' @('-NoProfile','-EncodedCommand','$encoded')).Out"
        $result = Invoke-E2eProcess $pwsh (Get-ChildArguments $code) -Environment @{
            BEYOND_GATE_COBALT_SESSION = 'synthetic-cookie-canary'
            BEYOND_GATE_LIVE_API_APPROVED = 'synthetic-approval'
            BEYOND_GATE_LIVE_ENTITY_SELECTIONS = 'synthetic-selection'
            BEYOND_GATE_LIVE_ENTITY = 'synthetic-entity'
            GH_TOKEN = 'synthetic-github-canary'
        }
        $result.Code | Should -Be 0
        $result.Err | Should -BeExactly ''
        $result.Out.Trim() | Should -BeExactly '0'
    }
    It 'accepts exact output and input boundaries' {
        $result = Invoke-E2eProcess $pwsh (Get-ChildArguments "[Console]::Out.Write('x' * 128)") -MaxBytes 128
        $result.Code | Should -Be 0
        $result.Out.Length | Should -Be 128
        $result = Invoke-E2eProcess $pwsh (Get-ChildArguments '[Console]::In.ReadToEnd() | Out-Null') -InputText ('x' * 65536)
        $result.Code | Should -Be 0
        $result.Out | Should -BeExactly ''
    }
    It 'rejects excessive <Stream> output' -ForEach @(
        @{ Stream = 'Out'; Canary = 'cookie-canary' }
        @{ Stream = 'Error'; Canary = 'bearer-canary' }
    ) {
        { Invoke-E2eProcess $pwsh (Get-ChildArguments "[Console]::$Stream.Write('$Canary' * 10000)") -MaxBytes 128 } |
            Should -Throw -ExpectedMessage 'output_limit'
    }
    It 'enforces process suite input and start failure limits' {
        { Invoke-E2eProcess $pwsh (Get-ChildArguments 'Start-Sleep -Seconds 30') -TimeoutSeconds 1 } |
            Should -Throw -ExpectedMessage 'process_timeout'
        { Invoke-E2eProcess $pwsh (Get-ChildArguments 'Start-Sleep -Seconds 30') -Deadline ([datetime]::UtcNow.AddSeconds(-1)) } |
            Should -Throw -ExpectedMessage 'suite_timeout'
        { Invoke-E2eProcess $pwsh (Get-ChildArguments 'exit 0') -InputText ('x' * 65537) } |
            Should -Throw -ExpectedMessage 'input_limit'
        { Invoke-E2eProcess (Join-Path $PSScriptRoot 'nonexistent-canary.exe') } |
            Should -Throw -ExpectedMessage 'process_start'
    }
    It 'rejects malformed protocol <Output>' -ForEach @(
        @{ Output = 'private-id cookie-canary' }
        @{ Output = '{"jsonrpc":"1.0","id":1}' }
        @{ Output = '{broken' }
    ) {
        $child = Start-E2eChild $pwsh (Get-ChildArguments "[Console]::Out.WriteLine('$Output')")
        { Invoke-E2eRequest $child @{ jsonrpc = '2.0'; id = 1; method = 'synthetic' } } |
            Should -Throw -ExpectedMessage 'protocol_invalid'
    }
    It 'rejects a missing protocol response' {
        $child = Start-E2eChild $pwsh (Get-ChildArguments 'exit 0')
        { Invoke-E2eRequest $child @{ jsonrpc = '2.0'; id = 1; method = 'synthetic' } } |
            Should -Throw -ExpectedMessage 'protocol_missing'
    }
    It 'terminates its child process' {
        $child = Start-E2eChild $pwsh (Get-ChildArguments 'Start-Sleep -Seconds 30')
        $pidToCheck = $child.Process.Id
        Stop-E2eChild $child
        $child = $null
        Get-Process -Id $pidToCheck -ErrorAction SilentlyContinue | Should -BeNullOrEmpty
    }
    It 'terminates descendants within the cleanup deadline' {
        $descendantCode = @'
$start = [Diagnostics.ProcessStartInfo]::new('__PWSH__')
$start.UseShellExecute = $false
foreach ($argument in @('-NoProfile', '-Command', 'Start-Sleep -Seconds 30')) { $start.ArgumentList.Add($argument) }
$descendant = [Diagnostics.Process]::Start($start)
[Console]::Out.WriteLine($descendant.Id)
Start-Sleep -Seconds 30
'@
        $descendantCode = $descendantCode.Replace('__PWSH__', $pwsh.Replace("'", "''"))
        $child = Start-E2eChild $pwsh (Get-ChildArguments $descendantCode)
        do {
            Receive-E2eBytes $child
            $text = [Text.Encoding]::UTF8.GetString($child.Streams.Out.Data.ToArray())
            Start-Sleep -Milliseconds 5
        } until ($text.Contains("`n"))
        $text.Trim() | Should -Match '^[1-9][0-9]*$'
        $descendantId = [int]$text.Trim()
        Stop-E2eChild $child
        $child = $null
        $descendant = Get-Process -Id $descendantId -ErrorAction SilentlyContinue
        if ($descendant) {
            try { $descendant.WaitForExit(5000) | Should -BeTrue }
            finally {
                if (!$descendant.HasExited) { $descendant.Kill($true) }
                $descendant.Dispose()
            }
        }
    }
}

Describe 'Dispatch refusal boundaries' -Tag 'Harness' {
    It 'stops the real launcher before a prompt or GitHub operation' {
        $result = Invoke-E2eProcess $pwsh @('-NoProfile', '-File', (Join-Path $PSScriptRoot '..\dispatch-live-e2e.ps1'))
        $result.Code | Should -Be 1
        $result.Out | Should -BeExactly ''
        $result.Err.Contains('canary') | Should -BeFalse
    }
    It 'keeps manual workflow code and credentials restricted' {
        $workflow = Get-Content -LiteralPath (Join-Path $PSScriptRoot '..\..\.github\workflows\live-e2e.yml') -Raw
        $workflow | Should -Match '(?m)^  workflow_dispatch:\r?$'
        $workflow | Should -Not -Match '(?m)^  (push|pull_request|schedule|workflow_call):'
        $workflow | Should -Match 'runs-on: windows-latest'
        $workflow | Should -Not -Match 'secrets\.|secrets\['
        $workflow | Should -Match 'Microsoft.VisualStudio.Product.BuildTools'
        $workflow | Should -Match '\[17\.0,18\.0\)'
        $workflow.Contains("`$env:REF -cne 'refs/heads/main'") | Should -BeTrue
        $workflow | Should -Match 'persist-credentials: false'
    }
}
