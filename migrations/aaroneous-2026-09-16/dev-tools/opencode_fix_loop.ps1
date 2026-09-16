param (
    [int]$MaxLoops = 20
)

$QueuePath = "d:\Aaroneous\dev\docs\audits\active\ACTIVE_AUDIT_QUEUE.md"

Write-Host "Starting OpenCode Autonomous Fix Loop..." -ForegroundColor Cyan

for ($i = 1; $i -le $MaxLoops; $i++) {
    $QueueContent = Get-Content $QueuePath -Raw
    
    # Check if there are any pending [ ] items
    if ($QueueContent -notmatch "- \[ \] \*\*") {
        Write-Host "All items in ACTIVE_AUDIT_QUEUE.md are resolved!" -ForegroundColor Green
        break
    }

    # Match next pending item
    $Match = [regex]::Match($QueueContent, "- \[ \] \*\*([^\*]+)\*\*")
    $TaskTitle = $Match.Groups[1].Value
    Write-Host "`n[$i/$MaxLoops] Executing OpenCode CLI on: $TaskTitle" -ForegroundColor Yellow

    # Launch opencode CLI with live activity streaming, thinking, and auto-approval
    npx opencode run --agent auditor --command fix --thinking --auto

    Write-Host "Completed cycle $i. Pausing 3s before checking queue..." -ForegroundColor DarkGray
    Start-Sleep -Seconds 3
}
