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
$installation = [IO.Path]::GetTempPath()
$executable = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$arguments = '-NoProfile -NonInteractive -Command "[Console]::OpenStandardInput().CopyTo([Console]::OpenStandardOutput()); [Console]::Error.Write(''synthetic-error''); exit 7"'
$inputBytes = [byte[]]@(0, 10, 65, 128, 255)
$results = @(Invoke-PortableProcess $executable $arguments $inputBytes)
Assert-Portable ($results.Count -eq 1 -and $results[0] -is [hashtable]) 'Process capture must return exactly one result, without task objects.'
$result = $results[0]
Assert-Portable ($result.code -eq 7 -and $result.error -ceq 'synthetic-error' -and
  ($result.bytes -join ',') -ceq ($inputBytes -join ',') -and
  $result.text -ceq [Text.Encoding]::UTF8.GetString($inputBytes)) 'Process exit, binary output, text, or diagnostics changed.'
Write-Output 'Verified Windows PowerShell process capture without stray task output.'
if ($BinaryDirectory) {
  $installation = (Resolve-Path -LiteralPath $BinaryDirectory).Path
  $report = @{ cases = @() }
  Assert-PortableStartup $Version
  Assert-Portable ($report.cases.Count -eq 4) 'Expected both help/version checks and both protocol exchanges.'
  Write-Output 'Verified all portable startup contracts locally; this is not clean-host acceptance.'
}
