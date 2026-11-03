# Installs only the pinned debugger on a disposable GitHub Actions Windows runner.
[CmdletBinding()]
param([ValidateSet('chrome', 'edge')][string]$Browser = 'chrome')

$ErrorActionPreference = 'Stop'

# Verifies public symbol bytes without returning a cache path or raw tool output.
function Assert-PublicStartupSymbol {
    param([string]$Path, [string]$ExpectedHash)

    if ((Get-Item -LiteralPath $Path).Length -gt 134217728 -or
        (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -cne $ExpectedHash) {
        throw 'Public browser symbol checksum or size verification failed.'
    }
}

if ($MyInvocation.InvocationName -eq '.') { return }

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
if ($Browser -eq 'chrome') {
    $curlVersion = & curl.exe --disable --version
    if ($LASTEXITCODE -ne 0 -or $curlVersion[0] -notmatch '^curl (\d+\.\d+\.\d+)' -or
        [version]$Matches[1] -lt [version]'8.4.0') {
        throw 'Bounded public symbol preparation requires curl 8.4.0 or later.'
    }
    $browserCache = Join-Path $env:RUNNER_TEMP 'beyond-gate-startup-chrome-symbols'
    $null = [IO.Directory]::CreateDirectory($browserCache)
    # These public identities were observed after completed Chrome 156.0.8078.80 cases.
    # No installed browser file is read during this network-only preparation.
    foreach ($entry in @(
        @{ file = 'chrome.exe.pdb'; id = '5E4C58B1741C6F834C4C44205044422E1'
           hash = '407BC1B7B5A33DEA7D036488BF36E9D48E5141D35B4ACEBE1E8C17876429757B' },
        @{ file = 'chrome_elf.dll.pdb'; id = '2C1043DE62C22AE14C4C44205044422E1'
           hash = 'B30757F72CEAACD6C073FFCB7947F3BCB709D01170E9E771670709A31072CFE5' }
    )) {
        $path = Join-Path $browserCache $entry.file
        try {
            $url = 'https://chromium-browser-symsrv.commondatastorage.googleapis.com/' +
                "$($entry.file)/$($entry.id)/$($entry.file)"
            & curl.exe --disable --silent --fail --max-time 30 --max-filesize 134217728 --proto '=https' --output $path $url 2>$null
            if ($LASTEXITCODE -ne 0) { throw 'Public browser symbol download failed.' }
            Assert-PublicStartupSymbol -Path $path -ExpectedHash $entry.hash
        } catch {
            if (Test-Path -LiteralPath $path -PathType Leaf) { Remove-Item -LiteralPath $path }
            throw
        }
    }
    Write-Output 'Two pinned public Chrome 156.0.8078.80 symbol files are available; other builds must still match in the debugger.'
}
