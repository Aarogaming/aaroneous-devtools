param (
    [string]$Topic = ""
)

Clear-Host
Write-Host "==========================================================" -ForegroundColor Magenta
Write-Host "           AARONEOUS FRONTIER DEVELOPMENT PLANNER         " -ForegroundColor Magenta
Write-Host "==========================================================" -ForegroundColor Magenta
Write-Host "Focus Domains:" -ForegroundColor Yellow
Write-Host "  1. .si Model Container & Cartridge Advancements" -ForegroundColor White
Write-Host "  2. Zero-Bloat UI / Studio Ergonomics (WGPU / egui)" -ForegroundColor White
Write-Host "  3. Crate Spec Sheets, READMEs & Protocol Standards" -ForegroundColor White
Write-Host "  4. High-ROI Machine-Native Capabilities (Vision/SSM)" -ForegroundColor White
Write-Host "==========================================================" -ForegroundColor Magenta

if ([string]::IsNullOrWhiteSpace($Topic)) {
    Write-Host "`nEnter planning focus or press Enter for general frontier roadmap review:" -ForegroundColor Cyan
    $Topic = Read-Host "> "
}

if ([string]::IsNullOrWhiteSpace($Topic)) {
    $Prompt = "/plan Synthesize the top 3 zero-bloat frontier advancements for Aaroneous across .si container format, UI ergonomics, and crate specifications."
} else {
    $Prompt = "/plan $Topic"
}

Write-Host "`nLaunching OpenCode with Frontier Architect..." -ForegroundColor Yellow
npx opencode run --agent architect --auto $Prompt
