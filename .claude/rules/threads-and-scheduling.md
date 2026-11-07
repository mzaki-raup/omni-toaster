# Threads, scheduling and subprocess rules

There is **no tokio** in this project and no async runtime. The concurrency
model is a small set of named threads talking over crossbeam channels. Don't
add an async runtime, and don't add `async fn` to a library crate to make a
single HTTP call look tidier.

## The scheduler thread is the single source of time

- `ot-sched` runs one dedicated thread that owns a priority queue keyed on each
  reminder's **next fire time**, stored (not recomputed from scratch each
  frame).
- It **recalculates** after a DST transition, after system resume, after a
  clock change, at midnight, after a location or zone change, and after a
  successful JAKIM fetch. Any of those invalidates the queue.
- It **never relies on a long sleep.** A thread parked for eight hours wakes up
  wrong on a laptop that slept, so wait on a condition variable with a bounded
  timeout and re-check the wall clock every time it wakes.
- Missed items are handled explicitly, not dropped: the catch-up window is one
  hour for Azan and twenty-four hours for everything else, and missed reminders
  are listed at next start with dismiss and snooze.
- No other module computes a fire time. The UI, `ot-prayer` and `ot-weather`
  publish facts; the scheduler decides when.

## Channels between modules

- Modules communicate through **crossbeam channels**, not direct calls. The UI,
  the scheduler and the services do not call each other.
- Channels into the UI are **bounded**, and the UI drains them at frame start.
  An unbounded channel into the UI is a memory leak waiting for a slow frame.
- A full bounded channel means the consumer is behind — decide and state what is
  dropped or coalesced (weather updates coalesce; a fired reminder must not be
  dropped).

## HTTP and other blocking work

- HTTP is blocking `ureq` on a **worker thread**, never on the UI thread and
  never on the scheduler thread. HTTPS only, ten-second timeout, User-Agent
  `OmniToaster/<version>`.
- Decoding an image or audio file, hashing an import and touching SQLite are all
  blocking work: off the UI thread, result back over a channel.
- Never `block_on` anything on the UI thread, and never sleep inside `ui()`.
- Retry with backoff (1, then 5, then 15 minutes) and then wait for the next
  scheduled check. **No polling loop.**

## Processes

- `OmniToaster.exe` runs as the **supervisor** by default and spawns itself with
  `--worker`. The supervisor watches the worker's exit status, records crash
  information, and restarts with exponential backoff; after five crashes in ten
  minutes it stops restarting and shows a bilingual notification naming the log
  path.
- The supervisor does nothing else. It owns no window, no tray icon and no
  scheduler.
- Every child process is spawned with `CREATE_NO_WINDOW` so no console flashes.
  Spawn through the helper in `ot-platform`, never a bare
  `std::process::Command::new` for an internal command.
- Never set the `PATH` environment variable from inside the process.
- Subprocess and worker output goes to the daily log; see the "Tray, Startup and
  Supervisor" section of `docs/GOTCHAS.md`.

## Cancellation and errors

- The prayer fetch, the weather fetch and the update check must all be
  cancellable, and cancel must actually stop the work rather than only hide its
  result — a settings window closed mid-fetch must not leave a thread writing to
  the cache.
- Surface a failure where the user started the action: the pane's error state
  **plus** the visible status line, with the fallback that was used named
  ("Calculated (fallback)", "Offline data"). A silent fallback is a defect.
- Each step of the prayer fallback chain is logged. Speech voice fallbacks are
  logged and shown in the settings status line — §6.5 of `PROPOSAL.md` is
  explicit that they are never silent.
