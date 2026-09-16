# deploy/install.ps1
# Aaroneous Installation Script for Windows
# Installs Aaroneous as a standard Windows Application named "Aaroneous".

[CmdletBinding()]
param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Aaroneous",
    [switch]$SkipBuild,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

Write-Host "=================================================================" -ForegroundColor Cyan
Write-Host "  AARONEOUS - WINDOWS INSTALLER" -ForegroundColor Cyan
Write-Host "=================================================================" -ForegroundColor Cyan

$WorkspaceRoot = (Get-Item $PSScriptRoot).Parent.FullName
$TargetBinDir = Join-Path $InstallDir "bin"
$TargetDataDir = Join-Path $InstallDir "data"
$TargetConfigDir = Join-Path $InstallDir "config"

# 1. Build release binaries if needed
if (-not $SkipBuild) {
    Write-Host "[1/6] Building optimized release binaries..." -ForegroundColor Yellow
    $CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
    if (-not (Test-Path $CargoPath)) {
        $CargoPath = "cargo"
    }

    & $CargoPath build --release -p a_run --bin aaroneous --bin a_run --bin aaroneous-setup --bin aaroneous-uninstall --manifest-path (Join-Path $WorkspaceRoot "Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to build release binaries."
        exit 1
    }
}

# 2. Create Target Directories
Write-Host "[2/6] Preparing installation target: $InstallDir" -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $TargetBinDir | Out-Null
New-Item -ItemType Directory -Force -Path $TargetDataDir | Out-Null
New-Item -ItemType Directory -Force -Path $TargetConfigDir | Out-Null

# 3. Copy Executables and Assets
Write-Host "[3/6] Installing executables..." -ForegroundColor Yellow
$SourceMain = Join-Path $WorkspaceRoot "target\release\aaroneous.exe"
$SourceRun = Join-Path $WorkspaceRoot "target\release\a_run.exe"

Copy-Item -Path $SourceMain -Destination (Join-Path $TargetBinDir "aaroneous.exe") -Force
Copy-Item -Path $SourceRun -Destination (Join-Path $TargetBinDir "a_run.exe") -Force

# Provide alias a_hud.exe for backwards compatibility
Copy-Item -Path $SourceMain -Destination (Join-Path $TargetBinDir "a_hud.exe") -Force

# Copy uninstaller into installation directory
Copy-Item -Path (Join-Path $PSScriptRoot "uninstall.ps1") -Destination (Join-Path $InstallDir "uninstall.ps1") -Force

# 4. Add to User PATH
Write-Host "[4/6] Updating Windows User PATH..." -ForegroundColor Yellow
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -notlike "*$TargetBinDir*") {
    $NewPath = "$UserPath;$TargetBinDir"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::User)
    $env:Path += ";$TargetBinDir"
    Write-Host "  Added '$TargetBinDir' to User PATH." -ForegroundColor Green
} else {
    Write-Host "  PATH already configured." -ForegroundColor Green
}

# 5. Create Start Menu and Desktop Shortcuts
Write-Host "[5/6] Creating Windows Shortcuts..." -ForegroundColor Yellow
$WshShell = New-Object -ComObject WScript.Shell

# Start Menu Shortcut
$StartProgramsDir = [System.IO.Path]::Combine($env:APPDATA, "Microsoft\Windows\Start Menu\Programs")
$StartShortcut = $WshShell.CreateShortcut((Join-Path $StartProgramsDir "Aaroneous.lnk"))
$StartShortcut.TargetPath = Join-Path $TargetBinDir "aaroneous.exe"
$StartShortcut.WorkingDirectory = $InstallDir
$StartShortcut.Description = "Aaroneous Synthetic Intelligence"
$StartShortcut.Save()

# Desktop Shortcut
$DesktopDir = [Environment]::GetFolderPath("Desktop")
$DesktopShortcut = $WshShell.CreateShortcut((Join-Path $DesktopDir "Aaroneous.lnk"))
$DesktopShortcut.TargetPath = Join-Path $TargetBinDir "aaroneous.exe"
$DesktopShortcut.WorkingDirectory = $InstallDir
$DesktopShortcut.Description = "Aaroneous Synthetic Intelligence"
$DesktopShortcut.Save()

# 6. Register in Windows Add/Remove Programs
Write-Host "[6/6] Registering in Windows Add/Remove Programs..." -ForegroundColor Yellow
$RegPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Aaroneous"
if (-not (Test-Path $RegPath)) {
    New-Item -Path $RegPath -Force | Out-Null
}

Set-ItemProperty -Path $RegPath -Name "DisplayName" -Value "Aaroneous" -Force
Set-ItemProperty -Path $RegPath -Name "DisplayVersion" -Value "0.1.0" -Force
Set-ItemProperty -Path $RegPath -Name "Publisher" -Value "Aaroneous" -Force
Set-ItemProperty -Path $RegPath -Name "InstallLocation" -Value $InstallDir -Force
Set-ItemProperty -Path $RegPath -Name "DisplayIcon" -Value (Join-Path $TargetBinDir "aaroneous.exe") -Force
Set-ItemProperty -Path $RegPath -Name "UninstallString" -Value "powershell.exe -ExecutionPolicy Bypass -File `"$InstallDir\uninstall.ps1`"" -Force

Write-Host "=================================================================" -ForegroundColor Green
Write-Host "  INSTALLATION COMPLETED SUCCESSFULLY!" -ForegroundColor Green
Write-Host "=================================================================" -ForegroundColor Green
Write-Host "Installed Location : $InstallDir" -ForegroundColor Cyan
Write-Host "Executable         : $TargetBinDir\aaroneous.exe" -ForegroundColor Cyan
Write-Host "Start Menu         : Start Menu > Aaroneous" -ForegroundColor Cyan
Write-Host "Desktop Shortcut   : Desktop > Aaroneous" -ForegroundColor Cyan
Write-Host "Terminal Command   : Type 'aaroneous' in any terminal window" -ForegroundColor Cyan
Write-Host "=================================================================" -ForegroundColor Green
