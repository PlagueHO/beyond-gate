param(
    [string]$BinaryDirectory,
    [ValidateSet('full', 'contracts')][string]$Selection = 'full',
    [switch]$ApproveNativeStorage,
    [datetime]$Deadline = [datetime]::UtcNow.AddSeconds(180)
)

BeforeDiscovery {
    Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking
    $cases = Get-E2eCases (Get-E2eInventory)
    $liveCases = @($cases | Where-Object class -EQ 'live' | ForEach-Object { @{ Case = $_ } })
    $nativeCases = @($cases | Where-Object class -EQ 'native' | ForEach-Object { @{ Case = $_ } })
    $skipNative = $Selection -eq 'contracts' -or !$ApproveNativeStorage -or !$IsWindows
}
BeforeAll {
    Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking
    Import-Module (Join-Path $PSScriptRoot 'fixtures.psm1') -DisableNameChecking
}

Describe 'Live prerequisites' -Tag 'Live' {
    It '<Case.id>' -ForEach $liveCases -Skip {
        Assert-E2eLiveReady
    }
}
Describe 'Isolated native prerequisites' -Tag 'Native' {
    BeforeEach {
        $context = New-E2eContext $BinaryDirectory $Deadline
        Start-E2eNativeContext $context
    }
    AfterEach { if ($context) { Close-E2eContext $context } }
    It '<Case.id>' -ForEach $nativeCases -Skip:$skipNative {
        if ($Case.id -in @('native.auth.stored-unverified', 'native.auth.logout', 'native.auth.repeated-logout')) {
            $seed = Invoke-E2eContextProcess $context 'helper' @('seed')
            $seed.Code | Should -Be 0
            $seed.Err | Should -BeExactly ''
        }
        if ($Case.id -eq 'native.auth.repeated-logout') {
            $first = Invoke-E2eContextProcess $context 'cli' @('auth', 'logout')
            $first.Code | Should -Be 0
            $first.Err | Should -BeExactly ''
        }
        $action = if ($Case.id -in @('native.auth.absent', 'native.auth.stored-unverified')) { 'status' } else { 'logout' }
        $result = Invoke-E2eContextProcess $context 'cli' @('auth', $action)
        $expected = switch ($Case.id) {
            'native.auth.absent' { 'absent:' }
            'native.auth.stored-unverified' { 'stored-unverified:' }
            default { 'removed or already absent' }
        }
        $result.Code | Should -Be 0
        $result.Err | Should -BeExactly ''
        $result.Out.Contains($expected) | Should -BeTrue
        $status = Invoke-E2eContextProcess $context 'cli' @('auth', 'status')
        $status.Code | Should -Be 0
        $status.Err | Should -BeExactly ''
        $statusExpected = if ($Case.id -eq 'native.auth.stored-unverified') { 'stored-unverified:' } else { 'absent:' }
        $status.Out.StartsWith($statusExpected) | Should -BeTrue
    }
}
