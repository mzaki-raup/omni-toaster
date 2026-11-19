# Spec — Reminders and scheduling

**Status:** approved · **Last updated:** 2026-11-19 · **Source:** [PROPOSAL.md](../../PROPOSAL.md) §8.9, §8.10
**Owner crates:** `ot-sched`, `ot-store`, `ot-ui` · **Phase:** 4

## 1. Purpose

The scheduler and the reminders it fires. The scheduler is **the single source
of time** for the whole app: prayer times, weather checks and announcements all
become entries in its queue. Nothing else decides when something happens.

## 2. User stories

- As **Mei**, I want a reminder a week before, a day before and on the day, so
  an anniversary never arrives unannounced.
- As **Mei**, I want a missed reminder to be waiting for me when I turn the PC
  back on, not silently gone.
- As **anyone**, I want snooze to mean snooze — and to stop nagging me after a
  sensible number of tries.

## 3. Types

Azan Prayer · Weather change · Event · Task · Anniversary · Fatigue (eye break,
posture, hydration, work-hour limit) · Timer · Alarm · Payment · Appointment ·
Custom.

Each type has its own template in both languages
([specs/toast.md](toast.md) §8). The type key is a stable identifier and never
changes with the UI language
([05-data-model-and-storage.md](../05-data-model-and-storage.md)).

Fatigue covers all four sub-kinds, each with its own default schedule, and the
user chooses which to enable. The work-hour limit counts **elapsed wall time**
from the first fatigue reminder of the day, not measured activity (§14,
question 2).

## 4. Triggers

- **At the exact time.**
- **Offset before:** 1 week · 2 days · 1 day · 1 hour · 15 minutes · custom
  (minutes, hours, days). A reminder may carry several offsets, each firing
  separately.
- **Recurrence:** once · daily · weekly · monthly · yearly (anniversaries) ·
  custom pattern. **Custom** is an interval over the fixed set — every *N*
  days, weeks, months or years — and weekly may name several weekdays; there is
  no expression language (§14, question 1). A monthly or yearly date the month
  does not have (the 31st, 29 February) falls on the month's last day.

A reminder with offsets fires at each offset **and** at its own time: "a week
before, a day before and on the day" is two offsets. Offsets in minutes and
hours are elapsed time; offsets in days and weeks are calendar days, so "1 day
before 07:00" is 07:00 the day before even when a DST change made that day 23
or 25 hours long. Two fires that land on the same instant — a daily reminder
whose 1-day offset meets the next day's own fire — are one fire, not two toasts
at once. An interval of every 0 never fires.

`trigger_json` and `recurrence_json` hold the detail; `next_fire_at` is the one
denormalized field the scheduler reads.

## 5. Snooze

5 minutes · 15 minutes · 1 hour · 1 day · custom. **Default maximum 10 snoozes
per reminder**, after which the toast offers only Dismiss and says so. Each
snooze is recorded in `snooze_log` with its count.

## 6. Missed reminders

If the PC was off or asleep, missed items are listed at next start with dismiss
and snooze options, rather than firing all at once.

| Catch-up window | For |
|---|---|
| 1 hour | Azan |
| 24 hours | everything else |

Both configurable. Anything older than its window is dropped with a log line,
not shown — a reminder from three weeks ago is noise.

## 7. Time correctness

This is where this feature earns its keep, and where it is easiest to get
wrong.

- The scheduler stores each reminder's **next absolute fire time** and
  recomputes it after: a DST transition, a system resume, a clock change, a time
  zone change, midnight, and any edit to the reminder.
- It **never relies on a long sleep.** It waits on a bounded timeout and
  re-examines the queue, so a suspended machine cannot carry a stale deadline.
- A recurring reminder's next occurrence is computed in the **local wall clock**,
  not by adding 24 hours — "every day at 07:00" stays 07:00 across a DST
  boundary. A wall-clock time a DST gap skips fires when the gap ends (02:30 on
  a day that jumps from 02:00 to 03:00 fires at 03:00); one a DST fold repeats
  fires once, on its first pass.
- A clock jumped forward, or a resume, fires each reminder it skipped **once**,
  not once per skipped occurrence; listing them instead is §6. A clock set back
  does not fire a one-off reminder again: one that has fired its last is
  marked `done`. A recurring one follows the wall clock and fires when its time
  comes round again.
- JAKIM prayer times are read in `Asia/Kuala_Lumpur` and converted to absolute
  timestamps when the queue is built, regardless of the PC's own zone
  ([11-prayer-and-azan.md](../11-prayer-and-azan.md) §4).
- A reminder fires **within one second** of its target (Phase 4's exit
  criterion).

## 8. Priority

When two things want the screen at once:

```
Azan  >  Alarm  >  everything else  >  time announcement
```

Quiet hours suppress announcements and non-urgent toasts; **Azan always has
priority** and is never suppressed
([specs/announcements.md](announcements.md)).

## 9. Data

`reminder`, `snooze_log`, `notification_log`, `category` — see
[05-data-model-and-storage.md](../05-data-model-and-storage.md). Migration
`003_locale_keys.sql` belongs to this phase — it was planned as `002`, which
Phase 2's `002_asset_label.sql` took.

`notification_log` records every fire with its action and source, which is what
makes the history pane and the "did it actually fire?" question answerable. It
is pruned at 90 days.

## 10. UI

Settings ▸ Reminders: the list, the editor, categories, lead offsets and snooze
defaults ([04-ui-design.md](../04-ui-design.md)). Deleting a reminder and
clearing history are both guarded. The list has an empty state; the editor
validates and says what is wrong in the user's language.

## 11. Fluent keys

Prefix `reminder-`: type names, offset labels, recurrence labels, snooze
labels, validation messages, the missed-reminders list, the empty state. Every
date and time in them goes through `ot_i18n::format_datetime`, never a manual
format string.

## 12. Threading

The scheduler runs on its own thread and owns the queue. It sends fire events
to the UI through a bounded channel drained at frame start, and receives edits
back the same way. Nothing blocks the UI thread; nothing calls `block_on`
([ADR-002](../08-decisions.md)).

## 13. Verification

| Check | Asserts |
|---|---|
| Recurrence math | daily, weekly, monthly, yearly across month ends and leap years |
| DST | "every day at 07:00" stays 07:00 across both transitions, in both directions |
| Offsets | several offsets on one reminder each fire once |
| Snooze | the count limit is enforced and recorded |
| Resume | a queue built before a simulated resume is recomputed, not trusted |
| Missed | items inside the window are listed, items outside are dropped and logged |
| Timing | a reminder fires within one second of target |
| Migration | `003` on a fresh and on an existing database |

Then in the running app: create a reminder, let it fire, snooze it, dismiss it,
check the history row.

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

## 14. Open questions

1. ~~Whether a "custom pattern" recurrence needs a real expression language or
   whether the fixed set plus an interval covers every case the owner wants.~~
   **Answered 2026-11-19, when Phase 4 started on the owner's "just proceed":**
   the fixed set plus an interval and chosen weekdays (§4). Reopened if the
   owner wants more.
2. ~~Whether the work-hour-limit fatigue kind needs to track actual activity, or
   only elapsed wall time since the day's first reminder.~~ **Answered the same
   way:** elapsed wall time (§3). Tracking activity would need input hooks the
   app has no other use for.

## What is implemented (2026-11-19)

Phase 4 is under way on `feat/phase-4-scheduler`; its items are CR-0007 and
FEAT-0039 to FEAT-0048 in `docs/TASKS.md` under `v0.4.0`.

- **The store (FEAT-0039).** Migration `003_locale_keys.sql` adds the
  `category` key columns (`prayer_key`, `weather_key`, `type_key`, one
  template per key), `authored_lang` on `category` and `reminder` (`en-US` or
  `ms-MY` only), and seeds a template with no custom text for each built-in
  reminder type: `reminder-event`, `-task`, `-anniversary`, `-fatigue-eye`,
  `-fatigue-posture`, `-fatigue-hydration`, `-fatigue-work-hours`, `-timer`,
  `-alarm`, `-payment`, `-appointment`, `-custom`. Azan and weather changes are
  seeded with their own keys by Phases 5 and 6.
- `ot_store::reminders` creates, reads, lists, updates and deletes reminders
  with parameterized statements. `trigger_json` and `recurrence_json` are typed
  (`Trigger`, `Recurrence`) and a field this build does not know is refused; a
  row it cannot read is an error from `reminder()` and is left out of
  `reminders()` with a log line. An edit clears `next_fire_at`, which only the
  scheduler writes (`set_next_fire_at`).
- Snoozes are recorded with their running count; `notification_log` rows are
  written with their action and pruned at 90 days on every start. Instants are
  stored as UTC text (`2026-11-19T00:00:00.000Z`) so they compare as text.

- **Recurrence and offsets (FEAT-0040).** `ot_sched::recurrence::next_fire`
  gives the first fire strictly after an instant, in a given zone: once, every
  *N* days, weeks (on chosen weekdays), months or years, counted from the
  reminder's own date; the 31st and 29 February clamped to the month's end and
  back again; gaps and folds as §7 says; offsets as §4 says. A pure function,
  tested in `America/New_York` for DST, since Malaysia has none.

- **The scheduler thread (FEAT-0041).** `ot_sched::scheduler` runs the thread
  `scheduler`, which owns the store's connection it was given and a queue of
  each active reminder's next fire, written to `next_fire_at`. It waits on its
  command channel for at most 30 seconds and never past local midnight,
  re-reading the wall clock on every wake. `Command::Changed`, `Removed` and
  `Recompute` come in on that channel; the queue is rebuilt from the database
  at midnight, on a clock jump it measures itself (the wall clock against the
  monotonic one, beyond 2 seconds), on a zone that differs from the last wake's,
  and on `ot_platform::system_events`' clock change and resume. Fires go out on
  a channel of 64; a full one holds them and retries every 100 ms. The UI
  drains it at frame start and, until FEAT-0042 makes them toasts, logs each.
  The log line `reminder fired` gives the target and `late_ms`.
