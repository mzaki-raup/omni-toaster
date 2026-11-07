# 01 — Vision and scope

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../PROPOSAL.md) §1, §2

Omni Toaster is a portable Windows desktop application that delivers
customizable, Windows-style toast notifications for reminders, prayer times
(Azan), weather changes and time announcements. It lives in the system tray,
starts with Windows, recovers from crashes, and keeps everything it owns in an
`OmniToasterData/` folder beside the executable.

It is fully bilingual in English (`en-US`) and Bahasa Melayu (`ms-MY`), and the
language switches at runtime without a restart.

## Personas

| Persona | Who they are | What they need |
|---|---|---|
| **Azlan — the praying professional** | Works at a PC all day in Malaysia, follows JAKIM times | Azan on time for his zone, matching the mosque, with his own audio per prayer |
| **Mei — the schedule keeper** | Runs her day from her desktop, dislikes phone notifications | Event, task, anniversary and timer reminders that look native and are impossible to miss |
| **Hafiz — the overseas Malaysian** | Lives abroad, wants Malaysian practice | Calculated times with a Malaysia preset, or a manually chosen JAKIM zone with a clear warning |
| **Siti — the Malay-first user** | Prefers Bahasa Melayu throughout | Every label, date, prayer name and spoken announcement in DBP Malay |
| **The tinkerer** | Keeps the app on a USB stick or a synced folder | One folder, no installer, no registry litter, nothing left behind |

## Goals

- Toast notifications that look and behave like Windows toasts, with broad
  customization of shape, colour, transparency, glass, animation and duration.
- Full English and Malay support for every user-visible string, date, prayer
  name and spoken announcement.
- Reliable scheduling for many reminder types, including Azan and weather.
- Prayer times from JAKIM e-Solat for Malaysian locations, with a calculated
  fallback.
- Portable distribution: one folder, no installer required.
- Self-recovery: auto-start at login, auto-restart after a crash, persistent
  local debug logs.
- Dark theme by default, with System, Dark and Light options.
- An in-app updater that reads the published release assets and updates the
  portable folder in place.

## Non-goals (initial release)

- Replacing or hooking into the native Windows Action Center.
- macOS and Linux support. Platform code is isolated in `ot-platform` so these
  can be added later. Cross-platform is wanted eventually (PROPOSAL §13 Q1) but
  is not in v1.0.
- Cloud sync, user accounts, telemetry.
- Jawi (Arabic-script Malay).
- Languages other than English and Malay.

## Constraints that shape every decision

- **Windows 10 1809+ and Windows 11.** No API newer than 1809 without a
  fallback path.
- **Portable.** Nothing is written outside the exe's own folder — not the
  registry (except the single HKCU `Run` value the user opts into), not
  `%APPDATA%`, not `%TEMP%`.
- **Local only.** The only outbound traffic is JAKIM e-Solat, Open-Meteo, the
  IP geolocation lookup and the update check, each disablable.
- **MIT licensed and public.** No bundled media whose licence does not allow
  redistribution ([11-prayer-and-azan.md](11-prayer-and-azan.md) §7).

## Where the rest of the specs are

[docs-and-release.md](../.claude/rules/docs-and-release.md) holds the full map.
