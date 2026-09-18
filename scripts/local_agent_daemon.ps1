[CmdletBinding()]
param (
    [Parameter(Mandatory=$false)]
    [string]$TargetWorkspace = "D:\Aaroneous",

    [Parameter(Mandatory=$false)]
    [string]$QueueFile = "D:\aaroneous-devtools\queue\task_queue.json",

    [Parameter(Mandatory=$false)]
    [string]$LogFile = "D:\aaroneous-devtools\queue\agent_progress_log.md",

    [Parameter(Mandatory=$false)]
    [int]$MaxIterations = 10,

    [Parameter(Mandatory=$false)]
    [switch]$SinglePass
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $QueueFile)) {
    Write-Error "Task queue file not found: $QueueFile"
}

if (-not (Test-Path $TargetWorkspace)) {
    Write-Error "Target workspace not found: $TargetWorkspace"
}

Write-Host "=== Aaroneous DevTools Local Agent Passive Progress Daemon ==="
Write-Host "Target Workspace: $TargetWorkspace"
Write-Host "Queue file:       $QueueFile"
Write-Host "Target model:     qwen3.5:9b-q6 (http://localhost:11434)"
Write-Host ""

$iteration = 0
while ($true) {
    $iteration++
    if ($MaxIterations -gt 0 -and $iteration -gt $MaxIterations) {
        Write-Host "Reached maximum iteration limit ($MaxIterations). Exiting daemon loop."
        break
    }

    $queueContent = Get-Content -Path $QueueFile -Raw -Encoding UTF8 | ConvertFrom-Json
    $pendingTasks = @($queueContent.queue | Where-Object { $_.status -eq "pending" })

    if ($pendingTasks.Count -eq 0) {
        Write-Host "No pending tasks remaining in queue. Engine idling..."
        if ($SinglePass) { break }
        Start-Sleep -Seconds 30
        continue
    }

    $task = $pendingTasks[0]
    Write-Host "----------------------------------------------------"
    Write-Host "[$($task.id)] Starting Task: $($task.title)"
    Write-Host "Target Relative Path: $($task.target_file)"

    $task.status = "in_progress"
    $queueContent | ConvertTo-Json -Depth 5 | Set-Content -Path $QueueFile -Encoding UTF8

    # Staged prompt path inside devtools scripts
    $stagedPromptPath = "D:\aaroneous-devtools\scripts\staged\prompt_$($task.id).txt"
    $stagedDir = Split-Path -Parent $stagedPromptPath
    if (-not (Test-Path $stagedDir)) { New-Item -ItemType Directory -Path $stagedDir -Force | Out-Null }
    Set-Content -Path $stagedPromptPath -Value $task.prompt -Encoding UTF8

    $fullTargetPath = Join-Path $TargetWorkspace $task.target_file

    # Execute code generation via local_agent_delegate.ps1
    $delegateScript = "D:\aaroneous-devtools\scripts\local_agent_delegate.ps1"
    $generationSuccess = $false
    $retryCount = 0
    $maxRetries = if ($queueContent.max_retries) { $queueContent.max_retries } else { 3 }

    while ($retryCount -lt $maxRetries -and -not $generationSuccess) {
        $retryCount++
        Write-Host "Execution Pass ${retryCount}/${maxRetries} for $($task.id)..."

        try {
            pwsh -File $delegateScript `
                -PromptFile $stagedPromptPath `
                -OutputFile $fullTargetPath `
                -Model "qwen3.5:9b-q6" `
                -NumPredict 4096 `
                -Temperature 0.0

            # Verify formatting and syntax in TargetWorkspace
            Push-Location $TargetWorkspace
            try {
                cargo fmt -- $fullTargetPath

                $targetParts = $task.target_file -split '[/\\]'
                if ($targetParts[0] -eq "crates" -or $targetParts[0] -eq "dev") {
                    $crateName = $targetParts[1]
                    Write-Host "Running compilation check for $crateName --all-targets in $TargetWorkspace..."
                    cargo check -p $crateName --all-targets

                } else {
                    cargo check -p xtask
                }
            } finally {
                Pop-Location
            }

            $generationSuccess = $true
            Write-Host "Verification PASSED for $($task.id)!"
        } catch {
            Write-Host "Verification FAILED on attempt ${retryCount}: ${_}"
            if ($retryCount -lt $maxRetries) {
                # Append failure traceback to prompt for self-repair
                $repairPrompt = "$($task.prompt)`n`nPREVIOUS ATTEMPT FAILED WITH ERROR:`n$_`n`nPlease fix the error and output complete, corrected Rust code."
                Set-Content -Path $stagedPromptPath -Value $repairPrompt -Encoding UTF8
            }
        }
    }

    if ($generationSuccess) {
        $task.status = "completed"
        $timestamp = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
        $logEntry = "## [$timestamp] $($task.id): $($task.title)`n- **Target File**: ``$($task.target_file)```n- **Status**: Completed & Verified`n- **Model**: ``qwen3.5:9b-q6`` (Ollama GPU)`n`n"
        $logDir = Split-Path -Parent $LogFile
        if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir -Force | Out-Null }
        Add-Content -Path $LogFile -Value $logEntry -Encoding UTF8
        Write-Host "[$($task.id)] Marked COMPLETED and logged."
    } else {
        $task.status = "failed"
        Write-Host "[$($task.id)] Marked FAILED after $maxRetries attempts."
    }

    $queueContent | ConvertTo-Json -Depth 5 | Set-Content -Path $QueueFile -Encoding UTF8

    if ($SinglePass) { break }
}
