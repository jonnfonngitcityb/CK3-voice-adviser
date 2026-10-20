# Builds ck3-extract if needed, then starts the save watcher in the background.
# The watcher keeps running after this script exits. PowerShell 5.1.

$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot
$Exe = Join-Path $Root "watcher\extract\target\release\ck3-extract.exe"

if (-not (Test-Path -LiteralPath $Exe)) {
    $cargo = $null
    $cmd = Get-Command cargo -ErrorAction SilentlyContinue
    if ($cmd) { $cargo = $cmd.Source }
    if (-not $cargo) {
        $local = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
        if (Test-Path -LiteralPath $local) { $cargo = $local }
    }
    if (-not $cargo) {
        Write-Host "Rust is required once. Install it from https://rustup.rs and run this script again."
        exit 1
    }
    Push-Location (Join-Path $Root "watcher\extract")
    Write-Host "Building the save reader. The first time can take a few minutes. Leave this window open."
    & $cargo build --release
    $build = $LASTEXITCODE
    if ($build -ne 0) {
        $rustup = Join-Path (Split-Path -Parent $cargo) "rustup.exe"
        if (-not (Test-Path -LiteralPath $rustup)) {
            $cmd = Get-Command rustup -ErrorAction SilentlyContinue
            if ($cmd) { $rustup = $cmd.Source }
        }
        if ($rustup -and (Test-Path -LiteralPath $rustup)) {
            Write-Host "The first build needs one more piece. Downloading it now. Leave this window open."
            & $rustup toolchain install stable-x86_64-pc-windows-gnu
            if ($LASTEXITCODE -eq 0) {
                & $cargo "+stable-x86_64-pc-windows-gnu" build --release
                $build = $LASTEXITCODE
            } else {
                $build = $LASTEXITCODE
            }
        }
    }
    Pop-Location
    if ($build -ne 0) { exit $build }
}

$watcher = Join-Path $Root "watcher\Watch-Ck3Saves.ps1"
$arg = "-NoProfile -ExecutionPolicy Bypass -File `"$watcher`""
Start-Process -FilePath "powershell.exe" -ArgumentList $arg -WindowStyle Hidden
Write-Host "CK3 save watcher started. It keeps running after Grok closes. Log: state\watch.log"
exit 0
