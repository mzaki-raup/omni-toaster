# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- The settings window (FEAT-0014): a left rail, the General pane — language, theme and Start with Windows — and a status line. Every string comes from Fluent in both languages, the fields line up in one column, and a narrow window moves each label above its field instead of squeezing it. Closing the window ends the app until the tray menu lands.
- `ot_core::settings` (FEAT-0014): `config/settings.toml`, the source of truth for configuration (ADR-007). A broken file is reported and the defaults used rather than stopping the app, an unknown key is refused instead of silently ignored, and a save goes through a temporary file so a crash cannot leave half of one.
- The language chosen in Settings is honoured at the next start, read out of `config/settings.toml` before localization comes up. `--lang` still wins over it, and the stored choice over the Windows locale.
- `ot_platform::ui_font` (FEAT-0014): Segoe UI and Consolas, read from the Windows font folder — what the app draws with now that no font is bundled (ADR-014).
- Screenshots of Settings ▸ General in both languages, the first the project has.
- Filed CR-0002: in Light mode the window's title bar stays dark, because that is Windows chrome and needs `DWMWA_USE_IMMERSIVE_DARK_MODE`.

### Fixed

- `a_second_instance_bows_out_instead_of_starting` no longer hangs: its third instance now short-circuits before the settings window like the others, so the suite stays headless. CI run 39 was cancelled by it.

### Changed

- `eframe` joins the workspace with `default-features = false` and only `glow` and `accesskit` (ADR-014). `BSL-1.0` is now an allowed licence: it arrives with it, through `arboard`'s Windows clipboard, and is permissive.
- `/tmp-data/` is git-ignored. Every rule tells a session to develop against it, and it holds a database and logs.

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
