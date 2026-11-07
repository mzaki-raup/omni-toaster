# 02 — Features

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../PROPOSAL.md) §8

Priority: **P0** must ship in v1.0 · **P1** wanted in v1.0, droppable · **P2**
after v1.0. Effort: **S** under a day, **M** a few days, **L** a week or more.
The **Phase** column is the phase in [07-roadmap.md](07-roadmap.md) that owns
it; the **Spec** column is where the detail lives.

## Appearance and theming

| Feature | P | Effort | Phase | Spec |
|---|---|---|---|---|
| System / Dark / Light themes, Dark default | P0 | S | 1 | [04-ui-design.md](04-ui-design.md) |
| Follow the Windows light/dark setting in System mode | P0 | S | 1 | [04-ui-design.md](04-ui-design.md) |
| Toast shapes: rectangle, rounded rectangle with a settable radius | P0 | M | 2 | [specs/toast.md](specs/toast.md) |
| Toast colour, transparency level, shadow, accent | P0 | M | 2 | [specs/toast.md](specs/toast.md) |
| Glass look: blur, Acrylic, Mica-like, with a solid fallback | P1 | M | 3 | [specs/toast.md](specs/toast.md) |
| Toast size, with enforced minimum and maximum | P0 | S | 2 | [specs/toast.md](specs/toast.md) |
| Header plus content, with user text and a user image | P0 | M | 2 | [specs/toast.md](specs/toast.md) |
| In/out animations: slide right, slide bottom, genie, pop, drawer, appear, dissolve, glow, alpha-to-solid | P0 | L | 3 | [specs/toast.md](specs/toast.md) |
| Reduced-motion and frame-budget fallback to fade | P0 | S | 3 | [specs/toast.md](specs/toast.md) |
| Display duration, with enforced minimum and maximum | P0 | S | 2 | [specs/toast.md](specs/toast.md) |
| Templates per category, inheriting from a global default | P0 | M | 2 | [specs/toast.md](specs/toast.md) |
| Live preview in Settings, rendered by the real renderer | P0 | S | 2 | [specs/toast.md](specs/toast.md) |
| Stacking, DPI 100–250%, multi-monitor placement | P0 | M | 2 | [specs/toast.md](specs/toast.md) |

## Tray, settings and startup

| Feature | P | Effort | Phase | Spec |
|---|---|---|---|---|
| Tray icon with a left-click and right-click menu | P0 | S | 1 | [specs/tray-and-settings.md](specs/tray-and-settings.md) |
| Pause all notifications for 30 min / 1 h / until tomorrow | P0 | S | 1 | [specs/tray-and-settings.md](specs/tray-and-settings.md) |
| Detailed settings window | P0 | L | 1 | [specs/tray-and-settings.md](specs/tray-and-settings.md) |
| Single instance (named mutex) | P0 | S | 1 | [specs/tray-and-settings.md](specs/tray-and-settings.md) |
| Start with Windows (HKCU `Run`, refreshed each launch) | P0 | S | 1 | [specs/tray-and-settings.md](specs/tray-and-settings.md) |
| Language selector with immediate switching | P0 | M | 1 | [10-i18n.md](10-i18n.md) |
| Quiet hours | P0 | S | 7 | [specs/announcements.md](specs/announcements.md) |

## Reminders and scheduling

| Feature | P | Effort | Phase | Spec |
|---|---|---|---|---|
| Reminder CRUD with categories | P0 | L | 4 | [specs/reminders.md](specs/reminders.md) |
| Types: event, task, anniversary, fatigue, timer, alarm | P0 | M | 4 | [specs/reminders.md](specs/reminders.md) |
| Fatigue sub-kinds: eye break, posture, hydration, work-hour limit | P0 | M | 4 | [specs/reminders.md](specs/reminders.md) |
| Lead offsets: on time, 15 min, 1 h, 1 day, 2 days, 1 week, custom | P0 | M | 4 | [specs/reminders.md](specs/reminders.md) |
| Snooze: 5 min, 1 h, 1 day, custom, with a snooze count limit | P0 | M | 4 | [specs/reminders.md](specs/reminders.md) |
| Recurrence | P0 | L | 4 | [specs/reminders.md](specs/reminders.md) |
| DST-safe scheduling, recompute on resume and clock change | P0 | M | 4 | [specs/reminders.md](specs/reminders.md) |
| Missed-reminder catch-up after sleep or shutdown | P0 | M | 4 | [specs/reminders.md](specs/reminders.md) |
| Notification history | P1 | S | 4 | [05-data-model-and-storage.md](05-data-model-and-storage.md) |

## Location, prayer and Azan

| Feature | P | Effort | Phase | Spec |
|---|---|---|---|---|
| IP geolocation with a manual override and a confirmation step | P0 | M | 5 | [11-prayer-and-azan.md](11-prayer-and-azan.md) |
| JAKIM e-Solat as the default source in Malaysia | P0 | L | 5 | [11-prayer-and-azan.md](11-prayer-and-azan.md) |
| Zone table and zone selection UI showing code and both names | P0 | M | 5 | [11-prayer-and-azan.md](11-prayer-and-azan.md) |
| Calculated times (`adhan`) with a Malaysia preset | P0 | M | 5 | [11-prayer-and-azan.md](11-prayer-and-azan.md) |
| Fallback chain with a visible source label | P0 | M | 5 | [11-prayer-and-azan.md](11-prayer-and-azan.md) |
| Per-prayer audio chosen or uploaded by the user | P0 | M | 5 | [11-prayer-and-azan.md](11-prayer-and-azan.md) |
| Bundled default Azan audio (licence permitting) | P1 | S | 9 | [11-prayer-and-azan.md](11-prayer-and-azan.md) |
| Pre-prayer reminders | P0 | S | 5 | [11-prayer-and-azan.md](11-prayer-and-azan.md) |
| Imsak and Dhuha, hidden until enabled | P0 | S | 5 | [11-prayer-and-azan.md](11-prayer-and-azan.md) |
| Notify on a published schedule correction | P1 | M | 5 | [11-prayer-and-azan.md](11-prayer-and-azan.md) |

## Weather

| Feature | P | Effort | Phase | Spec |
|---|---|---|---|---|
| Open-Meteo client with a TTL cache | P0 | M | 6 | [specs/weather.md](specs/weather.md) |
| Condition-change detection and per-condition templates | P0 | M | 6 | [specs/weather.md](specs/weather.md) |
| Weather code to Fluent key mapping, no API text shown | P0 | S | 6 | [specs/weather.md](specs/weather.md) |
| Offline stale marker | P0 | S | 6 | [specs/weather.md](specs/weather.md) |

## Time announcements

| Feature | P | Effort | Phase | Spec |
|---|---|---|---|---|
| Spoken announcements through the Windows `SpeechSynthesizer` | P0 | M | 7 | [specs/announcements.md](specs/announcements.md) |
| Schedules: every hour, start of day, custom | P0 | S | 7 | [specs/announcements.md](specs/announcements.md) |
| Voice selection by language tag, with a fallback hint | P0 | M | 7 | [specs/announcements.md](specs/announcements.md) |
| Chime fallback when no voice is installed | P0 | S | 7 | [specs/announcements.md](specs/announcements.md) |
| Azan takes priority over an announcement | P0 | S | 7 | [specs/announcements.md](specs/announcements.md) |

## Reliability and delivery

| Feature | P | Effort | Phase | Spec |
|---|---|---|---|---|
| Supervisor process with restart and exponential backoff | P0 | M | 8 | [specs/supervisor.md](specs/supervisor.md) |
| Crash-loop protection: 5 crashes in 10 min stops restarting | P0 | S | 8 | [specs/supervisor.md](specs/supervisor.md) |
| Panic hook, crash reports, daily rotating logs, log retention | P0 | M | 0, 8 | [specs/supervisor.md](specs/supervisor.md) |
| Optional minidump capture | P2 | M | 8 | [specs/supervisor.md](specs/supervisor.md) |
| Portable zip, SHA-256 checksums, signing | P0 | M | 9 | [09-ci-cd-and-releases.md](09-ci-cd-and-releases.md) |
| **In-app updater** reading the published release assets | P0 | L | 9 | [specs/updater.md](specs/updater.md) |
| Bilingual READMEs, user guide, screenshots and GIFs | P0 | M | 9 | [screenshots/README.md](screenshots/README.md) |

## Explicitly not features

Native Action Center integration · cross-platform builds · cloud sync, accounts
or telemetry · Jawi script · languages beyond English and Malay. See
[01-vision-and-scope.md](01-vision-and-scope.md).
