# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- `ot-store`: `data/omnitoaster.db` with WAL, enforced foreign keys and a `PRAGMA user_version` migration runner (FEAT-0005).
- `001_init.sql`: the eight Phase 0 tables as `STRICT`, with the partial index on `reminder.next_fire_at`.
- `rusqlite` with the `bundled` feature, so SQLite ships inside the exe; confirmed the release binary still has no dynamic CRT imports.

### Changed

- The product is now written **OmniToaster**, one word, everywhere a user can see it — the version line, the startup and error messages, the crash report header, both locale catalogues, the packaged `README.txt` and the release title (CR-0001). It already matched the executable, the data folder and the package; the two-word spelling reached users through `OmniToaster --version`. A test fails if a locale message reintroduces it. **v0.1.0 shipped with the old spelling and cannot be changed retrospectively.**
- Ticked FEAT-0009's forge-secret box: release run 14 signed `manifest.json` on the runner, and `verify-release.ps1 -Tag v0.1.0` verified it from a clean download.
- Recorded that `verify-release.ps1`'s "`--self-test` OK" line is a false positive until FEAT-0006 implements the flag.

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
