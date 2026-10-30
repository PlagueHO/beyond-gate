#Requires -Version 7.4
# Deterministic harness tests: synthetic values and child processes only.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Import-Module (Join-Path $PSScriptRoot 'e2e\common.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $PSScriptRoot 'e2e\dispatch.psm1') -Force
$pwsh = (Get-Command pwsh -CommandType Application | Select-Object -First 1).Source
$checks = 0

function Assert-Harness {
    param([bool]$Condition)
    if (!$Condition) { throw 'Synthetic harness assertion failed.' }
    $script:checks++
}

function Assert-SafeFailure {
    param([scriptblock]$Action, [string]$Reason)
    $caught = $false
    try { & $Action | Out-Null }
    catch {
        $caught = $true
        Assert-Harness ($_.Exception.Message -ceq $Reason)
    }
    Assert-Harness $caught
}

function Get-ChildArguments {
    param([string]$Code)
    return @('-NoProfile', '-EncodedCommand', [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($Code)))
}

$inventory = Get-E2eInventory
$cases = Get-E2eCases $inventory
Assert-Harness ($cases.Count -ge 150)
Assert-Harness (!(Get-E2eReport $cases).full_coverage)
Assert-Harness ((Get-E2eReport $cases).exit_code -eq 1)
foreach ($case in $cases) {
    if ($case.class -eq 'live') { Set-E2eResult $case 'blocked' 'live_prerequisites' }
    elseif ($case.class -eq 'native') { Set-E2eResult $case 'not_applicable' 'not_selected' }
    else { Set-E2eResult $case 'passed' 'verified' }
}
$partial = Get-E2eReport $cases 'contracts'
Assert-Harness ($partial.exit_code -eq 0 -and !$partial.full_coverage)
Assert-Harness ((($partial.counts.Keys | Sort-Object) -join ',') -ceq 'blocked,failed,not_applicable,passed')
Assert-Harness (($partial.counts.Values | Measure-Object -Sum).Sum -eq $cases.Count)
Assert-Harness ((Get-E2eReport $cases 'full').exit_code -eq 1)
Set-E2eResult $cases[0] 'failed' 'contract_mismatch'
Assert-Harness ((Get-E2eReport $cases 'contracts').exit_code -eq 1)
Set-E2eResult $cases[0] 'passed' 'verified'
Assert-SafeFailure { Get-E2eReport @($cases | Select-Object -Skip 1) } 'inventory_invalid'
Assert-SafeFailure { Get-E2eReport (@($cases) + @($cases[0])) } 'inventory_invalid'
Assert-SafeFailure { Set-E2eResult $cases[0] 'passed' 'synthetic-cookie-canary' } 'result_invalid'
Assert-SafeFailure { Set-E2eResult $cases[0] 'success' 'verified' } 'result_invalid'
Assert-SafeFailure { Assert-E2eLiveReady } 'live_prerequisites'

# Canary content must never enter a report, even when held in a case's input data.
$canary = 'cookie-canary bearer-canary account-canary private-id https://example.invalid/private proprietary-canary'
$cases[0].data['raw'] = $canary
$report = Get-E2eReport $cases | ConvertTo-Json -Depth 10
Assert-Harness (!$report.Contains($canary) -and !$report.Contains('cookie-canary'))
try { throw [InvalidOperationException]::new($canary) }
catch { Assert-Harness ((Get-E2eReason $_) -ceq 'harness_error') }
$result = Invoke-E2eProcess $pwsh (Get-ChildArguments "[Console]::Out.Write('synthetic-out'); [Console]::Error.Write('synthetic-err'); exit 7")
Assert-Harness ($result.Code -eq 7 -and $result.Out -ceq 'synthetic-out' -and $result.Err -ceq 'synthetic-err')
$result = Invoke-E2eProcess $pwsh (Get-ChildArguments "[Console]::Out.Write('x' * 128)") -MaxBytes 128
Assert-Harness ($result.Code -eq 0 -and $result.Out.Length -eq 128)
$result = Invoke-E2eProcess $pwsh (Get-ChildArguments '[Console]::In.ReadToEnd() | Out-Null') -InputText ('x' * 65536)
Assert-Harness ($result.Code -eq 0 -and $result.Out -eq '')
Assert-SafeFailure {
    Invoke-E2eProcess $pwsh (Get-ChildArguments "[Console]::Out.Write('cookie-canary' * 10000)") -MaxBytes 128
} 'output_limit'
Assert-SafeFailure {
    Invoke-E2eProcess $pwsh (Get-ChildArguments "[Console]::Error.Write('bearer-canary' * 10000)") -MaxBytes 128
} 'output_limit'
Assert-SafeFailure {
    Invoke-E2eProcess $pwsh (Get-ChildArguments 'Start-Sleep -Seconds 30') -TimeoutSeconds 1
} 'process_timeout'
Assert-SafeFailure {
    Invoke-E2eProcess $pwsh (Get-ChildArguments 'Start-Sleep -Seconds 30') -Deadline ([datetime]::UtcNow.AddSeconds(-1))
} 'suite_timeout'
Assert-SafeFailure {
    Invoke-E2eProcess $pwsh (Get-ChildArguments 'exit 0') -InputText ('x' * 65537)
} 'input_limit'
Assert-SafeFailure { Invoke-E2eProcess (Join-Path $PSScriptRoot 'nonexistent-canary.exe') } 'process_start'
foreach ($output in @('private-id cookie-canary', '{"jsonrpc":"1.0","id":1}', '{broken')) {
    $child = Start-E2eChild $pwsh (Get-ChildArguments "[Console]::Out.WriteLine('$output')")
    try {
        Assert-SafeFailure { Invoke-E2eRequest $child @{ jsonrpc = '2.0'; id = 1; method = 'synthetic' } } 'protocol_invalid'
    } finally { Stop-E2eChild $child }
}
$child = Start-E2eChild $pwsh (Get-ChildArguments 'Start-Sleep -Seconds 30')
$pidToCheck = $child.Process.Id
Stop-E2eChild $child
Assert-Harness ($null -eq (Get-Process -Id $pidToCheck -ErrorAction SilentlyContinue))
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
$descendantId = $null
try {
    do {
        Receive-E2eBytes $child
        $text = [Text.Encoding]::UTF8.GetString($child.Streams.Out.Data.ToArray())
        Start-Sleep -Milliseconds 5
    } until ($text.Contains("`n"))
    Assert-Harness ($text.Trim() -match '^[1-9][0-9]*$')
    $descendantId = [int]$text.Trim()
} finally { Stop-E2eChild $child }
$descendant = Get-Process -Id $descendantId -ErrorAction SilentlyContinue
if ($null -ne $descendant) {
    try { Assert-Harness ($descendant.WaitForExit(5000)) }
    finally {
        if (!$descendant.HasExited) { $descendant.Kill($true) }
        $descendant.Dispose()
    }
} else { Assert-Harness $true }
$child = Start-E2eChild $pwsh (Get-ChildArguments 'exit 0')
try {
    Assert-SafeFailure { Invoke-E2eRequest $child @{ jsonrpc = '2.0'; id = 1; method = 'synthetic' } } 'protocol_missing'
} finally { Stop-E2eChild $child }

# The fake gh records arguments and stdin in memory. It never contacts GitHub.
$seenNames = [Collections.Generic.HashSet[string]]::new()
foreach ($scenario in @('success', 'success-again', 'list', 'slot', 'capacity', 'upload', 'dispatch', 'run', 'lookup', 'queue', 'cancel', 'cancel-throw', 'cleanup', 'cleanup-throw')) {
    $script:scenario = $scenario
    $script:calls = [Collections.Generic.List[hashtable]]::new()
    $script:submission = ''
    $fakeGh = {
        param([string[]]$Arguments, [string]$InputText)
        $script:calls.Add(@{ Arguments = $Arguments; Input = $InputText })
        $operation = "$($Arguments[0]) $($Arguments[1])"
        $response = @{ Code = 0; Out = ''; Err = '' }
        switch ($operation) {
            'secret list' {
                $response.Out = '[]'
                if ($script:scenario -eq 'list') { $response.Code = 1 }
                if ($script:scenario -eq 'slot') {
                    $response.Out = '[{"name":"BEYOND_GATE_E2E_0123456789ABCDEF0123456789ABCDEF"}]'
                }
                if ($script:scenario -eq 'capacity') {
                    $response.Out = ConvertTo-Json -Compress @(1..100 | ForEach-Object { @{ name = "SYNTHETIC_$_" } })
                }
            }
            'secret set' {
                if ($script:scenario -eq 'upload') { $response.Code = 1; $response.Err = 'cookie-canary' }
            }
            'workflow run' {
                $field = $Arguments | Where-Object { $_.StartsWith('submission_id=') }
                $script:submission = $field.Substring('submission_id='.Length)
                if ($script:scenario -eq 'dispatch') { $response.Code = 1; $response.Err = 'private-id' }
            }
            'run list' {
                if ($script:scenario -eq 'lookup') { $response.Code = 1 }
                else {
                    $conclusion = if ($script:scenario -eq 'run') { 'failure' } else { 'success' }
                    $status = if ($script:scenario -in @('queue', 'cancel', 'cancel-throw')) { 'queued' } else { 'completed' }
                    $response.Out = ConvertTo-Json -Compress @(@{
                        databaseId = 123; displayTitle = "live-e2e-$script:submission"
                        status = $status; conclusion = $conclusion
                    })
                }
            }
            'run cancel' {
                if ($script:scenario -eq 'cancel') { $response.Code = 1 }
                if ($script:scenario -eq 'cancel-throw') { throw 'cookie-canary' }
            }
            'secret delete' {
                if ($script:scenario -eq 'cleanup') { $response.Code = 1 }
                if ($script:scenario -eq 'cleanup-throw') { throw 'private-url-canary' }
            }
            default { throw 'Unexpected synthetic gh operation.' }
        }
        return $response
    }
    $secret = ConvertTo-SecureString 'synthetic-cookie-canary' -AsPlainText -Force
    $options = @{ Cookie = $secret; InvokeGh = $fakeGh; QueueTimeoutSeconds = 1; TimeoutSeconds = 10 }
    if ($scenario -eq 'slot') { $options.SubmissionId = '0123456789abcdef0123456789abcdef' }
    try {
        if ($scenario -in @('success', 'success-again')) {
            $result = Invoke-E2eSecretSubmission @options
            Assert-Harness ($result.run_id -eq 123)
        } else {
            $caught = $false
            try { Invoke-E2eSecretSubmission @options | Out-Null }
            catch {
                $caught = $true
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
                Assert-Harness ($_.Exception.Message -cmatch $expected)
                Assert-Harness (!$_.Exception.Message.Contains('canary'))
            }
            Assert-Harness $caught
        }
        $sets = @($script:calls | Where-Object { $_.Arguments[0] -eq 'secret' -and $_.Arguments[1] -eq 'set' })
        $deletes = @($script:calls | Where-Object { $_.Arguments[0] -eq 'secret' -and $_.Arguments[1] -eq 'delete' })
        if ($scenario -in @('list', 'slot', 'capacity')) {
            Assert-Harness ($sets.Count -eq 0 -and $deletes.Count -eq 0)
        } else {
            Assert-Harness ($sets.Count -eq 1 -and $deletes.Count -eq 1)
            Assert-Harness ($sets[0].Arguments[2] -ceq $deletes[0].Arguments[2])
            Assert-Harness ($sets[0].Input -ceq 'synthetic-cookie-canary')
            Assert-Harness ($seenNames.Add($sets[0].Arguments[2]))
        }
        Assert-Harness (($script:calls.Arguments -join ' ') -notmatch 'synthetic-cookie-canary|--body')
        Assert-Harness (@($script:calls | Where-Object { $_.Arguments[0] -eq 'workflow' }).Count -le 1)
        if ($scenario -in @('queue', 'cancel', 'cancel-throw')) {
            Assert-Harness (@($script:calls | Where-Object { $_.Arguments[0] -eq 'run' -and $_.Arguments[1] -eq 'cancel' }).Count -eq 1)
        }
        if ($scenario -eq 'upload') {
            Assert-Harness (@($script:calls | Where-Object { $_.Arguments[0] -eq 'workflow' }).Count -eq 0)
        }
    } finally { $secret.Dispose() }
}

# Verify the real entry point stops before a prompt or gh when live is unavailable.
$result = Invoke-E2eProcess $pwsh @('-NoProfile', '-File', (Join-Path $PSScriptRoot 'dispatch-live-e2e.ps1'))
Assert-Harness ($result.Code -eq 1 -and $result.Out -eq '' -and !$result.Err.Contains('canary'))

# Verify workflow boundaries without dependency installation or a network YAML check.
$workflow = Get-Content -LiteralPath (Join-Path $PSScriptRoot '..\.github\workflows\live-e2e.yml') -Raw
Assert-Harness ($workflow -match '(?m)^  workflow_dispatch:\r?$' -and $workflow -notmatch '(?m)^  (push|pull_request|schedule|workflow_call):')
Assert-Harness ($workflow -match 'runs-on: windows-latest' -and $workflow -notmatch 'secrets\.|secrets\[')
Assert-Harness ($workflow -match "Microsoft.VisualStudio.Product.BuildTools" -and $workflow -match "\[17\.0,18\.0\)")
Assert-Harness ($workflow.Contains("`$env:REF -cne 'refs/heads/main'") -and $workflow -match 'persist-credentials: false')
Write-Output "Passed $checks offline harness assertions. No native credentials or live accounts were accessed."
