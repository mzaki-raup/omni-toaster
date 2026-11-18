# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

### Fixed

- A toast no longer shows at full strength for a moment before its fade-in: showing the window through winit rewrote its style and took its transparency away, so toast windows are now shown by the app itself (BUG-0012).
- Every toast animation now shows from its first frame: Windows' own show animation, which hid about the first 300 ms of each entry, is turned off on toast windows (BUG-0011).
- Hovering a toast pauses its countdown, as the spec always said: the pause was in the model, but nothing told it where the pointer was, so a toast under the pointer ran out anyway (BUG-0009).

### Changed

- Malay: the Dismiss button reads "Tutup" (close) instead of "Tolak" (reject), matching the spec (FEAT-0038).
- Toast windows are painted in the app's own frame, and DWM composites their alpha, so whatever a toast leaves unpainted shows the desktop — the groundwork for glass and animations (FEAT-0029, ADR-016). They look exactly as before.
- The app renders without vsync and paces its own frames, because with vsync each toast window waited for its own screen refresh and three toasts animated at 11–16 fps; release builds now animate at 137 fps or more (FEAT-0031, ADR-016).
- `cargo deny` ignores RUSTSEC-2026-0543 (`chrono` unmaintained, not vulnerable, no upgrade) with its reason; moving off `chrono` before Phase 4 is CR-0007.

### Added

- Settings ▸ Animation: the in and out animations, their duration and easing, and Reduce Motion with Windows' own setting stated under it; Play fires a test toast with them. English and Malay (FEAT-0036).
- Reduced motion: when Windows' "Show animations" is off, or `reduce_motion` is set in `[toast.animation]`, every toast animation plays as a fade. And a frame-time fallback: an animation that falls below 30 fps finishes as a fade, the next toast starts as one, and the log says so (FEAT-0035).
- Genie from bottom, a toast pouring out of the bottom edge into place in bands, and its mirror; and shrink to tray, Genie's way out into the tray icon, or the screen's corner when Windows will not say where the icon is (FEAT-0034).
- Three more toast animations, each with its mirror: dissolve (the toast builds up grain by grain), glow (a halo in the accent colour that swells and settles) and alpha to solid (the text first, the background fading in behind it). Windows' shadow on a Rectangle appears once such a toast has arrived (FEAT-0033).
- Five toast animations, each with its mirror on the way out: slide from right and slide from bottom (in from beyond the screen's working area, never over the taskbar), drawer (out of the edge the toast sits against), pop out (from the centre, overshooting with spring easing) and appear (a fade). Chosen in `[toast.animation]` until the Animation pane exists (FEAT-0032).
- Toasts have a row of buttons along the bottom, as many as the actions setting asks for (default two): Dismiss, always last, closes the toast with its out-animation, even while it is still arriving; Snooze and Open are shown greyed until reminders give them something to do. A toast that sizes to its content grows by the row (FEAT-0038).
- Toasts fade in and fade out: each enters, holds for its whole display duration — the countdown starts once it is fully in — and leaves before its window closes, and the toasts beyond a leaving one glide into its place. The `[toast.animation]` settings (effect in and out, 150–1000 ms, linear, ease-out or spring) are read already; every effect plays as a fade until it is built (FEAT-0031).
- Glass for Rectangle toasts: Settings ▸ Appearance ▸ Glass offers None, Blur, Acrylic and Mica-like — the desktop behind the toast blurred by Windows under 45 %, 60 % or 88 % of the toast's colour. Greyed out for the other shapes, with the reason; a glass toast has no shadow; drawn solid, with a line in the log, where Windows will not blur. New strings in English and Malay (FEAT-0030).
- `README.ms.md`, the README in Bahasa Melayu, and a link between it and `README.md` at the top of each (FEAT-0037).
- `assets/sound/notification.mp3`, a stand-in default chime (HTC's *Button* sound, committed on the owner's decision; Phase 9's terms check replaces it if it cannot be cleared). Not embedded until the audio engine exists.

## 0.10.0 — 2026-11-15

### Added

- Built-in pictures for prayers, weather and reminders (31f136bd)

## 0.9.1 — 2026-11-15

### Fixed

- The app keeps running with its window closed; double-click opens Settings (2bf65aa5)
- The title bar follows the theme; a killed run leaves no tray icon (568efee8)

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
