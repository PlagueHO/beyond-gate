# Check repository Markdown without network access or generated build output.
[CmdletBinding()]
param(
    [ValidateSet('All', 'Style', 'Links')]
    [string]$Mode = 'All'
)

$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot 'install-doc-tools.ps1') -Check
$root = Split-Path -Parent $PSScriptRoot
$suffix = if ($IsWindows) { '.exe' } else { '' }
$binary = Join-Path $root "target/dev-tools/bin/rumdl$suffix"
$arguments = @('check', '--config', (Join-Path $root '.rumdl.toml'))
if ($Mode -eq 'Links') {
    $arguments += @('--enable', 'MD051,MD057')
}
elseif ($Mode -eq 'Style') {
    $arguments += @('--disable', 'MD051,MD057')
}
$arguments += '.'

Push-Location $root
try {
    & $binary @arguments
    $checkerExitCode = $LASTEXITCODE
}
finally {
    Pop-Location
}
exit $checkerExitCode
