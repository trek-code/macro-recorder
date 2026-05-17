# Creates a desktop shortcut pointing to the built exe.
# Run after `npm run tauri build` (release) or use -Dev for the debug build.
param(
    [switch]$Dev
)

$projectRoot = Split-Path $PSScriptRoot -Parent

if ($Dev) {
    $exePath = Join-Path $projectRoot "src-tauri\target\debug\macro-recorder.exe"
} else {
    $exePath = Join-Path $projectRoot "src-tauri\target\release\macro-recorder.exe"
}

if (-not (Test-Path $exePath)) {
    Write-Error "Executable not found at: $exePath"
    Write-Host "Run 'npm run tauri build' first (or 'npm run tauri dev' and use -Dev flag)."
    exit 1
}

$desktopPath = [Environment]::GetFolderPath("Desktop")
$shortcutPath = Join-Path $desktopPath "Macro Recorder.lnk"

$wsh = New-Object -ComObject WScript.Shell
$shortcut = $wsh.CreateShortcut($shortcutPath)
$shortcut.TargetPath       = $exePath
$shortcut.WorkingDirectory = Split-Path $exePath -Parent
$shortcut.Description      = "Macro Recorder v0.3"

# Use the app icon if it exists
$icoPath = Join-Path $projectRoot "src-tauri\icons\icon.ico"
if (Test-Path $icoPath) { $shortcut.IconLocation = $icoPath }

$shortcut.Save()
Write-Host "Shortcut created: $shortcutPath"
