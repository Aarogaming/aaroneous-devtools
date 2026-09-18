[CmdletBinding()]
param (
    [Parameter(Mandatory=$true)]
    [string]$PromptFile,

    [Parameter(Mandatory=$false)]
    [string]$OutputFile,

    [Parameter(Mandatory=$false)]
    [string]$Model = "qwen3.5:9b-q6",

    [Parameter(Mandatory=$false)]
    [string]$SystemPrompt = "You are a senior Rust systems programmer. Do NOT output any lengthy thinking trace or reasoning. Output only pure, complete Rust code.",

    [Parameter(Mandatory=$false)]
    [int]$NumPredict = 4096,

    [Parameter(Mandatory=$false)]
    [float]$Temperature = 0.0
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $PromptFile)) {
    Write-Error "Prompt file not found: $PromptFile"
}

$promptContent = Get-Content -Path $PromptFile -Raw -Encoding UTF8

$body = @{
    model = $Model
    messages = @(
        @{
            role = "system"
            content = $SystemPrompt
        },
        @{
            role = "user"
            content = $promptContent
        }
    )
    options = @{
        temperature = $Temperature
        num_predict = $NumPredict
    }
    stream = $false
} | ConvertTo-Json -Depth 5

Write-Host "Dispatching prompt to local Ollama API ($Model)..."
$response = Invoke-RestMethod -Uri "http://localhost:11434/api/chat" -Method Post -Body $body -ContentType "application/json"

$generatedContent = $response.message.content

# Strip markdown code fencing if present
if ($generatedContent -match '(?s)```(?:rust)?\s*(.*?)\s*```') {
    $generatedContent = $matches[1]
}

if ($OutputFile) {
    $outDir = Split-Path -Parent $OutputFile
    if ($outDir -and -not (Test-Path $outDir)) {
        New-Item -ItemType Directory -Path $outDir -Force | Out-Null
    }
    Set-Content -Path $OutputFile -Value $generatedContent -Encoding UTF8
    Write-Host "Output successfully written to $OutputFile"
} else {
    Write-Output $generatedContent
}
