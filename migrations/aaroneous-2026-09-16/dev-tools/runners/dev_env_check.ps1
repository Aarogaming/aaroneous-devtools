# dev_env_check.ps1
# Verifies the developer environment prerequisites for building and running Aaroneous.

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Aaroneous Developer Environment Verification" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

# 1. Rust / Cargo
$cargoVer = cargo --version 2>$null
if ($cargoVer) {
    Write-Host "  [OK] Cargo is installed: $cargoVer" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Cargo / Rust toolchain not found in PATH!" -ForegroundColor Red
}

# 2. Node / NPM (For MaelstromUI)
$nodeVer = node --version 2>$null
if ($nodeVer) {
    Write-Host "  [OK] Node.js is installed: $nodeVer" -ForegroundColor Green
} else {
    Write-Host "  [WARN] Node.js not found in PATH (required for MaelstromUI frontend)." -ForegroundColor Yellow
}

# 3. Python 3
$pyVer = python --version 2>$null
if ($pyVer) {
    Write-Host "  [OK] Python is installed: $pyVer" -ForegroundColor Green
} else {
    Write-Host "  [WARN] Python not found in PATH." -ForegroundColor Yellow
}

# 4. NATS Server
$natsPath = "d:\Aaroneous\nats\nats-server-v2.10.14-windows-amd64\nats-server.exe"
if (Test-Path $natsPath) {
    Write-Host "  [OK] Bundled NATS Server found at: $natsPath" -ForegroundColor Green
} else {
    Write-Host "  [WARN] Bundled NATS server executable not found at expected path." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "Environment check complete." -ForegroundColor Cyan
