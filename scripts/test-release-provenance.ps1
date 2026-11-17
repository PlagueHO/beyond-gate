#Requires -Version 7.4
# Test local verification policy and inventory checks without credentials or network.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$temporary = Join-Path ([IO.Path]::GetTempPath()) "Beyond Gate provenance $([guid]::NewGuid().ToString('N'))"
$verifier = Join-Path $PSScriptRoot 'verify-release-provenance.ps1'
$inventory = Join-Path $PSScriptRoot 'test-release-assets.ps1'
$commit = 'a' * 40
$ref = 'refs/tags/v0.1.0'
$state = @{ Arguments = @(); ExitCode = 0; Calls = 0; Unavailable = $false }

# Replace gh only in this test's scope; never call the real service.
function gh {
  $state.Arguments = @($args)
  $state.Calls++
  if ($state.Unavailable) { throw 'Synthetic verification service unavailable.' }
  Set-Variable -Name LASTEXITCODE -Value $state.ExitCode -Scope 1
  if ($state.ExitCode -eq 0) { '{"synthetic":true}' }
}

# Require the intended rejection rather than an unrelated test error.
function Assert-ProvenanceRejection {
  param([scriptblock]$Action, [string]$Message)
  try { & $Action | Out-Null }
  catch {
    if (!$_.ToString().Contains($Message)) { throw }
    return
  }
  throw "Expected rejection: $Message"
}

# Rebuild a sidecar only when a synthetic fixture intentionally changes.
function Write-FixtureChecksum {
  param([string]$Path)
  $hash = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
  [IO.File]::WriteAllText("$Path.sha256", "$hash  $([IO.Path]::GetFileName($Path))`n")
}

try {
  New-Item -ItemType Directory -Path $temporary | Out-Null
  $artifact = Join-Path $temporary 'candidate with spaces.zip'
  [IO.File]::WriteAllText($artifact, 'Synthetic archive bytes, not a native package.')
  $output = & $verifier -Artifact $artifact -SourceCommit $commit -SourceRef $ref
  $expected = @(
    'attestation', 'verify', $artifact, '--repo', 'PlagueHO/beyond-gate',
    '--signer-workflow', 'PlagueHO/beyond-gate/.github/workflows/release.yml',
    '--source-digest', $commit, '--source-ref', $ref, '--predicate-type',
    'https://slsa.dev/provenance/v1', '--deny-self-hosted-runners', '--format', 'json'
  )
  if (($state.Arguments -join "`n") -cne ($expected -join "`n") -or $output -cne '{"synthetic":true}') {
    throw 'Verification did not enforce the exact artifact and identity policy.'
  }
  $null = & $verifier $artifact $commit 'refs/heads/main'
  $expected[10] = 'refs/heads/main'
  if (($state.Arguments -join "`n") -cne ($expected -join "`n")) {
    throw 'Manual candidate verification changed the expected source branch.'
  }
  $state.ExitCode = 1
  Assert-ProvenanceRejection { & $verifier $artifact $commit $ref } 'Release provenance verification failed'
  $state.Unavailable = $true
  Assert-ProvenanceRejection { & $verifier $artifact $commit $ref } 'Synthetic verification service unavailable'
  $state.Unavailable = $false
  $state.ExitCode = 0
  $calls = $state.Calls
  Assert-ProvenanceRejection { & $verifier (Join-Path $temporary 'missing.zip') $commit $ref } 'Artifact file is missing'
  $empty = Join-Path $temporary 'empty.zip'
  [IO.File]::WriteAllText($empty, '')
  Assert-ProvenanceRejection { & $verifier $empty $commit $ref } 'Artifact file is empty'
  Assert-ProvenanceRejection { & $verifier $artifact 'not-a-commit' $ref } 'SourceCommit'
  Assert-ProvenanceRejection { & $verifier $artifact "$commit`n" $ref } 'SourceCommit'
  Assert-ProvenanceRejection { & $verifier $artifact $commit 'refs/pull/1/merge' } 'SourceRef'
  Assert-ProvenanceRejection { & $verifier $artifact $commit "$ref`n" } 'SourceRef'
  if ($state.Calls -ne $calls) { throw 'Invalid inputs reached the verification service.' }

  # Hide commands only for this check, so a missing gh cannot look successful.
  function Get-Command { throw 'Synthetic gh command missing.' }
  try {
    Assert-ProvenanceRejection { & $verifier $artifact $commit $ref } 'Synthetic gh command missing'
  } finally { Remove-Item Function:\Get-Command }

  $dist = Join-Path $temporary 'release assets'
  New-Item -ItemType Directory -Path $dist | Out-Null
  $names = @(
    'beyond-gate-v0.1.0-x86_64-unknown-linux-gnu.tar.gz',
    'beyond-gate-v0.1.0-x86_64-pc-windows-msvc.zip',
    'beyond-gate-v0.1.0-aarch64-apple-darwin.tar.gz'
  )
  foreach ($name in $names) {
    $path = Join-Path $dist $name
    [IO.File]::WriteAllText($path, "Synthetic $name")
    Write-FixtureChecksum $path
  }
  $assets = @(& $inventory $dist '0.1.0')
  if ($assets.Count -ne 6 -or @($assets | Where-Object { !(Test-Path -LiteralPath $_ -PathType Leaf) }).Count) {
    throw 'Valid release inventory did not return six exact files.'
  }
  $extra = Join-Path $dist 'unexpected.txt'
  [IO.File]::WriteAllText($extra, 'Unexpected')
  Assert-ProvenanceRejection { & $inventory $dist '0.1.0' } 'Expected exactly'
  Remove-Item -LiteralPath $extra
  New-Item -ItemType Directory -Path $extra | Out-Null
  Assert-ProvenanceRejection { & $inventory $dist '0.1.0' } 'Expected exactly'
  Remove-Item -LiteralPath $extra
  $path = Join-Path $dist $names[0]
  $original = [IO.File]::ReadAllText($path)
  [IO.File]::AppendAllText($path, 'tampered')
  Assert-ProvenanceRejection { & $inventory $dist '0.1.0' } 'SHA-256 mismatch'
  [IO.File]::WriteAllText($path, '')
  Assert-ProvenanceRejection { & $inventory $dist '0.1.0' } 'Release archive is empty'
  [IO.File]::WriteAllText($path, $original)
  Write-FixtureChecksum $path
  $sidecar = [IO.File]::ReadAllText("$path.sha256")
  [IO.File]::WriteAllText("$path.sha256", $sidecar.Replace($names[0], '../different.tar.gz'))
  Assert-ProvenanceRejection { & $inventory $dist '0.1.0' } 'Invalid release SHA-256 sidecar'
  [IO.File]::WriteAllText("$path.sha256", 'invalid')
  Assert-ProvenanceRejection { & $inventory $dist '0.1.0' } 'Invalid release SHA-256 sidecar'
  Remove-Item -LiteralPath "$path.sha256"
  Assert-ProvenanceRejection { & $inventory $dist '0.1.0' } 'Expected exactly'
  Write-FixtureChecksum $path
  Assert-ProvenanceRejection { & $inventory $dist '0.2.0' } 'Expected exactly'
  Assert-ProvenanceRejection { & $inventory $dist "0.1.0`n" } 'Version'
  $null = & $inventory $dist '0.1.0'
  Write-Output 'Synthetic provenance policy, failure propagation, and final asset checks passed. No real signature was verified.'
}
finally {
  if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
}
exit 0
