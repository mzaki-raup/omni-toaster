# Testing rules

## Definition of done (the gate)

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets --locked -- -D warnings
```

```bash
cargo test --workspace --locked -j 4
```

```bash
cargo deny check
```

```bash
cargo run -p omnitoaster -- --self-test
```

Then the feature exercised in the running app:

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

All of it, not just the affected test — check the other crates that call the
changed code. `-j 4` is mandatory on this machine; a full parallel build
exhausts the page file.

`cargo deny check` is in the gate because CI runs it and its result does not
depend on the branch: advisories come from the RustSec database, so a branch
that was green yesterday can fail today with no change of its own. The fix is
usually `cargo update -p <crate>` for a patch bump — but **check whether the
code actually reaches the vulnerable path and say so either way**, rather than
implying a vulnerability shipped.

## What this project tests

From `PROPOSAL.md` §11. These are the areas where a regression is expensive
because nobody notices until a prayer is missed or a toast says the wrong time.

**Scheduler math** (`ot-sched`) — DST transitions in both directions, "offset
before" (minutes, hours, days), every recurrence kind (once, daily, weekly,
monthly, yearly), the snooze limit, and the missed-reminder catch-up windows.

**Prayer times** (`ot-prayer`) —
- the parser against `tests/fixtures/jakim_sample.json`: valid JSON, missing
  fields, malformed times, unknown extra fields;
- zone mapping: state capitals land in the expected zone family, coordinates
  outside Malaysia are rejected for JAKIM mode;
- the **fallback chain with the network disabled**: fresh cache, live fetch,
  bundled dataset, calculated, nothing — each must produce the expected source
  label;
- the **JAKIM reference test** against `tests/fixtures/jakim_reference.csv` —
  three zones and five dates captured by hand from the official portal, with
  the capture date recorded. When the live format changes, the fixture and the
  parser move together and the change goes in `docs/GOTCHAS.md`;
- timezone: JAKIM times read as UTC+8 even when the PC is set elsewhere.

**The formatter** (`ot-i18n`) — both languages, Malay day and month names,
12-hour and 24-hour, UTC+8.

**Speech voice selection** (`ot-media`) — the order in `PROPOSAL.md` §6.5:
matching language tag, then any voice in the other language with a notice, then
chime-only with a setup hint. Voices are discovered, never hard-coded by name.

**Imports and paths** (`ot-store`) — the size limit, the magic-byte format
check, path canonicalization against traversal, and that an import lands under
`OmniToasterData/assets/` with a content-hashed name and a relative path stored.

**Storage** (`ot-store`) — migrations on a **fresh** database and on an
**existing** one, settings round-trip, and the reminder lifecycle end to end:
create, fire, snooze, dismiss, recur.

**Localization** — key parity between `locales/en-US/omni_toaster.ftl` and
`locales/ms-MY/omni_toaster.ftl`, both files parsing without junk, and the
pseudo-locale layout check (about 40% longer strings) catching breakage Malay
does not.

**Rendering** (`ot-toast`) — `egui_kittest` snapshots for every toast shape, in
**both languages**, using the longest Malay strings.

## What to write

- Every bug fix **starts** with a test that reproduces it, then the fix that
  makes it pass.
- Every new feature gets unit tests; cross-module behaviour gets an integration
  test.
- Synthetic clicks never form double-clicks, so test double-click paths in the
  running app.
- A test that writes a file must assert where it landed, not only that it
  succeeded.

## What never to do

- Never restructure code just to pass a test. Fix the logic. If a test looks
  wrong, explain why and ask before changing it.
- Never weaken, skip or delete a test to get green. Report what you could not
  fix.
- **Never edit a test so it skips itself.** If a test needs a network service
  that is unreachable — e-Solat, Open-Meteo, the geolocation lookup — don't run
  it, and say in the report that it was not run and why. The test stays as it
  is.
- Never claim the running-app check from a passing test suite.

Release-planner tests are Pester 5, once `scripts/tests` exists — it does not
yet, so this step is simply not run until the release scripts get their tests:

```bash
pwsh -c 'Invoke-Pester scripts/tests'
```

## Don't fight the runner

The CI runner is this machine. Don't run local cargo commands while a CI job is
running — check the forge's `/actions/tasks?limit=4` first. A running
`OmniToaster.exe` locks its own file, so stop it before a release build:

```bash
Stop-Process -Name OmniToaster -Force
```
