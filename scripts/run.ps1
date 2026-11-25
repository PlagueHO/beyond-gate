# Build both native binaries, then run one with the caller's streams and arguments.
# Usage: pwsh -NoProfile -File scripts/run.ps1 cli [arguments...]
# Usage: pwsh -NoProfile -File scripts/run.ps1 mcp [arguments...]
$ErrorActionPreference = 'Stop'

if ($args.Count -eq 0 -or $args[0] -notin @('cli', 'mcp')) {
    [Console]::Error.WriteLine('Supply cli or mcp, followed by product arguments.')
    exit 2
}

$binaryName = if ($args[0] -eq 'cli') { 'beyond-gate' } else { 'beyond-gate-mcp' }
$productArguments = @($args | Select-Object -Skip 1)
$executable = $null
$wrapper = Join-Path $PSScriptRoot 'cargo.ps1'

# Cargo reports the actual artifact path, including configured target directories.
& pwsh -NoProfile -File $wrapper build --workspace --locked --message-format=json 2>&1 | ForEach-Object {
    $line = "$_"
    if ($line.StartsWith('{')) {
        $message = $line | ConvertFrom-Json
        if ($message.reason -eq 'compiler-artifact' -and $message.target.name -eq $binaryName -and $message.executable) {
            $executable = $message.executable
        }
        if ($message.reason -eq 'compiler-message' -and $message.message.rendered) {
            [Console]::Error.Write($message.message.rendered)
        }
    }
    else {
        [Console]::Error.WriteLine($line)
    }
}
$buildExitCode = $LASTEXITCODE
if ($buildExitCode -ne 0) {
    exit $buildExitCode
}
if (-not $executable -or -not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    [Console]::Error.WriteLine("Cargo did not report the $binaryName executable.")
    exit 1
}

& $executable @productArguments
exit $LASTEXITCODE
