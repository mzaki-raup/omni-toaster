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

1. Whether "two consecutive polls" is the right anti-flap rule, or whether a
   minimum time-in-class is better at a 15-minute interval.
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
  TTL is still fresh. Saving a place with other coordinates empties the cache,
  because another place's weather is not this place's; saving the same place again
  (the daily lookup) keeps it.
- **A licence point the owner must settle** (ADR-023): the free API is
  non-commercial and wants attribution.
