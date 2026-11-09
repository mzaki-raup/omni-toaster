# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- `ot_platform::single_instance` (FEAT-0011): a named, session-scoped mutex per role, so a second launch bows out with a localized notice and exit 0 instead of starting a second tray icon and a second scheduler. The supervisor and worker take different names so they cannot block each other.
- `ot_platform::autostart` (FEAT-0012): the single HKCU `Run` value, written only when enabled, refreshed at startup when the folder has moved, and read back from the registry rather than from `settings.toml` so a value removed elsewhere shows as off.
- `ot_platform::system_theme` and `ot_ui::theme` (FEAT-0013): the three themes — Dark by default, Light, and System, which reads the Windows `AppsUseLightTheme` setting every five seconds and follows a change without a restart. The palette is the seven semantic tokens from `docs/04-ui-design.md` for both appearances; nothing hard-codes a colour, and hover changes a control's fill or border but never its size.
- Theme names in both locales: `settings-theme-system`, `settings-theme-dark`, `settings-theme-light`.

### Changed

- `egui` is taken with `default-features = false` (ADR-014). Its `default_fonts` feature bundles the Ubuntu font under licences the project does not allow; OmniToaster will draw in the Windows UI font instead, loaded from the system by the settings window.
- `deny.toml` ignores RUSTSEC-2026-0192 (`ttf-parser` unmaintained, no upgrade available). `egui` reaches it through `ab_glyph`, and the only fonts it parses are the ones the app chooses to load — there is no path for a user-supplied font file to reach it.

### Fixed

- The v0.5.0 release could not build: `crates/omnitoaster` pinned `ot-platform = "0.4.0"` on a path dependency, so the workspace bump left the requirement behind and cargo refused to resolve it (BUG-0003). The pin is gone, and a new test fails the gate on any dependency line carrying both a `path` and a `version`.

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
