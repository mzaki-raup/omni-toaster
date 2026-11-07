# Spec — Supervisor, crash recovery and logging

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../../PROPOSAL.md) §5, §8.3 · **ADR:** [ADR-005](../08-decisions.md)
**Owner crates:** `ot-platform`, `ot-core` · **Phases:** 0 (logging and panic hook), 8 (supervisor and hardening)

## 1. Purpose

"Auto start when apps crash with debug log saved for future debug" was an
explicit requirement. A panic hook alone cannot deliver it: a GPU driver fault,
a stack overflow or an aborting allocator kills the process without unwinding,
and those are exactly the failures a GPU-backed UI hits.

## 2. User stories

- As **anyone**, I want the app to come back by itself after it dies, without
  my noticing it was gone.
- As **anyone**, I want it to stop trying when it clearly cannot run, and tell
  me where the log is — not thrash forever.
- As **the owner debugging a report**, I want a log that says what happened.

## 3. The two roles

One executable, two roles, chosen by the command line:

| Invocation | Role |
|---|---|
| `OmniToaster.exe` | **Supervisor.** Spawns the worker, watches it, restarts it. No UI, no tray icon, no scheduler. |
| `OmniToaster.exe --worker` | **Worker.** The whole application: tray, settings, toasts, scheduler, services. |
| `OmniToaster.exe --self-test` | Headless smoke check for CI. Runs neither role's loop. |
| `OmniToaster.exe --data-dir <path>` | Modifies either role: use this folder instead of `OmniToasterData/`. Passed through to the worker. |

The supervisor passes its own `--data-dir` to the worker, so a scratch run
stays a scratch run across a restart. Each role takes its own single-instance
mutex name ([specs/tray-and-settings.md](tray-and-settings.md) §5).

## 4. Restart policy

- The supervisor waits on the worker's exit. A clean exit (the user chose Quit)
  ends the supervisor too.
- A non-zero or signalled exit is a crash: record it, then restart with
  **exponential backoff** — 1 s, 2 s, 4 s, 8 s, 16 s.
- **After 5 crashes within 10 minutes, stop restarting.** Show a bilingual
  notification naming the log path, and exit. Thrashing is worse than being
  down, and it hides the cause.
- The crash counter is a sliding 10-minute window, so an app that crashes once
  a day runs forever and an app that cannot start gives up quickly.

## 5. Crash capture

- `std::panic::set_hook` writes a panic report with the backtrace before the
  process dies — this is Phase 0 work, not Phase 8.
- The supervisor records the worker's exit status, the time, and the tail of the
  log, which is what catches the failures the hook cannot.
- Optional minidump capture (`minidumper`) is a Phase 8 stretch, off by default.

A crash report contains: the app version, the OS build, the last 200 log lines,
and the panic message or exit status. **No personal data** — no reminder
titles, no coordinates, no file names from imports
([06-security-and-privacy.md](../06-security-and-privacy.md)). Reports stay
local and are never uploaded.

## 6. Logging

- `tracing` with `tracing-appender`, daily rotation, into
  `OmniToasterData/logs/app-YYYY-MM-DD.log`.
- 14-day retention for logs, the last 20 crash reports, both configurable.
- Pruning happens at startup, so a long-running instance does not accumulate
  forever and a restart always tidies up.
- The log records what the user cannot see: which prayer source answered, which
  voice was chosen, every fallback taken, every rejected import and why.
- Any subprocess output goes to the log. There are no secrets to redact
  ([ADR-011](../08-decisions.md)), but a file path from a user's import is
  still their data and is logged as a relative path, not an absolute one.

## 7. Process hygiene

Every child process is spawned with `CREATE_NO_WINDOW`, so a restart never
flashes a console window. Commands are never resolved by setting the `PATH`
environment variable from inside the process
([.claude/rules/threads-and-scheduling.md](../../.claude/rules/threads-and-scheduling.md)).

## 8. Hardening (Phase 8)

Phase 8 is also the security review pass: the import paths
([06-security-and-privacy.md](../06-security-and-privacy.md) §"Input
validation"), the external data paths (JAKIM, Open-Meteo, the IP lookup, the
update check), and the decoder bounds. Running the media decoders in the worker
process — now that a worker crash is survivable — is the mitigation this phase
is allowed to choose.

## 9. Fluent keys

Prefix `error-` for the crash-loop notification and the log-path message; both
must exist in Malay, because a user whose app has given up is exactly the user
who should not be shown English they cannot read.

## 10. Verification

| Check | Asserts |
|---|---|
| Panic recovery | a forced panic in the worker is logged and the worker restarts |
| Backoff | the restart delays follow 1, 2, 4, 8, 16 s |
| Crash loop | 5 crashes in 10 minutes stops the supervisor, with the notification shown and the log path correct |
| Sliding window | 5 crashes spread over an hour do **not** stop it |
| Clean exit | Quit ends both processes, with no orphan |
| `--data-dir` | the flag reaches the worker across a restart |
| Retention | logs older than the window and crash reports beyond the last 20 are pruned at startup |

Then in the running app: force a panic, watch it come back, force five, watch
it stop and read the notification (Phase 8's exit criteria).

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

## 11. Open questions

1. Whether the supervisor should itself be restartable by Windows if it dies
   (it holds no state, so probably not worth it).
2. Whether the crash-loop notification should use the app's own toast
   renderer — which may be the very thing that is broken — or a plain Win32
   message box.

## What is implemented (2026-11-08)

Nothing. Phase 0 has not started; this file is the requirement.
