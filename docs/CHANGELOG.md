# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

### Changed

- Realigned the agent scaffolding in `.claude/` off the HyperStack project it was copied from: the rules now live in `.claude/rules/` one file per area, and every command, skill and subagent names Omni Toaster's own crates, commands and documents instead of modules that do not exist here.
- Repointed `CLAUDE.md`'s project section at the files that exist — `.claude/README.md`, `.claude/rules/00-index.md`, `docs/TASKS.md`, `docs/GOTCHAS.md` and the `docs/01`–`docs/11` spec map — and described the app as the portable Windows tray application it is. Added the Fluent-key rule to the non-negotiables and corrected the gate to name `cargo run -p omnitoaster -- --self-test`.
- Rewrote `README.md` from the proposal: the notification types, bilingual English and Bahasa Melayu with runtime switching, the three themes, toast customization, prayer times from JAKIM e-Solat with a calculated fallback, portable mode and the `OmniToasterData/` layout, requirements, build and run commands, the restrictions and non-goals, the MIT licence and the JAKIM attribution.
- Repaired the section numbering in `PROPOSAL.md`: the duplicate §12, §13 and §14 are renumbered §15 Risks and Mitigations, §16 Stakeholder Questions and §17 Next Steps, and every cross-reference now resolves. Retitled §11 to "Quality and Testing", leaving the pipeline in §12.
- Replaced the proposal's `docs/RULES/` table with the document map actually in use — the numbered `docs/01`–`docs/11` specs, the per-area specs under `docs/specs/`, the four ledger files in `docs/`, and `.claude/rules/` for the agent-facing rules — and replaced the recommended `CLAUDE.md` block with the structure the file now has. Its `docs/TASKS.md` row now describes the actual ledger — `FEAT-`/`BUG-`/`CR-` items under task versions, with never-reused ids and boxes ticked only after the check ran — and a line under the table divides TASKS, CHANGELOG and CHECKLIST so none restates another.
- Corrected the proposal's CI/CD row and branching paragraph: Forgejo Actions on the owner's own machine holds version authority, GitHub Actions runs the same checks on the public mirror, and every cargo step on the Forgejo runner is bounded with `-j 4`.
- Replaced the proposal's single-crate source layout with the Cargo workspace: the `omnitoaster` binary plus the ten `ot-*` library crates.
- Renamed the CI workflows and release scripts to Omni Toaster's asset contract: `OmniToaster.exe`, `OmniToaster-<version>-windows-x64.zip`, `manifest.json`, SHA-256 checksums and the rsign signature.
- Fixed the phase headings in `docs/CHECKLIST.md`: the second "Phase 2 — Shell" is "Phase 2 — Toast Core", and "Phase 5 — Location and Azan" is "Phase 5 — Location, Prayer and Azan".
- Pruned `docs/GOTCHAS.md` to the entries that apply here. The machine-level and toolchain-level traps are kept — the `origin` fetch refspec, merge resolution as one `&&` chain, exit-code gates, the GnuPG pair, CHANGELOG conflict markers, the paging-file and PDB failures, `-j 4` in CI, the Edition 2024 `gen` keyword, items after a test module — and the entries naming the other project's crates, release runs and test names are gone. The headings are now the fifteen areas of this project.
- Corrected the proposal's version line to 1.0 (reviewed, 2026-11-08), since the stakeholder questions in §16 are answered.

### Added

- Wrote the `docs/01`–`docs/11` specification set from the proposal: vision and scope, features, architecture, UI design, data model and storage, security and privacy, the phase 0–9 roadmap, the decision log, CI/CD and releases, i18n, and prayer and Azan.
- Wrote the per-area specifications under `docs/specs/`: `toast.md`, `reminders.md`, `weather.md`, `announcements.md`, `tray-and-settings.md`, `supervisor.md` and `updater.md`.
- Seeded `docs/TASKS.md` with the Phase 0 work under task version v0.0.1: FEAT-0001 the Cargo workspace skeleton, FEAT-0002 portable path resolution with the writability check, FEAT-0003 logging and the panic hook, FEAT-0004 the Fluent setup with both locale files and locale detection, FEAT-0005 the SQLite store and migration `001_init.sql`, FEAT-0006 the `--self-test` and `--data-dir` flags, and FEAT-0007 the CI skeleton. Nothing is ticked; no check has run.
- Filed FEAT-0008, the in-app updater, under Backlog at Phase 9, and added it to the proposal as a required Phase 9 deliverable with its own exit criterion, a row in the technology stack and a line in the Goals list.
- Added the release asset contract to the proposal's CI section, and the local gate alongside the CI pipeline.
- Built `docs/CHECKLIST.md` out of the phase plan: one row per Key Deliverable for each of phases 0 to 9, each phase's exit criteria under its own heading, and a Planning section for this realignment work. Every state is `Pending` except the planning rows that are done.


