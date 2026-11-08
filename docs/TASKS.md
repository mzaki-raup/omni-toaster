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
Bugs : BUG-0002
Change Request : CR-0001
```

---

## v0.0.1 — Planned

Phase 0, Foundation. The Phase 0 exit criteria from `PROPOSAL.md` §13: the app
starts in English and in Malay, writes a log, the locale key parity check
passes, and CI is green. Nothing here is `Completed` until all of that holds.

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
- **State:** Pending · **Spec:** docs/05-data-model-and-storage.md · **Phase:** 0
- **Branch:** feat/phase-0-foundation
- **Depends on:** FEAT-0001
- [ ] `ot-core::paths` resolves the data root: `--data-dir <dir>` when given, otherwise `OmniToasterData/` beside the exe — never `%APPDATA%`, `%TEMP%` or the registry
- [ ] Create the subtree on first run: `config/`, `data/`, `locales/`, `assets/{images,audio/azan,audio/chimes,icons}`, `logs/crash/`, `cache/`
- [ ] Writability check at startup: write and delete a probe file in the root; on failure return a typed error that the caller can render bilingually, and exit without touching existing data
- [ ] Path canonicalization helper that refuses any path resolving outside the data root
- [ ] Test: a resolved root under a temp dir gets the whole subtree; a read-only root produces the writability error and creates nothing; `../` and an absolute path outside the root are both refused
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: `--data-dir ./tmp-data` — the folder appears with every subdirectory; point `--data-dir` at a read-only folder and the app reports it and exits instead of panicking
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** The read-only case is awkward to automate on Windows; if `icacls` cannot produce a genuinely unwritable folder in the test, the test covers the error path by injection and the real check goes in the App box.

### FEAT-0003 · Logging and the panic hook
- **State:** Pending · **Spec:** docs/03-architecture.md · **Phase:** 0
- **Branch:** feat/phase-0-foundation
- **Depends on:** FEAT-0002
- [ ] `tracing` + `tracing-appender` writing `logs/app-YYYY-MM-DD.log`, daily rotation, retention configurable with a 14-day default
- [ ] A log level from settings, overridable by `--log-level` for a session
- [ ] `std::panic::set_hook` writing `logs/crash/<timestamp>.txt` with the app version, the OS build, the panic message, the location and the last 200 log lines — and no personal data
- [ ] Retention sweep at startup deletes log files past the retention window
- [ ] Test: a span and an event reach the log file; the panic hook writes a crash file whose contents contain the panic message and no absolute user path; the sweep deletes an aged file and keeps a fresh one
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: `--data-dir ./tmp-data` — the log file appears and grows; a deliberate panic behind a debug-only trigger leaves a crash file
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** The supervisor that restarts after a crash is Phase 8. This item only has to capture the crash.

### FEAT-0004 · Fluent setup, both locale files, and locale detection
- **State:** Pending · **Spec:** docs/10-i18n.md · **Phase:** 0
- **Branch:** feat/phase-0-foundation
- **Depends on:** FEAT-0001
- [ ] `locales/en-US/omni_toaster.ftl` and `locales/ms-MY/omni_toaster.ftl`, embedded with `rust-embed`, holding the Phase 0 keys: the startup and writability errors, the log-path notice, the language names, and `format-` keys
- [ ] `ot-i18n` builds the bundles with `i18n-embed` and exposes the `fl!()` path; a missing Malay key falls back to English and logs a warning
- [ ] Locale detection: `GetUserDefaultLocaleName` in `ot-platform`, `ms-*` → `ms-MY`, everything else → `en-US`; the settings value `system` follows it and an explicit value overrides it
- [ ] User locale overrides loaded from `<data>/locales/` and validated; a broken override is ignored with a logged warning and the embedded text is used
- [ ] The single `format_datetime(dt, lang, prefs)` formatter — the only function in the workspace that formats a date, time or number
- [ ] Test: key parity — the `en-US` and `ms-MY` key sets are identical, and the test fails naming the keys that are missing from each side; both bundles parse with no syntax errors; a missing Malay key falls back to English; `format_datetime` produces the expected string in both languages for 12-hour and 24-hour
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: `--data-dir ./tmp-data` — start with the Windows display language set to English, then to Malay, and confirm the startup messages appear in each; drop a malformed `.ftl` into `<data>/locales/` and confirm the app still starts on the embedded text with a warning in the log
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** **Edit both `.ftl` files with the Edit tool, never a heredoc** — an unquoted heredoc eats `{ $placeholders }` and backticks and silently drops the message. The key parity test is what the CI parity step runs.

### FEAT-0005 · SQLite store and migration `001_init.sql`
- **State:** Pending · **Spec:** docs/05-data-model-and-storage.md · **Phase:** 0
- **Branch:** feat/phase-0-foundation
- **Depends on:** FEAT-0002
- [ ] `ot-store` opens `data/omnitoaster.db` with `rusqlite`, WAL journal, foreign keys on
- [ ] Migration runner tracked with `PRAGMA user_version`, applying numbered files in order inside one transaction each
- [ ] `001_init.sql`: `settings`, `location`, `category`, `reminder`, `snooze_log`, `notification_log`, `asset`, `weather_cache`, per `PROPOSAL.md` §9
- [ ] Every statement parameterized; no string-built SQL anywhere in the crate
- [ ] Test: a fresh database migrates to `user_version = 1` and has every table; running the migration twice is a no-op; an existing database at version 1 is left alone; a settings round trip returns what was written
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: `--data-dir ./tmp-data` — `data/omnitoaster.db` appears on first run, `PRAGMA user_version` reads 1, and a second run does not re-migrate
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** Migrations `002_locale_keys.sql` and `003_jakim.sql` belong to Phases 4 and 5. Do not write them here.

### FEAT-0006 · The `--self-test` and `--data-dir` flags
- **State:** Pending · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Branch:** feat/phase-0-foundation
- **Depends on:** FEAT-0002, FEAT-0003, FEAT-0004, FEAT-0005
- [ ] CLI parsing in `omnitoaster` for `--data-dir <dir>`, `--self-test`, `--worker` and `--version`, with a bilingual `--help`
- [ ] `--self-test` runs the headless checks and prints one flushed line per check with `ok` or `FAILED`: paths resolve and are writable, the log file is written, both locale bundles parse, key parity holds, the database migrates to the expected version, the date formatter returns a value in each language
- [ ] Exit code 0 only when every check printed `ok`; a panic inside a check becomes a `FAILED` line naming it, not a silent abort
- [ ] `--worker` is accepted and recognised now but does nothing until Phase 8
- [ ] Test: `--self-test` against a temp data dir exits 0 and its output contains every check name; a deliberately broken locale makes it exit non-zero and name the locale check
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: `cargo run -p omnitoaster -- --self-test` exits 0 with every line `ok`; `--data-dir ./tmp-data` still starts normally; `--help` reads correctly in both languages
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** Flush each line as it prints. A buffered self-test that dies mid-check tells you nothing about which check died — `docs/GOTCHAS.md` records that lesson.

### FEAT-0007 · CI skeleton
- **State:** Pending · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Branch:** feat/phase-0-foundation
- **Depends on:** FEAT-0006
- [ ] `.forgejo/workflows/ci.yml` on the owner's machine: fmt, clippy `-D warnings`, `test --workspace --locked -j 4`, locale key parity, `cargo deny check`, release build, then `--version` and `--self-test` on the built exe — every cargo step bounded with `-j 4`
- [ ] `.github/workflows/ci.yml` running the same checks on the mirror, guarded with `if: github.server_url == 'https://github.com'`
- [ ] `ci.yml` does not run on `main`; documentation-only pushes do not trigger a release
- [ ] Test: not applicable — a workflow is verified by a green run, recorded in Notes
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: not applicable — this item ships no user-visible behaviour
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** The runner **is** this machine, so no local cargo command may run while a job is. The release workflow itself, the asset contract and the updater are Phase 9 — this item only builds the checking pipeline. Another session owns `.forgejo/`, `.github/` and `scripts/`; coordinate before editing them.

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
- [ ] Rename the forge secrets: the workflows read `OT_MINISIGN_KEY` and `OT_FORGE_UPLOAD_URL`, and the GitHub mirror `OT_SIGNING_KEY_EXTRA`. The owner reports `OT_MINISIGN_KEY` is set. **Unverifiable from here** — secret names need an authenticated API call, and their values are never readable — so this box stays unticked until a release run signs successfully, which is the only honest evidence
- [ ] Test: `pwsh -c 'Invoke-Pester scripts/tests'` green; `scripts/build-and-package.ps1` produces the full asset set and `scripts/verify-release.ps1` accepts a published release. **Mostly done — see Verified below.** The box stays unticked for three reasons: the Pester tests do not exist, `verify-release.ps1` needs a published release to run against, and the signing proven so far used the owner's local key file rather than the forge secret
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: not applicable — this item ships no user-visible behaviour
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** Filed 2026-11-08 during the scaffolding realignment, when both `release.yml` files called `scripts/build-and-package.ps1` and referenced `keys/README.md` while neither existed. Both were written the same day and are ticked above; they are **parsed, not executed**, because there is no workspace to build until FEAT-0001. What is still open is `keys/minisign.pub`, the Pester tests, and the forge secret rename. **Now confirmed, and `Blocked` on the owner.** The prediction came true the moment FEAT-0001 merged: release run 5 on `main` failed. Before FEAT-0001 the release job's "Plan the release" step short-circuited on `if (-not (Test-Path Cargo.toml))` and emitted an empty version, so every later step skipped and a docs push to `main` stayed green. A `Cargo.toml` now exists and a `feat:` commit is in history, so the planner computes a minor bump to `0.1.0` and the job proceeds to the signing step, which throws because `OT_MINISIGN_KEY` is unset and `keys/minisign.pub` is absent. The failure was contained: it dies before the push and publish steps, so **no tag was pushed, no release was created, and `main` is untouched** at `71a5ac3` — verified with `git ls-remote --tags` (empty), the releases API (0 releases) and a remote/local `main` comparison. There is no tag yet, so **every further push to `main` will re-attempt `0.1.0` and fail the same way** until two things the owner must do themselves are done: generate the minisign keypair (`rsign generate --unencrypted`, per `keys/README.md`) and commit `keys/minisign.pub`, then set the `OT_MINISIGN_KEY` secret on both forges, along with `OT_FORGE_UPLOAD_URL` and the mirror's `OT_SIGNING_KEY_EXTRA`. **I did not generate the keypair:** a release signing key is the owner's to create and hold, and nothing should ever put its private half where an agent has handled it. `.forgejo/workflows/ci.yml`'s Pester step was unguarded and would have failed the very first branch push; it is guarded now, and this item removes the guard. The asset names are a contract the updater reads — see `docs/specs/updater.md` and ADR-010; renaming one needs a new ADR.

---

## Backlog

### BUG-0001 · A branch push created no `ci.yml` run — not a defect, a forge outage
- **State:** Completed · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Verified:** reproduction attempt on 2026-11-08, which did **not** reproduce
- [x] Reproduce: push a branch and check whether a workflow run is created
- [x] Establish the cause
- [x] Confirm `ci.yml` still does not run on `main`
- [x] Test: not unit-testable — settled by a real branch push
- [x] Docs: CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** Filed and closed the same day; **the original diagnosis was wrong and no code was changed.** What happened: branch `docs/realign-scaffolding` was pushed at 05:26 and reached Forgejo (confirmed at `2a720c0` with `git ls-remote`), but 20 minutes of polling showed `total_count` still 1. I suspected the `on.push` filter's `!` negation, which Forgejo 9.0.3 might not support. Pushing `feat/phase-0-foundation` disproved it: `ci.yml` triggered within seconds and ran. The real cause was a **transient outage of the forge's Actions dispatch** in exactly that window — the runner logged `fetching a task timed out after 5s` and `failed to fetch task: unavailable: 502 Bad Gateway` at 05:40, and my own API probes returned connection failures around 05:45 before recovering. The filter is fine, the runner is fine, and `release.yml` on `main` went green throughout. Kept as a closed record rather than deleted, because the next person to see an absent CI run should check the runner log and the forge before touching the workflow — which is the lesson now in `docs/GOTCHAS.md`.

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

