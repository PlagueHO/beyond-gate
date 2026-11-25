# Check the built runtime image without network, host credentials, or live reads.
[CmdletBinding()]
param(
    [string]$Image = 'beyond-gate:local'
)
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'developer-checks.psm1') -Force
$engine = Invoke-DeveloperProcess docker @('info', '--format', '{{.OSType}}')
if ($engine.ExitCode -ne 0) { throw "Docker engine check failed: $($engine.Stderr)" }
if ($engine.Stdout.Trim() -ne 'linux') { throw 'A running Linux Docker engine is required.' }
$configuration = Invoke-DeveloperProcess docker @('image', 'inspect', $Image)
if ($configuration.ExitCode -ne 0) { throw "Build $Image first: $($configuration.Stderr)" }
$imageInfo = @($configuration.Stdout | ConvertFrom-Json)[0]
if ($imageInfo.Config.User -ne '10001:10001' -or $imageInfo.Config.ExposedPorts -or
    @($imageInfo.Config.Env | Where-Object { $_ -match '^(BEYOND_GATE_|GH_TOKEN=|COPILOT_GH_ACCOUNT_)' }).Count) {
    throw 'Runtime image has an unexpected user, exposed port, or credential environment.'
}
$cases = @(
    @{ Binary = 'beyond-gate'; Arguments = @('--help'); ExitCode = 0; Pattern = 'Usage:' },
    @{ Binary = 'beyond-gate-mcp'; Arguments = @('--help'); ExitCode = 0; Pattern = 'Usage:' },
    @{ Binary = 'beyond-gate'; Arguments = @('--invented-workflow-option'); ExitCode = 2 },
    @{ Binary = 'beyond-gate-mcp'; Arguments = @('--invented-workflow-option'); ExitCode = 2 },
    @{ Binary = 'beyond-gate'; Arguments = @('mcp', 'start'); ExitCode = 0; Protocol = $true },
    @{ Binary = 'beyond-gate-mcp'; Arguments = @(); ExitCode = 0; Protocol = $true }
)
foreach ($case in $cases) {
    $name = "beyond-gate-check-$([guid]::NewGuid().ToString('N'))"
    try {
        $arguments = @('run', '--rm', '-i', '--name', $name, '--network', 'none',
            '--read-only', '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges',
            '--entrypoint', $case.Binary, $Image) + $case.Arguments
        $inputText = if ($case.Protocol) { Get-DeveloperProtocolInput } else { '' }
        $result = Invoke-DeveloperProcess docker $arguments -InputText $inputText
        if ($result.ExitCode -ne $case.ExitCode) { throw "Container exit status failed: $($result.Stderr)" }
        if ($case.Protocol) {
            Assert-DeveloperProtocol $result
            if ($result.Stderr) { throw "Unexpected MCP diagnostics: $($result.Stderr)" }
        }
        elseif ($case.Pattern) {
            if ($result.Stdout -notmatch $case.Pattern -or $result.Stderr) { throw 'Container help output failed.' }
        }
        elseif ($result.Stdout -or -not $result.Stderr) {
            throw 'Container argument errors did not use stderr only.'
        }
        $remaining = Invoke-DeveloperProcess docker @('ps', '-aq', '--filter', "name=^/$name$")
        if ($remaining.ExitCode -ne 0 -or $remaining.Stdout.Trim()) { throw 'Owned container did not shut down and remove itself.' }
    }
    finally {
        $remaining = Invoke-DeveloperProcess docker @('ps', '-aq', '--filter', "name=^/$name$")
        if ($remaining.ExitCode -ne 0) { throw "Cannot inspect owned container cleanup: $($remaining.Stderr)" }
        if ($remaining.Stdout.Trim()) {
            $cleanup = Invoke-DeveloperProcess docker @('rm', '--force', $name)
            if ($cleanup.ExitCode -ne 0) { throw "Cannot remove owned container: $($cleanup.Stderr)" }
        }
    }
}
Write-Host 'Container help, failure status, MCP discovery, stdin closure, and cleanup passed.'
