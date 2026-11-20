<div align="center">

<img src="assets/icon/omnitoaster-128.png" alt="OmniToaster" width="128" height="128">

# OmniToaster

</div>

**English** · [Bahasa Melayu](README.ms.md)

OmniToaster is a portable Windows desktop application that delivers
customizable, Windows-style toast notifications for reminders, prayer times
(Azan), weather changes and time announcements. It runs in the system tray,
starts with Windows, recovers from crashes, and keeps every byte of its data in
an `OmniToasterData/` folder next to the executable.

**Status:** Phases 0–2 are done: the app starts in English or Malay,
keeps its data in a portable folder, refuses to run twice, lives in the
**system tray** and paints its own **toasts** — three shapes, nine screen
positions or a point of your own, on the monitor you choose, with a picture
library, and a live preview in Settings ▸ Appearance. The language and theme
switch **while it runs**. Double-click the tray icon to open Settings; closing
the window leaves the app running behind the icon, and Quit is in the tray
menu, which also pauses all notifications for half an hour, an hour or until
you resume them. **Reminders** are made in Settings ▸ Reminders and fire as
toasts on time, with snooze, missed-reminder catch-up and a history; the Azan
and weather are the next phases.
`--data-dir`, `--self-test`, `--version`, `--help`, `--lang` and `--worker`
all work.

> On Windows 11 a new tray icon starts in the **hidden-icons overflow**,
> behind the chevron at the left of the notification area. Drag it onto the
> taskbar to keep it in sight.

See [PROPOSAL.md](PROPOSAL.md) §13 for the phase plan and
[docs/CHECKLIST.md](docs/CHECKLIST.md) for progress.

---

## Notifications

Every notification is a *category* with its own template — title, description,
image, accent colour, sound and style overrides — in both languages:

| Type | What it does |
|---|---|
| **Azan prayer** | The five daily prayers, plus optional Syuruk and Jumu'ah, with per-prayer audio and an optional pre-prayer reminder |
| **Weather change** | Fires on a condition change (clear, cloudy, rain, storm, snow, fog) or a temperature threshold |
| **Event** | A one-off or recurring calendar-style reminder |
| **Task** | A to-do with an offset-before trigger |
| **Anniversary** | Yearly recurrence |
| **Fatigue** | Eye breaks, posture, drinking water and a work-hour limit, each switched on in Settings ▸ Reminders with its own interval, counted from when you start using the PC |
| **Timer** | A countdown from now |
| **Alarm** | A persistent toast that stays until you act on it, for up to ten minutes |
| **Payment** and **Appointment** | Offset-before reminders for dated obligations |
| **Hourly time announcement** | The time spoken aloud in the active language — hourly, every 30 or 15 minutes, start or end of day, or at custom times |

Reminders are made and edited in Settings ▸ Reminders: a title and details
in your own words, one of the types above, a date and a time, any number of
"remind me before" times, and a repeat. The editor says what is wrong under
the field it is about, and deleting a reminder or clearing its history asks
first. Each reminder shows its recent history: shown, snoozed, dismissed,
opened, missed. Open on a reminder's toast lands on it there.

Scheduling supports exact times, offsets before (15 minutes to a week, or
custom), and recurrence (once, daily, weekly, monthly, yearly). Snooze puts a
reminder off for 5 minutes, 15 minutes, an hour, a day or a length of your own,
chosen in Settings ▸ Reminders, up to a limit per fire (10 by default); after
that the toast offers only Dismiss and says so. Reminders missed
while the PC was off or asleep are not fired all at once at next start: one
toast says some were missed, and Settings ▸ Reminders lists them with Snooze
and Dismiss. Ones older than the catch-up window (24 hours by default) are
dropped and recorded.

## Bilingual

Every user-visible string — labels, toasts, validation messages, prayer names,
spoken announcements — comes from a Fluent key in both English (`en-US`) and
Bahasa Melayu (`ms-MY`). The language follows the Windows display language by
default and can be switched at runtime from Settings; the tray menu and every
open window re-render within a second, with no restart and no data migration.
Malay spelling follows Dewan Bahasa dan Pustaka. Prayer names are shown in both
sets, for example "Subuh (Fajr)", so users of either can find the prayer.

## Appearance

Three themes — **Dark (the default)**, Light, and System, which follows the
Windows light/dark setting. A global accent colour, with an optional per-category
override.

Toasts are painted by the app, not handed to the Action Center, so almost
everything about them is a setting:

- **Shape:** rectangle, rounded rectangle or pill, with a corner radius of 0–40 px
- **Size:** width 280–600 px (default 360), height auto or fixed
- **Colour:** solid or gradient background, border width and colour, and a shadow
  for the Rectangle shape
- **Transparency:** 70–100 % opacity; the whole toast fades, text included
- **Glass:** none, Blur, Acrylic or Mica-like — the desktop behind a Rectangle
  toast, blurred by Windows, under 45 %, 60 % or 88 % of the toast's colour.
  Rectangle only, because Windows blurs past rounded corners; Windows 10 1809
  or later, and drawn solid where Windows refuses. A glass toast has no shadow
- **Animations:** eight-plus in presets — slide from right, slide from bottom,
  Genie from bottom, pop out, drawer, appear, dissolve, glow, alpha-to-solid —
  each with a mirrored out preset plus "shrink to tray", a 150–1000 ms duration
  and linear, ease-out or spring easing, chosen in Settings ▸ Animation, where
  Play shows a test toast with them. Reduce Motion — or Windows' own "Show
  animations" turned off — plays every one as a fade, and an animation that
  falls below 30 fps finishes as one
- **Duration:** 3–60 seconds (default 8), or persistent for urgent types, with
  hover pausing the timer
- **Header:** icon, title, app name and timestamp, each toggleable
- **Body:** text with bold and italic, plus an image (PNG, JPG or WebP, up to 5 MB)
- **Actions:** one to three buttons along the bottom — Snooze, Dismiss, Open.
  Dismiss closes the toast with its out-animation, Open brings Settings
  forward, and Snooze puts a reminder off; on a toast with nothing to snooze,
  such as the test toast, Snooze is greyed
- **Picture Library:** upload a picture once — a mosque, a meeting, an alarm
  clock — name it, and use it in any notification as its icon or its image.
  Removing a picture still in use asks first. Nineteen pictures ship with the
  app — a mosque for each prayer, nine kinds of weather, an alarm clock, a
  meeting and a training session — ready to choose from the same lists
- **Position and stacking:** nine presets or a point of your own, on the primary
  monitor or one you choose, 1–5 visible at once
- **Accent bar:** on the side of the toast that faces the screen's edge, with
  that side square

## Prayer times

Inside Malaysia the default source is **JAKIM e-Solat**, by zone code (about 60
zones), so times match the mosques, schools and government offices users follow.
The app detects the location, proposes the nearest zone and asks the user to
confirm it. Times are fetched for the current and next month, cached, and
refreshed weekly.

Outside Malaysia — or offline, or when e-Solat is unavailable — times are
**calculated** with the `adhan` crate. Method presets are Muslim World League,
ISNA, Umm al-Qura and "Malaysia (JAKIM angles, approximate)" at Fajr 20.0° and
Isha 18.0°. Asar follows Shafi'i by default. Calculated times inside Malaysia
are labelled approximate. The source in use, and the time of the last fetch, are
always on screen.

## Portable mode

The executable never writes outside its own folder — no registry beyond the
`Run` autostart value, no `%APPDATA%`, no `%TEMP%`:

```
OmniToaster/
├── OmniToaster.exe                 # Supervisor (default) and worker (--worker)
├── README.md
└── OmniToasterData/
    ├── config/settings.toml        # Language, theme, toast style, location, prayer, startup
    ├── data/omnitoaster.db         # SQLite: reminders, categories, caches, history
    ├── locales/                    # Optional user overrides of the embedded .ftl files
    ├── assets/                     # User images, Azan audio, chimes, icons
    ├── logs/                       # Daily rotating log, plus crash/
    └── cache/                      # Weather and geolocation responses, TTL-based
```

At startup the app checks that `OmniToasterData/` is writable; if it is not (the
exe sits in `Program Files`, say) it shows a bilingual error and exits without
touching existing data. User media is copied in under content-hashed names, and
paths are stored relative, so the folder can be moved or carried on a stick.

## Requirements

- Windows 10 version 1809 or later, or Windows 11, 64-bit
- A stable Rust toolchain with the MSVC target, to build from source
- No installer, no administrator rights, no account

## Build and run

```bash
cargo build --release -p omnitoaster
```

Produces `target/release/OmniToaster.exe`.

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

Runs against a throwaway data folder. **Always pass `--data-dir` with a scratch
folder while developing**, so the real `OmniToasterData/` beside the exe is
never touched.

```bash
cargo run -p omnitoaster -- --self-test
```

The smoke check CI runs.

A running `OmniToaster.exe` keeps its own file locked, so a release build fails
with "Access is denied" while an instance is open — `Stop-Process -Name
OmniToaster -Force` first.

## Restrictions and non-goals

- **Windows only** for this release. Platform code is isolated in one crate so
  other systems can be added later.
- **Not the Action Center.** OmniToaster paints its own windows; it does not
  replace or hook into the native Windows notification system.
- **No cloud, no accounts, no telemetry.** Nothing leaves the machine except
  requests to Open-Meteo, the IP geolocation service and JAKIM e-Solat — all
  HTTPS, and each can be turned off. Crash reports stay local and are never
  uploaded.
- **Glass effects need Windows 10 1809+** and fall back to a solid background.
- **Expensive animations degrade.** Each effect has a frame-time budget and
  falls back to fade below 30 fps.
- **No Jawi script** and no languages beyond English and Malay in this release.
- **Text-to-speech uses installed Windows voices.** With no voice for the active
  language the app plays the chime and shows a setup hint; it downloads nothing.

## The icon

<img src="assets/icon/omnitoaster-64.png" alt="" width="64" height="64" align="left" hspace="12">

`assets/icon/omnitoaster-512.png` is the master. Every other size, and
`omnitoaster.ico` for the executable, is generated from it by
`scripts/make-icons.ps1` — never edited by hand, so the sizes cannot drift
apart. The `.ico` carries 16 through 256 and is embedded into
`OmniToaster.exe` at build time along with the version metadata Explorer
shows in Properties ▸ Details.

<br clear="left">

Sizes at or below 32 px are sharpened after downscaling. Without it the card
stack and the bell merge into a coloured blob at 16 px, which is the size the
tray and the title bar use. Even sharpened, 16 px is the weakest size: the
artwork has three stacked cards, a bell and its rays, which is a lot of detail
for 256 pixels.

## Screenshots

A toast, as the app paints it — here at the default Bottom Right, so the accent
bar is on the side facing the screen's edge:

![A toast in the default rounded rectangle](docs/screenshots/toast-dark.png)

The three shapes, each with the side against the screen edge square:

![Rectangle, rounded rectangle and pill toasts](docs/screenshots/toast-shapes.png)

The same toast over a light window is
[toast-light.png](docs/screenshots/toast-light.png): a toast keeps its own
colours whatever the window's theme.

A Rectangle toast with Acrylic glass: the desktop behind it — here a striped
window — blurred by Windows under 60 % of the toast's colour:

![A Rectangle toast with Acrylic glass over a striped window](docs/screenshots/toast-glass.png)

Slide from right, in and out, and Genie from bottom, pouring out of the screen's
bottom edge in bands — both recorded from a release build:

![A toast sliding in from the right, holding, and sliding out](docs/screenshots/anim-slide.gif)

![A toast pouring up out of the bottom edge and back down into it](docs/screenshots/anim-genie.gif)

Settings ▸ General, the first screen the app has. Clock writes every time in
the app 12-hour or 24-hour; Automatic is 24-hour in Bahasa Melayu and follows
Windows in English, as English dates follow Windows' day or month first:

![Settings ▸ General in the Dark theme](docs/screenshots/settings-general.png)

The same pane in Bahasa Melayu is
[settings-general-ms.png](docs/screenshots/settings-general-ms.png).

Settings ▸ Appearance, where the preview is drawn by the same code as a real toast:

![Settings ▸ Appearance with the live preview](docs/screenshots/settings-appearance.png)

In Bahasa Melayu: [settings-appearance-ms.png](docs/screenshots/settings-appearance-ms.png).

Settings ▸ Animation, where the in and out animations, their duration and
easing are chosen, Windows' own animation setting is stated under Reduce
Motion, and Play fires a test toast with them:

![Settings ▸ Animation](docs/screenshots/settings-animation.png)

In Bahasa Melayu: [settings-animation-ms.png](docs/screenshots/settings-animation-ms.png).

Settings ▸ Picture Library, where pictures are uploaded once and named, with
the ones that ship with the app listed below them:

![Settings ▸ Picture Library with two uploaded pictures and the built-in ones](docs/screenshots/settings-pictures.png)

The same screen in Bahasa Melayu:
[settings-pictures-ms.png](docs/screenshots/settings-pictures-ms.png).

Settings ▸ Reminders: the missed reminders, your reminders with the one being
edited, its history, and Snooze's length and limit and the catch-up window
below:

![Settings ▸ Reminders with a weekly reminder open in the editor](docs/screenshots/settings-reminders.png)

In Bahasa Melayu: [settings-reminders-ms.png](docs/screenshots/settings-reminders-ms.png). The rest
are **not captured yet** — the screens do not exist. The places are reserved
and the capture procedure is written down in
[docs/screenshots/README.md](docs/screenshots/README.md); each image below is
taken as its phase lands:

| Image | Shows | Lands in |
|---|---|---|
| `docs/screenshots/tray-menu.png` | The tray icon and its menu | Phase 1 |
| `docs/screenshots/settings-prayer.png` | Settings → Prayer, zone confirmation and source label | Phase 5 |
| `docs/screenshots/flow-reminder.gif` | Creating a reminder and seeing it fire | Phase 4 |

Each screenshot is taken in both English and Malay before Phase 9 ships.

## Licence

MIT. See [LICENSE](LICENSE).

## Attribution

Official prayer times come from **JAKIM e-Solat** (Jabatan Kemajuan Islam
Malaysia, `e-solat.gov.my`). OmniToaster is not affiliated with or endorsed by
JAKIM. Where times are shown from that source the app labels them "Source:
JAKIM e-Solat" with the time of the last fetch; calculated times are labelled as
calculated. Whether the JAKIM dataset may be bundled with a release is settled
by the terms check in Phase 9 — until then the data is fetched or downloaded by
the user.

Weather data comes from [Open-Meteo](https://open-meteo.com/) under its own
terms. Prayer calculation uses the [`adhan`](https://crates.io/crates/adhan)
crate.
