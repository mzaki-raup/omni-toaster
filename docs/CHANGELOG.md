# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- Settings ▸ Reminders makes and edits reminders: a list of them with when each next fires, and an editor for the title and details, the type, the date and time (new shared date and time fields), any number of "remind me before" times and a repeat, which says under each field what is wrong; Delete and Clear History ask first; each reminder shows its recent history. Open on a reminder's toast lands on it there. The list draws only the rows in view, so a long list costs what a short one does (FEAT-0046, and the last box of FEAT-0042).
- Missed reminders: a reminder whose time passed while the PC was off or asleep, or across a clock jump, is no longer skipped or fired late as a toast — one toast says reminders were missed, and Settings ▸ Reminders lists them with Snooze and Dismiss; a recurring one is listed once, at its latest occurrence. Ones older than the new Catch-Up Window (Hours) — 24 by default, 1 to 168 — are dropped with a log line and a missed row. The custom snooze length's unit moved into its label, "Custom Length (Minutes)", so it no longer reads "1 minutes" (FEAT-0045).
- An Alarm's toast stays until you press a button, for up to ten minutes, and one left alone that long is recorded as missed; every other reminder keeps the display duration (FEAT-0044).
- Snooze works: on a reminder's toast it puts the reminder off for the length set in the new Settings ▸ Reminders — 5 minutes, 15 minutes, 1 hour, 1 day or a custom length — and the reminder comes back with its count; after the limit (10 by default, 1 to 50) the toast offers only Dismiss and says "No more snoozes for this reminder." in English and Malay. The count is per fire, so a recurring reminder's next occurrence starts again at zero; a waiting snooze survives a restart; each snooze is recorded with its count and in the history. Snooze stays greyed on the test toast (FEAT-0043).
- A fired reminder is a toast: its own title and body as typed, or its template's, or the built-in text of its type in the active language — the twelve type keys (Event, Task, Anniversary, the four fatigue reminders, Timer, Alarm, Payment, Appointment, Custom) each have a name and a default text in English and Malay. Alarm, Timer, Appointment and Event show their built-in pictures; an Alarm waits ahead of every other queued toast; Open now brings Settings forward; every fire, dismissal and Open is written to the notification history, and a fire during a pause is recorded as missed instead of shown (FEAT-0042).
- The scheduler thread: it keeps every active reminder's next fire time in a queue, fires it within a second of its target (9 and 13 ms late in the app check) and hands it to the UI over a bounded channel that holds a fire rather than dropping it; it rebuilds the queue from the database after an edit, at midnight, after a clock or time-zone change and after a resume, which Windows reports through a new `ot-platform` watcher, and catches a clock jump itself. A one-off reminder that has fired is marked done, so a clock set back does not fire it again. The toast each fire becomes is FEAT-0042 (FEAT-0041).
- When a reminder fires next: once, or every *N* days, weeks (on chosen weekdays), months or years, in the local wall clock so 07:00 stays 07:00 across a DST change; the 31st and 29 February fall on the month's last day; a time a DST gap skips fires when the gap ends, one a fold repeats fires once; and offsets before the time each fire on their own, days counted as calendar days (FEAT-0040).
- The reminder store: migration 003 gives templates their built-in keys and one template per reminder type, records the language a title was typed in, and `ot-store` now creates, reads, lists, updates and deletes reminders, refusing a trigger or recurrence it does not understand, records snoozes with their count, and logs every notification, pruning the log at 90 days on start. The database is now at schema version 3, and `--self-test` checks for exactly that (FEAT-0039).

### Changed

- Dates and times use `jiff` instead of `chrono`, which is unmaintained (RUSTSEC-2026-0543): the toast timestamp, the pause, the log and the crash report read the same as before, and `Asia/Kuala_Lumpur` now resolves from a zone database bundled in the exe, ready for the prayer times. The advisory's ignore is gone from `deny.toml` (CR-0007, ADR-017).

- Phase 4 planned: the reminders spec answers its two open questions (custom recurrence is an interval over the fixed set; the work-hour limit counts wall time), and the locale-keys migration is `003_locale_keys.sql`, JAKIM's `004_jakim.sql`, in the roadmap, the proposal and the CHECKLIST, since Phase 2 took `002` (CR-0007, FEAT-0039 to FEAT-0048).

## 0.11.0 — 2026-11-18

### Added

- Settings Γû╕ Animation (6f2f29a9)
- Reduced motion and the 30 fps fallback (d2779ff2)
- Genie from bottom, and shrink to tray (8eaf19aa)
- Dissolve, glow and alpha to solid, in and out (2b5dbfb4)
- Slide, drawer, pop out and appear, in and out (12950fa8)
- The action row, with Dismiss working (a1725913)
- The toast timeline ΓÇö enter, hold, leave (c8c3b271)
- Glass for Rectangle toasts ΓÇö Blur, Acrylic, Mica-like (b84da8dc)
- Paint toasts from the root's frame, with per-pixel alpha (5e14e963)

### Fixed

- Show toast windows ourselves, not through winit (c93c6a25)
- Turn off Windows' show animation on toast windows (bd96c328)
- Hovering a toast pauses its countdown (4618d912)

## 0.10.0 — 2026-11-15

### Added

- Built-in pictures for prayers, weather and reminders (31f136bd)

## 0.9.1 — 2026-11-15

### Fixed

- The app keeps running with its window closed; double-click opens Settings (2bf65aa5)
- The title bar follows the theme; a killed run leaves no tray icon (568efee8)

## 0.9.0 — 2026-11-14

### Added

- Display Duration, 3 to 60 seconds; record the owner's checks (2ec69bc5)
- Shadow is On or Off, and it is Windows' own shadow (5456a8f6)
- Opacity fades the whole toast, from 70% (878262cd)
- See-through corners, the accent bar on the edge side, a monitor choice (91e966bd)
- Stacking controls, a steadier position map, legible toast text (0f4c5351)
- The image library - upload once, name, use anywhere (0972d9d5)
- Icon, image and accent belong to each notification (31bdf2e4)
- Settings > Appearance, with a preview that is the real toast (48eacb4e)
- Import an image, the way the security spec fixes it (8cbaec70)
- Place toasts - nine presets, a custom point, stacking (63128383)
- The header, the body and the ellipsis rules (ad30b1c7)
- Paint the toast - shapes, gradient, opacity, border, shadow (7986c803)
- The toast viewport, and the Win32 call it turned out to need (4e7926f1)
- ToastSpec, with its limits enforced in the model (9dac44a9)

## 0.8.1 — 2026-11-10

### Fixed

- Wait for the log line instead of looking once (651cb3d7)

## 0.8.0 — 2026-11-09

### Added

- Switch the language while the app runs (159ac6c2)

### Security

- Bundle SQLite 3.54.1, which fixes CVE-2026-119375 (038d2e96)

## 0.7.0 — 2026-11-09

### Added

- The tray icon, its menu and pausing notifications (f8878dc0)

## 0.6.0 — 2026-11-09

### Added

- The settings window and its General pane (09ae6f9f)

## 0.5.0 — 2026-11-09

### Added

- The three themes and Windows theme detection (b97dad32)
- Single instance and Start with Windows (a7f0af2f)

### Fixed

- Take ot-platform by path alone, so a minor bump resolves (da99e173)
- A missing Run key means autostart is off, not an error (211d5fb5)

## 0.4.1 — 2026-11-08

### Fixed

- Make the mirror step say what it did (41c82fd6)

## 0.4.0 — 2026-11-08

### Added

- Add the app icon and embed it in the executable (fa575538)

## 0.3.0 — 2026-11-08

### Added

- Implement --self-test and the real command line (56ca4373)

## 0.2.0 — 2026-11-08

### Added

- Open the database and run migration 001 (3dd251ec)

## 0.1.0 — 2026-11-08

### Added

- Make the app bilingual, with locale detection (50b9d6e9)
- Log to a rotating file and capture panics (25d36d39)
- Resolve the portable data root and prove it is writable (9ce84e14)
- Commit the minisign public key (cd925f7a)
- Create the Cargo workspace and its eleven crates (3a89f8b3)

### Fixed

- Stop two tests racing on the global Fluent loader (0a84278a)

### Changed

- Update Documentations (291717e0)
- Update License (bc81fbdf)
- Proposal phase (03889588)
- Initial commit (ae85deb2)
