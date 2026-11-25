# Shared bounded child-process checks for native and Docker development workflows.
$ErrorActionPreference = 'Stop'

function Invoke-DeveloperProcess {
    <#
    .SYNOPSIS
    Runs an owned process with separate streams, optional input, and a deadline.
    #>
    param(
        [Parameter(Mandatory)][string]$Command,
        [string[]]$Arguments = @(),
        [string]$InputText = '',
        [string]$WorkingDirectory = '',
        [int]$TimeoutSeconds = 120
    )
    $start = [Diagnostics.ProcessStartInfo]::new($Command)
    $start.UseShellExecute = $false
    $start.RedirectStandardInput = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    if ($WorkingDirectory) { $start.WorkingDirectory = $WorkingDirectory }
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
    $start.Environment['BEYOND_GATE_COBALT_SESSION'] = 'invented invalid workflow sentinel'
    $start.Environment.Remove('BEYOND_GATE_E2E_CONTEXT') | Out-Null
    $start.Environment.Remove('BEYOND_GATE_E2E_NATIVE_APPROVED') | Out-Null
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $start
    $started = $false
    try {
        if (-not $process.Start()) { throw "Cannot start $Command." }
        $started = $true
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        $process.StandardInput.Write($InputText)
        $process.StandardInput.Close()
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            throw "$Command exceeded the $TimeoutSeconds second check deadline."
        }
        return @{
            ExitCode = $process.ExitCode
            Stdout = $stdout.GetAwaiter().GetResult()
            Stderr = $stderr.GetAwaiter().GetResult()
        }
    }
    finally {
        if ($started -and -not $process.HasExited) {
            $process.Kill($true)
            $process.WaitForExit()
        }
        $process.Dispose()
    }
}

function Get-DeveloperProtocolInput {
    <#
    .SYNOPSIS
    Builds discovery and invalid-input requests that cannot perform live reads.
    #>
    $messages = @(
        @{ jsonrpc = '2.0'; id = 1; method = 'initialize'; params = @{
            protocolVersion = '2026-07-28'; capabilities = @{}
            clientInfo = @{ name = 'developer-workflow-check'; version = '1' }
        }},
        @{ jsonrpc = '2.0'; method = 'notifications/initialized' },
        @{ jsonrpc = '2.0'; id = 2; method = 'tools/list'; params = @{} },
        @{ jsonrpc = '2.0'; id = 3; method = 'tools/call'; params = @{
            name = 'character_show'; arguments = @{ id = 'valid'; extra = $true }
        }}
    )
    return (($messages | ForEach-Object { $_ | ConvertTo-Json -Depth 8 -Compress }) -join "`n") + "`n"
}

function Assert-DeveloperProtocol {
    <#
    .SYNOPSIS
    Checks clean stdin closure, protocol-only stdout, discovery, and input refusal.
    #>
    param([Parameter(Mandatory)][hashtable]$Result)
    if ($Result.ExitCode -ne 0) { throw "MCP process failed: $($Result.Stderr)" }
    $messages = @($Result.Stdout -split "`n" | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json })
    if ($messages.Count -ne 3 -or @($messages | Where-Object { $_.jsonrpc -ne '2.0' }).Count) {
        throw 'MCP stdout did not contain exactly three JSON-RPC responses.'
    }
    $initialized = $messages | Where-Object id -EQ 1
    $listed = $messages | Where-Object id -EQ 2
    $refused = $messages | Where-Object id -EQ 3
    if ($null -eq $initialized.result.capabilities.tools -or -not $listed.result.tools.Count -or
        -not $refused.result.isError -or $refused.result.content[0].text -notmatch 'unknown field') {
        throw 'MCP initialization, discovery, or unknown-field refusal failed.'
    }
}

Export-ModuleMember -Function Invoke-DeveloperProcess, Get-DeveloperProtocolInput, Assert-DeveloperProtocol
