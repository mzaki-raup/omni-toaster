# Rules index

OmniToaster is a portable Windows desktop app in Rust: a Cargo workspace whose
binary crate is `crates/omnitoaster` (package `omnitoaster`, exe
`OmniToaster.exe`) and whose library crates each own one concern —
`ot-core` (settings, paths, event bus, errors, logging, panic hook), `ot-i18n`
(Fluent bundles, locale detection, the one date and number formatter),
`ot-store` (SQLite, migrations, asset store), `ot-platform` (**all** Windows-only
code: tray, HKCU `Run` autostart, theme detection, single-instance mutex, window
vibrancy, supervisor), `ot-ui` (shared egui widgets, theme, settings window),
`ot-toast` (`ToastSpec`, renderer, animations), `ot-sched` (scheduler thread,
recurrence, snooze), `ot-prayer` (`PrayerSource`, `JakimSource`,
`CalculatedSource`, the zone table), `ot-weather` (Open-Meteo) and `ot-media`
(audio through `rodio`, speech through the Windows `SpeechSynthesizer`).

It ships as one exe that keeps all of its data in `OmniToasterData/` beside
itself. [`PROPOSAL.md`](../../PROPOSAL.md) is the product authority;
[`docs/03-architecture.md`](../../docs/03-architecture.md) has the crate layout.

The root [CLAUDE.md](../../CLAUDE.md) holds the behavioural guidelines that
apply to any project. **These files hold OmniToaster's own rules** — read the
one for the area you are about to change.

| File | Read it before you… |
|---|---|
| [rust.md](rust.md) | write or change any Rust: the shape of a change, language rules, reuse, performance, build hygiene on this machine |
| [ui.md](ui.md) | touch `ot-ui`, `ot-toast`, a settings pane, a toast style or an animation |
| [i18n.md](i18n.md) | add, change or remove any user-visible string, or touch either `.ftl` file, a date format or a spoken announcement |
| [threads-and-scheduling.md](threads-and-scheduling.md) | put work on a thread: the scheduler, a worker fetch, the event bus, the supervisor, any child process |
| [security.md](security.md) | import media, write a file, build SQL, touch the registry value, or handle a crash report or log |
| [testing.md](testing.md) | run the gate, write a test, or decide whether a branch is green |
| [build-and-run.md](build-and-run.md) | build, run or debug the app on this machine |
| [docs-and-release.md](docs-and-release.md) | update the ledger or a spec, or go near a version or a release |
| [git.md](git.md) | commit, branch, merge or push |

Always also read the matching section of
[docs/GOTCHAS.md](../../docs/GOTCHAS.md).

## The non-negotiables, in one place

- **Portable:** all app data under `OmniToasterData/` beside the exe. The one
  exception is the single HKCU `Run` value for "Start with Windows". Never
  `%APPDATA%`, never `%TEMP%`, never any other registry key. Never commit
  `OmniToasterData/`.
- **No secrets, and no vault either.** The app needs no credentials, so there
  is nothing to encrypt. Never put a secret in the repository; if an API key
  ever becomes necessary it goes in the Windows Credential Manager, never in
  plain text and never in a file under the data folder.
- **The UI thread never blocks, and the scheduler thread owns time.** The
  scheduler holds every reminder's next fire time and recalculates after DST
  changes, system resume and clock changes. HTTP is blocking `ureq` on a worker
  thread. Modules talk through crossbeam channels, drained bounded at frame
  start. No tokio, no `block_on`.
- **Every visible string is a Fluent key** present in **both**
  `locales/en-US/omni_toaster.ftl` and `locales/ms-MY/omni_toaster.ftl`, edited
  with the **Edit tool** and never a heredoc. Dates and numbers go through the
  one formatter in `ot-i18n`.
- **Specs are requirements.** A design change updates its document in the same
  commit.
