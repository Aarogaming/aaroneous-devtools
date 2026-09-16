# deploy/uninstall.ps1
# Aaroneous Uninstallation Script for Windows
# Removes shortcuts, registry entries, PATH integration, and binaries.

[CmdletBinding()]
param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Aaroneous",
    [switch]$Force
)

$ErrorActionPreference = "Continue"

Write-Host "=================================================================" -ForegroundColor Magenta
Write-Host "  AARONEOUS - UNINSTALLER" -ForegroundColor Magenta
Write-Host "=================================================================" -ForegroundColor Magenta

# 1. Remove Shortcuts
Write-Host "[1/4] Removing Start Menu and Desktop shortcuts..." -ForegroundColor Yellow
$StartProgramsDir = [System.IO.Path]::Combine($env:APPDATA, "Microsoft\Windows\Start Menu\Programs")
$StartShortcut = Join-Path $StartProgramsDir "Aaroneous.lnk"
if (Test-Path $StartShortcut) {
    Remove-Item -Force $StartShortcut
}

$StartMenuFolder = Join-Path $StartProgramsDir "Aaroneous"
if (Test-Path $StartMenuFolder) {
    Remove-Item -Recurse -Force $StartMenuFolder
}

$DesktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "Aaroneous.lnk"
if (Test-Path $DesktopShortcut) {
    Remove-Item -Force $DesktopShortcut
}

$OldDesktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "Aaroneous Sovereign HUD.lnk"
if (Test-Path $OldDesktopShortcut) {
    Remove-Item -Force $OldDesktopShortcut
}

# 2. Remove from User PATH
Write-Host "[2/4] Removing from User PATH..." -ForegroundColor Yellow
$TargetBinDir = Join-Path $InstallDir "bin"
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -like "*$TargetBinDir*") {
    $CleanPath = ($UserPath -split ';' | Where-Object { $_ -ne $TargetBinDir -and $_ -ne "" }) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $CleanPath, [EnvironmentVariableTarget]::User)
    Write-Host "  Removed '$TargetBinDir' from User PATH." -ForegroundColor Green
}

# 3. Remove Registry Entry
Write-Host "[3/4] Removing Windows Add/Remove Programs entry..." -ForegroundColor Yellow
$RegPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Aaroneous"
if (Test-Path $RegPath) {
    Remove-Item -Recurse -Force $RegPath
}

# 4. Remove Files and Directories
Write-Host "[4/4] Removing installed program files: $InstallDir..." -ForegroundColor Yellow
if (Test-Path $InstallDir) {
    Remove-Item -Recurse -Force $InstallDir
}

Write-Host "=================================================================" -ForegroundColor Green
Write-Host "  UNINSTALLATION COMPLETED SUCCESSFULLY!" -ForegroundColor Green
Write-Host "=================================================================" -ForegroundColor Green
