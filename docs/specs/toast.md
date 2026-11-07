# Spec — Toasts

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../../PROPOSAL.md) §8.4–8.6
**Owner crates:** `ot-toast`, `ot-ui` · **Phases:** 2 (core), 3 (glass and animations)

## 1. Purpose

Everything the user sees when a notification fires. A toast is a borderless,
transparent, always-on-top viewport that never steals focus and never appears
in the taskbar or in Alt+Tab. It is **not** a Windows Action Center notification
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
| Position | Corner or edge, with a monitor choice | Respects the work area |
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

- A chosen screen and corner, stacking toward the screen centre, never covering
  the taskbar (the work area, not the full monitor rect).
- Correct at 100%–250% DPI, per monitor, including a DPI change mid-session and
  a monitor being unplugged while a toast is visible.
- When more toasts fire than `max visible`, the surplus queues and shows as the
  earlier ones retire.

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
`toast-app-name`. Per-category default titles and bodies live under their own
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

1. Whether "pill" shape earns its place alongside rounded rectangle, or is just
   radius 40 under another name.
2. Whether Genie's mesh warp is affordable on integrated graphics, or should be
   gated behind a capability check rather than a frame-time fallback.

## What is implemented (2026-11-08)

Nothing. Phase 2 has not started; this file is the requirement.
