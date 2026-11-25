# Build the sibling binaries required by the focused MCP protocol process checks.
$ErrorActionPreference = 'Stop'
$wrapper = Join-Path $PSScriptRoot 'cargo.ps1'
& pwsh -NoProfile -File $wrapper build -p beyond-gate-cli -p beyond-gate-mcp --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& pwsh -NoProfile -File $wrapper test -p beyond-gate-mcp --test cli --locked
exit $LASTEXITCODE
