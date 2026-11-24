param([string]$BinaryDirectory, [datetime]$Deadline = [datetime]::UtcNow.AddSeconds(180))

BeforeDiscovery {
    Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking
    $cases = Get-E2eCases (Get-E2eInventory)
    $modes = @(foreach ($mode in @('direct', 'start')) {
        @{
            Mode = $mode
            ModeCases = @($cases | Where-Object class -CEQ "mcp-$mode" | ForEach-Object { @{ Case = $_ } })
        }
    })
}
BeforeAll {
    Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking
    Import-Module (Join-Path $PSScriptRoot 'fixtures.psm1') -DisableNameChecking
    $inventory = Get-E2eInventory
}

Describe 'MCP <Mode> contracts' -ForEach $modes -Tag 'Contracts', 'Mcp' {
    BeforeEach {
        $context = New-E2eContext $BinaryDirectory $Deadline
        $child = $null
        $id = 1
        $executable = if ($Mode -eq 'direct') { $context.Binaries.server } else { $context.Binaries.cli }
        $arguments = if ($Mode -eq 'direct') { @() } else { @('mcp', 'start') }
        $child = Start-E2eChild $executable $arguments $context.Environment $context.Directory 30 1048576 $Deadline
        $response = Invoke-E2eRequest $child @{
            jsonrpc = '2.0'; id = $id; method = 'initialize'; params = @{
                protocolVersion = '2026-07-28'; capabilities = @{}
                clientInfo = @{ name = 'beyond-gate-e2e'; version = '0.1.0' }
            }
        }
        $response.result.capabilities.ContainsKey('tools') | Should -BeTrue
        Send-E2eInput $child ('{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' + "`n")
    }
    AfterEach {
        try {
            if ($child) {
                try {
                    $result = Complete-E2eChild $child
                    $result.Code | Should -Be 0
                    $result.Err | Should -BeExactly ''
                    @(Get-E2eMessages $child).Count | Should -Be $id
                    $result.Out.EndsWith("`n") | Should -BeTrue
                } finally { Stop-E2eChild $child }
            }
        } finally { if ($context) { Close-E2eContext $context } }
    }
    It '<Case.id>' -ForEach $modeCases {
        $name = $Case.id.Split('.')[-1]
        if ($name -eq 'lifecycle') {
            $id++
            $response = Invoke-E2eRequest $child @{ jsonrpc = '2.0'; id = $id; method = 'ping'; params = @{} }
            $response.ContainsKey('result') | Should -BeTrue
        } elseif ($name -eq 'discovery') {
            $id++
            $response = Invoke-E2eRequest $child @{ jsonrpc = '2.0'; id = $id; method = 'tools/list'; params = @{} }
            $names = @($response.result.tools.name | Sort-Object)
            $expectedTools = @($inventory.reads.tool) + $inventory.session_validation.tool
            ($names -join ',') | Should -BeExactly (($expectedTools | Sort-Object) -join ',')
            foreach ($tool in $response.result.tools) {
                $tool.annotations.readOnlyHint | Should -BeTrue
                $tool.annotations.destructiveHint | Should -BeFalse
                $tool.inputSchema.additionalProperties | Should -BeFalse
                $read = (@($inventory.reads) + $inventory.session_validation) | Where-Object tool -CEQ $tool.name
                $properties = if ($read.ContainsKey('input_fields')) { @($read.input_fields) } else { @($read.arguments.Keys) }
                if ($tool.name -ceq 'book_read') { $properties += 'continuation' }
                $actual = if ($tool.inputSchema.ContainsKey('properties')) { @($tool.inputSchema.properties.Keys | Sort-Object) } else { @() }
                ($actual -join ',') | Should -BeExactly (($properties | Sort-Object) -join ',')
            }
            $book = $response.result.tools | Where-Object name -CEQ 'book_read'
            $book.inputSchema.properties.continuation.maxLength | Should -Be 16384
        } elseif ($name -eq 'cancellation') {
            Send-E2eInput $child ('{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":999,"reason":"synthetic"}}' + "`n")
            $id++
            $response = Invoke-E2eRequest $child @{ jsonrpc = '2.0'; id = $id; method = 'ping'; params = @{} }
            $response.ContainsKey('result') | Should -BeTrue
        } else {
            $read = @($inventory.reads | Where-Object tool -CEQ $name)
            if ($read.Count -eq 1) {
                $requiresSession = $read[0].ContainsKey('requires_session') -and $read[0].requires_session
                $tool = $read[0].tool
                $arguments = if ($requiresSession) { @{ unexpected = $true } } else { $read[0].arguments }
            } else {
                $check = switch ($name) {
                    'invalid-input' { @{ tool = 'character_show'; arguments = @{ id = 'synthetic'; extra = $true } } }
                    'invalid-validation-input' { @{ tool = $inventory.session_validation.tool; arguments = @{ extra = $true } } }
                    'invalid-token' { @{ tool = 'book_read'; arguments = @{ book_id = 'synthetic'; section_id = 'opening'; continuation = 'invalid' } } }
                    'unknown-tool' { @{ tool = 'not_a_tool'; arguments = @{} } }
                    default { throw [InvalidOperationException]::new('inventory_invalid') }
                }
                $tool = $check.tool
                $arguments = $check.arguments
            }
            $id++
            $response = Invoke-E2eRequest $child @{
                jsonrpc = '2.0'; id = $id; method = 'tools/call'
                params = @{ name = $tool; arguments = $arguments }
            }
            if ($read.Count -eq 1) {
                $response.result.isError | Should -BeTrue
                $expected = if ($requiresSession) { 'unknown field' } else { 'unsupported' }
                ($response.result.content[0].text -cmatch $expected) | Should -BeTrue
            } else {
                ($response.ContainsKey('error') -or ($response.ContainsKey('result') -and $response.result.isError -eq $true)) | Should -BeTrue
            }
        }
    }
}
