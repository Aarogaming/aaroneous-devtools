param (
    [string]$Mode = "fix",
    [int]$MaxLoops = 20
)

$QueuePath = "d:\Aaroneous\dev\docs\audits\active\ACTIVE_AUDIT_QUEUE.md"
$LmStudioUrl = "http://127.0.0.1:1234/v1/chat/completions"
$ApiKey = "sk-lm-jK3LgcoP:JMdzXzEQ2ZBrv5mVWIXL"
$Model = "qwen/qwen3.5-9b:2"

Write-Host "Starting Autonomous Fix Loop..." -ForegroundColor Cyan

for ($i = 1; $i -le $MaxLoops; $i++) {
    $QueueContent = Get-Content $QueuePath -Raw
    
    if ($QueueContent -notmatch "- \[ \] \*\*") {
        Write-Host "All audit items in ACTIVE_AUDIT_QUEUE.md are completed!" -ForegroundColor Green
        break
    }
    
    $Match = [regex]::Match($QueueContent, "- \[ \] \*\*([^\*]+)\*\*(?:\r?\n\s+-\s+File:\s+([^\r\n]+))?(?:\r?\n\s+-\s+Action:\s+([^\r\n]+))?")
    $TaskTitle = $Match.Groups[1].Value
    $TargetFile = $Match.Groups[2].Value
    $ActionRequired = $Match.Groups[3].Value

    Write-Host "`n[$i/$MaxLoops] Remediating: $TaskTitle ($TargetFile)" -ForegroundColor Yellow

    $FileContext = ""
    if ($TargetFile -and (Test-Path "d:\Aaroneous\$TargetFile")) {
        $FileContext = Get-Content "d:\Aaroneous\$TargetFile" -Raw
    }

    $Prompt = @"
[FIX: REMEDIATION]
Task: $TaskTitle
Action: $ActionRequired
File: $TargetFile

Code:
$FileContext

Directive:
Provide the clean idiomatic Rust replacement eliminating panic/leak vectors.
"@

    $Body = @{
        model = $Model
        messages = @(
            @{ role = "system"; content = "Role: Aaroneous systems engineer. Output the exact Rust replacement code without filler." },
            @{ role = "user"; content = $Prompt }
        )
        temperature = 0.2
    } | ConvertTo-Json -Depth 5

    try {
        $Response = Invoke-RestMethod -Uri $LmStudioUrl -Method Post -Headers @{ "Authorization" = "Bearer $ApiKey" } -ContentType "application/json" -Body $Body
        $Output = $Response.choices[0].message.content
        Write-Host "Model generated fix for $TaskTitle" -ForegroundColor Green
        
        $FixDir = "d:\Aaroneous\dev\reports\fixes"
        if (!(Test-Path $FixDir)) { New-Item -ItemType Directory -Path $FixDir -Force | Out-Null }
        $SanitizedName = $TaskTitle -replace "[^a-zA-Z0-9_-]", "_"
        $Output | Set-Content "$FixDir\$SanitizedName.rs"
        Write-Host "Saved patch to: dev\reports\fixes\$SanitizedName.rs" -ForegroundColor Gray
    } catch {
        Write-Host "Error querying LM Studio: $_" -ForegroundColor Red
        break
    }

    Start-Sleep -Seconds 2
}
