# Submission lifecycle, tested with a stub gh. The entry script gates real use.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-Submission {
    param([bool]$Condition, [string]$Reason)
    if (!$Condition) { throw [InvalidOperationException]::new($Reason) }
}

function Invoke-E2eSecretSubmission {
    param(
        [Security.SecureString]$Cookie,
        [Parameter(Mandatory)][scriptblock]$InvokeGh,
        [string]$SubmissionId = [guid]::NewGuid().ToString('N'),
        [ValidateRange(1, 1800)][int]$TimeoutSeconds = 600,
        [ValidateRange(1, 300)][int]$QueueTimeoutSeconds = 120
    )
    Assert-Submission ($SubmissionId -cmatch '^[a-f0-9]{32}$') 'submission_invalid'
    $submission = $SubmissionId
    $name = "BEYOND_GATE_E2E_$($submission.ToUpperInvariant())"
    $repository = 'PlagueHO/beyond-gate'
    $deadline = [datetime]::UtcNow.AddSeconds($TimeoutSeconds)
    $queueDeadline = [datetime]::UtcNow.AddSeconds($QueueTimeoutSeconds)
    $attempted = $false
    $runId = $null
    $finished = $false
    try {
        $result = & $InvokeGh @('secret', 'list', '--repo', $repository, '--json', 'name') ''
        Assert-Submission ($result.Code -eq 0) 'secret_list_failed'
        $names = @(ConvertFrom-Json -InputObject $result.Out -AsHashtable)
        Assert-Submission (@($names | Where-Object { $_.name -ieq $name }).Count -eq 0) 'secret_slot_exists'
        Assert-Submission ($names.Count -lt 100) 'secret_capacity'
        $pointer = [IntPtr]::Zero
        $plain = $null
        try {
            $pointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($Cookie)
            $plain = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pointer)
            Assert-Submission ($plain.Length -gt 0 -and [Text.Encoding]::UTF8.GetByteCount($plain) -le 8192 -and $plain -notmatch '[\r\n]') 'cookie_invalid'
            $attempted = $true
            $result = & $InvokeGh @('secret', 'set', $name, '--repo', $repository) $plain
            Assert-Submission ($result.Code -eq 0) 'secret_upload_failed'
        } finally {
            $plain = $null
            if ($pointer -ne [IntPtr]::Zero) { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pointer) }
        }
        $result = & $InvokeGh @(
            'workflow', 'run', 'live-e2e.yml', '--repo', $repository, '--ref', 'main',
            '-f', "submission_id=$submission", '-f', 'selection=full', '-f', 'acknowledge=true'
        ) ''
        Assert-Submission ($result.Code -eq 0) 'dispatch_failed'
        while ([datetime]::UtcNow -lt $deadline) {
            $result = & $InvokeGh @(
                'run', 'list', '--repo', $repository, '--workflow', 'live-e2e.yml',
                '--event', 'workflow_dispatch', '--branch', 'main', '--limit', '100',
                '--json', 'databaseId,displayTitle,status,conclusion'
            ) ''
            Assert-Submission ($result.Code -eq 0) 'run_lookup_failed'
            $runs = @(ConvertFrom-Json -InputObject $result.Out -AsHashtable |
                Where-Object { $_.displayTitle -ceq "live-e2e-$submission" })
            Assert-Submission ($runs.Count -le 1) 'run_ambiguous'
            if ($runs.Count -eq 1) {
                Assert-Submission ([string]$runs[0].databaseId -match '^[1-9][0-9]*$') 'run_invalid'
                $runId = $runs[0].databaseId
                if ($runs[0].status -eq 'completed') {
                    $finished = $true
                    Assert-Submission ($runs[0].conclusion -eq 'success') 'run_failed'
                    return @{ submission_id = $submission; run_id = $runId }
                }
                Assert-Submission ($runs[0].status -in @('queued', 'in_progress', 'waiting', 'pending', 'requested')) 'run_invalid'
                if ($runs[0].status -ne 'in_progress') {
                    Assert-Submission ([datetime]::UtcNow -lt $queueDeadline) 'queue_timeout'
                }
            } else { Assert-Submission ([datetime]::UtcNow -lt $queueDeadline) 'queue_timeout' }
            Start-Sleep -Seconds 2
        }
        throw [InvalidOperationException]::new('submission_timeout')
    } finally {
        $cancelFailed = $false
        try {
            if ($runId -and !$finished) {
                try {
                    $cancel = & $InvokeGh @('run', 'cancel', [string]$runId, '--repo', $repository) ''
                    $cancelFailed = $cancel.Code -ne 0
                } catch { $cancelFailed = $true }
            }
        } finally {
            if ($attempted) {
                $cleanupFailed = $false
                try {
                    $cleanup = & $InvokeGh @('secret', 'delete', $name, '--repo', $repository) ''
                    $cleanupFailed = $cleanup.Code -ne 0
                } catch { $cleanupFailed = $true }
                if ($cleanupFailed) {
                    # Only a generated secret name is included, never gh output.
                    throw [InvalidOperationException]::new("secret_cleanup_failed:$name")
                }
            }
        }
        Assert-Submission (!$cancelFailed) 'run_cancel_failed'
    }
}

Export-ModuleMember -Function Invoke-E2eSecretSubmission
