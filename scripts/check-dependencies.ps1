# Apply the same locked dependency policy locally and in CI.
[CmdletBinding()]
param(
    [switch]$Offline,
    [string]$ManifestPath = (Join-Path (Split-Path -Parent $PSScriptRoot) 'Cargo.toml'),
    [string]$ConfigPath = (Join-Path (Split-Path -Parent $PSScriptRoot) 'deny.toml')
)

$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot 'install-dependency-tools.ps1') -Check
$root = Split-Path -Parent $PSScriptRoot
$arguments = @(
    'deny', '--workspace', '--locked',
    '--manifest-path', [IO.Path]::GetFullPath($ManifestPath),
    '--config', [IO.Path]::GetFullPath($ConfigPath)
)
if ($Offline) {
    $arguments += '--offline'
}
$arguments += @('check', 'advisories', 'licenses')

$previousPath = $env:PATH
try {
    $env:PATH = (Join-Path $root 'target/dev-tools/bin') + [IO.Path]::PathSeparator + $previousPath
    & (Join-Path $PSScriptRoot 'cargo.ps1') @arguments
    $checkerExitCode = $LASTEXITCODE
}
finally {
    $env:PATH = $previousPath
}
exit $checkerExitCode
