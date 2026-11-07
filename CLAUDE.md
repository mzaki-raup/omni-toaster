# CLAUDE.md

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

---

## Project-specific instructions: OmniToaster

Omni Toaster is a portable Windows tray application that paints its own toast
notifications — for reminders, Azan prayer times, weather changes and hourly
time announcements. It is a Cargo workspace: the binary crate `omnitoaster`
plus the `ot-*` library crates. All of its data lives in `OmniToasterData/`
beside the exe.

The guidelines above are general. Omni Toaster's own rules live in
`.claude/rules/`, split by area so a session loads only what it needs. **Read
the ones your task touches before you start** — they are requirements, not
suggestions.

| Where | What |
|---|---|
| [.claude/README.md](.claude/README.md) | How the scaffolding fits together: commands, skills, subagents, rules, templates |
| [.claude/rules/00-index.md](.claude/rules/00-index.md) | The non-negotiables in one page, and which rules file covers which area: `rust.md`, `ui.md`, `threads-and-scheduling.md`, `security.md`, `testing.md`, `i18n.md`, `docs-and-release.md`, `build-and-run.md`, `git.md`, `workflow.md` |
| [docs/TASKS.md](docs/TASKS.md) | The work ledger: `FEAT-`/`BUG-`/`CR-` ids under task versions, each with its checklist |
| [docs/GOTCHAS.md](docs/GOTCHAS.md) | Every known mistake and its fix. Read your area's section **before** fixing a bug; add an entry for every new one |
| [PROPOSAL.md](PROPOSAL.md) | The product authority: scope, stack, feature specification, phases 0–9 |

The specs, one file per area:

| Document | Covers |
|---|---|
| [docs/01-vision-and-scope.md](docs/01-vision-and-scope.md) | Scope, personas, non-goals |
| [docs/02-features.md](docs/02-features.md) | Features with priority and effort |
| [docs/03-architecture.md](docs/03-architecture.md) | Crate layout, event bus, scheduler thread, supervisor |
| [docs/04-ui-design.md](docs/04-ui-design.md) | Settings window, tray, toast layout, theming |
| [docs/05-data-model-and-storage.md](docs/05-data-model-and-storage.md) | `OmniToasterData/`, settings file, SQLite schema, migrations, asset store |
| [docs/06-security-and-privacy.md](docs/06-security-and-privacy.md) | Threat model, import validation, what leaves the machine |
| [docs/07-roadmap.md](docs/07-roadmap.md) | Phases 0–9 (progress in `docs/CHECKLIST.md`) |
| [docs/08-decisions.md](docs/08-decisions.md) | The ADR log |
| [docs/09-ci-cd-and-releases.md](docs/09-ci-cd-and-releases.md) | CI, releases, the asset contract, the updater |
| [docs/10-i18n.md](docs/10-i18n.md) | Fluent, locale files, the shared date and number formatter |
| [docs/11-prayer-and-azan.md](docs/11-prayer-and-azan.md) | JAKIM e-Solat, zones, caching, the fallback chain, calculated mode |
| [docs/specs/](docs/specs/) | One spec per area: `toast.md`, `reminders.md`, `weather.md`, `announcements.md`, `tray-and-settings.md`, `supervisor.md`, `updater.md` |

The four that bite hardest if you skip them:

- **The gate is the definition of done**: `cargo fmt --all --check`, `cargo
  clippy --workspace --all-targets --locked -- -D warnings`, `cargo test
  --workspace --locked -j 4`, `cargo deny check`, `cargo run -p omnitoaster --
  --self-test`, **and** the feature exercised in the running app with
  `--data-dir ./tmp-data`.
- **Branch → push → CI green → merge to `main` → push.** Never merge red; the
  CI runner is this machine.
- **Specs are requirements**, and a design change updates its document in the
  same commit.
- **Every visible string is a Fluent key** present in both
  `locales/en-US/omni_toaster.ftl` and `locales/ms-MY/omni_toaster.ftl`, edited
  with the Edit tool and never a heredoc.
