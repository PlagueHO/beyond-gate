#Requires -Version 5.1
# Check a Windows ZIP as a standard user without installing development or product runtimes.
[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$Archive,
  [Parameter(Mandatory)][string]$Checksum,
  [Parameter(Mandatory)][string]$Version,
  [Parameter(Mandatory)][string]$ReportPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class BeyondGateWindowsProtection {
  [DllImport("sfc.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool SfcIsFileProtected(IntPtr rpc, string path);
}
'@
$installation = $null
$report = [ordered]@{
  scope = 'clean-windows-startup'
  status = 'failed'
  version = $Version
  archive_sha256 = $null
  os = [Environment]::OSVersion.Version.ToString()
  x64 = [Environment]::Is64BitOperatingSystem -and [Environment]::Is64BitProcess
  powershell = $PSVersionTable.PSVersion.ToString()
  administrator = $null
  runtime_before = $null
  runtime_after = $null
  cases = @()
  cleanup = $false
  credential_access = $false
  live_request_budget = 0
}

# Stop at the first failed contract and retain the incomplete result.
function Assert-Portable {
  param([bool]$Condition, [string]$Message)
  if (!$Condition) { throw $Message }
}

# Recognize reviewed OS/CLR names only when Windows also confirms file protection.
function Test-PortableWindowsComponent {
  param([string]$Name, [bool]$WindowsProtected)
  return $WindowsProtected -and $Name -in @(
    'mfc40.dll', 'mfc40u.dll', 'mfc42.dll', 'mfc42u.dll', 'msvcp60.dll',
    'msvcp110_win.dll', 'msvcp120_clr0400.dll', 'msvcp140_clr0400.dll',
    'msvcr100_clr0400.dll', 'msvcr120_clr0400.dll',
    'vcruntime140_clr0400.dll', 'vcruntime140_1_clr0400.dll'
  )
}

# Check both installed products and runtime files available to the Windows loader.
function Get-RuntimeInventory {
  $products = @()
  foreach ($root in @(
    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
  )) {
    if (!(Test-Path -LiteralPath $root)) { continue }
    foreach ($key in Get-ChildItem -LiteralPath $root) {
      $name = $key.GetValue('DisplayName')
      if ($name -and $name -match 'Microsoft Visual C\+\+.*(?:Redistributable|Runtime)') {
        $products += $name
      }
    }
  }
  $files = @()
  $windowsComponents = @()
  foreach ($directory in @('System32', 'SysWOW64')) {
    $path = Join-Path $env:SystemRoot $directory
    foreach ($file in @(Get-ChildItem -LiteralPath $path -File | Where-Object {
      $_.Name -match '^(?:(?:vcruntime|msvcp|msvcr|concrt|vcomp|vcamp|vccorlib|mfc|mfcm|atl)\d[a-z0-9_]*|ucrtbased)\.dll$'
    })) {
      $protected = [BeyondGateWindowsProtection]::SfcIsFileProtected([IntPtr]::Zero, $file.FullName)
      if (Test-PortableWindowsComponent $file.Name $protected) {
        $windowsComponents += "$directory\$($file.Name)"
      }
      else { $files += "$directory\$($file.Name)" }
    }
  }
  $registered = @()
  foreach ($architecture in @('x64', 'x86')) {
    $key = "HKLM:\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\$architecture"
    if (Test-Path -LiteralPath $key) {
      if ((Get-Item -LiteralPath $key).GetValue('Installed') -eq 1) { $registered += $architecture }
    }
  }
  return @{
    products = @($products); files = @($files); registered = @($registered)
    windows_components = @($windowsComponents)
  }
}

# Keep binary framing intact on Windows PowerShell, with concurrent stream reads and a fixed process cap.
function Invoke-PortableProcess {
  param([string]$Executable, [string]$Arguments, [byte[]]$InputBytes = @())
  $start = New-Object System.Diagnostics.ProcessStartInfo
  $start.FileName = $Executable
  $start.Arguments = $Arguments
  $start.WorkingDirectory = $installation
  $start.UseShellExecute = $false
  $start.CreateNoWindow = $true
  $start.RedirectStandardInput = $true
  $start.RedirectStandardOutput = $true
  $start.RedirectStandardError = $true
  $start.EnvironmentVariables['PATH'] = Join-Path $env:SystemRoot 'System32'
  $process = New-Object System.Diagnostics.Process
  $process.StartInfo = $start
  $output = New-Object System.IO.MemoryStream
  $started = $false
  try {
    $started = $process.Start()
    Assert-Portable $started 'Product process did not start.'
    $stdout = $process.StandardOutput.BaseStream.CopyToAsync($output)
    $stderr = $process.StandardError.ReadToEndAsync()
    $process.StandardInput.BaseStream.Write($InputBytes, 0, $InputBytes.Length)
    $process.StandardInput.Close()
    if (!$process.WaitForExit(20000)) {
      $process.Kill()
      $process.WaitForExit()
      throw 'Product process exceeded its startup test limit.'
    }
    [void]$stdout.GetAwaiter().GetResult()
    return @{
      code = $process.ExitCode
      bytes = $output.ToArray()
      text = [Text.Encoding]::UTF8.GetString($output.ToArray())
      error = $stderr.GetAwaiter().GetResult()
    }
  }
  finally {
    if ($started -and !$process.HasExited) { $process.Kill(); $process.WaitForExit() }
    $process.Dispose()
    $output.Dispose()
  }
}

# Check the same startup contracts locally and on the clean guest.
function Assert-PortableStartup {
  param([string]$Version)
  foreach ($name in @('beyond-gate', 'beyond-gate-mcp')) {
    $binary = Join-Path $installation "$name.exe"
    $result = Invoke-PortableProcess $binary '--version'
    Assert-Portable ($result.code -eq 0 -and $result.error -ceq '' -and
      $result.text.TrimEnd("`r", "`n") -ceq "$name $Version") "$name version contract failed."
    $result = Invoke-PortableProcess $binary '--help'
    Assert-Portable ($result.code -eq 0 -and $result.error -ceq '' -and
      $result.text.Contains("Usage: $name")) "$name help contract failed."
    $report.cases += "$name.startup"
  }
  $inputText = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"portable-test","version":"0.1.0"}}}' + "`n" +
    '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' + "`n" +
    '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' + "`n"
  $result = Invoke-PortableProcess (Join-Path $installation 'beyond-gate-mcp.exe') '' ([Text.Encoding]::UTF8.GetBytes($inputText))
  Assert-Portable ($result.code -eq 0 -and $result.error -ceq '') 'MCP startup or shutdown failed.'
  $responses = @($result.text -split '\r?\n' | Where-Object { $_ } | ForEach-Object { ConvertFrom-Json $_ })
  Assert-Portable ($responses.Count -eq 2) 'Expected exactly two MCP protocol responses.'
  $initialize = @($responses | Where-Object { $_.id -eq 1 -and $_.jsonrpc -ceq '2.0' })
  $listed = @($responses | Where-Object { $_.id -eq 2 -and $_.jsonrpc -ceq '2.0' })
  Assert-Portable ($initialize.Count -eq 1 -and $listed.Count -eq 1 -and
    $null -ne $initialize[0].result.capabilities.tools -and
    $listed[0].result.tools.name -contains 'book_list' -and
    $listed[0].result.tools.name -contains 'auth_validate') 'MCP initialization or discovery contract failed.'
  $report.cases += 'mcp.initialize-and-discovery'

  $preface = [Text.Encoding]::UTF8.GetBytes('{"protocol":"bg-browser-handoff","version":1,"type":"pair","browser":"edge","profile":"cobalt-session-v1","consent":false,"key":"synthetic-invalid"}')
  $frame = [BitConverter]::GetBytes([uint32]$preface.Length) + $preface
  $result = Invoke-PortableProcess (Join-Path $installation 'beyond-gate-native-host.exe') 'chrome-extension://gnhdconfgjfdbbgkemadegjcjnnodefh/' $frame
  Assert-Portable ($result.code -eq 1 -and $result.error -ceq "error: consent_required`n" -and
    $result.bytes.Length -gt 4) 'Native-host refusal failed.'
  $length = [BitConverter]::ToUInt32($result.bytes, 0)
  Assert-Portable ($result.bytes.Length -eq $length + 4) 'Expected exactly one native-host response frame.'
  $response = ConvertFrom-Json ([Text.Encoding]::UTF8.GetString($result.bytes, 4, $length))
  Assert-Portable (@($response.PSObject.Properties).Count -eq 5 -and $response.protocol -ceq 'bg-browser-handoff' -and
    $response.version -eq 1 -and $response.type -ceq 'error' -and
    $null -eq $response.transaction -and $response.code -ceq 'consent_required') 'Native-host frame contract failed.'
  $report.cases += 'native-host.framed-refusal'
}

try {
  $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
  $report.administrator = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
  Assert-Portable (!$report.administrator -and $report.x64) 'Use a non-administrator Windows x64 process.'
  $report.runtime_before = Get-RuntimeInventory
  Assert-Portable (($report.runtime_before.products.Count + $report.runtime_before.files.Count +
    $report.runtime_before.registered.Count) -eq 0) 'The guest already has a separate Visual C++ runtime.'
  $report.cases += 'baseline.clean-standard-user'

  $archivePath = (Resolve-Path -LiteralPath $Archive).Path
  $line = [IO.File]::ReadAllText((Resolve-Path -LiteralPath $Checksum).Path)
  $match = [regex]::Match($line, '\A([0-9a-f]{64})  ([^\r\n]+)\r?\n?\z')
  $report.archive_sha256 = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
  Assert-Portable ($match.Success -and $match.Groups[2].Value -ceq [IO.Path]::GetFileName($archivePath) -and
    $match.Groups[1].Value -ceq $report.archive_sha256) 'Archive checksum verification failed.'
  $expected = @('beyond-gate.exe', 'beyond-gate-mcp.exe', 'beyond-gate-native-host.exe',
    'beyond-gate-browser.zip', 'README.md', 'LICENSE', 'CHANGELOG.md', 'installation.md')
  $zip = [IO.Compression.ZipFile]::OpenRead($archivePath)
  try {
    $names = @($zip.Entries | ForEach-Object { $_.FullName })
    Assert-Portable ((($names | Sort-Object -CaseSensitive) -join ',') -ceq
      (($expected | Sort-Object -CaseSensitive) -join ',')) 'Unexpected ZIP contents.'
    foreach ($entry in $zip.Entries) {
      Assert-Portable ($entry.Length -gt 0 -and (($entry.ExternalAttributes -shr 16) -band 0xF000) -in @(0, 0x8000)) 'ZIP entries must be regular nonempty files.'
    }
  }
  finally { $zip.Dispose() }
  $installation = Join-Path $env:LOCALAPPDATA "Programs\Beyond Gate portable $([guid]::NewGuid().ToString('N'))"
  [IO.Compression.ZipFile]::ExtractToDirectory($archivePath, $installation)
  $report.cases += 'archive.checksum-and-user-local-extraction'
  Assert-PortableStartup $Version
  $report.runtime_after = Get-RuntimeInventory
  Assert-Portable (($report.runtime_after.products.Count + $report.runtime_after.files.Count +
    $report.runtime_after.registered.Count) -eq 0) 'The runtime baseline changed during testing.'
  Assert-Portable ((($report.runtime_before.windows_components | Sort-Object) -join ',') -ceq
    (($report.runtime_after.windows_components | Sort-Object) -join ',')) 'The Windows component baseline changed during testing.'
  $report.cases += 'baseline.unchanged'
  $report.status = 'passed'
}
finally {
  try {
    if ($installation -and (Test-Path -LiteralPath $installation)) {
      Remove-Item -LiteralPath $installation -Recurse -Force
      Assert-Portable (!(Test-Path -LiteralPath $installation)) 'Test installation cleanup failed.'
    }
    $report.cleanup = $true
  }
  catch { $report.status = 'failed'; throw }
  finally {
    [IO.File]::WriteAllText($ReportPath, ($report | ConvertTo-Json -Depth 10))
  }
}
Write-Output 'Clean Windows standard-user package startup and protocol checks passed.'
