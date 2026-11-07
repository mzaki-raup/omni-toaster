# Working on this project

## Before coding

1. **Understand before changing.** Read the module, its tests, its callers, and
   the relevant spec in `docs/` first.
2. **Check [docs/GOTCHAS.md](../../docs/GOTCHAS.md)** for your area before
   fixing a bug, so you don't repeat a recorded mistake.
3. **Restate** the task in one or two sentences, list the files to change and
   the existing functions to reuse, and note what else could break.
4. **Ask before implementing** when requirements are unclear. State your
   assumptions, and present alternatives instead of picking one silently.

## While coding

- **Smallest change that works.** No speculative features, no "flexibility"
  nobody asked for, no refactors of working code, no dependency upgrades unless
  required.
- **Surgical edits.** Don't touch adjacent code, comments or formatting. Match
  existing style. Remove only the imports or functions your change made
  unused. Mention unrelated dead code; don't delete it.
- **Never restructure code just to pass a test.** Fix the logic. If a test
  seems wrong, explain why and ask before changing it.
- **Never weaken or delete tests** to get green. Report anything you couldn't
  fix.
- **Check side effects.** Run the full gate, not just the affected test. Check
  the other crates that call the changed code.

## After coding

1. Update the docs the change touches:
   [`docs/TASKS.md`](../../docs/TASKS.md) (the item's boxes and state),
   `README.md` and `README.ms.md`, the affected `docs/0x-*.md` and
   `docs/specs/<area>.md`, `docs/CHECKLIST.md`, and the Unreleased section of
   `docs/CHANGELOG.md`. A line for every add, change, delete or fix — see
   [docs-and-release.md](docs-and-release.md).
2. Verify the feature **in the running app**, not only in tests, before marking
   anything Completed.
3. Work on a branch. Push it, wait for CI to go green, then merge into `main`
   and push. Never merge while CI is red — see [git.md](git.md).
4. Record every new mistake you found and fixed in `docs/GOTCHAS.md`, under the
   matching heading.
5. Summarise what changed, why, and any side effects or follow-ups.

## Phases

Delivery is the ten numbered phases of
[`docs/07-roadmap.md`](../../docs/07-roadmap.md): 0 Foundation, 1 Shell,
2 Toast Core, 3 Glass and Animations, 4 Scheduler and Reminders, 5 Location,
Prayer and Azan, 6 Weather, 7 Time Announcements, 8 Supervisor and Hardening,
9 Packaging and Release. There is no lettered programme.

- Work the phases **in order**. Don't open a later phase while an earlier one
  is still open unless the owner says so.
- A phase is several ledger items under one task version heading, each with
  `**Phase:** <n>`, not one big item. `/phase <n>` drives it.
- Track each item as `Completed`, `In-Progress` or `Pending` — those three
  states only.
- Before marking a phase done: its **exit criteria from the roadmap table**
  are met, all gate checks pass, **and** the feature has been exercised in the
  running app (with `--data-dir` on a scratch folder). Update
  [`docs/CHECKLIST.md`](../../docs/CHECKLIST.md) right after that verification,
  and write down any check that could not be made on this machine rather than
  implying it passed.

## Specs are requirements

- Read the relevant spec in `docs/` before implementing a feature area.
- If implementation forces a design change, update that document **in the same
  commit**.

## Editing files safely

These have cost real work before now (`docs/GOTCHAS.md` → "Patch Scripts and
Text Editing"):

- **Locale files go through the Edit tool, never a heredoc.** An unquoted
  heredoc expands backticks and `$name`, which silently mangles Fluent
  messages — the only symptom is a parse-error count in the log and a failing
  locale test. Both `locales/en-US/omni_toaster.ftl` and
  `locales/ms-MY/omni_toaster.ftl` are affected; see [i18n.md](i18n.md).
- Prefer the Edit tool generally; if you must script an edit, write the script
  to a file and run it rather than piping a heredoc. Quote heredocs (`<<'EOF'`)
  when you do use them.
- Make each patch step idempotent, and test for a marker that exists only
  *after* that step — re-running a half-applied patch has duplicated whole
  functions.
- **Don't write Rust or Windows paths with escapes inside Python or TOML
  strings.** Use raw strings (`r"C:\Users\..."`) and TOML literal strings
  (single quotes). `"C:\Users"` fails on `\U`, and an invalid TOML escape fails
  the whole file — and the only symptom is a "TOML parse error", with the
  setting it carried silently absent.
- Insert new Rust functions **before** the `#[cfg(test)] mod tests` block;
  never append with `cat >>`.
