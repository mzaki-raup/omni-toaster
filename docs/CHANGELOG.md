# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- `--self-test` (FEAT-0006): six headless checks — paths, the log file, both locale bundles, key parity, the database migration, and the date formatter — one flushed line each, exit 0 only when all say `ok`. The check's name prints **before** it runs, so an abort leaves the dying check's name as the last line; `panic = "abort"` in the release profile means a panic cannot be caught and turned into a `FAILED` line, and the panic strategy matters more than the self-test's convenience. `docs/09` specifies the ordering and why.
- A real command line: `--help` in both languages, `-h`/`-V` aliases, `--self-test`, `--worker` (accepted, inert until Phase 8), `--data-dir`, `--log-level`, `--lang`.
- An argument starting `--` that is not a known flag is now rejected and named. A silently ignored `--data-dirr` typo would have run the app against the real `OmniToasterData/`. A test keeps `--help`'s list and the accepted set in step.
- Public `ot_i18n::message_keys`, `check_key_parity` and `probe_language`, so `--self-test` and the unit test share one parity implementation instead of each parsing the catalogues their own way.
- `help-` strings in both catalogues, and `--self-test` now runs the parity check on the built exe, not only at `cargo test` time.

### Changed

- Closed FEAT-0007, which had sat `Pending` while its CHECKLIST row read `Completed`, and set the v0.0.1 batch heading to `In-Progress` rather than `Done` — two of its items are still open.

### Fixed

- `--self-test` was reported as passing by `scripts/verify-release.ps1` from v0.1.0 onward while the flag did not exist: it was ignored and the process exited 0 for the wrong reason. The check is real from this release on.

## 0.2.0 — 2026-11-08

### Added

- Open the database and run migration 001 (3dd251ec)

## 0.1.0 — 2026-11-08

### Added

- Make the app bilingual, with locale detection (50b9d6e9)
- Log to a rotating file and capture panics (25d36d39)
- Resolve the portable data root and prove it is writable (9ce84e14)
- Commit the minisign public key (cd925f7a)
- Create the Cargo workspace and its eleven crates (3a89f8b3)

### Fixed

- Stop two tests racing on the global Fluent loader (0a84278a)

### Changed

- Update Documentations (291717e0)
- Update License (bc81fbdf)
- Proposal phase (03889588)
- Initial commit (ae85deb2)
