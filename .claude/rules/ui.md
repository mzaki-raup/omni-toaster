# UI and UX rules

One UI language across the settings window and the toasts.
[docs/04-ui-design.md](../../docs/04-ui-design.md) is the spec; this file is the
rule set a session has to hold while writing egui code.

The framework is **eframe / egui**. Shared widgets, the theme and the settings
window live in `ot-ui`; the toast window, its styles and its animations live in
`ot-toast`.

## The two surfaces

**The settings window** (`ot-ui`) is one window with panes: General (language,
theme, accent, startup), Toasts (appearance, animation, duration, position and
stacking), Reminders (the list and the editor), Prayer (source, zone, per-prayer
templates and audio), Weather (polling, thresholds, per-condition templates),
Announcements (schedule, voice, quiet hours), and About (version, log path,
attribution, update check).

**The toast** (`ot-toast`) is a borderless, transparent, always-on-top window of
its own.

## A toast is drawn only from a `ToastSpec`

This is the rule that keeps the preview honest. A `ToastSpec` carries the
content, the style and the animation; the renderer takes a `ToastSpec` and
nothing else. The settings preview and the real toast therefore cannot diverge.

- The renderer never reads a setting, a reminder row or a category directly.
- Building a `ToastSpec` from a reminder plus its template is one function, used
  by both the preview and the scheduler.
- A style option the preview cannot show is a design bug, not a preview
  limitation.

## Theme

- Three modes: **Dark (the default)**, Light, and System (reads the Windows
  setting, rechecked every five seconds).
- A global accent colour, optionally overridden per category.
- Every pane and every toast shape must be looked at in **all three** modes. A
  hand-painted rect with a hard-coded colour is a finding.
- Hover highlights a control; it never grows or moves it. Hover changes the fill
  or draws a border — never a larger rect, a bigger font or a scaled icon.

## Text and strings

- **Every** visible string is a Fluent key present in both
  `locales/en-US/omni_toaster.ftl` and `locales/ms-MY/omni_toaster.ftl`. A
  literal passed to `label`, `button`, `on_hover_text` or a window title is a
  defect, and CI lints for it. The rules live in [i18n.md](i18n.md).
- Casing:
  - **Title Case** for button labels, menu and list items, setting names, pane
    and category labels. Minor words (a, the, of, to, from, at, in, on, and)
    stay lowercase.
  - **Sentence case** for tooltips, hints, placeholders, dialog bodies, toast
    bodies, empty states and validation messages.
  - A parenthetical attached to a label starts with a capital — "System
    (Follows Windows)". One inside a sentence stays lowercase.
- **Never format a date, a time or a number by hand.** `ot-i18n` has one
  formatter and the toast, the tray tooltip and the speech service all use it.
- **Malay runs up to 30% wider than English.** Lay a screen out so it survives
  the longest Malay string, not the English one. Check at least
  `tray-menu-pause-30` ("Jeda semua pemberitahuan selama 30 minit"), the
  announcement and quiet-hours labels, and the prayer rows, which carry both
  names ("Subuh (Fajr)").
- Text sizes to content in both languages. A long word wraps or truncates with
  an ellipsis; nothing overflows a toast or a pane. An ellipsis is for user data
  that can be any length, never for the app's own labels while there is room.

## Layout

- Nothing touches a pane edge. A pane's content is inset by the shared margin,
  and sections of a pane (a header, a form, a list, a status line) are set apart
  by a separator or a card with the shared gap above and below.
- Every row holding more than one control lines its controls up on **one middle
  line**: a label, a text field, a dropdown, a checkbox and a button in one row
  are the same height and vertically centred. Set the field height before the
  row opens — egui fixes a horizontal row's height when the row opens, so doing
  it inside the closure leaves the label sitting high.
- A description under a field goes on its own row, never in the field's cell.
- A field is wide enough for the values it holds, in Malay.
- A row of more than two buttons wraps rather than running past the pane.
- Options sharing a row are spaced far enough apart to read as separate
  choices; options in a form grid stay one per row.

## The right control for the value

| The value is | Use |
|---|---|
| on / off | a checkbox |
| one of a fixed set (theme, animation, shape, source) | a dropdown |
| a number in a range (width, radius, opacity, duration) | a slider, or a drag value beside its unit |
| a colour (accent, background, border) | the shared colour picker |
| a time of day (quiet hours, announcement times) | the shared time field |
| a date (an anniversary, a one-off event) | the shared date field |
| a JAKIM zone, a city, a voice | a searchable list — about sixty zones is too many for a plain dropdown |
| a file the user supplies (audio, an image) | the import control, which validates and copies in |
| a short text (a reminder title, a template body) | a text field |

A value is never squeezed into a weaker control (a fixed set as free text) nor
into a stronger one the user has to fight (a free title as a dropdown).

## DPI, monitors and positioning

- The app must be correct at **100% to 250%** scaling. Nothing may be sized in
  raw pixels that should scale with the font.
- Toast position is a corner or an edge of a **chosen monitor**. Work out the
  position from that monitor's work area, not from the primary monitor's, and
  handle a monitor being disconnected while a toast is up.
- Stacking is one to five visible toasts with a configurable gap; the stack must
  not run off the work area at 250%.

## States

- Every list and every fetch has an **empty**, a **loading** and an **error**
  state, and each one says something. An empty reminder list says so in a
  sentence; a failed prayer fetch names what it fell back to.
- Offline or stale data is marked as such, in both languages.
- A reduced-motion setting follows the Windows "Show animations" setting and
  switches every animation to fade only. An animation that misses its frame
  budget falls back to fade below 30 fps.

## Per-frame cost

egui is immediate mode: `ui()` runs every frame. Never parse, lay out, decode,
hash, compile a regex or format a whole list inside `ui()` without caching on a
hash of the inputs. Thumbnails are decoded once and cached. Test with a long
reminder list, not three rows.

## Icons

Verify a glyph by looking at it on screen. A codepoint being present in the font
does not mean it draws the picture you expect — say "unverified" rather than
assuming.

## Checking a screen

Tests are not enough. Run the app on a scratch data folder and look:

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

Every item below must hold before a screen is reported done. Write what you
actually checked in the ledger item's **App** box.

1. **Both languages.** Switch the language at runtime and look again. The pane
   and the tray menu re-render within a second, and no Malay string is clipped,
   wrapped mid-word or pushing a control off the pane.
2. **All three themes.** Dark, Light, and System with the Windows setting
   toggled. No hard-coded colour shows through.
3. **One height, one middle line.** Every control in a row is the same height
   and vertically centred with its label.
4. **Fields fit their values.** A zone name, a long reminder title, a file path
   and a voice name each show whole or truncate with an ellipsis, never
   mid-glyph.
5. **DPI.** 100%, 150% and 200% at least; 250% for the toast stack. Nothing
   clipped, nothing a fixed pixel size that should have scaled.
6. **Monitors.** The toast appears on the chosen monitor, in the chosen corner,
   inside its work area.
7. **Preview equals the real thing.** Change a style in Settings, look at the
   preview, then fire a test toast from the tray. They must be identical.
8. **States.** Empty, loading and error each say something, in both languages.
9. **Text.** Title Case for labels and buttons, sentence case for hints and
   toasts, every string from Fluent, no hand-formatted date or number.
10. **Narrow window.** Shrink the settings window: button rows wrap, nothing
    overlaps, nothing is cut off.
11. **Glyphs.** Every new glyph seen on screen, not assumed.

`egui_kittest` covers widget and toast snapshots, and the snapshots are taken in
both languages with the longest Malay strings — but synthetic clicks never form
double-clicks, so exercise any double-click path in the running app.
