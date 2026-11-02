# Installs only the pinned debugger on a disposable GitHub Actions Windows runner.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
if (!$IsWindows -or $env:GITHUB_ACTIONS -ne 'true' -or !$env:RUNNER_TEMP) {
    throw 'Debugger setup requires a disposable GitHub Actions Windows runner.'
}
$debugger = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Debuggers\x64\cdb.exe'
if (!(Test-Path -LiteralPath $debugger -PathType Leaf)) {
    $installer = Join-Path $env:RUNNER_TEMP 'beyond-gate-startup-sdksetup.exe'
    try {
        Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/?linkid=2382321' -OutFile $installer
        $expectedHash = 'B0BDBAD38AE74C40CCAE8CDBCF229807627FBA2BC3100BE28301C4BA884F755E'
        if ((Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash -cne $expectedHash) {
            throw 'Pinned debugger installer checksum verification failed.'
        }
        $signature = Get-AuthenticodeSignature -LiteralPath $installer
        if ($signature.Status -ne 'Valid' -or
            $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
            throw 'Debugger installer Microsoft signature verification failed.'
        }
        $setup = Start-Process -FilePath $installer -ArgumentList @(
            '/features', 'OptionId.WindowsDesktopDebuggers', '/quiet', '/norestart', '/ceip', 'off'
        ) -PassThru
        if (!$setup.WaitForExit(180000)) {
            $setup.Kill($true)
            $setup.WaitForExit()
            throw 'Debugger installation exceeded its three-minute limit.'
        }
        if ($setup.ExitCode -notin @(0, 3010)) { throw 'Debugger installation failed.' }
    } finally {
        if (Test-Path -LiteralPath $installer -PathType Leaf) { Remove-Item -LiteralPath $installer }
    }
}
if (!(Test-Path -LiteralPath $debugger -PathType Leaf)) {
    throw 'Required debugger executable is missing after setup.'
}
. (Join-Path $PSScriptRoot 'startup-stack.ps1') -BrowserProcessId 1 -BrowserExecutable fake -RunDirectory fake
$help = [Diagnostics.Process]::new()
$helpStarted = $false
$output = [IO.MemoryStream]::new([byte[]]::new(16384), 0, 16384, $true, $true)
$errors = [IO.MemoryStream]::new([byte[]]::new(4096), 0, 4096, $true, $true)
try {
    $help.StartInfo.FileName = $debugger
    $help.StartInfo.UseShellExecute = $false
    $help.StartInfo.CreateNoWindow = $true
    $help.StartInfo.RedirectStandardOutput = $true
    $help.StartInfo.RedirectStandardError = $true
    foreach ($argument in @('-nosqm', '-noshell', '-netsyms:no', '-?')) {
        $help.StartInfo.ArgumentList.Add($argument)
    }
    $helpStarted = $help.Start()
    if (!$helpStarted) { throw 'Cannot inspect debugger command-line metadata.' }
    $capture = Read-BoundedProbeOutput $help $output $errors 10000
    $text = [Text.Encoding]::UTF8.GetString($output.GetBuffer(), 0, [int]$output.Position)
    $text += "`n" + [Text.Encoding]::UTF8.GetString($errors.GetBuffer(), 0, [int]$errors.Position)
    @{
        startup_debugger_help = $(if ($capture.completed) { 'read' } else { 'timed-out' })
        exit_code = $(if ($capture.completed) { $help.ExitCode } else { $null })
        switches = @([regex]::Matches($text, '(?mi)(?:^|[ \t])(-[a-z?][a-z0-9:]*)') |
            ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique)
        progress = Get-SafeProbeProgress -Text $text
    } | ConvertTo-Json -Compress
} finally {
    if ($helpStarted -and !$help.HasExited) { $help.Kill($true); $help.WaitForExit() }
    $help.Dispose()
    $output.Dispose()
    $errors.Dispose()
}
Write-Output 'Startup debugger is available; network symbol lookup remains disabled.'
