# Fixed live scope. Never return raw subprocess output or credential values.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'common.psm1') -DisableNameChecking

function New-LiveApiExecutable {
    param([ValidateSet('live_api', 'live_entities')][string]$TestTarget = 'live_api')
    $root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
    $build = Invoke-E2eProcess (Join-Path $PSHOME 'pwsh') @(
        '-NoProfile', '-File', (Join-Path $root 'scripts\cargo.ps1'),
        'test', '-p', 'beyond-gate-core', '--test', $TestTarget, '--no-run',
        '--locked', '--message-format=json'
    ) -Directory $root -TimeoutSeconds 900 -Deadline ([datetime]::UtcNow.AddSeconds(910))
    if ($build.Code -ne 0) { throw [InvalidOperationException]::new('build_failed') }
    $executables = @(foreach ($line in $build.Out.Split("`n")) {
        if (!$line.StartsWith('{')) { continue }
        try { $message = ConvertFrom-Json $line -AsHashtable }
        catch { throw [InvalidOperationException]::new('build_contract_failed') }
        if ($message.reason -ceq 'compiler-artifact' -and $message.target.name -ceq $TestTarget -and
            $message.target.kind -contains 'test' -and $message.executable) {
            $message.executable
        }
    })
    if ($executables.Count -ne 1 -or !(Test-Path -LiteralPath $executables[0] -PathType Leaf)) {
        throw [InvalidOperationException]::new('build_contract_failed')
    }
    return $executables[0]
}

function Get-LiveApiSecret {
    param([switch]$UseEnvironment)
    if ($UseEnvironment) {
        $value = [Environment]::GetEnvironmentVariable('BEYOND_GATE_COBALT_SESSION')
        if ([string]::IsNullOrEmpty($value)) { throw [InvalidOperationException]::new('input_missing') }
        return $value
    }
    if ([Console]::IsInputRedirected) { throw [InvalidOperationException]::new('terminal_required') }
    $secret = Read-Host 'CobaltSession (hidden; process-only; not saved)' -AsSecureString
    $pointer = [IntPtr]::Zero
    try {
        $pointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secret)
        return [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pointer)
    } finally {
        if ($pointer -ne [IntPtr]::Zero) { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pointer) }
        $secret.Dispose()
    }
}

function Assert-LiveApiSecret {
    param([string]$Value)
    if ([string]::IsNullOrEmpty($Value)) { throw [InvalidOperationException]::new('input_missing') }
    if ($Value.Length -gt 2552 -or $Value.StartsWith('CobaltSession=', [StringComparison]::Ordinal) -or
        $Value.StartsWith('Cookie:', [StringComparison]::OrdinalIgnoreCase) -or
        $Value -cmatch '[^\x21\x23-\x2b\x2d-\x3a\x3c-\x5b\x5d-\x7e]') {
        throw [InvalidOperationException]::new('input_invalid')
    }
}

function ConvertTo-LiveApiOutcome {
    param([hashtable]$ProcessResult)
    $labels = @(
        'passed', 'approval_required', 'input_missing', 'input_invalid', 'unsupported_platform',
        'session_rejected', 'session_expired', 'timed_out', 'access_denied',
        'private_request_failed', 'public_request_failed', 'contract_mismatch'
    )
    $markers = @([regex]::Matches($ProcessResult.Out,
        '(?m)^(?:test public_and_authenticated_library \.\.\. )?beyond_gate_live_api=([a-z_]+)\r?$'))
    $reason = 'harness_error'
    if ($markers.Count -eq 1 -and $markers[0].Groups[1].Value -cin $labels) {
        $candidate = $markers[0].Groups[1].Value
        if (($ProcessResult.Code -eq 0) -eq ($candidate -ceq 'passed')) { $reason = $candidate }
    }
    return @{ status = $(if ($reason -ceq 'passed') { 'passed' } else { 'failed' }); reason = $reason }
}

function Invoke-LiveApiProcess {
    param([string]$Executable, [switch]$ApproveLive, [switch]$UseEnvironment)
    if (!$ApproveLive) { throw [InvalidOperationException]::new('approval_required') }
    if (!$IsWindows -and !$IsLinux) { throw [InvalidOperationException]::new('unsupported_platform') }
    $environment = @{}
    $value = $null
    try {
        $value = Get-LiveApiSecret -UseEnvironment:$UseEnvironment
        Assert-LiveApiSecret $value
        $environment['BEYOND_GATE_COBALT_SESSION'] = $value
        $environment['BEYOND_GATE_LIVE_API_APPROVED'] = 'public-and-authenticated-library-five-attempts'
        $processResult = Invoke-E2eProcess $Executable @(
            '--ignored', '--exact', 'public_and_authenticated_library', '--nocapture', '--test-threads=1'
        ) -Environment $environment -TimeoutSeconds 40 -MaxBytes 16384
        return ConvertTo-LiveApiOutcome $processResult
    } finally {
        $environment.Clear()
        $value = $null
    }
}

function Get-LiveEntityScope {
    param([string]$Entity = 'all')
    $budgets = [ordered]@{
        book = 11; character = 5; campaign = 6; class = 6; subclass = 10; monster = 6
        background = 5; feat = 5; species = 4; spell = 4; equipment = 6; magic_item = 6
    }
    if ($Entity -cne 'all' -and $Entity -cnotin $budgets.Keys) {
        throw [InvalidOperationException]::new('selection_invalid')
    }
    foreach ($name in $budgets.Keys) {
        if ($Entity -cne 'all' -and $Entity -cne $name) { continue }
        foreach ($operation in @('list', 'detail')) {
            [pscustomobject]@{ id = "$name.$operation"; family = $name; budget = $budgets[$name] }
        }
    }
}

function New-FailedEntityRows {
    param([string]$Entity = 'all', [string]$Reason = 'harness_error')
    foreach ($case in (Get-LiveEntityScope $Entity)) {
        @{ id = $case.id; status = 'failed'; reason = $Reason }
    }
}

function ConvertTo-LiveEntityOutcome {
    param([hashtable]$ProcessResult, [string]$Entity = 'all')
    $scope = @(Get-LiveEntityScope $Entity)
    $labels = @(
        'verified', 'stopped_after_failure', 'read_failed', 'selection_invalid', 'timed_out',
        'rate_limited', 'access_denied', 'redirect_rejected', 'body_limit', 'session_absent',
        'session_rejected', 'session_expired', 'invalid_material', 'unsupported_contract',
        'storage_access', 'storage_unavailable', 'ambiguous_entry', 'storage_failure',
        'validation_unavailable', 'unsupported_platform', 'authenticated_read_failed',
        'record_not_found', 'session_changed'
    )
    $budgets = @([regex]::Matches($ProcessResult.Out,
        '(?m)^(?:test public_and_authenticated_entities \.\.\. )?beyond_gate_live_entities_budget=([0-9]{1,2})\r?$'))
    $expectedBudget = ($scope | Group-Object family | ForEach-Object { $_.Group[0].budget } |
        Measure-Object -Sum).Sum
    $markers = @([regex]::Matches($ProcessResult.Out,
        '(?m)^beyond_gate_live_entity=([a-z_]+\.(?:list|detail)):(passed|failed|blocked):([a-z_]+)\r?$'))
    $allMarkers = @([regex]::Matches($ProcessResult.Out, '(?m)^beyond_gate_live_entity='))
    $valid = $budgets.Count -eq 1 -and [int]$budgets[0].Groups[1].Value -eq $expectedBudget -and
        $markers.Count -eq $scope.Count -and $allMarkers.Count -eq $scope.Count
    $rows = @()
    $stopped = $false
    for ($index = 0; $index -lt $markers.Count; $index++) {
        $groups = $markers[$index].Groups
        $id = $groups[1].Value; $status = $groups[2].Value; $reason = $groups[3].Value
        if ($index -ge $scope.Count -or $id -cne $scope[$index].id -or $reason -cnotin $labels) { $valid = $false }
        if (($stopped -and $status -cne 'blocked') -or (!$stopped -and $status -ceq 'blocked') -or
            ($status -ceq 'passed' -and $reason -cne 'verified') -or
            ($status -ceq 'failed' -and $reason -cin @('verified', 'stopped_after_failure')) -or
            ($status -ceq 'blocked' -and $reason -cne 'stopped_after_failure')) { $valid = $false }
        if ($status -ceq 'failed') { $stopped = $true }
        $rows += @{ id = $id; status = $status; reason = $reason }
    }
    if (($ProcessResult.Code -eq 0) -ne (!$stopped)) { $valid = $false }
    if (!$valid) { return New-FailedEntityRows $Entity }
    return $rows
}

function Invoke-LiveEntitiesProcess {
    param([string]$SelectionJson, [string]$Entity = 'all', [switch]$ApproveLive, [switch]$UseEnvironment)
    if (!$ApproveLive) { throw [InvalidOperationException]::new('approval_required') }
    if (!$IsWindows -and !$IsLinux) { throw [InvalidOperationException]::new('unsupported_platform') }
    $null = @(Get-LiveEntityScope $Entity)
    if ([Text.Encoding]::UTF8.GetByteCount($SelectionJson) -gt 16384) {
        throw [InvalidOperationException]::new('selection_invalid')
    }
    $environment = @{
        BEYOND_GATE_LIVE_ENTITY_SELECTIONS = $SelectionJson
        BEYOND_GATE_LIVE_ENTITY = $Entity
    }
    $value = $null
    try {
        $executable = New-LiveApiExecutable -TestTarget live_entities
        $preflight = Invoke-E2eProcess $executable @(
            '--ignored', '--exact', 'validate_entity_selections', '--nocapture', '--test-threads=1'
        ) -Environment $environment -MaxBytes 16384
        $markers = @([regex]::Matches($preflight.Out,
            '(?m)^(?:test validate_entity_selections \.\.\. )?beyond_gate_live_entities_selection=valid\r?$'))
        if ($preflight.Code -ne 0 -or $markers.Count -ne 1) {
            throw [InvalidOperationException]::new('selection_invalid')
        }
        $value = Get-LiveApiSecret -UseEnvironment:$UseEnvironment
        Assert-LiveApiSecret $value
        $environment['BEYOND_GATE_COBALT_SESSION'] = $value
        $environment['BEYOND_GATE_LIVE_API_APPROVED'] = 'selected-entity-readers-74-attempts'
        $result = Invoke-E2eProcess $executable @(
            '--ignored', '--exact', 'public_and_authenticated_entities', '--nocapture', '--test-threads=1'
        ) -Environment $environment -TimeoutSeconds 610 -Deadline ([datetime]::UtcNow.AddSeconds(620)) -MaxBytes 16384
        return ConvertTo-LiveEntityOutcome $result $Entity
    } finally {
        $environment.Clear()
        $value = $null
    }
}

Export-ModuleMember -Function New-LiveApiExecutable, Invoke-LiveApiProcess, ConvertTo-LiveApiOutcome,
    Get-LiveEntityScope, New-FailedEntityRows, ConvertTo-LiveEntityOutcome, Invoke-LiveEntitiesProcess
