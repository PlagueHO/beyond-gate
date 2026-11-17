#Requires -Version 7.4
# Check the complete release inventory and final checksums without running binaries.
[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$Directory,
  [Parameter(Mandatory)]
  [ValidatePattern('\A[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?\z')]
  [string]$Version
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path -LiteralPath $Directory).ProviderPath
$archives = @(
  "beyond-gate-v$Version-x86_64-unknown-linux-gnu.tar.gz",
  "beyond-gate-v$Version-x86_64-pc-windows-msvc.zip",
  "beyond-gate-v$Version-aarch64-apple-darwin.tar.gz"
)
$expected = @($archives | ForEach-Object { $_; "$_.sha256" })
$entries = @(Get-ChildItem -LiteralPath $root -Force)
if ($entries.Count -ne $expected.Count -or
    @($entries | Where-Object { $_.PSIsContainer -or $_.LinkType -or $_.Name -cnotin $expected }).Count -ne 0) {
  throw 'Expected exactly three native archives and their three checksum sidecars.'
}
foreach ($archive in $archives) {
  $path = Join-Path $root $archive
  if ((Get-Item -LiteralPath $path).Length -eq 0) { throw 'Release archive is empty.' }
  $line = [IO.File]::ReadAllText("$path.sha256")
  $match = [regex]::Match($line, '\A([0-9a-f]{64})  ([^\r\n]+)\r?\n?\z')
  if (!$match.Success -or $match.Groups[2].Value -cne $archive) {
    throw 'Invalid release SHA-256 sidecar.'
  }
  $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -cne $match.Groups[1].Value) { throw 'Release archive SHA-256 mismatch.' }
}
$expected | ForEach-Object { Join-Path $root $_ }
