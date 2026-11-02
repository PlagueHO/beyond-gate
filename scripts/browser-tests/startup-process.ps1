# Reports fixed startup state for an owned synthetic browser, never paths or command lines.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][int]$BrowserProcessId,
    [Parameter(Mandatory)][string]$BrowserExecutable,
    [Parameter(Mandatory)][string]$RunDirectory
)

$ErrorActionPreference = 'Stop'
$browser = Get-Process -Id $BrowserProcessId
if ($browser.MainModule.FileName -ne $BrowserExecutable) {
    @{ status = 'identity-changed' } | ConvertTo-Json -Compress
    exit 0
}

$modules = @($browser.Modules | ForEach-Object { $_.ModuleName })
$threads = @($browser.Threads)
$waiting = @($threads | Where-Object { $_.ThreadState -eq 'Wait' })
$children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $BrowserProcessId" |
    Where-Object { $_.ExecutablePath -eq $BrowserExecutable })
$kinds = @($children | ForEach-Object {
    if ($_.CommandLine -match '(?:^| )--type=(crashpad-handler|gpu-process|renderer|utility)(?: |$)') {
        $Matches[1]
    } else {
        'other'
    }
} | Group-Object | ForEach-Object { @{ kind = $_.Name; count = $_.Count } })

@{
    status = 'read'
    cpu_ms = [math]::Round($browser.TotalProcessorTime.TotalMilliseconds)
    thread_count = $threads.Count
    waiting_thread_count = $waiting.Count
    wait_reasons = @($waiting | ForEach-Object { [int]$_.WaitReason } | Sort-Object -Unique)
    thread_states = @(foreach ($thread in ($threads | Select-Object -First 5)) {
        ,@([int]$thread.ThreadState,
            $(if ($thread.ThreadState -eq 'Wait') { [int]$thread.WaitReason } else { $null }))
    })
    chrome_dll_loaded = $modules -contains 'chrome.dll'
    chrome_elf_loaded = $modules -contains 'chrome_elf.dll'
    child_kinds = $kinds
    profile_created = Test-Path -LiteralPath (Join-Path $RunDirectory 'profile') -PathType Container
    crashpad_created = Test-Path -LiteralPath (Join-Path $RunDirectory 'profile\Crashpad') -PathType Container
    local_state_created = Test-Path -LiteralPath (Join-Path $RunDirectory 'profile\Local State') -PathType Leaf
} | ConvertTo-Json -Compress -Depth 4
