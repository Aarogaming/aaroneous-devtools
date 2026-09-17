[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$BinaryDirectory,
    [Parameter(Mandatory)][string]$WorkerConfig,
    [Parameter(Mandatory)][string]$RuntimeDirectory,
    [int]$Port = 8787,
    [switch]$AllowJobBuild
)
$ErrorActionPreference = 'Stop'
$utility = Join-Path $BinaryDirectory 'devtools.exe'
$foundation = Join-Path $BinaryDirectory 'foundation.exe'
$arguments = @('--worker-config', $WorkerConfig, '--runtime', $RuntimeDirectory, '--foundation', $foundation, '--port', $Port, '--open')
if ($AllowJobBuild) { $arguments += '--allow-job-build' }
& $utility @arguments
exit $LASTEXITCODE
