#Requires -Version 7.4
# Verify exact artifact bytes against the approved repository and build identity.
[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$Artifact,
  [Parameter(Mandatory)][ValidatePattern('\A[0-9a-f]{40}\z')][string]$SourceCommit,
  [Parameter(Mandatory)][ValidatePattern('\Arefs/(heads|tags)/[^\s\x00-\x1f\x7f]+\z')][string]$SourceRef
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (!(Test-Path -LiteralPath $Artifact -PathType Leaf)) { throw 'Artifact file is missing.' }
$path = (Resolve-Path -LiteralPath $Artifact).ProviderPath
if ((Get-Item -LiteralPath $path).Length -eq 0) { throw 'Artifact file is empty.' }
$null = Get-Command gh -ErrorAction Stop
$LASTEXITCODE = 0
& gh attestation verify $path --repo PlagueHO/beyond-gate `
  --signer-workflow PlagueHO/beyond-gate/.github/workflows/release.yml `
  --source-digest $SourceCommit --source-ref $SourceRef `
  --predicate-type https://slsa.dev/provenance/v1 --deny-self-hosted-runners --format json
if ($LASTEXITCODE -ne 0) { throw 'Release provenance verification failed. Do not use the artifact.' }
