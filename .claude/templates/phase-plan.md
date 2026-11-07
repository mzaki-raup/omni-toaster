# Template — phase plan

Write this out before starting a numbered phase of
[docs/07-roadmap.md](../../docs/07-roadmap.md), and keep it in the session (not
as a committed file — no summary file exists in this repo and none should be
created unless asked).

```markdown
# Phase <n> — <name>

**Branch:** feat/phase-<n>-<slug>
**Spec(s):** docs/specs/<area>.md · docs/0x-<…>.md
**Ledger:** docs/TASKS.md items <ids>, under `## v<x.y.z>` with `**Phase:** <n>`

## Key deliverables (verbatim from the roadmap table)

<paste the Key Deliverables cell>

## Exit criteria (verbatim from the roadmap table)

<paste the Exit Criteria cell — verbatim, not paraphrased. This is what
`docs/CHECKLIST.md` moves on, and a paraphrase is how a phase gets closed
early.>

## Crates this touches

- `ot-<name>` → what it already does, what this phase adds. Prefer adding to an
  existing crate over creating one; a new crate is an ADR.
- Anything Windows-only goes in `ot-platform` behind an interface, so clippy
  still passes on Linux.

## Steps, each with its check

1. <step> → verify: <the test, or the thing seen in the running app>
2. <step> → verify: <…>
3. <step> → verify: <…>

## Design changes this forces

Which spec sections change, and whether an ADR in `docs/08-decisions.md` is
needed — a new dependency, an external endpoint, a storage format, a change to
the thread or process structure.

## Threads and time

- What goes on the **scheduler thread**: which fire times, and what invalidates
  them (DST, system resume, a clock change, midnight, a location change, a
  successful fetch).
- What goes on a **worker thread**: the blocking fetches, the decodes, the
  hashing.
- What crosses a bounded channel into the UI, and what happens when it is full.
- Nothing blocking inside `ui()`; no `block_on`; no async runtime.

## Strings

Which Fluent prefix, roughly how many keys, and the longest Malay string the
new layouts have to survive. Both locale files move in the same commit.

## Guards

Anything this phase lets the user delete, clear, overwrite or change
irreversibly — a reminder, the history, an imported asset, the autostart value
— and how each is confirmed.

## What else could break

Other crates that call the changed code and the tests that cover them, plus the
earlier phases' exit criteria that must stay true.

## Verification plan

- Unit / integration tests: …
- Gate: the five commands in `.claude/rules/testing.md`.
- Running app (`--data-dir ./tmp-data`): what to click, **in which language,
  which theme and at which DPI**. A passing test suite never ticks this.
- **Not checkable on this machine:** <a second monitor, a 250% display, an
  installed Malay voice, reference hardware for the 60 fps animation check, a
  live e-Solat response if the endpoint is unreachable> — each of these goes
  into the item's **Notes** and into `docs/CHECKLIST.md` as a check **not
  made**, never implied.

## Exit criteria, one by one

| Criterion (verbatim) | How it was demonstrated | Met? |
|---|---|---|
| … | … | yes / no / not checkable here |

## Paperwork at the end

docs/TASKS.md items ticked and Completed · the spec · README.md and
README.ms.md (+ screenshots, + the GIF where a flow changed) ·
docs/CHANGELOG.md (Unreleased) · the docs/CHECKLIST.md row · docs/GOTCHAS.md
entries
```
