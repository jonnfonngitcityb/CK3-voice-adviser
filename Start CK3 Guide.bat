@echo off
cd /d "%~dp0"
echo Starting the CK3 guide.
echo The first time can take a few minutes. Leave this window open until Grok appears.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Start-CK3.ps1"
echo.
echo Grok has closed. The watcher may still be running.
echo Double-click "Stop the watcher" to turn that off.
pause