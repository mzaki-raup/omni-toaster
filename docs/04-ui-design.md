# 04 — UI design

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../PROPOSAL.md) §6, §8

Two surfaces, one visual language: the **Settings window** (a normal resizable
eframe window) and the **toast** (a borderless, transparent, always-on-top
viewport). The agent-facing rules are in
[.claude/rules/ui.md](../.claude/rules/ui.md); this file is the design.

## Themes

| Mode | Behaviour |
|---|---|
| **Dark** | The default. |
| **Light** | — |
| **System** | Reads the Windows `AppsUseLightTheme` setting through `ot_platform::system_theme`, and follows a change at runtime. |

A theme is a palette plus a set of semantic tokens (`surface`, `surface_raised`,
`text`, `text_dim`, `accent`, `border`, `danger`). Nothing hard-codes a colour;
a toast's own accent colour overrides the token only where the template says
so. Hover changes a control's fill or border — it never grows, moves or scales
a glyph.

`ot_ui::theme` owns this: `ThemeMode` is what the user chose, `Appearance` is
what it resolves to, and `Tokens` is the palette. System re-reads Windows every
`ot_platform::system_theme::POLL_INTERVAL` rather than once per frame, and a
reading Windows declines falls back to Dark, never to Light.

## Typography

No font is bundled (ADR-014). Text draws in the Windows UI font, loaded from
the system at startup — egui's `default_fonts` is off, so nothing renders
until that load succeeds and a failure to find the font is a startup error,
not an empty window.

## The Settings window

A left rail of sections, a content pane on the right, and a status line at the
bottom.

| Section | Contains |
|---|---|
| **General** | Language (System · English · Bahasa Melayu), theme, start with Windows, pause controls, single-instance notice |
| **Appearance** | Toast shape, corner radius, size, colours, transparency, glass effect, shadow — with the live preview |
| **Picture Library** | Pictures uploaded once and used by any notification: thumbnail, name, Remove, and a question before removing one in use |
| **Animation** | In and out animation per direction, duration, reduced-motion switch |
| **Notifications** | Display duration, stacking, screen and corner, quiet hours |
| **Reminders** | The reminder list and editor, categories, lead offsets, snooze defaults |
| **Prayer** | Source (JAKIM · Calculated), zone picker, per-prayer audio, pre-prayer lead, Imsak and Dhuha switches, the source label |
| **Weather** | Enable, location, units, which condition changes notify |
| **Announcements** | Schedule, voice, test button, chime fallback |
| **Location** | Detected location, manual override, the confirmation step |
| **Templates** | Per-category title, body, image, accent, sound, style overrides; "Use default text" |
| **Logs and Updates** | Log folder, retention, crash reports, update channel and the update check |
| **About** | Version, licence, the JAKIM attribution, third-party notices |

General is the only pane that exists so far: language, theme and Start with
Windows, with the status line under it. The others are listed above as the
design, not as something the window has; an empty tab promises a screen that
is not there, so a pane appears when its phase lands.

Rules for every pane:

- A row that holds more than one control lines its controls up on one middle
  line. Labels sit on the field's middle line, not above it — **except** when
  the pane is too narrow for a label and a usable field side by side, where
  the label moves above its field rather than the field being squeezed or run
  off the edge. `ot_ui::layout::columns` decides that once per pane, so every
  row of a form divides the same way and the fields line up in one column.
- A field is wide enough for the values it holds. A value is never squeezed
  into a weaker control (a yes/no as a text field, a fixed set as free text) or
  into a stronger one the user has to fight (a free name as a dropdown).
- Sections are separated by a rule or a card, with consistent spacing. Nothing
  touches a pane edge.
- Every list has an empty state that says something, every fetch has a loading
  state, and every failure has an error state with the reason.
- Every destructive action is guarded: deleting a reminder, clearing history,
  replacing imported media, turning off autostart.

### The right control for the value

| The value is | Use |
|---|---|
| on / off | a checkbox |
| one of a fixed set | a dropdown |
| a number in a range | a slider, or a drag value beside its unit |
| a colour | the colour picker, with the hex value readable |
| a duration | a drag value in seconds, clamped to the min and max |
| a file the user supplies | a file picker that copies into the asset store |
| a date or a time | the date/time field, formatted by `ot_i18n` |
| a name or short text | a single-line text field |
| many rows to read and sort | the data table |

## The toast

A toast is a separate borderless viewport. It never steals focus and never
appears in the taskbar or in Alt+Tab.

```
┌──────────────────────────────────────────┐
│ ▎ [icon]  Header                      ✕  │   accent bar, header row
│                                          │
│   [image]   Body text, up to N lines     │   content row
│                                          │
│              [ Snooze ]  [ Dismiss ]     │   action row
└──────────────────────────────────────────┘
```

- **Shape:** rectangle or rounded rectangle with a settable radius.
- **Fill:** solid, a transparency level, or a glass effect (blur · Acrylic ·
  Mica-like) with a solid fallback when the effect is unavailable or the frame
  budget is missed.
- **Size:** user-set within an enforced minimum and maximum
  ([specs/toast.md](specs/toast.md) has the numbers).
- **Placement:** a chosen screen and corner, stacking downward or upward from
  it, respecting the work area so it never covers the taskbar.
- **DPI:** correct at 100% to 250%, per-monitor, including a mid-session DPI
  change.
- **Duration:** user-set within an enforced minimum and maximum. Hovering
  pauses the dismissal countdown.
- **Actions:** Snooze and Dismiss on every toast; a category may add one more.

Everything above comes from the `ToastSpec` the renderer is handed, so the
Appearance pane's preview is the same code path as a real toast.

## Text

- **Title Case** for button labels, menu and list items, setting names, section
  and category labels. Minor words (a, the, of, to, from, at, in, on, and) stay
  lowercase.
- **Sentence case** for tooltips, hints, placeholders, dialog bodies, toasts
  and empty states.
- A parenthetical attached to a label starts with a capital — "None (Direct
  connection)". One inside a sentence stays lowercase.
- Every visible string is a Fluent key present in both locale files. No string
  is built by concatenating translated fragments, and no date or number is
  formatted by hand. See [10-i18n.md](10-i18n.md).
- Malay text runs up to 30% wider than English. Every layout is checked with
  the longest Malay string it can hold, and a pseudo-locale build (about 40%
  longer) is used to find breakage early.

## Icons

The tray icon and the in-app glyphs come from `assets/icon/`, regenerated by a
script rather than edited by hand. A glyph is unverified until someone has seen
it on screen — a codepoint existing in a font does not mean it draws the right
picture.

## Checking a screen

Run the app on a scratch data folder and look, at the default size and at a
narrow window, in both languages and all three themes:

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

1. Fields fit their values; long values ellipsize, never clip mid-glyph.
2. One height, one middle line, per row.
3. Sections separated; nothing touching a pane edge.
4. The right control for each value.
5. Title Case labels, sentence-case hints; every string from Fluent, in both
   languages.
6. Empty, loading and error states each say something.
7. Toasts: every shape, every animation, at 100%/150%/200% DPI, on each
   monitor, in both languages.
8. The preview matches the real toast.
