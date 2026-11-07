# Security and privacy rules

Read [docs/06-security-and-privacy.md](../../docs/06-security-and-privacy.md)
before changing anything here. `PROPOSAL.md` §10 is the authority behind it.

This app has no accounts, no credentials and no cloud. Its security surface is
therefore **the files it writes and the files the user gives it**, not
authentication. There is no vault and no `secret:<ulid>` — if you find either
mentioned anywhere in this repository, it is a leftover and should be removed.

## Portability

- All app data goes under `OmniToasterData/` beside the exe: `config/`,
  `data/`, `locales/`, `assets/`, `logs/`, `cache/`. Nothing is written outside
  it.
- **The one exception** is the single HKCU `Run` value for "Start with Windows",
  written through `ot-platform` and refreshed on each launch so a moved
  portable folder still starts. No other registry key, ever.
- Never `%APPDATA%`, never `%TEMP%`, never `Documents`, never a path derived
  from an environment variable that could point outside the folder.
- Never commit `OmniToasterData/`.
- At startup, verify `OmniToasterData/` is **writable**. If it is not — the exe
  sitting in `Program Files`, say — show a clear **bilingual** error and exit
  without touching existing data. Don't fall back to another location; that
  would break portability silently.
- Only **relative** paths are stored in the database and in `settings.toml`, so
  the folder can be moved to another drive or machine. An absolute path stored
  anywhere persistent is a defect.

## Imported media

Every file the user supplies — an image for a toast, an Azan clip, a chime —
goes through the same path, and each step is required:

1. **Size limit** first, before reading: images 5 MB, audio to the limit in the
   spec. Reject over-size files with a localized message.
2. **Format by magic bytes**, never by extension. PNG, JPG and WebP for images;
   the audio formats the spec names. A `.png` that is not a PNG is rejected.
3. **Path canonicalization** on the source path, to catch traversal, and on the
   destination, to prove the write lands inside `OmniToasterData/assets/`.
4. **Copy in under a content-hashed name** (`asset.sha256`), and store the
   relative path. The app never reads a file from where the user left it.
5. **Bounded decode.** A pixel limit of 4096 × 4096 on images, and audio decode
   bounded too. A corrupt or hostile file must fail with a localized message,
   not hang or exhaust memory.
6. Decoding belongs in the **worker process** where practical, so a malformed
   file cannot take the UI down with it.

A thumbnail is cached under the data folder; it is derived data and may be
regenerated, so never treat it as the source of truth.

## Database

- **Parameterized statements only.** No SQL assembled from a string, not even
  for an identifier the code "knows" is safe — a zone code, a category key and
  a setting key all arrive from data.
- Migrations are numbered and tracked with `PRAGMA user_version`, applied in a
  transaction, and tested on both a fresh and an existing database.

## Network

- The only outbound requests are JAKIM e-Solat, Open-Meteo, the IP geolocation
  lookup, and the update check. HTTPS only, ten-second timeout, and the user can
  disable each of them.
- Location data is stored locally and sent only to those services.
- Every field of an external response is validated. The e-Solat endpoint is not
  a documented public API: a missing or malformed field marks the whole day
  invalid, unknown extra fields are ignored, and a format change must never
  crash the app.
- No telemetry, no analytics, no crash upload.

## Secrets

- No secrets in the repository. Nothing in `keys/` beyond what the release
  signing step needs, and nothing a session should read.
- If an API key ever becomes necessary, it goes in the **Windows Credential
  Manager** — never in `settings.toml`, never in the database, never in an
  environment variable baked into a script.

## Logs and crash reports

- Logs are daily files under `OmniToasterData/logs/`, retained 14 days by
  default. Crash reports and minidumps go under `logs/crash/`.
- A crash report contains the app version, the OS build, the panic message and
  the last 200 log lines. **No personal data**, no reminder bodies, no file
  paths outside the data folder.
- Crash reports and logs **stay local** and are never uploaded, not even
  optionally, not even with a prompt.

## Supply chain and release

- `cargo audit` and `cargo deny check` run in CI, and `cargo deny check` is part
  of the local gate — see [testing.md](testing.md) for why.
- Release binaries are code-signed and published with SHA-256 checksums
  alongside an rsign signature; the updater verifies both before replacing the
  exe.
