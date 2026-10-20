# CK3 voice advisor

A folder you run with Grok voice while a Crusader Kings III campaign is paused. It answers the decision in front of you. It remembers the save. It does not invent a rule.

Vanilla PC, patch 1.20, every DLC through By God Alone. Silk & Silver is not covered. Mods and console are not covered.

The pages were checked against the CK3 wiki on 2026-10-19. The wiki marks those law pages as verified for 1.20. Patch 1.20.1 (13 Oct 2026) is a later small patch. Its notes do not change succession, factions, claims, schemes, domain, or raiding. If your version string is newer than 1.20 and a button disagrees with a page, the button wins. Say so, and the page gets a correction.

## Start

1. Install Grok and log in.
2. Open a terminal in this folder. `ck3-guide`, not the repo root.
3. Run `.\Start-CK3.ps1`. If PowerShell blocks the script, run `grok` from this folder instead. On macOS or Linux, run `grok` from this folder.
4. Trust the folder if Grok asks. That is what loads the rules.
5. Press Ctrl+Space, or type `/voice`.

The first time, say who you are playing. One sentence is enough: ruler, title, and the succession law on the Realm screen. Grok writes that into `campaign.md`. A new session still reads the file.

## How an answer should sound

The move, then what the two or three real choices change. Then it stops. Ask "why" if you want the mechanic.

If the pages do not cover it, it should say so and name the screen to look at. Faith reform, culture, activities, landless play, administrative realms, nomads, theocracies, and Silk & Silver are not written.

## When an answer is wrong

Say that it was wrong, and say what happened in the game. Grok logs it in `CORRECTIONS.md` and rewrites that page. Allow the edit. If the new line was not checked against the wiki, the page marks it `reported from campaign, unverified`, and the next answer has to say so.

Allow writes to `campaign.md` when you state a fact about the save. Do not allow a guide edit unless you are correcting one.

## One campaign

`campaign.md` is the current save. Say "new campaign" when you start another. Grok asks you to confirm, then clears the file.

## What is in here

- `AGENTS.md` tells Grok the hard stops.
- `.grok/skills/ck3-advisor/SKILL.md` is the procedure.
- `index.md` is the list of decisions. One question opens one page.
- `guide/` holds the pages.
- `Start-CK3.ps1` starts Grok in this folder.
