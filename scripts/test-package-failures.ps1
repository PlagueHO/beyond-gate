#Requires -Version 7.4
# Regression checks for the archive verifier, using copies of one native package.
[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$Archive,
  [Parameter(Mandatory)][string]$Version,
  [Parameter(Mandatory)][string]$Target
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$archivePath = (Resolve-Path -LiteralPath $Archive).Path
$checker = Join-Path $PSScriptRoot 'test-package.ps1'
$temporary = Join-Path ([System.IO.Path]::GetTempPath()) "Beyond Gate rejection $([guid]::NewGuid().ToString('N'))"
$extension = if ($IsWindows) { '.zip' } else { '.tar.gz' }
$testArchive = Join-Path $temporary ([System.IO.Path]::GetFileName($archivePath))
$checksum = "$testArchive.sha256"

# Produce a correctly formatted sidecar for the current test bytes.
function Write-TestChecksum {
  param([string]$Path)
  $hash = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
  [System.IO.File]::WriteAllText("$Path.sha256", "$hash  $([System.IO.Path]::GetFileName($Path))`n")
}

# A rejected fixture must fail for the intended reason, not an unrelated exception.
function Assert-RejectedPackage {
  param([string]$Path, [string]$Expected, [string]$ExpectedVersion = $Version, [string]$ExpectedTarget = $Target)
  try {
    & $checker -Archive $Path -Checksum "$Path.sha256" -Version $ExpectedVersion -Target $ExpectedTarget
  }
  catch {
    if (!$_.Exception.Message.Contains($Expected)) {
      throw "Expected rejection '$Expected', received: $($_.Exception.Message)"
    }
    Write-Output "Verified rejection: $Expected"
    return
  }
  throw "Invalid package was accepted: $Expected"
}

# Rebuild a fixture with the same flat layout and native permissions as packaging.
function Write-TestArchive {
  param([string]$Directory)
  Remove-Item -LiteralPath $testArchive
  if ($IsWindows) {
    [System.IO.Compression.ZipFile]::CreateFromDirectory($Directory, $testArchive)
  } else {
    & tar -czf $testArchive -C $Directory .
    if ($LASTEXITCODE -ne 0) { throw 'Cannot create rejection fixture.' }
  }
  Write-TestChecksum $testArchive
}

try {
  New-Item -ItemType Directory -Path $temporary | Out-Null
  Copy-Item -LiteralPath $archivePath -Destination $testArchive
  Write-TestChecksum $testArchive
  $validChecksum = [System.IO.File]::ReadAllText($checksum)
  [System.IO.File]::WriteAllText($checksum, 'invalid checksum')
  Assert-RejectedPackage $testArchive 'Invalid SHA-256 sidecar'
  [System.IO.File]::WriteAllText($checksum, $validChecksum.Replace([System.IO.Path]::GetFileName($testArchive), 'different.zip'))
  Assert-RejectedPackage $testArchive 'Invalid SHA-256 sidecar'
  [System.IO.File]::WriteAllText($checksum, $validChecksum)
  $stream = [System.IO.File]::Open($testArchive, [System.IO.FileMode]::Open, [System.IO.FileAccess]::ReadWrite)
  try {
    $stream.Position = $stream.Length - 1
    $byte = $stream.ReadByte()
    $stream.Position--
    $stream.WriteByte($byte -bxor 1)
  }
  finally { $stream.Dispose() }
  Assert-RejectedPackage $testArchive 'Archive SHA-256 mismatch'
  Copy-Item -LiteralPath $archivePath -Destination $testArchive -Force
  Write-TestChecksum $testArchive

  $wrongVersion = "$Version-rejection"
  $wrongVersionPath = Join-Path $temporary "beyond-gate-v$wrongVersion-$Target$extension"
  Copy-Item -LiteralPath $archivePath -Destination $wrongVersionPath
  Write-TestChecksum $wrongVersionPath
  Assert-RejectedPackage $wrongVersionPath 'version contract failed' $wrongVersion
  $wrongTarget = if ($IsWindows) { 'x86_64-unknown-linux-gnu' } else { 'x86_64-pc-windows-msvc' }
  $wrongTargetPath = Join-Path $temporary "beyond-gate-v$Version-$wrongTarget$extension"
  Copy-Item -LiteralPath $archivePath -Destination $wrongTargetPath
  Write-TestChecksum $wrongTargetPath
  Assert-RejectedPackage $wrongTargetPath 'Package target does not match' $Version $wrongTarget

  $contents = Join-Path $temporary 'fixture contents'
  New-Item -ItemType Directory -Path $contents | Out-Null
  if ($IsWindows) {
    [System.IO.Compression.ZipFile]::ExtractToDirectory($archivePath, $contents)
  } else {
    & tar -xzf $archivePath -C $contents
    if ($LASTEXITCODE -ne 0) { throw 'Cannot extract rejection fixture.' }
  }
  $guide = Join-Path $contents 'installation.md'
  $guideText = [System.IO.File]::ReadAllText($guide)
  Remove-Item -LiteralPath $guide
  Write-TestArchive $contents
  Assert-RejectedPackage $testArchive 'Package is missing required files'
  [System.IO.File]::WriteAllText($guide, $guideText)
  [System.IO.File]::WriteAllText((Join-Path $contents 'unexpected.txt'), 'synthetic')
  Write-TestArchive $contents
  Assert-RejectedPackage $testArchive 'Unexpected or unsafe archive member'
  Remove-Item -LiteralPath (Join-Path $contents 'unexpected.txt')
  [System.IO.File]::WriteAllText($guide, '')
  Write-TestArchive $contents
  Assert-RejectedPackage $testArchive 'Package files must not be empty'
}
finally {
  if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
}
