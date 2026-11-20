# Exercise the real checker with synthetic advisory data and local license changes.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$root = Split-Path -Parent $PSScriptRoot
& (Join-Path $PSScriptRoot 'install-dependency-tools.ps1') -Check
$fixture = Join-Path ([IO.Path]::GetTempPath()) "Beyond Gate dependency checks $([guid]::NewGuid())"
$suffix = if ($IsWindows) { '.exe' } else { '' }
$manifest = Join-Path $fixture 'Cargo.toml'
$config = Join-Path $fixture 'deny.toml'
$checker = Join-Path $fixture 'scripts/check-dependencies.ps1'
$lockfile = Join-Path $fixture 'Cargo.lock'

function Assert-FixtureCheck {
    param(
        [string]$Name,
        [int]$ExitCode,
        [string]$Diagnostic
    )

    $before = Get-FileHash -LiteralPath $lockfile
    $output = & pwsh -NoProfile -File $checker -Offline -ManifestPath $manifest -ConfigPath $config 2>&1
    $actual = $LASTEXITCODE
    $after = Get-FileHash -LiteralPath $lockfile
    if ($actual -ne $ExitCode -or ($output -join "`n") -notmatch $Diagnostic) {
        $output | Out-Host
        throw "$Name returned $actual; expected $ExitCode and diagnostic '$Diagnostic'."
    }
    if ($before.Hash -ne $after.Hash) {
        throw "$Name changed Cargo.lock."
    }
    Write-Host "Passed: $Name."
}

try {
    foreach ($directory in @('scripts', 'target/dev-tools/bin', 'src', 'platform/src')) {
        New-Item -ItemType Directory -Path (Join-Path $fixture $directory) -Force | Out-Null
    }
    foreach ($name in @('cargo.ps1', 'check-dependencies.ps1', 'install-dependency-tools.ps1')) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $fixture 'scripts')
    }
    $binary = Join-Path $fixture "target/dev-tools/bin/cargo-deny$suffix"
    Copy-Item -LiteralPath (Join-Path $root "target/dev-tools/bin/cargo-deny$suffix") -Destination $binary

    # Use the workspace's cached serde only as the subject of an invented advisory.
    $serde = [regex]::Match(
        [IO.File]::ReadAllText((Join-Path $root 'Cargo.lock')),
        '(?m)^name = "serde"\r?\nversion = "([^"]+)"'
    )
    if (-not $serde.Success) {
        throw 'Cannot find the locked serde version for the offline fixture.'
    }
    [IO.File]::WriteAllText($manifest, @"
[package]
name = "policy-fixture"
version = "0.1.0"
edition = "2024"
license = "MIT"
publish = false

[workspace]
exclude = ["platform"]

[features]
probe = ["dep:policy-platform"]

[target.'cfg(target_os = "none")'.dependencies]
policy-platform = { path = "platform", optional = true }

[dev-dependencies]
serde = "=$($serde.Groups[1].Value)"
"@ + "`n")
    $platformManifest = Join-Path $fixture 'platform/Cargo.toml'
    $platformText = @'
[package]
name = "policy-platform"
version = "0.1.0"
edition = "2024"
license = "MIT"
publish = false
'@ + "`n"
    [IO.File]::WriteAllText($platformManifest, $platformText)
    [IO.File]::WriteAllText((Join-Path $fixture 'src/lib.rs'), '')
    [IO.File]::WriteAllText((Join-Path $fixture 'platform/src/lib.rs'), '')
    & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'cargo.ps1') generate-lockfile --offline --manifest-path $manifest
    if ($LASTEXITCODE -ne 0) {
        throw 'Cannot prepare the offline fixture. Fetch the locked workspace dependencies first.'
    }

    $databaseRoot = Join-Path $fixture 'advisory databases'
    # This cache name belongs to the default RustSec URL in pinned cargo-deny 0.22.0.
    $database = Join-Path $databaseRoot 'advisory-db-3157b0e258782691'
    New-Item -ItemType Directory -Path (Join-Path $database 'crates/serde') -Force | Out-Null
    & git init --quiet $database
    if ($LASTEXITCODE -ne 0) { throw 'Cannot prepare the synthetic advisory repository.' }
    $fetchHead = Join-Path $database '.git/FETCH_HEAD'
    [IO.File]::WriteAllText($fetchHead, '')
    $advisoryPath = Join-Path $database 'crates/serde/RUSTSEC-2000-9999.md'
    $advisory = @'
```toml
[advisory]
id = "RUSTSEC-2000-9999"
package = "serde"
date = "2000-01-01"
url = "https://example.invalid/synthetic-advisory"

[versions]
patched = [">=0.0.0"]
```

# Synthetic dependency policy test

This invented advisory is used only by an offline test.
'@ + "`n"
    [IO.File]::WriteAllText($advisoryPath, $advisory)
    $policy = [IO.File]::ReadAllText((Join-Path $root 'deny.toml'))
    # Production exceptions do not apply to the isolated synthetic graph.
    $policy = [regex]::Replace($policy, '(?ms)^ignore = \[\r?\n.*?^\]', 'ignore = []')
    $policy = $policy.Replace(
        '[advisories]',
        "[advisories]`ndb-path = '$databaseRoot'`ndisable-yank-checking = true"
    )
    [IO.File]::WriteAllText($config, $policy)

    Assert-FixtureCheck 'valid offline graph in a path with spaces' 0 'licenses ok'
    [IO.File]::WriteAllText($advisoryPath, $advisory.Replace('>=0.0.0', '>=999.0.0'))
    Assert-FixtureCheck 'advisory in a test-only dependency' 1 'error\[vulnerability\]'
    $tomorrow = [DateTime]::UtcNow.AddDays(1).ToString('yyyy-MM-dd')
    $temporary = 'ignore = [{ id = "RUSTSEC-2000-9999", reason = "Offline fixture only", expiry = "' + $tomorrow + '" }]'
    [IO.File]::WriteAllText($config, $policy.Replace('ignore = []', $temporary))
    Assert-FixtureCheck 'unexpired synthetic advisory exception' 0 'licenses ok'
    $expired = 'ignore = [{ id = "RUSTSEC-2000-9999", reason = "Offline fixture only", expiry = "2000-01-01" }]'
    [IO.File]::WriteAllText($config, $policy.Replace('ignore = []', $expired))
    Assert-FixtureCheck 'expired advisory exception' 1 'error\[vulnerability\]'
    [IO.File]::WriteAllText($config, $policy)
    [IO.File]::WriteAllText($advisoryPath, $advisory)

    [IO.File]::WriteAllText($platformManifest, $platformText.Replace('license = "MIT"', 'license = "GPL-3.0-only"'))
    Assert-FixtureCheck 'disallowed license in an optional foreign-target dependency' 4 'error\[rejected\]'
    [IO.File]::WriteAllText($platformManifest, $platformText.Replace('license = "MIT"', ''))
    Assert-FixtureCheck 'missing license in a foreign-target dependency' 4 'error\[unlicensed\]'
    [IO.File]::WriteAllText($platformManifest, $platformText)

    (Get-Item -LiteralPath $fetchHead).LastWriteTimeUtc = [DateTime]::UtcNow.AddDays(-7).AddMinutes(-1)
    Assert-FixtureCheck 'database older than seven days' 1 'repository is stale'
    (Get-Item -LiteralPath $fetchHead).LastWriteTimeUtc = [DateTime]::UtcNow.AddDays(-7).AddMinutes(1)
    Assert-FixtureCheck 'database newer than seven days' 0 'licenses ok'
    $missingDatabase = Join-Path $fixture 'missing database'
    [IO.File]::WriteAllText($config, $policy.Replace($databaseRoot, $missingDatabase))
    Assert-FixtureCheck 'missing offline database' 1 'failed to'
    [IO.File]::WriteAllText($config, $policy)

    $installer = Join-Path $fixture 'scripts/install-dependency-tools.ps1'
    $installText = [IO.File]::ReadAllText($installer)
    $pin = [regex]::Match($installText, '(?m)^\$version = ''[^'']+''$')
    if (-not $pin.Success) { throw 'Cannot locate the checker version pin.' }
    [IO.File]::WriteAllText($installer, $installText.Replace($pin.Value, "`$version = '0.0.0'"))
    Assert-FixtureCheck 'wrong checker version' 1 'cargo-deny 0\.0\.0 is required'
    [IO.File]::WriteAllText($installer, $installText)
    Remove-Item -LiteralPath $binary
    Assert-FixtureCheck 'missing checker' 1 'cargo-deny .+ is required'
    Write-Host 'Dependency checker scope, freshness, lockfile, and failure cases passed.'
}
finally {
    if (Test-Path -LiteralPath $fixture) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
exit 0
