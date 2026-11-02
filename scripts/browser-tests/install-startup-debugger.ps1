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
$symbolCache = Join-Path $env:RUNNER_TEMP 'beyond-gate-startup-ntdll-symbols'
$null = [IO.Directory]::CreateDirectory($symbolCache)
$symbols = [Diagnostics.Process]::new()
$symbolsStarted = $false
$output = [IO.MemoryStream]::new([byte[]]::new(16384), 0, 16384, $true, $true)
$errors = [IO.MemoryStream]::new([byte[]]::new(4096), 0, 4096, $true, $true)
try {
    $symbols.StartInfo.FileName = Join-Path (Split-Path $debugger) 'symchk.exe'
    $symbols.StartInfo.UseShellExecute = $false
    $symbols.StartInfo.CreateNoWindow = $true
    $symbols.StartInfo.RedirectStandardOutput = $true
    $symbols.StartInfo.RedirectStandardError = $true
    foreach ($argument in @(
        (Join-Path $env:SystemRoot 'System32\ntdll.dll'), '/s',
        "srv*$symbolCache*https://msdl.microsoft.com/download/symbols"
    )) {
        $symbols.StartInfo.ArgumentList.Add($argument)
    }
    $symbolsStarted = $symbols.Start()
    if (!$symbolsStarted) { throw 'Cannot prepare the Windows loader symbols.' }
    $capture = Read-BoundedProbeOutput $symbols $output $errors 30000
    if (!$capture.completed) { throw 'Windows loader symbol preparation exceeded its thirty-second limit.' }
    if ($symbols.ExitCode -ne 0) { throw 'Windows loader symbol verification failed.' }
    $files = @(Get-ChildItem -LiteralPath $symbolCache -Filter ntdll.pdb -File -Recurse)
    if ($files.Count -ne 1) { throw 'Expected one verified Windows loader symbol file.' }
} finally {
    if ($symbolsStarted -and !$symbols.HasExited) { $symbols.Kill($true); $symbols.WaitForExit() }
    $symbols.Dispose()
    $output.Dispose()
    $errors.Dispose()
}
Write-Output 'Startup debugger and verified Windows loader symbols are available; probe network symbol lookup remains disabled.'
