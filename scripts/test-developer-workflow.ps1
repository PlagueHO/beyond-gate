# Check native launch streams, argument forwarding, failures, and task menu parity.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'developer-checks.psm1') -Force
$root = Split-Path -Parent $PSScriptRoot
$fixture = Join-Path ([IO.Path]::GetTempPath()) "Beyond Gate workflow $([guid]::NewGuid())"

try {
    New-Item -ItemType Directory -Path (Join-Path $fixture 'scripts') | Out-Null
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run.ps1') -Destination (Join-Path $fixture 'scripts')
    $fakeCargo = @'
if (($args -join ' ') -ne 'build --workspace --locked --message-format=json') { exit 31 }
[Console]::Out.WriteLine('synthetic developer-shell diagnostic')
[Console]::Error.WriteLine('synthetic build diagnostic')
if ($env:WORKFLOW_BUILD_FAILURE -eq '1') { exit 17 }
if ($env:WORKFLOW_MISSING_ARTIFACT -ne '1') {
    foreach ($name in @('beyond-gate', 'beyond-gate-mcp')) {
        @{ reason = 'compiler-artifact'; target = @{ name = $name }; executable = (Get-Command pwsh).Source } |
            ConvertTo-Json -Depth 4 -Compress
    }
}
exit 0
'@
    $fakeProduct = @'
    [Console]::Out.WriteLine((@{ arguments = @($args); input = [Console]::In.ReadToEnd() } | ConvertTo-Json -Compress))
[Console]::Error.WriteLine('synthetic product diagnostic')
exit 23
'@
    [IO.File]::WriteAllText((Join-Path $fixture 'scripts/cargo.ps1'), $fakeCargo)
    $product = Join-Path $fixture 'product with spaces.ps1'
    [IO.File]::WriteAllText($product, $fakeProduct)
    $runner = Join-Path $fixture 'scripts/run.ps1'
    foreach ($mode in @('cli', 'mcp')) {
        $result = Invoke-DeveloperProcess pwsh @('-NoProfile', '-File', $runner, $mode, '-NoProfile', '-File', $product, 'two words', '--flag', 'quote"value') -InputText "synthetic input`n"
        $output = $result.Stdout | ConvertFrom-Json
        $forwarded = @($output.arguments)
        if ($result.ExitCode -ne 23 -or $forwarded.Count -ne 3 -or
            $output.input -cne "synthetic input`n" -or
            $forwarded[0] -cne 'two words' -or $forwarded[1] -cne '--flag' -or $forwarded[2] -cne 'quote"value' -or
            $result.Stderr -notmatch 'synthetic developer-shell diagnostic' -or
            $result.Stderr -notmatch 'synthetic build diagnostic' -or
            $result.Stderr -notmatch 'synthetic product diagnostic') {
            throw "The $mode runner changed arguments, output streams, or failure status."
        }
    }
    $env:WORKFLOW_BUILD_FAILURE = '1'
    $result = Invoke-DeveloperProcess pwsh @('-NoProfile', '-File', $runner, 'cli')
    if ($result.ExitCode -ne 17 -or $result.Stdout) { throw 'Build failure did not stop launch.' }
    Remove-Item Env:WORKFLOW_BUILD_FAILURE
    $env:WORKFLOW_MISSING_ARTIFACT = '1'
    $result = Invoke-DeveloperProcess pwsh @('-NoProfile', '-File', $runner, 'mcp')
    if ($result.ExitCode -ne 1 -or $result.Stdout -or $result.Stderr -notmatch 'did not report') {
        throw 'Missing executable did not fail explicitly.'
    }
    Remove-Item Env:WORKFLOW_MISSING_ARTIFACT
    $result = Invoke-DeveloperProcess pwsh @('-NoProfile', '-File', $runner, 'unknown')
    if ($result.ExitCode -ne 2 -or $result.Stdout) { throw 'Invalid runner mode was accepted.' }

    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'validate-workspace.ps1') -Destination (Join-Path $fixture 'scripts')
    [IO.File]::WriteAllText((Join-Path $fixture 'scripts/cargo.ps1'), 'exit 19')
    $result = Invoke-DeveloperProcess pwsh @('-NoProfile', '-File', (Join-Path $fixture 'scripts/validate-workspace.ps1'))
    if ($result.ExitCode -ne 19) { throw 'Workspace validation did not stop on the first failed check.' }
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'test-mcp.ps1') -Destination (Join-Path $fixture 'scripts')
    [IO.File]::WriteAllText((Join-Path $fixture 'scripts/cargo.ps1'), '[Console]::Out.WriteLine($args[0]); exit 19')
    $result = Invoke-DeveloperProcess pwsh @('-NoProfile', '-File', (Join-Path $fixture 'scripts/test-mcp.ps1'))
    if ($result.ExitCode -ne 19 -or $result.Stdout.Trim() -cne 'build') { throw 'MCP checks did not stop on build failure.' }

    $siteDriver = Join-Path $fixture 'site-driver.ps1'
    $fakePnpm = @'
function pnpm {
    [Console]::Out.WriteLine(($args -join ' '))
    $global:LASTEXITCODE = 29
}
& $args[0]
exit $LASTEXITCODE
'@
    [IO.File]::WriteAllText($siteDriver, $fakePnpm)
    $result = Invoke-DeveloperProcess pwsh @('-NoProfile', '-File', $siteDriver, (Join-Path $PSScriptRoot 'validate-doc-site.ps1'))
    if ($result.ExitCode -ne 29 -or $result.Stdout.Trim() -cne '--dir docs build') {
        throw 'Site validation did not stop on build failure.'
    }
    try {
        Invoke-DeveloperProcess pwsh @('-NoProfile', '-Command', 'Start-Sleep -Seconds 30') -TimeoutSeconds 1 | Out-Null
        throw 'The child-process deadline was not enforced.'
    }
    catch {
        if ($_.Exception.Message -notmatch 'exceeded the 1 second check deadline') { throw }
    }

    $editor = Get-Content -LiteralPath (Join-Path $root '.vscode/tasks.json') -Raw | ConvertFrom-Json
    $app = Get-Content -LiteralPath (Join-Path $root '.github/github-app.yml') -Raw
    $entries = [regex]::Matches($app, "(?m)^  - name: '([^']+)'\r?\n    command: ([^\r\n]+)$")
    $appNames = @($entries | ForEach-Object { $_.Groups[1].Value })
    $appCommands = @{}
    foreach ($entry in $entries) { $appCommands[$entry.Groups[1].Value] = $entry.Groups[2].Value -split ' ' }
    $editorNames = @($editor.tasks.label)
    if ($appNames.Count -ne $editorNames.Count -or
        @($appNames | Select-Object -Unique).Count -ne $appNames.Count -or
        @($editorNames | Select-Object -Unique).Count -ne $editorNames.Count -or
        (Compare-Object $appNames $editorNames)) {
        throw 'Copilot App and VS Code task names differ or contain duplicates.'
    }
    foreach ($task in $editor.tasks) {
        foreach ($dependency in $task.dependsOn) {
            if ($dependency -notin $editorNames) { throw "Unknown task dependency: $dependency." }
        }
    }
    if ($app -match '(?m)^\s+triggers:' -or $appNames -match '^(?!Rust:|CLI:|MCP:|Docs:|Dependencies:|Docker:|Workspace:)') {
        throw 'Tasks must be manual and use subject-first names.'
    }
    foreach ($entry in $entries) {
        $command = $entry.Groups[2].Value
        if ($command.StartsWith('pwsh ') -and -not $command.StartsWith('pwsh -NoProfile -File ./scripts/')) {
            throw 'PowerShell tasks must use -File to preserve script exit codes.'
        }
    }

    $command = $appCommands['CLI: Run']
    $result = Invoke-DeveloperProcess $command[0] @($command | Select-Object -Skip 1) -WorkingDirectory $root -TimeoutSeconds 600
    if ($result.ExitCode -ne 2 -or $result.Stdout -or $result.Stderr -notmatch 'Usage:') {
        throw 'The configured CLI run task changed the no-command usage contract.'
    }
    foreach ($mode in @('cli', 'mcp')) {
        $taskName = if ($mode -eq 'cli') { 'CLI: Run' } else { 'MCP: Serve stdio' }
        $command = $appCommands[$taskName]
        $arguments = @($command | Select-Object -Skip 1)
        $result = Invoke-DeveloperProcess $command[0] ($arguments + @('--help')) -WorkingDirectory $root -TimeoutSeconds 600
        if ($result.ExitCode -ne 0 -or $result.Stdout -notmatch 'Usage:') { throw "Native $mode help failed: $($result.Stderr)" }
        $result = Invoke-DeveloperProcess $command[0] ($arguments + @('--invented-workflow-option')) -WorkingDirectory $root
        if ($result.ExitCode -ne 2 -or $result.Stdout) { throw "Native $mode argument refusal failed." }
    }
    $command = $appCommands['MCP: Serve stdio']
    $result = Invoke-DeveloperProcess $command[0] @($command | Select-Object -Skip 1) -WorkingDirectory $root -InputText (Get-DeveloperProtocolInput)
    Assert-DeveloperProtocol $result
    Write-Host "Native workflow and $($appNames.Count) matching developer tasks passed."
}
finally {
    Remove-Item Env:WORKFLOW_BUILD_FAILURE, Env:WORKFLOW_MISSING_ARTIFACT -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $fixture -Recurse -Force
}
