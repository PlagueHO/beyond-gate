#Requires -Version 7.4
# Check import classification and inspection failures without installed-runtime assumptions.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'windows-runtime.ps1')

# Load only the pure classifier, without starting clean-guest acceptance on this host.
$portableAst = [Management.Automation.Language.Parser]::ParseFile(
  (Join-Path $PSScriptRoot 'test-windows-portable.ps1'), [ref]$null, [ref]$null)
$classifier = @($portableAst.FindAll({
  param($node)
  $node -is [Management.Automation.Language.FunctionDefinitionAst] -and
    $node.Name -ceq 'Test-PortableWindowsComponent'
}, $false))
if ($classifier.Count -ne 1) { throw 'Expected one clean-guest Windows component classifier.' }
. ([scriptblock]::Create($classifier[0].Extent.Text))
foreach ($name in @('mfc42.dll', 'msvcp140_clr0400.dll', 'vcruntime140_1_clr0400.dll')) {
  if (!(Test-PortableWindowsComponent $name $true) -or (Test-PortableWindowsComponent $name $false)) {
    throw 'Only reviewed, Windows-protected components may be excluded from the runtime baseline.'
  }
}
foreach ($name in @('vcruntime140.dll', 'msvcp140.dll', 'mfc140u.dll', 'vcruntime140_clr9999.dll')) {
  if (Test-PortableWindowsComponent $name $true) {
    throw 'A redistributable or unreviewed name was excluded from the runtime baseline.'
  }
}

# Require the intended rejection rather than an unrelated PowerShell failure.
function Assert-RuntimeRejected {
  param([scriptblock]$Action, [string]$Expected)
  try { & $Action }
  catch {
    if (!$_.Exception.Message.Contains($Expected)) { throw }
    return
  }
  throw "Expected runtime rejection: $Expected"
}

$ordinary = @'
File Type: EXECUTABLE IMAGE

  Section contains the following imports:

    KERNEL32.dll
    api-ms-win-crt-runtime-l1-1-0.dll

  Summary
'@
$imports = @(ConvertFrom-WindowsImports $ordinary)
Assert-WindowsSystemImports $imports
if (($imports -join ',') -cne 'api-ms-win-crt-runtime-l1-1-0.dll,kernel32.dll') {
  throw 'Normal import inspection lost a DLL.'
}
foreach ($name in @(
  'VCRUNTIME140.dll', 'vcruntime140_1.dll', 'VCRUNTIME140D.dll',
  'MSVCP140.dll', 'msvcp140_atomic_wait.dll', 'msvcp140_codecvt_ids.dll',
  'MSVCR120.dll', 'CONCRT140.dll', 'vcomp140.dll', 'vcamp140.dll',
  'vccorlib140.dll', 'mfc140u.dll', 'mfcm140.dll', 'atl100.dll', 'ucrtbased.dll'
)) {
  foreach ($heading in @('imports', 'delay load imports')) {
    $text = $ordinary.Replace('  Summary', "  Section contains the following ${heading}:`n`n    $name`n`n  Summary")
    Assert-RuntimeRejected {
      Assert-WindowsSystemImports @(ConvertFrom-WindowsImports $text)
    } 'Forbidden Visual C++ runtime import:'
  }
}
Assert-WindowsSystemImports @('KERNEL32.dll', 'msvcrt.dll', 'ucrtbase.dll', 'ext-ms-win-ntuser-window-l1-1-0.dll')
Assert-RuntimeRejected { Assert-WindowsSystemImports @('invented.dll') } 'Unreviewed Windows dependency:'
foreach ($name in @('unreviewed library.dll', 'invented.pyd', 'invented.exe')) {
  $text = $ordinary.Replace('KERNEL32.dll', $name)
  Assert-RuntimeRejected {
    Assert-WindowsSystemImports @(ConvertFrom-WindowsImports $text)
  } 'Unreviewed Windows dependency:'
}
foreach ($text in @('', 'synthetic', $ordinary.Replace('File Type: EXECUTABLE IMAGE', ''), $ordinary.Replace('  Summary', ''))) {
  Assert-RuntimeRejected { ConvertFrom-WindowsImports $text } 'inspection output is incomplete'
}

if ($IsWindows) {
  & (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') -NoProfile -File (
    Join-Path $PSScriptRoot 'test-windows-portable-process.ps1')
  if ($LASTEXITCODE -ne 0) { throw 'Windows PowerShell portable process regression failed.' }
  $temporary = Join-Path ([IO.Path]::GetTempPath()) "Beyond Gate imports $([guid]::NewGuid().ToString('N'))"
  try {
    New-Item -ItemType Directory -Path $temporary | Out-Null
    $invalid = Join-Path $temporary 'invalid.exe'
    [IO.File]::WriteAllText($invalid, 'not a PE file')
    Assert-RuntimeRejected { Assert-WindowsRuntime @($invalid) } 'Windows import inspection failed:'
    Assert-RuntimeRejected { Assert-WindowsRuntime @((Join-Path $temporary 'missing.exe')) } 'Windows import inspection failed:'
  }
  finally {
    if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
  }
}
Write-Output 'Verified normal/delay runtime import rejection, system imports, and inspection failures.'
# Expected dumpbin failures must not become the Actions shell's final native exit code.
exit 0
