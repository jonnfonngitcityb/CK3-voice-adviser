Set-Location $PSScriptRoot
& (Join-Path $PSScriptRoot "Start-Watcher.ps1")
if (-not $?) {
    Write-Host "The watcher did not start. Grok will still open. The save stays on the stub until a snapshot exists."
}
Write-Host "Grok will start in ck3-guide."
Write-Host "Trust this folder if asked. Then press Ctrl+Space, or type /voice."
grok
