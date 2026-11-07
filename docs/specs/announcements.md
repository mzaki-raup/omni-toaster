# Spec — Time announcements

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../../PROPOSAL.md) §6.5, §8.11
**Owner crates:** `ot-media`, `ot-platform`, `ot-sched` · **Phase:** 7

## 1. Purpose

Speak the time aloud on a schedule, in the active language. Also the home of
quiet hours, because quiet hours are what keep a spoken feature from becoming
an enemy.

## 2. User stories

- As **Siti**, I want the hour announced in Malay, so I don't have to look at a
  clock.
- As **anyone**, I want nothing spoken between 22:00 and 06:00.
- As **anyone with no Malay voice installed**, I want to be told why the
  announcement came out in English, rather than wondering.

## 3. Schedule

Every hour · every 30 minutes · every 15 minutes · start of day · end of day ·
custom times. Entries go into the scheduler's queue like anything else
([specs/reminders.md](reminders.md)), so they inherit DST and resume handling
for free.

## 4. What is spoken

Generated from a Fluent template, never from a formatted string built in code:

```ftl
announce-time = Sekarang pukul { $hour } { $minute }.
announce-time-with-date = Sekarang pukul { $hour } { $minute }, { $date }.
```

12-hour or 24-hour by the user's setting, with an optional date. The values
come from `ot_i18n::format_datetime`, which is the only function in the app
that formats a time ([10-i18n.md](../10-i18n.md)).

## 5. Voice selection

Through the Windows `SpeechSynthesizer`, reached via `ot_platform::speech`.
Voices are discovered from `SpeechSynthesizer.AllVoices` — **never by
hard-coded name**.

In order:

1. A voice whose language tag matches the active language (`ms-MY` or `en-US`).
2. Otherwise, any voice in the other supported language, **with a visible
   notice** that the announcement is in the wrong language.
3. Otherwise, play the chime only and show a setup hint naming the Windows path
   (Settings ▸ Time & Language ▸ Speech).

**No fallback is silent.** Each one is logged and visible in the settings status
line. A Malay voice being absent is common, so step 2 and step 3 are normal
paths, not error paths.

## 6. Quiet hours

A start and an end time — 22:00 to 06:00, say — crossing midnight correctly.
During quiet hours:

| Suppressed | Not suppressed |
|---|---|
| Time announcements | **Azan** |
| Non-urgent toasts | Alarm |

Azan always has priority and is never suppressed
([11-prayer-and-azan.md](../11-prayer-and-azan.md) §9). A suppressed
announcement is dropped, not queued — an hour announced an hour late is wrong.

## 7. Audio

`rodio` for playback. One output stream owned by `ot-media`, so an announcement
and an Azan clip cannot fight over the device. Speech and audio both run off
the UI thread.

When Azan and an announcement collide, the announcement is dropped and logged;
Azan is never interrupted or mixed.

## 8. Settings

Settings ▸ Announcements: the schedule, the 12/24-hour choice, whether to
include the date, the voice (with the detected voices listed and the fallback
state shown), a **Test** button that speaks immediately, and the chime fallback
choice. Settings ▸ Notifications holds the quiet-hours range, since it governs
toasts too.

## 9. Fluent keys

Prefix `announce-`: the spoken templates, the schedule labels, the voice-state
messages (matched · wrong-language notice · no voice), the setup hint, the test
button. Quiet-hours labels live under `settings-`.

## 10. Verification

| Check | Asserts |
|---|---|
| Voice selection order | matching tag → other language with notice → no voice → chime; driven by a fake voice list, not by the installed set |
| Templates | both languages render with correct hour and minute, 12- and 24-hour |
| Quiet hours | a range crossing midnight suppresses an announcement inside it and not outside it |
| Azan priority | an announcement colliding with Azan is dropped and logged |
| Schedule | every-15, every-30, hourly and custom entries land in the queue at the right times |

Then in the running app, which is the only place this can really be judged:
announcements in Malay **and** in English, with a Malay voice installed and
with none, and quiet hours observed (Phase 7's exit criteria).

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

## 11. Open questions

1. Whether "end of day" should speak a summary (what fired, what is tomorrow)
   or only the time.
2. Whether the wrong-language notice belongs in a toast or only in the settings
   status line — a toast per hour would be worse than the problem.

## What is implemented (2026-11-08)

Nothing. Phase 7 has not started; this file is the requirement.
