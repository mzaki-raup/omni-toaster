# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

### Changed

- A prayer toast's bottom line is now the place its times are for — the name and zone code over the coordinates, on two rows — in place of the source label, and the place is no longer repeated under the message; a weather toast puts its name and coordinates on the bottom line the same way (FEAT-0083).
- A little more room between a toast's icon and its title (FEAT-0082).

- (nothing yet)

## 0.20.0 — 2026-11-27

### Added

- Give the weather its own place, searched or typed in its pane (0e138731)

## 0.19.1 — 2026-11-27

### Fixed

- Give the Settings window the app icon instead of eframe default (156014d3)

## 0.19.0 — 2026-11-27

### Added

- Prove the published exe was built for the forge it is on (c1b149db)
- Settings > Updates with Check Now, Download and Install, and Restart (ce4adee9)
- Swap a staged exe in with an update helper, rolling back if it does not take (7689ac68)
- Verify the signed manifest, download and stage the exe (b2509774)
- Where a copy looks for releases, and the check (2cf29790)

## 0.18.0 — 2026-11-27

### Added

- The release check now proves a release was built for the website it is published on: the exe must carry that website's address, and the manifest's links must start with it, or the release job fails. This is what keeps a GitHub download updating from GitHub and a git.zacx.net download from git.zacx.net (FEAT-0081).
- Settings ▸ Updates: the running version, the website this copy updates from, Check for Updates on or off (off sends nothing), an optional once-a-day check that never installs by itself, Check Now, and Download and Install then Restart to Finish for a newer version — in English and Bahasa Melayu, with every failure named (FEAT-0080).
- Updates, third step: a staged update can now be swapped in. A short-lived helper (the running exe started with `--apply-update`) waits for the app to quit, replaces the exe, starts the new one and waits for it to confirm; a new exe that will not start, exits early or reports another version is rolled back and the old one restarted. Nothing in the app starts it yet (FEAT-0079).
- Updates, second step: the app can now fetch a release's signed manifest, verify its signature against the project's key built into the program, download the new `OmniToaster.exe` holding it to the signed size and checksum, and stage it beside the running one — rejecting a tampered manifest, a substituted or oversize download, a replayed old release and a read-only folder. It does not install anything yet, and nothing in the app shows it (FEAT-0078).

- Updates, first step: the app can now ask the forge it came from — GitHub for a copy downloaded from GitHub, git.zacx.net for one from git.zacx.net — whether a newer release exists and whether it can be installed. Nothing in the app shows or installs it yet; that comes in the next steps (FEAT-0077).

- Settings > Reminders as two columns (e1062ddb)

## 0.17.0 — 2026-11-26

### Added

- The weather and prayer toasts name the place (fd67c946)

## 0.16.0 — 2026-11-26

### Added

- Weather and prayer notifications now say where they are for: a weather toast names the place, and a prayer toast — and the reminder ahead of it — names the place with the JAKIM zone beside it when the times are JAKIM's, for example "Kuala Lumpur (WLY01)" (FEAT-0075, FEAT-0076).

- The full Azan is every prayer's default sound (d5e6be5a)

## 0.15.0 — 2026-11-26

### Added

- The current weather and the offline stale marker (5c6462d8)
- Settings > Weather, with a template for each condition (08a40aa5)
- The service - polling, the cache, backoff and a toast on a change (333df6b1)
- Change detection - once, after two readings, never from stale data (c90ad12d)
- The weather cache, emptied when the place moves (f80fa3c6)
- Weather codes grouped into conditions, with Fluent names (71d3418c)
- The Open-Meteo client and strict reader, verified live (5e44f340)

## 0.14.0 — 2026-11-26

### Added

- Weather: Settings ▸ Weather now shows the current weather and how old it is. When the app is offline, or the last answer is old, it says so — "Not up to date: showing the weather from 5 hours ago" — in English and Bahasa Melayu, and no notification is ever produced from such old data (FEAT-0071).

- Settings ▸ Weather: turn weather notifications on, see the place they use, choose how often to check, which kinds of weather to be told about and how big a sudden temperature fall or rise must be, and write your own title, message, accent colour, picture and sound for each condition, with Use Default Text and a preview. Weather data is credited to Open-Meteo on the pane (FEAT-0070).

- Weather: with `enabled = true` under `[weather]` in the settings file (the Weather pane comes next), the app polls Open-Meteo for your saved place, keeps the last answer so it survives a restart and an offline spell, and shows a toast when the weather changes or the temperature falls quickly. It is off by default, and off means no request is sent (FEAT-0069).

- Weather change detection: a change of weather is reported once, only after two separate readings agree, so weather that flickers between two kinds never notifies; a temperature drop (or rise) beyond a set amount within a window is reported once per spell; the first reading after start and anything from the cache never notify. Nothing in the app uses it yet (FEAT-0068).

- Weather cache: the last weather answer is kept for the saved place, with when it was fetched, so it survives a restart and can be shown marked as old when there is no network. Choosing a different place empties it (FEAT-0067).

- Weather conditions: the weather service's numeric codes are grouped into clear, cloudy, rain, thunderstorm, snow and fog, each named in English and Bahasa Melayu; a code the app does not know gets a generic name and a log line. Nothing displays them yet (FEAT-0066).

- Weather groundwork: a client that asks Open-Meteo for the current temperature and weather code and the last few hours, over Windows' own TLS, sending coordinates rounded to two decimals, and reads the answer strictly. It is checked against captured answers for three cities and, on request, the live service; nothing in the app uses it yet (FEAT-0065).

- Jumu'ah, the Friday prayer, in Dhuhr's place (36e02fe6)

## 0.13.0 — 2026-11-26

### Added

- Jumu'ah: the Friday prayer can now have a notification of its own, switched on in Settings > Prayer > Notifications. On Fridays it takes the place of Dhuhr's, at the same time, with a template, a sound and a reminder ahead of it like the other prayers (FEAT-0064).

- An optional reminder so many minutes before each prayer (398b5d52)
- A sound for each prayer, played when it is time (218def3e)
- Audio checking and playback with rodio (90e42879)
- The prayer template editor in Settings > Prayer (8f378b7c)
- A template for each prayer, and the toast built from it (bbc01bdb)
- The prayer service, the prayer toast and Settings > Prayer notifications (f5c20a3d)
- Prayers in the scheduler (2b2aaf0f)
- Settings > Prayer, the pane (d5a6855d)
- The data side of Settings > Prayer (2761bc56)
- The user's location, with consent and manual entry (27bef873)
- The fallback chain and the source label (73af7eeb)
- Calculated prayer times, and a Malaysia preset at 18 degrees (fc3b06dd)
- The prayer-times cache and migration 004 (96913cfc)
- The e-Solat client, over Windows TLS (36f063cf)
- The JAKIM parser, against the fixtures (ead0a90f)
- The JAKIM zone table (17d2edbb)

### Fixed

- Starting the app could report "the data folder is not writable" when two parts of it checked the folder at the same moment, because they shared one test file; each check now uses its own (BUG-0017).

- The single-instance tests take turns (472c3039)
- Checkboxes in Settings > Prayer level with their labels (0cb083c2)
- Prayer pane review fixes and what driving the app found (3737cdd4)

## 0.12.1 — 2026-11-26

### Fixed

- No console window behind a release exe (557be578)

### Security

- Url 2.5.9, which fixes RUSTSEC-2026-0592 (5a4354af)

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

- Internal: the HTTP connection setup and the cancel flag the prayer and weather services each carried are now one shared crate, ready for the updater (CR-0009). Nothing changes for you.

- Settings ▸ Reminders is now two columns: your reminders' titles on the left, each with when it next fires, and the chosen reminder's details and history on the right. New Reminder empties the same form instead of opening another, and a window too narrow for two columns stacks them. A blank form no longer shows an empty-title error before you have typed anything (FEAT-0073).

- Every prayer now plays the full Azan by default — Subuh's own for Fajr, the same Azan for the rest — instead of the short chime. Syuruk, the sunrise, keeps the chime because it is not a call to prayer. Each prayer's sound can still be changed in Settings ▸ Prayer. A prayer whose template had the chime chosen before this change now follows the new default, and you can choose the chime again (CR-0008).

- Update Documentations (291717e0)
- Update License (bc81fbdf)
- Proposal phase (03889588)
- Initial commit (ae85deb2)
