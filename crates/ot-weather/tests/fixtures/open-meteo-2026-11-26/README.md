# Open-Meteo responses, captured 2026-11-26

Raw, unedited, from `https://api.open-meteo.com/v1/forecast` with `curl` (FEAT-0065,
ADR-023). The places are public cities — Kuala Lumpur, Kuching, London — never a
user's own location. A change in the live shape means a new dated capture beside
these and a `docs/GOTCHAS.md` entry, not an edit of these files.

| File | Request (after `…/v1/forecast?`) | Shows |
|---|---|---|
| `kuala-lumpur-hourly-forecast.json` | `latitude=3.14&longitude=101.69&current=temperature_2m,weather_code&hourly=temperature_2m,weather_code&forecast_hours=6&timezone=auto` | HTTP 200. The answer's own coordinates are the grid cell's (3.1282952), not the ones asked for; times are local, with no offset in the string, so `utc_offset_seconds` is needed |
| `kuching-hourly-forecast.json` | same, 1.55 / 110.35 | A second zone; code 45 (fog) in the hourly series |
| `london-hourly-forecast.json` | same, 51.51 / -0.13 | `utc_offset_seconds` is 0 in winter, so only `timezone` says it is local |
| `kuala-lumpur-past-3h-local-time.json` | `…&hourly=temperature_2m,weather_code&past_hours=3&forecast_hours=1&timezone=auto` | `past_hours` puts the last hours, ending at the current one, in `hourly` — the history a temperature threshold needs |
| `kuala-lumpur-past-3h-gmt.json` | `…&hourly=temperature_2m&past_hours=3&forecast_hours=1&timezone=GMT` | **The request the app makes** (with six past hours): `timezone=GMT` makes every time UTC, so no offset arithmetic, and `utc_offset_seconds` is 0 |
| `http400-latitude-out-of-range.json` | `latitude=999&…` | HTTP 400, `{"error":true,"reason":"…"}`. The reason is the service's text and is logged, never displayed |
| `http400-invalid-variable.json` | `current=nonsense` | HTTP 400, the same shape; the reason leaks the service's internals |
| `http200-no-coordinates-empty-body.txt` | `current=temperature_2m` with no coordinates | **HTTP 200 with an empty body.** Not an error status, so the client must treat an empty or non-JSON 200 as a failure |

Headers worth knowing, from the same captures: `Content-Type: application/json;
charset=utf-8`, chunked, and `X-RateLimit-Limit: 600` per 60 s window with
`X-RateLimit-Cost: 1.0` for one place and a few variables.
