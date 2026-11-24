# Bounded real process fixtures. Each owner must close its own context.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking

function New-E2eContext {
    param([string]$BinaryDirectory, [datetime]$Deadline)
    $identity = [guid]::NewGuid().ToString('N')
    $temporary = Join-Path ([IO.Path]::GetTempPath()) "Beyond Gate E2E $identity"
    $homePath = Join-Path $temporary 'synthetic home'
    $clients = Join-Path $temporary 'clients'
    $suffix = if ($IsWindows) { '.exe' } else { '' }
    $directory = [IO.Path]::GetFullPath($BinaryDirectory)
    $context = @{
        Identity = $identity; Directory = $temporary; Home = $homePath; Deadline = $Deadline
        Binaries = @{
            cli = Join-Path $directory "beyond-gate$suffix"
            server = Join-Path $directory "beyond-gate-mcp$suffix"
            helper = Join-Path $directory "examples\e2e-session$suffix"
        }
        Environment = @{
            BEYOND_GATE_E2E_CONTEXT = $identity; BEYOND_GATE_E2E_NATIVE_APPROVED = '0'
            HOME = $homePath; USERPROFILE = $homePath; APPDATA = $homePath
            LOCALAPPDATA = $homePath; XDG_CONFIG_HOME = $homePath
            COPILOT_HOME = Join-Path $homePath 'copilot'; PATH = $clients; PATHEXT = '.CMD;.EXE'
        }
        NativeStarted = $false
    }
    try {
        [IO.Directory]::CreateDirectory($clients) | Out-Null
        [IO.Directory]::CreateDirectory($homePath) | Out-Null
        foreach ($name in @('code', 'claude', 'copilot')) {
            $filename = if ($IsWindows) { "$name.cmd" } else { $name }
            [IO.File]::WriteAllText((Join-Path $clients $filename), "# Synthetic marker; never executed.`n")
        }
        return $context
    } catch {
        if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
        throw [InvalidOperationException]::new('harness_error')
    }
}

function Invoke-E2eContextProcess {
    param([hashtable]$Context, [string]$Binary, [string[]]$Arguments = @(), [string]$InputText = '')
    return Invoke-E2eProcess -Executable $Context.Binaries[$Binary] -Arguments $Arguments `
        -InputText $InputText -Environment $Context.Environment -Directory $Context.Directory -Deadline $Context.Deadline
}

function Close-E2eContext {
    param([hashtable]$Context)
    $cleanupKinds = @()
    try {
        if ($Context.NativeStarted) {
            try {
                $result = Invoke-E2eProcess $Context.Binaries.helper @('cleanup') `
                    -Environment $Context.Environment -Directory $Context.Directory
                Assert-E2e ($result.Code -eq 0 -and $result.Err -eq '') 'cleanup_failed'
            } catch {
                [Console]::Error.WriteLine("Isolated native cleanup failed. Recovery context: $($Context.Identity). See docs/testing.md.")
                $cleanupKinds += 'native'
            }
        }
    } finally {
        if (Test-Path -LiteralPath $Context.Directory) {
            try { Remove-Item -LiteralPath $Context.Directory -Recurse -Force }
            catch { $cleanupKinds += 'file' }
        }
    }
    if ($cleanupKinds.Count -gt 0) {
        $failure = [InvalidOperationException]::new('cleanup_failed')
        $failure.Data['E2eCleanup'] = $cleanupKinds
        throw $failure
    }
}

function Start-E2eNativeContext {
    param([hashtable]$Context)
    foreach ($binary in @('cli', 'server', 'helper')) {
        $arguments = if ($binary -eq 'helper') { @('check') } else { @('--e2e-context-check') }
        $result = Invoke-E2eContextProcess $Context $binary $arguments
        Assert-E2e ($result.Code -eq 0 -and $result.Err -eq '' -and $result.Out.Trim() -ceq 'beyond-gate-e2e-isolation-v1') 'isolation_required'
    }
    $Context.Environment.BEYOND_GATE_E2E_NATIVE_APPROVED = '1'
    $Context.NativeStarted = $true
    [Console]::Error.WriteLine("Isolated native test context: $($Context.Identity). Keep this value for targeted cleanup after interruption.")
    $result = Invoke-E2eContextProcess $Context 'helper' @('cleanup')
    Assert-E2e ($result.Code -eq 0 -and $result.Err -eq '') 'cleanup_failed'
}

Export-ModuleMember -Function *-E2e*
