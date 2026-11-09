# 03 — Architecture

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../PROPOSAL.md) §5

## Crate layout

OmniToaster is a Cargo workspace. The version lives once, under
`[workspace.package]` in the root `Cargo.toml`; every member declares
`version.workspace = true`. See [ADR-001](08-decisions.md).

```
omni-toaster/
├── Cargo.toml                  # [workspace] + [workspace.package] version
├── .cargo/config.toml          # +crt-static, so the MSVC build stays portable
├── crates/
│   ├── omnitoaster/            # bin: OmniToaster.exe — supervisor, worker, CLI
│   ├── ot-core/                # settings, paths, event bus, errors, logging, panic hook
│   ├── ot-i18n/                # Fluent bundles, locale detection, the one formatter
│   ├── ot-store/               # SQLite, migrations, asset store
│   ├── ot-platform/            # ALL Windows API code (see "Platform isolation")
│   ├── ot-ui/                  # shared egui widgets, theme, settings window
│   ├── ot-toast/               # ToastSpec, renderer, animations
│   ├── ot-sched/               # the scheduler thread, recurrence, snooze
│   ├── ot-prayer/              # PrayerSource, JakimSource, CalculatedSource, zones
│   ├── ot-weather/             # Open-Meteo client, condition-change detection
│   └── ot-media/               # audio playback (rodio), speech (SpeechSynthesizer)
├── locales/{en-US,ms-MY}/omni_toaster.ftl
├── assets/
├── data/jakim_zones.csv
├── tests/fixtures/
├── docs/
└── scripts/
```

Workspace-wide settings that are decisions rather than taste:

| Setting | Why |
|---|---|
| `resolver = "3"` | The edition 2024 resolver. |
| `[workspace.lints]` with `unsafe_code`, `clippy::unwrap_used`, `todo`, `dbg_macro` set to `warn`, and every member opting in with `[lints] workspace = true` | The project rules forbid a crate-wide `allow`, so the lints that must hold everywhere are declared once. The gate runs clippy with `-D warnings`, so these are errors in practice. |
| `[profile.release]` `panic = "abort"` | A panicking worker should die and be restarted, not limp on half-unwound. The panic hook still runs and still writes the crash report, then the process aborts with a non-zero exit — which is exactly the signal the supervisor waits for ([specs/supervisor.md](specs/supervisor.md) §4). It also means `catch_unwind` is not available as a recovery mechanism anywhere, which is the intent. |
| `[profile.release]` `strip`, `lto = "thin"`, `codegen-units = 1` | One portable exe, as small as the toolchain will make it. |
| `[profile.dev] debug = 1` | Pinned in the manifest rather than passed as `CARGO_PROFILE_DEV_DEBUG` on the command line, which changes every fingerprint and rebuilds the workspace ([GOTCHAS.md](GOTCHAS.md)). |
| `.cargo/config.toml` with `-C target-feature=+crt-static` | The MSVC runtime is linked in, so the folder runs on a machine with no Visual C++ redistributable. Verified by the absence of `VCRUNTIME`/`msvcp`/`api-ms-win-crt` imports in the built exe. |

Dependency direction is one way only:

```
omnitoaster ─► ot-ui ─► ot-toast ─┐
     │          │  │              ├─► ot-core ◄─ ot-i18n
     │          │  └──────────────┘        ▲
     │          ├─► ot-i18n ───────────────┤
     │          └─► ot-platform ───────────┤
     ├─► ot-sched ─► ot-store ─────────────┤
     ├─► ot-prayer ────────────────────────┤
     ├─► ot-weather ───────────────────────┤
     ├─► ot-media ─────────────────────────┤
     └─► ot-platform ──────────────────────┘
```

`ot-core` depends on nothing of ours. No library crate depends on `ot-ui`, and
no service crate (`ot-sched`, `ot-prayer`, `ot-weather`, `ot-media`) depends on
another service crate — they meet only in the binary, through the event bus.

**`ot-ui` depends on `ot-platform` and `ot-i18n`**, which the first drawing of
this diagram did not allow for. The theme reads the Windows setting itself
(`ot_platform::system_theme`) and owns the five-second recheck, and the window
loads the Windows UI font (`ot_platform::ui_font`, ADR-014); every string it
draws comes from Fluent. Passing a reading and a bundle in from the binary
would have moved that policy into `main` for no gain. The rule that matters is
unchanged: **Windows code lives in `ot-platform`**, behind an interface named
for the capability, and `ot-ui` calls it rather than reaching for `windows` or
`winreg` itself.

## Runtime shape

```
┌────────────────────────────────────────────────────────────────────────┐
│                           OmniToaster.exe                              │
│                                                                        │
│  ┌────────────┐  ┌─────────────┐  ┌──────────────────────┐             │
│  │ Tray & Menu│  │ Settings UI │  │   Toast Renderer     │             │
│  │ ot-platform│  │   ot-ui     │  │      ot-toast        │             │
│  └─────┬──────┘  └──────┬──────┘  └──────────┬───────────┘             │
│        └───────┬────────┴────────────────────┘                         │
│                ▼                                                       │
│        ┌──────────────┐        ┌───────────────────────────────────┐   │
│        │  App Core    │◄──────►│   Event bus (crossbeam channels)  │   │
│        │   ot-core    │        └───────────────────────────────────┘   │
│        └──────┬───────┘                                                │
│   ┌───────────┼────────────┬──────────────┬───────────┬──────────┐     │
│   ▼           ▼            ▼              ▼           ▼          ▼     │
│ ot-sched   ot-prayer   ot-weather     ot-media    ot-platform  ot-i18n │
│   │           │              │                                         │
│   └───────────┴──────┬───────┘                                         │
│                      ▼                                                 │
│      ┌────────────────────────────┐   ┌───────────────────────────┐    │
│      │  ot-store (SQLite + TOML)  │   │ Logging & crash reports   │    │
│      └────────────────────────────┘   └───────────────────────────┘    │
│                                                                        │
│  Supervisor (ot-platform): launches the worker, restarts with backoff  │
└────────────────────────────────────────────────────────────────────────┘
```

## Key design decisions

1. **Supervisor and worker processes.** `OmniToaster.exe` runs as a supervisor
   by default and spawns itself with `--worker`. It watches the worker's exit
   status, records crash information and restarts with exponential backoff.
   After 5 crashes in 10 minutes it stops and shows a bilingual notification
   with the log path. This catches what `catch_unwind` cannot — GPU driver
   faults, stack overflow, an aborting allocator. See
   [specs/supervisor.md](specs/supervisor.md) and
   [ADR-005](08-decisions.md).
2. **Event-driven core.** Modules communicate through crossbeam channels. The
   UI, the scheduler and the services never call each other directly.
3. **No tokio.** The scheduler owns time on its own thread; HTTP is blocking
   `ureq` on short-lived worker threads. A single-threaded async runtime would
   buy nothing here and would add a dependency the whole workspace would have
   to agree on. See [ADR-002](08-decisions.md) and
   [threads-and-scheduling.md](../.claude/rules/threads-and-scheduling.md).
4. **The scheduler is the single source of time.** It stores the next fire time
   for each reminder, recalculates after a DST change, a system resume or a
   clock change, and never relies on a long sleep.
5. **Toast rendering is separate from content.** A `ToastSpec` (content, style,
   animation) is built from the reminder and its template; the renderer draws
   only `ToastSpec`s. The Settings preview and the real toast therefore cannot
   diverge.
6. **Templates by category.** Each category — Azan per prayer, weather per
   condition, each reminder type — has a template with title, description,
   image, accent colour, sound and style overrides, inheriting from a global
   default.
7. **Localization is resolved at display time.** Built-in text comes from
   Fluent keys in the active language; only user-written text is stored. A
   language switch never needs a data migration.
8. **One formatter for dates and numbers.** `ot_i18n::format_datetime(dt,
   prefs)` is the only function that formats a date or a time. The toast, the
   tray tooltip and the speech service all call it. It takes no language
   argument: the words come from the active Fluent bundle, and the only
   language-dependent choice — 12- against 24-hour — is carried by
   `TimePrefs::for_language`. An earlier draft of this document gave it a
   `lang` parameter, which the implementation could only have ignored.
9. **Prayer source abstraction.** `PrayerSource` is implemented by
   `JakimSource` and `CalculatedSource`. The scheduler sees a list of prayer
   times with a source label; the fallback chain lives in one place.

## Platform isolation

Every Windows API call lives in `ot-platform`, behind a Rust interface that
names the capability rather than the API: `tray`, `autostart`, `system_theme`,
`single_instance`, `window_effects`, `supervisor`, `speech`. Another crate
reaching for the `windows` or `winreg` crate directly is a defect — it is what
would block the eventual macOS and Linux work
([01-vision-and-scope.md](01-vision-and-scope.md) non-goals).

`ot-platform` compiles its Windows paths under `#[cfg(windows)]` and provides
`unimplemented`-returning stubs elsewhere, so `cargo clippy` passes on Linux.

## Where the runtime state lives

| State | Owner | Persisted in |
|---|---|---|
| Settings (language, theme, toast style, location, prayer, startup) | `ot-core` | `config/settings.toml` |
| Reminders, categories, history, caches | `ot-store` | `data/omnitoaster.db` |
| The next fire time per reminder | `ot-sched` (in memory) | recomputed from the database on start |
| Active toasts and their animation state | `ot-toast` (in memory) | not persisted |
| Resolved strings | `ot-i18n` (in memory) | the `.ftl` files |
| Imported media | `ot-store` asset store | `assets/`, content-hashed |

[05-data-model-and-storage.md](05-data-model-and-storage.md) has the schema.
