# Build and run

Omni Toaster is a Cargo workspace. The binary crate is `crates/omnitoaster`
(package `omnitoaster`); the library crates are the `ot-*` crates listed in
[00-index.md](00-index.md). [docs/03-architecture.md](../../docs/03-architecture.md)
has the layout.

## Build and run

```bash
cargo build --release -p omnitoaster
```

Produces `target/release/OmniToaster.exe`.

```bash
cargo run -p omnitoaster -- --data-dir ./tmp-data
```

Run against a throwaway data folder. **Always pass `--data-dir` with a scratch
folder while developing**, so the real `OmniToasterData/` beside the exe is not
touched.

```bash
cargo run -p omnitoaster -- --self-test
```

What the CI smoke step runs: start, resolve paths, open the database, load both
locales, exit non-zero on any failure. No window, no tray icon.

```bash
cargo test -p ot-sched
```

One library crate at a time while iterating. The gate still runs the whole
workspace — see [testing.md](testing.md).

## The three modes of the exe

`OmniToaster.exe` with no flags is the **supervisor**: it spawns itself with
`--worker`, watches the worker and restarts it with backoff. `--worker` is the
real app — the tray icon, the settings window, the scheduler, the toasts.
`--self-test` is the CI smoke check. While developing, running with
`--data-dir` goes through the supervisor as a user's copy would; pass `--worker`
too when you want the app in one process so a debugger or a panic backtrace is
not swallowed by the restart.

## Running the app while developing

- A running `OmniToaster.exe` locks its own file, so a release build fails with
  "Access is denied" while an old instance is open. The supervisor means there
  may be **two** processes to stop:

```bash
Stop-Process -Name OmniToaster -Force
```

- The app takes a named single-instance mutex, so a second copy started by hand
  will refuse to run rather than giving you two tray icons. Stop the first one.
- Tests that need a network service (e-Solat, Open-Meteo, the geolocation
  lookup) are simply not run when it is unreachable. **Don't edit the test to
  skip itself** — see [testing.md](testing.md).

## Release and packaging scripts

```bash
pwsh -File scripts/next-release.ps1
```

What the next push to `main` would release.

```bash
pwsh -File scripts/verify-release.ps1 -Tag v0.1.0
```

Checks a published release's assets and checksums.

```bash
pwsh -File scripts/update-version.ps1
```

The only thing that moves `[workspace.package] version` in the root
`Cargo.toml`. Never edit that line by hand.

Packaging itself — the portable zip, the checksums, the signature, the update
manifest — happens in the **release workflow**, not in a local script. There is
no `build-and-package.ps1` in this repository; don't invent one.

## This machine

- Use `cargo test -j 4`. A full parallel build can exhaust the page file and
  corrupt `.rmeta`/`.rlib` files; if that happens, delete `target/debug` and
  rebuild.
- Watch disk space. Delete `target/debug/incremental` after long sessions of
  repeated builds. Do **not** set `CARGO_PROFILE_DEV_DEBUG=0` on the command
  line.
- Keep `.cargo/config.toml` (`+crt-static`) so the MSVC build stays portable —
  the point of the whole project is one exe that runs from a folder.
- The CI runner is this machine: don't run local cargo commands while a CI job
  is running — check the forge's `/actions/tasks?limit=4` first.

## Assets

Phase 0 must create `assets/icon/` — the app icon in the sizes the exe, the
tray and the installer-free zip need. It does not exist yet. Generate the sizes
from one source SVG with a script committed alongside them, never by hand, and
say in the ADR or the phase plan which tool produced them.

`assets/prayer/jakim/` holds the bundled JAKIM dataset (Phase 5, subject to the
terms check), and `data/jakim_zones.csv` the zone table — which changes only
through a reviewed commit with its source noted in `docs/CHANGELOG.md`.

README screenshots and GIFs: `docs/screenshots/README.md`.
