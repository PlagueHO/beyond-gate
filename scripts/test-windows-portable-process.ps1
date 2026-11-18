#Requires -Version 5.1
# Check clean-guest process capture under the built-in Windows PowerShell runtime.
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
foreach ($name in @('Assert-Portable', 'Invoke-PortableProcess', 'Assert-PortableStartup')) {
  $definition = @($ast.FindAll({
    param($node)
    $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq $name
  }, $false))
  if ($definition.Count -ne 1) { throw "Expected one process-test function: $name" }
  . ([scriptblock]::Create($definition[0].Extent.Text))
}
$installation = Join-Path (Split-Path $PSScriptRoot -Parent) "target\portable process $([guid]::NewGuid().ToString('N'))"
try {
  New-Item -ItemType Directory -Path $installation | Out-Null
  # Windows PowerShell can add a UTF-8 BOM before its command runs on UTF-8 system locales.
  # Compile a raw-stream child with the .NET Framework compiler already required by that shell.
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
  }
}
finally {
  if (Test-Path -LiteralPath $installation) { Remove-Item -LiteralPath $installation -Recurse -Force }
}
Write-Output 'Verified Windows PowerShell process capture without stray task output.'
if ($BinaryDirectory) {
  $installation = (Resolve-Path -LiteralPath $BinaryDirectory).Path
  $report = @{ cases = @() }
  Assert-PortableStartup $Version
  Assert-Portable ($report.cases.Count -eq 4) 'Expected both help/version checks and both protocol exchanges.'
  Write-Output 'Verified all portable startup contracts locally; this is not clean-host acceptance.'
}
