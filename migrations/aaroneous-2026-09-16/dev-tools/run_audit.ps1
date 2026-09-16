param (
    [switch]$Json = $false
)

$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$ReportsDir = "d:\Aaroneous\dev\reports"
$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"

Write-Host "👁️ Initiating Native Codebase Audit..." -ForegroundColor Cyan

# 1. Run Cargo Clippy
Write-Host "Running Cargo Clippy (Workspace, All Features)..." -ForegroundColor Yellow
if ($Json) {
    & $CargoPath clippy --workspace --all-features --message-format=json > "$ReportsDir\clippy_$Timestamp.json"
} else {
    & $CargoPath clippy --workspace --all-features > "$ReportsDir\clippy_$Timestamp.txt" 2>&1
}

if ($LASTEXITCODE -eq 0) {
    Write-Host "Clippy complete. Zero errors!" -ForegroundColor Green
} else {
    Write-Host "Clippy found issues. See reports directory." -ForegroundColor Red
}

# 2. Run Cargo Audit (Checks dependencies for CVEs)
Write-Host "Running Cargo Audit (CVE Check)..." -ForegroundColor Yellow
try {
    if ($Json) {
        & $CargoPath audit --json > "$ReportsDir\audit_$Timestamp.json" 2>&1
    } else {
        & $CargoPath audit > "$ReportsDir\audit_$Timestamp.txt" 2>&1
    }
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Audit complete. Zero vulnerabilities found!" -ForegroundColor Green
    } else {
        Write-Host "Audit found vulnerabilities. See reports directory." -ForegroundColor Red
    }
} catch {
    Write-Host "cargo-audit is not installed. Run: $CargoPath install cargo-audit" -ForegroundColor DarkGray
}

Write-Host "Audit Sweep Complete. Reports saved to $ReportsDir" -ForegroundColor Cyan
