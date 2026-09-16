# check_ports_and_locks.ps1
# Specifically tests TCP ports 8765, 4222 and checks for file locks on database and shared memory files.

Write-Host "Checking Aaroneous network endpoints and file locks..." -ForegroundColor Cyan

# Check Port 8765 (HTTP/SSE)
$p8765 = Get-NetTCPConnection -LocalPort 8765 -ErrorAction SilentlyContinue
if ($p8765) {
    Write-Host "Port 8765 (HTTP/SSE): BOUND by PID $($p8765.OwningProcess)" -ForegroundColor Yellow
} else {
    Write-Host "Port 8765 (HTTP/SSE): FREE (Available for Federation server)" -ForegroundColor Green
}

# Check Port 4222 (NATS)
$p4222 = Get-NetTCPConnection -LocalPort 4222 -ErrorAction SilentlyContinue
if ($p4222) {
    Write-Host "Port 4222 (NATS): BOUND by PID $($p4222.OwningProcess)" -ForegroundColor Yellow
} else {
    Write-Host "Port 4222 (NATS): FREE (Available for NATS server)" -ForegroundColor Green
}

# Check hox.db and hive.db locks
$workspace = "d:\Aaroneous"
$databases = @("hox.db", "hive.db", "data\styles.db")
foreach ($db in $databases) {
    $dbPath = Join-Path $workspace $db
    if (Test-Path $dbPath) {
        Write-Host "Database $db exists at $dbPath" -ForegroundColor Green
    } else {
        Write-Host "Database $db not found (will be initialized on first run)" -ForegroundColor Gray
    }
}
