# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

- (nothing yet)

## 0.9.0 — 2026-11-14

### Added

- Display Duration, 3 to 60 seconds; record the owner's checks (2ec69bc5)
- Shadow is On or Off, and it is Windows' own shadow (5456a8f6)
- Opacity fades the whole toast, from 70% (878262cd)
- See-through corners, the accent bar on the edge side, a monitor choice (91e966bd)
- Stacking controls, a steadier position map, legible toast text (0f4c5351)
- The image library - upload once, name, use anywhere (0972d9d5)
- Icon, image and accent belong to each notification (31bdf2e4)
- Settings > Appearance, with a preview that is the real toast (48eacb4e)
- Import an image, the way the security spec fixes it (8cbaec70)
- Place toasts - nine presets, a custom point, stacking (63128383)
- The header, the body and the ellipsis rules (ad30b1c7)
- Paint the toast - shapes, gradient, opacity, border, shadow (7986c803)
- The toast viewport, and the Win32 call it turned out to need (4e7926f1)
- ToastSpec, with its limits enforced in the model (9dac44a9)

## 0.8.1 — 2026-11-10

### Fixed

- Wait for the log line instead of looking once (651cb3d7)

## 0.8.0 — 2026-11-09

### Added

- Switch the language while the app runs (159ac6c2)

### Security

- Bundle SQLite 3.54.1, which fixes CVE-2026-119375 (038d2e96)

## 0.7.0 — 2026-11-09

### Added

- The tray icon, its menu and pausing notifications (f8878dc0)

## 0.6.0 — 2026-11-09

### Added

- The settings window and its General pane (09ae6f9f)

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
