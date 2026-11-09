# OmniToaster — implementation checklist

States: `Completed` · `In-Progress` · `Pending`. Update a line only after the work is verified (checks pass and the feature was exercised in the running app), then commit and push.

The rows come from the Key Deliverables of each phase in `PROPOSAL.md` §13; the
exit criteria under each heading are that phase's own, and no row in a phase may
read `Completed` until they all hold. The work itself, by id, is in
[TASKS.md](TASKS.md).

Last updated: 2026-11-08

## Planning
> **Done when:** the scaffolding and the specifications describe OmniToaster and point only at files that exist; no reference to the HyperStack project remains.

| Item | State | Notes |
|---|---|---|
| Proposal reviewed and stakeholder questions answered | Completed | `PROPOSAL.md` §16; version 1.0, 2026-11-08 |
| Proposal structure repaired (section numbering, dangling references) | Completed | Duplicate §12–§14 renumbered to §15–§17; §15, §17, §18, §9.7 and one §10 reference resolved |
| Agent scaffolding realigned off HyperStack | Completed | `.claude/` — 33 files: 11 rewritten, 20 edited, `async-and-jobs.md` → `threads-and-scheduling.md` and `spec-tool.md` → `spec-area.md`, new `rules/i18n.md`. The phantom skills table is gone; the commands are the mechanism |
| Root documents and work ledger realigned | Completed | `CLAUDE.md`, `README.md`, `docs/TASKS.md`, `docs/CHECKLIST.md`, `docs/GOTCHAS.md`, `docs/CHANGELOG.md` |
| Numbered specifications written (`docs/01`–`docs/11`) | Completed | Includes the ADR log with ADR-001–012 |
| Per-area specifications written (`docs/specs/`) | Completed | `toast`, `reminders`, `weather`, `announcements`, `tray-and-settings`, `supervisor`, `updater` |
| CI workflows and scripts renamed to the OmniToaster asset contract | Completed | `.forgejo/`, `.github/`, `scripts/`; all YAML and all six scripts parse. Not executed — the workflows can only run on a push |
| Screenshot procedure written | Completed | `docs/screenshots/README.md`; no images yet, no code to photograph |
| Every cross-reference resolves | Completed | Repo-wide link check over every `*.md`: zero broken relative links. Marker grep clean apart from prose that deliberately names what was removed |

## Phase 0 — Foundation
> **Exit criteria:** the app starts in English and in Malay, writes a log, the locale key parity check passes, and CI is green.
>
> **All four hold as of 2026-11-08.** The app starts in English and in Malay
> and renders its startup errors in each; it writes a rotating log; the locale
> parity check passes, and now runs inside `--self-test` as well as in the unit
> tests; and CI is green. v0.1.0 and v0.2.0 are published and
> `scripts/verify-release.ps1` verifies them.
>
> One caveat that is not an exit criterion but should not be lost: BUG-0001 is
> open and `Blocked` on the owner — the forge intermittently stops creating
> workflow runs, which makes the mandated branch → CI green → merge flow
> unreliable rather than wrong.

| Item | State | Notes |
|---|---|---|
| Repository and `docs/` skeleton | Completed | `docs/` complete; the Cargo workspace and all eleven crates build — FEAT-0001 |
| `CLAUDE.md` | Completed | Realigned, and `.claude/rules/` has landed — every link resolves |
| Cargo workspace skeleton | Completed | FEAT-0001. 11 members, `cargo fmt`/`clippy -D warnings`/`test --locked -j 4`/`deny check` all green; `OmniToaster.exe` builds at 223 KB and prints its version; no dynamic CRT imports, so `+crt-static` is confirmed |
| CI skeleton | Completed | FEAT-0007. `ci.yml` green on every branch push that the forge dispatched; `release.yml` published v0.1.0 (run 14) and v0.2.0 (run 18), both verified by `scripts/verify-release.ps1`. BUG-0001 is **open** — the forge stops creating runs intermittently, three times on 2026-11-08 |
| App icon | Completed | FEAT-0010. Master plus seven generated sizes and a seven-entry `.ico`, embedded in the exe with its version metadata; small sizes sharpened and inspected visually |
| `--self-test` and the command line | Completed | FEAT-0006. Six checks, one flushed line each, name before verdict so an abort still names the dying check; `--help` in both languages; an unknown flag is named rather than ignored. 14 tests in the binary, 7 of them driving the real exe |
| Logging | Completed | FEAT-0003. `tracing` + `tracing-appender`, daily rotation, 14-day retention swept at startup, `--log-level` override; verified in the app |
| Panic hook | Completed | FEAT-0003. Crash report with version, OS, message, location and the last 200 log lines, data folder redacted; verified in the app with a debug-only `--panic-now` |
| Portable path resolution with writability check | Completed | FEAT-0002. `ot-core::paths`; 17 tests green; verified in the app both ways — the subtree appears under a scratch `--data-dir`, and a genuinely ACL-denied folder is reported with its OS cause, exits 1 and creates nothing |
| Fluent setup with both `.ftl` files | Completed | FEAT-0004. 16 tests including key parity, prefixes and the Malay fallback; both languages verified in the app |
| `fl!()` wired in | Completed | **Not as planned, and deliberately so:** `fl!` derives the catalogue filename from the crate name, which would rename the locale files away from `omni_toaster.ftl`. The loader is built with an explicit domain and the compile-time key check is replaced by the parity, prefix and casing tests — ADR-006 amendment, FEAT-0004 |
| Locale detection | Completed | FEAT-0004. `GetUserDefaultLocaleName` in `ot-platform`, the only `unsafe` in the workspace, scoped with a reason; `--lang` overrides it |
| Migration 001 | Completed | FEAT-0005. `ot-store` with bundled SQLite, WAL, foreign keys enforced, `STRICT` tables; 8 tests; verified in the app that the database appears and a second run does not re-migrate |

## Phase 1 — Shell
> **Exit criteria:** the tray works, the theme switches, autostart survives a
> reboot, and a language switch updates the tray and the settings window in
> under one second.

| Item | State | Notes |
|---|---|---|
| Single instance | Completed | FEAT-0011. Named mutex per role; proven across two real processes, not just within one |
| Start with Windows | In-Progress | FEAT-0012. Write, read-back and the moved-folder refresh all verified against the real registry; the enable toggle and the reboot check wait on the settings window |
| The three themes | In-Progress | FEAT-0013. The palette, the three modes and the `AppsUseLightTheme` poll are done, tested and now seen on screen — Dark, Light and System each drew correctly once the window existed. Toggling the Windows setting itself, with System selected, is the owner's check: it is their setting to change |
| Settings window | In-Progress | FEAT-0014. The window, the General pane and the status line are done and checked in both languages, all three themes, at the default size and at the minimum. Closing it now hides it rather than ending the app (FEAT-0015); the Start with Windows click still needs the owner |
| Tray icon and menu | In-Progress | FEAT-0015. The icon, the menu, pause and Quit are written and the icon is created without error in both languages; Windows 11 hides a new icon in the overflow, so clicking through the menu is the owner's check. CR-0003 covers the ghost icon a killed process leaves |
| Live language switch | In-Progress | FEAT-0016. The loader is swappable and the window follows in 2.4 ms, title bar included, measured in the running app; the tray menu's half of the check is behind the same Windows 11 overflow as FEAT-0015 |

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
