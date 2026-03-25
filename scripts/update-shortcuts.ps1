$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$launcherScript = Join-Path $PSScriptRoot "launch-vibingide.ps1"
$desktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "VibingIDE.lnk"
$startupShortcut = Join-Path ([Environment]::GetFolderPath("Startup")) "VibingIDE.lnk"
$releaseExe = Join-Path $repoRoot "target\release\vibingide.exe"
$debugExe = Join-Path $repoRoot "target\debug\vibingide.exe"

$iconSource = if (Test-Path $releaseExe) {
    $releaseExe
} elseif (Test-Path $debugExe) {
    $debugExe
} else {
    Join-Path $env:SystemRoot "System32\shell32.dll"
}

$powershellExe = (Get-Command powershell.exe).Source
$shell = New-Object -ComObject WScript.Shell

$shortcuts = @(
    @{
        Path = $desktopShortcut
        Description = "Launch VibingIDE from the desktop"
    },
    @{
        Path = $startupShortcut
        Description = "Launch VibingIDE automatically at sign-in"
    }
)

foreach ($item in $shortcuts) {
    $shortcut = $shell.CreateShortcut($item.Path)
    $shortcut.TargetPath = $powershellExe
    $shortcut.Arguments = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$launcherScript`""
    $shortcut.WorkingDirectory = $repoRoot
    $shortcut.Description = $item.Description
    $shortcut.IconLocation = $iconSource
    $shortcut.WindowStyle = 7
    $shortcut.Save()
}

Write-Host "Updated shortcuts:"
Write-Host " - $desktopShortcut"
Write-Host " - $startupShortcut"
