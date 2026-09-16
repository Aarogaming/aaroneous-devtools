# deploy/package_release.ps1
# Packages Aaroneous into a standalone distributable ZIP release with installer.

[CmdletBinding()]
param(
    [string]$Version = "0.3.0",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$WorkspaceRoot = (Get-Item $PSScriptRoot).Parent.FullName
$DistDir = Join-Path $WorkspaceRoot "dist"
$StagingDir = Join-Path $DistDir "Aaroneous-v$Version-windows-x86_64"
$ZipFile = Join-Path $DistDir "Aaroneous-v$Version-windows-x86_64.zip"

Write-Host "=================================================================" -ForegroundColor Cyan
Write-Host "  PACKAGING AARONEOUS STANDALONE RELEASE (v$Version)" -ForegroundColor Cyan
Write-Host "=================================================================" -ForegroundColor Cyan

# 1. Clean & Prepare Staging
if (Test-Path $StagingDir) {
    Remove-Item -Recurse -Force $StagingDir
}
New-Item -ItemType Directory -Force -Path (Join-Path $StagingDir "bin") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $StagingDir "config") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $StagingDir "shaders") | Out-Null

# 2. Build and verify executables
if (-not $SkipBuild) {
    Write-Host "Building release binaries..." -ForegroundColor Yellow
    & cargo build --release -p a_run --bin aaroneous --bin a_run --bin aaroneous-setup --bin aaroneous-uninstall --manifest-path (Join-Path $WorkspaceRoot "Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        throw "Release binary build failed."
    }
}

$RequiredBinaries = @(
    "aaroneous.exe",
    "a_run.exe",
    "aaroneous-setup.exe",
    "aaroneous-uninstall.exe"
)
foreach ($Binary in $RequiredBinaries) {
    $BinaryPath = Join-Path $WorkspaceRoot "target\release\$Binary"
    if (-not (Test-Path $BinaryPath)) {
        throw "Required release binary is missing: $BinaryPath. Build without -SkipBuild or provide the complete release output."
    }
}

# 3. Copy Executables
Write-Host "Copying release binaries..." -ForegroundColor Yellow
Copy-Item -Path (Join-Path $WorkspaceRoot "target\release\aaroneous.exe") -Destination (Join-Path $StagingDir "bin\aaroneous.exe") -Force
Copy-Item -Path (Join-Path $WorkspaceRoot "target\release\a_run.exe") -Destination (Join-Path $StagingDir "bin\a_run.exe") -Force
Copy-Item -Path (Join-Path $WorkspaceRoot "target\release\aaroneous-uninstall.exe") -Destination (Join-Path $StagingDir "bin\aaroneous-uninstall.exe") -Force
Copy-Item -Path (Join-Path $WorkspaceRoot "target\release\aaroneous-setup.exe") -Destination (Join-Path $StagingDir "Aaroneous-Setup.exe") -Force

# 4. Copy Configurations & Shaders
Write-Host "Copying configuration and shader assets..." -ForegroundColor Yellow
if (Test-Path (Join-Path $WorkspaceRoot "config")) {
    Copy-Item -Path (Join-Path $WorkspaceRoot "config\*") -Destination (Join-Path $StagingDir "config") -Recurse -Force
}
if (Test-Path (Join-Path $WorkspaceRoot "shaders")) {
    Copy-Item -Path (Join-Path $WorkspaceRoot "shaders\*") -Destination (Join-Path $StagingDir "shaders") -Recurse -Force
}
if (Test-Path (Join-Path $WorkspaceRoot "deploy\mcp_clients")) {
    Copy-Item -Path (Join-Path $WorkspaceRoot "deploy\mcp_clients") -Destination (Join-Path $StagingDir "mcp_clients") -Recurse -Force
}

# 5. Copy Installer Scripts & README
Write-Host "Copying installation scripts and documentation..." -ForegroundColor Yellow
Copy-Item -Path (Join-Path $PSScriptRoot "install.ps1") -Destination (Join-Path $StagingDir "install.ps1") -Force
Copy-Item -Path (Join-Path $PSScriptRoot "uninstall.ps1") -Destination (Join-Path $StagingDir "uninstall.ps1") -Force
if (Test-Path (Join-Path $WorkspaceRoot "README.md")) {
    Copy-Item -Path (Join-Path $WorkspaceRoot "README.md") -Destination (Join-Path $StagingDir "README.md") -Force
}

# 6. Create Distributable ZIP
Write-Host "Creating distributable archive: $ZipFile..." -ForegroundColor Yellow
if (Test-Path $ZipFile) {
    Remove-Item -Force $ZipFile
}
Compress-Archive -Path "$StagingDir\*" -DestinationPath $ZipFile -CompressionLevel Optimal

$SizeMb = [math]::Round((Get-Item $ZipFile).Length / 1MB, 2)

Write-Host "=================================================================" -ForegroundColor Green
Write-Host "  RELEASE PACKAGE CREATED SUCCESSFULLY!" -ForegroundColor Green
Write-Host "=================================================================" -ForegroundColor Green
Write-Host "Archive Path : $ZipFile" -ForegroundColor Cyan
Write-Host "File Size    : $SizeMb MB" -ForegroundColor Cyan
Write-Host "=================================================================" -ForegroundColor Green
