# Inspect Windows application imports without treating installed runtimes as proof of portability.
Set-StrictMode -Version Latest

# Load the same Visual Studio product and version range as the Cargo wrapper.
function Initialize-WindowsRuntimeTools {
  if (!$IsWindows) { throw 'Windows runtime inspection requires Windows.' }
  $developerShell = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\2022\BuildTools\Common7\Tools\Launch-VsDevShell.ps1'
  if (!(Test-Path -LiteralPath $developerShell)) {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (!(Test-Path -LiteralPath $vswhere)) { throw 'Visual Studio 2022 Build Tools was not found.' }
    $installation = & $vswhere -latest -version '[17.0,18.0)' -products Microsoft.VisualStudio.Product.BuildTools -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($LASTEXITCODE -ne 0 -or !$installation) { throw 'Visual Studio 2022 Build Tools was not found.' }
    $developerShell = Join-Path $installation 'Common7\Tools\Launch-VsDevShell.ps1'
  }
  $developerOutput = & $developerShell -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
  $succeeded = $?
  $developerOutput | Out-Host
  if (!$succeeded) { throw 'Visual Studio 2022 environment setup failed.' }
  $inspector = Join-Path $env:VCToolsInstallDir 'bin\Hostx64\x64\dumpbin.exe'
  if (!(Test-Path -LiteralPath $inspector)) { throw 'Visual Studio 2022 dumpbin was not found.' }
  return $inspector
}

# Include both ordinary and delay-loaded imports; reject incomplete inspection output.
function ConvertFrom-WindowsImports {
  param([Parameter(Mandatory)][AllowEmptyString()][string]$Text)
  # DUMPBIN uses four spaces for library names and deeper indentation for symbols.
  $imports = @([regex]::Matches($Text, '(?m)^ {4}(\S[^\r\n]*)\r?$') |
    ForEach-Object { $_.Groups[1].Value.TrimEnd().ToLowerInvariant() } | Sort-Object -Unique)
  if ($Text -notmatch '(?m)^File Type: EXECUTABLE IMAGE\s*$' -or
      $Text -notmatch '(?m)^\s+Summary\s*$' -or $imports.Count -eq 0) {
    throw 'Windows import inspection output is incomplete.'
  }
  return $imports
}

# Permit reviewed Windows OS imports, not arbitrary DLLs present on the build host.
function Assert-WindowsSystemImports {
  param([Parameter(Mandatory)][string[]]$Imports)
  $systemDlls = @(
    'advapi32.dll', 'bcryptprimitives.dll', 'crypt32.dll', 'kernel32.dll',
    'ktmw32.dll', 'msvcrt.dll', 'ntdll.dll', 'ucrtbase.dll', 'user32.dll', 'ws2_32.dll'
  )
  if ($Imports.Count -eq 0) { throw 'Windows import inspection found no dependencies.' }
  foreach ($name in $Imports) {
    if ($name -match '^(?:(?:vcruntime|msvcp|msvcr|concrt|vcomp|vcamp|vccorlib|mfc|mfcm|atl)\d[a-z0-9_]*|ucrtbased)\.dll$') {
      throw "Forbidden Visual C++ runtime import: $name"
    }
    if ($name -notin $systemDlls -and $name -notmatch '^(?:api|ext)-ms-win-[a-z0-9-]+\.dll$') {
      throw "Unreviewed Windows dependency: $name"
    }
  }
}

# Inspect actual extracted PE files and fail on tool errors before any process starts.
function Assert-WindowsRuntime {
  param([Parameter(Mandatory)][string[]]$Binaries)
  $inspector = Initialize-WindowsRuntimeTools
  foreach ($binary in $Binaries) {
    $output = & $inspector /NOLOGO /IMPORTS $binary 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Windows import inspection failed: $([IO.Path]::GetFileName($binary))" }
    $imports = @(ConvertFrom-WindowsImports ($output -join "`n"))
    Assert-WindowsSystemImports $imports
    Write-Output "$([IO.Path]::GetFileName($binary)): $($imports -join ', ')"
  }
}
