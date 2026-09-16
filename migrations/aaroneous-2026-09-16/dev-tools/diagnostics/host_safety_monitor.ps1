# host_safety_monitor.ps1
# Scans the host system for runaway Aaroneous processes, locked ports, and orphaned memory maps.

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   Aaroneous Host Safety & Health Monitor" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

$issuesFound = 0

# 1. Check for running Aaroneous binaries
Write-Host "[1/4] Checking for running Aaroneous processes..." -ForegroundColor Yellow
$processNames = @("a_run", "a-run", "spatial_kinetic", "nats-server", "marionette", "chimera")
foreach ($proc in $processNames) {
    $running = Get-Process -Name $proc -ErrorAction SilentlyContinue
    if ($running) {
        Write-Host "  [WARNING] Process '$proc' is currently RUNNING (PID: $($running.Id -join ', '))" -ForegroundColor Red
        $issuesFound++
    } else {
        Write-Host "  [OK] Process '$proc' is not running." -ForegroundColor Green
    }
}

# 2. Check for port conflicts (8765 for REST/SSE, 4222 for NATS)
Write-Host ""
Write-Host "[2/4] Checking key ports (8765, 4222)..." -ForegroundColor Yellow
$ports = @(8765, 4222)
foreach ($port in $ports) {
    $active = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
    if ($active) {
        Write-Host "  [ALERT] Port $port is in LISTEN state by PID $($active.OwningProcess)" -ForegroundColor Yellow
    } else {
        Write-Host "  [OK] Port $port is available." -ForegroundColor Green
    }
}

# 3. Check for orphaned .synapse memory-mapped files in %TEMP%
Write-Host ""
Write-Host "[3/4] Checking for orphaned .synapse shared memory files in Temp..." -ForegroundColor Yellow
$tempPath = [System.IO.Path]::GetTempPath()
$synapseFiles = Get-ChildItem -Path $tempPath -Filter "*.synapse" -ErrorAction SilentlyContinue
if ($synapseFiles) {
    foreach ($file in $synapseFiles) {
        Write-Host "  [WARNING] Found locked/orphaned synapse file: $($file.FullName) ($($file.Length) bytes)" -ForegroundColor Red
        $issuesFound++
    }
    Write-Host "  -> Tip: Run 'dev/tools/maintenance/purge_stale_synapse.ps1' to clean these." -ForegroundColor Yellow
} else {
    Write-Host "  [OK] No orphaned .synapse files found in $tempPath." -ForegroundColor Green
}

# 4. Check CPU & Memory Load
Write-Host ""
Write-Host "[4/4] Checking overall host resource utilization..." -ForegroundColor Yellow
$cpu = (Get-Counter '\Processor(_Total)\% Processor Time' -ErrorAction SilentlyContinue).CounterSamples[0].CookedValue
$mem = Get-CimInstance Win32_OperatingSystem | Select-Object TotalVisibleMemorySize, FreePhysicalMemory
$memUsedPct = [math]::Round((($mem.TotalVisibleMemorySize - $mem.FreePhysicalMemory) / $mem.TotalVisibleMemorySize) * 100, 1)

Write-Host "  CPU Load: $([math]::Round($cpu, 1))%" -ForegroundColor $(if ($cpu -gt 85) { "Red" } else { "Green" })
Write-Host "  RAM Usage: $memUsedPct%" -ForegroundColor $(if ($memUsedPct -gt 85) { "Red" } else { "Green" })

Write-Host ""
Write-Host "======================================================" -ForegroundColor Cyan
if ($issuesFound -eq 0) {
    Write-Host "  Status: System is SAFE. No runaway processes or locked shared memory segments." -ForegroundColor Green
} else {
    Write-Host "  Status: $issuesFound issue(s) detected. Review warnings above." -ForegroundColor Red
}
Write-Host "======================================================" -ForegroundColor Cyan
