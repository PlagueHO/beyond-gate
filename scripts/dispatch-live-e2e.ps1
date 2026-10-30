#Requires -Version 7.4
# Hidden cookie input remains unreachable until production live contracts exist.
[CmdletBinding()]
param([switch]$ContractsOnly)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Import-Module (Join-Path $PSScriptRoot 'e2e\common.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $PSScriptRoot 'e2e\dispatch.psm1') -Force

function Invoke-DispatchGh {
    param([string[]]$Arguments, [string]$InputText)
    $gh = (Get-Command gh -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    Invoke-E2eProcess $gh $Arguments -InputText $InputText -TimeoutSeconds 30 -AllowGitHubAuthentication
}

try {
    $null = Get-E2eInventory
    if ($ContractsOnly) {
        $submission = [guid]::NewGuid().ToString('N')
        $result = Invoke-DispatchGh @(
            'workflow', 'run', 'live-e2e.yml', '--repo', 'PlagueHO/beyond-gate', '--ref', 'main',
            '-f', "submission_id=$submission", '-f', 'selection=contracts', '-f', 'acknowledge=true'
        ) ''
        Assert-E2e ($result.Code -eq 0) 'contract_mismatch'
        Write-Output "Contract-only dispatch accepted: live-e2e-$submission. No credential was requested."
        exit 0
    }
    Assert-E2eLiveReady
    Assert-E2e (![Console]::IsInputRedirected -and ![Console]::IsOutputRedirected) 'contract_mismatch'
    # This branch is deliberately disabled by the readiness gate above.
    $submission = [guid]::NewGuid().ToString('N')
    Write-Output "Submission: $submission. Recovery secret: BEYOND_GATE_E2E_$($submission.ToUpperInvariant())"
    $cookie = Read-Host 'CobaltSession (hidden; use an unrecorded terminal)' -AsSecureString
    try {
        $result = Invoke-E2eSecretSubmission -Cookie $cookie -InvokeGh ${function:Invoke-DispatchGh} -SubmissionId $submission
        Write-Output "Submission completed: $($result.submission_id). Its secret was removed."
    } finally { $cookie.Dispose() }
} catch {
    # Do not print gh's response, secret input, or arbitrary exception messages.
    if ($_.Exception.Message -cmatch '^secret_cleanup_failed:(BEYOND_GATE_E2E_[A-F0-9]{32})$') {
        Write-Warning "Secret cleanup failed. Run: gh secret delete $($Matches[1]) --repo PlagueHO/beyond-gate"
    }
    Write-Error 'Live dispatch is blocked or failed. No live credentials are currently supported. See docs/testing.md.' -ErrorAction Continue
    exit 1
}
