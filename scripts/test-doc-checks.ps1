# Verify checker failures with synthetic files in an isolated path with spaces.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
& (Join-Path $PSScriptRoot 'install-doc-tools.ps1') -Check
$fixture = Join-Path ([IO.Path]::GetTempPath()) "Beyond Gate docs $([guid]::NewGuid())"
$suffix = if ($IsWindows) { '.exe' } else { '' }

try {
    New-Item -ItemType Directory -Path (Join-Path $fixture 'scripts'), (Join-Path $fixture 'target/dev-tools/bin'), (Join-Path $fixture 'docs'), (Join-Path $fixture '.github') | Out-Null
    Copy-Item -LiteralPath (Join-Path $root '.rumdl.toml') -Destination $fixture
    foreach ($name in @('check-docs.ps1', 'install-doc-tools.ps1')) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $fixture 'scripts')
    }
    Copy-Item -LiteralPath (Join-Path $root "target/dev-tools/bin/rumdl$suffix") -Destination (Join-Path $fixture 'target/dev-tools/bin')
    $readme = Join-Path $fixture 'README.md'
    $template = Join-Path $fixture '.github/template.md'
    $guide = Join-Path $fixture 'docs/guide.md'
    [IO.File]::WriteAllText($guide, "# Guide`n`n## Target`n")
    $valid = "# Example`n`n[Guide](docs/guide.md#target)`n`n[Example heading](#example)`n`n[External reference](https://offline.invalid/private)`n"
    [IO.File]::WriteAllText($readme, $valid)
    [IO.File]::WriteAllText($template, "# Template`n`n[Guide](../docs/guide.md#target)`n")
    [IO.File]::WriteAllText((Join-Path $fixture 'target/ignored.md'), "# Ignored`n`n[Missing](missing.md)`n")
    $checker = Join-Path $fixture 'scripts/check-docs.ps1'
    & pwsh -NoProfile -File $checker
    if ($LASTEXITCODE -ne 0) { throw 'Valid offline links or ignored build output failed.' }

    $cases = @(
        @{ Name = 'style'; Mode = 'Style'; Text = "# Example`n`nText `n"; Rule = 'MD009' },
        @{ Name = 'missing file'; Mode = 'Links'; Text = "# Example`n`n[Missing](missing.md)`n"; Rule = 'MD057' },
        @{ Name = 'wrong file case'; Mode = 'Links'; Text = "# Example`n`n[Guide](docs/Guide.md#target)`n"; Rule = 'MD057' },
        @{ Name = 'same-file anchor'; Mode = 'Links'; Text = "# Example`n`n[Missing](#missing)`n"; Rule = 'MD051' },
        @{ Name = 'cross-file anchor'; Mode = 'Links'; Text = "# Example`n`n[Missing](docs/guide.md#missing)`n"; Rule = 'MD051' }
    )
    foreach ($case in $cases) {
        [IO.File]::WriteAllText($readme, $case.Text)
        $output = & pwsh -NoProfile -File $checker -Mode $case.Mode 2>&1
        if ($LASTEXITCODE -eq 0 -or ($output -join "`n") -notmatch $case.Rule) {
            $output | Out-Host
            throw "The $($case.Name) check did not fail with $($case.Rule)."
        }
        Write-Host "Rejected $($case.Name)."
    }

    [IO.File]::WriteAllText($readme, $valid)
    [IO.File]::WriteAllText($template, "# Template`n`n[Missing](missing.md)`n")
    $output = & pwsh -NoProfile -File $checker -Mode Links 2>&1
    if ($LASTEXITCODE -eq 0 -or ($output -join "`n") -notmatch 'MD057') {
        throw 'Markdown under .github was not checked.'
    }
    [IO.File]::WriteAllText($template, "# Template`n")

    $installer = Join-Path $fixture 'scripts/install-doc-tools.ps1'
    $installText = [IO.File]::ReadAllText($installer)
    [IO.File]::WriteAllText($installer, $installText.Replace("`$version = '0.3.2'", "`$version = '0.0.0'"))
    & pwsh -NoProfile -File $checker *> $null
    if ($LASTEXITCODE -eq 0) { throw 'A wrong checker version passed.' }
    [IO.File]::WriteAllText($installer, $installText)

    Remove-Item -LiteralPath (Join-Path $fixture "target/dev-tools/bin/rumdl$suffix")
    & pwsh -NoProfile -File $checker *> $null
    if ($LASTEXITCODE -eq 0) { throw 'A missing checker passed.' }
    Write-Host 'Documentation checker success, scope, and failure cases passed.'
}
finally {
    if (Test-Path -LiteralPath $fixture) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
