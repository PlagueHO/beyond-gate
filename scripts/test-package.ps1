#Requires -Version 7.4
# Verify a native archive without using a real account, credential store, or client.
# Requires PowerShell 7.4 or later for the .NET tar reader.
# All processes run from a temporary directory with isolated client settings.
[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$Archive,
  [Parameter(Mandatory)][string]$Checksum,
  [Parameter(Mandatory)][string]$Version,
  [Parameter(Mandatory)]
  [ValidateSet('x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc', 'aarch64-apple-darwin')]
  [string]$Target
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Fail with the contract being checked, rather than a successful empty result.
function Assert-Package {
  param([bool]$Condition, [string]$Message)
  if (!$Condition) { throw $Message }
}

# Capture both streams concurrently and bound the lifetime of each child tree.
function Invoke-PackagedProcess {
  param([string]$Executable, [string[]]$Arguments = @(), [string]$InputText = '')
  $start = [System.Diagnostics.ProcessStartInfo]::new($Executable)
  $start.UseShellExecute = $false
  $start.WorkingDirectory = $temporary
  $start.RedirectStandardInput = $true
  $start.RedirectStandardOutput = $true
  $start.RedirectStandardError = $true
  foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
  $start.Environment['PATH'] = $clients
  foreach ($name in @('HOME', 'USERPROFILE', 'APPDATA', 'LOCALAPPDATA', 'XDG_CONFIG_HOME')) {
    $start.Environment[$name] = $homePath
  }
  $start.Environment['COPILOT_HOME'] = $copilotHome
  $start.Environment['PATHEXT'] = '.CMD;.EXE'
  $process = [System.Diagnostics.Process]::new()
  $process.StartInfo = $start
  $started = $false
  try {
    $started = $process.Start()
    Assert-Package $started "Cannot start $Executable."
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    $process.StandardInput.Write($InputText)
    $process.StandardInput.Close()
    if (!$process.WaitForExit(20000)) {
      # This process and its companion are owned by this check, not by the user.
      $process.Kill($true)
      $process.WaitForExit()
      throw "Packaged process timed out: $Executable $Arguments"
    }
    return @{
      Code = $process.ExitCode
      Out = $stdout.GetAwaiter().GetResult()
      Err = $stderr.GetAwaiter().GetResult()
    }
  }
  finally {
    try {
      if ($started -and !$process.HasExited) {
        $process.Kill($true)
        $process.WaitForExit()
      }
    }
    finally { $process.Dispose() }
  }
}

# Check protocol behavior through either the server or the CLI companion launcher.
function Assert-PackagedProtocol {
  param([string]$Executable, [string[]]$Arguments = @())
  $messages = @(
    @{jsonrpc = '2.0'; id = 1; method = 'initialize'; params = @{
      protocolVersion = '2026-07-28'; capabilities = @{}
      clientInfo = @{name = 'beyond-gate-package-check'; version = '0.1.0'}
    }}
    @{jsonrpc = '2.0'; method = 'notifications/initialized'; params = @{}}
    @{jsonrpc = '2.0'; id = 2; method = 'tools/list'; params = @{}}
  )
  $operations = @(
    @{name = 'character_list'; arguments = @{}}
    @{name = 'character_show'; arguments = @{id = 'synthetic'}}
    @{name = 'campaign_list'; arguments = @{}}
    @{name = 'campaign_show'; arguments = @{id = 'synthetic'}}
    @{name = 'book_list'; arguments = @{}}
    @{name = 'book_outline'; arguments = @{book_id = 'synthetic'}}
    @{name = 'book_read'; arguments = @{book_id = 'synthetic'; section_id = 'opening'}}
  )
  for ($index = 0; $index -lt $operations.Count; $index++) {
    $messages += @{jsonrpc = '2.0'; id = $index + 3; method = 'tools/call'; params = $operations[$index]}
  }
  $inputText = ($messages | ForEach-Object { ConvertTo-Json $_ -Depth 10 -Compress }) -join "`n"
  $result = Invoke-PackagedProcess $Executable $Arguments "$inputText`n"
  Assert-Package ($result.Code -eq 0 -and $result.Err -eq '') "MCP startup or shutdown failed: $($result.Err)"
  $responses = @($result.Out -split '\r?\n' | Where-Object { $_ -ne '' } | ForEach-Object {
    ConvertFrom-Json -InputObject $_ -AsHashtable
  })
  Assert-Package ($responses.Count -eq 9) 'Expected initialization, discovery, and seven tool responses.'
  foreach ($response in $responses) {
    Assert-Package ($response.jsonrpc -ceq '2.0') 'MCP stdout must contain protocol messages only.'
  }
  $initialized = @($responses | Where-Object { $_.id -eq 1 })
  Assert-Package ($initialized.Count -eq 1 -and $initialized[0].result.capabilities.ContainsKey('tools')) 'MCP initialization did not advertise tools.'
  $listed = @($responses | Where-Object { $_.id -eq 2 })
  Assert-Package ($listed.Count -eq 1) 'MCP tool discovery response is missing.'
  $tools = $listed[0].result.tools
  $expectedNames = @($operations.name | Sort-Object)
  $names = @($tools.name | Sort-Object)
  Assert-Package (($names -join ',') -ceq ($expectedNames -join ',')) 'Packaged MCP tool names differ from the CLI contract.'
  foreach ($tool in $tools) {
    Assert-Package ($tool.annotations.readOnlyHint -eq $true -and $tool.annotations.destructiveHint -eq $false) 'Packaged MCP tools must be read-only.'
  }
  $bookRead = $tools | Where-Object { $_.name -ceq 'book_read' }
  Assert-Package ($bookRead.inputSchema.properties.continuation.maxLength -eq 16384) 'MCP continuation input must remain bounded.'
  for ($id = 3; $id -le 9; $id++) {
    $response = @($responses | Where-Object { $_.id -eq $id })
    Assert-Package ($response.Count -eq 1 -and $response[0].result.isError -eq $true) 'Unsupported MCP reads must return tool errors.'
    $text = $response[0].result.content | ConvertTo-Json -Depth 10 -Compress
    Assert-Package ($text -cmatch 'unsupported') 'MCP read failure must state the current implementation limit.'
  }
}

$archivePath = (Resolve-Path -LiteralPath $Archive).Path
$checksumPath = (Resolve-Path -LiteralPath $Checksum).Path
$extension = if ($IsWindows) { '.zip' } else { '.tar.gz' }
$archiveName = "beyond-gate-v$Version-$Target$extension"
Assert-Package ([System.IO.Path]::GetFileName($archivePath) -ceq $archiveName) 'Archive name does not match the expected version and target.'
$nativeTarget = if ($IsWindows) { 'x86_64-pc-windows-msvc' } elseif ($IsLinux) {
  'x86_64-unknown-linux-gnu'
} elseif ($IsMacOS) { 'aarch64-apple-darwin' } else { throw 'Unsupported package-check platform.' }
$architecture = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
$expectedArchitecture = if ($IsMacOS) { 'Arm64' } else { 'X64' }
Assert-Package ($Target -ceq $nativeTarget -and $architecture -ceq $expectedArchitecture) 'Package target does not match this native runner.'
$checksumText = [System.IO.File]::ReadAllText($checksumPath)
$checksumMatch = [regex]::Match($checksumText, '\A([0-9a-f]{64})  ([^\r\n]+)\r?\n?\z')
Assert-Package ($checksumMatch.Success -and $checksumMatch.Groups[2].Value -ceq $archiveName) 'Invalid SHA-256 sidecar format or archive filename.'
$actualHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
Assert-Package ($actualHash -ceq $checksumMatch.Groups[1].Value) 'Archive SHA-256 mismatch.'

$suffix = if ($IsWindows) { '.exe' } else { '' }
$expectedFiles = @("beyond-gate$suffix", "beyond-gate-mcp$suffix", 'README.md', 'LICENSE', 'CHANGELOG.md', 'installation.md')
$seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)

# Only flat regular files are permitted. Check before allowing an extractor to write.
function Assert-ArchiveMember {
  param([string]$Name)
  if ($Name.StartsWith('./', [System.StringComparison]::Ordinal)) { $Name = $Name.Substring(2) }
  Assert-Package ($expectedFiles -ccontains $Name) "Unexpected or unsafe archive member: $Name"
  Assert-Package ($seen.Add($Name)) "Duplicate archive member: $Name"
}

if ($IsWindows) {
  $zip = [System.IO.Compression.ZipFile]::OpenRead($archivePath)
  try {
    foreach ($entry in $zip.Entries) {
      Assert-ArchiveMember $entry.FullName
      $fileType = ($entry.ExternalAttributes -shr 16) -band 0xF000
      Assert-Package ($fileType -in @(0, 0x8000)) 'ZIP members must be regular files.'
      Assert-Package ($entry.Length -gt 0) 'Package files must not be empty.'
    }
  }
  finally { $zip.Dispose() }
} else {
  Add-Type -AssemblyName System.Formats.Tar
  $file = [System.IO.File]::OpenRead($archivePath)
  $gzip = [System.IO.Compression.GZipStream]::new($file, [System.IO.Compression.CompressionMode]::Decompress)
  $reader = [System.Formats.Tar.TarReader]::new($gzip)
  try {
    while ($null -ne ($entry = $reader.GetNextEntry())) {
      if ($entry.Name -ceq './' -and $entry.EntryType.ToString() -ceq 'Directory') { continue }
      Assert-ArchiveMember $entry.Name
      Assert-Package ($entry.EntryType.ToString() -in @('RegularFile', 'V7RegularFile')) 'Tar members must be regular files.'
      Assert-Package ($entry.Length -gt 0) 'Package files must not be empty.'
    }
  }
  finally { $reader.Dispose(); $gzip.Dispose(); $file.Dispose() }
}
Assert-Package ($seen.Count -eq $expectedFiles.Count) 'Package is missing required files.'

$temporary = Join-Path ([System.IO.Path]::GetTempPath()) "Beyond Gate package $([guid]::NewGuid().ToString('N'))"
$extracted = Join-Path $temporary 'extracted package'
$clients = Join-Path $temporary 'fake clients'
$homePath = Join-Path $temporary 'synthetic home'
$copilotHome = Join-Path $homePath 'copilot settings'
try {
  New-Item -ItemType Directory -Path $extracted, $clients, $copilotHome | Out-Null
  if ($IsWindows) {
    [System.IO.Compression.ZipFile]::ExtractToDirectory($archivePath, $extracted)
  } else {
    $file = [System.IO.File]::OpenRead($archivePath)
    $gzip = [System.IO.Compression.GZipStream]::new($file, [System.IO.Compression.CompressionMode]::Decompress)
    try { [System.Formats.Tar.TarFile]::ExtractToDirectory($gzip, $extracted, $false) }
    finally { $gzip.Dispose(); $file.Dispose() }
  }
  $files = @(Get-ChildItem -LiteralPath $extracted -File)
  Assert-Package ($files.Count -eq $expectedFiles.Count) 'Extracted package contents differ from the archive contract.'
  foreach ($name in @('beyond-gate', 'beyond-gate-mcp')) {
    $binary = Join-Path $extracted "$name$suffix"
    if (!$IsWindows) {
      $mode = [System.IO.File]::GetUnixFileMode($binary)
      Assert-Package (($mode -band [System.IO.UnixFileMode]::UserExecute) -ne 0) "$name lost its executable permission."
    }
    $result = Invoke-PackagedProcess $binary @('--version')
    Assert-Package ($result.Code -eq 0 -and $result.Err -eq '' -and $result.Out.TrimEnd("`r", "`n") -ceq "$name $Version") "$name version contract failed."
    $result = Invoke-PackagedProcess $binary @('--help')
    Assert-Package ($result.Code -eq 0 -and $result.Err -eq '' -and $result.Out.Contains("Usage: $name")) "$name help contract failed."
    Assert-Package ($result.Out -match 'not implemented|unsupported') "$name help must keep the implementation limit."
    $result = Invoke-PackagedProcess $binary @('--unknown')
    Assert-Package ($result.Code -eq 2 -and $result.Out -eq '' -and $result.Err -ne '') "$name invalid usage contract failed."
  }
  $cli = Join-Path $extracted "beyond-gate$suffix"
  $server = Join-Path $extracted "beyond-gate-mcp$suffix"
  $result = Invoke-PackagedProcess $cli
  Assert-Package ($result.Code -eq 2 -and $result.Out -eq '' -and $result.Err.Contains('Usage:')) 'CLI no-argument contract failed.'
  $result = Invoke-PackagedProcess $cli @('auth', 'login')
  Assert-Package ($result.Code -eq 1 -and $result.Out -eq '' -and $result.Err.Contains('no working session handoff')) 'Login must report its blocker without collecting credentials.'
  foreach ($arguments in @(
    ,@('character', 'list')
    ,@('character', 'show', 'synthetic')
    ,@('campaign', 'list')
    ,@('campaign', 'show', 'synthetic')
    ,@('book', 'list')
    ,@('book', 'outline', 'synthetic')
    ,@('book', 'read', 'synthetic', 'opening')
  )) {
    $result = Invoke-PackagedProcess $cli $arguments
    Assert-Package ($result.Code -eq 1 -and $result.Out -eq '' -and $result.Err.Contains('unsupported')) "Unsupported read contract failed: $arguments"
  }
  Assert-PackagedProtocol $server
  Assert-PackagedProtocol $cli @('mcp', 'start')

  foreach ($name in @('code', 'claude', 'copilot')) {
    $filename = if ($IsWindows) { "$name.cmd" } else { $name }
    [System.IO.File]::WriteAllText((Join-Path $clients $filename), "# Synthetic client marker; never executed.`n")
  }
  foreach ($client in @('vscode', 'claude-code', 'copilot-cli')) {
    $settings = if ($client -ceq 'claude-code') {
      Join-Path $homePath '.claude.json'
    } else { Join-Path $copilotHome 'mcp-config.json' }
    $original = '{"keep":true,"mcpServers":{"other":{"command":"synthetic-other"}}}'
    [System.IO.File]::WriteAllText($settings, $original)
    $result = Invoke-PackagedProcess $cli @('mcp', 'install', $client, '--dry-run')
    Assert-Package ($result.Code -eq 0 -and $result.Err -eq '' -and $result.Out.Contains('Proposed')) "$client preview failed."
    Assert-Package ([System.IO.File]::ReadAllText($settings) -ceq $original) "$client preview changed settings."
    $result = Invoke-PackagedProcess $cli @('mcp', 'install', $client) "n`n"
    Assert-Package ($result.Code -eq 0 -and [System.IO.File]::ReadAllText($settings) -ceq $original) "$client cancellation changed settings."
    $result = Invoke-PackagedProcess $cli @('mcp', 'install', $client) "y`n"
    Assert-Package ($result.Code -eq 0 -and $result.Err -eq '') "$client registration failed."
    $contents = [System.IO.File]::ReadAllText($settings)
    $config = ConvertFrom-Json $contents -AsHashtable
    Assert-Package ($config.keep -eq $true -and $config.mcpServers.other.command -ceq 'synthetic-other') "$client lost unrelated settings."
    $expectedEntry = @{command = $server; args = @()}
    if ($client -cne 'claude-code') { $expectedEntry.type = 'local' }
    $entry = $config.mcpServers['beyond-gate']
    Assert-Package ($entry.command -ceq $server -and $entry.args.Count -eq 0) "$client did not register the packaged sibling."
    Assert-Package (($entry.Keys | Sort-Object) -join ',' -ceq (($expectedEntry.Keys | Sort-Object) -join ',')) "$client registration contains unexpected fields."
    if ($client -cne 'claude-code') {
      Assert-Package ($entry.type -ceq 'local') "$client must use local stdio."
    }
    $result = Invoke-PackagedProcess $cli @('mcp', 'install', $client)
    Assert-Package ($result.Code -eq 0 -and $result.Err -eq '' -and [System.IO.File]::ReadAllText($settings) -ceq $contents) "$client repeated registration must be a no-op."
  }
  Write-Output "Verified extracted package: $archiveName"
}
finally {
  if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
}
