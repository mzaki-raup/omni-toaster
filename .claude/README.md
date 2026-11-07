# `.claude/` — agent scaffolding for Omni Toaster

Everything here is instructions for Claude Code, not application code. The root
[CLAUDE.md](../CLAUDE.md) stays the law; these files break it into pieces that
load only when they are needed, so a session doesn't carry the whole rulebook
for a one-line fix.

[`PROPOSAL.md`](../PROPOSAL.md) is the product authority — the stack, the data
layout, the feature spec and the ten delivery phases. The `docs/` tree turns it
into specs, and these files turn those into rules a session can hold.

The work itself lives in [docs/TASKS.md](../docs/TASKS.md): `FEAT-`, `BUG-` and
`CR-` items, each under one **task version** heading, each with its own
checklist. `/dev-task v0.0.1` runs everything under that heading;
`/dev-task FEAT-0012` runs one item. A task version is the owner's batch label,
not the app's release number.

## Commands (type these)

| Command | Does |
|---|---|
| [`/dev-task`](commands/dev-task.md) | Run every `docs/TASKS.md` item under a task version, or one item by id |
| [`/task-new`](commands/task-new.md) | File a new `FEAT` / `BUG` / `CR` item with the next free id |
| [`/feat`](commands/feat.md) | Implement a feature item: spec first, branch, reuse, guards, gate, app check, docs |
| [`/bug`](commands/bug.md) | Fix a bug item: prior art first, reproducing test, smallest fix |
| [`/cr`](commands/cr.md) | Work a change request: spec edit first, then the change, then a test that the old behaviour is gone |
| [`/review`](commands/review.md) | Review the current change — three reviewers in parallel; `--file` files findings as items |
| [`/gate`](commands/gate.md) | Run the full gate and report, without fixing anything |
| [`/phase`](commands/phase.md) | Start, continue or finish a numbered delivery phase (0–9) |
| [`/docsync`](commands/docsync.md) | Audit, or apply, the ledger and document updates a change owes |
| [`/gotcha`](commands/gotcha.md) | Record a mistake in `docs/GOTCHAS.md` under the right heading |

`CR` means **change request** throughout — the code review is `/review`.

## Agents (subagents, spawned by the commands or by name)

| Agent | Does | Writes code? |
|---|---|---|
| [`rust-reviewer`](agents/rust-reviewer.md) | Reviews a diff against the project's rules — scope, crate boundaries, threads, portability, inputs — most severe first | no |
| [`ui-auditor`](agents/ui-auditor.md) | Audits `ot-ui` / `ot-toast` work: Fluent in both locales, casing, Malay width, DPI and monitors, three themes, `ToastSpec`-only rendering, states | no |
| [`spec-guard`](agents/spec-guard.md) | Spec agreement plus the `TASKS` / `CHANGELOG` / `CHECKLIST` / README / `GOTCHAS` / ADR paperwork | no |
| [`task-auditor`](agents/task-auditor.md) | Hunts unsupported ticks, id drift and version placement in `docs/TASKS.md` | no |
| [`gate-runner`](agents/gate-runner.md) | Runs the gate and reports the failure with its output | no |
| [`gotcha-scout`](agents/gotcha-scout.md) | Finds prior art for a symptom before the fix is written | no |

## Rules (read the one for the area you are touching)

[`rules/00-index.md`](rules/00-index.md) maps them and lists the
non-negotiables:
[rust](rules/rust.md) · [ui](rules/ui.md) · [i18n](rules/i18n.md) ·
[threads-and-scheduling](rules/threads-and-scheduling.md) ·
[security](rules/security.md) · [testing](rules/testing.md) ·
[build-and-run](rules/build-and-run.md) ·
[docs-and-release](rules/docs-and-release.md) · [git](rules/git.md).

## Templates (fill-in forms)

[area spec](templates/spec-area.md) · [ADR](templates/adr.md) ·
[bug report](templates/bug-report.md) ·
[feature request](templates/feature-request.md) ·
[phase plan](templates/phase-plan.md).

## The short version

- **Omni Toaster is a Cargo workspace.** The binary is `crates/omnitoaster`
  (exe `OmniToaster.exe`); the library crates are `ot-core`, `ot-i18n`,
  `ot-store`, `ot-platform`, `ot-ui`, `ot-toast`, `ot-sched`, `ot-prayer`,
  `ot-weather` and `ot-media`. One concern per crate, and **all** Windows-only
  code in `ot-platform`.
- **The ledger is what is to be done.** `docs/TASKS.md` by id and task version;
  `docs/CHANGELOG.md` is what was done; `docs/CHECKLIST.md` is how far the
  phases got. Ids are never reused; commits carry `Refs: <id>`; a box is ticked
  only after that check ran and passed.
- **Specs are requirements.** `docs/specs/*.md` and `docs/01`–`docs/11` are the
  contract. Implementation that forces a design change changes the document in
  the same commit.
- **The gate is the definition of done**: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked -j 4`, `cargo deny check`,
  `cargo run -p omnitoaster -- --self-test`, plus the feature exercised in the
  running app with `--data-dir ./tmp-data`. `-j 4` is mandatory on this
  machine.
- **The UI thread never blocks and the scheduler thread owns time.** No tokio
  and no `block_on`: blocking `ureq` and other blocking work on worker threads,
  crossbeam channels between modules, bounded channels into the UI drained at
  frame start, and every fire time held and recalculated by `ot-sched`.
- **Portable.** All app data under `OmniToasterData/` beside the exe, relative
  paths only, the single HKCU `Run` value the one exception. No vault and no
  credentials — there is nothing to encrypt.
- **Every visible string through Fluent**, present in **both**
  `locales/en-US/omni_toaster.ftl` and `locales/ms-MY/omni_toaster.ftl`, added
  in the same commit, edited with the **Edit tool**, never a heredoc. Dates and
  numbers only through the one formatter in `ot-i18n`.
- **Delivery is phases 0–9** of `docs/07-roadmap.md`, worked in order, each
  closed on its own exit criteria. There is no lettered programme.
- **Read `docs/GOTCHAS.md`** for your area before fixing a bug, and add an
  entry for every new mistake you find.
- **Branch → push → CI green → merge to `main` → push.** Never merge red; the
  runner is this machine, so don't build locally while a job runs.

## Not included on purpose

No `settings.json` and no hooks: permissions and automation are the owner's
choice, and a committed allowlist would apply to everyone who clones the repo.
