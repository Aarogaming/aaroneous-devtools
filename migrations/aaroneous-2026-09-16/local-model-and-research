$ollamaExe = "C:\Users\aarog\AppData\Local\Programs\Ollama\ollama.exe"

$models = @(
    @{
        Tag = "qwen3.5:9b-q6"
        GGUF = "C:\Users\aarog\.lmstudio\models\lmstudio-community\Qwen3.5-9B-GGUF\Qwen3.5-9B-Q6_K.gguf"
        Ctx = 32768
    },
    @{
        Tag = "qwen2.5-coder:14b"
        GGUF = "C:\Users\aarog\.lmstudio\models\lmstudio-community\Qwen2.5-Coder-14B-Instruct-GGUF\Qwen2.5-Coder-14B-Instruct-Q4_K_M.gguf"
        Ctx = 32768
    },
    @{
        Tag = "qwen3-coder:30b"
        GGUF = "C:\Users\aarog\.lmstudio\models\lmstudio-community\Qwen3-Coder-30B-A3B-Instruct-GGUF\Qwen3-Coder-30B-A3B-Instruct-Q3_K_L.gguf"
        Ctx = 32768
    }
)

foreach ($m in $models) {
    if (Test-Path $m.GGUF) {
        Write-Host "Updating $($m.Tag)..." -ForegroundColor Cyan
        $tmpFile = [System.IO.Path]::GetTempFileName()
        
        $lines = @(
            "FROM $($m.GGUF)",
            'TEMPLATE """{{- if .System }}<|im_start|>system',
            '{{ .System }}<|im_end|>',
            '{{ end }}{{- range .Messages }}<|im_start|>{{ .Role }}',
            '{{ .Content }}<|im_end|>',
            '{{ end }}<|im_start|>assistant',
            '{{- if .Response }}',
            '{{ .Response }}<|im_end|>',
            '{{ end }}"""',
            'PARAMETER stop "<|im_start|>"',
            'PARAMETER stop "<|im_end|>"',
            'PARAMETER stop "<|endoftext|>"',
            "PARAMETER num_ctx $($m.Ctx)"
        )
        
        [System.IO.File]::WriteAllLines($tmpFile, $lines)
        & $ollamaExe create $m.Tag -f $tmpFile
        Remove-Item $tmpFile -Force
        Write-Host "Done: $($m.Tag)" -ForegroundColor Green
    }
}
