# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

### Fixed

- The GitHub mirror step now reports what it did on every path — a warning annotation and a run-summary line — so a skipped or failed mirror can no longer pass for success (BUG-0002). A failed tag push also names the likely cause: a 403 means the token is short of Contents, Workflows or Actions write, where a 404 would mean the URL is wrong.
- Filed BUG-0002: the GitHub mirror handoff skips and exits 0 when its secrets are unset, so v0.1.0 to v0.3.0 reached Forgejo while GitHub received no tags and no releases. `main` looked current on the mirror throughout, because `origin` carries GitHub as a second push url and ordinary pushes reach both forges while tags do not.

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
