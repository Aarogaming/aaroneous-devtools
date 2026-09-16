$WorklistPath = Join-Path $PSScriptRoot "..\..\docs\WORKLIST.md"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "          AARONEOUS ROADMAP & MILESTONE PROGRESS           " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

if (Test-Path $WorklistPath) {
    $WorklistLines = Get-Content $WorklistPath
    $Active = ($WorklistLines | Select-String '^### A').Count
    $Ready = ($WorklistLines | Select-String '\*\*Status:\*\* Ready').Count

    Write-Host "`n[Canonical Worklist]" -ForegroundColor Yellow
    Write-Host "  Active items: $Active" -ForegroundColor Cyan
    Write-Host "  Ready items:  $Ready" -ForegroundColor Green
    Write-Host "`nCurrent items:" -ForegroundColor Yellow
    $WorklistLines | Select-String '^### [ASB][0-9]' | Select-Object -First 5 | ForEach-Object {
        Write-Host "  -> $($_.Line.TrimStart('#', ' '))" -ForegroundColor White
    }
}

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "Canonical worklist: $WorklistPath" -ForegroundColor Gray
