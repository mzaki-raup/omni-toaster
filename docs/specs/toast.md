# Spec — Toasts

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../../PROPOSAL.md) §8.4–8.6
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
the title is the app's own name so a language switch cannot move it. It is **not** a Windows Action Center notification
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
| Opacity | Percent | 40%–100% |
| Glass effect | None, Blur, Acrylic, Mica-like | Windows 10 1809+; solid fallback otherwise |
| Border | Width and colour | 0–4 px |
| Shadow | Off, soft, strong | |
| Header | Icon, title, app name, timestamp | Each toggleable |
| Body | Text (bold and italic), image | Image max 5 MB; PNG, JPG, WebP |
| Actions | Snooze, Dismiss, Open | 1–3 buttons |
| Position | One of nine presets, or a custom point, with a monitor choice | Respects the work area; a custom point is clamped into it |
| Stacking | Max visible 1–5, with a gap | |

**Minimums and maximums are enforced in the model, not only in the UI.** A
settings file edited by hand to width 20 is clamped on load, with a logged
warning.

**Text:** boxes size to content in both languages. A long word wraps, or
truncates with an ellipsis. Nothing overflows the toast. Layouts are checked
with the longest Malay string ([10-i18n.md](../10-i18n.md)).

**Images:** validated by magic bytes, not by extension. Decoded with a pixel
limit of 4096 × 4096. Resized to a cached thumbnail. An oversize or corrupt
file is rejected with a localized message naming the reason
([06-security-and-privacy.md](../06-security-and-privacy.md)).

## 5. Animations

**In:** slide from right · slide from bottom · Genie from bottom · pop out
(scale with overshoot) · drawer · appear (fade) · dissolve · glow · alpha to
solid.

**Out:** the mirror of each in-preset, plus "shrink to tray" for Genie.

**Settings:** type, duration 150–1000 ms (default 350 ms), easing (linear,
ease-out, spring).

**Reduced motion** follows the Windows "Show animations" setting and switches to
fade only.

**Implementation.** Slide and pop are window position and scale updates.
Dissolve and glow are painted meshes with alpha masks. Genie is a mesh warp.
Each effect has a frame-time budget and **falls back to fade below 30 fps** —
the fallback is automatic and logged, never a visible stutter.

## 6. Duration

- Default 8 seconds, range 3–60 seconds.
- A persistent option for urgent types (Alarm, Azan), capped at 10 minutes.
- Hovering pauses the countdown.

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
- The position, the custom point and the monitor choice are global settings,
  not per-category: a user who wants to know where to look should only have to
  learn one place.

## 8. Templates

Each category — Azan per prayer, weather per condition, each reminder type —
has a template carrying title, body, image, accent colour, sound and style
overrides, inheriting from a global default. Built-in default text exists in
both languages, and the editor offers "Use default text" to restore the
built-in text in the current language. Only user-written text is stored
([05-data-model-and-storage.md](../05-data-model-and-storage.md)).

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

- Snapshot tests per shape, in both languages, with the longest Malay strings.
- A pseudo-locale build (~40% longer) to catch layout breakage.
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
2. Whether Genie's mesh warp is affordable on integrated graphics, or should be
   gated behind a capability check rather than a frame-time fallback.

## What is implemented (2026-11-09)

**Placement** (FEAT-0022, in part). The nine presets and the custom point, in
`ot_toast::placement`, worked out against the chosen monitor's work area with
no screen in sight so the arithmetic can be tested against a table — including
a monitor with a **negative** origin, which is the case a routine written
against the primary monitor's rect gets wrong. `ot_platform::monitors` asks
Windows for the work areas and reports them in points. The work area is read
every frame rather than cached, so a monitor unplugged mid-toast falls back to
the primary one. The two ways of *setting* a custom point are controls, and
land with the Appearance pane.

**The header and the body** (FEAT-0020). The category mark, the title, and
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

**The viewport** (FEAT-0018). One deferred viewport per toast, keyed on the
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
