# CK3 voice advisor

A folder you run with Grok voice while a Crusader Kings III campaign is paused. It answers the decision in front of you. It remembers the save. It does not invent a rule.

Vanilla PC, patch 1.20, every DLC through By God Alone. Silk & Silver is not covered. Mods and console are not covered.

The pages were checked against the CK3 wiki on 2026-10-19. The wiki marks those law pages as verified for 1.20. Patch 1.20.1 (13 Oct 2026) is a later small patch. Its notes do not change succession, factions, claims, schemes, domain, or raiding. If your version string is newer than 1.20 and a button disagrees with a page, the button wins. Say so, and the page gets a correction.

## For the person playing

Open `Read-Me-First.txt` in this folder. That file is the click-by-click steps. Double-click `Start CK3 Guide.bat` to play. Change what the watcher reads in `watcher-settings.txt`.

The rest of this file is for the person who maintains the folder.

## Start

1. Install Grok and log in.
2. Install Rust once, from https://rustup.rs. The watcher builds a small reader the first time it runs. The default Windows toolchain needs the Visual Studio C++ build tools so `link.exe` exists. If the build says the linker is missing, run `rustup toolchain install stable-x86_64-pc-windows-gnu` and `rustup default stable-x86_64-pc-windows-gnu`.
3. One time, dump the ironman token names. See below. Without that file, an ironman autosave stays header-only.
4. Open a terminal in this folder. `ck3-guide`, not the repo root.
5. Run `.\Start-CK3.ps1`. That starts the save watcher, then Grok. The watcher keeps running after Grok closes. If PowerShell blocks the script, run `powershell -ExecutionPolicy Bypass -File .\Start-CK3.ps1`.
6. Trust the folder if Grok asks. That is what loads the rules.
7. Press Ctrl+Space, or type `/voice`.

The watcher writes `state\realm.md` from the newest `autosave.ck3`. A manual save counts when it is newer than that autosave. If the game does not use the name `autosave.ck3`, the watcher takes the newest `.ck3` whose header says ironman and logs the file name. `state\watch.log` says which file was read.

Until that file exists, say who you are playing. One sentence is enough: ruler, title, and the succession law on the Realm screen.

## How an answer should sound

The move, then what the two or three real choices change. Then it stops. Ask "why" if you want the mechanic.

If the pages do not cover it, it should say so and name the screen to look at. Faith reform, culture, activities, landless play, administrative realms, nomads, theocracies, and Silk & Silver are not written.

## When an answer is wrong

Say that it was wrong, and say what happened in the game. Grok logs it in `CORRECTIONS.md` and rewrites that page. Allow the edit. If the new line was not checked against the wiki, the page marks it `reported from campaign, unverified`, and the next answer has to say so.

Allow writes to `state\spoken.md` when you state a fact about the save. Do not allow a guide edit unless you are correcting one.

## One campaign

`state\realm.md` is the current realm, once the watcher has read an autosave. `campaign.md` stays a stub. Say "new campaign" when you start another. Grok asks you to confirm, then clears `state\realm.md` and `state\spoken.md`. The next autosave fills the realm again.

## Ironman tokens, once

The reader does not open `ck3.exe` and this folder does not contain a token list. Paradox does not ship that list. On the PC where the game is installed:

1. Start CK3 with `-debug_mode`.
2. Open the console and run `oos_dump`.
3. The game writes `Documents\Paradox Interactive\Crusader Kings III\oos\dummy\tokens_1.tok`.

The watcher copies that file to `%LOCALAPPDATA%\ck3-guide\tokens.txt` when the cache is empty. You can also set `CK3_IRONMAN_TOKENS` to the path of the file. Console use here is only this one dump. It is not part of an answer.

Binary numbers use the CK3 1.5+ scale. If a number disagrees with the screen, the screen wins. The first real 1.20 ironman read is the line in `state\watch.log` that says the autosave was installed.

## What is in here

- `AGENTS.md` tells Grok the hard stops.
- `.grok/skills/ck3-advisor/SKILL.md` is the procedure.
- `index.md` is the list of decisions. One question opens one page.
- `guide/` holds the pages.
- `Read-Me-First.txt` is the click-by-click sheet for the person playing.
- `watcher-settings.txt` is the two lines he can change: which folder, and which save name.
- `Start CK3 Guide.bat` starts the watcher, then Grok.
- `Stop the watcher.bat` stops the watcher.
- `Start-CK3.ps1` is what the start bat runs.
- `Start-Watcher.ps1` starts only the watcher. `Stop-Watcher.ps1` stops it.
- `watcher\` is the save reader. `state\` is the local snapshot and is not committed.
