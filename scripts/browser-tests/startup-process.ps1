# Reports fixed startup state for an owned synthetic browser, never paths or command lines.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][int]$BrowserProcessId,
    [Parameter(Mandatory)][string]$BrowserExecutable,
    [Parameter(Mandatory)][string]$RunDirectory
)

$ErrorActionPreference = 'Stop'

# Reads a public build identity, never the embedded PDB path or process memory.
function Get-PublicBrowserSymbolId {
    param([string]$Path)

    $names = @{
        'chrome.exe' = 'chrome_exe'; 'chrome_elf.dll' = 'chrome_elf'
        'msedge.exe' = 'msedge_exe'; 'msedge_elf.dll' = 'msedge_elf'
    }
    $name = [IO.Path]::GetFileName($Path).ToLowerInvariant()
    if (!$names.ContainsKey($name)) { throw [ArgumentException]::new('Unsupported public browser module.') }
    $stream = $null
    $reader = $null
    try {
        $stream = [IO.FileStream]::new($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read,
            ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete))
        $reader = [Reflection.PortableExecutable.PEReader]::new($stream)
        $entries = @($reader.ReadDebugDirectory() | Where-Object {
            $_.Type -eq [Reflection.PortableExecutable.DebugDirectoryEntryType]::CodeView
        })
        if ($entries.Count -ne 1) {
            return @{ module = $names[$name]; status = 'codeview-unavailable' }
        }
        $data = $reader.ReadCodeViewDebugDirectoryData($entries[0])
        if ($data.Guid -eq [Guid]::Empty -or $data.Age -le 0) {
            return @{ module = $names[$name]; status = 'codeview-invalid' }
        }
        return @{
            module = $names[$name]; status = 'read'
            pdb_id = $data.Guid.ToString('N').ToUpperInvariant() +
                $data.Age.ToString('X', [Globalization.CultureInfo]::InvariantCulture)
        }
    } catch {
        $kind = $_.Exception.GetBaseException().GetType().Name
        if ($kind -cnotin @(
            'IOException', 'FileNotFoundException', 'DirectoryNotFoundException',
            'UnauthorizedAccessException', 'BadImageFormatException', 'ArgumentException'
        )) { $kind = 'other' }
        return @{ module = $names[$name]; status = 'unreadable'; cause_kind = $kind }
    } finally {
        if ($reader) { $reader.Dispose() }
        if ($stream) { $stream.Dispose() }
    }
}

if ($MyInvocation.InvocationName -eq '.') { return }

$browser = Get-Process -Id $BrowserProcessId
if ($browser.MainModule.FileName -ne $BrowserExecutable) {
    @{ status = 'identity-changed' } | ConvertTo-Json -Compress
    exit 0
}

$moduleInfo = @($browser.Modules)
$modules = @($moduleInfo | ForEach-Object { $_.ModuleName })
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
    public_module_symbols = @($moduleInfo | Where-Object {
        $_.ModuleName -cin @('chrome.exe', 'chrome_elf.dll', 'msedge.exe', 'msedge_elf.dll')
    } | Select-Object -First 2 | ForEach-Object { Get-PublicBrowserSymbolId $_.FileName })
    child_kinds = $kinds
    profile_created = Test-Path -LiteralPath (Join-Path $RunDirectory 'profile') -PathType Container
    crashpad_created = Test-Path -LiteralPath (Join-Path $RunDirectory 'profile\Crashpad') -PathType Container
    local_state_created = Test-Path -LiteralPath (Join-Path $RunDirectory 'profile\Local State') -PathType Leaf
} | ConvertTo-Json -Compress -Depth 4
