# 06 — Security and privacy

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../PROPOSAL.md) §10

The agent-facing rules are in
[.claude/rules/security.md](../.claude/rules/security.md); this file is the
threat model and the policy behind them.

## What the app is trusted with

Nothing valuable, by design. There are no accounts, no credentials and no
secrets to protect — the feature set was chosen so that none is needed. What
*is* at risk is the user's machine (a malformed image or audio file is
attacker-controlled input), the user's privacy (a location is personal data),
and the user's trust (prayer times that are wrong are worse than absent).

## Threat model

| Threat | Exposure | Mitigation |
|---|---|---|
| Malformed image or audio import crashes or exploits a decoder | The user, or anything that can write to a folder they import from | Size limit, format check by magic bytes, bounded decode, decoding in the worker process so a crash is recovered |
| Path traversal through an imported file name | Same | Canonicalize and reject anything resolving outside the data folder; store content-hashed names, never the supplied one |
| A hostile response from JAKIM, Open-Meteo or the IP lookup | Network path, or DNS | HTTPS only, response size cap, strict parsing with no `unwrap`, a fallback for every fetch, no HTML or API text ever rendered |
| SQL injection through a user-written reminder title | The user's own input | Parameterized statements only, everywhere |
| A tampered update | Network path | Signature verified before anything is replaced; see [specs/updater.md](specs/updater.md) |
| Location leaking | The IP lookup, and the weather request | Both disablable; coordinates rounded before they are sent; nothing else leaves the machine |
| A crash report leaking user data | The local log folder | Reports stay local and are never uploaded; secrets do not exist to redact, but reminder titles are user text and are **not** written into a crash report |
| SmartScreen blocking an unsigned exe | Every new user | Code signing ([09-ci-cd-and-releases.md](09-ci-cd-and-releases.md)); until a certificate exists, published SHA-256 checksums and documented verification |

## Portability is a security property

All app data goes under `OmniToasterData/` beside the exe. Never the registry,
never `%APPDATA%`, never `%LOCALAPPDATA%`, never `%TEMP%`. The one exception is
the single HKCU `Run` value that implements "Start with Windows", which the
user opts into and which the app refreshes on each launch so a moved folder
still starts. `OmniToasterData/` is never committed.

This is enforced, not just documented: `--self-test` in CI asserts that nothing
named `OmniToaster*` appeared under `%APPDATA%`, `%LOCALAPPDATA%` or `%TEMP%`
during the run.

## Network

The only outbound requests are:

| To | For | Disablable | Data sent |
|---|---|---|---|
| JAKIM e-Solat | Official prayer times | yes (calculated mode) | the zone code |
| Open-Meteo | Weather | yes | rounded coordinates |
| The IP geolocation service | First-run location guess | yes (manual location) | nothing but the request itself |
| The release host | The update check | yes | the current version and channel |

All HTTPS. No telemetry, no analytics, no crash upload, no account.

## Input validation

Every import goes through one path in `ot-store`:

1. Size limit before the file is read.
2. Format identified by magic bytes, not by extension.
3. Decode with explicit bounds (pixel count, sample count, duration).
4. Canonicalize the destination and reject anything outside the data folder.
5. Hash the content, store under the hash, record the relative path.

A file that fails any step is rejected with a message that names the reason.

## Secrets

There are none, and that is a deliberate design constraint rather than an
oversight: no API key is required by any service the app uses. If one ever
becomes necessary it goes in the Windows Credential Manager through
`ot_platform`, never in a file, never in the repository, never in a log. No
encrypted vault is built, because there is nothing to put in it.

## Dependencies

`cargo audit` and `cargo deny check` run in CI. `cargo deny` is also part of
the local gate, because its advisories come from the RustSec database and move
on their own — a branch that was green yesterday can fail today with no change
of its own. When one fires, say whether the code actually reaches the
vulnerable path rather than implying a vulnerability shipped. New dependencies
get an ADR ([08-decisions.md](08-decisions.md)); pure-Rust and `rustls` are
preferred over C-bound and OpenSSL alternatives.

## Privacy statement for the README

The app is public and MIT licensed, so this is stated plainly where users can
see it: Omni Toaster stores everything locally in its own folder, sends nothing
anywhere except the four requests tabled above, has no accounts and no
telemetry, and can be made fully offline by turning those four off.
