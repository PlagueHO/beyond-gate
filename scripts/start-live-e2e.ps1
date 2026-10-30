#Requires -Version 7.4
# Build without credentials, then run the same suite as the manual workflow.
[CmdletBinding()]
param(
    [ValidateSet('full', 'contracts')][string]$Selection = 'full',
    [switch]$ApproveNativeStorage,
    [string]$ReportPath
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$target = Join-Path $root 'target\manual-e2e'
& (Join-Path $PSScriptRoot 'cargo.ps1') build --workspace --locked `
    --features 'beyond-gate-cli/e2e-isolation,beyond-gate-mcp/e2e-isolation' --target-dir $target
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& (Join-Path $PSScriptRoot 'cargo.ps1') build -p beyond-gate-core --example e2e-session `
    --features e2e-isolation --locked --target-dir $target
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& (Join-Path $PSScriptRoot 'test-e2e.ps1') -BinaryDirectory (Join-Path $target 'debug') `
    -Selection $Selection -ApproveNativeStorage:$ApproveNativeStorage -ReportPath $ReportPath
exit $LASTEXITCODE
