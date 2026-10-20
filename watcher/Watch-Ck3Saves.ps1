# Reads the newest CK3 ironman autosave and writes state/realm.md.
# One instance. Leaves state/spoken.md alone. PowerShell 5.1.

$ErrorActionPreference = "Continue"
$Root = Split-Path -Parent $PSScriptRoot
$State = Join-Path $Root "state"
$Exe = Join-Path $Root "watcher\extract\target\release\ck3-extract.exe"
$Log = Join-Path $State "watch.log"
$PidFile = Join-Path $State "watcher.pid"

function Stamp {
    Get-Date -Format "yyyy-MM-ddTHH:mm:ss"
}

function Write-Log([string]$line) {
    New-Item -ItemType Directory -Force -Path $State | Out-Null
    Add-Content -LiteralPath $Log -Value $line -Encoding utf8
}

function Test-WatcherRunning([string]$procId) {
    if ($procId -notmatch '^\d+$') {
        return $false
    }
    $proc = Get-Process -Id ([int]$procId) -ErrorAction SilentlyContinue
    return ($null -ne $proc -and $proc.Id -ne $PID)
}

if (Test-Path -LiteralPath $PidFile) {
    $old = (Get-Content -LiteralPath $PidFile -Raw -ErrorAction SilentlyContinue)
    if ($old) { $old = $old.Trim() }
    if (Test-WatcherRunning $old) {
        Write-Output "Watcher already running ($old)."
        exit 0
    }
}

New-Item -ItemType Directory -Force -Path $State | Out-Null
Set-Content -LiteralPath $PidFile -Value $PID -Encoding ascii

$script:SaveFolder = ""
$script:SaveName = "autosave.ck3"
$script:SettingsSig = $null

function Read-Settings {
    $path = Join-Path $Root "watcher-settings.txt"
    $folder = ""
    $name = "autosave.ck3"
    $unused = New-Object System.Collections.Generic.List[string]
    if (Test-Path -LiteralPath $path) {
        foreach ($raw in @(Get-Content -LiteralPath $path -ErrorAction SilentlyContinue)) {
            $line = "$raw".Trim()
            if ($line.Length -eq 0) { continue }
            if ($line.StartsWith("#")) { continue }
            $eq = $line.IndexOf("=")
            if ($eq -lt 1) { continue }
            $key = $line.Substring(0, $eq).Trim().ToLower()
            $val = $line.Substring($eq + 1).Trim()
            if ($val.Length -ge 2 -and $val.StartsWith('"') -and $val.EndsWith('"')) {
                $val = $val.Substring(1, $val.Length - 2).Trim()
            }
            if ($key -eq "save folder") {
                $folder = $val
                continue
            }
            if ($key -eq "save name") {
                if ($val) { $name = $val }
                continue
            }
            $unused.Add($key)
        }
    }
    $fileNote = ""
    if ($folder -and (Test-Path -LiteralPath $folder -PathType Leaf)) {
        $folder = Split-Path -Parent $folder
        $fileNote = "save folder was a file; using the folder around it"
    }
    $script:SaveFolder = $folder
    $script:SaveName = $name
    $sig = "$folder|$name|$($unused -join ',')|$fileNote"
    if ($sig -ne $script:SettingsSig) {
        $script:SettingsSig = $sig
        Write-Log "$(Stamp) settings save-folder='$folder' save-name='$name'"
        if ($fileNote) {
            Write-Log "$(Stamp) $fileNote"
        }
        if ($folder -and -not (Test-Path -LiteralPath $folder)) {
            Write-Log "$(Stamp) save folder not found: $folder"
        }
        if ($unused.Count -gt 0) {
            Write-Log "$(Stamp) settings lines not used: $($unused -join ', ')"
        }
    }
}

function Get-SaveRoots {
    Read-Settings
    $roots = New-Object System.Collections.Generic.List[string]
    if ($script:SaveFolder) {
        $roots.Add($script:SaveFolder)
        return ($roots | Select-Object -Unique)
    }
    $docs = [Environment]::GetFolderPath("MyDocuments")
    if ($docs) {
        $roots.Add((Join-Path $docs "Paradox Interactive\Crusader Kings III\save games"))
    }
    $profile = $env:USERPROFILE
    if ($profile) {
        $roots.Add((Join-Path $profile "OneDrive\Documents\Paradox Interactive\Crusader Kings III\save games"))
        Get-ChildItem -LiteralPath $profile -Directory -ErrorAction SilentlyContinue | ForEach-Object {
            if ($_.Name -like "OneDrive*") {
                $roots.Add((Join-Path $_.FullName "Documents\Paradox Interactive\Crusader Kings III\save games"))
            }
        }
        $steamRoots = @(
            (Join-Path $profile "Steam\userdata"),
            "C:\Program Files (x86)\Steam\userdata",
            "C:\Program Files\Steam\userdata"
        )
        foreach ($steam in $steamRoots) {
            if (-not (Test-Path -LiteralPath $steam)) { continue }
            Get-ChildItem -LiteralPath $steam -Directory -ErrorAction SilentlyContinue | ForEach-Object {
                $roots.Add((Join-Path $_.FullName "1158310\remote\save games"))
            }
        }
    }
    return ($roots | Select-Object -Unique)
}

function Get-Candidates {
    $list = New-Object System.Collections.Generic.List[System.IO.FileInfo]
    foreach ($root in (Get-SaveRoots)) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        Get-ChildItem -LiteralPath $root -Filter *.ck3 -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object {
            if ($_.Name -like "*_backup*") { return }
            if ($_.Length -lt 64) { return }
            $list.Add($_)
        }
        $parent = Split-Path -Parent $root
        foreach ($name in @("autosave_exit.ck3", "last_save.ck3")) {
            $special = Join-Path $parent $name
            if (Test-Path -LiteralPath $special) {
                $item = Get-Item -LiteralPath $special
                if ($item.Length -ge 64) { $list.Add($item) }
            }
        }
    }
    return $list
}

function Select-Target($files) {
    $files = @($files | Where-Object { $_ -is [System.IO.FileInfo] })
    if ($files.Count -eq 0) {
        return $null
    }
    $wanted = $script:SaveName
    if (-not $wanted) { $wanted = "autosave.ck3" }
    $autos = $files | Where-Object { $_.Name -ieq $wanted } | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
    if ($autos) {
        $newer = $files | Where-Object {
            $_.FullName -ne $autos.FullName -and $_.LastWriteTimeUtc -gt $autos.LastWriteTimeUtc
        } | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
        if ($newer) {
            Write-Log "$(Stamp) newer than ${wanted}: $($newer.FullName)"
            return $newer
        }
        return $autos
    }
    $ordered = @($files | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 3)
    foreach ($file in $ordered) {
        $probe = & $Exe probe $file.FullName 2>&1 | Out-String
        Write-Log "$(Stamp) probe $($file.FullName) $probe"
        if ($probe -match "ironman=yes") {
            Write-Log "$(Stamp) no ${wanted}; using $($file.FullName)"
            return $file
        }
    }
    Write-Log "$(Stamp) no ${wanted} and no ironman header in the newest saves"
    return $null
}

function Wait-Stable([string]$path) {
    $lastLen = -1
    $since = Get-Date
    $deadline = (Get-Date).AddSeconds(120)
    while ((Get-Date) -lt $deadline) {
        if (-not (Test-Path -LiteralPath $path)) { return $false }
        $item = Get-Item -LiteralPath $path
        $opened = $false
        try {
            $stream = [System.IO.File]::Open($path, 'Open', 'Read', 'ReadWrite')
            $stream.Close()
            $opened = $true
        } catch {
            $opened = $false
        }
        if ($opened -and $item.Length -gt 0 -and $item.Length -eq $lastLen -and ((Get-Date) - $since).TotalSeconds -ge 3) {
            return $true
        }
        if ($item.Length -ne $lastLen) {
            $lastLen = $item.Length
            $since = Get-Date
        }
        Start-Sleep -Seconds 1
    }
    return $false
}

function Wait-Tick {
    $event = Wait-Event -Timeout 2 -ErrorAction SilentlyContinue
    if ($event) {
        Remove-Event -EventIdentifier $event.EventIdentifier -ErrorAction SilentlyContinue
    }
}

function Install-Snapshot([string]$md, [string]$json) {
    $realm = Join-Path $State "realm.md"
    $latest = Join-Path $State "latest.json"
    Move-Item -LiteralPath $md -Destination $realm -Force
    Move-Item -LiteralPath $json -Destination $latest -Force
}

$script:watchers = @()
try {
    if (-not (Test-Path -LiteralPath $Exe)) {
        Write-Log "$(Stamp) missing $Exe"
        Write-Output "ck3-extract.exe is missing. Run Start-Watcher.ps1 so it can build."
        exit 1
    }
    $index = 0
    foreach ($root in (Get-SaveRoots)) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        $watcher = New-Object System.IO.FileSystemWatcher
        $watcher.Path = $root
        $watcher.Filter = "*.ck3"
        $watcher.IncludeSubdirectories = $true
        $watcher.EnableRaisingEvents = $true
        Register-ObjectEvent -InputObject $watcher -EventName Created -SourceIdentifier "ck3save$index" | Out-Null
        $index++
        Register-ObjectEvent -InputObject $watcher -EventName Changed -SourceIdentifier "ck3save$index" | Out-Null
        $index++
        $script:watchers += $watcher
    }
    Write-Log "$(Stamp) watcher started pid=$PID"
    $done = ""
    while ($true) {
        try {
        $target = Select-Target (Get-Candidates)
        if (-not $target) {
            Wait-Tick
            continue
        }
        if (-not (Test-Path -LiteralPath $target.FullName)) {
            Wait-Tick
            continue
        }
        $fresh = Get-Item -LiteralPath $target.FullName
        $sig = "{0}|{1}|{2}" -f $fresh.FullName, $fresh.Length, $fresh.LastWriteTimeUtc.Ticks
        if ($sig -eq $done) {
            Wait-Tick
            continue
        }
        if (-not (Wait-Stable $fresh.FullName)) {
            Write-Log "$(Stamp) file not stable $($fresh.FullName)"
            Wait-Tick
            continue
        }
        $fresh = Get-Item -LiteralPath $fresh.FullName
        $sig = "{0}|{1}|{2}" -f $fresh.FullName, $fresh.Length, $fresh.LastWriteTimeUtc.Ticks
        $temp = Join-Path $env:TEMP ("ck3-guide-" + [guid]::NewGuid().ToString("n") + ".ck3")
        try {
            Copy-Item -LiteralPath $fresh.FullName -Destination $temp -Force
        } catch {
            Write-Log "$(Stamp) copy failed $($fresh.FullName) $($_.Exception.Message)"
            Wait-Tick
            continue
        }
        $tmpMd = Join-Path $env:TEMP "ck3-guide-realm.md"
        $tmpJson = Join-Path $env:TEMP "ck3-guide-realm.json"
        $output = & $Exe write $temp --md $tmpMd --json $tmpJson --label $fresh.Name 2>&1 | Out-String
        $code = $LASTEXITCODE
        Remove-Item -LiteralPath $temp -Force -ErrorAction SilentlyContinue
        Write-Log "$(Stamp) file=$($fresh.FullName) exit=$code"
        Write-Log $output
        if ($code -eq 0) {
            Install-Snapshot $tmpMd $tmpJson
            Write-Log "$(Stamp) installed realm.md from $($fresh.Name)"
        } elseif ($code -eq 2) {
            $realm = Join-Path $State "realm.md"
            if (-not (Test-Path -LiteralPath $realm)) {
                Install-Snapshot $tmpMd $tmpJson
                Write-Log "$(Stamp) installed header-only realm.md from $($fresh.Name)"
            } else {
                Remove-Item -LiteralPath $tmpMd, $tmpJson -Force -ErrorAction SilentlyContinue
                Write-Log "$(Stamp) header-only, kept previous realm.md"
            }
        } else {
            Remove-Item -LiteralPath $tmpMd, $tmpJson -Force -ErrorAction SilentlyContinue
            Write-Log "$(Stamp) kept previous realm.md"
        }
        $done = $sig
        } catch {
            Write-Log "$(Stamp) $($_.Exception.Message)"
            Wait-Tick
        }
    }
} finally {
    Get-EventSubscriber -ErrorAction SilentlyContinue | Where-Object { $_.SourceIdentifier -like "ck3save*" } | ForEach-Object {
        Unregister-Event -SourceIdentifier $_.SourceIdentifier -ErrorAction SilentlyContinue
    }
    foreach ($watcher in $script:watchers) {
        $watcher.EnableRaisingEvents = $false
        $watcher.Dispose()
    }
    if (Test-Path -LiteralPath $PidFile) {
        $current = (Get-Content -LiteralPath $PidFile -Raw -ErrorAction SilentlyContinue)
        if ($current) { $current = $current.Trim() }
        if ($current -eq "$PID") {
            Remove-Item -LiteralPath $PidFile -Force -ErrorAction SilentlyContinue
        }
    }
}
