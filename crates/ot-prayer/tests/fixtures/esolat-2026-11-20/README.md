# e-Solat responses, captured 2026-11-20

The Phase 5 verification spike's raw responses ([ADR-018](../../../../../docs/08-decisions.md)),
kept exactly as the server sent them so the parser is tested against the real
thing. Captured at 09:58–10:00 Malaysian time from
`https://www.e-solat.gov.my/index.php?r=esolatApi/takwimsolat` with
`User-Agent: OmniToaster/0.12.0`.

| File | Request | Answer |
|---|---|---|
| `SGR01-month.json`, `WLY01-month.json`, `JHR01-month.json` | GET `&period=month&zone=<zone>` | 200, `status: "OK!"`, 1–30 November 2026 |
| `SGR01-duration-december.json` | POST `&period=duration&zone=SGR01`, body `datestart=2026-12-01&dateend=2026-12-31` | 200, `OK!`, 1–31 December, dates spelled `01-Dis-2026` |
| `SGR01-duration-next-year-no-record.json` | the same for January 2027 | 200, `status: "NO_RECORD!"`, `prayerTime` an object — the year is not published yet |
| `unknown-zone-no-record.json` | GET `&period=month&zone=XXX99` | 200, `NO_RECORD!` |
| `duration-as-get-http500-html-body.txt` | GET `&period=duration` with the dates as query parameters | **500**, `Content-Type: application/json`, an HTML body |

Do not edit these files. When the live format changes, capture a new dated
folder beside this one and record the change in `docs/GOTCHAS.md` under
"Prayer Times and JAKIM".
