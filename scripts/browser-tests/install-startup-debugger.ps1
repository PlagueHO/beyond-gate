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
Write-Output 'Startup debugger is available; network symbol lookup remains disabled.'
