# 11 — Prayer times and Azan

**Status:** approved, pending the Phase 5 spike · **Last updated:** 2026-11-08
**Source:** [PROPOSAL.md](../PROPOSAL.md) §7, §7.8 · **ADR:** [ADR-008](08-decisions.md)
**Owner crate:** `ot-prayer` · **Phase:** 5

This is the full spec for prayer times and Azan. There is no
`docs/specs/prayer.md` — this file is it.

## 1. Sources

| Source | Is | Used |
|---|---|---|
| **JAKIM e-Solat** | Official times for a JAKIM zone code (`SGR01`, `WLY01`, …) | Default for locations inside Malaysia |
| **Calculated** | Local calculation with `adhan` and a chosen method | Outside Malaysia, by preference, offline, or when JAKIM is unavailable |

| Item | Decision |
|---|---|
| Default source | `auto` — JAKIM inside Malaysia, Calculated otherwise |
| Manual override | Settings ▸ Prayer ▸ Source: Auto · JAKIM · Calculated |
| JAKIM while abroad | Allowed with a manually chosen Malaysian zone, and a warning that local times may differ |
| Prayers shown | Subuh, Syuruk, Zohor, Asar, Maghrib, Isyak. Imsak and Dhuha are optional and **hidden until enabled** |
| Attribution | "Source: JAKIM e-Solat" with the last fetch time, in both languages |

**Why JAKIM is the default in Malaysia:** mosques, schools and government
offices follow JAKIM times. A user who sees a different time loses trust in the
app, and calculated times can differ by a few minutes at some locations.

`PrayerSource` is the trait; `JakimSource` and `CalculatedSource` implement it.
The scheduler sees a list of times plus a source label and nothing else, so the
fallback chain lives in exactly one place
([03-architecture.md](03-architecture.md) decision 8).

## 2. JAKIM data access

The e-Solat portal (`e-solat.gov.my`) publishes times by zone code; Malaysia
has about 60 zones.

| Option | Is | Used |
|---|---|---|
| **A. e-Solat JSON endpoint** (primary) | Monthly or weekly JSON for one zone | Live fetch and the monthly cache |
| **B. Official export** (fallback) | Monthly or yearly tables bundled as JSON | Offline fallback, only if the terms allow — the Phase 9 terms check |
| **C. Calculated** (last resort) | `adhan` with Fajr 20.0° and Isha 18.0° | When A and B both fail |

> **The endpoint is not a documented public API.** Its path and response shape
> may change without notice. **Phase 5 begins with a verification spike**
> against three zones, and its result is recorded as a new ADR. Until then,
> everything below about the response shape is illustrative.

Expected shape, to be verified:

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

### Parsing rules

- Times are `HH:MM` in `Asia/Kuala_Lumpur`, **regardless of the PC's time
  zone**.
- A missing or malformed field marks the **whole day** invalid. Partial data is
  never shown.
- Unknown extra fields are ignored, so an added field does not break the app.
- No `unwrap` anywhere in the parser, and a response size cap before parsing.
- The parser is tested against `tests/fixtures/jakim_sample.json`. If the live
  format changes, the fixture and the parser change together and the change is
  recorded in [GOTCHAS.md](GOTCHAS.md) under "Prayer Times and JAKIM".

## 3. Zone mapping

1. Detect the location — IP lookup or manual entry (§6).
2. Check the coordinates are inside Malaysia: a bounding box **plus** the
   country code from the geolocation result.
3. Find the nearest zone from `data/jakim_zones.csv` (zone code, state and area
   names in both languages, representative coordinates).
4. Show the result: "Detected zone: SGR01 · Gombak, Hulu Langat, Sepang,
   Petaling · Selangor. Change zone?"
5. The user confirms, or picks another zone from a searchable list.

Rules:

- Nearest-centroid mapping **can be wrong near a boundary**, so confirmation is
  required and the user can always override.
- The app stores the **zone code** as the source key for JAKIM times, never the
  coordinates.
- The zone list shows the code and both language names together, because users
  confuse codes with names.
- `data/jakim_zones.csv` changes only through a reviewed commit, with its source
  noted in [CHANGELOG.md](CHANGELOG.md).

Schema: see `jakim_zones` in
[05-data-model-and-storage.md](05-data-model-and-storage.md).

## 4. Caching and fetch schedule

- Fetch the **current month** and the **next month** for the selected zone once
  a week, and again whenever the zone changes.
- Re-fetch if the cache is older than 30 days or has no entry for today.
- HTTPS only, 10-second timeout, `User-Agent: OmniToaster/<version>`.
- Retry with backoff: 1, then 5, then 15 minutes. Then wait for the next
  scheduled check. **No polling loop.**
- Times are stored as local clock values and converted to absolute timestamps
  in `Asia/Kuala_Lumpur` when the scheduler builds its queue.

Schema: `jakim_prayer_cache` in
[05-data-model-and-storage.md](05-data-model-and-storage.md). `source` is
`"api"` or `"bundled"`.

The fetch runs on a worker thread with blocking `ureq`
([ADR-002](08-decisions.md)) and is cancellable — cancel must actually stop the
work, not merely hide it.

## 5. Bundled dataset (option B)

- Covers the current year and the next, one file per zone, embedded with
  `rust-embed`.
- Used **only** if the Phase 9 licensing review permits bundling. Otherwise the
  user triggers a download.
- Its `source` is `bundled` and the UI shows "Offline data".
- The app warns the user 30 days before the last bundled date, because bundled
  data expiring silently would stop prayer times without explanation.

## 6. Location

- Auto-detect by IP lookup, once per day, cached. **A consent prompt on first
  run.**
- Manual entry by city search or coordinates; the time zone is derived from the
  coordinates.
- Stored locally. Sent only to the prayer, weather and geolocation services,
  rounded ([06-security-and-privacy.md](06-security-and-privacy.md)).
- The detected city is always shown for confirmation, because IP geolocation is
  frequently wrong and a wrong location means wrong prayer times.

## 7. Fallback order

```
1. A fresh JAKIM cache entry for this zone and date  → use it      (source: api)
2. A live fetch succeeds                             → update cache (source: api)
3. The bundled dataset has the date                  → use it      (source: bundled)
4. Calculated (adhan, Fajr 20°, Isha 18°)            → use it, labelled "Calculated (fallback)"
5. Nothing available                                 → one error toast per day, keep the last known times
```

The source label is **always visible** in the UI. The chain is tested with the
network disabled, with HTTP 500, with bad JSON, with an empty cache, and with
the bundled data present and absent — each must produce the expected label
([.claude/rules/testing.md](../.claude/rules/testing.md)).

## 8. Calculated mode

- Method presets: Muslim World League (the default outside Malaysia), ISNA,
  Umm al-Qura, and **"Malaysia (JAKIM angles, approximate)"** with Fajr 20.0°
  and Isha 18.0°.
- Asar madhab (Shafi'i or Hanafi) is a user setting, visible and documented
  because a mismatch silently changes Asar. **The default is Shafi'i**, which
  the owner confirmed is JAKIM's convention; the spike re-confirms it.
- When the zone is in Malaysia and the source is not JAKIM, times are labelled
  **"approximate"**.

## 9. Azan behaviour

- Five daily prayers, plus optional Syuruk and Jumu'ah.
- Per prayer: an audio file the user supplies, title, description, image,
  accent colour, a style override, and a "full Azan" or "short chime" choice.
- Default Azan audio ships with the app where the licence allows
  ([01-vision-and-scope.md](01-vision-and-scope.md)); the user can replace it.
- An optional pre-prayer reminder — 10 minutes before, say.
- Times are recalculated at midnight, after a location change, after a zone
  change, and after every successful JAKIM fetch.
- **Azan takes priority** over time announcements and non-urgent toasts
  ([specs/announcements.md](specs/announcements.md)).
- An Azan toast may be persistent, capped at 10 minutes
  ([specs/toast.md](specs/toast.md)).

## 10. Localization

Prayer identifiers never change with language. English uses Fajr, Dhuhr, Asr,
Maghrib, Isha; Malay uses Subuh, Zohor, Asar, Maghrib, Isyak; Syuruk and
Jumaat/Jumu'ah follow the same rule. **Settings always shows both names** —
"Subuh (Fajr)" — so a user of either set can find the prayer. The reference
table is in [10-i18n.md](10-i18n.md).

## 11. Verification

| Check | How |
|---|---|
| Parser | `tests/fixtures/jakim_sample.json`, three zones |
| Reference times | `tests/fixtures/jakim_reference.csv` — three zones (`SGR01`, `WLY01`, `JHR01`) × five dates, captured by hand from the official portal with the capture date recorded |
| Zone mapping | State capitals map to the expected zone family; coordinates outside Malaysia are rejected for JAKIM mode |
| Time zone | JAKIM times are read as UTC+8 even when the PC is set to another zone |
| Fallback chain | Network disabled, HTTP 500, bad JSON, empty cache, bundled present and absent |
| Ramadan | Imsak appears only when enabled, with the correct time |
| Attribution | "Source: JAKIM e-Solat" present in both languages |

## 12. Open questions

1. The exact e-Solat endpoint path, parameters and response shape — **the
   Phase 5 spike**, recorded as a new ADR.
2. Whether JAKIM's published times use Shafi'i for Asar in every zone.
3. Whether the terms permit bundling the dataset (option B) — the Phase 9 terms
   check.
4. Whether a published correction can be detected cheaply enough to notify the
   user, which the owner has asked for; the weekly re-fetch is the floor.
