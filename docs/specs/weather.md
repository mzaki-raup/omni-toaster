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

- Polled every **30 minutes**, configurable, **minimum 15 minutes**.
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

## What is implemented (2026-11-08)

Nothing. Phase 6 has not started; this file is the requirement.
