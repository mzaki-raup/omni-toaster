# Spec — Reminders and scheduling

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../../PROPOSAL.md) §8.9, §8.10
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
user chooses which to enable.

## 4. Triggers

- **At the exact time.**
- **Offset before:** 1 week · 2 days · 1 day · 1 hour · 15 minutes · custom
  (minutes, hours, days). A reminder may carry several offsets, each firing
  separately.
- **Recurrence:** once · daily · weekly · monthly · yearly (anniversaries) ·
  custom pattern.

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
  boundary.
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
`002_locale_keys.sql` belongs to this phase.

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
| Migration | `002` on a fresh and on an existing database |

Then in the running app: create a reminder, let it fire, snooze it, dismiss it,
check the history row.

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

## 14. Open questions

1. Whether a "custom pattern" recurrence needs a real expression language or
   whether the fixed set plus an interval covers every case the owner wants.
2. Whether the work-hour-limit fatigue kind needs to track actual activity, or
   only elapsed wall time since the day's first reminder.

## What is implemented (2026-11-08)

Nothing. Phase 4 has not started; this file is the requirement.
