#Requires -Version 5.1
# Check clean-guest process capture and reporting under built-in Windows PowerShell.
[CmdletBinding()]
param(
  [string]$BinaryDirectory,
  [string]$Version = '0.1.0'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$tokens = $null
$errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile(
  (Join-Path $PSScriptRoot 'test-windows-portable.ps1'), [ref]$tokens, [ref]$errors)
if ($errors.Count -gt 0) { throw ($errors | Out-String) }
foreach ($name in @('Assert-Portable', 'Write-PortableReport', 'Invoke-PortableProcess', 'Assert-PortableStartup')) {
  $definition = @($ast.FindAll({
    param($node)
    $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq $name
  }, $false))
  if ($definition.Count -ne 1) { throw "Expected one process-test function: $name" }
  . ([scriptblock]::Create($definition[0].Extent.Text))
}
# Exercise the real finalization with a synthetic body, without clean-host checks.
$acceptance = @($ast.EndBlock.Statements | Where-Object {
  $_ -is [Management.Automation.Language.TryStatementAst]
})
if ($acceptance.Count -ne 1) { throw 'Expected one acceptance try statement.' }
$body = $acceptance[0].Body.Extent
$acceptanceText = $acceptance[0].Extent.Text
$acceptanceText = $acceptanceText.Remove(
  ($body.StartOffset - $acceptance[0].Extent.StartOffset), $body.Text.Length).Insert(
  ($body.StartOffset - $acceptance[0].Extent.StartOffset),
  '{ if ($failCheck) { throw "Synthetic acceptance failure." }; $report.status = "passed" }')
$failureCheck = @($ast.EndBlock.Statements | Where-Object {
  $_ -is [Management.Automation.Language.IfStatementAst]
})
if ($failureCheck.Count -ne 1) { throw 'Expected one acceptance failure check.' }
$reportCheck = [scriptblock]::Create("$acceptanceText`n$($failureCheck[0].Extent.Text)")
$installation = Join-Path (Split-Path $PSScriptRoot -Parent) "target\portable process $([guid]::NewGuid().ToString('N'))"
$inputEncoding = [Console]::InputEncoding
try {
  New-Item -ItemType Directory -Path $installation | Out-Null
  # Keep the child independent of shell encoding so only the capture helper can add bytes.
  # The compiler is part of the built-in .NET Framework used by Windows PowerShell.
  $source = Join-Path $installation 'capture.cs'
  $executable = Join-Path $installation 'capture.exe'
  [IO.File]::WriteAllText($source, @'
using System;
using System.Text;
public static class CaptureFixture {
  /// Echo stdin bytes unchanged, write fixed stderr bytes, and return a nonzero exit code.
  public static int Main() {
    Console.OpenStandardInput().CopyTo(Console.OpenStandardOutput());
    byte[] error = Encoding.ASCII.GetBytes("synthetic-error");
    Console.OpenStandardError().Write(error, 0, error.Length);
    return 7;
  }
}
'@)
  $compiler = Join-Path $env:SystemRoot 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
  & $compiler /nologo /target:exe "/out:$executable" $source
  if ($LASTEXITCODE -ne 0) { throw 'Cannot build the raw-stream process fixture.' }
  foreach ($withBom in @($false, $true)) {
    [Console]::InputEncoding = New-Object Text.UTF8Encoding($withBom)
    $expectedPreamble = [Console]::InputEncoding.GetPreamble() -join ','
    foreach ($inputBytes in @(
      [byte[]]@(0, 10, 65, 128, 255),
      [byte[]]@(239, 187, 191, 0, 10, 65, 128, 255)
    )) {
      $results = @(Invoke-PortableProcess $executable '' $inputBytes)
      Assert-Portable ($results.Count -eq 1 -and $results[0] -is [hashtable]) 'Process capture must return exactly one result, without task objects.'
      $result = $results[0]
      Assert-Portable ($result.code -eq 7) "Synthetic process exit changed: $($result.code)"
      Assert-Portable ($result.error -ceq 'synthetic-error') (
        "Synthetic process diagnostics changed (UTF-8 base64): $([Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($result.error)))")
      Assert-Portable (($result.bytes -join ',') -ceq ($inputBytes -join ',')) (
        "Synthetic process binary output changed: $($result.bytes -join ',')")
      Assert-Portable ($result.text -ceq [Text.Encoding]::UTF8.GetString($inputBytes)) (
        "Synthetic process text changed (UTF-8 base64): $([Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($result.text)))")
      Assert-Portable ([Console]::InputEncoding.CodePage -eq 65001 -and
        ([Console]::InputEncoding.GetPreamble() -join ',') -ceq $expectedPreamble) 'Process capture changed the parent console input encoding.'
    }
  }
  $originalLocation = Get-Location
  $processDirectory = [Environment]::CurrentDirectory
  $reportDirectory = Join-Path $installation 'reports'
  New-Item -ItemType Directory -Path $reportDirectory | Out-Null
  try {
    Set-Location -LiteralPath $reportDirectory
    [Environment]::CurrentDirectory = $installation
    foreach ($case in @(
      @{ path = 'relative-report.json'; fail = $false; file = 'relative-report.json'; error = $null },
      @{ path = 'relative-failure.json'; fail = $true; file = 'relative-failure.json'; error = 'Synthetic acceptance failure.' },
      @{ path = "FileSystem::$reportDirectory\provider-report.json"; fail = $false; file = 'provider-report.json'; error = $null },
      @{ path = 'missing\report.json'; fail = $true; file = $null; error = 'Synthetic acceptance failure.' },
      @{ path = 'Env:\BEYOND_GATE_SYNTHETIC_REPORT'; fail = $false; file = $null; error = 'Portable acceptance report requires a FileSystem destination.' },
      @{ path = 'Env:\BEYOND_GATE_SYNTHETIC_REPORT'; fail = $true; file = $null; error = 'Synthetic acceptance failure.' },
      @{ path = 'cleanup-failure.json'; fail = $true; cleanupFail = $true; file = 'cleanup-failure.json'; error = 'Synthetic acceptance failure.' },
      @{ path = 'cleanup-only.json'; fail = $false; cleanupFail = $true; file = 'cleanup-only.json'; error = 'Synthetic cleanup failure.' }
    )) {
      # Use a child scope so report cleanup cannot remove the process fixture.
      & {
        $installation = $null
        $report = [ordered]@{ status = 'failed'; cleanup = $false }
        $failures = [System.Collections.Generic.List[System.Management.Automation.ErrorRecord]]::new()
        $ReportPath = $case.path
        $failCheck = $case.fail
        $cleanupFailed = $case.ContainsKey('cleanupFail')
        if ($cleanupFailed) {
          $installation = $reportDirectory
          # Inject cleanup refusal without removing any fixture or real installation.
          function Remove-Item { throw 'Synthetic cleanup failure.' }
        }
        $message = $null
        try { & $reportCheck 2>$null }
        catch { $message = $_.Exception.Message }
        Assert-Portable ($message -ceq $case.error) 'Acceptance reporting changed the primary failure.'
        if ($case.file) {
          $destination = Join-Path $reportDirectory $case.file
          Assert-Portable (Test-Path -LiteralPath $destination) "Report did not use the PowerShell location: $($case.path)"
          Assert-Portable (!(Test-Path -LiteralPath (Join-Path (Split-Path $reportDirectory -Parent) $case.file))) 'Report used the process directory.'
          $saved = ConvertFrom-Json ([IO.File]::ReadAllText($destination))
          $expectedStatus = if ($case.fail -or $cleanupFailed) { 'failed' } else { 'passed' }
          Assert-Portable ($saved.status -ceq $expectedStatus -and $saved.cleanup -eq !$cleanupFailed) 'Acceptance report lost its result or cleanup status.'
        }
      }
    }
  }
  finally {
    Set-Location -LiteralPath $originalLocation.Path
    [Environment]::CurrentDirectory = $processDirectory
  }
}
finally {
  [Console]::InputEncoding = $inputEncoding
  if (Test-Path -LiteralPath $installation) { Remove-Item -LiteralPath $installation -Recurse -Force }
}
Write-Output 'Verified Windows PowerShell process capture without stray task output.'
Write-Output 'Verified report destinations, provider rejection, and original failure preservation.'
if ($BinaryDirectory) {
  $installation = (Resolve-Path -LiteralPath $BinaryDirectory).Path
  $report = @{ cases = @() }
  Assert-PortableStartup $Version
  Assert-Portable ($report.cases.Count -eq 4) 'Expected both help/version checks and both protocol exchanges.'
  Write-Output 'Verified all portable startup contracts locally; this is not clean-host acceptance.'
}
