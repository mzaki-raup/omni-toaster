# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

- (nothing yet)

## 0.5.0 — 2026-11-09

### Added

- The three themes and Windows theme detection (b97dad32)
- Single instance and Start with Windows (a7f0af2f)

### Fixed

- Take ot-platform by path alone, so a minor bump resolves (da99e173)
- A missing Run key means autostart is off, not an error (211d5fb5)

## 0.4.1 — 2026-11-08

### Fixed

- Make the mirror step say what it did (41c82fd6)

## 0.4.0 — 2026-11-08

### Added

- Add the app icon and embed it in the executable (fa575538)

## 0.3.0 — 2026-11-08

### Added

- Implement --self-test and the real command line (56ca4373)

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
