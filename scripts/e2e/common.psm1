# Shared manual-suite contracts. No raw child output is suitable for diagnostics.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-E2e {
    param([bool]$Condition, [string]$Reason = 'contract_mismatch')
    if (!$Condition) { throw [InvalidOperationException]::new($Reason) }
}

function Get-E2eInventory {
    $inventory = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'capabilities.json') -Raw |
        ConvertFrom-Json -AsHashtable
    Assert-E2e ($inventory.schema -eq 1 -and $inventory.live_request_budget -eq 0 -and !$inventory.live_ready) 'live_prerequisites'
    return $inventory
}

function Assert-E2eLiveReady {
    # No production session or endpoint contract exists. There is deliberately no override.
    throw [InvalidOperationException]::new('live_prerequisites')
}

function New-E2eCase {
    param([string]$Id, [string]$Class, [hashtable]$Data = @{})
    Assert-E2e ($Id -cmatch '^[a-z0-9][a-z0-9._-]*$') 'inventory_invalid'
    return @{
        id = $Id; class = $Class; required = $true; data = $Data
        status = 'blocked'; reason = 'untested'
    }
}

function Get-E2eCases {
    param([hashtable]$Inventory)
    $cases = [Collections.Generic.List[hashtable]]::new()
    foreach ($command in $Inventory.cli) {
        $name = if ($command.path) { $command.path.Replace(' ', '.') } else { 'root' }
        $arguments = if ($command.path) { @($command.path.Split(' ')) } else { @() }
        $cases.Add((New-E2eCase "cli.help.$name" 'cli' @{
            arguments = @($arguments) + '--help'; code = 0; contains = 'Usage:'
        }))
    }
    foreach ($binary in @('cli', 'server')) {
        $cases.Add((New-E2eCase "$binary.version" 'cli' @{ binary = $binary; arguments = @('--version'); code = 0; contains = 'beyond-gate' }))
        $cases.Add((New-E2eCase "$binary.invalid-usage" 'cli' @{ binary = $binary; arguments = @('--invalid'); code = 2 }))
    }
    $cases.Add((New-E2eCase 'server.help' 'cli' @{ binary = 'server'; arguments = @('--help'); code = 0; contains = 'Usage:' }))
    $cases.Add((New-E2eCase 'cli.no-action' 'cli' @{ arguments = @(); code = 2 }))
    $cases.Add((New-E2eCase 'cli.help-subcommand' 'cli' @{ arguments = @('help', 'book', 'read'); code = 0; contains = 'Usage:' }))
    $cases.Add((New-E2eCase 'cli.auth.login-blocker' 'cli' @{
        arguments = @('auth', 'login'); code = 1; error = 'no working session handoff'
    }))
    foreach ($shell in $Inventory.shells) {
        $cases.Add((New-E2eCase "cli.completions.$shell" 'cli' @{
            arguments = @('completions', $shell); code = 0; contains = 'beyond-gate'
        }))
    }
    foreach ($read in $Inventory.reads) {
        $arguments = @($read.path.Split(' '))
        foreach ($key in @('id', 'book_id', 'section_id')) {
            if ($read.arguments.ContainsKey($key)) { $arguments += $read.arguments[$key] }
        }
        $modes = if ($read.list) { @('table', 'json', 'tsv', 'table-no-headers', 'tsv-no-headers') } else { @('json') }
        foreach ($mode in $modes) {
            $selected = @($arguments)
            if ($read.list) {
                $selected += @('--format', $mode.Replace('-no-headers', ''))
                if ($mode.EndsWith('-no-headers')) { $selected += '--no-headers' }
            }
            $name = $read.path.Replace(' ', '.')
            $cases.Add((New-E2eCase "contract.cli.$name.$mode" 'cli' @{
                arguments = $selected; code = 1; error = 'unsupported'
            }))
            $cases.Add((New-E2eCase "live.cli.$name.$mode" 'live'))
        }
        if ($read.list) {
            $cases.Add((New-E2eCase "cli.$($read.tool).json-no-headers" 'cli' @{
                arguments = $arguments + @('--format', 'json', '--no-headers'); code = 2
            }))
        }
    }
    foreach ($mode in @('direct', 'start')) {
        foreach ($name in @('lifecycle', 'discovery', 'invalid-input', 'invalid-token', 'unknown-tool', 'cancellation')) {
            $cases.Add((New-E2eCase "mcp.$mode.$name" "mcp-$mode"))
        }
        foreach ($read in $Inventory.reads) {
            $cases.Add((New-E2eCase "contract.mcp.$mode.$($read.tool)" "mcp-$mode"))
            $cases.Add((New-E2eCase "live.mcp.$mode.$($read.tool)" 'live'))
        }
    }
    foreach ($name in @('absent', 'stored-unverified', 'logout', 'repeated-logout')) {
        $cases.Add((New-E2eCase "native.auth.$name" 'native'))
    }
    foreach ($client in $Inventory.clients) {
        foreach ($scenario in @('preview', 'decline', 'confirm', 'idempotence', 'conflict', 'malformed')) {
            $cases.Add((New-E2eCase "install.$client.$scenario" 'install' @{ client = $client; scenario = $scenario }))
        }
    }
    foreach ($name in $Inventory.live_assertions) {
        $cases.Add((New-E2eCase "live.$name" 'live'))
    }
    Assert-E2e (@($cases.id | Sort-Object -Unique).Count -eq $cases.Count) 'inventory_duplicate'
    return ,$cases
}

function Set-E2eResult {
    param([hashtable]$Case, [string]$Status, [string]$Reason)
    Assert-E2e ($Status -cin @('passed', 'failed', 'blocked', 'not_applicable')) 'result_invalid'
    Assert-E2e ($Reason -cin @(
        'verified', 'live_prerequisites', 'native_approval_required', 'native_platform',
        'contract_mismatch', 'process_start', 'process_timeout', 'output_limit',
        'input_limit', 'protocol_invalid', 'protocol_missing', 'suite_timeout',
        'harness_error', 'cleanup_failed', 'isolation_required', 'not_selected',
        'untested', 'inventory_invalid', 'inventory_duplicate'
    )) 'result_invalid'
    $Case.status = $Status
    $Case.reason = $Reason
}

function Get-E2eReason {
    param([System.Management.Automation.ErrorRecord]$Record)
    $candidate = $Record.Exception.Message
    $probe = @{}
    try { Set-E2eResult $probe 'failed' $candidate; return $candidate }
    catch { return 'harness_error' }
}

function Get-E2eReport {
    param([Collections.IEnumerable]$Cases, [string]$Selection = 'full')
    Assert-E2e ($Selection -cin @('full', 'contracts')) 'inventory_invalid'
    $expected = Get-E2eCases (Get-E2eInventory)
    foreach ($case in $expected) {
        Assert-E2e (@($Cases | Where-Object { $_.id -ceq $case.id }).Count -eq 1) 'inventory_invalid'
    }
    $allowed = @($expected.id) + @('suite.infrastructure', 'suite.native-cleanup', 'suite.file-cleanup')
    foreach ($case in $Cases) { Assert-E2e ($case.id -cin $allowed) 'inventory_invalid' }
    $rows = @($Cases | ForEach-Object {
        Assert-E2e ($_.id -cmatch '^[a-z0-9][a-z0-9._-]*$') 'inventory_invalid'
        Set-E2eResult $_ $_.status $_.reason
        @{ case = $_.id; status = $_.status; reason = $_.reason }
    })
    Assert-E2e ($rows.Count -gt 0 -and @($rows.case | Sort-Object -Unique).Count -eq $rows.Count) 'inventory_duplicate'
    $failed = @($Cases | Where-Object { $_.required -and $_.status -ne 'passed' }).Count
    return @{
        schema = 1; selection = $Selection; full_coverage = ($Selection -eq 'full' -and $failed -eq 0)
        exit_code = $(if ($Selection -eq 'full') { [int]($failed -ne 0) } else {
            [int](@($Cases | Where-Object { $_.class -notin @('live', 'native') -and $_.status -ne 'passed' }).Count -ne 0)
        })
        live_request_budget = 0; cases = $rows
        counts = @{
            passed = @($rows | Where-Object status -eq 'passed').Count
            failed = @($rows | Where-Object status -eq 'failed').Count
            blocked = @($rows | Where-Object status -eq 'blocked').Count
            not_applicable = @($rows | Where-Object status -eq 'not_applicable').Count
        }
    }
}

function Start-E2eChild {
    param(
        [string]$Executable, [string[]]$Arguments = @(), [hashtable]$Environment = @{},
        [string]$Directory, [int]$TimeoutSeconds = 20, [int]$MaxBytes = 1048576,
        [datetime]$Deadline = [datetime]::UtcNow.AddMinutes(5),
        [switch]$AllowGitHubAuthentication
    )
    Assert-E2e ([datetime]::UtcNow -lt $Deadline) 'suite_timeout'
    $start = [Diagnostics.ProcessStartInfo]::new($Executable)
    $start.UseShellExecute = $false
    $start.RedirectStandardInput = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    if ($Directory) { $start.WorkingDirectory = $Directory }
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
    # A launcher-only secret must never leak to a normal product child.
    foreach ($name in @(
        'BEYOND_GATE_E2E_COOKIE', 'BEYOND_GATE_E2E_CONTEXT',
        'BEYOND_GATE_E2E_NATIVE_APPROVED', 'RUST_LOG'
    )) {
        $start.Environment.Remove($name) | Out-Null
    }
    if (!$AllowGitHubAuthentication) {
        $start.Environment.Remove('GH_TOKEN') | Out-Null
        $start.Environment.Remove('GITHUB_TOKEN') | Out-Null
    }
    foreach ($key in $Environment.Keys) { $start.Environment[$key] = $Environment[$key] }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $start
    try {
        Assert-E2e ($process.Start()) 'process_start'
    } catch {
        $process.Dispose()
        throw [InvalidOperationException]::new('process_start')
    }
    $child = @{
        Process = $process; End = [datetime]::UtcNow.AddSeconds($TimeoutSeconds)
        Deadline = $Deadline; MaxBytes = $MaxBytes; Streams = @{}
    }
    foreach ($name in @('Out', 'Err')) {
        $stream = if ($name -eq 'Out') { $process.StandardOutput.BaseStream } else { $process.StandardError.BaseStream }
        $buffer = [byte[]]::new(4096)
        $child.Streams[$name] = @{
            Stream = $stream; Buffer = $buffer; Data = [IO.MemoryStream]::new()
            Task = $stream.ReadAsync($buffer, 0, $buffer.Length); Done = $false
        }
    }
    return $child
}

function Receive-E2eBytes {
    param([hashtable]$Child)
    Assert-E2e ([datetime]::UtcNow -lt $Child.Deadline) 'suite_timeout'
    Assert-E2e ([datetime]::UtcNow -lt $Child.End) 'process_timeout'
    foreach ($state in $Child.Streams.Values) {
        if (!$state.Done -and $state.Task.IsCompleted) {
            $count = $state.Task.GetAwaiter().GetResult()
            if ($count -eq 0) { $state.Done = $true; continue }
            Assert-E2e ($state.Data.Length + $count -le $Child.MaxBytes) 'output_limit'
            $state.Data.Write($state.Buffer, 0, $count)
            $state.Task = $state.Stream.ReadAsync($state.Buffer, 0, $state.Buffer.Length)
        }
    }
}

function Stop-E2eChild {
    param([hashtable]$Child)
    try {
        if (!$Child.Process.HasExited) {
            $Child.Process.Kill($true)
            Assert-E2e ($Child.Process.WaitForExit(5000)) 'cleanup_failed'
        }
    } finally {
        foreach ($state in $Child.Streams.Values) { $state.Data.Dispose() }
        $Child.Process.Dispose()
    }
}

function Send-E2eInput {
    param([hashtable]$Child, [string]$Text)
    Assert-E2e ([Text.Encoding]::UTF8.GetByteCount($Text) -le 65536) 'input_limit'
    $task = $Child.Process.StandardInput.WriteAsync($Text)
    while (!$task.IsCompleted) { Receive-E2eBytes $Child; Start-Sleep -Milliseconds 5 }
    $null = $task.GetAwaiter().GetResult()
    $task = $Child.Process.StandardInput.FlushAsync()
    while (!$task.IsCompleted) { Receive-E2eBytes $Child; Start-Sleep -Milliseconds 5 }
    $null = $task.GetAwaiter().GetResult()
}

function Complete-E2eChild {
    param([hashtable]$Child)
    $Child.Process.StandardInput.Close()
    while (!$Child.Process.HasExited -or @($Child.Streams.Values | Where-Object { !$_.Done }).Count -gt 0) {
        Receive-E2eBytes $Child
        Start-Sleep -Milliseconds 5
    }
    return @{
        Code = $Child.Process.ExitCode
        Out = [Text.UTF8Encoding]::new($false, $true).GetString($Child.Streams.Out.Data.ToArray())
        Err = [Text.UTF8Encoding]::new($false, $true).GetString($Child.Streams.Err.Data.ToArray())
    }
}

function Invoke-E2eProcess {
    param(
        [string]$Executable, [string[]]$Arguments = @(), [string]$InputText = '',
        [hashtable]$Environment = @{}, [string]$Directory, [int]$TimeoutSeconds = 20,
        [int]$MaxBytes = 1048576, [datetime]$Deadline = [datetime]::UtcNow.AddMinutes(5),
        [switch]$AllowGitHubAuthentication
    )
    $child = Start-E2eChild $Executable $Arguments $Environment $Directory $TimeoutSeconds $MaxBytes $Deadline `
        -AllowGitHubAuthentication:$AllowGitHubAuthentication
    try {
        Send-E2eInput $child $InputText
        return Complete-E2eChild $child
    } finally { Stop-E2eChild $child }
}

function Get-E2eMessages {
    param([hashtable]$Child)
    $text = [Text.UTF8Encoding]::new($false, $true).GetString($Child.Streams.Out.Data.ToArray())
    $lines = $text.Split("`n")
    $messages = @()
    for ($index = 0; $index -lt $lines.Count - 1; $index++) {
        try {
            $message = ConvertFrom-Json -InputObject $lines[$index] -AsHashtable
            Assert-E2e ($message.jsonrpc -ceq '2.0') 'protocol_invalid'
            $messages += $message
        } catch { throw [InvalidOperationException]::new('protocol_invalid') }
    }
    return $messages
}

function Invoke-E2eRequest {
    param([hashtable]$Child, [hashtable]$Message)
    Send-E2eInput $Child "$(($Message | ConvertTo-Json -Depth 20 -Compress))`n"
    while ($true) {
        Receive-E2eBytes $Child
        $matches = @(Get-E2eMessages $Child | Where-Object { $_.ContainsKey('id') -and $_.id -eq $Message.id })
        Assert-E2e ($matches.Count -le 1) 'protocol_invalid'
        if ($matches.Count -eq 1) { return $matches[0] }
        Assert-E2e (!$Child.Process.HasExited) 'protocol_missing'
        Start-Sleep -Milliseconds 5
    }
}

function Invoke-E2eProtocol {
    param([string]$Executable, [string[]]$Arguments, [hashtable]$Inventory, [hashtable]$Environment, [string]$Directory, [datetime]$Deadline)
    $child = Start-E2eChild $Executable $Arguments $Environment $Directory 30 1048576 $Deadline
    $verified = [Collections.Generic.List[string]]::new()
    try {
        $response = Invoke-E2eRequest $child @{
            jsonrpc = '2.0'; id = 1; method = 'initialize'; params = @{
                protocolVersion = '2026-07-28'; capabilities = @{}
                clientInfo = @{ name = 'beyond-gate-e2e'; version = '0.1.0' }
            }
        }
        Assert-E2e ($response.result.capabilities.ContainsKey('tools')) 'contract_mismatch'
        Send-E2eInput $child ('{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' + "`n")
        $response = Invoke-E2eRequest $child @{ jsonrpc = '2.0'; id = 2; method = 'tools/list'; params = @{} }
        $names = @($response.result.tools.name | Sort-Object)
        Assert-E2e (($names -join ',') -ceq (($Inventory.reads.tool | Sort-Object) -join ',')) 'inventory_invalid'
        foreach ($tool in $response.result.tools) {
            Assert-E2e ($tool.annotations.readOnlyHint -eq $true -and $tool.annotations.destructiveHint -eq $false)
            Assert-E2e ($tool.inputSchema.additionalProperties -eq $false)
            $read = $Inventory.reads | Where-Object { $_.tool -ceq $tool.name }
            $properties = @($read.arguments.Keys)
            if ($tool.name -ceq 'book_read') { $properties += 'continuation' }
            $actualProperties = if ($tool.inputSchema.ContainsKey('properties')) { @($tool.inputSchema.properties.Keys | Sort-Object) } else { @() }
            Assert-E2e (($actualProperties -join ',') -ceq (@($properties | Sort-Object) -join ',')) 'inventory_invalid'
        }
        $book = $response.result.tools | Where-Object { $_.name -ceq 'book_read' }
        Assert-E2e ($book.inputSchema.properties.continuation.maxLength -eq 16384)
        $verified.Add('discovery')
        $id = 3
        foreach ($read in $Inventory.reads) {
            $response = Invoke-E2eRequest $child @{
                jsonrpc = '2.0'; id = $id; method = 'tools/call'
                params = @{ name = $read.tool; arguments = $read.arguments }
            }
            Assert-E2e ($response.result.isError -eq $true)
            Assert-E2e ($response.result.content[0].text -cmatch 'unsupported')
            $verified.Add($read.tool)
            $id++
        }
        foreach ($check in @(
            @{ name = 'invalid-input'; tool = 'character_show'; arguments = @{ id = 'synthetic'; extra = $true } }
            @{ name = 'invalid-token'; tool = 'book_read'; arguments = @{ book_id = 'synthetic'; section_id = 'opening'; continuation = 'invalid' } }
            @{ name = 'unknown-tool'; tool = 'not_a_tool'; arguments = @{} }
        )) {
            $response = Invoke-E2eRequest $child @{
                jsonrpc = '2.0'; id = $id; method = 'tools/call'
                params = @{ name = $check.tool; arguments = $check.arguments }
            }
            Assert-E2e ($response.ContainsKey('error') -or ($response.ContainsKey('result') -and $response.result.isError -eq $true))
            $verified.Add($check.name)
            $id++
        }
        Send-E2eInput $child ('{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":999,"reason":"synthetic"}}' + "`n")
        $response = Invoke-E2eRequest $child @{ jsonrpc = '2.0'; id = $id; method = 'ping'; params = @{} }
        Assert-E2e ($response.ContainsKey('result'))
        $verified.Add('cancellation')
        $result = Complete-E2eChild $child
        Assert-E2e ($result.Code -eq 0 -and $result.Err -eq '')
        Assert-E2e (@(Get-E2eMessages $child).Count -eq $id)
        Assert-E2e ($result.Out.EndsWith("`n"))
        $verified.Add('lifecycle')
        return ,$verified
    } finally { Stop-E2eChild $child }
}

Export-ModuleMember -Function *-E2e*
