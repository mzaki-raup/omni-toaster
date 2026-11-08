# OmniToaster — work ledger

Every unit of work has an **id**, lives under exactly one **task version**, and
carries its own **checklist**.

```
/dev-task v0.0.1       every item under that task version
/dev-task FEAT-0001    that one item, and nothing else
```

## Task versions are not app versions

A heading here is **my own batch label** — a bag of work I want done together.
It is not the application's release number, it does not have to match it, and
nothing in this file moves `[workspace.package] version`. The app's number is
decided by the release pipeline from the Conventional Commit types, whatever this file is called.

So `## v0.0.1` here means "task batch v0.0.1". A batch may ship across two
app releases, or three batches may ship in one. Where it is worth recording
which release carried a batch, the heading gets a `> **Shipped in:**` note
after the fact.

Companion files, and what each is for:

| File | Holds |
|---|---|
| **TASKS.md** (this file) | what is to be done, by id and by task version, with its checklist |
| `CHANGELOG.md` | what was done, one line per change, under Unreleased until the app releases |
| `CHECKLIST.md` | how far the roadmap phases have got |

---

## Format

### Ids

`FEAT-0001` · `BUG-0001` · `CR-0001` — four digits, allocated once, **never
reused and never renumbered**, because commits, branches and `CHANGELOG` lines
point at them.

| Prefix | Means |
|---|---|
| `FEAT` | new capability the user can see |
| `BUG` | behaviour that contradicts its spec or crashes |
| `CR` | change request: a deliberate change to existing behaviour that is neither new capability nor a defect (a changed default, a renamed label, a tightened guard, a dropped option) |

The next free number per prefix:

```bash
grep -oE '^### (FEAT|BUG|CR)-[0-9]{4}' TASKS.md | sort | tail -5
```

### Task version headings

```
  ## v0.7.1 — Planned
  > **Shipped in:** app v0.3.0, 2026-11-04
```

(Indented here only so this example isn't itself matched as a heading — real
headings start at column 0.)

- The label is mine to choose: `v0.7.1`, `v3`, `sprint-11`, `2026-11`. One
  token, no spaces. Keep one style across the file so sorting stays sane.
- Status is `Planned`, `In-Progress`, or `Done <YYYY-MM-DD>` — "done" meaning
  every item in the batch is `Completed`, not that the app released. A
  reconstructed batch whose finish date nobody recorded says `Done` with no
  date rather than inventing one.
- The optional `> **Shipped in:**` line records which app release carried the
  batch, filled in afterwards. The task version is **never** renamed to match
  the app's number, and `[workspace.package] version` is never edited by hand.
- Unscheduled work sits under `## Backlog`. An item belongs to exactly one
  heading; moving it means moving the whole `###` block.

### An item

```markdown
  ### FEAT-0012 · Mica-like glass on the toast window
  - **State:** Pending · **Spec:** docs/specs/toast.md · **Phase:** 3
  - **Branch:** feat/phase-3-glass
  - **Depends on:** FEAT-0011
  - [ ] <step>
  - [ ] <step>
  - [ ] Test: <what it asserts>
  - [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
  - [ ] App: `--data-dir ./tmp-data` — <what to click>
  - [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
  - **Notes:** <anything that would otherwise be lost>
```

Rules that keep the file honest:

- **State** is `Pending`, `In-Progress`, `Completed` or `Blocked` — the same
  vocabulary as `CHECKLIST.md`, plus `Blocked`. `Completed` only once every box
  is ticked, with one exception: when I close something with boxes still open,
  the boxes **stay open** and the heading or the item's **Notes** says I closed
  it and what went unchecked. The State is my call; the boxes are the record of
  what was actually verified, and nothing may tick them on my say-so.
- The last four boxes (**Test**, **Gate**, **App**, **Docs**) are on every
  item **that is still to be done**. A box is ticked only when that check
  actually ran and passed. A check that cannot be made on this machine (no
  server here) is left unticked with the reason in **Notes** — never ticked
  optimistically.
- `Blocked` always has a reason in **Notes**.
- **Depends on** names ids. A batch runs dependencies first.
- `BUG` items get a reproducing test before the fix, so their **Test** box is
  the first thing that goes green.
- Optional fields: **Spec** (the document in `docs/` that the item must match —
  a numbered spec such as `docs/11-prayer-and-azan.md` or an area spec such as
  `docs/specs/toast.md`), **Phase** (`0`–`9`, the delivery phases of
  `PROPOSAL.md` §13), **Branch**, **Depends on**, **Source**
  (where the request came from), **Kind** (`major`, on the items that moved the
  minor digit) and **Verified** (what the `Completed` claim rests on).

### Commits

Reference the id in the footer so history maps back to the ledger:

```
fix(ot-prayer): keep the zone selection across a failed fetch

Refs: BUG-0003

```
### Latest Vacant/Free Task

```
Version : v0.0.2
Features : FEAT-0010
Bugs : BUG-0003
Change Request : CR-0002
```

---

## v0.0.1 — In-Progress

Phase 0, Foundation. The Phase 0 exit criteria from `PROPOSAL.md` §13: the app
starts in English and in Malay, writes a log, the locale key parity check
passes, and CI is green. Nothing here is `Completed` until all of that holds.

**All four exit criteria hold as of 2026-11-08, and FEAT-0001 to FEAT-0007 are
`Completed`.** The batch is still `In-Progress` rather than `Done`, because the
batch closes only when *every* item in it is `Completed` and two are not:
FEAT-0009 (the release packaging) is `In-Progress` with the Pester tests and
`THIRD_PARTY_NOTICES.md` outstanding, and BUG-0001 is `Blocked` on the owner.
Neither is an exit criterion, so Phase 1 is not held up by them.

> **Shipped in:** app v0.1.0 and v0.2.0, 2026-11-08 — the batch spans two
> releases, which is exactly what this file's own Format section says a task
> version may do.

The items are ordered by dependency: FEAT-0001 first, then 0002 and 0003 on top
of it, then 0004, 0005 and 0006 in any order.

### FEAT-0001 · Cargo workspace skeleton
- **State:** Completed · **Spec:** docs/03-architecture.md · **Phase:** 0
- **Branch:** feat/phase-0-foundation
- **Verified:** the gate below, run 2026-11-08 on this machine, plus the built exe
- [x] Root `Cargo.toml` with `[workspace]`, `[workspace.package]` (version, edition 2024, licence MIT, repository) and `[workspace.dependencies]` for everything shared
- [x] `crates/omnitoaster` — package `omnitoaster`, `[[bin]] name = "OmniToaster"`, a `main` that prints its version and exits
- [x] The ten library crates, each compiling with a doc comment and nothing else: `ot-core`, `ot-i18n`, `ot-store`, `ot-platform`, `ot-ui`, `ot-toast`, `ot-sched`, `ot-prayer`, `ot-weather`, `ot-media`
- [x] `.cargo/config.toml` with `+crt-static` so the MSVC build stays portable
- [x] `rust-toolchain.toml` pinning the stable channel and the MSVC target
- [x] `deny.toml` so `cargo deny check` has something to read
- [x] `.gitignore` covering `target/`, `tmp-data/` and `OmniToasterData/`
- [x] Test: `cargo metadata` lists all eleven members; a test in `omnitoaster` asserts `env!("CARGO_PKG_VERSION")` is non-empty
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check
- [x] App: `cargo build --release -p omnitoaster` produces `target/release/OmniToaster.exe`, and running it prints the version
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** `ot-platform` is the only crate allowed to contain Windows-only code, so every other crate's clippy must pass on Linux too. No tokio — `PROPOSAL.md` §5 and the architecture spec settle this. Two deliberate deviations from the boxes as written, both recorded in `docs/03-architecture.md`: `[workspace.dependencies]` is present but **empty**, because populating it would have meant inventing twenty version numbers that `--locked` would then enforce — each later item adds its own with `cargo add`, so the versions come from the registry; and `crates/omnitoaster` declares **no** path dependencies yet for the same reason an unused dependency is dead weight, with `cargo build --workspace` still compiling every member so the gate covers them. `.gitignore` landed earlier, in the realignment batch. The release profile sets `panic = "abort"`, which is a design choice the supervisor depends on and is written up in the architecture spec. `+crt-static` was confirmed, not assumed: the built exe imports only `KERNEL32.dll` and `ntdll.dll`, with no `VCRUNTIME`, `msvcp` or `api-ms-win-crt` entries. The gate's `--self-test` step is not in this item's box because the flag arrives with FEAT-0006.

### FEAT-0002 · Portable path resolution and the writability check
- **State:** Completed · **Spec:** docs/05-data-model-and-storage.md · **Phase:** 0
- **Branch:** feat/phase-0-paths
- **Depends on:** FEAT-0001
- **Verified:** the gate below plus both halves of the App box, run 2026-11-08
- [x] `ot-core::paths` resolves the data root: `--data-dir <dir>` when given, otherwise `OmniToasterData/` beside the exe — never `%APPDATA%`, `%TEMP%` or the registry
- [x] Create the subtree on first run: `config/`, `data/`, `locales/`, `assets/{images,audio/azan,audio/chimes,icons}`, `logs/crash/`, `cache/`
- [x] Writability check at startup: write and delete a probe file in the root; on failure return a typed error that the caller can render bilingually, and exit without touching existing data
- [x] Path canonicalization helper that refuses any path resolving outside the data root
- [x] Test: a resolved root under a temp dir gets the whole subtree; a read-only root produces the writability error and creates nothing; `../` and an absolute path outside the root are both refused
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check — 17 tests pass. `--self-test` is **not** in this run because the flag does not exist until FEAT-0006
- [x] App: `--data-dir ./tmp-data` — the folder appears with every subdirectory; point `--data-dir` at a read-only folder and the app reports it and exits instead of panicking
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** The read-only case did **not** need the escape hatch this item anticipated: `icacls /inheritance:d` followed by a `(OI)(CI)(W,AD)` deny produced a genuinely unwritable folder, PowerShell confirmed it refused a write, and the app reported `the data folder … is not writable: Access is denied. (os error 5)`, printed that nothing was changed, exited 1 and — with the ACL restored afterwards to check — had created nothing inside it. The folder and its ACL were cleaned up. `PathError` is a `thiserror` enum with four variants kept separate because their user-facing advice differs; FEAT-0004 turns the binary's plain-English rendering into `error-` Fluent keys. Two things found while building it, both now in `docs/GOTCHAS.md` under "Storage and Migrations": `fs::canonicalize` returns a `\\?\` extended-length path on Windows that leaks into messages and subprocess arguments, so it is stripped unless the path is at or past 260 characters where the prefix is load-bearing; and writability must be proved by writing, since a permission check still says yes on a full disk or a read-only mount. `join()` also re-checks with `canonicalize` where the target exists, so a symlink inside the root that points outside it is refused too. **Scope note:** minimal `--data-dir` parsing landed in `crates/omnitoaster/src/main.rs` here rather than in FEAT-0006, because this item's App box cannot be exercised without it; FEAT-0006 still owns `--self-test`, `--worker` and the bilingual `--help`.

### FEAT-0003 · Logging and the panic hook
- **State:** Completed · **Spec:** docs/03-architecture.md · **Phase:** 0
- **Branch:** feat/phase-0-paths
- **Depends on:** FEAT-0002
- **Verified:** the gate below plus both halves of the App box, run 2026-11-08
- [x] `tracing` + `tracing-appender` writing `logs/app-YYYY-MM-DD.log`, daily rotation, retention configurable with a 14-day default
- [x] A log level from settings, overridable by `--log-level` for a session — the `--log-level` half is done; reading the default from `settings.toml` waits for the settings file itself, which no item has created yet
- [x] `std::panic::set_hook` writing `logs/crash/<timestamp>.txt` with the app version, the OS build, the panic message, the location and the last 200 log lines — and no personal data
- [x] Retention sweep at startup deletes log files past the retention window
- [x] Test: a span and an event reach the log file; the panic hook writes a crash file whose contents contain the panic message and no absolute user path; the sweep deletes an aged file and keeps a fresh one
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check — 27 tests pass. `--self-test` is not in this run because the flag does not exist until FEAT-0006
- [x] App: `--data-dir ./tmp-data` — the log file appears and grows; a deliberate panic behind a debug-only trigger leaves a crash file
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** The supervisor that restarts after a crash is Phase 8; this item only captures the crash. Verified in the app: `app.2026-11-08` appeared with the startup and `startup complete` lines, and `--panic-now` (compiled only under `debug_assertions`) produced `logs/crash/2026-11-08T12-37-36.553.txt` carrying the version, `os: windows x86_64`, the panic message, the source location, and the recent log line with the data folder **redacted to `<data>`** — grepped afterwards for an absolute user path and found none. Three decisions worth knowing. `RecentLines` is a bounded in-memory ring of the last 200 formatted lines, so a crash report has context without re-reading the log while the process is already failing; its mutex recovers from poisoning rather than panicking again inside the logging layer, since a panic is exactly when it is needed. `HostInfo` is passed in rather than discovered, because reading the Windows build number is platform code and `ot-core` depends on nothing — for now it reports `windows x86_64`, and Phase 1 can supply the real build through `ot-platform`. And `LevelFilter` was chosen over `EnvFilter`: per-module levels would be nice but `EnvFilter` pulls regex machinery into a binary that ships as one portable exe, and nothing asks for them. **Found a real bug while doing this** — see the paths note below.

### FEAT-0004 · Fluent setup, both locale files, and locale detection
- **State:** Completed · **Spec:** docs/10-i18n.md · **Phase:** 0
- **Branch:** feat/phase-0-paths
- **Depends on:** FEAT-0001
- **Verified:** the gate below plus the App box in both languages, run 2026-11-08
- [x] `locales/en-US/omni_toaster.ftl` and `locales/ms-MY/omni_toaster.ftl`, embedded with `rust-embed`, holding the Phase 0 keys: the startup and writability errors, the log-path notice, the language names, and `format-` keys
- [x] `ot-i18n` builds the bundles with `i18n-embed`; a missing Malay key falls back to English and logs a warning. **The `fl!()` path is deliberately not used** — see Notes and the ADR-006 amendment
- [x] Locale detection: `GetUserDefaultLocaleName` in `ot-platform`, `ms-*` → `ms-MY`, everything else → `en-US`; an explicit `--lang` overrides it
- [x] User locale overrides loaded from `<data>/locales/` and validated; a broken override is ignored with a logged warning and the embedded text is used
- [x] The single `format_datetime(dt, prefs)` formatter — the only function in the workspace that formats a date, time or number
- [x] Test: key parity — the `en-US` and `ms-MY` key sets are identical, and the test fails naming the keys that are missing from each side; both bundles parse with no syntax errors; a missing Malay key falls back to English; `format_datetime` produces the expected string in both languages for 12-hour and 24-hour
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check — 44 tests pass. `--self-test` is not in this run because the flag does not exist until FEAT-0006. **This box was first ticked on a run that passed by luck:** two tests raced on the global Fluent loader and one unwrapped `init`, so the suite's result depended on thread scheduling. It went green locally, then failed the v0.1.0 release job with `AlreadyInitialised`. Re-ticked after the fix, proven with 20 consecutive parallel runs, a `--test-threads=1` run, and each racing test in isolation
- [x] App: `--data-dir ./tmp-data` in both languages, including the unwritable-folder error path
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** **Edit both `.ftl` files with the Edit tool, never a heredoc** — an unquoted heredoc eats `{ $placeholders }` and backticks and silently drops the message. 16 tests in `ot-i18n` cover parity (naming the missing keys per side), bundle parsing, key casing and prefixes, the OS-locale mapping, override validation, and the Malay-vs-English fallback.
  **Three design changes this forced, each with its document updated in the same commit.** (1) `fl!` and `i18n-embed-fl` are dropped: both derive the catalogue filename from the *crate* name, so they look for `ot-i18n.ftl`, while the locale files are named after the product as the data-model spec requires. `i18n-config` cannot override the domain, so the loader is built with `FluentLanguageLoader::new("omni_toaster", …)` and the compile-time key check is replaced by the parity, prefix and casing tests — ADR-006 carries the amendment and the condition for revisiting it. (2) `format_datetime` lost its `lang` parameter: the words come from the active bundle and `TimePrefs::for_language` carries the only language-dependent choice, so a `lang` argument could only have been ignored. `docs/03` and `docs/10` are corrected. (3) `publish = false` is now inherited by every member, which is true (the deliverable is the exe) and is what lets `cargo deny` accept versionless intra-workspace path dependencies.
  **Two bugs the App check caught that the tests had not**, both now in GOTCHAS: `load_languages` takes the *active* language first and fallbacks after, so "English first for the fallback" made every lookup answer in English while the UI reported Malay — invisible to parity tests and to the language-name key, which reads the same in both files; and Fluent's U+2068/U+2069 isolating marks were reaching users as stray boxes in `Logs are kept in ⁨D:\…⁩.`, now stripped in `text_with`, the one place every substituted string passes through. A regression test covers each.
  `cargo deny` also went red twice: `basic-toml` is flagged **unmaintained** (not vulnerable, no upgrade available) and arrives only inside the `i18n-embed` proc-macro at build time, so it is ignored in `deny.toml` with that reasoning written down.

### FEAT-0005 · SQLite store and migration `001_init.sql`
- **State:** Completed · **Spec:** docs/05-data-model-and-storage.md · **Phase:** 0
- **Branch:** feat/phase-0-store
- **Depends on:** FEAT-0002
- **Verified:** the gate below plus the App box, run 2026-11-08
- [x] `ot-store` opens `data/omnitoaster.db` with `rusqlite`, WAL journal, foreign keys on
- [x] Migration runner tracked with `PRAGMA user_version`, applying numbered files in order inside one transaction each
- [x] `001_init.sql`: `settings`, `location`, `category`, `reminder`, `snooze_log`, `notification_log`, `asset`, `weather_cache`, per `PROPOSAL.md` §9
- [x] Every statement parameterized; no string-built SQL anywhere in the crate
- [x] Test: a fresh database migrates to `user_version = 1` and has every table; running the migration twice is a no-op; an existing database at version 1 is left alone; a settings round trip returns what was written
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check — 52 tests pass. `--self-test` still does not exist; FEAT-0006 owns it
- [x] App: `--data-dir ./tmp-data` — `data/omnitoaster.db` appears on first run, `PRAGMA user_version` reads 1, and a second run does not re-migrate
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** Migrations `002_locale_keys.sql` and `003_jakim.sql` belong to Phases 4 and 5 and are **not** written here — `category` therefore has no key columns and no `authored_lang` yet, which is what `docs/05` assigns to `002`. 8 tests cover the migration, the table set, idempotence, the settings round trip, and three things the schema would otherwise only claim: that foreign keys are actually enforced (SQLite defaults them **off**, which would make every `ON DELETE` clause decorative), that a delete cascades, and that `STRICT` rejects a text value in an INTEGER column. A database at a *higher* `user_version` than the build knows is refused with a message telling the user to update, rather than being written to with an older schema.
  **Decisions.** `rusqlite` uses the `bundled` feature, so SQLite is compiled from source into the exe and a portable folder can never meet a missing or ancient system `sqlite3.dll`; it also makes `STRICT` tables safe to rely on, since the bundled engine is new enough. Verified that this does not break portability: the release exe has **zero** dynamic CRT imports, so the C compilation picked up `+crt-static` correctly, and the binary grew from 970 KB to 2.6 MB — still one file. `Store` has a hand-written `Debug` that prints only the schema version, because the contents are the user's reminders and should not reach a log line or a crash report by accident. Cargo selected `rusqlite` 0.40.2 rather than 0.41.1 because the newer one needs Rust 1.89 and the workspace declares 1.88; worth revisiting only if 0.40 picks up an advisory.
  **A false positive worth knowing about:** `scripts/verify-release.ps1` reports "`--version` and `--self-test` OK" for v0.1.0, but `--self-test` does not exist yet, so the flag is ignored and the exit code is 0 for the wrong reason. That check only becomes real with FEAT-0006.

### FEAT-0006 · The `--self-test` and `--data-dir` flags
- **State:** Completed · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Branch:** feat/phase-0-cli
- **Depends on:** FEAT-0002, FEAT-0003, FEAT-0004, FEAT-0005
- **Verified:** the gate plus the release exe, run 2026-11-08
- [x] CLI parsing in `omnitoaster` for `--data-dir <dir>`, `--self-test`, `--worker` and `--version`, with a bilingual `--help`
- [x] `--self-test` runs the headless checks and prints one flushed line per check with `ok` or `FAILED`: paths resolve and are writable, the log file is written, both locale bundles parse, key parity holds, the database migrates to the expected version, the date formatter returns a value in each language
- [x] Exit code 0 only when every check printed `ok`. **The panic half of this box could not be built as written** — see Notes; the name is printed before the check instead, so an abort still names what died
- [x] `--worker` is accepted and recognised now but does nothing until Phase 8
- [x] Test: `--self-test` against a temp data dir exits 0 and its output contains every check name; the broken-locale case is covered, with a correction — see Notes
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · **--self-test on the release exe** — 61 tests, and the self-test's six checks all `ok`
- [x] App: `--self-test` exits 0 with every line `ok`; `--data-dir ./tmp-data` still starts normally; `--help` reads correctly in both languages
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** **The `--self-test` gate box on FEAT-0001 to FEAT-0005 was left unticked because the flag did not exist. It does now, and it passes on the release exe**, so that caveat on those five items is resolved — their boxes stay as they were ticked, since they record what ran at the time.
  **One box could not be built as written.** "A panic inside a check becomes a `FAILED` line naming it" needs `catch_unwind`, and the release profile sets `panic = "abort"` so the supervisor sees a non-zero exit rather than a half-unwound process — nothing unwinds, so nothing can be caught. Rather than weaken the panic strategy for the benefit of the self-test, the check's **name is printed and flushed before the check runs** and the verdict after it, so an abort leaves the dying check's name as the last line of output. Same diagnostic value, different mechanism; `docs/09-ci-cd-and-releases.md` now specifies the ordering and why, and a test asserts the name precedes the verdict.
  **A correction to this item's own test box.** It asked for "a deliberately broken locale makes it exit non-zero and name the locale check". That contradicts `docs/05-data-model-and-storage.md`, which requires a broken *override* to be ignored with a logged warning so a user cannot break their app by mis-editing one. Both cannot be true. The spec wins: the test now asserts the broken override is survived, the locale check still runs, and nothing crashes or hangs. A broken *embedded* catalogue would fail the check, but it cannot be produced at runtime — it would fail `cargo test` first.
  **Beyond the boxes, one addition worth naming:** an argument starting `--` that is not a known flag is now rejected and named. A silently ignored `--data-dirr` typo would have started the app against the **real** `OmniToasterData/`, which is the kind of thing that eats someone's reminders. A test keeps `--help`'s list and the accepted set from drifting apart.
  `ot-i18n` gained public `message_keys`, `check_key_parity` and `probe_language` so the self-test and the unit test share one parity implementation rather than each parsing the catalogues their own way.

### FEAT-0007 · CI skeleton
- **State:** Completed · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Branch:** the realignment batch, then `feat/phase-0-cli` for this bookkeeping
- **Depends on:** FEAT-0006
- **Verified:** nine green `ci.yml` runs and two published releases — the run numbers are in Notes
- [x] `.forgejo/workflows/ci.yml` on the owner's machine: fmt, clippy `-D warnings`, `test --workspace --locked -j 4`, locale key parity, `cargo deny check`, release build, then `--version` and `--self-test` on the built exe — every cargo step bounded with `-j 4`
- [x] `.github/workflows/ci.yml` running the same checks on the mirror, guarded with `if: github.server_url == 'https://github.com'`
- [x] `ci.yml` does not run on `main`; documentation-only pushes do not trigger a release
- [x] Test: not applicable — a workflow is verified by a green run, recorded in Notes
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [x] App: not applicable — this item ships no user-visible behaviour
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** **This item sat `Pending` while its work was finished and its CHECKLIST row read `Completed`** — the two disagreed for most of Phase 0, which is the exact drift `task-auditor` exists to catch, and it was my oversight rather than anything about the work. The pipeline itself was built in the realignment batch and has been exercised continuously since.
  The evidence this box rests on: `ci.yml` green on runs 3, 4, 9, 10, 11, 13, 15, 17 and 19 across six branches; `release.yml` published v0.1.0 (run 14) and v0.2.0 (run 18), both then verified from a clean download by `scripts/verify-release.ps1`. Docs-only pushes to `main` stayed green and published nothing while no `Cargo.toml` existed (runs 1 and 2), which is the behaviour the third box asks for. Two release runs failed first — run 5 at the version bump with no signing key configured, run 12 at the tests step on a flaky test — and both failed **safely**, before the push and publish steps, leaving no tag and no release.
  The runner **is** this machine, so no local cargo command may run while a job is; that rule held throughout. The asset contract and the updater are Phase 9. **BUG-0001 is open against this pipeline** — the forge intermittently stops creating runs — and it is a fault in the forge rather than in these workflows, which is why it does not block this item.

### FEAT-0009 · Release packaging, signing keys and release-planner tests
- **State:** In-Progress · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Branch:** feat/phase-0-foundation
- **Depends on:** FEAT-0001
- **Verified:** 2026-11-08, a signed local packaging run of 0.1.0, checked independently afterwards. `rsign verify -p keys/minisign.pub -x manifest.json.minisig manifest.json` exits 0 with "Signature and comment signature verified" and the trusted comment `OmniToaster-0.1.0-stable`, so **the committed public key and the owner's private key are a matching pair** and the key is unencrypted — an encrypted one would have hung the 60-second timeout instead. The signing ran through the `$env:OT_MINISIGN_KEY` path from the raw file contents, so the script's two-line normalisation is exercised as the forge will exercise it. All six assets were produced. `manifest.json` reads schema 1, product `OmniToaster`, version `0.1.0`, channel `stable`, tag `v0.1.0`, `min_supported_version` `0.1.0`, `data_schema` 1; both asset entries' recorded size and SHA-256 match the real files; `SHA256SUMS.txt` covers the other five files, every hash agrees, and it correctly omits itself. The zip holds `OmniToaster.exe`, `README.txt`, `LICENSE` and `CHANGELOG-0.1.0.md`. The manifest's asset URLs point at the Forgejo download path, which is right for a Forgejo release — the GitHub pipeline passes `-RepoUrl` explicitly for exactly this reason.
- [x] `scripts/build-and-package.ps1` — builds the release exe, assembles `OmniToaster-<version>-windows-x64.zip` (exe + `README.txt` + `LICENSE` + `THIRD_PARTY_NOTICES.md` + notes), writes `manifest.json`, writes `SHA256SUMS.txt` and `CHANGELOG-<version>.md`, signs the manifest with rsign, and fails if any contract member is missing. Written 2026-11-08; **parsed, never executed** — there is no workspace to build yet
- [x] `keys/README.md` — generation, where the public key is embedded, the `OT_MINISIGN_KEY` forge secret, and rotation. Written 2026-11-08
- [x] `keys/minisign.pub` — the owner generated the keypair 2026-11-08; the public half is committed. Shape verified: two lines, `untrusted comment: minisign public key`, a 56-character `RWQ…` body, and no secret material. The private half sits at `./omnitoaster-minisign.key`, which `.gitignore` covers — `git add` refuses it, confirmed, and `git status --ignored` shows it as `!!`
- [ ] `THIRD_PARTY_NOTICES.md` — the packaging script copies it into the zip only `if (Test-Path)`, and it does not exist, so the 0.1.0 zip shipped without it. Harmless today because the workspace has no third-party dependencies; it must exist before the first release that does. Generate it from `cargo about` or `cargo-license` rather than by hand
- [ ] `scripts/tests/` — Pester 5 tests for the release planner against throwaway git repositories, covering each Conventional Commit type's bump and a documentation-only push producing no release
- [ ] Remove the `Test-Path scripts/tests` guard added to `.forgejo/workflows/ci.yml` once the tests exist
- [x] Rename the forge secrets: the workflows read `OT_MINISIGN_KEY` and `OT_FORGE_UPLOAD_URL`, and the GitHub mirror `OT_SIGNING_KEY_EXTRA`. **Proven by release run 14**, which signed `manifest.json` on the runner and published v0.1.0; `scripts/verify-release.ps1 -Tag v0.1.0` then verified that signature against the committed public key from a clean download
- [ ] Test: `pwsh -c 'Invoke-Pester scripts/tests'` green; `scripts/build-and-package.ps1` produces the full asset set and `scripts/verify-release.ps1` accepts a published release. **Two of the three now hold:** packaging produces the full set, and `verify-release.ps1 -Tag v0.1.0` passed against the real published release — downloaded anonymously, signature verified against `keys/minisign.pub`, every size and SHA-256 matched. The box stays unticked **only** because `scripts/tests/` does not exist. The script's "`--self-test` OK" line **was** a false positive from v0.1.0 to v0.2.0 — the flag was unimplemented, so it was ignored and the exe exited 0 for the wrong reason. FEAT-0006 fixed it, and `verify-release.ps1 -Tag v0.3.0` now prints the six check lines inside its own output, so that line means what it says
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: not applicable — this item ships no user-visible behaviour
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** Filed 2026-11-08 during the scaffolding realignment, when both `release.yml` files called `scripts/build-and-package.ps1` and referenced `keys/README.md` while neither existed. Both were written the same day and are ticked above; they are **parsed, not executed**, because there is no workspace to build until FEAT-0001. What is still open is `keys/minisign.pub`, the Pester tests, and the forge secret rename. **Now confirmed, and `Blocked` on the owner.** The prediction came true the moment FEAT-0001 merged: release run 5 on `main` failed. Before FEAT-0001 the release job's "Plan the release" step short-circuited on `if (-not (Test-Path Cargo.toml))` and emitted an empty version, so every later step skipped and a docs push to `main` stayed green. A `Cargo.toml` now exists and a `feat:` commit is in history, so the planner computes a minor bump to `0.1.0` and the job proceeds to the signing step, which throws because `OT_MINISIGN_KEY` is unset and `keys/minisign.pub` is absent. The failure was contained: it dies before the push and publish steps, so **no tag was pushed, no release was created, and `main` is untouched** at `71a5ac3` — verified with `git ls-remote --tags` (empty), the releases API (0 releases) and a remote/local `main` comparison. There is no tag yet, so **every further push to `main` will re-attempt `0.1.0` and fail the same way** until two things the owner must do themselves are done: generate the minisign keypair (`rsign generate --unencrypted`, per `keys/README.md`) and commit `keys/minisign.pub`, then set the `OT_MINISIGN_KEY` secret on both forges, along with `OT_FORGE_UPLOAD_URL` and the mirror's `OT_SIGNING_KEY_EXTRA`. **I did not generate the keypair:** a release signing key is the owner's to create and hold, and nothing should ever put its private half where an agent has handled it. `.forgejo/workflows/ci.yml`'s Pester step was unguarded and would have failed the very first branch push; it is guarded now, and this item removes the guard. The asset names are a contract the updater reads — see `docs/specs/updater.md` and ADR-010; renaming one needs a new ADR.

---

## Backlog

### CR-0001 · The product is "OmniToaster", one word, everywhere a user sees it
- **State:** Completed · **Spec:** docs/01-vision-and-scope.md · **Phase:** 0
- **Branch:** cr/product-name-omnitoaster
- **Source:** the owner, on seeing `OmniToaster --version` print `Omni Toaster 0.1.0`
- **Verified:** the gate plus the app, run 2026-11-08
- [x] Spec first: `docs/01-vision-and-scope.md` states the canonical spelling and what keeps the hyphenated repository name
- [x] Both locale catalogues, so the Malay strings do not keep a spelling the English ones dropped
- [x] The version line, the startup and error messages, the crash report header
- [x] The packaged `README.txt`, the release title and `TAGGER_NAME` on both forges
- [x] Test: no message in either `.ftl` contains the two-word spelling, naming the offending lines
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check — 53 tests
- [x] App: `--version` prints `OmniToaster 0.1.0`; both languages and the crash report header checked
- [x] Docs: spec · README · CHANGELOG · CHECKLIST
- **Notes:** 66 occurrences across 32 files, done as a literal replacement with no regex or escapes — the only kind of scripted edit this project trusts (`docs/GOTCHAS.md`). The real defect was that the product had **two** spellings: the exe, the data folder, the package, `docs/CHANGELOG.md` and `docs/CHECKLIST.md` already said `OmniToaster`, while the proposal, the specs and every user-visible string said `Omni Toaster`. Two things were deliberately left alone: `Feasibility Study.txt`, which records the owner's own original words and is a historical input rather than a document we maintain; and the released `0.1.0` section of `docs/CHANGELOG.md`, which belongs to the release workflow and is never hand-edited — it happens to contain no occurrence anyway, since its entries are commit subjects. **v0.1.0 shipped with the old spelling** and cannot be changed retrospectively; v0.2.0 carries the fix.
  **A second mistake, found afterwards and worth more than the first:** this was committed as `style:`, which `scripts/next-release.ps1` lists in `$NoReleaseTypes` — no version bump and **no line in the release notes**. So the rename reached users in v0.2.0 carried by the `feat(store)` commit in the same push, and v0.2.0's notes do not mention it. Released sections are never hand-edited, so that cannot be corrected now. A change to user-visible text is `fix:` or `feat:` however cosmetic the diff looks; `docs/09` and `docs/GOTCHAS.md` now say so. The same review also found `docs/09`'s bump table wrongly listing `refactor:` and `build:` as patch bumps when the script treats both as no-release.

### BUG-0002 · The GitHub mirror handoff skips silently, so no release reaches GitHub
- **State:** Blocked · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Blocked on:** the owner — the fix needs two forge secrets only they can set
- **Source:** the owner asked why GitHub had built no release
- [x] Establish what is actually missing
- [x] Set `OT_GITHUB_MIRROR_URL` and `OT_GITHUB_MIRROR_TOKEN` as repository secrets on the Forgejo instance, and `OT_MINISIGN_KEY` on GitHub so its release job can sign — the owner set all three on 2026-11-08. **Unverified from here:** secret names are not readable, so the only proof is a release run that mirrors successfully
- [ ] Back-fill the three tags GitHub never received. Either a `workflow_dispatch` of GitHub's Release workflow per tag from its Actions tab, or pushing each tag to the GitHub remote. Both create public releases, so both are the owner's to trigger
- [ ] Make the skip **loud**: `scripts/push-to-mirror.ps1` should still exit 0 when the mirror is deliberately unconfigured, but the workflow step should surface it — a forge notice, or a step summary line — so three releases cannot go by unnoticed again
- [ ] Decide whether an unconfigured mirror should fail the release outright once the secrets exist, since from then on a skip means something broke rather than something was never set up
- [ ] Test: a release run reports what it did with the mirror either way
- [ ] Docs: spec · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** Found 2026-11-08. GitHub has **no tags and no releases**, while Forgejo has v0.1.0, v0.2.0 and v0.3.0. `.github/workflows/release.yml` triggers on `push: tags: ['v*']`, so with no tag it has never run — GitHub's 10 Actions runs are all `ci.yml`, all green.
  The tags never arrive because the Forgejo release job's last step runs `scripts/push-to-mirror.ps1`, which opens with: if `OT_GITHUB_MIRROR_URL` or `OT_GITHUB_MIRROR_TOKEN` is unset, print "GitHub mirror not configured … skipping" and **`exit 0`**. The job stays green having mirrored nothing. That silence is the actual defect — the missing secrets are just configuration.
  `main` *is* current on GitHub, which is what makes this easy to miss: `origin` carries GitHub as a second **push url**, so `git push origin main` reaches both. That pushes the refs named on the command line, never tags, so branches mirror as a side effect of ordinary pushes while tags depend entirely on the step that is skipping.
  **I may have caused it.** Earlier the same day I renamed every `HS_*` pipeline variable to `OT_*`, including these two. If they had been configured under the old names, that rename broke the handoff — and because the script exits 0, nothing said so. I cannot read secret names from here to tell whether they were ever set for this repository, so the honest position is that the rename is a candidate cause rather than a proven one.
  Note that GitHub's own release job will also need `OT_MINISIGN_KEY` to sign, and uses `GITHUB_TOKEN` plus the `gh` CLI for the rest.

### BUG-0001 · A branch push intermittently creates no `ci.yml` run
- **State:** Blocked · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Blocked on:** the owner — this is the forge's Actions dispatch, not anything in this repository
- **Reopened:** 2026-11-08, after a second and third occurrence. The earlier closure called it a one-off transient outage; it is a recurring one, which is a different thing
- [x] Reproduce: a branch push creates no workflow run, while the forge, the runner and the git push itself are all healthy
- [x] Rule out the workflow's `on.push` filter — a later branch push triggered `ci.yml` within seconds with the filter untouched
- [ ] Establish why the forge stops creating runs. First occurrence came with `502 Bad Gateway` in the runner's task fetch; the second and third came with **no error at all** in the runner log, so a 502 is a symptom rather than the cause
- [ ] Decide what the project does about it: whether `workflow_dispatch` is the manual nudge, or whether the branch gate needs a documented "CI did not run" path so work is not blocked indefinitely
- [ ] Docs: CHANGELOG · CHECKLIST · GOTCHAS
- [x] Reproduce: push a branch and check whether a workflow run is created
- [x] Establish the cause
- [x] Confirm `ci.yml` still does not run on `main`
- [x] Test: not unit-testable — settled by a real branch push
- [x] Docs: CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** My first diagnosis — the `on.push` filter's `!` negation — was wrong, and no code was changed for it. My second — "a transient outage, closed" — was also wrong, just less so.
  **Occurrence 1, 05:26.** `docs/realign-scaffolding` reached Forgejo at `2a720c0` (`git ls-remote` confirmed it) and 20 minutes of polling showed `total_count` still 1. The runner logged `fetching a task timed out after 5s` and `failed to fetch task: unavailable: 502 Bad Gateway` at 05:40. A later push of `feat/phase-0-foundation` triggered `ci.yml` in seconds with the filter untouched, which is what ruled the filter out.
  **Occurrences 2 and 3, ~13:30 onward.** `feat/phase-0-store` at `c084482` and `cr/product-name-omnitoaster` at `e7ec0f0` both reached the remote and neither produced a run. This time the diagnostic came back completely clean: forge responding on `/api/v1/version` with `9.0.3+gitea-1.22.0`, `gitea-runner` service **Running**, and **no error of any kind** in the runner log — whose last task fetch was 13:12:45, consistent with no task existing to fetch rather than a failure to fetch one. So the 502 in occurrence 1 was a symptom of the same underlying stall, not its cause.
  **Why this matters beyond the annoyance:** the project's mandated flow is branch → push → CI green → merge, and it cannot be followed while runs are not created. Work either waits indefinitely or merges unverified. That decision is the owner's, which is why this is `Blocked` rather than carried further here.
  The diagnostic order itself is worth keeping and is in `docs/GOTCHAS.md`: confirm the commit is on the remote, read the runner log, then check the forge — before touching a workflow that is probably innocent.

### FEAT-0008 · In-app updater
- **State:** Pending · **Spec:** docs/specs/updater.md · **Phase:** 9
- [ ] Fetch `manifest.json` over HTTPS on a schedule and on demand from Settings → About
- [ ] Verify the detached rsign signature against a public key embedded in the exe **before** reading the manifest's contents as trustworthy
- [ ] Compare the manifest version with `CARGO_PKG_VERSION` by semver, not by string
- [ ] Download the bare `OmniToaster.exe` (not the zip — the zip is the manual path, see `docs/specs/updater.md` §3), check its SHA-256 against the manifest, and stage it beside the running exe as `OmniToaster.exe.new`
- [ ] Replace the running exe on restart, leaving `OmniToasterData/` untouched
- [ ] Never update silently: the user is told what version, what size and what changed, and chooses
- [ ] A failed signature or digest is reported and abandoned — never retried against an unsigned asset, never fallen back to an unverified download
- [ ] Test: a manifest with a bad signature is refused; a good manifest with a mismatched digest is refused; an equal or older version offers no update; a newer one does; the staged-replace path is exercised against a fixture
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: `--data-dir ./tmp-data` — install the previous release on a clean machine, let it find the current one, accept, and confirm the new version starts with its data intact; then point it at a tampered manifest and confirm it refuses
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** The owner has made this a requirement, not an option: it is a Phase 9 deliverable with its own exit criterion in `PROPOSAL.md` §13, and the asset contract it depends on is in §12. It stays in the Backlog until Phase 9 opens as a task version.

---

