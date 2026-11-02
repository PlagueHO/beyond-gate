# Collects fixed stack hints from the exact owned synthetic process only.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][int]$BrowserProcessId,
    [Parameter(Mandatory)][string]$BrowserExecutable,
    [Parameter(Mandatory)][string]$RunDirectory,
    [long]$DeadlineUnixMs = [long]::MaxValue
)

$ErrorActionPreference = 'Stop'

# Discards addresses, offsets, arbitrary symbols, paths, and debugger diagnostics.
function ConvertTo-SafeStartupStack {
    param([string]$Text, [ValidateRange(1, 32)][int]$MaxFrames = 32)

    $region = [regex]::Match($Text, '(?ms)^BG_STACK_BEGIN\r?\n(.*?)^BG_STACK_END\r?$')
    if (!$region.Success) { return @{ status = 'unreadable'; reason = 'markers-not-found' } }
    $allowedModules = @(
        'ntdll', 'kernelbase', 'kernel32', 'chrome', 'chrome_elf', 'msedge',
        'msedge_elf', 'wintrust', 'crypt32', 'cryptbase', 'bcrypt', 'ncrypt',
        'rpcrt4', 'combase', 'advapi32', 'user32', 'win32u', 'node'
    )
    $allowedExports = @(
        'NtWaitForSingleObject', 'NtWaitForMultipleObjects', 'NtDelayExecution',
        'NtReadFile', 'NtWriteFile', 'NtCreateFile', 'NtOpenFile',
        'NtRemoveIoCompletion', 'NtAlpcSendWaitReceivePort', 'NtQueryInformationFile',
        'WaitForSingleObject', 'WaitForSingleObjectEx', 'WaitForMultipleObjects',
        'WaitForMultipleObjectsEx', 'ReadFile', 'WriteFile', 'CreateFileW',
        'GetQueuedCompletionStatus', 'GetQueuedCompletionStatusEx',
        'LoadLibraryExW', 'LdrLoadDll', 'WinVerifyTrust', 'CryptQueryObject',
        'ChromeMain', 'BaseThreadInitThunk', 'RtlUserThreadStart'
    )
    $allowedLoaderFunctions = @(
        'LdrpDrainWorkQueue', 'LdrpLoadDll', 'LdrpLoadDllInternal',
        'LdrpInitializeThread', 'LdrpCallInitRoutine', 'LdrpCallTlsInitializers',
        'LdrpProcessWork', 'LdrpWorkCallback', 'LdrpMapDllNtFileName',
        'LdrpMapDllWithSectionHandle', 'LdrpPrepareModuleForExecution',
        'LdrpAcquireLoaderLock', 'LdrLockLoaderLock', 'RtlEnterCriticalSection',
        'RtlAcquireSRWLockExclusive', 'NtCreateSection', 'NtMapViewOfSection',
        'NtWaitForWorkViaWorkerFactory', 'TppWorkerThread', 'NtRemoveIoCompletionEx',
        'RtlpWaitOnCriticalSection', 'RtlpWaitOnAddress', 'RtlpWaitOnAddressWithTimeout'
    )
    $frames = @([regex]::Matches($region.Groups[1].Value,
        '(?mi)^[ \t]*[0-9a-f]{2,3}[ \t]+[0-9a-f`]+[ \t]+[0-9a-f`]+[ \t]+([a-z0-9_]+)(?:!([a-z0-9_]+))?(?:\+0x[0-9a-f]+)?[ \t]*\r?$') |
        Select-Object -First $MaxFrames)
    if ($frames.Count -eq 0) { return @{ status = 'unreadable'; reason = 'frames-not-found' } }
    @{
        status = 'read'
        thread = 'debugger-thread-zero'
        frame_count = $frames.Count
        modules = @($frames | ForEach-Object {
            $module = $_.Groups[1].Value.ToLowerInvariant()
            if ($module -cin $allowedModules) { $module }
        } | Select-Object -Unique)
        export_hints = @($frames | ForEach-Object {
            $module = $_.Groups[1].Value.ToLowerInvariant()
            $symbol = $_.Groups[2].Value
            if ($module -cin $allowedModules -and $symbol -cin $allowedExports) { $symbol }
        } | Select-Object -Unique)
        frame_hints = @($frames | ForEach-Object {
            $module = $_.Groups[1].Value.ToLowerInvariant()
            $symbol = $_.Groups[2].Value
            if ($module -cnotin $allowedModules) { $module = 'other' }
            if ($module -eq 'other' -or
                ($symbol -cnotin $allowedExports -and
                !($module -eq 'ntdll' -and $symbol -cin $allowedLoaderFunctions))) {
                $symbol = 'other'
            }
            ,@($module, $symbol)
        })
    }
}

# Drains both pipes after capture limits so a full buffer cannot block debugger exit.
function Read-BoundedProbeOutput {
    param(
        [Diagnostics.Process]$Probe,
        [IO.MemoryStream]$Output,
        [IO.MemoryStream]$Errors,
        [int]$TimeoutMs
    )

    $clock = [Diagnostics.Stopwatch]::StartNew()
    $streams = @(
        @{ source = $Probe.StandardOutput.BaseStream; sink = $Output; bytes = 0L },
        @{ source = $Probe.StandardError.BaseStream; sink = $Errors; bytes = 0L }
    )
    foreach ($stream in $streams) {
        $stream.buffer = [byte[]]::new(1024)
        $stream.closed = $false
        $stream.pending = $stream.source.ReadAsync($stream.buffer, 0, $stream.buffer.Length)
    }
    while (@($streams | Where-Object { !$_.closed }).Count) {
        $active = @($streams | Where-Object { !$_.closed })
        $wait = [int]($TimeoutMs - $clock.ElapsedMilliseconds)
        if ($wait -le 0 -or
            [Threading.Tasks.Task]::WaitAny([Threading.Tasks.Task[]]$active.pending, $wait) -eq -1) {
            return @{ completed = $false; streams = $streams }
        }
        foreach ($stream in $active) {
            if (!$stream.pending.IsCompleted) { continue }
            $count = $stream.pending.GetAwaiter().GetResult()
            if ($count -eq 0) { $stream.closed = $true; continue }
            $stream.bytes += $count
            $keep = [int][math]::Min($count, $stream.sink.Length - $stream.sink.Position)
            if ($keep -gt 0) { $stream.sink.Write($stream.buffer, 0, $keep) }
            $stream.pending = $stream.source.ReadAsync($stream.buffer, 0, $stream.buffer.Length)
        }
    }
    $wait = [int][math]::Max(0, $TimeoutMs - $clock.ElapsedMilliseconds)
    @{ completed = $Probe.WaitForExit($wait); streams = $streams }
}

# Identifies fixed debugger startup milestones without returning their text.
function Get-SafeProbeProgress {
    param([string]$Text)

    @{
        gallery_preparing = $Text.Contains('Preparing the environment for Debugger Extensions Gallery repositories')
        gallery_waiting = $Text.Contains('Waiting for Debugger Extensions Gallery to Initialize')
        engine_banner = $Text.Contains('Windows Debugger Version')
        symbol_path_reported = $Text.Contains('Symbol search path is:')
        stack_started = [regex]::IsMatch($Text, '(?m)^BG_STACK_BEGIN\r?$')
        stack_completed = [regex]::IsMatch($Text, '(?m)^BG_STACK_END\r?$')
        attach_failure_reported = $Text -match 'Cannot debug pid|Debuggee initialization failed|Unable to attach'
        access_denied_reported = $Text -match 'Access is denied|Win32 error 0n5\b'
        invalid_parameter_reported = $Text -match 'The parameter is incorrect|Win32 error 0n87\b'
        process_exit_reported = $Text -match 'process has exited|process is terminating'
        partial_copy_reported = $Text -match 'Win32 error 0n299\b'
    }
}

if ($MyInvocation.InvocationName -eq '.') { return }

$debugger = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Debuggers\x64\cdb.exe'
if (!(Test-Path -LiteralPath $debugger -PathType Leaf)) {
    @{ status = 'debugger-missing' } | ConvertTo-Json -Compress
    exit 0
}
$browser = Get-Process -Id $BrowserProcessId
$probe = [Diagnostics.Process]::new()
$started = $false
$output = [IO.MemoryStream]::new([byte[]]::new(16384), 0, 16384, $true, $true)
$errors = [IO.MemoryStream]::new([byte[]]::new(4096), 0, 4096, $true, $true)
$stage = 'identity'
try {
    # Keep the original process handle open so Windows cannot reuse its PID.
    $null = $browser.Handle
    $identity = Get-CimInstance Win32_Process -Filter "ProcessId = $BrowserProcessId"
    $profile = [regex]::Escape((Join-Path $RunDirectory 'profile'))
    $profileArgument = '(?:^|\s)(?:"--user-data-dir=' + $profile +
        '"|--user-data-dir="' + $profile + '"|--user-data-dir=' + $profile + ')(?:\s|$)'
    if ($browser.MainModule.FileName -ne $BrowserExecutable -or
        $identity.CommandLine -notmatch $profileArgument) {
        @{ status = 'identity-changed' } | ConvertTo-Json -Compress
        exit 0
    }
    if ($env:GITHUB_ACTIONS -eq 'true' -and $env:RUNNER_TEMP) {
        $symbolCache = Join-Path $env:RUNNER_TEMP 'beyond-gate-startup-ntdll-symbols'
        if (Test-Path -LiteralPath $symbolCache -PathType Container) {
            $files = @(Get-ChildItem -LiteralPath $symbolCache -Filter ntdll.pdb -File -Recurse)
            if ($files.Count -ne 1) { throw 'Expected one prepared Windows loader symbol file.' }
            Copy-Item -LiteralPath $files[0].FullName -Destination (Join-Path $RunDirectory 'ntdll.pdb')
        }
    }
    $remaining = [math]::Min(3000,
        $DeadlineUnixMs - [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - 1000)
    if ($remaining -le 0) {
        @{ status = 'timed-out' } | ConvertTo-Json -Compress
        exit 0
    }
    $stage = 'start'
    $probe.StartInfo.FileName = $debugger
    $probe.StartInfo.WorkingDirectory = $RunDirectory
    $probe.StartInfo.UseShellExecute = $false
    $probe.StartInfo.CreateNoWindow = $true
    $probe.StartInfo.RedirectStandardOutput = $true
    $probe.StartInfo.RedirectStandardError = $true
    $commands = '.reload /f ntdll.dll;.echo BG_STACK_BEGIN;~0s;kn 0n32;.echo BG_STACK_END;'
    for ($index = 1; $index -le [math]::Min(4, $browser.Threads.Count - 1); $index++) {
        $commands += ".echo BG_STACK_${index}_BEGIN;~${index}s;kn 0n8;.echo BG_STACK_${index}_END;"
    }
    $commands += '.echo BG_SYMBOLS_BEGIN;lm m ntdll;.echo BG_SYMBOLS_END;qd'
    foreach ($argument in @(
        '-pv', '-p', "$BrowserProcessId", '-sins', '-nosqm', '-noshell', '-kqm',
        '-netsyms:no', '-y', $RunDirectory, '-c',
        $commands
    )) { $probe.StartInfo.ArgumentList.Add($argument) }
    $started = $probe.Start()
    if (!$started) { throw 'Cannot start the owned stack probe.' }
    $stage = 'read'
    $capture = Read-BoundedProbeOutput -Probe $probe -Output $output -Errors $errors -TimeoutMs $remaining
    $counts = @{
        stdout_bytes = $capture.streams[0].bytes
        stderr_bytes = $capture.streams[1].bytes
        stdout_limit_reached = $capture.streams[0].bytes -ge $output.Length
        stderr_limit_reached = $capture.streams[1].bytes -ge $errors.Length
        target_exited_during_probe = $browser.HasExited
    }
    $text = [Text.Encoding]::UTF8.GetString($output.GetBuffer(), 0, [int]$output.Position)
    $counts.progress = Get-SafeProbeProgress -Text $text
    if (!$capture.completed) {
        $probe.Kill($true)
        $probe.WaitForExit()
        $counts.status = 'timed-out'
        $counts | ConvertTo-Json -Compress
    } else {
        if ($probe.ExitCode -ne 0) {
            @{ status = 'debugger-failed'; exit_code = $probe.ExitCode } | ConvertTo-Json -Compress
        } else {
            $stage = 'parse'
            $stack = ConvertTo-SafeStartupStack -Text $text
            $symbols = [regex]::Match($text, '(?ms)^BG_SYMBOLS_BEGIN\r?\n(.*?)^BG_SYMBOLS_END\r?$')
            $stack.os_symbols_loaded = $symbols.Success -and
                $symbols.Groups[1].Value -match '\bntdll[ \t]+\((?:private )?pdb symbols\)'
            $stack.secondary_threads = @(foreach ($index in 1..4) {
                $region = [regex]::Match($text, "(?ms)^BG_STACK_${index}_BEGIN\r?\n(.*?)^BG_STACK_${index}_END\r?$")
                if ($region.Success) {
                    $other = ConvertTo-SafeStartupStack -MaxFrames 8 -Text (
                        "BG_STACK_BEGIN`n" + $region.Groups[1].Value + "BG_STACK_END")
                    @{ index = $index; status = $other.status; frame_hints = $other.frame_hints }
                }
            })
            foreach ($key in $counts.Keys) { $stack[$key] = $counts[$key] }
            $stack | ConvertTo-Json -Depth 6 -Compress
        }
    }
} catch {
    $kind = $_.Exception.GetBaseException().GetType().Name
    if ($kind -cnotin @(
        'Win32Exception', 'UnauthorizedAccessException', 'NotSupportedException',
        'IOException', 'InvalidOperationException', 'MethodException', 'ArgumentException'
    )) { $kind = 'other' }
    @{ status = 'unreadable'; collector_stage = $stage; collector_exception = $kind } |
        ConvertTo-Json -Compress
} finally {
    if ($started -and !$probe.HasExited) { $probe.Kill($true); $probe.WaitForExit() }
    $probe.Dispose()
    $browser.Dispose()
    $output.Dispose()
    $errors.Dispose()
}
