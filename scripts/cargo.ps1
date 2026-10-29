# Run Cargo in the workspace with the Visual Studio 2022 Build Tools environment on Windows.
# All arguments are forwarded to Cargo. Documentation builds reject warnings.
$ErrorActionPreference = 'Stop'

if ($args.Count -eq 0) {
    throw 'Supply a Cargo command, such as build, test, or clippy.'
}

if ($IsWindows) {
    $developerShell = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\2022\BuildTools\Common7\Tools\Launch-VsDevShell.ps1'
    if (-not (Test-Path -LiteralPath $developerShell)) {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (-not (Test-Path -LiteralPath $vswhere)) {
            throw 'Visual Studio 2022 Build Tools was not found. Install its C++ workload and Windows SDK.'
        }
        $installation = & $vswhere -latest -version '[17.0,18.0)' -products Microsoft.VisualStudio.Product.BuildTools -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($LASTEXITCODE -ne 0 -or -not $installation) {
            throw 'No Visual Studio 2022 Build Tools installation with C++ tools was found.'
        }
        $developerShell = Join-Path $installation 'Common7\Tools\Launch-VsDevShell.ps1'
    }
    $developerOutput = & $developerShell -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
    $developerShellSucceeded = $?
    $developerOutput | Out-Host
    if (-not $developerShellSucceeded) {
        throw 'Visual Studio 2022 Build Tools environment setup failed.'
    }
}

if ($args[0] -in @('doc', 'rustdoc')) {
    $env:RUSTDOCFLAGS = "$env:RUSTDOCFLAGS -D warnings".Trim()
}

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    & cargo @args
    $cargoExitCode = $LASTEXITCODE
}
finally {
    Pop-Location
}
exit $cargoExitCode
