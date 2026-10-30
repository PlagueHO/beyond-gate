# Install the pinned development-only Markdown checker, or verify its version.
[CmdletBinding()]
param(
    [switch]$Check
)

$ErrorActionPreference = 'Stop'
$version = '0.3.2'
$root = Join-Path (Split-Path -Parent $PSScriptRoot) 'target/dev-tools'
$suffix = if ($IsWindows) { '.exe' } else { '' }
$binary = Join-Path $root "bin/rumdl$suffix"

if (Test-Path -LiteralPath $binary) {
    $actual = & $binary --version
    if ($LASTEXITCODE -ne 0) {
        throw 'Cannot read the installed rumdl version.'
    }
    if ($actual -ceq "rumdl $version") {
        return
    }
}

if ($Check) {
    throw "rumdl $version is required. Run pwsh -NoProfile -File ./scripts/install-doc-tools.ps1."
}

& (Join-Path $PSScriptRoot 'cargo.ps1') install rumdl --version "=$version" --locked --root $root --force
if ($LASTEXITCODE -ne 0) {
    throw "rumdl $version installation failed."
}

$actual = & $binary --version
if ($LASTEXITCODE -ne 0 -or $actual -cne "rumdl $version") {
    throw "The installed checker is not rumdl $version."
}
