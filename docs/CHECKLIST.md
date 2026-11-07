# OmniToaster — implementation checklist

States: `Completed` · `In-Progress` · `Pending`. Update a line only after the work is verified (checks pass and the feature was exercised in the running app), then commit and push.

The rows come from the Key Deliverables of each phase in `PROPOSAL.md` §13; the
exit criteria under each heading are that phase's own, and no row in a phase may
read `Completed` until they all hold. The work itself, by id, is in
[TASKS.md](TASKS.md).

Last updated: 2026-11-08

## Planning
> **Done when:** the scaffolding and the specifications describe Omni Toaster and point only at files that exist; no reference to the HyperStack project remains.

| Item | State | Notes |
|---|---|---|
| Proposal reviewed and stakeholder questions answered | Completed | `PROPOSAL.md` §16; version 1.0, 2026-11-08 |
| Proposal structure repaired (section numbering, dangling references) | Completed | Duplicate §12–§14 renumbered to §15–§17; §15, §17, §18, §9.7 and one §10 reference resolved |
| Agent scaffolding realigned off HyperStack | Completed | `.claude/` — 33 files: 11 rewritten, 20 edited, `async-and-jobs.md` → `threads-and-scheduling.md` and `spec-tool.md` → `spec-area.md`, new `rules/i18n.md`. The phantom skills table is gone; the commands are the mechanism |
| Root documents and work ledger realigned | Completed | `CLAUDE.md`, `README.md`, `docs/TASKS.md`, `docs/CHECKLIST.md`, `docs/GOTCHAS.md`, `docs/CHANGELOG.md` |
| Numbered specifications written (`docs/01`–`docs/11`) | Completed | Includes the ADR log with ADR-001–012 |
| Per-area specifications written (`docs/specs/`) | Completed | `toast`, `reminders`, `weather`, `announcements`, `tray-and-settings`, `supervisor`, `updater` |
| CI workflows and scripts renamed to the Omni Toaster asset contract | Completed | `.forgejo/`, `.github/`, `scripts/`; all YAML and all six scripts parse. Not executed — the workflows can only run on a push |
| Screenshot procedure written | Completed | `docs/screenshots/README.md`; no images yet, no code to photograph |
| Every cross-reference resolves | Completed | Repo-wide link check over every `*.md`: zero broken relative links. Marker grep clean apart from prose that deliberately names what was removed |

## Phase 0 — Foundation
> **Exit criteria:** the app starts in English and in Malay, writes a log, the locale key parity check passes, and CI is green.

| Item | State | Notes |
|---|---|---|
| Repository and `docs/` skeleton | Completed | `docs/` complete; the Cargo workspace and all eleven crates build — FEAT-0001 |
| `CLAUDE.md` | Completed | Realigned, and `.claude/rules/` has landed — every link resolves |
| Cargo workspace skeleton | Completed | FEAT-0001. 11 members, `cargo fmt`/`clippy -D warnings`/`test --locked -j 4`/`deny check` all green; `OmniToaster.exe` builds at 223 KB and prints its version; no dynamic CRT imports, so `+crt-static` is confirmed |
| CI skeleton | In-Progress | FEAT-0007. The workflows exist and `release.yml` goes green on `main`, but `ci.yml` does not trigger on a branch push — BUG-0001 |
| Logging | Pending | FEAT-0003 |
| Panic hook | Pending | FEAT-0003 |
| Portable path resolution with writability check | Pending | FEAT-0002 |
| Fluent setup with both `.ftl` files | Pending | FEAT-0004 |
| `fl!()` wired in | Pending | FEAT-0004 |
| Locale detection | Pending | FEAT-0004 |
| Migration 001 | Pending | FEAT-0005 |

## Phase 1 — Shell
> **Exit criteria:** the tray works, the theme switches, autostart survives a reboot, and a language switch updates the tray and settings in under one second.

| Item | State | Notes |
|---|---|---|
| eframe settings window | Pending | |
| Tray icon and menu | Pending | |
| Single instance | Pending | Named mutex |
| Theme system (System, Dark, Light; Dark default) | Pending | |
| Start with Windows | Pending | HKCU `Run`, refreshed each launch |
| Language selector | Pending | |
| Live language switch | Pending | No restart, no migration |

## Phase 2 — Toast Core
> **Exit criteria:** every shape renders correctly at 100, 150 and 200 percent DPI in both languages, with no overflow.

| Item | State | Notes |
|---|---|---|
| Borderless transparent always-on-top toast window | Pending | |
| Shapes | Pending | Rectangle, rounded rectangle, pill |
| Corner radius | Pending | 0–40 px |
| Colours | Pending | Solid and gradient |
| Opacity | Pending | 40–100 % |
| Shadow | Pending | Off, soft, strong |
| Header | Pending | Icon, title, app name, timestamp |
| Text | Pending | Bold and italic |
| Image | Pending | Magic-byte validation, pixel limit, thumbnail cache |
| Stacking | Pending | 1–5 visible, configurable gap |
| DPI and multi-monitor positioning | Pending | |
| Ellipsis rules | Pending | Nothing overflows the toast |
| Malay length testing | Pending | Longest Malay strings |

## Phase 3 — Glass and Animations
> **Exit criteria:** each animation runs at 60 fps on reference hardware and falls back cleanly.

| Item | State | Notes |
|---|---|---|
| Blur | Pending | |
| Acrylic | Pending | |
| Mica-like | Pending | Windows 10 1809+ |
| In animations | Pending | Slide right, slide bottom, Genie, pop out, drawer, appear, dissolve, glow, alpha-to-solid |
| Out animations | Pending | Mirrors, plus shrink to tray |
| Reduced-motion fallback | Pending | Follows the Windows "Show animations" setting |
| Frame-time fallbacks | Pending | Fade below 30 fps |

## Phase 4 — Scheduler and Reminders
> **Exit criteria:** the scheduling tests pass, reminders fire within one second of target, and the date format is correct for each language.

| Item | State | Notes |
|---|---|---|
| Reminder CRUD | Pending | |
| Offsets | Pending | 15 minutes to a week, or custom |
| Recurrence | Pending | Once, daily, weekly, monthly, yearly, custom |
| Snooze | Pending | 5 minutes to a day; max per reminder |
| Duration | Pending | 3–60 s, hover pauses |
| Persistent alarms | Pending | 10-minute cap on urgent types |
| Missed-reminder handling | Pending | 1 h catch-up for Azan, 24 h for others |
| DST-safe scheduling | Pending | Recompute on resume and clock change |
| Localized reminder types | Pending | Templates in both languages |
| Shared date formatter | Pending | The only formatter in the workspace |
| Migration 002 | Pending | `002_locale_keys.sql` |

## Phase 5 — Location, Prayer and Azan
> **Exit criteria:** the parser passes the fixtures for three zones; live JAKIM times match the official portal on the same dates for at least three zones; the fallback chain passes with the network disabled; prayer labels match the reference table in both languages.

| Item | State | Notes |
|---|---|---|
| e-Solat verification spike | Pending | **First.** Three zones; result recorded in `docs/08-decisions.md` |
| Zone table | Pending | `data/jakim_zones.csv`, reviewed commits only |
| JAKIM client | Pending | HTTPS, 10 s timeout, backoff 1/5/15 min, no polling |
| Parser with fixtures | Pending | `tests/fixtures/jakim_sample.json` |
| Cache and migration 003 | Pending | `003_jakim.sql` |
| Fallback chain | Pending | Cache → live → bundled → calculated → last known |
| Zone selection UI | Pending | Detected zone shown for confirmation, always overridable |
| Calculated mode with Malaysia preset | Pending | Fajr 20.0°, Isha 18.0°, Asar Shafi'i |
| Per-prayer templates | Pending | Audio, title, body, image, accent, style |
| Audio playback | Pending | Full Azan or short chime |
| Pre-prayer reminders | Pending | |
| Dual prayer names | Pending | "Subuh (Fajr)" in settings |

## Phase 6 — Weather
> **Exit criteria:** a condition change triggers a toast in simulation, and offline mode shows the stale marker in both languages.

| Item | State | Notes |
|---|---|---|
| Open-Meteo service | Pending | No API key |
| Caching | Pending | TTL-based |
| Condition change detection | Pending | Condition change or temperature threshold |
| Per-condition templates | Pending | |
| Weather code to Fluent key mapping | Pending | No API text displayed |
| Offline stale marker | Pending | |

## Phase 7 — Time Announcements
> **Exit criteria:** announcements fire on schedule in the selected language or fall back with a visible hint, and quiet hours are respected.

| Item | State | Notes |
|---|---|---|
| Speech synthesis | Pending | Windows `SpeechSynthesizer` |
| Language-tag voice selection | Pending | From `AllVoices`, never a hard-coded name |
| Malay voice detection and fallback hint | Pending | Chime plus a setup hint when no voice exists |
| Schedules | Pending | Hourly, 30 min, 15 min, start/end of day, custom |
| Quiet hours | Pending | Suppress announcements and non-urgent toasts |
| Azan priority | Pending | Always wins |

## Phase 8 — Supervisor and Hardening
> **Exit criteria:** a forced panic is recovered and logged, a crash loop stops at the threshold, and the security review checklist is complete.

| Item | State | Notes |
|---|---|---|
| Watchdog supervisor | Pending | Default process spawns `--worker` |
| Crash-loop protection | Pending | Exponential backoff; stop after 5 crashes in 10 min |
| Crash reports | Pending | Version, OS build, last 200 log lines, panic message; no personal data |
| Log retention | Pending | 14 days, configurable |
| Security review of imports and external data | Pending | Size limits, magic bytes, path canonicalization |

## Phase 9 — Packaging and Release
> **Exit criteria:** the clean-machine test passes in both languages; reviewer sign-off recorded; terms decision recorded; the release is published with the full asset set; and the updater moves a clean install from the previous release to the current one, refuses a manifest whose signature does not verify, and leaves `OmniToasterData/` intact.

| Item | State | Notes |
|---|---|---|
| Portable zip | Pending | `OmniToaster-<version>-windows-x64.zip` |
| Checksums | Pending | SHA-256 per asset |
| Signing | Pending | rsign signature over `manifest.json`; code signing to be decided |
| `manifest.json` | Pending | What the updater reads |
| In-app updater | Pending | FEAT-0008 — a requirement, not an option |
| Bilingual READMEs | Pending | `README.md`, `README.ms.md` |
| Bilingual user guide | Pending | |
| Bilingual screenshots and GIFs | Pending | `docs/screenshots/`; none captured yet |
| JAKIM terms check | Pending | Decides whether the dataset may be bundled |
| Attribution in app and docs | Pending | "Source: JAKIM e-Solat" in both languages |
| Malay translation review sign-off | Pending | Named native speaker |
