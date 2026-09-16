# launch_safe_mode.ps1
# Launches Aaroneous / SpatialKinetic in strictly sandboxed Safe Mode (Zero OS HID takeover).

param(
    [string]$Target = "spatial_kinetic",
    [double]$Fps = 15.0
)

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Aaroneous Sandboxed Safe Mode Launcher" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "  Target: $Target" -ForegroundColor Yellow
Write-Host "  FPS: $Fps" -ForegroundColor Yellow
Write-Host "  HID Output: DISABLED (Host mouse/keyboard is 100% safe)" -ForegroundColor Green
Write-Host "======================================================" -ForegroundColor Cyan

# Explicitly ensure host input is NOT allowed
$env:AARONEOUS_ALLOW_HOST_INPUT = "0"

if ($Target -eq "spatial_kinetic") {
    $binPath = "d:\Aaroneous\target\debug\spatial_kinetic.exe"
    if (-not (Test-Path $binPath)) {
        Write-Host "Building spatial_kinetic binary first..." -ForegroundColor Yellow
        cargo build --bin spatial_kinetic --manifest-path "d:\Aaroneous\core\hypervisor\Cargo.toml"
    }

    Write-Host "Launching spatial_kinetic with --no-hid --fps $Fps..." -ForegroundColor Cyan
    & $binPath --no-hid --fps $Fps
} elseif ($Target -eq "a_run") {
    $binPath = "d:\Aaroneous\target\debug\a_run.exe"
    if (-not (Test-Path $binPath)) {
        Write-Host "Building a_run binary first..." -ForegroundColor Yellow
        cargo build --bin a_run --manifest-path "d:\Aaroneous\core\hypervisor\Cargo.toml"
    }

    Write-Host "Launching a_run autonomic loop in safe mode..." -ForegroundColor Cyan
    & $binPath start
} else {
    Write-Host "Unknown target: $Target. Choose 'spatial_kinetic' or 'a_run'." -ForegroundColor Red
}
