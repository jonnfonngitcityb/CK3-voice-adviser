# Stops the save watcher started by Start-Watcher.ps1. PowerShell 5.1.

$ErrorActionPreference = "Continue"
$Root = $PSScriptRoot
$PidFile = Join-Path $Root "state\watcher.pid"

if (-not (Test-Path -LiteralPath $PidFile)) {
    Write-Host "The watcher is not running."
    exit 0
}

$procId = (Get-Content -LiteralPath $PidFile -Raw -ErrorAction SilentlyContinue)
if ($procId) { $procId = $procId.Trim() }

if ($procId -match '^\d+$') {
    $proc = Get-Process -Id ([int]$procId) -ErrorAction SilentlyContinue
    if ($proc) {
        Stop-Process -Id ([int]$procId) -Force -ErrorAction SilentlyContinue
        Write-Host "Watcher stopped."
    } else {
        Write-Host "The watcher was already stopped."
    }
} else {
    Write-Host "The watcher was already stopped."
}

Remove-Item -LiteralPath $PidFile -Force -ErrorAction SilentlyContinue
exit 0
