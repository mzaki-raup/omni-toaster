# Documentation and release rules

Specs are requirements. If the implementation forces a design change, change
the document **in the same commit**.

## After every change, update what it touches

1. [`docs/TASKS.md`](../../docs/TASKS.md) — the item's boxes, ticked only for
   checks that actually ran, and its **State** (`Completed` only when every box
   is ticked). Work with no item gets one filed; see [the ledger](#the-ledger).
2. [`docs/CHANGELOG.md`](../../docs/CHANGELOG.md) — one line under
   **Unreleased** for every add, change, delete or fix, in code or deployment.
   Translation changes count.
3. `README.md` and `README.ms.md` — when features, requirements, build commands
   or restrictions change, **both** of them, each still linking to the other.
   **Screenshots and GIFs too:** a batch that changes what a screen looks
   like retakes the README's screenshots of that screen and, where a flow
   changed, its GIF animation (`docs/screenshots/`, made as
   `docs/screenshots/README.md` describes, on a scratch `--data-dir`), before
   the batch merges. A new feature with a visible screen gets a screenshot, and
   every new feature, setting or option the batch adds is described in the
   README's feature text too — not only in the changelog.
4. The affected `docs/0x-*.md` and `docs/specs/<area>.md`.
5. [`docs/CHECKLIST.md`](../../docs/CHECKLIST.md) — only after the work is
   verified. States are exactly `Completed`, `In-Progress`, `Pending`.
6. [`docs/GOTCHAS.md`](../../docs/GOTCHAS.md) — an entry under the matching
   heading for every new mistake you found and fixed.

No summary file exists. Don't create one unless asked.

## The ledger

[`docs/TASKS.md`](../../docs/TASKS.md) holds the work itself: `FEAT-`, `BUG-`
and `CR-` items, each under one task version heading, each with its own
checklist. It is **what is to be done**; `docs/CHANGELOG.md` is what was done
and `docs/CHECKLIST.md` is how far the phases got. Keep all three in step and
don't restate one inside another.

- Ids are allocated once and never reused or renumbered — commits, branches and
  changelog lines point at them. Commit footer: `Refs: FEAT-0012`.
- A box is ticked only after that check ran and passed. One that cannot be made
  on this machine stays unticked with the reason in **Notes**.
- `docs/TASKS.md`'s own `## Format` section is authoritative for the shape of an
  item; read it before filing one.

## Where things are

| Topic | File |
|---|---|
| The product authority: stack, data layout, features, phases | `PROPOSAL.md` |
| Work items by id and task version, with checklists | `docs/TASKS.md` |
| What was done, per release and under Unreleased | `docs/CHANGELOG.md` |
| How far the phases got (`Completed` / `In-Progress` / `Pending`) | `docs/CHECKLIST.md` |
| Every known mistake and its fix | `docs/GOTCHAS.md` |
| Scope, personas, non-goals | `docs/01-vision-and-scope.md` |
| Features with priority and effort | `docs/02-features.md` |
| Crate layout, the event bus, the thread model, the supervisor | `docs/03-architecture.md` |
| Settings window, panes, toast appearance, theming, animations | `docs/04-ui-design.md` |
| Data folder, `settings.toml`, the SQLite schema, migrations, the asset store | `docs/05-data-model-and-storage.md` |
| Portability, imports, parameterized SQL, logs and crash reports | `docs/06-security-and-privacy.md` |
| Phases 0–9 with their exit criteria | `docs/07-roadmap.md` |
| Decisions (the ADR log) | `docs/08-decisions.md` |
| CI, releases, the asset contract, the updater | `docs/09-ci-cd-and-releases.md` |
| Locale files, keys, the formatter, translation process | `docs/10-i18n.md` |
| Prayer sources, JAKIM zones and parsing, the fallback chain, Azan | `docs/11-prayer-and-azan.md` |
| One spec per feature area (`toast`, `reminders`, `weather`, `announcements`, `tray-and-settings`, `supervisor`, `updater`) | `docs/specs/<area>.md` |
| README screenshots and GIFs | `docs/screenshots/README.md` |
| Agent-facing rules (these files) | `.claude/rules/` |

Prayer has no file under `docs/specs/` — its spec is
`docs/11-prayer-and-azan.md`.

## Release mechanics

- The version lives once, in `[workspace.package] version` in the root
  `Cargo.toml`. Never edit it by hand; `scripts/update-version.ps1` moves it.
- Releases publish automatically from `main`. Don't tag or publish by hand.
- `ci.yml` does not run on `main`. Documentation-only pushes to `main` don't
  trigger a release.
- Release assets keep their contract: `OmniToaster.exe`,
  `OmniToaster-<version>-windows-x64.zip`, `manifest.json`, SHA-256 checksums
  and the rsign signature. The in-app updater reads `manifest.json` and
  verifies the checksum and signature before replacing the exe, so changing an
  asset name is a breaking change for every installed copy.
- Packaging happens in the release workflow, not in a local script.
- Dry runs:

```bash
pwsh -File scripts/next-release.ps1
```

```bash
pwsh -File scripts/verify-release.ps1 -Tag v0.1.0
```
