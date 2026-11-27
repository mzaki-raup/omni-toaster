# Spec — Toasts

**Status:** approved · **Last updated:** 2026-11-17 · **Source:** [PROPOSAL.md](../../PROPOSAL.md) §8.4–8.6
**Owner crates:** `ot-toast`, `ot-ui` · **Phases:** 2 (core), 3 (glass and animations)

## 1. Purpose

Everything the user sees when a notification fires. A toast is a borderless,
transparent, always-on-top viewport that never steals focus and never appears
in the taskbar or in Alt+Tab.

**Those last two need a Win32 call, not only a viewport builder** (FEAT-0018).
`egui::ViewportBuilder` gives borderless, transparent, topmost and unfocused,
but it leaves `WS_EX_TOOLWINDOW` clear and `WS_EX_APPWINDOW` set — a toast in
Alt+Tab with a taskbar button. `with_taskbar(false)` removes the taskbar
*button* through the shell's taskbar list; Alt+Tab membership is decided by
the extended style, so `ot_platform::toast_window` sets the style on the real
window once it exists. The window is found by title within this process, and
the title is the app's own name so a language switch cannot move it.

**The window is cut to the toast's shape** (BUG-0007, ADR-015). A rounded or
pill toast leaves its corners unpainted, and the graphics driver may offer no
transparent window to show the desktop through them, so each toast window is
given a Win32 window region from the same corner radii the renderer paints
with. Each toast's window title carries its id (`OmniToaster 7`) so each can
be cut to its own shape. The cut edge is whole pixels. It is **not** a Windows Action Center notification
— the app paints its own, which is what makes the customization possible
([01-vision-and-scope.md](../01-vision-and-scope.md) non-goals).

## 2. User stories

- As **Mei**, I want a reminder that looks native and is impossible to miss, so
  I stop missing things while heads-down at my PC.
- As **Azlan**, I want the Azan toast to stay until I acknowledge it, so I don't
  lose it to a glance away from the screen.
- As **the tinkerer**, I want to shape, colour and animate the toast myself and
  see the result before I commit to it.

## 3. `ToastSpec` is the only input

```
ToastSpec {
    content:   { header, title, body, image, timestamp, app_name, actions }
    style:     { shape, radius, width, height, background, opacity, glass,
                 border, shadow, accent }
    animation: { in_kind, out_kind, duration_ms, easing }
    behaviour: { duration, persistent, max_snoozes, screen, corner, stack_index }
}
```

The renderer draws **only** a `ToastSpec`. A `ToastSpec` is built from the
reminder and its category template, so the Settings ▸ Appearance preview and a
real toast go through the same code path and cannot diverge. Anything that
renders without going through a `ToastSpec` is a defect.

## 4. Appearance

| Setting | Options | Limits |
|---|---|---|
| Width | Pixels | 280–600 px (default 360) |
| Height | Auto or fixed | 80–300 px |
| Shape | Rectangle, rounded rectangle, pill | Corner radius 0–40 px |
| Background | Solid colour, gradient | |
| Opacity | Percent — fades the whole toast, text included (CR-0005) | 70%–100% |
| Glass effect | None, Blur, Acrylic, Mica-like (§4.2) | Rectangle only; greyed out for a rounded rectangle or a pill. Windows 10 1809+; solid fallback otherwise. Turns Windows' shadow off |
| Border | Width and colour | 0–4 px |
| Shadow | On or off — Windows' own shadow, which it draws only for the Rectangle shape (CR-0006) | Greyed out for a rounded rectangle or a pill |
| Header | Icon, title, app name, timestamp | Each toggleable |
| Body | Text (bold and italic), image | Image max 5 MB; PNG, JPG, WebP |
| Actions | Snooze, Dismiss, Open, in a row along the bottom (§4.3) | 1–3 buttons (default 2) |
| Position | One of nine presets, or a custom point, with a monitor choice | Respects the work area; a custom point is clamped into it |
| Stacking | Max visible 1–5, with a gap | |

**Minimums and maximums are enforced in the model, not only in the UI.** A
settings file edited by hand to width 20 is clamped on load, with a logged
warning.

**Text:** boxes size to content in both languages. A long word wraps, or
truncates with an ellipsis. Nothing overflows the toast. Layouts are checked
with the longest Malay string ([10-i18n.md](../10-i18n.md)).

### 4.1 The accent bar

The bar sits on **the side of the toast that faces the edge of the screen**,
and that side is square; the other sides keep the shape's rounding, or a
pill's round end (CR-0004, the owner's design):

| Position | Bar | Square | Rounded |
|---|---|---|---|
| Top Left, Left, Bottom Left | down the left | the left side | the right side |
| Top Right, Right, Bottom Right | down the right | the right side | the left side |
| Top | along the top | the top | the bottom corners |
| Bottom | along the bottom | the bottom | the top corners |
| Centre | along the top and the bottom | — | every corner |
| A custom point | as the preset whose ninth of the work area it falls in | | |

At Centre each bar follows the rounded corners round and stops where the
curve ends. A pill's corners are its whole ends, so its two bars meet in a
ring. A pill's text is inset for the curve on its round side or sides only.

### 4.2 Glass

The desktop behind the toast, blurred, under the toast's own colour — and
always a blur that Windows draws, never a picture of the desktop the app takes.
Windows 11's Mica and Acrylic backdrops draw only their flat fallback on a
window that is not active, and a toast never is, so all three kinds come from
the one call that blurs an inactive window: the accent policy's Acrylic
(ADR-016). They differ in how much of the toast's background colour lies over
the blur:

| Kind | Tint over the blur | `settings.toml` |
|---|---|---|
| None | — the solid background, as in Phase 2 | `none` (the default) |
| Blur | The background colour at 45%: the lightest glass | `blur` |
| Acrylic | The background colour at 60% | `acrylic` |
| Mica-like | The background colour at 88%: the desktop shows only as a hint of colour | `mica-like` |

Blur was first specified as "almost none" (10%). Built that way, the text —
whose colour follows the toast's own background — vanished over a desktop of
the opposite lightness, so the owner set 45% on 2026-11-18. An unknown kind in
`settings.toml` loads as `none` with a logged warning, and the rest of the
toast's style still loads.

**Rectangle only** (the owner, 2026-11-18). Windows draws the blur over the
whole window rectangle and ignores the region that cuts a rounded rectangle's
or a pill's corners, so those shapes with glass would show blurred square
corners past their edge. The Glass control is greyed out for them, with the
reason under it, like the shadow; the setting is kept and does nothing until
the shape is a Rectangle again. Windows 11's own corner rounding would clip
the blur, but only to its fixed radius, on all four corners and with its own
shadow, so it can make neither a pill nor the square side the accent bar sits
on (§4.1).

Opacity still fades the whole window, the blur included, so below 100% the
unblurred desktop shows faintly through a glass toast.

A gradient tints with its first colour, and the text colour is judged by that
colour alone. Where Windows refuses the call, or is
older than 10 1809, the toast is drawn solid and the log says why — a glass
that silently is not there is a defect. Glass and Windows' shadow exclude each
other (ADR-016), so a Rectangle with glass has no shadow and the shadow
control says so. The Settings preview shows the tint over the pane's own
colour, not a blur: a preview cannot blur what is behind the settings window,
and says so beside the control.

**Images:** validated by magic bytes, not by extension. Decoded with a pixel
limit of 4096 × 4096. Resized to a cached thumbnail. An oversize or corrupt
file is rejected with a localized message naming the reason
([06-security-and-privacy.md](../06-security-and-privacy.md)).

### 4.3 The action row

(FEAT-0038, the owner's design, 2026-11-18.) Every toast has a row of buttons
along the bottom of its content, **right-aligned**, below the body:

| Actions setting | Buttons, left to right |
|---|---|
| 1 | Dismiss |
| 2 (the default) | Snooze · Dismiss |
| 3 | Open · Snooze · Dismiss |

Dismiss is always there and always last, nearest the screen's edge on the
right-hand presets. It plays the toast's out-animation (§5).

**Open brings Settings forward** (FEAT-0042), and for a reminder's toast
writes an `opened` row to the history; landing on the reminder itself comes
with Settings ▸ Reminders (FEAT-0046). **Snooze works on a reminder's toast**
(FEAT-0043) and is drawn greyed on one with nothing to snooze — the test
toast and the preview (`Content::snoozable`). A greyed button takes no hover
highlight. Once a fire has been snoozed as many times as Settings ▸ Reminders
allows, its toast offers only Dismiss, with the line
`toast-snooze-limit` beside it in the row (`Content::note`), dim and on one
line; a test holds that both languages' lines fit whole on every shape at the
default width at 100%, 150% and 200%.

**An auto-height toast grows by the row** — 32 points, the row and the gap
above it — so the body keeps every line it had. A toast with a fixed height
keeps that height, and the body gives up the room. The window, the stack and
the placement map all use the grown height.

Each button is sized to its label with the same padding on each side, and is
at least 64 points wide so a short label is still a comfortable target. If
the row is wider than the content — the pseudo-locale on a narrow pill — the
buttons share the width equally and a label that still does not fit ends in an
ellipsis; Malay's "Tunda · Tutup · Buka" fits every shape at the narrowest
width. **Hover** fills the button more strongly; it never grows or moves it.
Colours come from the toast's own text colour, like its text: no colour is
hard-coded. The labels are `toast-action-*` (§9), carried in the spec as text
like every other string, so the Settings preview draws the same row — and the
preview's buttons are drawn, not pressed.

## 5. Animations

**In:** slide from right · slide from bottom · Genie from bottom · pop out
(scale with overshoot) · drawer · appear (fade) · dissolve · glow · alpha to
solid.

**Out:** the mirror of each in-preset, plus "shrink to tray" for Genie.

**Settings:** type, duration 150–1000 ms (default 350 ms), easing (linear,
ease-out, spring). Defaults: in and out **appear**, **ease-out**. In
`settings.toml` they are the `[toast.animation]` table — `enter`, `leave`,
`duration_ms`, `easing`, kebab-case; an unknown effect loads as `appear` and an
unknown easing as `ease-out`, each with a warning, and shrink to tray as an
in-animation becomes appear. Until an effect is built (FEAT-0032 to FEAT-0034)
it plays as a fade.

**Settings ▸ Animation** (FEAT-0036) holds them, in this order: Animation
In (the nine in-effects), Animation Out (those and Shrink to Tray), Duration
(a slider, 150–1000 ms), Easing, and Reduce Motion — a checkbox, with a line
under it saying whether Windows' own "Show animations" is on, and, when either
reduces motion, that every animation plays as a fade. **Play** fires a test
toast with the choices as they stand; the pane has no preview, since an
animation cannot be shown in a still one. Every label is a Fluent key in both
languages, laid out for the longest Malay string and the pseudo-locale.

**Reduced motion** (FEAT-0035) follows the Windows "Show animations" setting
(`SPI_GETCLIENTAREAANIMATION`, re-read every five seconds and logged when it
changes) and the `reduce_motion` setting in `[toast.animation]`: either one
makes every effect, in and out, a fade. The setting's switch is the
Animation pane's (FEAT-0036).

**Implementation.** Appear and alpha to solid are alpha ramps — alpha to
solid fades the background in behind text that is already there. Dissolve and
glow are painted with alpha masks; glow's halo lies in a transparent margin.
Genie is a mesh warp inside a window that covers its path. Every one of them
needs per-pixel alpha, which the toast window has (ADR-016).

One pure function, `ot_toast::effects::look`, decides each frame of each
effect (FEAT-0032): the window's rect, where the toast sits in it, the
transform its content is drawn with, the part of the window that shows, and
an alpha. The window is cut to the toast as transformed, intersected with
that part, so nothing outside it shows or takes a click. An out-animation is
its in-animation run backwards.

| Effect | What moves | What shows |
|---|---|---|
| Slide from right / from bottom | The **window**, from just beyond the work area's right or bottom edge to its place | Only what is inside the work area: never over the taskbar, never on the monitor next door |
| Drawer | The **toast inside its window**, out of the edge it sits against (§4.1; the centre opens from the top) | Only the part that is out |
| Pop out | The toast, **scaled** from its centre from 30% to full size, fading in as it grows | All of it; the window has a transparent margin of 8% of the toast's size on each side, room for a spring's overshoot (about 14%) |
| Appear | Nothing: a fade | All of it |
| Dissolve (FEAT-0033) | Nothing: the toast builds up **grain by grain** — 6-point cells, each with a fixed noise value made once per toast size and cached, showing once the progress passes it | The cells that show: the window's region is cut to them, joined into runs along each row so it stays cheap to rebuild every frame |
| Glow (FEAT-0033) | A **halo** in the toast's accent colour, swelling while the toast fades in and settled to nothing when it has arrived | All of it; the window has an 18-point transparent margin, and its region grows by the halo's reach while there is one |
| Genie from bottom (FEAT-0034) | The toast **pours out of a point** on the work area's bottom edge under it, in 24 horizontal bands: the top band leaves first, the bottom one last, each narrowing towards the point and stacked on the one above with no gap. egui scales evenly, so a narrow band is also a short one — a trumpet rather than macOS's bent sheet — and its edge is stepped | The bands within the work area; the window spans the whole path, for the toast's whole life, so it never changes size |
| Shrink to tray (FEAT-0034, out only) | Genie's warp, into the **tray icon** — or the work area's bottom-right corner when Windows will not say where the icon is | As Genie |
| Alpha to solid (FEAT-0033) | Only the **background**, border and accent bar fade in; the text is there from the first frame | All of it |

Every toast starts each effect invisible — beyond the edge, shut, or at alpha
0 — and rests exactly where the stack puts it. A click is brought back
through the effect's transform before it meets the buttons, so a drawer
still sliding out has no Dismiss where the resting toast's would be. When
several toasts slide in from the same edge together, they pass over one
another on the way to their places. Mid-warp a Genie's buttons are bands on a
path and take no click. Windows' shadow, on a Rectangle, waits while an effect needs
the window see-through — a halo, a background not yet solid — and appears when
the toast has arrived; a window with a transparent margin (pop, glow) never
has it.

**The timeline.** A toast enters, holds, and leaves. Its display duration
(§6) counts the hold only: the countdown starts when the in-animation ends,
and a toast that is dismissed or runs out plays its out-animation before its
window closes. A toast waiting in the stacking queue has not entered. While a
toast leaves, the ones below it move up with the same easing rather than
jumping, and its place is free at once, so a queued toast starts to enter
while it goes. A toast dismissed while it enters leaves from where it had got
to, taking only as long as it had come. Its window is kept **hidden until it
has been shaped** and given its alpha, and the in-animation starts then — a
window shown first flashes at full strength before its fade; one whose window
has not turned up within a second starts anyway, with a warning in the log.
Hovering during the entry costs the countdown nothing, since it has not
started. A toast's window is shown by the app (`toast_window::reveal`), never by egui:
showing a window, winit rewrites its whole extended style, which took away the
alpha a toast starts at and showed it at full strength for a frame (BUG-0012).
Windows' own show animation is turned off on every toast window
(`DWMWA_TRANSITIONS_FORCEDISABLED`): while it played, the screen showed it
instead of the toast's frames, and the first third of every entry was lost
(BUG-0011). The toast animates itself.

**One clock.** Every toast is painted in the same frame as the app's root
window (ADR-016), so all of them animate together, and the frame time measured
is the whole frame's. Each effect has a frame-time budget and **falls back to
fade below 30 fps** — the fallback is automatic and logged, never a visible
stutter. While a toast animates, the intervals between its last six frames
are kept; when their median is slower than 1/30 s, that toast finishes as a
fade and the next toast to arrive starts as one, and the log names the toast
and the frame time measured. A median, so a single slow frame — a window being
created — does not trigger it. In a debug build `--reduce-motion` and
`--slow-frames` (every frame counted as 50 ms) force the two for checking. 60 fps is measured on the reference machine (ADR-016), in a release
build. **There is no vsync**: with it, each window's present waited for the
next refresh in turn, and four windows held a frame to 12–32 fps. Instead a
frame is asked for every 1/60 s while anything moves, and none while nothing
does except at the moment a countdown runs out. Every animation logs, at debug
level, the frames it took and its frame rate — the measurement itself.

**Easing.** Linear, ease-out (cubic), and spring (a damped overshoot that
settles within the duration). Spring applies to motion and scale; an alpha
never overshoots past solid.

## 6. Duration

- Default 8 seconds, range 3–60 seconds, set in Settings ▸ Appearance ▸
  Display Duration in whole seconds (FEAT-0026).
- A persistent option for urgent types (Alarm, Azan), capped at 10 minutes.
  An Alarm's toast is persistent (FEAT-0044; Azan follows in Phase 5): it
  stays until a button is pressed, up to the cap, and one that reaches the
  cap untouched is recorded as `missed` in the history. Every other type
  keeps the display duration.
- Hovering pauses the countdown, a persistent toast's included.

## 7. Placement, DPI and monitors

A toast is placed on **a chosen monitor**, at **one of nine presets or at a
custom point**, inside that monitor's **work area** — never the full monitor
rect, so a toast never covers the taskbar.

### 7.1 The nine presets

**Four corners, four edges and the centre.** The same nine serve every
monitor:

```
┌─────────────────────────────────────────────┐
│ Top Left            Top            Top Right │
│                                              │
│ Left              Centre              Right  │
│                                              │
│ Bottom Left       Bottom       Bottom Right  │
└─────────────────────────────────────────────┘
```

| Preset | Horizontally | Vertically |
|---|---|---|
| **Top Left** | against the left edge | against the top edge |
| **Top** | centred | against the top edge |
| **Top Right** | against the right edge | against the top edge |
| **Left** | against the left edge | centred |
| **Centre** | centred | centred |
| **Right** | against the right edge | centred |
| **Bottom Left** | against the left edge | against the bottom edge |
| **Bottom** | centred | against the bottom edge |
| **Bottom Right** | against the right edge | against the bottom edge |

`Bottom Right` is the default: it is where Windows puts its own notifications,
so it is where people already look.

"Against an edge" means a **margin of 16 px** from it, scaled with the
monitor's DPI like every other measurement. "Centred" means centred on that
axis, with no margin involved.

### 7.2 A custom point

The user may place the toast anywhere in the work area instead. The point is
set two ways, and both drive the same value:

- by **dragging the preview** in Settings ▸ Appearance onto a small map of the
  chosen monitor;
- by typing **X and Y** beside it, in percent.

**Stored as a fraction of the work area, not as pixels.** A point kept as
`(1850, 1010)` is wrong the moment the screen resolution changes, the taskbar
moves, or the toast is shown on a different monitor; a point kept as
`(0.96, 0.94)` means the same place on any of them. The fraction addresses the
toast's **anchor corner**, which is the corner nearest the point, so a toast
placed near the bottom-right grows up and left rather than off the screen.

A custom point is **clamped so the whole toast stays inside the work area**.
Clamping happens at paint time against the monitor in use, not once at save
time, because the monitor can change underneath it.

**There is one custom point, not one per monitor.** It is a fraction, so it
already means the same place on any screen, and a user who wants one place to
look should not have to say where twice. Moving the toast to another monitor
keeps it in the same relative spot there.

### 7.3 Stacking direction

Stacking runs **toward the centre of the work area**, away from the nearest
edge, so a stack never walks off the screen:

| Position | Toasts stack |
|---|---|
| `Top Left`, `Top`, `Top Right` | downward |
| `Bottom Left`, `Bottom`, `Bottom Right` | upward |
| `Left`, `Centre`, `Right` | downward |
| A custom point | away from whichever horizontal edge the anchor is nearer; downward when it is in the vertical middle |

### 7.4 Everything else

- Correct at 100%–250% DPI, per monitor, including a DPI change mid-session and
  a monitor being unplugged while a toast is visible. When the chosen monitor
  disappears, toasts move to the primary monitor and say so in the log — they
  are not left on a screen that is no longer there.
- When more toasts fire than `max visible`, the surplus queues and shows as the
  earlier ones retire.
- **The monitor choice** is Settings ▸ Appearance ▸ Monitor: Primary Monitor
  (the default, whichever Windows calls primary at the time) or an attached
  monitor, listed by display number, its own name and its size. A choice is
  remembered by the monitor's device path, which survives a reboot and a
  re-plug; the `\\.\DISPLAYn` name does not. A chosen monitor that is not
  connected stays chosen and is shown as not connected.
- The position, the custom point and the monitor choice are global settings,
  not per-category: a user who wants to know where to look should only have to
  learn one place.

## 8. Templates

Each category — Azan per prayer, weather per condition, each reminder type —
has a template carrying title, body, **icon**, **image**, **accent colour**,
sound and style overrides, inheriting from a global default. Built-in default
text exists in both languages, and the editor offers "Use default text" to
restore the built-in text in the current language. Only user-written text is
stored ([05-data-model-and-storage.md](../05-data-model-and-storage.md)).

**Every notification can differ** in title, body, icon, image and accent
colour; Settings ▸ Appearance holds only the *global style* every notification
inherits — shape, size, opacity, shadow, the default accent, and where toasts
appear. The two pictures mean different things and are never interchangeable:

| | Where | When absent |
|---|---|---|
| **Icon** | The header, beside the title | The app's own icon |
| **Image** | Inside the content, beside the body text, with an optional border | No image; the body takes the full width |
| **Accent** | The bar on the side facing the screen edge (§4.1) | The default from Settings ▸ Appearance |

As built for the prayers (FEAT-0059): migration 005 seeds one template per prayer
(`fajr`, `syuruk`, `dhuhr`, `asr`, `maghrib`, `isha`), empty, so the built-in text
and picture show until the user writes their own. Settings ▸ Prayer ▸ Prayer
Templates edits a prayer's **title**, **message**, **accent** and **picture** (from
the image library); an empty field is the built-in, and "Use Default Text" clears
the title and message so the built-in text in the active language shows again.
What is typed is stored as typed with the language it was written in, and never
translated. The built-in picture is the masjid picture of that prayer (Syuruk
borrows Dhuha's). The fire and the editor's **Preview Toast** both come through
`reminder_toast::prayer_spec`, so they cannot disagree. The toast's note, its bottom line,
is the place the times are for (FEAT-0083); the day's source label is a fact about the
times and is shown in Settings ▸ Prayer, not on the toast. A per-prayer **style** override is not built.

**The place (FEAT-0075, FEAT-0076, FEAT-0083).** A prayer's toast, the reminder ahead of
it and a weather toast each say where they are for as the toast's **bottom line** — its
note, dim like every note, beside Dismiss — on **two rows**: the name, with the JAKIM zone
code in brackets when the times are JAKIM's, over the coordinates to two decimals
("Kuala Lumpur (WLY01)" over "3.15, 101.69"; a weather toast has no code). The two rows
are drawn at 10 points so both fit the button row; `reminder_toast::place_line` builds
them and `actions::note_rows` and `note_size` draw them. A place typed as coordinates is
named by them, so it is one row (the coordinates and the code), not the same numbers
twice. It is data, never translated; only the **name** is cut (at 26 characters, with an
ellipsis), so the code and the coordinates always show. A toast with no place has no
bottom line. Nothing is added to the body, so a user's own message is exactly as typed
and a long one is not pushed off by it. (Before FEAT-0083 the place was the body's last
line and the prayer's note was the source label.)

### 8.1 The image library

Pictures are uploaded **once** into a library and then **reused** by any
template that wants them — a meeting picture for the meeting reminder, a
mosque for each Azan, an alarm clock for alarms, a clock face for the hourly
announcement. A template refers to a picture in the library; it never carries
a copy of its own.

- **Upload** goes through the import path of §4 and
  `docs/06-security-and-privacy.md` unchanged: size first, magic bytes, a
  bounded decode, a content-hashed copy under `OmniToasterData/assets/`.
  Because the copy is named by its hash, the same picture uploaded twice is
  stored once.
- Each entry has a **label** the user gives it ("Mosque", "Meeting"), so a
  template's picker reads as a list of names with thumbnails rather than a
  list of hashes.
- The library is **one list for both uses**: any picture may serve as an icon
  in one template and an image in another.
- **Removing** a picture that a template still uses asks first and names the
  templates; the templates then fall back as the table above says, rather
  than pointing at a file that is gone.
- Every picker that chooses a picture — the icon and image of every template —
  chooses **from the library**, with "Upload a New Picture" at the end of the
  list so nobody has to leave the editor to add one.

### 8.2 Built-in pictures and sounds

OmniToaster ships a default picture for each prayer, each weather condition and
three of the reminder types, so a notification looks right before the user
imports anything. The files, their categories and their provenance are
`assets/README.md`; this is how they behave.

- **Embedded in the exe, not copied into the data folder.** The portable
  folder stays the user's own, nothing is written on first run, and a picture
  can never go missing from under a template that names it. 19 pictures at
  256 × 256 WebP, 326 KB in all, with a test that holds the budget.
- **Named by a key, `@builtin/<name>`**, which stands where a library picture's
  relative path would, as the app's own icon's `@app-icon` does. It resolves
  only against the embedded table — never against the disk, so a key cannot be
  made to name a file. A key this build does not have (from a newer one) is
  treated as no picture.
- **One list for both uses**, like the library: any built-in picture may be a
  header icon in one template and the body image in another. Settings ▸
  Appearance's pickers list the user's own pictures, then the built-in ones
  under a heading each — Prayer Times, Weather, Reminders — then "Upload a New
  Picture". Settings ▸ Picture Library lists them read-only below the user's
  own: they cannot be renamed or removed, and so are never asked about.
- **Two sizes.** A toast's image is drawn up to 72 points (180 pixels at 250%)
  and so is decoded to 192 pixels; the header icon is 20 points and stays at
  64, because a large texture shrunk twenty-fold without mipmaps shimmers.
- **Names are Fluent keys** (`settings-builtin-<name>`), in both languages,
  with the prayers carrying both of their names like everywhere else in Settings.
- **Not in the `asset` table.** A built-in picture has no content hash and no
  row, and `category.image_id` references `asset(sha256)`, so a template cannot
  point at one yet. Binding them to categories is the job of the phases that
  create the templates — the reminder types (Phase 4), the prayers (Phase 5)
  and the weather (Phase 6) — and the first of them adds the column that holds
  a built-in key.
- **Sounds are staged, not embedded.** Three clips (`alarm`, `azan`,
  `azan-subuh`) sit in `assets/sound/` with their categories recorded, and are
  embedded when the audio engine (`ot-media`) can play them. The default chime
  is still to be found (`assets/README.md`).

## 9. Fluent keys

Prefix `toast-`. Built-in labels and actions only: `toast-action-snooze`,
`toast-action-dismiss`, `toast-action-open`, `toast-snoozed-until`,
`toast-app-name`. The nine positions are `toast-position-` plus the preset's
own name — `top-left`, `top`, `top-right`, `left`, `centre`, `right`,
`bottom-left`, `bottom`, `bottom-right` — and the custom option is
`toast-position-custom`. Per-category default titles and bodies live under their own
prefix (`prayer-`, `weather-`, `reminder-`). User text is never a key.

## 10. Threading

Rendering happens on the UI thread and must stay cheap: no parse, layout,
regex compile or image decode inside the frame loop without caching on a hash
of its inputs. Decoding and resizing an imported image is worker-thread work,
and the result reaches the renderer through a bounded channel drained at frame
start ([.claude/rules/threads-and-scheduling.md](../../.claude/rules/threads-and-scheduling.md)).

## 11. Verification

- Layout tests, not pixel snapshots (the owner's decision, FEAT-0024): every
  shape's content rect and every line of its text stay inside the toast at
  100%, 150% and 200%, with the longest Malay strings and the pseudo-locale;
  and `crates/ot-ui/tests/layout.rs` drives the real Appearance controls with
  `egui_kittest` — no window, no GPU — and fails if any widget runs past the
  pane, in English, Malay and the pseudo-locale, at the default width and the
  narrow minimum. Colour and paint are checked in the running app.
- The pseudo-locale (~40% longer) is `ot_i18n::set_pseudo`: every looked-up
  string bracketed and padded, for layout tests; the app never turns it on.
- Every shape at 100%, 150% and 200% DPI with no overflow — **in the running
  app**, because this is what Phase 2's exit criteria ask for.
- Each animation at 60 fps on the reference machine, and its fallback forced.
- The preview compared against a real toast, side by side.

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

## 12. Open questions

1. ~~Whether "pill" shape earns its place alongside rounded rectangle, or is
   just radius 40 under another name.~~ **Answered (FEAT-0017): it earns its
   place.** A pill's radius is half its height, so it stays a pill as the
   toast grows; a fixed radius of 40 stops being half the height the moment
   the toast is taller than 80 px. The shape decides its own radius —
   rectangle is always 0, pill is always half the height, and only the rounded
   rectangle uses the number on the slider.
2. ~~Whether Genie's mesh warp is affordable on integrated graphics, or should be
   gated behind a capability check rather than a frame-time fallback.~~
   **Answered (FEAT-0034): affordable — no capability check.** Measured on the
   reference machine (Intel Iris Xe), release build, three toasts at 350 ms:
   Genie in at 189 fps or more and shrink to tray out at 97 or more, twice. The
   frame-time fallback of FEAT-0035 is the only guard it needs.

## What is implemented (2026-11-09)

**The image library** (FEAT-0025). Settings ▸ Picture Library lists every
upload with its thumbnail and an editable name — the file's own name to
start with — and a Remove button. Uploads go through §4's import path
unchanged and are recorded in the `asset` table with their `label`; the
same picture uploaded twice keeps its first name. The icon and image
pickers of Test Toast Content choose from it, with "Upload a New Picture"
last. Removing a picture in use asks first: the test toast is named, and
templates are counted from `category.image_id` (they have no names to
list until Phase 4). A confirmed removal deletes the row — `ON DELETE SET
NULL` clears every template's reference in the same statement — and then
the file. All of it runs on the `image-library` thread with its own
connection.

**Built-in pictures** (FEAT-0027, after Phase 2 closed). 19 pictures embedded
as `@builtin/<name>` (`ot_store::builtin`), listed in both pickers and in the
Picture Library, with the toast's image decoded at 192 pixels and the icon at
64. The categories they are for are recorded (`assets/README.md`) and bound by
Phases 4 to 6; three sounds are staged under `assets/sound/` for the audio
engine.

**Settings ▸ Appearance** (FEAT-0023). Shape, radius, width, opacity, border,
shadow, background and accent colours, header switches, and the position
picker with a map of the monitor: nine dots to click for the presets, and the
toast drawn where it will land, which can be dragged anywhere for a custom
point. The preview is drawn by `Prefs::spec`, `render::frame` and
`content::draw` — the same function and the same two painters as a real toast
— and was checked side by side against one. The style is saved to the
`[toast]` table of `settings.toml` when the pointer is released.

**Stacking controls** (FEAT-0022). Settings ▸ Appearance sets how many toasts
show at once (Visible at Once, 1–5) and the gap between them (0–40 px), under the
position map. The Custom Point fields sit below the map too, so that choosing a
custom point by dragging never moves the map under the pointer.

**Text colour.** A toast's text takes the Dark palette's colours on a dark
background and the Light palette's on a light one, judged by the background's
luminance (a gradient by both ends) — never by the window's theme, which a
toast does not follow.

**Placement** (FEAT-0022, in part). The nine presets and the custom point, in
`ot_toast::placement`, worked out against the chosen monitor's work area with
no screen in sight so the arithmetic can be tested against a table — including
a monitor with a **negative** origin, which is the case a routine written
against the primary monitor's rect gets wrong. `ot_platform::monitors` asks
Windows for the work areas and reports them in points. The work area is read
every frame rather than cached, so a monitor unplugged mid-toast falls back to
the primary one. The two ways of *setting* a custom point are controls, and
land with the Appearance pane.

**The header and the body** (FEAT-0020). The category mark, the title (set 12 points
from the mark, `TITLE_GAP`, FEAT-0082), and
the app name and time on the header's right-hand side; the body beneath,
wrapped at word boundaries and truncated with an ellipsis when even wrapping
cannot fit it. The app's own labels are laid out first and get their room; the
title is user text and is what truncates. The timestamp is formatted by the
caller, because a spec holds text and never keys. Body emphasis is bold or
italic for the body **as a whole** — a markup language inside a notification
body waits until something asks for it.

**The renderer** (FEAT-0019). Shape, radius, solid and gradient background,
opacity, border, shadow and an accent bar down the leading edge, all from the
spec and nothing else. A gradient is a mesh painted over a rounded fill,
because egui has no rounded gradient fill; a pill insets its content further
than a rectangle, because its round ends would clip the first glyph. Checked
in the running app: every shape at 100%, 150% and 200%, in Malay, with nothing
overflowing.

**The viewport** (FEAT-0018; immediate since FEAT-0029, ADR-016). One viewport per toast, painted in the root window's frame, keyed on the
toast's own id so it keeps one window instead of getting a new one each frame.
Several live at once, which is what stacking needs. Hovering pauses the
countdown, and the surplus beyond `max visible` queues rather than being
dropped — a queued toast starts its countdown when it appears, not when it was
pushed, or one that waited behind four others would arrive already expired.

**`ToastSpec` and its limits** (FEAT-0017). The model in `ot_toast::spec`,
with every limit in §4's table enforced by `ToastSpec::clamped` rather than by
the UI: the model is what the renderer trusts, and a settings file edited by
hand is as real an input as a slider. Each clamp logs the field and both
values. A non-finite value takes the field's default instead, because
`f32::clamp` keeps a NaN and a NaN width draws a toast of no size.

Images are FEAT-0021; where the toast
appears is FEAT-0022, which is also where the nine presets and the custom
point of §7 are built.
