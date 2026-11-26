# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- The JAKIM zone table: all 60 zones, with their states and areas exactly as JAKIM's e-Solat portal lists them (read 2026-11-20) and in English, and 208 places from GeoNames (MY extract of 2026-11-20, CC BY 4.0) that locate each zone's areas; the nearest area gives a location's zone, and a location outside Malaysia is offered none. Sources and the few places that could not be located are in `data/README.md` (FEAT-0050).

- The JAKIM e-Solat parser: it reads the shape the spike verified — Malay and English month names, `HH:MM:SS` and `HH:MM` times taken as Malaysian time whatever the PC's zone — treats "no record" as no data rather than a failure, refuses an oversized body or an HTML error page, and sets aside whole any day with a missing, malformed or out-of-order time, tested against the three zones' captures and a hand-built sample of broken days (FEAT-0051).

- The e-Solat client: one blocking attempt on a worker thread over Windows' own TLS (ADR-019), HTTPS only, ten-second timeout, `User-Agent: OmniToaster/<version>`, one POST for today to the end of next month, the body capped as it is read, and a cancel that stops the attempt before it sends and drops its result before it is delivered. When to look again is pure code the scheduler will call — weekly, daily while the answer stops short at the year's end, and 1, 5 and 15 minutes after a failure. Tested against a local server, and live for SGR01, WLY01 and JHR01 (FEAT-0052).

- The prayer-times cache: migration 004 adds `jakim_prayer_cache`, and the store writes a fetched range in one transaction, replacing a day that was corrected, reads a zone's day with its source and when it was fetched, and says whether the cache can serve today — it has the day and wrote it under 30 days ago. Tested on a fresh database, on a version 3 database as v0.12.0 left it, round trip, at the 30-day edge and with a damaged row (FEAT-0053).

- Calculated prayer times: Muslim World League, ISNA, Umm al-Qura and "Malaysia (JAKIM angles, approximate)", with Asar Shafi'i (the default) or Hanafi, from the `adhan` crate (ADR-020). Each time is an instant, and one the sun does not reach — inside the polar circles — is absent rather than invented. The Malaysia preset is within three minutes of JAKIM's published times in SGR01, WLY01 and JHR01; the other three match the adhan-js reference on 60 rows, including places east and west of Greenwich (FEAT-0054).

- The prayer fallback chain and the source label: a fresh cache, then a live fetch, then a cached day past its age, then the offline data, then a calculation, then nothing — in one function, tested with the network down, an HTTP 500, bad JSON, an oversized answer, "no record", an empty cache and the offline data present and absent, each ending at its expected label. The label reads "Source: JAKIM e-Solat" with when it was fetched, "Offline data", or "Calculated", with "(fallback)" when JAKIM could not be had and "(approximate)" in Malaysia, in English and Malay (FEAT-0055).

- The user's location: an IP lookup that asks first — nothing is sent until the user agrees, the answer is looked up again at most once a day, and the IP address in it is dropped — plus city search and typed coordinates, every place rounded to about a kilometre, and the saved place kept in the database so one chosen by hand is never overwritten by a detection. The services are ipapi.co and Open-Meteo's geocoding (ADR-021) (FEAT-0056).

- Prayer names: Settings and the notifications name each prayer in the active language with the other language's name in brackets — "Subuh (Fajr)", "Zohor (Dhuhr)", "Asar (Asr)", "Isyak (Isha)", "Syuruk (Sunrise)" — and a test checks every label against the reference table in the i18n document, in both languages (FEAT-0062).

- Pre-prayer reminders: each prayer can also give a short reminder ahead of it — "Asr (Asar) is in 10 minutes." — a number of minutes you choose (10 by default, up to 120), set in Settings > Prayer > Notifications. The reminder moves with the prayer's time, is skipped if the PC was asleep through it, and is held back by a pause, as any ordinary reminder is (FEAT-0061).

- Prayer sounds: each prayer can play the short chime (the default), the full Azan that ships with the app (Subuh's own for Fajr), an audio clip of your own, or nothing. A chosen WAV, MP3 or Ogg clip is checked first — at most 20 MB and ten minutes, and what is inside must really be that kind of audio, not just named like it — and kept inside the data folder. The sound starts with the notification and stops when it is dismissed, and Play Sound lets you hear it in Settings (FEAT-0060).

- Prayer templates: each prayer's notification can have a title, message, accent colour and picture of its own, edited in Settings ▸ Prayer ▸ Prayer Templates with a Preview Toast button and a Use Default Text button that brings back the built-in text in the language in force. What you write is kept as typed and never translated; the built-in picture is the masjid of that prayer. A prayer's notification carries the source of the day's times (FEAT-0059).

- Prayer notifications: a background service keeps the day's times current while Settings is closed — it fetches from e-Solat weekly (daily while the year's end leaves a gap), when the zone changes and when the saved times lack today, tries again after 1, 5 and 15 minutes when that fails, looks the place up again once a day if you agreed, and tells you once a day if no times can be had. A prayer's notification stays until dismissed, up to ten minutes, comes ahead of everything else, and is not held back by a pause. Settings ▸ Prayer gains Notifications: Sunrise (Syuruk) can have one of its own, and the window in which a prayer missed while the PC was off is still listed can be changed (FEAT-0058).

- The scheduler's side of the prayers: each enabled prayer is a managed reminder that fires at the times the app gives it, as instants, so the PC's own time zone never matters; it is never marked done, only waits for the next day's times; two fires due together come in the order Azan, Alarm, the rest; and a prayer missed while the PC was off is listed if it is within an hour, and dropped with a `missed` row if not. The one-hour window can be changed. These prayers do not show in the reminders list (FEAT-0058).

- Settings ▸ Prayer: a new section of the settings window, in English and Malay. It asks before looking up the user's place from their internet address, and offers city search and typed coordinates instead; shows the saved place and the JAKIM zone found for it, with a searchable list of all 60 zones to choose another; chooses the source (Auto, JAKIM e-Solat or Calculated), the calculation method and the Asar school; and lists today's six prayers, each with both names, beside the source label and when it was fetched. The label and the times follow a change of language or clock at once (FEAT-0057).

- Settings ▸ Prayer's data side: a `[prayer]` section in `settings.toml` (source, zone, method, Asar school — a value nobody recognises falls back to the default), a search over the 60 zones by code, state or area in either language, the prayer cache behind the fallback chain, and the work — a store read, an IP lookup, a city search, a day's times — each on a thread of its own, so a slow e-Solat fetch never holds a search behind it, and a reply that a newer request has replaced is dropped (FEAT-0057).

### Changed

- The Malaysia preset's Fajr angle is 18.0°, not the 20.0° the spec gave: JAKIM's own November tables put 20.0° ten minutes early, and the data fit about 17.5°. The spec, `PROPOSAL.md` and both READMEs say 18.0° (FEAT-0054).

- `deny.toml`'s reason for banning `openssl-sys` no longer says "use rustls": the prayer fetch uses Windows' SChannel through `native-tls` (ADR-019), and the ban on OpenSSL stands (FEAT-0052).

- Phase 5 begun with its spike: the JAKIM e-Solat endpoint verified for SGR01, WLY01 and JHR01 and recorded as ADR-018 — times are `HH:MM:SS`, dates use Malay month abbreviations, "no data" comes inside an HTTP 200, next month is a POST range, and there is no data for a year JAKIM has not published. The prayer spec now describes the verified shape, and Asar is confirmed Shafi'i. The phase is filed as FEAT-0049 to FEAT-0063 (FEAT-0049).

## 0.12.0 — 2026-11-20

### Added

- Fatigue reminders (a61529e1)
- The shared date formatter, for reminders (43462338)
- Settings > Reminders, the list and the editor (b7a69c81)
- Missed reminders (da69487b)
- Persistent alarms (53bd1f0e)
- Snooze, with its limit (34c6b20f)
- Reminder types and their toasts (b803cdc8)
- The scheduler thread (ce3d3e8f)
- Recurrence and offsets in the local wall clock (9dc9fd89)
- Migration 003 and the reminder store (9b78869d)

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
