param (
    [int]$MaxLoops = 10
)

$TargetDir = "d:\Aaroneous"

Write-Host "Starting OpenCode Autonomous Audit Loop..." -ForegroundColor Cyan

for ($i = 1; $i -le $MaxLoops; $i++) {
    Write-Host "`n[$i/$MaxLoops] Executing OpenCode CLI Audit Scan..." -ForegroundColor Yellow

    # Launch opencode CLI with live activity streaming and thinking
    npx opencode run --agent auditor --command audit --thinking --auto

    Write-Host "Completed audit cycle $i. Pausing 5s before next check..." -ForegroundColor DarkGray
    Start-Sleep -Seconds 5
}
