# Spec — Weather notifications

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../../PROPOSAL.md) §8.8 · **ADR:** [ADR-009](../08-decisions.md)
**Owner crates:** `ot-weather`, `ot-store` · **Phase:** 6

## 1. Purpose

Tell the user when the weather **changes** — not what it is. A weather widget
is not the goal; a toast when rain is starting is.

## 2. User stories

- As **Mei**, I want to know rain is coming before I leave, so I take an
  umbrella.
- As **anyone offline**, I want to be told the weather I'm seeing is stale,
  rather than shown old data as if it were current.

## 3. Source

Open-Meteo. No API key, which is why it was chosen — the app holds no
credentials ([ADR-011](../08-decisions.md)).

- Polled every **30 minutes**, configurable, **minimum 15 minutes** (the service's own values change in 15-minute steps, ADR-023).
- HTTPS, a timeout, a response size cap, and retry with backoff. No polling
  loop tighter than the configured interval.
- Coordinates are **rounded before they are sent**
  ([06-security-and-privacy.md](../06-security-and-privacy.md)).
- The request runs on a worker thread with blocking `ureq`, and is cancellable.

## 4. Triggers

| Trigger | Fires when |
|---|---|
| Condition change | The condition class changes: clear · cloudy · rain · storm · snow · fog |
| Threshold | A temperature move beyond a set delta in a window — a 5 °C drop within 3 hours, say |

A change fires **once**. Flapping between two classes must not produce a toast
per poll: a class change is only accepted when it persists across two
consecutive polls, and the last notified class is what is compared against, not
the last observed one.

## 5. Weather codes and text

Open-Meteo's numeric weather codes map to Fluent keys under the `weather-`
prefix. **No API text is ever displayed** — every visible word is ours, which
is the only way both languages are covered ([10-i18n.md](../10-i18n.md)).

Temperature is °C in both languages.

A code we do not recognize maps to a generic key and is logged, so an added
code is a log line rather than an empty toast.

## 6. Templates

Per condition: image, title, description, accent colour, sound
([specs/toast.md](toast.md) §8). Built-in default text in both languages, with
"Use default text" to restore it.

## 7. Offline and staleness

- The last successful response is cached in `weather_cache` with its fetch
  time.
- Offline, the cached data is used and **marked stale** — the UI says how old
  it is, in both languages.
- Stale data never triggers a change notification. A change is only ever
  derived from a fresh response.

## 8. Data

`weather_cache (location_id, fetched_at, payload_json)` and `location` — see
[05-data-model-and-storage.md](../05-data-model-and-storage.md). The cache is
disposable: deleting it costs a refetch, never correctness.

## 9. Settings

Settings ▸ Weather: enable, location (shared with prayer — see
[11-prayer-and-azan.md](../11-prayer-and-azan.md) §6), poll interval, which
condition changes notify, which thresholds, and the per-condition templates.
Turning weather off stops all outbound weather requests.

## 10. Fluent keys

Prefix `weather-`: one key per condition class, the staleness message, the
threshold messages, the error states, the empty state. Dates and temperatures
go through `ot_i18n`.

## 11. Verification

| Check | Asserts |
|---|---|
| Code mapping | every documented Open-Meteo code maps to a key; an unknown code maps to the generic key and logs |
| Change detection | a class change fires once, not per poll; flapping does not notify |
| Threshold | the delta-in-window rule fires at the boundary and not inside it |
| Offline | cached data is shown with the stale marker, in both languages, and does not notify |
| Parsing | a truncated, empty or malformed response is handled without a panic |
| Cache | round-trip through `weather_cache`, and a TTL expiry |

Then in the running app: simulate a condition change and watch the toast
appear; disconnect the network and confirm the stale marker in both languages
(Phase 6's exit criteria).

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

## 12. Open questions

1. ~~Whether "two consecutive polls" is the right anti-flap rule, or whether a
   minimum time-in-class is better at a 15-minute interval.~~ **Answered
   (FEAT-0068):** two consecutive polls, but two *distinct* readings — a reading with
   the same `at` as the one before is not a second sighting, because the service
   moves in 15-minute steps and a faster poll would otherwise confirm a change with
   the same data twice. A time-in-class rule adds nothing at the 15-minute minimum,
   where two readings already mean 15 to 30 minutes.
2. Whether a severe-weather alert (as opposed to a condition change) deserves
   the Alarm priority class.

## What is implemented

- **The client and the reader (FEAT-0065, ADR-023).** `ot_weather::fetch::Client`
  asks `api.open-meteo.com/v1/forecast` for the current temperature and code and the
  last six hourly temperatures, in UTC, with the coordinates rounded to two
  decimals; `ot_weather::observation::parse` reads the answer into an
  `Observation` and refuses what is not the shape Open-Meteo sends. No polling, no
  cache, no change detection and no toast yet: those are FEAT-0066 to FEAT-0071.
  Verified live for Kuala Lumpur, Kuching and London; the fixtures are in
  `crates/ot-weather/tests/fixtures/open-meteo-2026-11-26/`.
- **Condition classes and keys (FEAT-0066).** `ot_weather::condition::classify` maps
  the 29 WMO codes Open-Meteo documents to clear (0, 1), cloudy (2, 3), fog (45, 48),
  rain (drizzle 51–57, rain 61–67, showers 80–82), snow (71–77, 85, 86) and storm
  (95–99), and any other code to `Unknown` with a warning in the log. Each class has
  a `weather-` key (`weather-clear`, `-cloudy`, `-rain`, `-storm`, `-snow`, `-fog`,
  and `weather-unknown` for the generic one) in both locale files. Nothing shows
  them yet: the pane is FEAT-0070.
- **The cache (FEAT-0067).** `weather_cache` was already in migration 001, so there
  is no migration 008. `ot_store::weather` keeps one row for the one saved place: the
  response body as the service sent it, and when it was fetched. The store never
  reads the payload; `ot-weather`'s parser does, so a damaged row is a failed parse
  and a refetch. `CachedWeather::age` and `is_stale(now, ttl)` say how old it is,
  with `DEFAULT_TTL` of two hours (four polls at the default interval); exactly the
  TTL is still fresh. Choosing a **weather place** with other coordinates empties the
  cache, because another place's weather is not this place's; choosing the same place
  again keeps it. Moving the **prayer** place no longer touches it (FEAT-0072: the two
  places are independent). The cache belongs to `location` row 2.
- **Change detection (FEAT-0068).** `ot_weather::detect::Detector` is a plain value fed
  one fresh `Observation` at a time (no clock, no I/O) that returns the `Change`s.
  The first reading is a silent baseline. A class change fires once, on its second
  consecutive distinct sighting, against the last class **notified**; a class that
  flaps and returns, or three different classes in a row, never fires. An unknown
  class is no information: it neither confirms nor breaks a change and is never
  announced. A `Threshold` (direction, delta in °C, window) fires when the biggest
  move from any hourly point inside the window reaches the delta — exactly the delta
  fires — and not again until the move falls back under it, so one cold front is one
  toast; `Threshold::default_fall()` is 5 °C in 3 hours. A reading that is not fresh
  changes nothing, and nothing is stored: the service keeps `notified()` across a
  restart (FEAT-0069).
- **The service (FEAT-0069).** `ot_ui::weather_service`: an `Engine` of one testable
  pass and a thread, started with the window, like the prayer service. **Off by
  default** (`[weather] enabled`; the Weather pane is FEAT-0070), and off means no
  outbound request at all. It polls for the weather place every `poll_minutes` (30; 15
  to 240), keeps each answer in the cache, and feeds each fresh one to the detector with
  the thresholds the settings ask for (`fall_*`, `rise_*`), keeping only changes **to**
  a class listed in `notify_classes`. The last class told is kept in the store
  (`weather.notified`), so a restart is not a first reading. A failed fetch keeps the
  cache, backs off 1, 5 and 15 minutes, and tells nothing; cached data is judged stale
  after the TTL (two hours, or two polls if longer) and is never compared. Cancel
  stops the work in flight. The window shows a change as an ordinary toast — the
  condition's name and "changed from … to …", or the temperature's move — with the
  condition's built-in picture, held back by a pause. Temperatures are whole degrees,
  because `ot-i18n` has no number formatter yet. The per-condition templates and the
  stale marker are FEAT-0070 and FEAT-0071.
- **The weather has its own place (FEAT-0072).** `location` row 2 (migration 009,
  `ot_store::location::WEATHER_ROW`): it starts as a copy of the prayer place and is then
  chosen in Settings ▸ Weather with the same two controls as Prayer — a city search
  (Open-Meteo geocoding, ADR-021) or typed coordinates — which are one shared piece,
  `ot_ui::place_picker`. Prayer's place, zone and times are not touched. Choosing another
  place empties the cache **and** clears `weather.notified` in one store call, and the
  service drops its detector when the coordinates differ from the last pass, so the new
  place starts from a fresh baseline instead of being compared with the old place's sky.
  A fetch that finishes after the place moved is not cached, shown or compared: the
  cache write is conditional on the place still being at the coordinates that were
  fetched, and the pass looks again at once. The service, the pane and the toast's place
  name all read row 2.
- **The place on the toast (FEAT-0075).** A weather toast names the weather place
  beneath its text, the same way a prayer's does (`docs/specs/toast.md` §8); the
  service reads the name with each change and the preview uses the pane's. A
  simulation has no place and so none is shown. When the weather has a place of its
  own (FEAT-0072) it is that place's name.
- **Simulation.** A debug build takes `--simulate-weather <code>`: the service then
  reads a sky that is clear once and then that code, every five seconds, with no
  setting, place or cache, so the toast can be watched without waiting for the
  weather. Not in a release.
- **Settings ▸ Weather (FEAT-0070).** `ot_ui::weather_pane`: the switch (with what
  it sends), the weather place with Change Location (the shared city search and
  coordinates, FEAT-0072), the poll interval,
  a checkbox for each class a change **to** is told about, the two temperature
  thresholds (a fall and a rise, each by 1 to 30 whole °C within 1 to 6 hours), and a
  template for each condition. The settings are `[weather]` in `settings.toml`, brought
  into range when read. A template is a `category` row with a `weather_key`
  (migration 008 seeds the six; the column dates from migration 003): a title,
  message, accent colour and picture, each empty meaning the built-in, and a sound
  that is **silent by default** (a change of weather is not an Azan) or the short
  chime — the Azan and a clip of the user's own are the Prayer's. Use Default Text
  clears the words; Preview Toast shows what a change to that condition would, from
  the same `weather_spec` the real one uses. A template applies to a change **to**
  its condition; a temperature move has none and says its built-in text. The pane
  reads and writes the store on short-lived threads, drained each frame, and credits
  Open-Meteo at the bottom (CC BY 4.0). The generic words of the template editor are
  the Prayer's keys (`prayer-template-*`, `prayer-sound-*`).
- **The stale marker (FEAT-0071).** The service sends the pane a `Snapshot` (the class,
  the temperature, when it was fetched and its TTL) after each pass, and Settings ▸
  Weather shows it under "Current Weather": fresh weather says "Updated 6 minutes ago.",
  and weather older than its TTL says "Not up to date: showing the weather from 5 hours
  ago." in the accent colour instead — offline, or whenever the last fetch is old.
  Staleness is judged when drawn, so weather that ages past its TTL between two polls
  is marked without waiting for one. The age is in minutes, hours or days from Fluent
  plurals, never formatted by hand. As §7 requires, stale data never produces a
  notification (FEAT-0068, FEAT-0069). Whole degrees, as the toast has.
- **A licence point the owner must settle** (ADR-023): the free API is
  non-commercial and wants attribution.
