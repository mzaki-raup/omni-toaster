# 07 — Roadmap

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../PROPOSAL.md) §13

Ten phases, 0 to 9, in this order. Each one ends with the gate green, the work
exercised in the running app, [CHECKLIST.md](CHECKLIST.md) updated,
[CHANGELOG.md](CHANGELOG.md) lines written, and the branch merged into `main`
after CI goes green. **No checklist item is marked `Completed` before it is
verified.** Progress lives in [CHECKLIST.md](CHECKLIST.md); the work items live
in [TASKS.md](TASKS.md).

There is no lettered phase programme. Phases are numbered, and `/phase <n>`
works one of them.

| Phase | Name | Key deliverables | Exit criteria |
|---|---|---|---|
| **0** | Foundation | The Cargo workspace and its eleven crates, `docs/` skeleton, CI skeleton, logging, panic hook, portable path resolution with the writability check, Fluent setup with both `.ftl` files and `fl!()` wired in, locale detection, migration `001_init.sql`, the `--data-dir` / `--self-test` / `--worker` flags | The app starts in English and in Malay, writes a log, the key-parity check passes, `--self-test` passes, CI green |
| **1** | Shell | eframe settings window, tray icon and menu, single instance, the three themes with Dark default, Start with Windows, language selector, live language switch | Tray works, theme switches, autostart survives a reboot, a language switch updates the tray and the settings window in under one second |
| **2** | Toast Core | Borderless transparent always-on-top toast viewport, shapes, radius, colours, opacity, shadow, header, text, image, stacking, DPI and multi-monitor positioning, ellipsis rules, `ToastSpec`, the Settings preview, Malay length testing | Every shape renders correctly at 100%, 150% and 200% DPI in both languages with no overflow, and the preview matches the real toast |
| **3** | Glass and Animations | Blur, Acrylic and Mica-like effects; every in and out animation; reduced-motion and frame-budget fallbacks | Each animation runs at 60 fps on the reference machine and falls back cleanly when it cannot |
| **4** | Scheduler and Reminders | Reminder CRUD, lead offsets, recurrence, snooze, duration, persistent alarms, missed-reminder catch-up, DST-safe scheduling, localized reminder types, the shared date formatter, migration `002_locale_keys.sql` | Scheduling tests pass, reminders fire within one second of target, the date format is correct in each language |
| **5** | Location, Prayer and Azan | **Spike first:** verify the e-Solat endpoint and response for three zones and record the result as an ADR. Then: the zone table, the JAKIM client, the parser with fixtures, the cache and migration `003_jakim.sql`, the fallback chain, the zone selection UI, calculated mode with the Malaysia preset, per-prayer templates, audio playback, pre-prayer reminders, dual prayer names | The parser passes the fixtures for three zones. Live JAKIM times match the official portal on the same dates for at least three zones. The fallback chain passes with the network disabled. Prayer labels match the reference table in both languages. |
| **6** | Weather | The Open-Meteo service, caching, condition-change detection, per-condition templates, weather code to Fluent key mapping, the offline stale marker | A simulated condition change triggers a toast; offline mode shows the stale marker in both languages |
| **7** | Time Announcements | Speech synthesis, voice selection by language tag, Malay voice detection and the fallback hint, schedules, quiet hours, Azan priority | Announcements fire on schedule in the selected language or fall back with a visible hint; quiet hours are respected |
| **8** | Supervisor and Hardening | The watchdog supervisor, crash-loop protection, crash reports, log retention, a security review of the imports and the external data paths | A forced panic is recovered and logged; a crash loop stops at the threshold; the security review checklist is complete |
| **9** | Packaging and Release | The portable zip, checksums, signing, **the in-app updater**, bilingual READMEs, a bilingual user guide, bilingual screenshots and GIFs, the JAKIM terms check, attribution in the app and the docs, Malay translation review sign-off | A clean-machine test passes in both languages; the updater moves a real release forward and verifies its signature; the reviewer sign-off is recorded; the terms decision is recorded; the release is published |

## Dependencies between phases

```
0 ──► 1 ──► 2 ──► 3
      │     │
      └──►  4 ──► 5 ──► 6
            │     └──►  7
            └──────────►8 ──► 9
```

Phase 3 needs Phase 2's renderer. Phase 5 needs Phase 4's scheduler. Phase 7
needs Phase 5 only for the Azan-priority rule. Phase 9 needs everything,
because it ships it.

## Effort

| Phases | Estimate |
|---|---|
| 0 – 2 | 3–4 weeks |
| 3 – 5 | 6–7 weeks (Phase 5 includes about a week for the JAKIM spike and integration) |
| 6 – 9 | 4–5 weeks, plus the in-app updater |
| Localization overhead, spread across all phases | 1–1.5 weeks (250–400 strings, layout testing, voice handling) |
| **Total** | **roughly 14–17 weeks** for one experienced Rust developer, depending on animation scope, JAKIM verification effort and translator availability |

## Risks

The full register is in [PROPOSAL.md](../PROPOSAL.md) (Risks and Mitigations).
The four that most often change a plan:

| Risk | Why it bites | What we do |
|---|---|---|
| Transparent and glass windows behave differently across Windows builds | Phase 3's output looks wrong on a machine nobody tested | A Win10/Win11 test matrix, and a solid fallback that is always available |
| The e-Solat endpoint changes or goes away | Phase 5's whole premise | A fixture-based parser, the monthly cache, the bundled dataset, the calculated fallback, and a visible source label so the user always knows what they are looking at |
| Scheduler drift after sleep or a DST change | A missed Azan, which is the one failure users will not forgive | Recompute on resume, missed-reminder catch-up, and DST tests from Phase 4 onward |
| No TTS voice installed for the chosen language | Phase 7 silently does nothing | Detect the installed voices, let the user pick, fall back to a chime, and say so in the UI |
