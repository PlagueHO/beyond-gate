param([string]$BinaryDirectory, [datetime]$Deadline = [datetime]::UtcNow.AddSeconds(180))

BeforeDiscovery {
    Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking
    $cases = Get-E2eCases (Get-E2eInventory)
    $cliCases = @($cases | Where-Object { $_.class -in @('cli', 'install') } |
        ForEach-Object { @{ Case = $_ } })
}
BeforeAll {
    Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking
    Import-Module (Join-Path $PSScriptRoot 'fixtures.psm1') -DisableNameChecking
    $versionMatch = [regex]::Match(
        (Get-Content -LiteralPath (Join-Path $PSScriptRoot '..\..\Cargo.toml') -Raw),
        '(?m)^version = "([^"]+)"\r?$'
    )
    $versionMatch.Success | Should -BeTrue
    $version = $versionMatch.Groups[1].Value
}

Describe 'CLI and installation contracts' -Tag 'Contracts', 'Cli' {
    BeforeEach { $context = New-E2eContext $BinaryDirectory $Deadline }
    AfterEach { if ($context) { Close-E2eContext $context } }

    It '<Case.id>' -ForEach $cliCases {
        $data = $Case.data
        if ($Case.class -eq 'cli') {
            $binary = if ($data.ContainsKey('binary')) { $data.binary } else { 'cli' }
            $result = Invoke-E2eContextProcess $context $binary $data.arguments
            $result.Code | Should -Be $data.code
            if ($data.code -eq 0) {
                $result.Err | Should -BeExactly ''
                $result.Out.Contains($data.contains) | Should -BeTrue
            } else {
                $result.Out | Should -BeExactly ''
                $result.Err | Should -Not -BeNullOrEmpty
                if ($data.ContainsKey('error')) { $result.Err.Contains($data.error) | Should -BeTrue }
            }
            if ($Case.id.EndsWith('.version')) {
                $name = if ($binary -eq 'server') { 'beyond-gate-mcp' } else { 'beyond-gate' }
                $result.Out.TrimEnd("`r", "`n") | Should -BeExactly "$name $version"
            }
        } else {
            $path = if ($data.client -eq 'claude-code') {
                Join-Path $context.Home '.claude.json'
            } else { Join-Path $context.Environment.COPILOT_HOME 'mcp-config.json' }
            $settings = '{"keep":{"synthetic":true},"mcpServers":{"other":{"command":"synthetic-other"}}}'
            if ($data.scenario -eq 'conflict') {
                $settings = '{"mcpServers":{"beyond-gate":{"command":"synthetic-conflict"}}}'
            } elseif ($data.scenario -eq 'malformed') { $settings = '{not-json' }
            [IO.Directory]::CreateDirectory((Split-Path -Parent $path)) | Out-Null
            [IO.File]::WriteAllText($path, $settings)
            $arguments = @('mcp', 'install', $data.client)
            $inputText = "n`n"
            if ($data.scenario -eq 'preview') { $arguments += '--dry-run' }
            if ($data.scenario -in @('confirm', 'idempotence')) { $inputText = "y`n" }
            $result = Invoke-E2eContextProcess $context 'cli' $arguments $inputText
            if ($data.scenario -in @('conflict', 'malformed')) {
                $result.Code | Should -Be 1
                $result.Out | Should -BeExactly ''
                $result.Err | Should -Not -BeNullOrEmpty
            } else {
                $result.Code | Should -Be 0
                $result.Err | Should -BeExactly ''
            }
            if ($data.scenario -in @('confirm', 'idempotence')) {
                $updated = [IO.File]::ReadAllText($path) | ConvertFrom-Json -AsHashtable
                $updated.keep.synthetic | Should -BeTrue
                $updated.mcpServers.other.command | Should -BeExactly 'synthetic-other'
                $entry = $updated.mcpServers.'beyond-gate'
                $entry.command | Should -BeExactly $context.Binaries.server
                $entry.args.Count | Should -Be 0
                if ($data.client -ne 'claude-code') { $entry.type | Should -BeExactly 'local' }
                if ($data.scenario -eq 'idempotence') {
                    $before = [IO.File]::ReadAllText($path)
                    $result = Invoke-E2eContextProcess $context 'cli' $arguments
                    $result.Code | Should -Be 0
                    $result.Err | Should -BeExactly ''
                    $result.Out.Contains('already registered') | Should -BeTrue
                    [IO.File]::ReadAllText($path) | Should -BeExactly $before
                }
            } else { [IO.File]::ReadAllText($path) | Should -BeExactly $settings }
        }
    }
}
