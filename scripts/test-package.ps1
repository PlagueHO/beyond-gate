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
  [string]$Target,
  [string]$ReportPath,
  [ValidatePattern('^([0-9a-f]{40}|[0-9a-f]{64})$')]
  [string]$SourceCommit
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$caseIds = @(
  'package.identity', 'archive.contents', 'archive.extraction', 'binary.startup',
  'browser.package', 'cli.contracts', 'mcp.protocol',
  'registration.vscode', 'registration.claude-code', 'registration.copilot-cli',
  'lifecycle.path', 'lifecycle.missing-sibling', 'lifecycle.reinstall',
  'lifecycle.relocation', 'lifecycle.removal', 'cleanup'
)
$cases = @($caseIds | ForEach-Object {
  [ordered]@{ id = $_; status = 'blocked'; reason = 'not_run' }
})
$currentCase = 'package.identity'
$temporary = $null
$actualHash = $null
$browserVersion = $null
$browserId = $null
$failures = [System.Collections.Generic.List[System.Management.Automation.ErrorRecord]]::new()

# Mark a complete group only after every assertion in that group succeeds.
function Complete-PackageCase {
  param([string]$Id)
  $row = $cases | Where-Object { $_.id -ceq $Id }
  $row.status = 'passed'
  $row.reason = 'verified'
}

# Store only fixed categories and public candidate metadata, never child output or paths.
function Write-PackageReport {
  if (!$ReportPath) { return }
  $offlinePassed = @($cases | Where-Object { $_.status -notin @('passed', 'not_applicable') }).Count -eq 0
  $manualCases = @(
    'desktop.clean-install', 'desktop.real-upgrade', 'desktop.minimum-runtime',
    'desktop.trust', 'native-store', 'client.vscode', 'client.claude-code',
    'client.copilot-cli', 'browser.chrome', 'browser.edge', 'account.reads'
  )
  $report = [ordered]@{
    schema = 1
    scope = 'offline-package'
    status = $(if ($offlinePassed) { 'passed' } else { 'failed' })
    exit_code = $(if ($offlinePassed) { 0 } else { 1 })
    full_coverage = $false
    candidate = [ordered]@{
      version = $Version; target = $Target; sha256 = $actualHash
      source_commit = $(if ($SourceCommit) { $SourceCommit } else { $null })
      source_identity = $(if ($SourceCommit) { 'supplied_not_attested' } else { 'unavailable' })
      extension_version = $browserVersion; extension_id = $browserId
    }
    host = [ordered]@{
      os = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
      os_version = [System.Environment]::OSVersion.Version.ToString()
      process_architecture = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
      powershell_version = $PSVersionTable.PSVersion.ToString()
    }
    command = 'pwsh -NoProfile -File .\scripts\test-package.ps1 -Archive <archive> -Checksum <checksum> -Version <version> -Target <target> -ReportPath <report>' +
      $(if ($SourceCommit) { ' -SourceCommit <source-commit>' } else { '' })
    cases = @($cases)
    manual = @($manualCases | ForEach-Object {
      [ordered]@{ id = $_; status = 'blocked'; reason = 'separate_evidence_required' }
    })
    live_request_budget = 0
    credential_access = $false
    real_clients = $false
  }
  $json = ConvertTo-Json $report -Depth 10
  $provider = $null
  $drive = $null
  $destination = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($ReportPath, [ref]$provider, [ref]$drive)
  if ($provider.Name -cne 'FileSystem') { throw 'Package evidence report requires a FileSystem destination.' }
  [System.IO.File]::WriteAllText($destination, "$json`n")
}

# Fail with the contract being checked, rather than a successful empty result.
function Assert-Package {
  param([bool]$Condition, [string]$Message)
  if (!$Condition) { throw $Message }
}

# Capture both streams concurrently and bound the lifetime of each child tree.
function Invoke-PackagedProcess {
  param(
    [string]$Executable, [string[]]$Arguments = @(), [string]$InputText = '',
    [string]$SearchPath = $clients
  )
  $start = [System.Diagnostics.ProcessStartInfo]::new($Executable)
  $start.UseShellExecute = $false
  $start.WorkingDirectory = $temporary
  $start.RedirectStandardInput = $true
  $start.RedirectStandardOutput = $true
  $start.RedirectStandardError = $true
  foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
  $start.Environment['PATH'] = $SearchPath
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
    @{name = 'character_list'; arguments = @{unexpected = $true}}
    @{name = 'character_show'; arguments = @{id = 'synthetic'; unexpected = $true}}
    @{name = 'campaign_list'; arguments = @{unexpected = $true}}
    @{name = 'campaign_show'; arguments = @{id = 'synthetic'; unexpected = $true}}
    @{name = 'class_list'; arguments = @{unexpected = $true}}
    @{name = 'class_show'; arguments = @{id = '10'; unexpected = $true}}
    @{name = 'book_list'; arguments = @{unexpected = $true}}
    @{name = 'book_outline'; arguments = @{book_id = 'synthetic'; unexpected = $true}}
    @{name = 'book_read'; arguments = @{book_id = 'synthetic'; section_id = 'opening'; unexpected = $true}}
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
  Assert-Package ($responses.Count -eq ($operations.Count + 2)) 'Expected initialization, discovery, and all resource tool responses.'
  foreach ($response in $responses) {
    Assert-Package ($response.jsonrpc -ceq '2.0') 'MCP stdout must contain protocol messages only.'
  }
  $initialized = @($responses | Where-Object { $_.id -eq 1 })
  Assert-Package ($initialized.Count -eq 1 -and $initialized[0].result.capabilities.ContainsKey('tools')) 'MCP initialization did not advertise tools.'
  $listed = @($responses | Where-Object { $_.id -eq 2 })
  Assert-Package ($listed.Count -eq 1) 'MCP tool discovery response is missing.'
  $tools = $listed[0].result.tools
  $expectedNames = @(@($operations.name) + @('auth_validate') | Sort-Object)
  $names = @($tools.name | Sort-Object)
  Assert-Package (($names -join ',') -ceq ($expectedNames -join ',')) 'Packaged MCP tool names differ from the CLI contract.'
  foreach ($tool in $tools) {
    Assert-Package ($tool.annotations.readOnlyHint -eq $true -and $tool.annotations.destructiveHint -eq $false) 'Packaged MCP tools must be read-only.'
  }
  $authValidate = $tools | Where-Object { $_.name -ceq 'auth_validate' }
  Assert-Package ($authValidate.annotations.idempotentHint -eq $false -and $authValidate.annotations.openWorldHint -eq $true) 'Packaged session validation annotations differ from the CLI contract.'
  $bookRead = $tools | Where-Object { $_.name -ceq 'book_read' }
  Assert-Package ($bookRead.inputSchema.properties.continuation.maxLength -eq 16384) 'MCP continuation input must remain bounded.'
  $bookList = $tools | Where-Object { $_.name -ceq 'book_list' }
  Assert-Package ($bookList.inputSchema.additionalProperties -eq $false) 'MCP library input must reject unknown fields.'
  for ($id = 3; $id -lt ($operations.Count + 3); $id++) {
    $response = @($responses | Where-Object { $_.id -eq $id })
    Assert-Package ($response.Count -eq 1 -and $response[0].result.isError -eq $true) 'Invalid MCP read input must return a tool error.'
    $text = $response[0].result.content | ConvertTo-Json -Depth 10 -Compress
    Assert-Package ($text -cmatch 'unknown field' -and $text -cmatch 'unexpected') 'MCP read input must fail before session or network access.'
  }
}

try {
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
  Complete-PackageCase 'package.identity'

  $currentCase = 'archive.contents'
  $suffix = if ($IsWindows) { '.exe' } else { '' }
  $expectedFiles = @("beyond-gate$suffix", "beyond-gate-mcp$suffix", 'README.md', 'LICENSE', 'CHANGELOG.md', 'installation.md')
  if ($IsWindows) { $expectedFiles += @('beyond-gate-native-host.exe', 'beyond-gate-browser.zip') }
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
  Complete-PackageCase 'archive.contents'

  $currentCase = 'archive.extraction'
  $temporary = Join-Path ([System.IO.Path]::GetTempPath()) "Beyond Gate package $([guid]::NewGuid().ToString('N'))"
  $extracted = Join-Path $temporary 'extracted package'
  $clients = Join-Path $temporary 'fake clients'
  $homePath = Join-Path $temporary 'synthetic home'
  $copilotHome = Join-Path $homePath 'copilot settings'
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
  Complete-PackageCase 'archive.extraction'
  $currentCase = 'binary.startup'
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
    Assert-Package ($result.Out -match '(?s)Character.*campaign.*read.*fixed.*bounded') "$name help must describe bounded character and campaign reads."
    $result = Invoke-PackagedProcess $binary @('--unknown')
    Assert-Package ($result.Code -eq 2 -and $result.Out -eq '' -and $result.Err -ne '') "$name invalid usage contract failed."
  }
  Complete-PackageCase 'binary.startup'
  $cli = Join-Path $extracted "beyond-gate$suffix"
  $server = Join-Path $extracted "beyond-gate-mcp$suffix"
  $currentCase = 'browser.package'
  if ($IsWindows) {
    $native = Join-Path $extracted 'beyond-gate-native-host.exe'
    $result = Invoke-PackagedProcess $native @('chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/')
    Assert-Package ($result.Code -eq 1 -and $result.Out -eq '' -and $result.Err.Contains('origin_rejected')) 'Native host must refuse another extension before input or endpoint access.'
    $browserZip = [System.IO.Compression.ZipFile]::OpenRead((Join-Path $extracted 'beyond-gate-browser.zip'))
    $browserNames = @('manifest.json', 'launch.mjs', 'flow.mjs', 'login.html', 'login.css', 'login.mjs')
    $browserSeen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    try {
      foreach ($entry in $browserZip.Entries) {
        Assert-Package ($browserNames -ccontains $entry.FullName) 'Unexpected or unsafe browser archive member.'
        Assert-Package ($browserSeen.Add($entry.FullName)) 'Duplicate browser archive member.'
        $fileType = ($entry.ExternalAttributes -shr 16) -band 0xF000
        Assert-Package ($fileType -in @(0, 0x8000) -and $entry.Length -gt 0 -and $entry.Length -le 65536) 'Browser archive members must be bounded nonempty regular files.'
      }
      Assert-Package ($browserSeen.Count -eq $browserNames.Count) 'Browser package is missing required files.'
      $reader = [System.IO.StreamReader]::new($browserZip.GetEntry('manifest.json').Open())
      try { $manifest = ConvertFrom-Json $reader.ReadToEnd() -AsHashtable }
      finally { $reader.Dispose() }
      Assert-Package ($manifest.version -ceq $Version) 'Browser version does not match the candidate version.'
      $browserVersion = $manifest.version
      Assert-Package ($manifest.manifest_version -eq 3 -and $manifest.incognito -ceq 'not_allowed') 'Browser package must use MV3 and refuse incognito.'
      Assert-Package (($manifest.permissions | Sort-Object) -join ',' -ceq 'cookies,nativeMessaging') 'Browser package permission scope changed.'
      Assert-Package (($manifest.host_permissions | Sort-Object) -join ',' -ceq 'https://dndbeyond.com/*,https://www.dndbeyond.com/*') 'Browser package host scope changed.'
      $hash = [System.Security.Cryptography.SHA256]::HashData([Convert]::FromBase64String($manifest.key))
      $id = -join (0..15 | ForEach-Object {
        [char](97 + ($hash[$_] -shr 4)); [char](97 + ($hash[$_] -band 15))
      })
      Assert-Package ($id -ceq 'gnhdconfgjfdbbgkemadegjcjnnodefh') 'Browser package identity does not match the native host.'
      $browserId = $id
      Assert-Package (!$manifest.ContainsKey('content_scripts') -and !$manifest.ContainsKey('web_accessible_resources')) 'Browser package must not expose scripts to websites.'
      Assert-Package ($manifest.content_security_policy.extension_pages.Contains("connect-src 'none'") -and $manifest.content_security_policy.extension_pages.Contains("script-src 'self'")) 'Browser package must prohibit remote connections and scripts.'
    }
    finally { $browserZip.Dispose() }
    Complete-PackageCase 'browser.package'
  } else {
    $row = $cases | Where-Object { $_.id -ceq 'browser.package' }
    $row.status = 'not_applicable'
    $row.reason = 'windows_only'
  }
  $currentCase = 'cli.contracts'
  $result = Invoke-PackagedProcess $cli
  Assert-Package ($result.Code -eq 2 -and $result.Out -eq '' -and $result.Err.Contains('Usage:')) 'CLI no-argument contract failed.'
  $result = Invoke-PackagedProcess $cli @('auth', 'login')
  $loginReason = if ($IsWindows) { 'requires interactive stdin, stdout, and stderr' } else { 'no working session handoff' }
  Assert-Package ($result.Code -eq 1 -and $result.Out -eq '' -and $result.Err.Contains($loginReason)) 'Redirected login must fail without collecting credentials.'
  # Reject invalid input before a read can open protected storage or contact its source.
  foreach ($arguments in @(
    ,@('character', 'list')
    ,@('character', 'show', 'synthetic')
    ,@('campaign', 'list')
    ,@('campaign', 'show', 'synthetic')
  )) {
    $result = Invoke-PackagedProcess $cli (@($arguments) + '--unexpected')
    Assert-Package ($result.Code -eq 2 -and $result.Out -eq '' -and $result.Err.Contains("unexpected argument '--unexpected'")) "Read argument validation failed: $arguments"
  }
  foreach ($arguments in @(
    ,@('book', 'outline', 'synthetic')
    ,@('book', 'read', 'synthetic', 'opening')
  )) {
    $result = Invoke-PackagedProcess $cli $arguments
    Assert-Package ($result.Code -eq 1 -and $result.Out -eq '' -and $result.Err.Contains('metadata is invalid')) "Invalid book selection must not access the session: $arguments"
  }
  if ($IsWindows) {
    # A valid library request can open the real OS store despite the isolated home.
    $result = Invoke-PackagedProcess $cli @('book', 'list', '--unexpected')
    Assert-Package ($result.Code -eq 2 -and $result.Out -eq '' -and $result.Err.Contains("unexpected argument '--unexpected'")) 'Invalid library input must fail before discovery.'
  } else {
    $result = Invoke-PackagedProcess $cli @('book', 'list', '--unexpected')
    Assert-Package ($result.Code -eq 2 -and $result.Out -eq '' -and $result.Err.Contains("unexpected argument '--unexpected'")) 'Invalid library input must fail before discovery.'
  }
  Complete-PackageCase 'cli.contracts'
  $currentCase = 'mcp.protocol'
  Assert-PackagedProtocol $server
  Assert-PackagedProtocol $cli @('mcp', 'start')
  Complete-PackageCase 'mcp.protocol'

  foreach ($name in @('code', 'claude', 'copilot')) {
    $filename = if ($IsWindows) { "$name.cmd" } else { $name }
    [System.IO.File]::WriteAllText((Join-Path $clients $filename), "# Synthetic client marker; never executed.`n")
  }
  foreach ($client in @('vscode', 'claude-code', 'copilot-cli')) {
    $currentCase = "registration.$client"
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
    Complete-PackageCase $currentCase
  }

  $currentCase = 'lifecycle.path'
  $searchPath = "$extracted$([System.IO.Path]::PathSeparator)$clients"
  $shell = if ($IsWindows) { Join-Path ([System.Environment]::SystemDirectory) 'cmd.exe' } else { '/bin/sh' }
  foreach ($name in @('beyond-gate', 'beyond-gate-mcp')) {
    $arguments = if ($IsWindows) { @('/d', '/c', "$name --version") } else { @('-c', "exec $name --version") }
    $result = Invoke-PackagedProcess -Executable $shell -Arguments $arguments -SearchPath $searchPath
    Assert-Package ($result.Code -eq 0 -and $result.Err -eq '' -and $result.Out.TrimEnd("`r", "`n") -ceq "$name $Version") 'Process-local PATH startup failed.'
  }
  Complete-PackageCase $currentCase

  $currentCase = 'lifecycle.missing-sibling'
  $heldServer = Join-Path $temporary "held-server$suffix"
  Move-Item -LiteralPath $server -Destination $heldServer
  try {
    $result = Invoke-PackagedProcess $cli @('mcp', 'start')
    Assert-Package ($result.Code -eq 1 -and $result.Out -eq '' -and $result.Err.Contains('sibling')) 'Missing sibling must produce an explicit error.'
  }
  finally { Move-Item -LiteralPath $heldServer -Destination $server }
  Complete-PackageCase $currentCase

  $currentCase = 'lifecycle.reinstall'
  $before = @{}
  $settingsBefore = @{}
  foreach ($settings in @((Join-Path $homePath '.claude.json'), (Join-Path $copilotHome 'mcp-config.json'))) {
    $settingsBefore[$settings] = [System.IO.File]::ReadAllText($settings)
  }
  foreach ($name in $expectedFiles) {
    $before[$name] = (Get-FileHash -LiteralPath (Join-Path $extracted $name) -Algorithm SHA256).Hash
  }
  if ($IsWindows) {
    [System.IO.Compression.ZipFile]::ExtractToDirectory($archivePath, $extracted, $true)
  } else {
    $file = [System.IO.File]::OpenRead($archivePath)
    $gzip = [System.IO.Compression.GZipStream]::new($file, [System.IO.Compression.CompressionMode]::Decompress)
    try { [System.Formats.Tar.TarFile]::ExtractToDirectory($gzip, $extracted, $true) }
    finally { $gzip.Dispose(); $file.Dispose() }
  }
  foreach ($name in $expectedFiles) {
    Assert-Package ((Get-FileHash -LiteralPath (Join-Path $extracted $name) -Algorithm SHA256).Hash -ceq $before[$name]) 'Same-candidate reinstall changed a package file.'
  }
  foreach ($settings in $settingsBefore.Keys) {
    Assert-Package ([System.IO.File]::ReadAllText($settings) -ceq $settingsBefore[$settings]) 'Same-candidate reinstall changed registration.'
  }
  $result = Invoke-PackagedProcess $cli @('--version')
  Assert-Package ($result.Code -eq 0 -and $result.Err -eq '' -and $result.Out.TrimEnd("`r", "`n") -ceq "beyond-gate $Version") 'Same-candidate reinstall startup failed.'
  Complete-PackageCase $currentCase

  $currentCase = 'lifecycle.relocation'
  $relocated = Join-Path $temporary 'relocated installation with spaces'
  Move-Item -LiteralPath $extracted -Destination $relocated
  $cli = Join-Path $relocated "beyond-gate$suffix"
  $oldServer = $server
  $server = Join-Path $relocated "beyond-gate-mcp$suffix"
  foreach ($settings in $settingsBefore.Keys) {
    Assert-Package ([System.IO.File]::ReadAllText($settings) -ceq $settingsBefore[$settings]) 'Moving an installation must not change registration.'
  }
  Assert-PackagedProtocol $cli @('mcp', 'start')
  foreach ($client in @('vscode', 'claude-code', 'copilot-cli')) {
    $settings = if ($client -ceq 'claude-code') { Join-Path $homePath '.claude.json' } else { Join-Path $copilotHome 'mcp-config.json' }
    # VS Code and Copilot CLI share settings, so restore the old registration for each case.
    $config = @{ keep = $true; mcpServers = @{
      other = @{ command = 'synthetic-other' }
      'beyond-gate' = @{ command = $oldServer; args = @() }
    }}
    if ($client -cne 'claude-code') { $config.mcpServers['beyond-gate'].type = 'local' }
    [System.IO.File]::WriteAllText($settings, (ConvertTo-Json $config -Depth 10))
    $original = [System.IO.File]::ReadAllText($settings)
    $result = Invoke-PackagedProcess $cli @('mcp', 'install', $client, '--dry-run')
    Assert-Package ($result.Code -eq 1 -and $result.Out -eq '' -and $result.Err.Contains('different server')) 'Relocation must not overwrite an old registration.'
    Assert-Package ([System.IO.File]::ReadAllText($settings) -ceq $original) 'Relocation conflict changed settings.'
    $config.mcpServers.Remove('beyond-gate')
    [System.IO.File]::WriteAllText($settings, (ConvertTo-Json $config -Depth 10))
    $result = Invoke-PackagedProcess $cli @('mcp', 'install', $client) "y`n"
    Assert-Package ($result.Code -eq 0 -and $result.Err -eq '') 'Explicit relocated registration failed.'
    $config = ConvertFrom-Json ([System.IO.File]::ReadAllText($settings)) -AsHashtable
    Assert-Package ($config.mcpServers['beyond-gate'].command -ceq $server -and $config.keep -eq $true -and $config.mcpServers.other.command -ceq 'synthetic-other') 'Relocated registration lost its path or unrelated settings.'
  }
  Complete-PackageCase $currentCase

  $currentCase = 'lifecycle.removal'
  foreach ($settings in @((Join-Path $homePath '.claude.json'), (Join-Path $copilotHome 'mcp-config.json'))) {
    $config = ConvertFrom-Json ([System.IO.File]::ReadAllText($settings)) -AsHashtable
    $config.mcpServers.Remove('beyond-gate')
    [System.IO.File]::WriteAllText($settings, (ConvertTo-Json $config -Depth 10))
    $config = ConvertFrom-Json ([System.IO.File]::ReadAllText($settings)) -AsHashtable
    Assert-Package (!$config.mcpServers.ContainsKey('beyond-gate') -and $config.keep -eq $true -and $config.mcpServers.other.command -ceq 'synthetic-other') 'Manual registration removal lost unrelated settings.'
  }
  Remove-Item -LiteralPath $relocated -Recurse -Force
  Assert-Package (!(Test-Path -LiteralPath $relocated) -and (Test-Path -LiteralPath $homePath)) 'Installation removal changed unrelated state.'
  Complete-PackageCase $currentCase
}
catch {
  $failures.Add($_)
  $row = $cases | Where-Object { $_.id -ceq $currentCase }
  $row.status = 'failed'
  $row.reason = 'check_failed'
}
finally {
  try {
    if ($temporary -and (Test-Path -LiteralPath $temporary)) {
      Remove-Item -LiteralPath $temporary -Recurse -Force
      Assert-Package (!(Test-Path -LiteralPath $temporary)) 'Owned package files remain after cleanup.'
    }
    Complete-PackageCase 'cleanup'
  }
  catch {
    $failures.Add($_)
    $row = $cases | Where-Object { $_.id -ceq 'cleanup' }
    $row.status = 'failed'
    $row.reason = 'cleanup_failed'
    Write-Error 'Owned package cleanup failed.' -ErrorAction Continue
  }
  try { Write-PackageReport }
  catch {
    $failures.Add($_)
    Write-Error 'Package evidence report could not be written.' -ErrorAction Continue
  }
}
if ($failures.Count -gt 0) { throw $failures[0] }
Write-Output "Verified extracted package: $archiveName"
