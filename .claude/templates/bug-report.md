# Template — bug report

Hand this to `/bug`, or to `/task-new bug` to file it for a later version
(save it anywhere outside the repo, or paste it). Either way it becomes a
`BUG-dddd` item in [docs/TASKS.md](../../docs/TASKS.md) with its own checklist.
The fields that matter most are the first three: without them the fix is
guesswork.

```markdown
## Symptom

What you saw, in the words the app used. Paste the error text, the toast, the
log line (`OmniToasterData/logs/app-YYYY-MM-DD.log`, or the crash report under
`logs/crash/`) or the screenshot.

## Steps

1. Started with `cargo run -p omnitoaster -- --data-dir ./tmp-data` (or the
   packaged exe — `OmniToaster.exe --version`).
2. …
3. …

## Expected / actual

Expected: …
Actual: …

## Where

Notification type: Azan prayer · weather · event · task · anniversary ·
fatigue · timer · alarm · payment · appointment · custom · time announcement ·
none (the settings window itself)
Settings pane, if the problem is in the window: General · Toasts · Reminders ·
Prayer · Weather · Announcements · About
Theme: Dark · Light · System (and which way Windows was set)
Language: English · Bahasa Melayu (and whether it was switched at runtime)
Toast shape and animation, for a rendering problem: …

## Environment

- OmniToaster version (`--version` or the About pane):
- Windows version and build (`winver`):
- DPI scaling: 100% · 125% · 150% · 175% · 200% · 250%
- Monitors: how many, and which one the toast was supposed to appear on
- Prayer source, for a prayer bug: JAKIM (which zone code) · Calculated
  (which method) · bundled · fallback — and what the source label said
- Reproducible from a clean `--data-dir`? yes / no / didn't try

## Anything already known

Does `docs/GOTCHAS.md` mention it? Did it work in an earlier version — which?
```

## What the fix will do with this

`/bug` turns it into a `BUG` item, then a failing test, then the smallest fix,
then the gate and the running-app check, then a `docs/GOTCHAS.md` entry and a
`docs/CHANGELOG.md` `fix:` line, ticking the item's boxes as each actually
passes. A report that cannot be reproduced gets said so rather than guessed at.
