# Template — feature request

Hand this to `/feat` to build it now, or to `/task-new feat --version v0.0.1`
to schedule it; either way it becomes a `FEAT-dddd` item in
[docs/TASKS.md](../../docs/TASKS.md). Unclear requirements get questions back
before code, not after, so the more of this that is filled in, the less there is
to ask. The **Behaviour** and **Done when** sections below become the item's
checklist, so write them as things someone can check.

```markdown
## What

One or two sentences: the capability, in terms of what the user can then do.

## Where

Area: toast · reminders · prayer and Azan · weather · announcements ·
tray and settings · supervisor · updater (or "new area")
Which screen: a settings pane (which) · the toast itself · the tray menu · none

## Why

The task it unblocks. Which phase of `docs/07-roadmap.md` it belongs to, and
whether it is part of that phase's exit criteria or extra to them.

## Behaviour

- Normal case: …
- Empty case: what the screen says with nothing to show.
- Error case: what the user sees when the fetch fails, the file is corrupt or
  the voice is missing — and which fallback is named in the UI.
- Offline case: does this work with no network, and is stale data marked?
- Slow case: is this over ~200 ms? Then it is not on the UI thread — which
  thread, and what does cancel stop?

## Time

Does this put anything on a schedule? Which fire times it contributes, what
invalidates them (DST, system resume, a clock change, midnight, a location
change), and its priority against Azan and against quiet hours. The scheduler
owns time; nothing else computes a fire time.

## Data

- New `settings.toml` keys? Which, and their defaults.
- New table or column? Then which numbered migration adds it, and does it have
  to work on an existing database as well as a fresh one.
- New Fluent keys? List them — they go in **both**
  `locales/en-US/omni_toaster.ftl` and `locales/ms-MY/omni_toaster.ftl` in the
  same commit. Any dual prayer name, any spoken string, any date or number that
  must go through the `ot-i18n` formatter.
- Imported media? Then the size limit, the accepted formats, and the
  confirmation that it is copied in content-hashed with a relative path stored.

## Guards

Anything that needs confirming before it happens: deleting a reminder or a
category, clearing the notification history, overwriting or removing an
imported asset, changing the autostart registry value, resetting settings to
defaults, or anything the updater replaces. Each needs a guard, and a delete
that cannot be undone needs to say so.

## Out of scope

What this explicitly does not include, so it doesn't grow.

## Done when

The checks that would convince you: a test asserting …, and this seen working
in the running app on a scratch `--data-dir` — in which language, which theme
and at which DPI.
```

## What `/feat` adds on its own

The ledger item and its four closing boxes, a spec update in the same commit,
reuse of the `ot-*` crates rather than new ones (`ot-ui` widgets, the `ot-i18n`
formatter, the `ot-store` migrations, the `ot-platform` interface for anything
Windows-only), Fluent strings in both files with the right casing, the gate, the
running-app check, and the `docs/CHANGELOG.md` / `docs/CHECKLIST.md` / README
lines.
