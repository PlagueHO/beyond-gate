#Requires -Version 7.4
# Regression checks for the archive verifier, using copies of one native package.
[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$Archive,
  [Parameter(Mandatory)][string]$Version,
  [Parameter(Mandatory)][string]$Target
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$archivePath = (Resolve-Path -LiteralPath $Archive).Path
$checker = Join-Path $PSScriptRoot 'test-package.ps1'
$temporary = Join-Path ([System.IO.Path]::GetTempPath()) "Beyond Gate rejection $([guid]::NewGuid().ToString('N'))"
$extension = if ($IsWindows) { '.zip' } else { '.tar.gz' }
$testArchive = Join-Path $temporary ([System.IO.Path]::GetFileName($archivePath))
$checksum = "$testArchive.sha256"
$reportPath = Join-Path $temporary 'evidence.json'

# Check decoded strings, including nested values and strings that embed private paths.
function Test-PrivateReportPath {
  param([AllowNull()][object]$Value, [string[]]$Paths)
  if ($Value -is [string]) {
    foreach ($path in $Paths) {
      if ($Value.Contains($path)) { return $true }
    }
  } elseif ($Value -is [System.Collections.IDictionary]) {
    foreach ($key in $Value.Keys) {
      if ((Test-PrivateReportPath $key $Paths) -or (Test-PrivateReportPath $Value[$key] $Paths)) {
        return $true
      }
    }
  } elseif ($Value -is [System.Collections.IEnumerable]) {
    foreach ($item in $Value) {
      if (Test-PrivateReportPath $item $Paths) { return $true }
    }
  }
  return $false
}

# Require each escaped path leak to fail without exposing it in diagnostics.
function Assert-ReportPrivacyRegression {
  $original = [System.IO.File]::ReadAllText($reportPath)
  try {
    foreach ($path in @($temporary, $archivePath)) {
      foreach ($value in @($path, "Embedded path: $path")) {
        $fixture = ConvertFrom-Json $original -AsHashtable
        $fixture['privacy_regression'] = @{ nested = @(@{ value = $value }) }
        [System.IO.File]::WriteAllText($reportPath, (ConvertTo-Json $fixture -Depth 20))
        $rejected = $false
        try { $null = Assert-TestReport 'passed' -WithSource }
        catch {
          if ($_.Exception.Message -cne 'Package report contains private paths, settings, or child output.') { throw }
          $rejected = $true
        }
        if (!$rejected) { throw 'Package report accepted a private path.' }
      }
    }
    $fixture = ConvertFrom-Json $original -AsHashtable
    $fixture['privacy_regression'] = @{ nested = @(@{ value = 'Safe package evidence.' }) }
    [System.IO.File]::WriteAllText($reportPath, (ConvertTo-Json $fixture -Depth 20))
    $null = Assert-TestReport 'passed' -WithSource
  }
  finally {
    [System.IO.File]::WriteAllText($reportPath, $original)
  }
  Write-Output 'Verified escaped private path rejection and safe nested report values.'
}

# Check evidence independently of raw process diagnostics and temporary paths.
function Assert-TestReport {
  param([string]$Status, [string]$FailedCase = '', [switch]$WithSource, [switch]$CleanupFailed)
  $text = [System.IO.File]::ReadAllText($reportPath)
  $report = ConvertFrom-Json $text -AsHashtable
  if ($report.schema -ne 1 -or $report.scope -cne 'offline-package' -or
      $report.status -cne $Status -or $report.full_coverage -ne $false -or
      $report.exit_code -ne $(if ($Status -ceq 'passed') { 0 } else { 1 }) -or
      $report.live_request_budget -ne 0 -or $report.credential_access -ne $false -or
      $report.real_clients -ne $false) {
    throw 'Package report has an invalid scope or result.'
  }
  $expectedIds = @(
    'package.identity', 'archive.contents', 'archive.extraction', 'binary.startup',
    'browser.package', 'cli.contracts', 'mcp.protocol',
    'registration.vscode', 'registration.claude-code', 'registration.copilot-cli',
    'lifecycle.path', 'lifecycle.missing-sibling', 'lifecycle.reinstall',
    'lifecycle.relocation', 'lifecycle.removal', 'cleanup'
  )
  if (($report.cases.id -join ',') -cne ($expectedIds -join ',') -or
      @($report.manual).Count -ne 11 -or
      @($report.manual | Where-Object { $_.status -cne 'blocked' -or $_.reason -cne 'separate_evidence_required' }).Count -ne 0) {
    throw 'Package report is missing required offline or manual cases.'
  }
  foreach ($row in $report.cases) {
    if ($row.status -cnotin @('passed', 'failed', 'blocked', 'not_applicable') -or
        $row.reason -cnotin @('verified', 'check_failed', 'not_run', 'windows_only', 'cleanup_failed')) {
      throw 'Package report contains an unsafe result category.'
    }
  }
  $cleanup = $report.cases | Where-Object id -CEQ 'cleanup'
  if ($CleanupFailed) {
    if ($cleanup.status -cne 'failed' -or $cleanup.reason -cne 'cleanup_failed') {
      throw 'Package report did not record the cleanup failure.'
    }
  } elseif ($cleanup.status -cne 'passed') {
    throw 'Package report did not confirm owned cleanup.'
  }
  if ($Status -ceq 'passed' -and @($report.cases | Where-Object { $_.status -notin @('passed', 'not_applicable') }).Count -ne 0) {
    throw 'Incomplete package checks were reported as passed.'
  }
  if ($FailedCase -and ($report.cases | Where-Object id -CEQ $FailedCase).status -cne 'failed') {
    throw 'Package rejection did not record the failed case.'
  }
  if ($WithSource) {
    if ($report.candidate.source_commit -cne ('a' * 40) -or $report.candidate.source_identity -cne 'supplied_not_attested') {
      throw 'Package report changed the supplied test source identity.'
    }
  } elseif ($null -ne $report.candidate.source_commit -or $report.candidate.source_identity -cne 'unavailable') {
    throw 'Package report invented a source identity.'
  }
  if (!$report.host.os -or !$report.host.os_version -or !$report.host.process_architecture -or
      !$report.host.powershell_version -or !$report.command.Contains('-Archive <archive>')) {
    throw 'Package report is missing host or command evidence.'
  }
  if ((Test-PrivateReportPath $report @($temporary, $archivePath)) -or $text.Contains('synthetic-other') -or
      $text.Contains('StandardOutput') -or $text.Contains('StandardError')) {
    throw 'Package report contains private paths, settings, or child output.'
  }
  return $report
}

# Produce a correctly formatted sidecar for the current test bytes.
function Write-TestChecksum {
  param([string]$Path)
  $hash = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
  [System.IO.File]::WriteAllText("$Path.sha256", "$hash  $([System.IO.Path]::GetFileName($Path))`n")
}

# A rejected fixture must fail for the intended reason, not an unrelated exception.
function Assert-RejectedPackage {
  param(
    [string]$Path, [string]$Expected, [string]$ExpectedVersion = $Version,
    [string]$ExpectedTarget = $Target, [string]$FailedCase = 'package.identity'
  )
  if (Test-Path -LiteralPath $reportPath) { Remove-Item -LiteralPath $reportPath }
  try {
    & $checker -Archive $Path -Checksum "$Path.sha256" -Version $ExpectedVersion -Target $ExpectedTarget -ReportPath $reportPath
  }
  catch {
    if (!$_.Exception.Message.Contains($Expected)) {
      throw "Expected rejection '$Expected', received: $($_.Exception.Message)"
    }
    $null = Assert-TestReport 'failed' $FailedCase
    Write-Output "Verified rejection: $Expected"
    return
  }
  throw "Invalid package was accepted: $Expected"
}

# Rebuild a fixture with the same flat layout and native permissions as packaging.
function Write-TestArchive {
  param([string]$Directory)
  Remove-Item -LiteralPath $testArchive
  if ($IsWindows) {
    [System.IO.Compression.ZipFile]::CreateFromDirectory($Directory, $testArchive)
  } else {
    & tar -czf $testArchive -C $Directory .
    if ($LASTEXITCODE -ne 0) { throw 'Cannot create rejection fixture.' }
  }
  Write-TestChecksum $testArchive
}

# Inject one final cleanup error only within this function's scope, then remove its exact owned path.
function Assert-CleanupFailure {
  param(
    [string]$CheckArchive = $testArchive, [string]$CheckVersion = $Version,
    [string]$ExpectedError = 'synthetic_cleanup_failure', [string]$FailedCase = 'cleanup'
  )
  $cleanupTargets = [System.Collections.Generic.List[string]]::new()
  function Remove-Item {
    param([string]$LiteralPath, [switch]$Recurse, [switch]$Force)
    if ([System.IO.Path]::GetFileName($LiteralPath) -cmatch '^Beyond Gate package [0-9a-f]{32}$') {
      $cleanupTargets.Add($LiteralPath)
      throw 'synthetic_cleanup_failure'
    }
    Microsoft.PowerShell.Management\Remove-Item @PSBoundParameters
  }
  $rejected = $false
  $cleanupErrors = @()
  try {
    try {
      & $checker -Archive $CheckArchive -Checksum "$CheckArchive.sha256" -Version $CheckVersion -Target $Target `
        -ReportPath $reportPath -ErrorVariable cleanupErrors
    } catch {
      if (!$_.Exception.Message.Contains($ExpectedError)) { throw }
      $rejected = $true
    }
    if (!$rejected -or $cleanupTargets.Count -ne 1 -or
        !(@($cleanupErrors) | Where-Object { $_.ToString().Contains('Owned package cleanup failed.') })) {
      throw 'A cleanup failure did not produce an explicit package failure.'
    }
    $null = Assert-TestReport 'failed' $FailedCase -CleanupFailed
    Write-Output 'Verified cleanup failure cannot produce package success.'
  }
  finally {
    foreach ($path in $cleanupTargets) {
      Microsoft.PowerShell.Management\Remove-Item -LiteralPath $path -Recurse -Force
      if (Test-Path -LiteralPath $path) { throw 'Cleanup rejection fixture remains.' }
    }
  }
}

# Resolve new report files at the PowerShell location, not the process directory.
function Assert-ReportDestination {
  $fixture = Join-Path (Split-Path $PSScriptRoot -Parent) "target\report destination $([guid]::NewGuid().ToString('N'))"
  $processLocation = Join-Path $fixture 'process location'
  $shellLocation = Join-Path $fixture 'shell location'
  $savedDirectory = [System.Environment]::CurrentDirectory
  $locationPushed = $false
  try {
    New-Item -ItemType Directory -Path $processLocation, $shellLocation | Out-Null
    Push-Location -LiteralPath $shellLocation
    $locationPushed = $true
    [System.Environment]::CurrentDirectory = $processLocation
    $destinations = @(
      @{ Path = '.\relative evidence.json'; File = Join-Path $shellLocation 'relative evidence.json' }
      @{ Path = Join-Path $shellLocation 'absolute evidence.json'; File = Join-Path $shellLocation 'absolute evidence.json' }
      @{ Path = '.\missing parent\evidence.json'; File = $null }
      @{ Path = 'Env:\BEYOND_GATE_REPORT_DESTINATION_TEST'; File = $null }
      @{ Path = 'BeyondGateMissingReportDrive:\evidence.json'; File = $null }
    )
    foreach ($destination in $destinations) {
      $rejected = $false
      $reportErrors = @()
      try {
        & $checker -Archive '.\report-destination-missing-archive.zip' -Checksum '.\missing.sha256' `
          -Version $Version -Target $Target -ReportPath $destination.Path -ErrorVariable reportErrors
      } catch {
        if (!$_.Exception.Message.Contains('report-destination-missing-archive.zip')) { throw }
        $rejected = $true
      }
      if (!$rejected) { throw 'Missing archive was accepted during report destination checks.' }
      $writeFailed = @($reportErrors | Where-Object { $_.ToString().Contains('Package evidence report could not be written.') }).Count -gt 0
      if ($destination.File) {
        if ($writeFailed -or !(Test-Path -LiteralPath $destination.File)) {
          throw 'Report was not written at the requested PowerShell destination.'
        }
        $report = ConvertFrom-Json ([System.IO.File]::ReadAllText($destination.File))
        if ($report.status -cne 'failed' -or ($report.cases | Where-Object id -CEQ 'package.identity').status -cne 'failed') {
          throw 'Destination check report did not preserve the archive rejection.'
        }
      } elseif (!$writeFailed) {
        throw 'Invalid report destination did not produce an explicit write failure.'
      }
    }
    if (@(Get-ChildItem -LiteralPath $processLocation -Force).Count -ne 0) {
      throw 'Report destination checks wrote into the process directory.'
    }
    Write-Output 'Verified relative and absolute report destinations and explicit destination failures.'
  }
  finally {
    [System.Environment]::CurrentDirectory = $savedDirectory
    if ($locationPushed) { Pop-Location }
    if (Test-Path -LiteralPath $fixture) { Remove-Item -LiteralPath $fixture -Recurse -Force }
  }
}

try {
  New-Item -ItemType Directory -Path $temporary | Out-Null
  Copy-Item -LiteralPath $archivePath -Destination $testArchive
  Write-TestChecksum $testArchive
  & $checker -Archive $testArchive -Checksum $checksum -Version $Version -Target $Target `
    -ReportPath $reportPath -SourceCommit ('a' * 40)
  $report = Assert-TestReport 'passed' -WithSource
  Assert-ReportPrivacyRegression
  $hash = (Get-FileHash -LiteralPath $testArchive -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($report.candidate.sha256 -cne $hash -or $report.candidate.version -cne $Version -or $report.candidate.target -cne $Target) {
    throw 'Package report does not identify the tested candidate.'
  }
  if ($IsWindows -and ($report.candidate.extension_version -cne $Version -or $report.candidate.extension_id -cne 'gnhdconfgjfdbbgkemadegjcjnnodefh')) {
    throw 'Package report does not identify the tested extension.'
  }
  Write-Output 'Verified offline evidence shape, lifecycle cases, candidate identity, and manual blockers.'
  $validChecksum = [System.IO.File]::ReadAllText($checksum)
  [System.IO.File]::WriteAllText($checksum, 'invalid checksum')
  Assert-RejectedPackage $testArchive 'Invalid SHA-256 sidecar'
  [System.IO.File]::WriteAllText($checksum, $validChecksum.Replace([System.IO.Path]::GetFileName($testArchive), 'different.zip'))
  Assert-RejectedPackage $testArchive 'Invalid SHA-256 sidecar'
  [System.IO.File]::WriteAllText($checksum, $validChecksum)
  $stream = [System.IO.File]::Open($testArchive, [System.IO.FileMode]::Open, [System.IO.FileAccess]::ReadWrite)
  try {
    $stream.Position = $stream.Length - 1
    $byte = $stream.ReadByte()
    $stream.Position--
    $stream.WriteByte($byte -bxor 1)
  }
  finally { $stream.Dispose() }
  Assert-RejectedPackage $testArchive 'Archive SHA-256 mismatch'
  Copy-Item -LiteralPath $archivePath -Destination $testArchive -Force
  Write-TestChecksum $testArchive

  $wrongVersion = "$Version-rejection"
  $wrongVersionPath = Join-Path $temporary "beyond-gate-v$wrongVersion-$Target$extension"
  Copy-Item -LiteralPath $archivePath -Destination $wrongVersionPath
  Write-TestChecksum $wrongVersionPath
  Assert-RejectedPackage $wrongVersionPath 'version contract failed' $wrongVersion $Target 'binary.startup'
  $wrongTarget = if ($IsWindows) { 'x86_64-unknown-linux-gnu' } else { 'x86_64-pc-windows-msvc' }
  $wrongTargetPath = Join-Path $temporary "beyond-gate-v$Version-$wrongTarget$extension"
  Copy-Item -LiteralPath $archivePath -Destination $wrongTargetPath
  Write-TestChecksum $wrongTargetPath
  Assert-RejectedPackage $wrongTargetPath 'Package target does not match' $Version $wrongTarget

  $contents = Join-Path $temporary 'fixture contents'
  New-Item -ItemType Directory -Path $contents | Out-Null
  if ($IsWindows) {
    [System.IO.Compression.ZipFile]::ExtractToDirectory($archivePath, $contents)
  } else {
    & tar -xzf $archivePath -C $contents
    if ($LASTEXITCODE -ne 0) { throw 'Cannot extract rejection fixture.' }
  }
  $guide = Join-Path $contents 'installation.md'
  $guideText = [System.IO.File]::ReadAllText($guide)
  Remove-Item -LiteralPath $guide
  Write-TestArchive $contents
  Assert-RejectedPackage $testArchive 'Package is missing required files' $Version $Target 'archive.contents'
  [System.IO.File]::WriteAllText($guide, $guideText)
  [System.IO.File]::WriteAllText((Join-Path $contents 'unexpected.txt'), 'synthetic')
  Write-TestArchive $contents
  Assert-RejectedPackage $testArchive 'Unexpected or unsafe archive member' $Version $Target 'archive.contents'
  Remove-Item -LiteralPath (Join-Path $contents 'unexpected.txt')
  [System.IO.File]::WriteAllText($guide, '')
  Write-TestArchive $contents
  Assert-RejectedPackage $testArchive 'Package files must not be empty' $Version $Target 'archive.contents'
  [System.IO.File]::WriteAllText($guide, $guideText)
  if ($IsWindows) {
    $browserArchive = Join-Path $contents 'beyond-gate-browser.zip'
    $browserBytes = [System.IO.File]::ReadAllBytes($browserArchive)
    $zip = [System.IO.Compression.ZipFile]::Open($browserArchive, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
      $entry = $zip.CreateEntry('../unexpected.txt')
      $writer = [System.IO.StreamWriter]::new($entry.Open())
      try { $writer.Write('invented') } finally { $writer.Dispose() }
    } finally { $zip.Dispose() }
    Write-TestArchive $contents
    Assert-RejectedPackage $testArchive 'Unexpected or unsafe browser archive member' $Version $Target 'browser.package'
    [System.IO.File]::WriteAllBytes($browserArchive, $browserBytes)
    $zip = [System.IO.Compression.ZipFile]::Open($browserArchive, [System.IO.Compression.ZipArchiveMode]::Update)
    try { $zip.GetEntry('flow.mjs').Delete() } finally { $zip.Dispose() }
    Write-TestArchive $contents
    Assert-RejectedPackage $testArchive 'Browser package is missing required files' $Version $Target 'browser.package'
    [System.IO.File]::WriteAllBytes($browserArchive, $browserBytes)
    $zip = [System.IO.Compression.ZipFile]::Open($browserArchive, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
      $entry = $zip.GetEntry('manifest.json')
      $reader = [System.IO.StreamReader]::new($entry.Open())
      try { $manifest = ConvertFrom-Json $reader.ReadToEnd() -AsHashtable }
      finally { $reader.Dispose() }
      $entry.Delete()
      $manifest.version = '0.0.0'
      $writer = [System.IO.StreamWriter]::new($zip.CreateEntry('manifest.json').Open())
      try { $writer.Write((ConvertTo-Json $manifest -Depth 10)) }
      finally { $writer.Dispose() }
    } finally { $zip.Dispose() }
    Write-TestArchive $contents
    Assert-RejectedPackage $testArchive 'Browser version does not match' $Version $Target 'browser.package'
  }

  # A report write failure must fail the invocation even after all package checks pass.
  Copy-Item -LiteralPath $archivePath -Destination $testArchive -Force
  Write-TestChecksum $testArchive
  Assert-CleanupFailure
  Assert-CleanupFailure $wrongVersionPath $wrongVersion 'version contract failed' 'binary.startup'
  $reportFailure = $false
  $writeErrors = @()
  try {
    & $checker -Archive $testArchive -Checksum $checksum -Version $Version -Target $Target `
      -ReportPath $temporary -ErrorVariable writeErrors
  } catch { $reportFailure = $true }
  if (!$reportFailure -or !(@($writeErrors) | Where-Object { $_.ToString().Contains('Package evidence report could not be written.') })) {
    throw 'An unwritable report did not produce an explicit failure.'
  }
  Write-Output 'Verified report-write failure cannot produce package success.'
  [System.IO.File]::WriteAllText($checksum, 'invalid checksum')
  $originalFailure = $false
  try {
    & $checker -Archive $testArchive -Checksum $checksum -Version $Version -Target $Target -ReportPath $temporary
  } catch {
    if (!$_.Exception.Message.Contains('Invalid SHA-256 sidecar')) { throw }
    $originalFailure = $true
  }
  if (!$originalFailure) { throw 'Report failure masked the original package failure.' }
  Write-Output 'Verified report failure preserves the original package failure.'
  Assert-ReportDestination
}
finally {
  if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
}
