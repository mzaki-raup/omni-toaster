# Spec — Tray, settings window and startup

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../../PROPOSAL.md) §8.1–8.3
**Owner crates:** `ot-platform`, `ot-ui` · **Phase:** 1

## 1. Purpose

The app's only persistent presence: a tray icon, its menu, and the settings
window behind it. There is no main window in the usual sense — the app's normal
state is a tray icon and nothing else on screen.

## 2. User stories

- As **anyone**, I want the app out of my way until I need it.
- As **anyone in a meeting**, I want one click to stop all notifications for
  half an hour.
- As **anyone**, I want it to start with Windows and to still start after I move
  the folder.

## 3. Tray

- **Left-click:** open or focus the settings window.
- **Right-click menu:** Open Settings · Pause All Notifications (30 minutes · 1
  hour · until resumed) · Test Toast · Snooze Last · Quit.
- While a pause is running, the pause submenu is replaced by **Resume
  Notifications**, rather than both being present and one of them dead. The
  menu is rebuilt when the pause starts or lapses, which is the same path a
  language change takes.
- Test Toast and Snooze Last are **present and disabled** until Phases 2 and 4
  give them something to act on, so the menu's shape does not change under the
  user later.

**Windows 11 puts a new tray icon in the hidden-icons overflow**, behind the
chevron, until the user drags it onto the taskbar. That is the user's choice
to make and the app must not fiddle with it, but it means the first run looks
like nothing happened — so the first-run experience cannot depend on the icon
being visible, and anyone verifying the tray has to look in the overflow.
- **Tooltip:** the next reminder or prayer — "Zohor · 13:19" — formatted by
  `ot_i18n`.
- Every label is localized and updates **immediately** on a language change,
  which means the menu is rebuilt rather than created once.
- Each menu item carries a glyph before its label, the same glyph for the same
  verb everywhere.

The tray icon belongs to the **worker** process, so a supervisor restart
recreates it ([specs/supervisor.md](supervisor.md)). A stale icon left by a
killed worker is removed on the next start.

## 4. Pause

Pausing suppresses toasts and announcements for the chosen window. **Azan is
not suppressed** — the user who wants that turns Azan off in Settings ▸ Prayer,
which is a deliberate choice rather than a side effect of a pause.

The paused state and its expiry are shown in the tooltip and in the settings
status line, so a user cannot forget the app is muted. A pause survives a
worker restart but not a Quit.

## 5. Single instance

A named mutex through `ot_platform::single_instance`, so a second launch
focuses the existing settings window instead of producing a second tray icon
and a second scheduler. The supervisor and its worker must not collide on the
name — each role takes its own.

## 6. Settings window

An eframe window: a left rail of sections, a content pane, a status line. The
sections and the rules for every pane are in
[04-ui-design.md](../04-ui-design.md). Closing the window does not quit the
app; Quit is only in the tray menu, and it asks for confirmation when a pause
is active.

The window and the tray share one thread and one event loop: the tray is
created inside the eframe app, so winit's message pump delivers its clicks,
and the window asks to be repainted on a short interval because nothing wakes
egui when an event lands in the tray's channel. A hidden window keeps being
updated — Windows throttles it to a few frames a second, which is well inside
what a menu click needs — so the app stays answerable with nothing on screen,
without a second thread and its own message loop.

The status line is where non-modal truth lives: the current prayer source and
last fetch time, the voice fallback state, a paused notice, the last error.

## 7. Start with Windows

- A single HKCU `Run` value, written only when the user enables it, through
  `ot_platform::autostart`.
- **Refreshed on every launch**, so a moved or renamed portable folder still
  starts. This is the one registry write the app makes
  ([ADR-004](../08-decisions.md)).
- Turning it off removes the value. The toggle reflects reality — it reads the
  registry rather than trusting the settings file, so a value removed by other
  means shows as off.

## 8. Theme

Dark by default. System mode reads `AppsUseLightTheme` through
`ot_platform::system_theme` and **rechecks every 5 seconds**, so a theme change
in Windows is followed without a restart.

## 9. Logs

Daily log files with 14-day retention, configurable
([05-data-model-and-storage.md](../05-data-model-and-storage.md)). Settings ▸
Logs and Updates opens the folder and shows the retention setting. A crash
report carries the app version, the OS build, the last 200 log lines and the
panic message — **and no personal data**, which means no reminder titles and no
coordinates ([06-security-and-privacy.md](../06-security-and-privacy.md)).

## 10. Fluent keys

Prefixes `tray-` (menu items, tooltip, pause states) and `settings-` (section
names, every label, hint and validation message, the status line). Title Case
for menu items and setting names; sentence case for hints and the status line.

## 11. Verification

| Check | Asserts |
|---|---|
| Single instance | a second launch focuses rather than duplicating |
| Autostart | the value is written, refreshed on launch, removed on disable; the toggle reads the registry |
| Theme | System mode follows a Windows change within ~5 seconds |
| Language switch | the tray menu and the open settings window re-render in under one second |
| Pause | each window suppresses toasts and announcements, not Azan, and expires correctly |

Then in the running app, including the checks tests cannot make: the tray icon
after a reboot, the menu after a language switch, and the window at a narrow
size in both languages (Phase 1's exit criteria).

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

## 12. Open questions

1. Whether Quit should also be reachable from the settings window, or stay
   tray-only to make accidental exit harder.
2. Whether "Snooze Last" should be disabled rather than hidden when nothing has
   fired yet.

## What is implemented (2026-11-08)

Nothing. Phase 1 has not started; this file is the requirement.
