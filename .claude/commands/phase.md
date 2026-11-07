---
description: Start, continue or finish a numbered delivery phase (0–9) end to end
argument-hint: "<number> [start|continue|finish]"
---

# /phase

Phase: $ARGUMENTS

Delivery is the ten numbered phases of
[docs/07-roadmap.md](../../docs/07-roadmap.md):

| | | | |
|---|---|---|---|
| **0** Foundation | **1** Shell | **2** Toast Core | **3** Glass and Animations |
| **4** Scheduler and Reminders | **5** Location, Prayer and Azan | **6** Weather | **7** Time Announcements |
| **8** Supervisor and Hardening | **9** Packaging and Release | | |

There is no lettered programme. If something in this repository still refers to
one, it is a leftover — say so rather than following it.

Before anything else, print a short plan from the phase's own row in the
roadmap table:

1. The phase's **Key Deliverables**, verbatim from the table.
2. Its **Exit Criteria**, verbatim from the table. These are the things that
   must be true before `docs/CHECKLIST.md` moves, so quote them rather than
   paraphrasing — the paraphrase is how a phase gets closed early.
3. What it builds on: the `ot-*` crates that already exist and what each needs
   added, rather than new crates nobody asked for.
4. The files you expect to change, and the spec that governs them —
   `docs/specs/<area>.md`, or `docs/11-prayer-and-azan.md` for Phase 5, or the
   relevant `docs/0x`.
5. What else could break, including the earlier phases' exit criteria that must
   stay true.
6. What is already done: the ledger items for this phase
   (`grep -n 'Phase:\*\* <n>' docs/TASKS.md`), the branch
   (`git log --oneline main..HEAD`), the row's state in `docs/CHECKLIST.md`,
   and anything uncommitted.

Phase 5 begins with a **spike**, not with code: verify the e-Solat endpoint and
its response shape for three zones and record the result as an ADR in
`docs/08-decisions.md` before anything is built on it.

Use [templates/phase-plan.md](../templates/phase-plan.md) for the plan's shape.

## The ledger

A phase is several items, not one. Split its deliverables into `FEAT` (and `CR`
where it changes documented behaviour) items with `/task-new`, each with a real
checklist and an **App** box saying what to click, all under one task version
heading in [docs/TASKS.md](../../docs/TASKS.md) with `**Phase:** <n>`. Then the
phase is just `/dev-task <that task version>`, and its progress is readable
without asking anyone. The heading is a batch label, not the app's release
number.

## Modes

- `start` — create the branch `feat/phase-<n>-<slug>` (for example
  `feat/phase-4-scheduler`), write or update the spec first, file the items,
  and begin.
- `continue` — pick up from the plan above; re-read the spec and the exit
  criteria before coding.
- `finish` — every item `Completed`, the gate green
  ([rules/testing.md](../rules/testing.md), all five commands), **and the
  phase's exit criteria demonstrably met in the running app** on a scratch
  `--data-dir`, in both languages. Then the spec, `README.md` and
  `README.ms.md` (with screenshots where a screen changed),
  `docs/CHANGELOG.md`, the `docs/CHECKLIST.md` row and `docs/GOTCHAS.md`. Then
  the branch flow in [rules/git.md](../rules/git.md).

`docs/CHECKLIST.md` moves only when the exit criteria are met — not when the
code compiles and not when the tests pass. Say, criterion by criterion, which
you demonstrated and how.

Phase order is the table's order. Don't begin a later phase while an earlier one
is open unless I say so. A check that cannot be made on this machine — a second
monitor, a 250% DPI display, an installed Malay voice, reference hardware for
the 60 fps animation check — goes into the item's **Notes** and into
`docs/CHECKLIST.md` as a check **not made**, never claimed.
