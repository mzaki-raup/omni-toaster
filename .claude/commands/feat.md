---
description: Implement a feature — ledger item, spec first, branch, reuse, guards, gate, app check, docs
argument-hint: "<FEAT-0012 | a feature description | a phase number>"
---

# /feat — implement a feature

Request: $ARGUMENTS

## 0. The ledger item

- An id (`FEAT-0012`) → read that block in
  [docs/TASKS.md](../../docs/TASKS.md), and work **its** checklist. The ledger is the
  requirement, not this conversation's paraphrase of it.
- A description → file it first with `/task-new feat <title>`, then work it.
  Nothing gets implemented without an id to tick.
- A phase number (0–9) of
  [docs/07-roadmap.md](../../docs/07-roadmap.md) → use `/phase <n>` instead; it
  is the fuller procedure, and it files the phase's items as it goes.

Set the item's **State** to `In-Progress` before you start.

## 1. Read, then restate

The area's spec in `docs/specs/` (or `docs/11-prayer-and-azan.md` for prayer),
the feature's row in `docs/02-features.md`, the crate and its tests, the
callers, and the matching `docs/GOTCHAS.md` section. Then restate in one or two
sentences, list the files to change and the existing `ot-*` functions and
`ot-ui` widgets to reuse, and name what else could break.

**Stop and ask** if the requirement is unclear or admits two readings. State
assumptions instead of inventing requirements. If a simpler approach exists,
say so.

## 2. Spec before code where design moves

The spec is the requirement. If this feature changes a design, change
`docs/specs/<area>.md` (or the relevant `docs/0x`) in the same commit; use
[templates/spec-area.md](../templates/spec-area.md) for a new feature area and
add an ADR to `docs/08-decisions.md` for a new dependency or protocol.

## 3. Branch

The item's **Branch** field, or `feat/<area>-<slug>` — and write it back into
the item. Never implement on `main`.

## 4. Build it

- Reuse before writing: the widgets and theme in `ot-ui`, the formatter in
  `ot-i18n`, the store in `ot-store`, the platform calls in `ot-platform`. One
  concern per crate; modules talk over the event bus rather than calling each
  other. Windows-only code lives in `ot-platform` and nowhere else.
- No tokio. Blocking `ureq` and other blocking work on a worker thread; results
  back over **bounded** channels drained at frame start; the scheduler thread
  owns every fire time; never `block_on` and never sleep on the UI thread — see
  [rules/threads-and-scheduling.md](../rules/threads-and-scheduling.md).
- A toast is drawn only from a `ToastSpec`, so the settings preview and the real
  toast cannot diverge. Imported media is size-limited, magic-byte checked,
  path-canonicalized and copied in content-hashed with a relative path stored.
  SQL is parameterized.
- Every visible string a Fluent key in **both**
  `locales/en-US/omni_toaster.ftl` and `locales/ms-MY/omni_toaster.ftl`, added
  with the **Edit tool** in the same commit; Title Case for labels and sentence
  case for hints and toasts; no hand-formatted date or number; empty, loading
  and error states; the layout checked against the longest Malay string.
- Unit tests for the feature, an integration test for cross-module behaviour,
  and a snapshot in both languages for anything `ot-toast` draws.

Tick each of the item's steps as it genuinely passes.

## 5. Close the four boxes, in order

1. **Test** — what the item says it asserts.
2. **Gate** — the gate in [.claude/rules/testing.md](../rules/testing.md), in
   full (or `/gate`).
3. **App** — `cargo run -p omnitoaster -- --data-dir ./tmp-data`, doing what the
   **App** box says.
4. **Docs** — spec, both READMEs (screenshot for a changed screen), one `docs/CHANGELOG.md` line
   per add/change/delete/fix, the `docs/CHECKLIST.md` row, and `docs/GOTCHAS.md` for
   anything you tripped over.

Never tick a box for a check that didn't run. A check impossible on this
machine stays unticked, with the reason in the item's **Notes**.

## 6. Ship

Commit with the id in the footer (`Refs: FEAT-0012`), Conventional Commits.
Then the branch flow in [.claude/rules/git.md](../rules/git.md): push, wait for
`ci.yml` green, merge into `main`, push. Never
merge red. Set the item's **State** to `Completed` only when every box is
ticked.

## 7. Report

What changed, why, which boxes are green, which are not and why, side effects
and follow-ups.
