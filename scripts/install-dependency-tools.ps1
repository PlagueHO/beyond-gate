# Install the pinned development-only dependency checker, or verify its version.
[CmdletBinding()]
param(
    [switch]$Check
)

$ErrorActionPreference = 'Stop'
$version = '0.22.0'
$root = Join-Path (Split-Path -Parent $PSScriptRoot) 'target/dev-tools'
$suffix = if ($IsWindows) { '.exe' } else { '' }
$binary = Join-Path $root "bin/cargo-deny$suffix"

if (Test-Path -LiteralPath $binary) {
    $actual = & $binary --version
    if ($LASTEXITCODE -ne 0) {
        throw 'Cannot read the installed cargo-deny version.'
    }
    if ($actual -ceq "cargo-deny $version") {
        return
    }
}

if ($Check) {
    throw "cargo-deny $version is required. Run pwsh -NoProfile -File ./scripts/install-dependency-tools.ps1."
}

& (Join-Path $PSScriptRoot 'cargo.ps1') install cargo-deny --version "=$version" --locked --root $root --force
if ($LASTEXITCODE -ne 0) {
    throw "cargo-deny $version installation failed."
}

$actual = & $binary --version
if ($LASTEXITCODE -ne 0 -or $actual -cne "cargo-deny $version") {
    throw "The installed checker is not cargo-deny $version."
}
