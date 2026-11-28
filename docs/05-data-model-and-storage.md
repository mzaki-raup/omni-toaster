# 05 — Data model and storage

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../PROPOSAL.md) §4, §9

## The data folder

Everything the app owns lives beside the executable. Nothing is written to the
registry (except the one HKCU `Run` value the user opts into), `%APPDATA%`,
`%LOCALAPPDATA%` or `%TEMP%`. `OmniToasterData/` is never committed.

```
OmniToaster/
├── OmniToaster.exe                 # supervisor (default) and worker (--worker)
├── README.md                       # English
├── README.ms.md                    # Bahasa Melayu
└── OmniToasterData/
    ├── config/
    │   └── settings.toml           # language, theme, toast style, location, prayer, startup
    ├── data/
    │   └── omnitoaster.db          # SQLite
    ├── locales/                    # optional user overrides, same file names as embedded
    │   ├── en-US/omni_toaster.ftl
    │   └── ms-MY/omni_toaster.ftl
    ├── assets/
    │   ├── images/                 # user images, content-hashed names
    │   ├── audio/
    │   │   ├── azan/
    │   │   └── chimes/
    │   └── icons/
    ├── logs/
    │   ├── app-YYYY-MM-DD.log      # daily rotation
    │   └── crash/                  # panic reports and minidumps
    └── cache/                      # weather and geolocation responses, TTL-based
```

`--data-dir <path>` replaces `OmniToasterData` for that run. **Always pass it
while developing**, so the real data folder is never touched:

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

### Safety rules

- At startup, verify the data folder is writable. If it is not — the exe is in
  `Program Files`, say — show a clear bilingual error and exit **without
  touching existing data**.
- Copy all user media into `assets/` under content-hashed names. Never
  reference a file outside the data folder.
- Store **relative** paths in the database and in `settings.toml`, so the
  folder can be moved or carried on a stick.
- Validate a locale override at load time. A broken override is ignored with a
  logged warning and the embedded text is used.

## Settings

`config/settings.toml` is human-editable TOML and is the source of truth for
configuration. The `settings` table in SQLite mirrors only what the scheduler
needs to read inside a transaction. Windows paths in TOML use single-quoted
literal strings — a double-quoted string processes `\U` and an invalid escape
fails the whole file.

What it holds so far, written by Settings ▸ General:

```toml
[general]
language = "ms-MY"   # absent means follow Windows
theme = "dark"       # system | dark | light
```

Each pane adds its own section rather than widening `[general]`. Rules the
implementation (`ot_core::settings`) keeps to:

- **A broken file never stops the app starting.** It is reported and the
  defaults are used — the file is the user's to edit, and a typo in it must
  not lock them out of the app that would let them fix it.
- **An unknown key is an error**, not something silently ignored, or a typo'd
  key would look like the app disregarding the user's edit with nothing said.
- A save goes to a temporary file in the same folder and is renamed over the
  target, so a crash mid-write cannot leave half a settings file.
- The language is read out of this file **before** the data folder has been
  validated, because the folder's own failure messages are localized. That
  read creates nothing and treats every failure as "no choice recorded".

## Schema

```
settings          (key TEXT PK, value TEXT)                -- mirror of settings.toml where needed
location          (id, name, lat, lon, tz, country_code, source, updated_at)
category          (id, kind, prayer_key | weather_key | type_key,   -- keys, not text
                   custom_title, custom_body,              -- NULL = use the built-in default
                   authored_lang,                          -- 'en-US' | 'ms-MY' | NULL
                   image_id, accent_color, sound_id,
                   style_json, animation_json, enabled)
reminder          (id, category_id, title, body, trigger_type, trigger_json,
                   recurrence_json, next_fire_at, status, created_at, updated_at,
                   authored_lang)                          -- 'en-US' | 'ms-MY'; instants are UTC text
snooze_log        (id, reminder_id, snoozed_until, count)   -- count: the snooze's place in its fire's chain
notification_log  (id, reminder_id, fired_at, action, source)
asset             (id, sha256 PK, kind, relative_path, bytes, created_at,
                   label)                                   -- the name the user gave it in the image library
weather_cache     (location_id, fetched_at, payload_json)
jakim_prayer_cache (zone_code, date, hijri, imsak, fajr, syuruk, dhuha,
                    dhuhr, asr, maghrib, isha, fetched_at, source,
                    PRIMARY KEY (zone_code, date))
```

`location` as built (FEAT-0056, `ot_store::location`): one row for each use, `id = 1` for
the place Prayer uses and `id = 2` for the one Weather uses (FEAT-0072, migration 009;
the table has no constraint on `id`, so no column was added). The weather place starts
as a copy of the prayer place and is independent after that; the detection below and the
manual place under Prayer touch row 1 only, and `weather_cache` belongs to row 2. In
row 1, `source`
is `ip` or `manual`; `lat` and `lon` are already rounded to two decimals; `tz` is
an IANA zone name; `updated_at` is when it was found, which makes the row the
cache of the daily IP lookup. A manual place replaces any detected one, and a
detection never replaces a manual one. Whether the user agreed to the lookup is
`[location] ip_consent` in `settings.toml`, not in the database.

`jakim_prayer_cache` as built (FEAT-0053, `ot_store::prayer`): `zone_code` is
upper case; `date` is `YYYY-MM-DD`; the eight times are `HH:MM:SS` Malaysian
clock values; `fetched_at` is the UTC instant the row was last written; `source`
is `api` or `bundled`, enforced by the table. A fetch **replaces** a day it
already holds. A cached day is *fresh* while it has a row for today and that row
was written under 30 days before now (a clock set back counts as fresh).

## Migrations

Numbered SQL files, tracked with `PRAGMA user_version`. A migration is never
edited once it has shipped; a correction is a new migration.

| File | Phase | Contents |
|---|---|---|
| `001_init.sql` | 0 | `settings`, `location`, `category`, `reminder`, `snooze_log`, `notification_log`, `asset`, `weather_cache` |
| `002_asset_label.sql` | 2 | `asset.label`, the name a picture has in the image library |
| `003_locale_keys.sql` | 4 | the `category` key columns (one template per key), `authored_lang` on `category` and `reminder`, and a template row for each built-in reminder type key |
| `004_jakim.sql` | 5 | `jakim_prayer_cache`. The zone table is embedded data — `data/jakim_zones.csv` and `data/jakim_zone_points.csv` — not a database table (FEAT-0050) |
| `009_weather_place.sql` | 6 | the weather place (`location` row 2), copied from the prayer place if there is one, and the cached weather moved to it. A fresh database gets row 2 when the first prayer place is saved (FEAT-0072) |

| `010_template_icon.sql` | 6 | `category.icon_id`, a template's header icon from the Icons library (FEAT-0088); NULL is the app's own icon. It references `asset` like `image_id` and `sound_id`, so a removal clears it and "in use" sees it |

Every migration is tested on both a fresh database and an existing one
([.claude/rules/testing.md](../.claude/rules/testing.md)).

## Rules

- **Built-in keys are stable identifiers.** `prayer-fajr`, `weather-rain`,
  `reminder-fatigue-eye` and their kind never change with the UI language.
  Renaming one requires a migration.
- **Translated built-in text is never stored.** It is resolved from Fluent at
  display time, so a language switch needs no data migration.
- **User-written text is stored exactly as typed**, with `authored_lang`
  recorded. The app never translates a user's title or body.
- **All SQL is parameterized.** No statement is built by string concatenation.
- **The asset store is content-addressed.** The same image imported twice is
  stored once; deleting a category does not delete an asset another category
  still references.
- **Caches are disposable.** Deleting `cache/` must only cost a refetch, never
  correctness. The prayer cache is the one exception: it is what makes the app
  work offline, so it is kept in the database, not in `cache/`.

## Retention

| Data | Kept |
|---|---|
| Logs | the last 14 days, rotated daily |
| Crash reports | the last 20 |
| `notification_log` | 90 days, then pruned on start |
| `weather_cache` | until the TTL (two hours) passes it is fresh; after that it is still shown, marked stale, until a fetch overwrites it. Emptied when the saved place moves |
| `jakim_prayer_cache` | the current and next month, refreshed weekly ([11-prayer-and-azan.md](11-prayer-and-azan.md) §4) |
