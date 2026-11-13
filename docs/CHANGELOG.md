# Changelog

All notable changes to OmniToaster are recorded here. The release workflow prepends a section for every version it publishes, generated from the Conventional Commit subjects since the previous tag. Do not edit released sections by hand.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning:
[Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- Settings ▸ Appearance ▸ **Monitor** (FEAT-0022): toasts go to the primary monitor by default, or to one you choose from those attached, listed by number, name and size. A chosen monitor is remembered by its device path, so it survives a reboot and being plugged into the same port again; when it is not connected it is shown as such and toasts go to the primary monitor.
- The debug `--test-toast` samples: the first now carries a long title, a long app name and a long body together, so the header's sharing of one line can be looked at in the running app (FEAT-0020).
- Settings ▸ Appearance: **Visible at Once** (1–5) and **Gap Between Toasts** (0–40 px). The toast model already honoured both; nothing let the user set them.
- Settings ▸ Picture Library (FEAT-0025): upload a picture once, give it a name, and use it in any notification as its icon or its image. The same picture uploaded twice is one entry under its first name. Every icon and image picker chooses from the library, with thumbnails and Upload a New Picture at the end of the list. Removing a picture something still uses asks first and says what uses it; whatever used it falls back to the app icon or to no image, and the file is deleted with it. Migration `002_asset_label.sql` adds the name. The library runs on a thread of its own with its own database connection, so the window never waits on SQLite.
- Layout tests (FEAT-0024): the real Settings ▸ Appearance controls driven headlessly by `egui_kittest` — no window, no GPU — in English, Malay and a pseudo-locale about 40% longer, at 100%, 150% and 200%, at the default and narrow widths, failing the gate if any widget runs past its pane; the toast's own text and content rect checked the same way for every shape. `ot_i18n::set_pseudo` is the pseudo-locale.
- `ot_toast::spec` (FEAT-0017): `ToastSpec`, the only thing the toast renderer will draw, so the Settings preview and a real toast cannot diverge. Every limit in the toast spec's table — width, height, radius, opacity, border, duration, visible count, actions — is enforced on the model rather than in the UI, so a `settings.toml` edited by hand to width 20 is clamped with a logged warning instead of reaching the renderer. A non-finite value takes the field's default, because `f32::clamp` keeps a NaN.
- Every notification can carry its own **icon** (in the header; the app's own icon when it has none), **image** (inside the content beside the body, with an optional border) and **accent colour** (overriding the default). Settings ▸ Appearance holds only the global style, and its Test Toast Content section tries pictures and an accent on the preview and the test toast without saving them.
- Choosing a picture opens the Windows Open dialog on a worker thread, so the settings window, the tray and every toast keep running while it is open; the import runs there too and its result arrives through a bounded channel.
- Filed FEAT-0025, the image library: upload pictures once, label them, and use them in any notification template — a meeting picture, a mosque, an alarm, the hourly announcement.
- Settings ▸ Appearance (FEAT-0023): the toast's shape, corner radius, width, opacity, border, shadow, background and accent colours, header switches, and its position — the nine presets, or a custom point dragged on a map of the monitor or typed as percentages. A live preview drawn by the same code as a real toast, and a button to show one. Saved to `settings.toml` when the pointer is released.
- Glyphs the UI font lacks now draw: Segoe UI Symbol is loaded as a fallback, so the window shows the same bell and clock the tray menu does instead of `?`.
- Importing an image (FEAT-0021): the path `docs/06-security-and-privacy.md` fixes, in its order — the size limit from the directory entry before a byte is read, the format from magic bytes rather than the extension, canonicalized source and destination, a copy under a content-hashed name with a relative path stored, and a bounded decode refused on the header so a 50000 × 50000 PNG never allocates. Each refusal names the rule it broke, in both languages. The decoder is `image` with only PNG, JPEG and WebP enabled.
- Toast placement (FEAT-0022): the nine presets and the custom point, placed against the chosen monitor's work area and never the full monitor rect, so a toast never covers the taskbar. Stacks grow away from the nearest edge. `ot_platform::monitors` reports each monitor's work area in points, read fresh every frame so a monitor unplugged while a toast is up falls back to the primary one.
- A toast's header and body (FEAT-0020): the category mark, the title, the app name and the time, each toggleable, with the body wrapped at word boundaries and truncated with an ellipsis when it still will not fit. Nothing overflows, in either language. The time follows the language — `9:37 pm` in English, `21:37` in Malay — because it goes through the one formatter.
- The toast renderer (FEAT-0019): rectangle, rounded rectangle and pill, with corner radius, solid and gradient backgrounds, opacity, border, shadow and an accent bar down the leading edge — all drawn from the `ToastSpec` and nothing else. Checked in the running app at three scales in Malay, with nothing overflowing.
- The toast viewport (FEAT-0018): a borderless, transparent, always-on-top window that never takes focus and appears in neither the taskbar nor Alt+Tab. One per toast, several at once, with the surplus beyond the visible count queuing rather than being dropped and hovering pausing the countdown. The tray's Test Toast is enabled now that it has something to show, and a debug-only `--test-toast` fires one at startup.
- `ot_platform::toast_window`: the Win32 call that actually keeps a toast out of Alt+Tab. `ViewportBuilder::with_taskbar(false)` removes the taskbar button but leaves the extended style alone, so the window stayed in Alt+Tab with `WS_EX_APPWINDOW` set.
- Phase 2 is filed as FEAT-0017 to FEAT-0024 under task version v0.2.0.
- Toast placement is specified as **nine presets plus a custom point** (`docs/specs/toast.md` §7, FEAT-0022): four corners, four edges and the centre of the chosen monitor's work area, with Bottom Right the default, or any point the user drags the preview to or types as a percentage. A custom point is stored as a fraction of the work area rather than as pixels, so it means the same place after a resolution change or on another monitor, and it addresses the toast's anchor corner so a toast near an edge grows inward. The nine position names are in both catalogues.

### Changed

- Opacity fades the whole toast, text included, so it is see-through rather than darkened, and its range is now 70–100% (was 40–100%) so text stays readable (CR-0005). A setting below 70% is raised to 70%.
- The accent bar sits on the side of the toast that faces the screen's edge, and that side is square (CR-0004): on the right at the default Bottom Right, on top at Top, along the top and bottom at Centre — following the rounded corners there — and for a custom point, on the side of the ninth of the screen it is in.

### Fixed

- Rounded and pill toasts no longer have black corners (BUG-0007). Each toast window is cut to its own shape, because the graphics driver offers no transparent window to draw into.
- A toast's text colour now follows the toast's own background, not the window's theme. In the Light theme the title and body were near-black on the default dark toast, on the preview and on every real toast alike.
- Dragging on the position map moves the toast with the pointer: the Custom Point row now sits under the map, where appearing mid-drag no longer pushes the map down. Its two fields show one decimal each instead of whatever the drag left.

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
