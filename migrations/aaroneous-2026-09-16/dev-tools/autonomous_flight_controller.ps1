param (
    [int]$AutoCycles = 5,
    [switch]$ClippyGate = $true,
    [switch]$AutoRollback = $true,
    [switch]$AutoBranch = $true,
    [switch]$NonInteractive = $false,
    [int]$MaxGpuTemp = 80,
    [switch]$RunTests = $true,
    [switch]$EnforceFormat = $true,
    [switch]$RunSecurity = $true,
    [switch]$BuildArtifacts = $true
)

# ------------------------------------------------------------------
# ENVIRONMENT: Ensure Cargo & Rust Binaries in PATH
# ------------------------------------------------------------------
$CargoBin = "$env:USERPROFILE\.cargo\bin"
if ((Test-Path $CargoBin) -and ($env:PATH -notlike "*$CargoBin*")) {
    $env:PATH = "$CargoBin;$env:PATH"
}

$RepoRoot = (Resolve-Path "$PSScriptRoot\..\..").Path
$LogDir = "$RepoRoot\dev\docs\audits\logs"
if (!(Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
}
$FlightLog = "$LogDir\flight_controller.log"

function Write-FlightLog {
    param([string]$Message, [string]$Level = "INFO")
    $Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $Formatted = "[$Timestamp] [$Level] $Message"
    Add-Content -Path $FlightLog -Value $Formatted -Encoding utf8
}

Clear-Host
Write-Host "==========================================================" -ForegroundColor Magenta
Write-Host "        AARONEOUS AUTONOMOUS RUNTIME CONTROLLER (ARC)     " -ForegroundColor Magenta
Write-Host "              [Plan -> Audit -> Fix Engine]              " -ForegroundColor Magenta
Write-Host "==========================================================" -ForegroundColor Magenta
Write-FlightLog "Runtime Controller session initialized."

$QueuePath = "$RepoRoot\dev\docs\audits\active\ACTIVE_AUDIT_QUEUE.md"
$RepairLogPath = "$RepoRoot\dev\docs\audits\REPAIR_LOG.md"
$ChangelogPath = "$RepoRoot\CHANGELOG.md"

# ------------------------------------------------------------------
# MAINTENANCE: Check & Protect Git Branch
# ------------------------------------------------------------------
$CurrentBranch = (git branch --show-current).Trim()
if ($AutoBranch -and ($CurrentBranch -eq "main" -or $CurrentBranch -eq "master")) {
    $DateTag = Get-Date -Format "yyyyMMdd-HHmm"
    $FlightBranch = "flight/auto-$DateTag"
    Write-Host "`n[GIT SAFETY] Currently on protected branch '$CurrentBranch'." -ForegroundColor Yellow
    Write-Host "  -> Auto-creating isolated flight branch: $FlightBranch" -ForegroundColor Cyan
    Write-FlightLog "Branch safety: Branching from $CurrentBranch to $FlightBranch" "WARN"
    git checkout -b $FlightBranch | Out-Null
    Write-Host "  -> Switched to $FlightBranch. Main is safe from unreviewed commits." -ForegroundColor Green
} else {
    Write-Host "`n[GIT] Active working branch: $CurrentBranch" -ForegroundColor DarkCyan
}

# Prompt user for execution mode
if ($NonInteractive) {
    $TotalCycles = $AutoCycles
    Write-Host "`n[MODE] Non-Interactive Auto Mode activated ($TotalCycles cycles)." -ForegroundColor Cyan
} else {
    $Answer = Read-Host "`nRun in Continuous Auto Mode? (y/n, default: n)"
    if ($Answer.Trim().ToLower() -eq "y") {
        $TotalCycles = $AutoCycles
        Write-Host "`n[MODE] Continuous Auto Mode activated ($TotalCycles cycles)." -ForegroundColor Cyan
    } else {
        $TotalCycles = 1
        Write-Host "`n[MODE] One-Off Single Flight activated (1 cycle)." -ForegroundColor Cyan
    }
}

# ------------------------------------------------------------------
# MAINTENANCE: GPU Thermal Safety Check
# ------------------------------------------------------------------
function Check-GpuThermals {
    try {
        $GpuInfo = nvidia-smi --query-gpu=temperature.gpu,memory.used --format=csv,noheader,nounits 2>$null
        if ($GpuInfo) {
            $Parts = $GpuInfo -split ","
            $Temp = [int]$Parts[0].Trim()
            $Vram = [int]$Parts[1].Trim()
            Write-Host "  [HARDWARE] GPU Temp: ${Temp}°C | VRAM: ${Vram} MB" -ForegroundColor DarkGray
            if ($Temp -ge $MaxGpuTemp) {
                Write-Host "  ⚠️ [THERMAL WARNING] GPU reached ${Temp}°C (Limit: ${MaxGpuTemp}°C)! Cooling down for 30s..." -ForegroundColor Red
                Start-Sleep -Seconds 30
            }
        }
    } catch {
        # nvidia-smi not in PATH or non-NVIDIA system
    }
}

# ------------------------------------------------------------------
# MAINTENANCE: Build Cache Size Check
# ------------------------------------------------------------------
function Check-BuildArtifactSize {
    $TargetDir = "$RepoRoot\target"
    if (Test-Path $TargetDir) {
        $SizeGB = [math]::Round(((Get-ChildItem $TargetDir -Recurse -File -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum / 1GB), 1)
        if ($SizeGB -ge 30) {
            Write-Host "`n  [STORAGE] target/ cache size is ${SizeGB} GB. Running cargo clean to prevent disk bloat..." -ForegroundColor Yellow
            cargo clean | Out-Null
            Write-Host "  [STORAGE] Cache cleaned successfully." -ForegroundColor Green
        }
    }
}

# ------------------------------------------------------------------
# MAINTENANCE: Sweep Completed Queue Items
# ------------------------------------------------------------------
function Sweep-CompletedTasks {
    if (!(Test-Path $QueuePath)) { return }

    $RawQueue = Get-Content $QueuePath -Raw
    $ItemMatches = [regex]::Matches($RawQueue, "(?ms)^-\s*\[x\]\s*\*\*([^\*]+)\*\*(.*?)(?=^-\s*\[[ x]\]|\z)")

    if ($ItemMatches.Count -gt 0) {
        $DateStr = Get-Date -Format "yyyy-MM-dd HH:mm"
        Write-Host "`n[SWEEP] Processing $($ItemMatches.Count) resolved task(s)..." -ForegroundColor Yellow

        $RepairHeader = "`n### 🛠️ Batch Remediation Sweep [$DateStr]`n"
        $RepairEntries = @()
        $ChangelogEntries = @()

        foreach ($m in $ItemMatches) {
            $TaskTitle = $m.Groups[1].Value.Trim()
            $TaskDetails = $m.Groups[2].Value.Trim()

            $RepairEntries += "- **$TaskTitle**`n  $TaskDetails"
            $ChangelogEntries += "- **$TaskTitle**: Remediated and verified via autonomous audit cycle."
        }

        $FullRepairBlock = $RepairHeader + ($RepairEntries -join "`n`n") + "`n"
        Add-Content -Path $RepairLogPath -Value $FullRepairBlock -Encoding utf8
        Write-Host "  -> Appended to dev/docs/audits/REPAIR_LOG.md" -ForegroundColor Green

        if (Test-Path $ChangelogPath) {
            $ChangelogContent = Get-Content $ChangelogPath -Raw
            if ($ChangelogContent -match "## \[Unreleased\]") {
                $ChangelogBlock = "`n`n### 🛡️ Automated Audit Remediations [$DateStr]`n" + ($ChangelogEntries -join "`n")
                $UpdatedChangelog = $ChangelogContent -replace "(## \[Unreleased\])", "`$1$ChangelogBlock"
                Set-Content -Path $ChangelogPath -Value $UpdatedChangelog -Encoding utf8
                Write-Host "  -> Synced into CHANGELOG.md under [Unreleased]" -ForegroundColor Green
            }
        }

        $CleanedQueue = $RawQueue
        foreach ($m in $ItemMatches) {
            $CleanedQueue = $CleanedQueue.Replace($m.Value, "")
        }
        $CleanedQueue = $CleanedQueue -replace "(\r?\n){3,}", "`r`n`r`n"
        Set-Content -Path $QueuePath -Value $CleanedQueue.Trim() -Encoding utf8
        Write-Host "  -> Purged resolved items from ACTIVE_AUDIT_QUEUE.md (prompt tokens optimized)." -ForegroundColor DarkGray
    }
}

for ($cycle = 1; $cycle -le $TotalCycles; $cycle++) {
    Write-Host "`n==========================================================" -ForegroundColor Cyan
    Write-Host ">>> CYCLE $cycle OF ${TotalCycles} - INITIATING TRI-PHASE ROTATION <<<" -ForegroundColor Cyan
    Write-Host "==========================================================" -ForegroundColor Cyan

    Check-GpuThermals

    # ----------------------------------------------------
    # PHASE 1: PLAN (Frontier Spec / Gap Analysis)
    # ----------------------------------------------------
    Write-Host "`n[PHASE 1: PLAN] Running Frontier Architect..." -ForegroundColor Magenta
    Write-FlightLog "Cycle $cycle - Starting Phase 1: Plan"
    $PlanOut = & npx opencode run --agent architect --command plan --thinking --auto "Top 1 high-depth architectural enhancement or missing crate README/spec sheet" 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-FlightLog "Cycle $cycle - Phase 1 Plan encountered warning or non-zero exit ($LASTEXITCODE): $PlanOut" "WARN"
    }
    Write-Host "Plan phase finished. Pausing 3s..." -ForegroundColor DarkGray
    Start-Sleep -Seconds 3

    # ----------------------------------------------------
    # PHASE 2: AUDIT (Forensic Scan & Queue Enqueue)
    # ----------------------------------------------------
    Write-Host "`n[PHASE 2: AUDIT] Running Safety & Debt Audit..." -ForegroundColor Yellow
    Write-FlightLog "Cycle $cycle - Starting Phase 2: Audit"
    $AuditOut = & npx opencode run --agent auditor --command audit --thinking --auto 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-FlightLog "Cycle $cycle - Phase 2 Audit encountered warning or non-zero exit ($LASTEXITCODE): $AuditOut" "WARN"
    }
    Write-Host "Audit sweep finished. Pausing 3s..." -ForegroundColor DarkGray
    Start-Sleep -Seconds 3

    # ----------------------------------------------------
    # PHASE 3: FIX (Autonomous Remediation with Rollback Gate)
    # ----------------------------------------------------
    Write-Host "`n[PHASE 3: FIX] Checking Active Queue for Remediation..." -ForegroundColor Green
    Write-FlightLog "Cycle $cycle - Starting Phase 3: Fix"
    $QueueContent = Get-Content $QueuePath -Raw -ErrorAction SilentlyContinue
    if ($QueueContent -match "- \[ \] \*\*") {
        for ($f = 1; $f -le 3; $f++) {
            $CurrentQueue = Get-Content $QueuePath -Raw -ErrorAction SilentlyContinue
            if ($CurrentQueue -notmatch "- \[ \] \*\*") { break }

            $Match = [regex]::Match($CurrentQueue, "- \[ \] \*\*([^\*]+)\*\*")
            $TaskTitle = $Match.Groups[1].Value
            Write-FlightLog "Cycle $cycle - Remediation subtask ${f}: $TaskTitle"
            $SubtaskLog = "$LogDir\subtask_${cycle}_${f}.log"
            # Execute with a 300-second (5 min) watchdog timeout to kill stuck/looping models
            $FixPrompt = "`"Remediate pending task: $TaskTitle`""
            $Process = Start-Process -FilePath "cmd.exe" -ArgumentList "/c npx opencode run --agent auditor --command fix --thinking --auto $FixPrompt > `"$SubtaskLog`" 2>&1" -PassThru -NoNewWindow
            $Completed = $Process.WaitForExit(300000) # 5 minutes max

            if (Test-Path $SubtaskLog) {
                $LogSnippet = Get-Content $SubtaskLog -Tail 15 -ErrorAction SilentlyContinue | Out-String
                if ($LogSnippet -match "(?i)error|exception|fail") {
                    Write-FlightLog "Cycle $cycle - Fix agent output flagged issues for ${TaskTitle}:`n$LogSnippet" "WARN"
                }
            }

            if (!$Completed) {
                Write-Host "     ⚠️ [WATCHDOG TIMEOUT] Model stalled or looped for >5m. Terminating turn..." -ForegroundColor Red
                Write-FlightLog "Cycle $cycle - Watchdog timeout (>5m) on $TaskTitle" "ERROR"
                Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
                if ($AutoRollback) {
                    git checkout -- .
                    Write-Host "     Clean state restored after timeout." -ForegroundColor DarkYellow
                    Write-FlightLog "Cycle $cycle - Rollback executed after timeout." "WARN"
                }
                continue
            }
            $CargoCheck = & cargo check --workspace 2>&1
            $CheckFailed = ($LASTEXITCODE -ne 0)

            if (!$CheckFailed -and $RunTests) {
                Write-Host "     [TEST] Running unit tests for validation..." -ForegroundColor Yellow
                $CargoTest = & cargo test --workspace 2>&1
                if ($LASTEXITCODE -ne 0) {
                    Write-Host "     [ERROR] Tests failed on $TaskTitle!" -ForegroundColor Red
                    Write-FlightLog "Cycle $cycle - Tests failed on $TaskTitle. Test output:`n$CargoTest" "ERROR"
                    $CheckFailed = $true
                }
            }

            if ($CheckFailed) {
                Write-Host "     [ERROR] Validation failed on $TaskTitle!" -ForegroundColor Red
                if ($AutoRollback) {
                    Write-Host "     [ROLLBACK] Reverting modifications to preserve integrity..." -ForegroundColor Yellow
                    git checkout -- .
                    Write-Host "     Clean state restored." -ForegroundColor DarkYellow
                    Write-FlightLog "Cycle $cycle - Rollback executed." "WARN"
                }
            } else {
                if ($EnforceFormat) {
                    Write-Host "     [FORMAT] Formatting code..." -ForegroundColor DarkGray
                    & cargo fmt --all
                }
                
                Write-Host "     [SUCCESS] Code changes validated cleanly!" -ForegroundColor Green
                Write-FlightLog "Cycle $cycle - Remediation validated for $TaskTitle" "INFO"

                # If the agent modified code and the compiler verified it clean, ensure the queue item is marked [x]
                $GitDiff = git status --porcelain
                if ($GitDiff) {
                    $QueueUpdate = Get-Content $QueuePath -Raw -ErrorAction SilentlyContinue
                    if ($QueueUpdate -match "- \[ \] \*\*\Q$TaskTitle\E\*\*") {
                        $QueueUpdate = $QueueUpdate.Replace("- [ ] **$TaskTitle**", "- [x] **$TaskTitle**")
                        Set-Content -Path $QueuePath -Value $QueueUpdate -Encoding utf8
                        Write-Host "     [AUTO-RESOLVE] Marked task [x] in ACTIVE_AUDIT_QUEUE.md" -ForegroundColor Green
                        Write-FlightLog "Cycle $cycle - Auto-marked [x] for $TaskTitle" "INFO"
                    }
                }
            }

            Start-Sleep -Seconds 2
        }
    } else {
        Write-Host "  No pending defects in queue. Skipping Fix phase." -ForegroundColor DarkGray
        Write-FlightLog "Cycle $cycle - No pending defects found in active queue." "INFO"
    }

    # ----------------------------------------------------
    # PHASE 4: POST-FIX SWEEP (REPAIR_LOG & CHANGELOG)
    # ----------------------------------------------------
    Sweep-CompletedTasks

    # ----------------------------------------------------
    # PHASE 5: VERIFICATION & CLIPPY GATE
    # ----------------------------------------------------
    Write-Host "`n[PHASE 5: VERIFY] Running Full Workspace Gatekeeper..." -ForegroundColor Cyan
    
    if ($EnforceFormat) {
        Write-Host "Running Code Formatter (cargo fmt)..." -ForegroundColor Yellow
        & cargo fmt --all
    }

    $CargoCheckFinal = & cargo check --workspace 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Workspace compilation failed at end of Cycle $cycle!" -ForegroundColor Red
        Write-FlightLog "Cycle $cycle - Final workspace compilation failed: $CargoCheckFinal" "ERROR"
        if ($TotalCycles -eq 1) { break }
    } else {
        Write-Host "[SUCCESS] Workspace compiles cleanly!" -ForegroundColor Green
        Write-FlightLog "Cycle $cycle - Workspace compiles cleanly." "INFO"
        if ($ClippyGate) {
            Write-Host "Running Clippy Quality Inspection..." -ForegroundColor Yellow
            $ClippyOut = & cargo clippy --workspace --no-deps 2>&1
            if ($LASTEXITCODE -ne 0) {
                Write-Host "[NOTICE] Clippy flagged items (captured for next audit pass)." -ForegroundColor DarkYellow
                Write-FlightLog "Cycle $cycle - Clippy warnings flagged" "WARN"
            } else {
                Write-Host "[SUCCESS] Zero clippy warnings across workspace!" -ForegroundColor Green
                Write-FlightLog "Cycle $cycle - Zero clippy warnings across workspace" "INFO"
            }
        }

        if ($RunTests) {
            Write-Host "Running Workspace Test Suite..." -ForegroundColor Yellow
            $FinalTestOut = & cargo test --workspace 2>&1
            if ($LASTEXITCODE -ne 0) {
                Write-Host "[ERROR] Workspace tests failed at end of Cycle $cycle!" -ForegroundColor Red
                Write-FlightLog "Cycle $cycle - Final workspace tests failed." "ERROR"
            } else {
                Write-Host "[SUCCESS] All workspace tests passed!" -ForegroundColor Green
            }
        }

        if ($RunSecurity) {
            Write-Host "Running Supply Chain Security Audit..." -ForegroundColor Yellow
            if (!(Get-Command cargo-audit -ErrorAction SilentlyContinue)) {
                Write-Host "  -> Installing cargo-audit..." -ForegroundColor DarkGray
                & cargo install cargo-audit
            }
            $AuditSecOut = & cargo audit 2>&1
            if ($LASTEXITCODE -ne 0) {
                Write-Host "[NOTICE] Vulnerabilities detected in supply chain." -ForegroundColor DarkYellow
                Write-FlightLog "Cycle $cycle - cargo audit flagged dependencies:`n$AuditSecOut" "WARN"
            } else {
                Write-Host "[SUCCESS] Supply chain is secure!" -ForegroundColor Green
            }
        }

        # ----------------------------------------------------
        # PHASE 6: ATOMIC GIT COMMIT
        # ----------------------------------------------------
        $GitStatus = git status --porcelain
        if ($GitStatus) {
            Write-Host "`n[GIT] Creating atomic commit for verified Cycle $cycle changes..." -ForegroundColor Cyan
            git add -A
            git commit -m "chore(flight): verified autonomous cycle $cycle [skip ci]" | Out-Null
            Write-Host "  -> Committed cycle progress cleanly to $CurrentBranch." -ForegroundColor Green
        }
    }

    Check-BuildArtifactSize

    if ($cycle -lt $TotalCycles) {
        Write-Host "`nCycle $cycle complete. VRAM cooldown for 5s before next rotation..." -ForegroundColor DarkGray
        Start-Sleep -Seconds 5
    }
}

# ----------------------------------------------------
# PHASE 7: DELIVERY (Artifact Generation)
# ----------------------------------------------------
if ($BuildArtifacts -and $LASTEXITCODE -eq 0) {
    Write-Host "`n[PHASE 7: DELIVERY] Building Release Artifacts..." -ForegroundColor Magenta
    Write-FlightLog "Starting Phase 7: Release build and packaging"
    & cargo build --release -p a_run
    
    $ReleaseDir = "$RepoRoot\releases"
    if (!(Test-Path $ReleaseDir)) { New-Item -ItemType Directory -Path $ReleaseDir | Out-Null }
    
    $ZipPath = "$ReleaseDir\Aaroneous_Flight_$DateTag.zip"
    $Exes = Get-ChildItem -Path "$RepoRoot\target\release\*.exe" -ErrorAction SilentlyContinue
    if ($Exes) {
        Compress-Archive -Path $Exes.FullName -DestinationPath $ZipPath -Force
        Write-Host "  -> Release packaged: $ZipPath" -ForegroundColor Green
        Write-FlightLog "Release artifact generated: $ZipPath" "INFO"
    } else {
        Write-Host "  -> No binaries found in target/release to package." -ForegroundColor DarkYellow
    }
}

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "🎉 Runtime Controller finished execution!" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan
