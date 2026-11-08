# OmniToaster: Project Proposal

**Version:** 1.0 (Reviewed — stakeholder answers in §16 are decided), 2026-11-08
**Target platform:** Windows 10 (1809+) and Windows 11
**Language:** Rust (stable toolchain)
**UI languages:** English (`en-US`) and Bahasa Melayu (`ms-MY`)
**Prayer sources:** JAKIM e-Solat (official, default in Malaysia) and calculated times (`adhan` crate)

---

## 1. Executive Summary

OmniToaster is a portable Windows desktop application that delivers customizable, Windows-style toast notifications for reminders, prayer times (Azan), weather changes, and time announcements. It runs in the system tray, starts with Windows, recovers from crashes, and stores all of its data in an `OmniToasterData` folder next to the executable.

The app is fully bilingual in English and Bahasa Melayu, and the language can be switched at runtime. For prayer times, it uses JAKIM e-Solat as the default source inside Malaysia, so times match the mosques, schools, and government offices that users follow. Calculated times (`adhan`) are available as an offline and overseas option.

The recommended UI framework is **eframe/egui**, with Slint as a documented fallback. The work is split into 10 phases (0 to 9). Each phase is verified, checklisted, changelogged, and committed before the next one starts.

---

## 2. Goals and Non-Goals

### Goals
- Toast notifications that look and behave like Windows toasts, with broad customization.
- Full English and Malay support for all user-visible text, dates, prayer names, and spoken announcements.
- Reliable scheduling for many reminder types, including Azan and weather.
- Prayer times from JAKIM for Malaysian locations, with a calculated fallback.
- Portable distribution: one folder, no installer required.
- Self-recovery: auto-start at login, auto-restart after a crash, and persistent local debug logs.
- Dark theme by default, with System, Dark, and Light options.
- An in-app updater: the app checks a signed release manifest, tells the user a new version exists, and replaces itself in place without losing `OmniToasterData/`.

### Non-Goals (Initial Release)
- Replacing or hooking into the native Windows Action Center.
- macOS and Linux support. Platform code is isolated so these can be added later.
- Cloud sync, user accounts, or telemetry.
- Built-in sound or image libraries beyond the default Azan audio and images that ship with the app (subject to the licensing review in the Phase 9 terms check, §13).
- Jawi (Arabic-based Malay) script. This is deferred to a later version.
- Languages other than English and Malay.

---

## 3. Technology Stack

| Concern | Recommendation | Alternatives | Rationale |
|---|---|---|---|
| UI framework | **eframe / egui** | Slint, Iced | Immediate mode suits custom animations and painted toasts. Multi-viewport support lets toasts run in separate windows from settings. Slint is the fallback if a more polished retained-mode settings UI is preferred. Iced has weak transparent-window support on Windows. |
| Window effects | `window-vibrancy` + `windows` crate | Direct DWM calls | Acrylic, Mica, and blur for the glass look. |
| Tray icon and menu | `tray-icon` + `muda` | `trayicon` | Maintained by the Tauri team, works with winit and eframe. |
| Single instance | Named mutex (`windows` crate) | `single-instance` | Prevents duplicate tray icons and scheduler runs. |
| Auto-start | HKCU `Run` registry key (`winreg`) | Task Scheduler | Per-user, no admin rights. The registry value is refreshed on each launch so a moved portable folder still starts. |
| Theme detection | Registry `AppsUseLightTheme` | `dark-light` crate | Reads the Windows light and dark setting for System mode. |
| Localization | **Project Fluent** via `i18n-embed`, `i18n-embed-fl`, `fluent-bundle`, `unic-langid` | `rust-i18n`, `gettext` | Translator-friendly `.ftl` files with variables and plural rules. The `fl!()` macro checks keys at compile time. |
| Embedded assets | `rust-embed` | External files only | Keeps the portable exe self-contained. |
| Locale detection | `GetUserDefaultLocaleName` (`windows` crate) | `sys-locale` | Native and reliable on Windows. |
| Storage | `rusqlite` (SQLite) + TOML for settings | JSON, sled | SQLite handles reminders, history, and caches with transactions. TOML is human-editable. |
| Logging | `tracing` + `tracing-appender` | `log` + `flexi_logger` | Structured logs with daily rotation. |
| Crash capture | `std::panic::set_hook` + optional minidump (`minidumper`) | Sentry | Local-only, consistent with privacy goals. |
| Audio playback | `rodio` | `kira` | Simple and reliable for Azan clips and chimes. |
| Text-to-speech | Windows `SpeechSynthesizer` via `windows` crate | `tts` crate | Uses installed Windows voices with no extra download. Voices are matched by language tag. |
| Calculated prayer times | `adhan` crate | Custom calculation | Standard methods (MWL, ISNA, Umm al-Qura, and so on) plus custom angles for the Malaysia preset. |
| Official prayer times | JAKIM e-Solat JSON (verified in Phase 5 spike) | Manual CSV import | Official times for Malaysian zones. See §7. |
| Weather | Open-Meteo API | OpenWeatherMap | No API key required. |
| Geolocation | IP lookup (ip-api.com or ipapi.co) with manual override | OS Location API | IP lookup needs no permissions. The OS API can be added later. |
| Images | `image` crate | `zune-image` | Decode and resize to thumbnails. |
| Scheduling | Dedicated scheduler thread with a priority queue | `tokio-cron-scheduler` | Full control over DST, snooze, sleep and resume, and clock changes. |
| HTTP | `ureq` (blocking) | `reqwest` | Lightweight for periodic API calls. |
| Serialization | `serde` | | Standard. |
| Errors | `thiserror` (libraries), `anyhow` (application) | | Standard. |
| Date and time | `chrono` + `chrono-tz` | `time` | Arithmetic and timezone handling. Malay names come from static tables. |
| CI/CD | **Forgejo Actions** on the owner's own Windows machine (`.forgejo/workflows/`), with GitHub Actions (`.github/workflows/`, `windows-latest`) on the public mirror | A single hosted runner | Forgejo holds version authority and publishes the release; GitHub is a read-only mirror that runs the same checks for contributors. Both run fmt, clippy, tests, lints, and release builds. Because the Forgejo runner *is* the development machine, every cargo step there is bounded with `-j 4`. |
| In-app updater | Signed release manifest (`manifest.json`) + rsign signature, checked over HTTPS; self-replace on restart | `cargo-dist` self-update, winget | Keeps the portable single-exe model: no installer, no admin rights, `OmniToasterData/` untouched. A Phase 9 deliverable. |

**Key Cargo additions for v1.0:** `i18n-embed` (with `fluent-system-ui`), `i18n-embed-fl`, `rust-embed`, `unic-langid`, `chrono-tz`, `adhan`, `ureq`, `rusqlite`, `rodio`, `tray-icon`, `muda`, `window-vibrancy`, `winreg`, `image`, `windows`.

---

## 4. Portable Mode and Data Layout

The executable must not write outside its own folder. This keeps the app portable and avoids permission problems.

```
OmniToaster/
├── OmniToaster.exe                 # Supervisor (default) and worker (--worker)
├── README.md                       # English
├── README.ms.md                    # Bahasa Melayu
└── OmniToasterData/
    ├── config/
    │   └── settings.toml           # Language, theme, toast style, location, prayer, startup
    ├── data/
    │   └── omnitoaster.db          # SQLite: reminders, categories, caches, history
    ├── locales/                    # Optional user overrides (same file names as embedded)
    │   ├── en-US/omni_toaster.ftl
    │   └── ms-MY/omni_toaster.ftl
    ├── assets/
    │   ├── images/                 # User images, copied in with content-hashed names
    │   ├── audio/
    │   │   ├── azan/               # Per-prayer audio
    │   │   └── chimes/             # Custom alert sounds
    │   └── icons/
    ├── logs/
    │   ├── app-YYYY-MM-DD.log      # Daily rotating log
    │   └── crash/                  # Panic reports and minidumps
    └── cache/                      # Weather and geolocation responses (TTL-based)
```

Source repository layout (developer view):

```
omni-toaster/
├── Cargo.toml                      # Workspace root; [workspace.package] version lives here
├── crates/
│   ├── omnitoaster/                # The binary: OmniToaster.exe
│   ├── ot-core/                    # Settings, paths, event bus, errors, logging, panic hook
│   ├── ot-i18n/                    # Fluent bundles, locale detection, the date/number formatter
│   ├── ot-store/                   # SQLite, migrations, asset store
│   ├── ot-platform/                # All Windows-only code: tray, autostart, theme, mutex, vibrancy, supervisor
│   ├── ot-ui/                      # Shared egui widgets, theme, settings window
│   ├── ot-toast/                   # ToastSpec, renderer, animations
│   ├── ot-sched/                   # Scheduler thread, recurrence, snooze
│   ├── ot-prayer/                  # PrayerSource trait, JakimSource, CalculatedSource, zone table
│   ├── ot-weather/                 # Open-Meteo
│   └── ot-media/                   # Audio (rodio), TTS (Windows SpeechSynthesizer)
├── locales/
│   ├── en-US/omni_toaster.ftl
│   └── ms-MY/omni_toaster.ftl
├── assets/prayer/jakim/            # Bundled JAKIM dataset (if licensing allows; Phase 9 terms check, §13)
├── data/jakim_zones.csv            # Zone table (reviewed commits only)
├── tests/
│   ├── fixtures/jakim_sample.json
│   └── fixtures/jakim_reference.csv
├── docs/                           # 01–11 specs, specs/, CHECKLIST, CHANGELOG, TASKS, GOTCHAS
├── .claude/rules/                  # Agent-facing rules, one file per area
├── CLAUDE.md
└── README.md
```

**Safety rules:**
- At startup, verify that `OmniToasterData` is writable. If not (for example, the exe sits in `Program Files`), show a clear bilingual error and exit without touching existing data.
- Copy all user media into `assets/` with content-hashed names. Never reference files outside the data folder.
- Store relative paths in the database and settings file so the folder can be moved.
- Locale overrides are validated at load time. A broken override file is ignored with a logged warning, and the embedded text is used.

---

## 5. Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                        OmniToaster                                    │
│                                                                        │
│  ┌────────────┐  ┌─────────────┐  ┌──────────────────────┐             │
│  │ Tray & Menu│  │ Settings UI │  │   Toast Renderer     │             │
│  │ (tray-icon)│  │ (egui)      │  │ (egui viewport,      │             │
│  └─────┬──────┘  └──────┬──────┘  │  animations, styles) │             │
│        │                │         └──────────┬───────────┘             │
│        └───────┬────────┴────────────────────┘                         │
│                ▼                                                       │
│        ┌──────────────┐        ┌───────────────────────────────────┐   │
│        │  App Core    │◄──────►│  Event Bus (crossbeam / channels) │   │
│        │  (state)     │        └───────────────────────────────────┘   │
│        └──────┬───────┘                                                │
│   ┌───────────┼────────────┬──────────────┬───────────┬──────────┐     │
│   ▼           ▼            ▼              ▼           ▼          ▼     │
│ Scheduler  Prayer Svc   Weather Svc   Speech/Audio  Startup    i18n    │
│ (reminders) (JAKIM +    (Open-Meteo)  (rodio, SAPI) (registry) (Fluent)│
│   │         calculated)      │                                         │
│   │           │              │                                         │
│   └───────────┴──────┬───────┘                                         │
│                      ▼                                                 │
│      ┌────────────────────────────┐   ┌───────────────────────────┐    │
│      │ Storage (SQLite + TOML)    │   │ Logging & Crash Reports   │    │
│      └────────────────────────────┘   └───────────────────────────┘    │
│                                                                        │
│   ┌──────────────────────────────┐                                     │
│   │ Date/Time Formatter (shared) │                                     │
│   └──────────────────────────────┘                                     │
│ Supervisor (watchdog): launches worker, restarts on crash with backoff │
└────────────────────────────────────────────────────────────────────────┘
```

### Key Design Decisions
1. **Supervisor and worker processes.** `OmniToaster.exe` runs as a supervisor by default. It spawns a worker with `--worker`, watches its exit status, records crash information, and restarts it with exponential backoff. After 5 crashes within 10 minutes, it stops restarting and shows a bilingual notification with the log path. This handles failures that `catch_unwind` cannot, such as GPU driver faults.
2. **Event-driven core.** Modules communicate through channels. UI, scheduler, and services do not call each other directly.
3. **Scheduler is the single source of time.** It stores the next fire time for each reminder, recalculates after DST changes, system resume, or clock changes, and never relies on long sleeps.
4. **Toast rendering is separate from content.** A `ToastSpec` (content, style, animation) is built from the reminder and template. The renderer only draws `ToastSpec`s. The settings preview and the real toast are therefore identical.
5. **Templates by category.** Each type (Azan per prayer, weather per condition, each reminder type) has a template with title, description, image, accent color, sound, and style overrides, inheriting from a global default.
6. **Localization is resolved at display time.** Built-in text comes from Fluent keys in the active language. Only user-written text is stored in the database. A language switch never needs a data migration.
7. **Single formatter for dates and numbers.** `format_datetime(dt, lang, prefs)` is the only function that formats dates or times. The toast, tray tooltip, and speech service all use it.
8. **Prayer source abstraction.** A `PrayerSource` trait is implemented by `JakimSource` and `CalculatedSource`. The scheduler sees only a list of prayer times with a source label. The fallback chain (§7.6) lives in one place.

---

## 6. Localization (English and Bahasa Melayu)

### 6.1 Language Policy

| Item | Decision |
|---|---|
| Supported languages | `en-US` (English), `ms-MY` (Bahasa Melayu) |
| Default | `system`: follows the Windows display language. `ms-*` maps to Malay. All others map to English. |
| Manual override | Settings > General > Language: System, English, Bahasa Melayu |
| Switching | Immediate, no restart. Tray menu and open windows re-render within one second. |
| Fallback | A missing Malay key falls back to English and logs a warning. |
| Spelling standard | Dewan Bahasa dan Pustaka (DBP) spelling and terminology |
| Malay variant | Malaysian Malay (`ms-MY`) |
| Date format (Malay) | `d/m/yyyy`, 24-hour by default, Malay day and month names |
| Date format (English) | Follows the Windows regional setting (`m/d/yyyy` or `d/m/yyyy`), 12-hour or 24-hour by user choice |
| Currency | Default `RM` in Malay mode and `MYR` in English mode. The user can set any code. |
| User-written content | Stored exactly as typed. The app never translates user titles or descriptions. |
| Time zone | Malaysia (`Asia/Kuala_Lumpur`, UTC+8) is the default for JAKIM data regardless of the PC's time zone. Reminders use the PC's local time zone unless the user sets otherwise. |

### 6.2 Locale File Format

Keys are lowercase, hyphenated, and grouped by area: `tray-`, `toast-`, `settings-`, `prayer-`, `weather-`, `reminder-`, `announce-`, `error-`, `format-`.

```ftl
# locales/en-US/omni_toaster.ftl
tray-menu-open-settings = Open Settings
tray-menu-pause-30 = Pause all notifications for 30 minutes
tray-menu-quit = Quit

toast-action-snooze = Snooze
toast-action-dismiss = Dismiss
toast-snoozed-until = Snoozed until { $time }

prayer-fajr = Fajr
prayer-dhuhr = Dhuhr
prayer-asr = Asr
prayer-maghrib = Maghrib
prayer-isha = Isha
prayer-sunrise = Sunrise
prayer-jumuah = Jumu'ah

weather-rain = Rain
weather-storm = Thunderstorm
```

```ftl
# locales/ms-MY/omni_toaster.ftl
tray-menu-open-settings = Buka Tetapan
tray-menu-pause-30 = Jeda semua pemberitahuan selama 30 minit
tray-menu-quit = Keluar

toast-action-snooze = Tunda
toast-action-dismiss = Tutup
toast-snoozed-until = Ditunda sehingga { $time }

prayer-fajr = Subuh
prayer-dhuhr = Zohor
prayer-asr = Asar
prayer-maghrib = Maghrib
prayer-isha = Isyak
prayer-sunrise = Syuruk
prayer-jumuah = Jumaat

weather-rain = Hujan
weather-storm = Ribut petir
```

Translator context comments are placed above each key in the `.ftl` file.

### 6.3 Localization Rules
- Every user-visible string uses a Fluent key. Hard-coded user-facing strings are rejected in review and by the hard-coded string lint in CI (§12).
- Never concatenate translated fragments in code. Use Fluent variables so word order can change per language.
- Never format dates or numbers manually. Use the shared formatter.
- Every new key is added to **both** `.ftl` files in the same commit.
- Malay text may be up to 30 percent wider than English. Layouts are tested with the longest Malay strings.

### 6.4 Localized Content by Area
- **Settings UI, tray menu, toast built-in labels, validation messages, empty states:** fully localized.
- **Prayer names:** English uses Fajr, Dhuhr, Asr, Maghrib, Isha. Malay uses Subuh, Zohor, Asar, Maghrib, Isyak. Syuruk and Jumaat/Jumu'ah follow the same rule. Settings always show both names (for example, "Subuh (Fajr)") so users of either set can find the prayer. Prayer identifiers never change with language.
- **Weather:** Open-Meteo weather codes map to Fluent keys. No API text is displayed. Temperature uses °C in both languages.
- **Templates:** Built-in default title and body exist in both languages. The template editor provides "Use default text" to restore the built-in text in the current language.
- **Speech:** Generated from Fluent templates, for example "Sekarang pukul { $hour } { $minute }."
- **Documentation:** `README.md` (English) and `README.ms.md` (Bahasa Melayu), each linking to the other. Developer documents stay in English.

### 6.5 Speech Voice Selection
1. Use a voice matching the active language tag (`ms-MY` or `en-US`).
2. If none, use any voice in the other language and show a notice.
3. If no voice exists, play the chime only and show a setup hint with the Windows path (Settings > Time & Language > Speech).
4. Voices are detected from `SpeechSynthesizer.AllVoices`, never by hard-coded name.
5. Failures are never silent. Each fallback is logged and visible in the settings status line.

### 6.6 Translation Process
- The English catalogue is the source of truth. Translators work from the English keys with context comments.
- Malay translations are reviewed by a named native speaker before Phase 9; the sign-off is a Phase 9 exit criterion (§13).
- A translation change that alters meaning is recorded in `GOTCHAS.md`. Every translation change is recorded in `CHANGELOG.md`.

---

## 7. Prayer Schedule

### 7.1 Prayer Sources

| Source | Description | Use |
|---|---|---|
| **JAKIM e-Solat** | Official times for a JAKIM zone code (for example, `SGR01`, `WLY01`) | Default for locations inside Malaysia |
| **Calculated** | Local calculation with `adhan` and a chosen method | Outside Malaysia, offline preference, or when JAKIM is unavailable |

**Source policy:**

| Item | Decision |
|---|---|
| Default source | `auto`: JAKIM inside Malaysia, Calculated otherwise |
| Manual override | Settings > Prayer > Source: Auto, JAKIM, Calculated |
| Malaysia outside use | JAKIM mode can be chosen manually for a Malaysian zone while abroad, with a warning that local times may differ |
| Shown prayers | Subuh, Syuruk, Zohor, Asar, Maghrib, Isyak. Imsak and Dhuha are optional. |
| Attribution | "Source: JAKIM e-Solat" with the last fetch time, in both languages |

**Why JAKIM is default in Malaysia:** Mosques, schools, and government offices follow JAKIM times. Users who see a different time will lose trust, and calculated times may differ by a few minutes at some locations.

### 7.2 JAKIM Data Access

The e-Solat portal (`e-solat.gov.my`) publishes times by zone code. Malaysia has about 60 zones.

| Option | Description | Use |
|---|---|---|
| **A. e-Solat JSON endpoint** (primary) | Monthly or weekly JSON for one zone | Live fetch and monthly cache |
| **B. Official export** (fallback) | Monthly or yearly tables bundled as JSON | Offline fallback, only if terms allow (Phase 9 terms check, §13) |
| **C. Calculated** (last resort) | `adhan` with Fajr 20.0° and Isha 18.0°, plus JAKIM precautionary offsets if confirmed | When A and B fail |

> **Important:** The e-Solat JSON endpoint is not a formally documented public API. Its path and response format may change without notice. Phase 5 begins with a verification spike (§13). The implementation treats the endpoint as unstable: every field is validated, missing keys are handled, and a format change never crashes the app.

**Expected response shape (illustrative, to be verified in the spike):**

```json
{
  "zone": "SGR01",
  "prayerTime": [
    {
      "hijri": "1447-07-01",
      "date": "01-Jan-2026",
      "day": "Thursday",
      "imsak": "05:47",
      "fajr": "06:00",
      "syuruk": "07:12",
      "dhuha": "07:36",
      "dhuhr": "13:19",
      "asr": "16:35",
      "maghrib": "19:24",
      "isha": "20:36"
    }
  ]
}
```

**Parsing rules:**
- Times are `HH:MM` in `Asia/Kuala_Lumpur`.
- A missing or malformed field marks the whole day invalid. Partial data is never shown.
- Unknown extra fields are ignored.
- The parser is tested against `tests/fixtures/jakim_sample.json`. If the live format changes, the fixture and parser are updated together and the change is recorded in `GOTCHAS.md`.

### 7.3 Zone Mapping

1. Detect location (IP lookup or manual entry, §8.7).
2. Check that the coordinates are inside Malaysia (bounding box plus the country code from the geolocation result).
3. Find the nearest zone from `data/jakim_zones.csv` (zone code, state and area names in both languages, representative coordinates).
4. Show the result on the setup screen: "Detected zone: SGR01 · Gombak, Hulu Langat, Sepang, Petaling · Selangor. Change zone?"
5. The user confirms or picks another zone from a searchable list.

**Rules:**
- Nearest-centroid mapping can be wrong near zone boundaries, so confirmation is required and the user can always override.
- The app stores the zone code as the source key for JAKIM times, not coordinates.
- The zone table changes only through a reviewed commit with its source noted in `CHANGELOG.md`.

**Zone table schema:**

```
jakim_zones (zone_code TEXT PK, state_en TEXT, state_ms TEXT,
             area_en TEXT, area_ms TEXT, lat REAL, lon REAL)
```

### 7.4 Caching and Fetch Schedule
- Fetch the **current month** and the **next month** for the selected zone once a week, and again when the zone changes.
- Re-fetch if the cache is older than 30 days or has no entry for today.
- HTTPS only, 10-second timeout, User-Agent `OmniToaster/<version>`.
- Retry with backoff: 1, 5, then 15 minutes. Then wait for the next scheduled check.
- No polling loop.

**Cache table:**

```
jakim_prayer_cache (zone_code TEXT, date TEXT,            -- ISO "2026-01-01"
                    hijri TEXT, imsak TEXT, fajr TEXT, syuruk TEXT, dhuha TEXT,
                    dhuhr TEXT, asr TEXT, maghrib TEXT, isha TEXT,
                    fetched_at TEXT, source TEXT,         -- "api" | "bundled"
                    PRIMARY KEY (zone_code, date))
```

Times are stored as local clock values and converted to absolute timestamps in `Asia/Kuala_Lumpur` when the scheduler builds its queue.

### 7.5 Bundled Dataset (Option B)
- Covers the current year and the next year, one file per zone, embedded with `rust-embed`.
- Used only if the licensing review (the Phase 9 terms check, §13) permits bundling. Otherwise, the user triggers a download.
- Its `source` value is `bundled`, and the UI shows "Offline data".
- The app warns the user 30 days before the last bundled date.

### 7.6 Fallback Order

```
1. Fresh JAKIM cache for zone and date   -> use it          (source: api)
2. Live fetch succeeds                   -> update cache    (source: api)
3. Bundled JAKIM dataset has the date    -> use it          (source: bundled)
4. Calculated (adhan, Fajr 20°, Isha 18°) -> use it, label "Calculated (fallback)"
5. Nothing available                     -> one error toast per day, keep last known times
```

### 7.7 Calculated Mode
- Method presets: Muslim World League (default outside Malaysia), ISNA, Umm al-Qura, and **"Malaysia (JAKIM angles, approximate)"** with Fajr 20.0° and Isha 18.0°.
- Asar method (Shafi'i or Hanafi) is a user setting. The default follows the JAKIM published convention, to be confirmed in the spike.
- When the zone is in Malaysia and the source is not JAKIM, times are labelled "approximate".

### 7.8 Azan Behavior
- Five daily prayers plus optional Syuruk and Jumu'ah.
- Per prayer: audio file (upload), title, description, image, accent color, style override, and "full Azan" or "short chime" option.
- Optional pre-prayer reminder (for example, 10 minutes before).
- Recalculated at midnight, after a location change, after a zone change, and after a successful JAKIM fetch.
- Azan takes priority over time announcements and non-urgent toasts.

---

## 8. Feature Specification

### 8.1 Theming
- Modes: **Dark (default)**, Light, System (reads the Windows setting, rechecked every 5 seconds).
- Global accent color, with optional per-category accent color.

### 8.2 Tray and Menu
- Left-click: open or focus the main window.
- Right-click menu: Open Settings, Pause All Notifications (30 min, 1 hour, until resumed), Test Toast, Snooze Last, Quit.
- Tooltip shows the next reminder or prayer, for example "Zohor · 13:19".
- All labels are localized and update immediately on language change.

### 8.3 Startup and Recovery
- "Start with Windows" option using the HKCU Run key.
- Supervisor with backoff (§5).
- Logs: daily files, 14-day retention (configurable). Crash reports contain the app version, OS build, last 200 log lines, and panic message. No personal data.

### 8.4 Toast Appearance

| Setting | Options | Limits |
|---|---|---|
| Width | Pixels | 280 to 600 px (default 360) |
| Height | Auto or fixed | 80 to 300 px |
| Shape | Rectangle, rounded rectangle, pill | Corner radius 0 to 40 px |
| Background | Solid color, gradient | |
| Opacity | Percent | 40% to 100% |
| Glass effect | None, Blur, Acrylic, Mica-like | Windows 10 1809+ |
| Border | Width and color | 0 to 4 px |
| Shadow | Off, soft, strong | |
| Header | Icon, title, app name, timestamp | Each toggleable |
| Body | Text (bold and italic), image | Image max 5 MB; PNG, JPG, WebP |
| Actions | Snooze, Dismiss, Open | 1 to 3 buttons |
| Position | Corner or edge, monitor selection | |
| Stacking | Max visible 1 to 5, gap | |

**Text rules:** Text boxes size to content in both languages. Long words wrap or are truncated with an ellipsis. Nothing overflows the toast.

**Image handling:** Validate by magic bytes, not extension. Decode with a pixel limit (4096 x 4096). Resize to a cached thumbnail. Reject oversize or corrupt files with a localized message.

### 8.5 Toast Animations

**In:** Slide from right, slide from bottom, Genie from bottom, pop out (scale with overshoot), drawer, appear (fade), dissolve, glow, alpha to solid.

**Out:** Mirror of the in-presets, plus "shrink to tray" for Genie.

**Settings:** Type, duration (150 to 1000 ms, default 350 ms), easing (linear, ease-out, spring). A reduced-motion option follows the Windows "Show animations" setting and switches to fade-only.

**Implementation:** Slide and pop are window position and scale updates. Dissolve and glow use painted meshes with alpha masks. Genie uses a mesh warp. Each effect has a frame-time budget and falls back to fade below 30 fps.

### 8.6 Display Duration
- Default 8 seconds, range 3 to 60 seconds.
- Persistent option for urgent types (Alarm, Azan), with a 10-minute cap.
- Hover pauses the timer.

### 8.7 Location
- Auto-detect by IP lookup, once per day, cached. A consent prompt appears on first run.
- Manual entry by city search or coordinates. Timezone is derived from coordinates.
- Stored locally. Sent only to the prayer, weather, and geolocation services.

### 8.8 Weather Notifications
- Open-Meteo data polled every 30 minutes (configurable, minimum 15 minutes).
- Triggers on condition change (clear, cloudy, rain, storm, snow, fog) or threshold (for example, a 5 °C drop within 3 hours).
- Per condition: image, title, description, accent color, sound.
- Offline: uses cached data and marks it as stale.

### 8.9 Reminder Types
Azan Prayer, Weather change, Event, Task, Anniversary, Fatigue (eye and posture breaks, hydration), Timer, Alarm, Payment, Appointment, Custom. Each type has its own template in both languages.

### 8.10 Reminder Scheduling

**Trigger options:**
- At the exact time.
- Offset before: 1 day, 2 days, 1 week, 1 hour, 15 minutes, or custom (minutes, hours, days).
- Recurrence: once, daily, weekly, monthly, yearly (anniversaries), or custom pattern.

**Snooze:** 5 minutes, 15 minutes, 1 hour, 1 day, or custom. Default maximum of 10 snoozes per reminder.

**Missed reminders:** if the PC was off or asleep, missed items are listed at next start with dismiss or snooze options. Catch-up window: 1 hour for Azan, 24 hours for others (configurable).

### 8.11 Time Announcements
- Options: every hour, every 30 minutes, every 15 minutes, start of day, end of day, custom times.
- Spoken with the selected voice in the active language. Format: 12-hour or 24-hour, optional date.
- Quiet hours (for example, 22:00 to 06:00) suppress announcements and non-urgent toasts.
- Azan always has priority.

---

## 9. Data Model (Summary)

```
settings          (key TEXT PK, value TEXT)                -- mirror of settings.toml where needed
location          (id, name, lat, lon, tz, country_code, source, updated_at)
category          (id, kind, prayer_key | weather_key | type_key,   -- keys, not text
                   custom_title, custom_body,              -- NULL = use built-in default
                   authored_lang,                          -- 'en-US' | 'ms-MY' | NULL
                   image_id, accent_color, sound_id,
                   style_json, animation_json, enabled)
reminder          (id, category_id, title, body, trigger_type, trigger_json,
                   recurrence_json, next_fire_at, status, created_at, updated_at)
snooze_log        (id, reminder_id, snoozed_until, count)
notification_log  (id, reminder_id, fired_at, action, source)
asset             (id, sha256 PK, kind, relative_path, bytes, created_at)
weather_cache     (location_id, fetched_at, payload_json)
jakim_zones       (zone_code PK, state_en, state_ms, area_en, area_ms, lat, lon)
jakim_prayer_cache (zone_code, date, hijri, imsak, fajr, syuruk, dhuha,
                    dhuhr, asr, maghrib, isha, fetched_at, source,
                    PRIMARY KEY (zone_code, date))
```

**Migrations (numbered, tracked with `PRAGMA user_version`):**
- `001_init.sql`: Phase 0 schema (settings, location, category, reminder, snooze_log, notification_log, asset, weather_cache).
- `002_locale_keys.sql`: category key columns, `authored_lang`, and built-in key identifiers.
- `003_jakim.sql`: zone table and prayer cache.

**Rules:**
- Built-in keys (for example, `prayer-fajr`, `weather-rain`) are stable identifiers. Renaming a key requires a migration.
- Translated built-in text is never stored in the database.
- A custom title or body is stored in the language it was typed in, with `authored_lang` recorded.

---

## 10. Security and Privacy Rules

- No data leaves the machine except requests to Open-Meteo and the IP geolocation service. Both are HTTPS only, and the user can disable them.
- Input validation for every file import: size limit, format check by magic bytes, and path canonicalization to prevent traversal outside `OmniToasterData`.
- Audio and image decoders run with size limits. Consider running decoding in the worker process so a malformed file cannot take down the UI.
- SQL uses parameterized statements only.
- No secrets in the repository. If an API key is later required, it is stored in the Windows Credential Manager, not in plain text.
- Release binaries are code-signed (recommended) and published with SHA-256 checksums.
- Dependencies are audited with `cargo audit` and `cargo deny` in CI.
- Crash reports stay local and are never uploaded automatically.

---

## 11. Quality and Testing

This section is the test plan. The pipeline that runs it is §12.

**Unit tests**
- Scheduler math: DST, offsets, recurrence, snooze limits.
- Prayer parsing: valid JSON, missing fields, malformed times, unknown extra fields.
- Zone mapping: state capitals map to the expected zone family; coordinates outside Malaysia are rejected for JAKIM mode.
- Date formatter: both languages, Malay day and month names, 12-hour and 24-hour, UTC+8.
- Speech voice selection order: matching language, other language, no voice.
- Path validation and import checks.

**Integration tests**
- Database migrations 001 to 003 on a fresh and an existing database.
- Settings round trip.
- Reminder lifecycle: create, fire, snooze, dismiss, recur.
- Fallback chain: no network, HTTP 500, bad JSON, empty cache, bundled data present or absent. Each must produce the expected source label.
- Timezone: JAKIM times are read as UTC+8 even when the PC is set to another time zone.

**Reference tests**
- For 3 zones (for example, `SGR01`, `WLY01`, `JHR01`) and 5 dates, compare stored times with official portal values captured by hand. Stored in `tests/fixtures/jakim_reference.csv` with the capture date.

**Rendering and layout tests**
- Snapshot tests for each toast shape, in English and Malay, using the longest Malay strings.
- Pseudo-locale build (about 40 percent longer strings) to detect layout breakage.

**Localization tests**
- Key parity: CI fails if `en-US` and `ms-MY` have different key sets.
- Hard-coded string lint (§12).

**Manual test checklist (each phase)**
- Animations at 60 fps on reference hardware.
- Tray behavior, multi-monitor, DPI 100 to 250 percent.
- Sleep and resume, DST change, language switching at runtime.
- Malay and English speech, with and without voices installed.
- Ramadan: Imsak shown only when enabled, with the correct time.
- Attribution "Source: JAKIM e-Solat" present in both languages.

---

## 12. Quality Gates and CI/CD

**The local gate — the definition of done for any change:**
1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `cargo test --workspace --locked -j 4`
4. `cargo deny check`
5. `cargo run -p omnitoaster -- --self-test`
6. The feature exercised in the running app with `--data-dir ./tmp-data`

`-j 4` is mandatory: an unbounded parallel build on this machine exhausts the
paging file and corrupts `.rmeta`/`.rlib` files.

**CI pipeline.** `.forgejo/workflows/` runs on the owner's own Windows machine
and holds version authority; `.github/workflows/` runs the same checks on the
public GitHub mirror for contributors. Because the Forgejo runner *is* the
development machine, every cargo step there is bounded with `-j 4`.

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `cargo test --workspace --locked -j 4`
4. Locale key parity check (script): fails if `en-US` and `ms-MY` have different key sets
5. Hard-coded string lint (script): fails on a user-facing string literal that is not a Fluent key
6. `cargo audit` and `cargo deny check`
7. Release build, packaging, signing, and publication of the assets below

**Release asset contract.** Every published release carries exactly these:

| Asset | What it is |
|---|---|
| `OmniToaster.exe` | The bare portable executable |
| `OmniToaster-<version>-windows-x64.zip` | The portable folder: the exe plus both READMEs |
| `manifest.json` | Version, release date, asset names, sizes and SHA-256 digests — what the in-app updater reads |
| SHA-256 checksums | One digest per asset, for manual verification |
| rsign signature | A detached signature over `manifest.json`, verified by the updater before it replaces the exe |

**The in-app updater** (Phase 9) fetches `manifest.json` over HTTPS, verifies the
rsign signature against a public key embedded in the exe, compares versions, and
offers the update to the user. It never updates silently, it never touches
`OmniToasterData/`, and a failed verification is reported and abandoned rather
than retried against an unsigned asset.

**Versioning:** Semantic versioning. The version lives once, in
`[workspace.package] version` in the root `Cargo.toml`, and is moved by the
release pipeline from the Conventional Commit types — never edited by hand.

**Branching:** Work happens on a short-lived branch. Push it, wait for the
Forgejo `ci.yml` run to go green, then merge to `main` and push. Never merge
red. Because the runner is this machine, do not run local cargo commands while a
CI job is running. Pushing `origin` (Forgejo) also pushes the GitHub mirror.
Each phase is merged to `main` only after its exit criteria pass.

---

## 13. Phased Delivery Plan

Each phase ends with verification, a checklist update, a changelog entry, and a commit and push (if the repository exists). No checklist item is marked Completed before verification.

| Phase | Name | Key Deliverables | Exit Criteria |
|---|---|---|---|
| **0** | Foundation | Repository, `docs/` skeleton, `CLAUDE.md`, CI skeleton, logging, panic hook, portable path resolution with writability check, Fluent setup with both `.ftl` files, `fl!()` wired in, locale detection, migration 001 | App starts in English and Malay, writes a log, key parity check passes, CI green |
| **1** | Shell | eframe settings window, tray icon and menu, single instance, theme system (System, Dark, Light; Dark default), Start with Windows, language selector, live language switch | Tray works, theme switches, autostart survives reboot, language switch updates tray and settings in under one second |
| **2** | Toast Core | Borderless transparent always-on-top toast window, shapes, radius, colors, opacity, shadow, header, text, image, stacking, DPI and multi-monitor positioning, ellipsis rules, Malay length testing | Every shape renders correctly at 100, 150, and 200 percent DPI in both languages, with no overflow |
| **3** | Glass and Animations | Blur, Acrylic, Mica-like effects; all in and out animations; reduced-motion and frame-time fallbacks | Each animation runs at 60 fps on reference hardware and falls back cleanly |
| **4** | Scheduler and Reminders | Reminder CRUD, offsets, recurrence, snooze, duration, persistent alarms, missed-reminder handling, DST-safe scheduling, localized reminder types, shared date formatter (migration 002) | Scheduling tests pass, reminders fire within 1 second of target, date format is correct for each language |
| **5** | Location, Prayer, and Azan | **Spike first:** verify the e-Solat endpoint and response for 3 zones; record results in `DECISIONS.md`. Then: zone table, JAKIM client, parser with fixtures, cache and migration 003, fallback chain, zone selection UI, calculated mode with Malaysia preset, per-prayer templates, audio playback, pre-prayer reminders, dual prayer names | Parser passes fixtures for 3 zones. Live JAKIM times match the official portal on the same dates for at least 3 zones. Fallback chain passes with network disabled. Prayer labels match the reference table in both languages. |
| **6** | Weather | Open-Meteo service, caching, condition change detection, per-condition templates, weather code to Fluent key mapping, offline stale marker | Condition change triggers a toast in simulation; offline mode shows the stale marker in both languages |
| **7** | Time Announcements | Speech synthesis, language-tag voice selection, Malay voice detection and fallback hint, schedules, quiet hours, Azan priority | Announcements fire on schedule in the selected language or fall back with a visible hint; quiet hours respected |
| **8** | Supervisor and Hardening | Watchdog supervisor, crash-loop protection, crash reports, log retention, security review of imports and external data | Forced panic is recovered and logged; crash loop stops at threshold; security review checklist complete |
| **9** | Packaging and Release | Portable zip, checksums, signing, `manifest.json`, **in-app updater** (manifest fetch, rsign signature check, version compare, user-confirmed self-replace), bilingual READMEs, bilingual user guide, bilingual screenshots and GIFs, JAKIM terms check, attribution in app and docs, Malay translation review sign-off | Clean-machine test passes in both languages; reviewer sign-off recorded; terms decision recorded; release published with the full asset set (§12); **the updater moves a clean install from the previous release to the current one, refuses a manifest whose signature does not verify, and leaves `OmniToasterData/` intact** |

**Estimated effort (single experienced Rust developer):**
- Phases 0 to 2: about 3 to 4 weeks
- Phases 3 to 5: about 6 to 7 weeks (Phase 5 includes about 1 week for the JAKIM spike and integration)
- Phases 6 to 9: about 4 to 5 weeks
- Localization overhead spread across phases: about 1 to 1.5 weeks (translation of roughly 250 to 400 strings, layout testing, voice handling)
- **Total: roughly 14 to 17 weeks**, depending on animation scope, JAKIM verification effort, and translator availability.

---

## 14. Documentation Plan

| File | Purpose | Updated When |
|---|---|---|
| `docs/CHECKLIST.md` | Every phase and sub-phase, state: Completed, In-Progress, Pending | After each verified task |
| `docs/CHANGELOG.md` | Every add, change, fix, or deployment, including translation changes | Every code, content, or deployment change |
| `docs/TASKS.md` | The work ledger: `FEAT-`, `BUG-` and `CR-` items, each under exactly one task version heading, each with its own checklist closing on Test, Gate, App and Docs boxes. Ids are allocated once and never reused or renumbered, because commits, branches and changelog lines point at them (`Refs: FEAT-0012`). A box is ticked only after that check actually ran and passed | When a feature, bug or change request is filed, and as each box is verified |
| `docs/GOTCHAS.md` | Issues, resolutions, lessons learned, JAKIM format changes | When a problem is found and fixed |
| `README.md` (root) | What the app is, notification types, features, requirements, build commands, restrictions, screenshots and flow GIFs, licence, JAKIM attribution | When features, requirements, or build steps change |
| `CLAUDE.md` (root) | General behavioural guidelines for the agent, plus the map of everything below | When the document map or the gate changes |

The first three divide the work between them and never restate each other:
`TASKS.md` is **what is to be done**, `CHANGELOG.md` is **what was done**, and
`CHECKLIST.md` is **how far the phases have got**. `TASKS.md`'s own Format
section is authoritative for the item shape.

**The specifications.** One numbered document per area, written from this
proposal and authoritative once written:

| File | Covers |
|---|---|
| `docs/01-vision-and-scope.md` | Scope, personas, non-goals (§1, §2) |
| `docs/02-features.md` | Features with priority and effort (§8) |
| `docs/03-architecture.md` | Crate layout, event bus, scheduler thread, supervisor (§5) |
| `docs/04-ui-design.md` | Settings window, tray, toast layout, theming (§8.1–8.6) |
| `docs/05-data-model-and-storage.md` | `OmniToasterData/`, settings file, SQLite schema, migrations, asset store (§4, §9) |
| `docs/06-security-and-privacy.md` | Threat model, import validation, what leaves the machine (§10) |
| `docs/07-roadmap.md` | Phases 0–9 (§13) |
| `docs/08-decisions.md` | The ADR log, including the Phase 5 e-Solat spike result |
| `docs/09-ci-cd-and-releases.md` | CI, releases, the asset contract, the updater (§12) |
| `docs/10-i18n.md` | Fluent, locale files, the shared date and number formatter (§6) |
| `docs/11-prayer-and-azan.md` | JAKIM e-Solat, zones, caching, the fallback chain, calculated mode (§7) |

**Per-area specs**, one file per feature area, under `docs/specs/`: `toast.md`,
`reminders.md`, `weather.md`, `announcements.md`, `tray-and-settings.md`,
`supervisor.md`, `updater.md`. Prayer has no file here — its spec is
`docs/11-prayer-and-azan.md`.

**Agent-facing rules** live in `.claude/rules/`, one file per area, loaded by a
session only when its task touches that area: `00-index.md`, `rust.md`, `ui.md`,
`threads-and-scheduling.md`, `security.md`, `testing.md`, `i18n.md`,
`docs-and-release.md`, `build-and-run.md`, `git.md`, `workflow.md`. There is no
`docs/rules/` folder.

**`CLAUDE.md` structure, as it stands:**
```
# CLAUDE.md
## 1. Think Before Coding        general behavioural guidelines
## 2. Simplicity First
## 3. Surgical Changes
## 4. Goal-Driven Execution
## Project-specific instructions: OmniToaster
   what the app is · the rules table (.claude/rules/) · the ledger ·
   the spec map (docs/01–11, docs/specs/) · the four non-negotiables
   (the gate · branch→CI→merge · specs are requirements · Fluent keys)
```

---

## 15. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Transparent and glass windows behave differently across Windows versions | Visual inconsistency | Test matrix on Win10 and Win11; provide solid fallback |
| Advanced animations (Genie, dissolve) are expensive | Dropped frames | Frame budget, fallback to fade, keep effects optional |
| Prayer times differ between calculation methods | User trust | Show method used, allow override, validate against published tables |
| IP geolocation is inaccurate | Wrong prayer times | Prominent manual override, show detected city for confirmation |
| Scheduler drift after sleep or DST | Missed Azan | Recompute on resume, missed-reminder catch-up, tests for DST |
| Crash loop consumes resources | System slowdown | Backoff and crash-loop threshold in supervisor |
| Large user images or audio | Memory use, slow UI | Size limits, thumbnail cache, decode in worker |
| Portable folder in read-only location | App cannot save | Writability check at startup with clear message |
| TTS voice not installed | No announcements | Detect available voices, let user choose, fall back to chime |
| e-Solat endpoint changes or becomes unavailable | No live JAKIM times | Bundled dataset, calculated fallback, fixture-based parser, visible source label |
| Endpoint blocks or rate-limits the app | Fetch failures | Monthly cache, weekly refresh, backoff, no polling |
| Zone mapping picks the wrong zone near a boundary | Wrong times for some users | Confirmation step, manual override, nearest-centroid table reviewed |
| JAKIM data terms restrict bundling or redistribution | Legal risk | Terms check in Phase 9; fall back to user-triggered download |
| Calculated times differ from JAKIM | User distrust | Default to JAKIM in Malaysia; label approximate values |
| Asar madhab mismatch | Asar time differs | Setting visible and documented |
| Bundled data expires | Offline times stop | Warning 30 days before expiry, refresh on next online launch |
| Users confuse zone codes with names | Wrong zone chosen | Show code and both-language names together |

---

## 16. Stakeholder Questions, Answered

Every question below has been answered by the owner. The answers are decisions,
not suggestions, and the rest of this document follows them.

1. Is Windows-only confirmed for the first release, or is cross-platform needed soon? - Cross-platform needed in future.
2. Which minimum Windows version should be supported (10 1809+ or Windows 11 only)? - Windows 10 1809+
3. Should the app ship with any default Azan audio and images, or should users supply everything? (Licensing check needed for bundled media.) - Ship with any default Azan audio and images. User can add custom later.
4. What is meant by "Fatigue reminder" (eye breaks, posture, hydration, or work-hour limits)? Default schedules would differ. - All of it. User can choose in settings.
5. Is code signing available? Unsigned executables trigger SmartScreen warnings. - Not sure. If non existed, please provide.
6. Is a Git repository already set up? If so, which hosting service and branch strategy should be used? - Both Github and Self Host Forgejo.
7. Should the app be published publicly, or is it for internal use? This affects licensing and privacy statements. - Public. Use MIT License.
8. Should the Malaysia preset be the default for all users in Malaysia, or should the app ask on first run? - Default Malaysia.
9. Is the bundled dataset (option B) acceptable under JAKIM's terms, or should users download it manually? - Download the bundle dataset (Option B).
10. Should the app show Imsak and Dhuha by default, or keep them hidden until enabled? - Keep hidden until enabled.
11. For users outside Malaysia who want JAKIM times (for example, Malaysians abroad), should JAKIM mode be allowed with a manual zone choice? Current proposal: yes, with a warning that the zone may not match local conditions. - Proceed with current proposal.
12. Which Asar madhab does JAKIM use for the published times? This must be confirmed in the Phase 5 spike. - Shafi'i
13. Should the app notify the user when JAKIM publishes a schedule change (for example, a correction)? Current proposal: no, but the app re-fetches on its weekly cycle. - Yes.

---

## 17. Next Steps

1. Add the JAKIM zone table and reference fixtures to the repository in Phase 5.
2. Create the repository and `docs/` skeleton (Phase 0). If files existed, modify and amend.
3. Begin Phase 0 with a verification step: app starts, writes to `OmniToasterData/logs`, and CI passes before any checklist item is marked Completed.

---