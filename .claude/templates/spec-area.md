# Template — area spec (`docs/specs/<area>.md`)

Copy this into `docs/specs/<area>.md`, fill it in, and commit it **before** the
code. The spec is the requirement: if the implementation later forces a design
change, change this file in the same commit.

The areas are `toast`, `reminders`, `weather`, `announcements`,
`tray-and-settings`, `supervisor` and `updater`. **Prayer has no file here** —
its spec is `docs/11-prayer-and-azan.md`, because it is large enough to warrant
a numbered document of its own.

Keep the numbered-section shape the existing specs use so they stay comparable,
and add a dated addendum section for later work rather than rewriting history.

---

```markdown
# Area spec — <Display Name>

**Status:** draft · **Last updated:** <YYYY-MM-DD>

## 1. Purpose

What this area is for, in two or three sentences, and what it is explicitly not
for — point at the non-goals in `docs/01-vision-and-scope.md` rather than
restating them.

## 2. User stories

- As a <persona from docs/01-vision-and-scope.md>, I want <…> so that <…>.

## 3. Screens and panes

Which settings pane(s) this area owns, what each shows, and what the user can
do there. For a toast-facing area, what the toast looks like and which parts of
the `ToastSpec` it sets. Name the `ot-ui` widgets reused rather than describing
new controls — and if a new shared widget is needed, say so here, because that
is a decision.

## 4. <Domain sections>

One per piece of the area — the list and the editor, the trigger options, the
condition detection, the voice selection. What each does, and what it refuses
to do.

## 5. Strings

The Fluent key prefix this area uses (one of `tray-`, `toast-`, `settings-`,
`prayer-`, `weather-`, `reminder-`, `announce-`, `error-`, `format-`), and the
keys it adds. Every one exists in both `locales/en-US/omni_toaster.ftl` and
`locales/ms-MY/omni_toaster.ftl`.

## 6. Data

Which tables in `docs/05-data-model-and-storage.md` this area reads and writes,
which `settings.toml` keys it owns, which migration introduces them, and what
it stores under `OmniToasterData/assets/` or `cache/`. Relative paths only.
Note any built-in key the database records, since renaming one needs a
migration.

## 7. Scheduling

What this area asks the scheduler for, and what the scheduler does with it:
which fire times it contributes, what invalidates them (DST, resume, a clock
change, midnight, a location change, a successful fetch), its priority against
Azan and against quiet hours. Omit this section only if the area never touches
time.

## 8. Failure and offline behaviour

What the user sees when the network is down, the response is malformed, the
cache is empty, the file is corrupt or the voice is missing. Each fallback is
named in the UI, not silent. What is logged.

## 9. Localization notes

Anything beyond "both files": dual names where the area shows them
(`Subuh (Fajr)`), the longest Malay string its layout must survive, whether any
of its text is spoken and therefore goes through a Fluent template, and which
values go through the `ot-i18n` formatter rather than being written out.

## 10. Implementation notes

The `ot-*` crates this area lives in and reuses (`ot-core`, `ot-i18n`,
`ot-store`, `ot-platform`, `ot-ui`, `ot-toast`, `ot-sched`, `ot-prayer`,
`ot-weather`, `ot-media`), the files it owns, which work goes on the scheduler
thread and which on a worker thread, how it reaches other modules through the
event bus, and the dependencies it adds — each with its ADR number from
`docs/08-decisions.md`.

## 11. Open questions

Numbered, so they can be answered in place later.

## What is implemented (<YYYY-MM-DD>)

The honest state: what works, what is stubbed, what was verified in the running
app (which language, which theme, which DPI) and what could not be verified on
this machine.
```
