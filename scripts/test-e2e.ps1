#Requires -Version 7.4
# Run built executables only. This stage never requests or exchanges credentials.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$BinaryDirectory,
    [ValidateSet('full', 'contracts')][string]$Selection = 'full',
    [switch]$ApproveNativeStorage,
    [string]$ReportPath,
    [ValidateRange(1, 600)][int]$SuiteTimeoutSeconds = 180
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Import-Module (Join-Path $PSScriptRoot 'e2e\common.psm1') -Force -DisableNameChecking

$inventory = Get-E2eInventory
$cases = Get-E2eCases $inventory
$suffix = if ($IsWindows) { '.exe' } else { '' }
$directory = [IO.Path]::GetFullPath($BinaryDirectory)
$binaries = @{ cli = Join-Path $directory "beyond-gate$suffix"; server = Join-Path $directory "beyond-gate-mcp$suffix" }
$helper = Join-Path $directory "examples\e2e-session$suffix"
$context = [guid]::NewGuid().ToString('N')
$temporary = Join-Path ([IO.Path]::GetTempPath()) "Beyond Gate E2E $context"
$homePath = Join-Path $temporary 'synthetic home'
$clients = Join-Path $temporary 'clients'
$environment = @{
    BEYOND_GATE_E2E_CONTEXT = $context
    BEYOND_GATE_E2E_NATIVE_APPROVED = '0'
    HOME = $homePath; USERPROFILE = $homePath; APPDATA = $homePath
    LOCALAPPDATA = $homePath; XDG_CONFIG_HOME = $homePath
    COPILOT_HOME = Join-Path $homePath 'copilot'
    PATH = $clients; PATHEXT = '.CMD;.EXE'
}
$deadline = [datetime]::UtcNow.AddSeconds($SuiteTimeoutSeconds)
$nativeStarted = $false
$versionMatch = [regex]::Match(
    (Get-Content -LiteralPath (Join-Path $PSScriptRoot '..\Cargo.toml') -Raw),
    '(?m)^version = "([^"]+)"\r?$'
)
Assert-E2e ($versionMatch.Success) 'inventory_invalid'
$version = $versionMatch.Groups[1].Value

function Invoke-SuiteProcess {
    param([string]$Executable, [string[]]$Arguments = @(), [string]$InputText = '')
    Invoke-E2eProcess -Executable $Executable -Arguments $Arguments -InputText $InputText `
        -Environment $environment -Directory $temporary -Deadline $deadline
}

function Assert-SuiteOutput {
    param([hashtable]$Result, [hashtable]$Data)
    Assert-E2e ($Result.Code -eq $Data.code)
    if ($Data.code -eq 0) {
        Assert-E2e ($Result.Err -eq '' -and $Result.Out.Contains($Data.contains))
    } else {
        Assert-E2e ($Result.Out -eq '' -and $Result.Err -ne '')
        if ($Data.ContainsKey('error')) { Assert-E2e ($Result.Err.Contains($Data.error)) }
    }
}

function Invoke-InstallCase {
    param([hashtable]$Data)
    $path = if ($Data.client -eq 'claude-code') {
        Join-Path $homePath '.claude.json'
    } else { Join-Path $environment.COPILOT_HOME 'mcp-config.json' }
    $settings = '{"keep":{"synthetic":true},"mcpServers":{"other":{"command":"synthetic-other"}}}'
    if ($Data.scenario -eq 'conflict') {
        $settings = '{"mcpServers":{"beyond-gate":{"command":"synthetic-conflict"}}}'
    } elseif ($Data.scenario -eq 'malformed') { $settings = '{not-json' }
    [IO.Directory]::CreateDirectory((Split-Path -Parent $path)) | Out-Null
    [IO.File]::WriteAllText($path, $settings)
    $arguments = @('mcp', 'install', $Data.client)
    $inputText = "n`n"
    if ($Data.scenario -eq 'preview') { $arguments += '--dry-run' }
    if ($Data.scenario -in @('confirm', 'idempotence')) { $inputText = "y`n" }
    $result = Invoke-SuiteProcess $binaries.cli $arguments $inputText
    if ($Data.scenario -in @('conflict', 'malformed')) {
        Assert-E2e ($result.Code -eq 1 -and $result.Out -eq '' -and $result.Err -ne '')
    } else { Assert-E2e ($result.Code -eq 0 -and $result.Err -eq '') }
    if ($Data.scenario -in @('confirm', 'idempotence')) {
        $updated = [IO.File]::ReadAllText($path) | ConvertFrom-Json -AsHashtable
        Assert-E2e ($updated.keep.synthetic -eq $true -and $updated.mcpServers.other.command -ceq 'synthetic-other')
        $entry = $updated.mcpServers.'beyond-gate'
        Assert-E2e ($entry.command -ceq $binaries.server -and $entry.args.Count -eq 0)
        if ($Data.client -ne 'claude-code') { Assert-E2e ($entry.type -ceq 'local') }
        if ($Data.scenario -eq 'idempotence') {
            $before = [IO.File]::ReadAllText($path)
            $result = Invoke-SuiteProcess $binaries.cli $arguments
            Assert-E2e ($result.Code -eq 0 -and $result.Err -eq '' -and $result.Out.Contains('already registered'))
            Assert-E2e ([IO.File]::ReadAllText($path) -ceq $before)
        }
    } else { Assert-E2e ([IO.File]::ReadAllText($path) -ceq $settings) }
}

try {
    [IO.Directory]::CreateDirectory($clients) | Out-Null
    [IO.Directory]::CreateDirectory($homePath) | Out-Null
    foreach ($name in @('code', 'claude', 'copilot')) {
        $filename = if ($IsWindows) { "$name.cmd" } else { $name }
        [IO.File]::WriteAllText((Join-Path $clients $filename), "# Synthetic marker; never executed.`n")
    }
    foreach ($case in $cases) {
        if ($case.class -eq 'live') {
            Set-E2eResult $case 'blocked' 'live_prerequisites'
            continue
        }
        if ($case.class -eq 'native') {
            if ($Selection -eq 'contracts') { Set-E2eResult $case 'not_applicable' 'not_selected' }
            elseif (!$ApproveNativeStorage) { Set-E2eResult $case 'blocked' 'native_approval_required' }
            elseif (!$IsWindows) { Set-E2eResult $case 'blocked' 'native_platform' }
            continue
        }
        if ($case.class.StartsWith('mcp-')) { continue }
        try {
            if ($case.class -eq 'cli') {
                $binary = if ($case.data.ContainsKey('binary')) { $case.data.binary } else { 'cli' }
                $result = Invoke-SuiteProcess $binaries[$binary] $case.data.arguments
                Assert-SuiteOutput $result $case.data
                if ($case.id.EndsWith('.version')) {
                    $name = if ($binary -eq 'server') { 'beyond-gate-mcp' } else { 'beyond-gate' }
                    Assert-E2e ($result.Out.TrimEnd("`r", "`n") -ceq "$name $version")
                }
            } elseif ($case.class -eq 'install') { Invoke-InstallCase $case.data }
            else { throw [InvalidOperationException]::new('inventory_invalid') }
            Set-E2eResult $case 'passed' 'verified'
        } catch { Set-E2eResult $case 'failed' (Get-E2eReason $_) }
    }
    foreach ($mode in @('direct', 'start')) {
        $group = @($cases | Where-Object { $_.class -ceq "mcp-$mode" })
        try {
            $executable = if ($mode -eq 'direct') { $binaries.server } else { $binaries.cli }
            $arguments = if ($mode -eq 'direct') { @() } else { @('mcp', 'start') }
            $verified = Invoke-E2eProtocol $executable $arguments $inventory $environment $temporary $deadline
            foreach ($case in $group) {
                Assert-E2e ($verified.Contains($case.id.Split('.')[-1]))
                Set-E2eResult $case 'passed' 'verified'
            }
        } catch {
            foreach ($case in $group) { Set-E2eResult $case 'failed' (Get-E2eReason $_) }
        }
    }
    if ($Selection -eq 'full' -and $ApproveNativeStorage -and $IsWindows) {
        try {
            # Check every binary before allowing any native operation.
            foreach ($executable in @($binaries.cli, $binaries.server, $helper)) {
                $arguments = if ($executable -eq $helper) { @('check') } else { @('--e2e-context-check') }
                $result = Invoke-SuiteProcess $executable $arguments
                Assert-E2e ($result.Code -eq 0 -and $result.Err -eq '' -and $result.Out.Trim() -ceq 'beyond-gate-e2e-isolation-v1') 'isolation_required'
            }
            $environment.BEYOND_GATE_E2E_NATIVE_APPROVED = '1'
            [Console]::Error.WriteLine("Isolated native test context: $context. Keep this value for targeted cleanup after interruption.")
            $nativeStarted = $true
            $result = Invoke-SuiteProcess $helper @('cleanup')
            Assert-E2e ($result.Code -eq 0 -and $result.Err -eq '')
            foreach ($case in @($cases | Where-Object { $_.class -eq 'native' })) {
                if ($case.id -eq 'native.auth.stored-unverified') {
                    $result = Invoke-SuiteProcess $helper @('seed')
                    Assert-E2e ($result.Code -eq 0 -and $result.Err -eq '')
                }
                $action = if ($case.id -in @('native.auth.absent', 'native.auth.stored-unverified')) { 'status' } else { 'logout' }
                $result = Invoke-SuiteProcess $binaries.cli @('auth', $action)
                $expected = switch ($case.id) {
                    'native.auth.absent' { 'absent:' }
                    'native.auth.stored-unverified' { 'stored-unverified:' }
                    default { 'removed or already absent' }
                }
                Assert-E2e ($result.Code -eq 0 -and $result.Err -eq '' -and $result.Out.Contains($expected))
                Set-E2eResult $case 'passed' 'verified'
            }
            $result = Invoke-SuiteProcess $binaries.cli @('auth', 'status')
            Assert-E2e ($result.Code -eq 0 -and $result.Out.StartsWith('absent:'))
        } catch {
            foreach ($case in @($cases | Where-Object { $_.class -eq 'native' })) {
                Set-E2eResult $case 'failed' (Get-E2eReason $_)
            }
        }
    }
} catch {
    $cases.Add((New-E2eCase 'suite.infrastructure' 'infrastructure'))
    Set-E2eResult $cases[-1] 'failed' (Get-E2eReason $_)
} finally {
    if ($nativeStarted) {
        try {
            $result = Invoke-E2eProcess $helper @('cleanup') -Environment $environment -Directory $temporary
            Assert-E2e ($result.Code -eq 0 -and $result.Err -eq '') 'cleanup_failed'
        } catch {
            $cases.Add((New-E2eCase 'suite.native-cleanup' 'infrastructure'))
            Set-E2eResult $cases[-1] 'failed' 'cleanup_failed'
            Write-Warning "Isolated native cleanup failed. Recovery context: $context. See docs/testing.md."
        }
    }
    try {
        if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
    } catch {
        $cases.Add((New-E2eCase 'suite.file-cleanup' 'infrastructure'))
        Set-E2eResult $cases[-1] 'failed' 'cleanup_failed'
    }
}
$report = Get-E2eReport $cases $Selection
$json = $report | ConvertTo-Json -Depth 10
if ($ReportPath) { [IO.File]::WriteAllText([IO.Path]::GetFullPath($ReportPath), "$json`n") }
$json
exit $report.exit_code
