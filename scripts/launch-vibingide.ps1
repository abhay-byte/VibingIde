param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$PassthroughArgs
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$releaseExe = Join-Path $repoRoot "target\release\vibingide.exe"
$debugExe = Join-Path $repoRoot "target\debug\vibingide.exe"
$cargoToml = Join-Path $repoRoot "Cargo.toml"

if (Test-Path $releaseExe) {
    Start-Process -FilePath $releaseExe -WorkingDirectory $repoRoot -ArgumentList $PassthroughArgs
    exit 0
}

if (Test-Path $debugExe) {
    Start-Process -FilePath $debugExe -WorkingDirectory $repoRoot -ArgumentList $PassthroughArgs
    exit 0
}

$cargoArgs = @("run", "--release", "--manifest-path", $cargoToml)
if ($PassthroughArgs.Count -gt 0) {
    $cargoArgs += "--"
    $cargoArgs += $PassthroughArgs
}

Start-Process -FilePath "cargo" -WorkingDirectory $repoRoot -ArgumentList $cargoArgs
