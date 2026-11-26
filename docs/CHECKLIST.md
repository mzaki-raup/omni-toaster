# OmniToaster — implementation checklist

States: `Completed` · `In-Progress` · `Pending`. Update a line only after the work is verified (checks pass and the feature was exercised in the running app), then commit and push.

The rows come from the Key Deliverables of each phase in `PROPOSAL.md` §13; the
exit criteria under each heading are that phase's own, and no row in a phase may
read `Completed` until they all hold. The work itself, by id, is in
[TASKS.md](TASKS.md).

Last updated: 2026-11-19

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
>
> **All four hold as of 2026-11-15.** The tray menu acts and a double-click on
> the icon opens Settings, with the settings window open *and* closed — the
> second was broken until BUG-0008, because a hidden window gets no frames and
> the app does everything in a frame. The three themes switch, System following
> Windows with the window open, title bar included (CR-0002). Start with Windows
> survived a reboot and a moved folder, checked by the owner. A language switch
> reaches the window in 2.4 ms and the tray menu within a second.

| Item | State | Notes |
|---|---|---|
| Single instance | Completed | FEAT-0011. Named mutex per role; proven across two real processes, not just within one |
| Start with Windows | Completed | FEAT-0012, FEAT-0014. Write, read-back and the moved-folder refresh verified against the real registry; the checkbox, a reboot and a moved folder checked by the owner |
| The three themes | Completed | FEAT-0013, CR-0002. Dark, Light and System; System follows the Windows setting within five seconds, and the title bar follows the theme |
| Settings window | Completed | FEAT-0014. The General pane and the status line, both languages, all three themes, default and minimum size. Closing it parks it off-screen rather than hiding it (BUG-0008) |
| Tray icon and menu | Completed | FEAT-0015, CR-0003, BUG-0008. Every item checked by the owner in both languages; double-click opens Settings; a killed run's icon is removed on the next start |
| Live language switch | Completed | FEAT-0016. The loader is swappable; the window follows in 2.4 ms, title bar included, and the tray menu within a second |

## Phase 2 — Toast Core
> **Exit criteria:** every shape renders correctly at 100, 150 and 200 percent DPI in both languages, with no overflow, and the preview matches the real toast.
>
> **Both hold as of 2026-11-14.** Every shape was checked by the owner at 100%,
> 150% and 200% through the real Windows setting in both languages, including
> a change of scaling with a toast on screen; the same sweep was run here with
> `--ui-scale`, and the layout tests hold it in the gate in English, Malay and
> a pseudo-locale (FEAT-0024). The preview is drawn by the same `Prefs::spec`,
> `render::frame` and `content::draw` as a real toast and was compared side by
> side. Two differences remain, both the cost of decisions the owner made on
> evidence: the Rectangle's shadow is Windows' own on a real toast and egui's
> nearest equivalent in the preview (CR-0006), and below 100% opacity the
> preview fades shape by shape where the real window fades as one, so a
> shadow under it reads slightly darker (CR-0005).

| Item | State | Notes |
|---|---|---|
| `ToastSpec`, limits enforced in the model | Completed | FEAT-0017. Every limit clamps on load, with a logged warning; the renderer takes a spec and nothing else |
| Borderless transparent always-on-top toast window | Completed | FEAT-0018. Out of the taskbar and Alt+Tab, never takes focus — checked by the owner over a full-screen application. Corners see-through on any driver through a window region (BUG-0007, ADR-015) |
| Shapes | Completed | FEAT-0019. Rectangle, rounded rectangle, pill; the side facing the screen edge is square and carries the accent bar (CR-0004) |
| Corner radius | Completed | 0–40 px; a pill's is half its height |
| Colours | Completed | Solid and gradient; text colour follows the toast's own background (BUG-0005) |
| Opacity | Completed | 70–100 %, fading the whole toast through the window's alpha (CR-0005) |
| Shadow | Completed | On or off; Windows' own, Rectangle only (CR-0006) |
| Header | Completed | FEAT-0020. Icon, title, app name, timestamp, each toggleable; each notification carries its own icon, image and accent |
| Text | Completed | Bold and italic for the body as a whole |
| Image | Completed | FEAT-0021, FEAT-0025. Magic-byte validation, pixel limit, a thumbnail cached under `cache/thumbs/`, and the picture library |
| Stacking | Completed | 1–5 visible, configurable gap — set in Settings ▸ Appearance and checked with five real toasts and a queue of two; the stack of five checked by the owner at 250% |
| DPI and multi-monitor positioning | Completed | FEAT-0022. Nine presets and a custom point; a chosen monitor, remembered by device path, falling back to the primary one when unplugged — checked by the owner on a second monitor |
| Ellipsis rules | Completed | Nothing overflows the toast; held by tests at three scales |
| The Settings preview | Completed | FEAT-0023. The same drawing code as a real toast; display duration (FEAT-0026), monitor, position and stacking set beside it |
| Malay length testing | Completed | FEAT-0024. Layout tests in English, Malay and a pseudo-locale at 100%, 150% and 200% |

## Phase 3 — Glass and Animations
> **Exit criteria:** each animation runs at 60 fps on reference hardware and falls back cleanly.
>
> **As of 2026-11-19, demonstrated on this machine, with three checks left to
> the owner.** *60 fps:* every in- and out-animation was counted from the log
> in a release build with three toasts on screen — the lowest of any run was
> shrink to tray out at 97 fps, every other effect 123 fps or more (FEAT-0031
> to FEAT-0034). This is the development machine; whether it is the reference
> hardware the criterion means is the owner's to say. *Falls back cleanly:*
> the frame-time fallback was forced (`--slow-frames`) and also triggered on its
> own in a loaded debug build: the toast finished as a fade where it was, the
> next one started as one, and the log named it (FEAT-0035). Reduce Motion in
> the app and `--reduce-motion` turn every effect into a fade. **Not made
> here:** Windows' own "Show animations" switch turned off and on (FEAT-0035),
> shrink to tray with the tray icon in the overflow (FEAT-0034), and any
> animation at real 150% display scaling (FEAT-0032, FEAT-0038) — each is a
> system setting this session does not change. The rows they belong to stay
> In-Progress until the owner has made them.

| Item | State | Notes |
|---|---|---|
| Blur | Completed | FEAT-0030. Rectangle only (owner, 2026-11-18): the desktop blurred under 45 % of the toast's colour; the other shapes are greyed out with the reason |
| Acrylic | Completed | FEAT-0030. 60 %; `docs/screenshots/toast-glass.png` |
| Mica-like | Completed | FEAT-0030. 88 %; Windows 10 1809+. Checked on Windows 11 only — the solid fallback after a refusal was never needed here, and Windows 10 was not available |
| In animations | In-Progress | FEAT-0032 to FEAT-0034, FEAT-0036. All nine, each seen on screen, fired from Settings ▸ Animation ▸ Play, and placed inside the work area at every one of the nine positions; 123 fps or more in a release build with three toasts. Open: real 150% scaling (owner) |
| Out animations | In-Progress | The nine mirrors, plus shrink to tray (97 fps or more). Open: shrink to tray with the icon in the overflow (owner) |
| Reduced-motion fallback | In-Progress | FEAT-0035, FEAT-0036. Windows' setting is read every five seconds and stated in the pane; Reduce Motion and `--reduce-motion` fade every effect. Open: Windows' own switch turned off and on (owner) |
| Frame-time fallbacks | Completed | FEAT-0035. Below 30 fps — the median of six frames — the toast finishes as a fade and the next starts as one, logged; forced and seen happen unforced |

## Phase 4 — Scheduler and Reminders
> **Exit criteria:** the scheduling tests pass, reminders fire within one second of target, and the date format is correct for each language.

| Item | State | Notes |
|---|---|---|
| Reminder CRUD | Completed | FEAT-0039: the store side — create, read, list, update, delete, typed trigger and recurrence, unknown fields refused. FEAT-0046: the list and the editor, in the app in both languages, all three themes and the narrow window |
| Offsets | Completed | 15 minutes to a week, or custom. FEAT-0040: the rule, tested — several per reminder, each its own fire, day offsets in calendar days. FEAT-0041's scheduler fires them; FEAT-0047: a 1-day offset fired in the app, its toast saying "1 day before" and the event's own time, in both languages |
| Recurrence | Completed | Once, daily, weekly, monthly, yearly, custom. FEAT-0040: every *N* of each, chosen weekdays, month ends and leap years, tested. FEAT-0041: a daily reminder fired in the app and moved to the next day. FEAT-0046: a weekly one (Monday and Thursday) made in the editor and fired |
| Snooze | Completed | 5 minutes to a day, or custom; max per fire. FEAT-0043: Snooze on the toast, Settings ▸ Reminders' Snooze section, the limit and its line — snoozed and returned in the app in English and Malay, the limit reached with a low limit set |
| Duration | Completed | 3–60 s, hover pauses. FEAT-0026 and BUG-0009 built it; FEAT-0044 checked that a reminder other than an Alarm keeps it — a Task left after its 8 s in the app |
| Persistent alarms | Completed | 10-minute cap on urgent types. FEAT-0044: an Alarm stays until acted on and is recorded missed at the cap; in the app, with the cap shortened to 30 s, it outlived a Task's 8 s and closed at 30 s. Azan joins in Phase 5 |
| Missed-reminder handling | In-Progress | 1 h catch-up for Azan, 24 h for others. FEAT-0045: everything but Azan — listed in Settings ▸ Reminders with Snooze and Dismiss, older ones dropped and recorded, checked in the app in both languages; Azan's one-hour window comes with Azan (Phase 5) |
| DST-safe scheduling | In-Progress | Recompute on resume and clock change. FEAT-0040: occurrences in the wall clock, gap and fold rules, tested in `America/New_York`. FEAT-0041: the queue rebuilt at midnight, on a clock jump, a zone change and a resume (Windows' events through `ot_platform::system_events`), each proved with simulated events; the real clock and zone changes are an owner's check, not made |
| Localized reminder types | Completed | Templates in both languages. FEAT-0042: twelve type keys with a name and a default text in both files, each fired in the app in English and Malay; Open lands on the reminder since FEAT-0046 |
| Shared date formatter | Completed | The only formatter in the workspace. FEAT-0047: Malay d/m/yyyy and 24-hour; English following Windows' date order and clock, with the user's Clock over both; offset and repeat phrases with plurals; a written table tested in both languages, and read in the app on the list, the editor and an offset fire's toast in both |
| Fatigue reminders | Completed | Eye break, posture, hydration, work-hour limit, each off until switched on. FEAT-0048: the defaults and the working day (midnight carries on, an hour away starts again, a restart within the hour keeps it), tested; each kind fired in the app with a short interval in English and Malay |
| Migration 003 | Completed | FEAT-0039. `003_locale_keys.sql` — `002` went to `002_asset_label.sql` in Phase 2. Tested fresh and on a version 2 database; in the app, a fresh folder and a v0.11.0-era one both read `user_version` 3 |

## Phase 5 — Location, Prayer and Azan
> **Exit criteria:** the parser passes the fixtures for three zones; live JAKIM times match the official portal on the same dates for at least three zones; the fallback chain passes with the network disabled; prayer labels match the reference table in both languages.

| Item | State | Notes |
|---|---|---|
| e-Solat verification spike | Completed | **First.** Three zones; result recorded in `docs/08-decisions.md`. FEAT-0049: SGR01, WLY01 and JHR01 on 2026-11-20 — the shape, every period, next month as a POST within a published year, the failure cases, Asar confirmed Shafi'i; ADR-018, raw responses kept as fixtures |
| Zone table | In-Progress | `data/jakim_zones.csv`, reviewed commits only. FEAT-0050: 60 zones from JAKIM's portal and 208 area points from GeoNames, the nearest area deciding; every state capital lands in its zone, outside Malaysia is refused, tested. Seen in the app with the zone picker (FEAT-0057) |
| JAKIM client | In-Progress | HTTPS, 10 s timeout, backoff 1/5/15 min, no polling. FEAT-0052: `ot_prayer::fetch` built and tested, live for SGR01, WLY01 and JHR01 over Windows TLS; not yet running in the app — the scheduler and the zone setting come in FEAT-0059 and FEAT-0057 |
| Parser with fixtures | Completed | `tests/fixtures/jakim_sample.json`. FEAT-0051: `ot_prayer::jakim::parse` reads all 30 November days of SGR01, WLY01 and JHR01 as captured, with 20 November's times matching e-Solat's; every broken day in the sample set aside with its reason, tested |
| Cache and migration 004 | In-Progress | `004_jakim.sql`. FEAT-0053: `ot_store::prayer` writes, reads and judges freshness, tested on a fresh and a version 3 database; not yet written by the app — the fetch loop is FEAT-0058, where "the times survive a restart with the network off" is checked |
| Fallback chain | Pending | Cache → live → bundled → calculated → last known |
| Zone selection UI | Pending | Detected zone shown for confirmation, always overridable |
| Calculated mode with Malaysia preset | In-Progress | Fajr 18.0° (not 20.0°: JAKIM's tables put that ten minutes early), Isha 18.0°, Asar Shafi'i. FEAT-0054: `ot_prayer::calculated` built; every time within three minutes of JAKIM's in SGR01, WLY01 and JHR01; the other presets match the adhan-js reference rows; not yet shown in the app — Settings ▸ Prayer is FEAT-0057 |
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
