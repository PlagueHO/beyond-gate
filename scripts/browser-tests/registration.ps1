# Own only the selected test's unique Windows native-host registration and process.
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet('CheckAbsent', 'Register', 'Remove', 'StopHost')]
    [string]$Action,
    [Parameter(Mandatory)]
    [ValidateSet('chrome', 'edge')]
    [string]$Browser,
    [Parameter(Mandatory)]
    [ValidatePattern('^com\.beyondgate\.synthetic\.[a-f0-9]{32}$')]
    [string]$HostName,
    [Parameter(Mandatory)]
    [string]$RunDirectory
)

$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'This fixture registration supports Windows only.' }
$root = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\target\browser-feasibility'))
$directory = Get-Item -LiteralPath $RunDirectory
if ($directory.Parent.FullName -ne $root -or
    $directory.Name -notmatch '^(chrome|edge)-[a-zA-Z0-9]+$' -or
    ($directory.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
    throw 'The run directory is not an owned fixture directory.'
}
$manifestPath = Join-Path $directory.FullName 'native-host.json'
$binaryPath = Join-Path $directory.FullName 'host.exe'

if ($Action -eq 'StopHost') {
    $reportPath = Join-Path $directory.FullName 'host-report.json'
    if (-not (Test-Path -LiteralPath $reportPath -PathType Leaf)) {
        @{ host_cleanup = 'not-started' } | ConvertTo-Json -Compress
        return
    }
    $report = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
    if ($report.pid -isnot [long] -and $report.pid -isnot [int]) {
        throw 'The owned host report has no valid process identifier.'
    }
    if ($report.pid -le 0 -or $report.pid -gt [int]::MaxValue) {
        throw 'The owned host process identifier is out of range.'
    }
    try {
        $process = [System.Diagnostics.Process]::GetProcessById([int]$report.pid)
    }
    catch [System.ArgumentException] {
        @{ host_cleanup = 'exited' } | ConvertTo-Json -Compress
        return
    }
    try {
        if ($process.HasExited) {
            @{ host_cleanup = 'exited' } | ConvertTo-Json -Compress
            return
        }
        $null = $process.Handle
        if ($process.MainModule.FileName -ne $binaryPath -or
            $process.StartTime.ToUniversalTime() -lt $directory.CreationTimeUtc) {
            throw 'The process does not match the owned fixture executable.'
        }
        if ($process.WaitForExit(2000)) {
            @{ host_cleanup = 'exited' } | ConvertTo-Json -Compress
            return
        }
        Stop-Process -Id $process.Id -ErrorAction Stop
        if (-not $process.WaitForExit(5000)) { throw 'The owned host did not stop.' }
        @{ host_cleanup = 'stopped' } | ConvertTo-Json -Compress
    }
    finally {
        $process.Dispose()
    }
    return
}

$parent = if ($Browser -eq 'chrome') {
    'HKCU:\Software\Google\Chrome\NativeMessagingHosts'
}
else {
    'HKCU:\Software\Microsoft\Edge\NativeMessagingHosts'
}
$key = Join-Path $parent $HostName
if ($Action -eq 'CheckAbsent') {
    if (Test-Path -LiteralPath $key) { throw 'Do not contact an existing native-host key.' }
    @{ registration = 'absent'; browser = $Browser } | ConvertTo-Json -Compress
    return
}
if ($Action -eq 'Register') {
    if (Test-Path -LiteralPath $key) { throw 'Do not overwrite an existing native-host key.' }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.name -ne $HostName -or $manifest.type -ne 'stdio' -or
        $manifest.path -ne $binaryPath -or
        -not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
        throw 'The synthetic native-host manifest is invalid.'
    }
    if (-not (Test-Path -LiteralPath $parent)) {
        $parentKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($parent.Substring('HKCU:\'.Length))
        if ($null -eq $parentKey) { throw 'The native-host namespace could not be created.' }
        $parentKey.Dispose()
    }
    New-Item -Path $key | Out-Null
    try {
        New-ItemProperty -LiteralPath $key -Name 'BeyondGateSyntheticOwner' `
            -Value $directory.FullName -PropertyType String | Out-Null
        Set-Item -LiteralPath $key -Value $manifestPath
    }
    catch {
        Remove-Item -LiteralPath $key -Force -ErrorAction Stop
        throw
    }
    @{ registration = 'created'; browser = $Browser } | ConvertTo-Json -Compress
}
else {
    if (-not (Test-Path -LiteralPath $key)) {
        @{ registration = 'absent'; browser = $Browser } | ConvertTo-Json -Compress
        return
    }
    $properties = Get-ItemProperty -LiteralPath $key
    if ($properties.BeyondGateSyntheticOwner -ne $directory.FullName -or
        $properties.'(default)' -ne $manifestPath) {
        throw 'Do not remove a native-host key that the fixture does not own.'
    }
    Remove-Item -LiteralPath $key -Force -ErrorAction Stop
    @{ registration = 'removed'; browser = $Browser } | ConvertTo-Json -Compress
}
