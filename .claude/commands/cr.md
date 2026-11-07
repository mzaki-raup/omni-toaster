---
description: Work a change request — spec edit first, then the change, then a test that the old behaviour is gone
argument-hint: "<CR-0007 | the change to existing behaviour>"
---

# /cr — change request

Request: $ARGUMENTS

A `CR` is a deliberate change to behaviour that already works and is already
documented: a changed default, a renamed label, a tightened guard, a dropped
option. It is not a defect (that's `/bug`) and not new capability (that's
`/feat`).

> Looking for the code review? That is now `/review`. `CR` here means change
> request, matching the `CR-` ids in [docs/TASKS.md](../../docs/TASKS.md).

## 0. The ledger item

An id → work that block in `docs/TASKS.md`. A description → file it with
`/task-new cr <title>` first. Set **State** to `In-Progress`.

## 1. Find what documents the current behaviour

The tool's spec in `docs/specs/`, the relevant `docs/0x`, the Fluent strings,
and any ADR that decided it. **If an ADR decided this, changing it needs a new
ADR** that supersedes it — not a silent edit.

Say plainly what the current behaviour is, what it becomes, and who or what
depended on the old one (other crates, `settings.toml`, stored reminder rows,
a built-in key the database records, an imported asset's relative path, a
Fluent key, muscle memory).

## 2. Spec edit in the same commit

This is the step that makes a `CR` different. The document moves **first**, so
the spec never describes behaviour the code no longer has.

## 3. Make the change

Smallest change that works, surgical. If the old behaviour was persisted —
a setting in `settings.toml`, a reminder row, a built-in key, an imported asset
— decide and state what happens to existing data: migrated (with a numbered
migration), defaulted, or left alone. Never silently invalidate a user's data
folder, and never rename a built-in Fluent key without a migration.

## 4. Test both directions

A test asserting the new behaviour, **and** that the old behaviour is gone.
Update the tests that asserted the old behaviour rather than deleting them —
and if one of them was asserting something the spec still requires, stop and
ask: the change may be wrong.

## 5. Close the boxes

**Test** → **Gate** (`/gate`, in full) → **App**
(`cargo run -p omnitoaster -- --data-dir ./tmp-data`) → **Docs** (spec,
both READMEs if a documented behaviour or restriction changed, a `docs/CHANGELOG.md`
line — `feat:` if it adds, `refactor:`/`fix:`/`!` as the change warrants,
`docs/CHECKLIST.md` if a row's claim changed).

## 6. Ship and report

Commit with `Refs: CR-0007`; add a `BREAKING CHANGE:` footer or `!` if this
changes behaviour users or saved files rely on. Then the branch flow in [.claude/rules/git.md](../rules/git.md).

Report: what changed, what depended on the old behaviour, what happens to
existing data, and what I should look at myself.
