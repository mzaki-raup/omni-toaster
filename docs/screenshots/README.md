# Screenshots and GIFs

**Last updated:** 2026-11-08

What goes in this folder, how to capture it, and when it has to be retaken.
The root [README.md](../../README.md) and the bilingual user guide are the only
consumers.

## Status

**Nine captured so far:** `toast-dark.png`, `toast-light.png` and `toast-shapes.png`, real toasts captured from the screen over the app's own settings window (so no desktop shows), from a debug build on a scratch `--data-dir`, when Phase 2 closed; and `settings-appearance.png` and `settings-appearance-ms.png` on a fresh scratch folder; `settings-pictures.png` and `settings-pictures-ms.png`, the Picture Library in both languages, taken from a debug build on a scratch `--data-dir` when FEAT-0025 landed, with three plain coloured test pictures; and `settings-general.png` and `settings-general-ms.png`,
the Settings ▸ General pane in both languages, taken from a release build on a
scratch `--data-dir` when FEAT-0014 landed. Everything else below is still a
placeholder name reserved for the phase that first produces that screen; the
README says plainly that those images do not exist yet rather than linking to
nothing.

They were captured with `PrintWindow` on the app's own window handle rather
than a desktop grab, which is what gives a crop with no desktop behind it, and
quantised with `magick <in> -strip -colors 256 -define png:compression-level=9
<out>` — both under 10 KB.

## What we keep

| File | Shows | First captured in |
|---|---|---|
| `toast-dark.png` | The test toast, rounded rectangle, over a Dark window | Phase 2 (captured) |
| `toast-light.png` | The same toast over a Light window | Phase 2 (captured) |
| `toast-glass.png` | The Acrylic/Mica glass look | Phase 3 |
| `toast-shapes.png` | Rectangle, rounded and pill side by side | Phase 2 (captured) |
| `settings-general.png` | Settings ▸ General, English | Phase 1 |
| `settings-general-ms.png` | Settings ▸ General, Bahasa Melayu | Phase 1 |
| `settings-appearance.png` | Settings ▸ Appearance with the live preview | Phase 2 (captured) |
| `settings-appearance-ms.png` | The same, Bahasa Melayu | Phase 2 (captured) |
| `settings-pictures.png` | Settings ▸ Picture Library with three pictures, English | Phase 2 (captured) |
| `settings-pictures-ms.png` | The same, Bahasa Melayu, asking before removing a picture in use | Phase 2 (captured) |
| `settings-prayer.png` | Settings ▸ Prayer with the zone picker and source label | Phase 5 |
| `settings-reminders.png` | Settings ▸ Reminders, list and editor | Phase 4 |
| `tray-menu.png` | The tray menu, both languages | Phase 1 |
| `azan-toast.png` | An Azan toast with the JAKIM source label | Phase 5 |
| `weather-toast.png` | A weather-change toast | Phase 6 |
| `anim-slide.gif` | The slide-in and slide-out animation | Phase 3 |
| `anim-genie.gif` | The Genie animation | Phase 3 |
| `flow-first-run.gif` | First run: consent, location, zone confirmation | Phase 5 |
| `flow-create-reminder.gif` | Creating a reminder and watching it fire | Phase 4 |
| `flow-language-switch.gif` | Switching language live, tray and settings | Phase 1 |
| `flow-update.gif` | The in-app updater finding and applying a release | Phase 9 |

Each screen that exists in both languages gets both, named `-ms` for the Malay
one. A screen whose Malay layout is identical in every respect may share one
image, and the table says so when that is the case.

## How to capture

1. Run the app against a **scratch** data folder, never the real one:

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

2. Seed it with the same demo content every time, so successive captures are
   comparable: one event reminder, one anniversary, one timer, Kuala Lumpur as
   the location, zone `WLY01`, Dark theme, default toast style.
3. Capture at **100% DPI** at the window's default size, unless the image is
   specifically about DPI or a narrow window — the README should not show a
   layout nobody will see.
4. Crop to the window, with no desktop background and no other application
   visible. A toast capture keeps a little surrounding desktop so its
   transparency and shadow read correctly.
5. Quantise PNGs before committing. These go in the repository and are served
   from the README, so a 4 MB screenshot is a defect.
6. GIFs: record the flow, then build the GIF with ImageMagick. Keep them under
   a few seconds and under about 2 MB; a GIF is there to show a motion, not to
   replace the text.

## When they have to be retaken

A batch that changes **what a screen looks like** retakes that screen's
screenshot, and where a flow changed, its GIF, **before the batch merges**. A
new feature with a visible screen gets a screenshot in the README, and the
README's feature text describes it too — not only the CHANGELOG
([.claude/rules/docs-and-release.md](../../.claude/rules/docs-and-release.md)).

This is checked by `spec-guard` as part of the paperwork, so a UI change with a
stale screenshot is a review finding, not an oversight.

## Attribution

Any screenshot showing prayer times carries the "Source: JAKIM e-Solat" label
as the app shows it, in the language of that capture
([11-prayer-and-azan.md](../11-prayer-and-azan.md) §1). A screenshot must never
be cropped to hide it.

## What does not go here

User-supplied media, test fixtures, the app icon (`assets/icon/`), and anything
containing a real person's data. The demo content above is the only content
that should ever appear in a committed capture.
