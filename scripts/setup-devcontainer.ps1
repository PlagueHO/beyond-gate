# Install the checkout-selected Rust toolchain and prepare offline workspace dependencies.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$workspaceRoot = Split-Path -Parent $PSScriptRoot
$toolchainFile = Join-Path $workspaceRoot 'rust-toolchain.toml'
$toolchainConfig = Get-Content -LiteralPath $toolchainFile -Raw
$channelMatch = [regex]::Match($toolchainConfig, '(?m)^channel\s*=\s*"([^"]+)"\s*(?:#.*)?$')
$profileMatch = [regex]::Match($toolchainConfig, '(?m)^profile\s*=\s*"([^"]+)"\s*(?:#.*)?$')

if (-not $channelMatch.Success) {
    throw "Cannot find the Rust channel in $toolchainFile."
}

$channel = $channelMatch.Groups[1].Value
$profile = if ($profileMatch.Success) { $profileMatch.Groups[1].Value } else { 'minimal' }
$components = @('rustfmt', 'clippy', 'rust-analyzer', 'rust-src')
$toolchainArguments = @('toolchain', 'install', $channel, '--profile', $profile)
foreach ($component in $components) {
    $toolchainArguments += @('--component', $component)
}

& rustup @toolchainArguments
if ($LASTEXITCODE -ne 0) {
    throw "Could not install the Rust toolchain selected by rust-toolchain.toml ($channel)."
}

Push-Location $workspaceRoot
try {
    & rustup show
    if ($LASTEXITCODE -ne 0) {
        throw 'Could not verify the selected Rust toolchain.'
    }

    & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'cargo.ps1') fetch --locked
    if ($LASTEXITCODE -ne 0) {
        throw 'Could not fetch the locked workspace dependencies.'
    }

    & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'install-doc-tools.ps1')
    if ($LASTEXITCODE -ne 0) {
        throw 'Could not install the pinned Markdown checker.'
    }
}
finally {
    Pop-Location
}
