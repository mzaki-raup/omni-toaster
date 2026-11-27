# 08 — Decisions (ADR log)

**Last updated:** 2026-11-08

One record per decision a later reader would otherwise have to guess at: a new
dependency, a wire protocol, a storage format, a UI structure, a platform
restriction. Numbering continues from the last record in this file and is never
reused. Use [.claude/templates/adr.md](../.claude/templates/adr.md).

**Status** is `Accepted` once the owner has approved the design,
`Implemented <phase>` once code lands against it, `Proposed` while it is still
open, and `Superseded by ADR-0NN` when it is replaced — never silently edited.
The records below were accepted in the proposal review on 2026-11-08; none is
implemented, because Phase 0 has not started.

---

## ADR-001 · A Cargo workspace of `ot-*` crates, not a single crate

**Status:** Accepted · **Phase:** 0

**Context.** [PROPOSAL.md](../PROPOSAL.md) §4 drew a single crate with a flat
`src/`. §5 then drew eight modules with a strict one-way dependency rule and a
requirement that all Windows API code be isolated so macOS and Linux can be
added later. A flat `src/` cannot enforce either: nothing stops a module
importing the `windows` crate, and nothing stops the renderer calling the
scheduler.

**Options.**

| Option | For | Against |
|---|---|---|
| **Workspace, binary + ten libraries** | The compiler enforces the dependency direction and the platform isolation; `cargo test -p ot-sched` is fast; the existing `scripts/update-version.ps1` and both CI workflows already assume `[workspace.package]` | Eleven `Cargo.toml` files; a little more ceremony per new module |
| Single crate with modules | Simplest possible layout; matches the proposal's drawing | Module boundaries are advisory only; the Linux-clippy guarantee becomes a review item instead of a compile error; every tooling script needs rewriting |

**Decision.** A workspace. Root `Cargo.toml` carries `[workspace.package]
version` and every member declares `version.workspace = true`. The binary crate
is `crates/omnitoaster` (package `omnitoaster`, binary `OmniToaster`); the
libraries are `ot-core`, `ot-i18n`, `ot-store`, `ot-platform`, `ot-ui`,
`ot-toast`, `ot-sched`, `ot-prayer`, `ot-weather`, `ot-media`.
[03-architecture.md](03-architecture.md) has the dependency graph.

**Consequences.** The version moves only through
`scripts/update-version.ps1`. Every gate command is `--workspace`. A new
workspace member added on a branch while a release bumps `main` is a known lock
conflict — [GOTCHAS.md](GOTCHAS.md) records it. `PROPOSAL.md` §4's repository
drawing is superseded by [03-architecture.md](03-architecture.md).

---

## ADR-002 · Threads and channels, not an async runtime

**Status:** Accepted · **Phase:** 0

**Context.** The scheduler must own time precisely across DST changes, system
resume and clock changes; HTTP happens a handful of times an hour; the UI is an
immediate-mode egui loop that must never block. The question is whether to
bring in tokio.

**Options.** A dedicated scheduler thread with crossbeam channels and blocking
`ureq` on short-lived worker threads · tokio with `reqwest` and
`tokio-cron-scheduler` · a hybrid with a small current-thread runtime.

**Decision.** Threads and crossbeam channels. HTTP is blocking `ureq` on worker
threads. There is no async runtime anywhere in the workspace.

**Consequences.** No `block_on`, no runtime handle to thread through every
crate, no async-in-trait friction in `PrayerSource`. The scheduler keeps full
control of DST and resume handling, which `tokio-cron-scheduler` does not give
us. The cost is that every network call occupies a thread while it waits, which
at this request volume is irrelevant. Results reach the UI through **bounded**
channels drained at frame start; the rules are in
[.claude/rules/threads-and-scheduling.md](../.claude/rules/threads-and-scheduling.md).

---

## ADR-003 · eframe / egui for both surfaces

**Status:** Accepted · **Phase:** 0

**Context.** The app needs custom-painted toasts with per-frame animation, a
transparent always-on-top borderless window, and a conventional settings
window.

**Options.** eframe/egui · Slint · Iced.

**Decision.** eframe/egui, with Slint documented as the fallback if a more
polished retained-mode settings UI is later preferred. Immediate mode suits
hand-painted toasts and frame-by-frame animation; multi-viewport support lets a
toast live in its own window while settings runs in another. Iced's transparent
window support on Windows is too weak for the glass requirement.

**Consequences.** Per-frame cost is a standing review concern: nothing inside
`ui()` may parse, lay out, compile a regex or diff without caching on a hash of
its inputs. Widget snapshot testing uses `egui_kittest`, and synthetic clicks
never form double-clicks, so double-click paths are exercised in the running
app.

---

## ADR-004 · Portable: all data beside the exe

**Status:** Accepted · **Phase:** 0

**Context.** The owner's requirement is a portable app — a folder that can be
carried on a stick or synced, leaving nothing behind.

**Decision.** Everything under `OmniToasterData/` beside the executable
([05-data-model-and-storage.md](05-data-model-and-storage.md)). Never the
registry, `%APPDATA%`, `%LOCALAPPDATA%` or `%TEMP%`. The single exception is
the HKCU `Run` value for "Start with Windows", which the user opts into and
which is refreshed on each launch so a moved folder still starts. Relative
paths everywhere. A writability check at startup, with a bilingual error and a
clean exit if it fails.

**Consequences.** `--data-dir` exists so development never touches the real
folder, and `--self-test` asserts in CI that no `OmniToaster*` path appeared
under `%APPDATA%`, `%LOCALAPPDATA%` or `%TEMP%`. An installer is out of scope.

---

## ADR-005 · A supervisor process, not only `catch_unwind`

**Status:** Accepted · **Phase:** 8

**Context.** "Auto start when apps crash" was an explicit requirement. A panic
hook cannot recover a GPU driver fault, a stack overflow or an aborting
allocator — exactly the failures a GPU-backed UI hits.

**Decision.** `OmniToaster.exe` runs as a supervisor by default and spawns
itself with `--worker`. It watches the worker's exit status, records crash
information and restarts with exponential backoff. After 5 crashes in 10
minutes it stops restarting and shows a bilingual notification with the log
path. [specs/supervisor.md](specs/supervisor.md) has the detail.

**Consequences.** One exe, two roles, so the CLI must be unambiguous and the
single-instance mutex must distinguish them. The tray icon belongs to the
worker, so a restart recreates it. Media decoding is a candidate for the worker
process precisely because a crash there is now survivable
([06-security-and-privacy.md](06-security-and-privacy.md)).

---

## ADR-006 · Project Fluent via `i18n-embed` for localization

**Status:** Accepted · **Phase:** 0

**Context.** English and Bahasa Melayu must both be first-class, switchable at
runtime, with variables and plural rules, and translator-friendly.

**Options.** Fluent (`i18n-embed`, `i18n-embed-fl`, `fluent-bundle`,
`unic-langid`) · `rust-i18n` · gettext.

**Decision.** Fluent. The `fl!()` macro checks keys at compile time, `.ftl`
files take translator context comments, and Fluent's variable substitution
means word order can change per language without code changes.

**Consequences.** Every visible string is a Fluent key present in **both**
`locales/en-US/omni_toaster.ftl` and `locales/ms-MY/omni_toaster.ftl`, added to
both in the same commit, with CI enforcing key parity. The files are edited with
the Edit tool and never with a heredoc — an unquoted heredoc expands `{ $var }`
and silently drops the message. `rust-embed` keeps the bundled locales inside
the exe while user overrides are layered over them from the data folder through
`i18n_embed::AssetsMultiplexor` with the `filesystem-assets` feature.
[10-i18n.md](10-i18n.md) has the policy.

**Amendment, 2026-11-08 (Phase 0, FEAT-0004).** `i18n-embed-fl`'s `fl!` macro
is **not** used, and the dependency is dropped. `fl!` and
`fluent_language_loader!` both derive the catalogue filename from the *crate*
name, so they look for `ot-i18n.ftl`; the locale files are named after the
product, `omni_toaster.ftl`, because they are a translator's catalogue and that
name is fixed by [05-data-model-and-storage.md](05-data-model-and-storage.md)
and by `PROPOSAL.md`. `i18n-config` has no setting to change the domain, so the
loader is constructed directly with `FluentLanguageLoader::new("omni_toaster",
…)`.

The cost is the compile-time key checking this ADR originally chose the stack
for. It is replaced by three tests in `ot-i18n`, which is weaker than a compile
error but not much: key parity between the two files, naming the missing keys
on each side; every key lowercase, hyphenated and using a documented prefix;
and `text()` logging a warning and returning the key rather than panicking when
a lookup misses. Renaming the locale files to match the crate would restore
`fl!`, and would be the thing to reconsider if key drift ever actually bites.

---

## ADR-007 · SQLite for data, TOML for settings

**Status:** Accepted · **Phase:** 0

**Context.** Reminders, history and caches need transactions and queries;
settings need to be human-editable in a portable folder.

**Decision.** `rusqlite` over `data/omnitoaster.db` for reminders, categories,
history and caches, with numbered migrations tracked by `PRAGMA user_version`.
`config/settings.toml` for configuration, mirrored into a `settings` table only
where the scheduler needs it inside a transaction. The file is read and written
with `toml` and `serde` derives in `ot_core::settings`, which is the only place
that touches it.

**Consequences.** All SQL is parameterized. A shipped migration is never
edited; a correction is a new migration. Windows paths in TOML use
single-quoted literal strings, because a double-quoted string processes `\U` and
one bad escape fails the whole file.

---

## ADR-008 · JAKIM e-Solat is the default prayer source in Malaysia

**Status:** Accepted · **Phase:** 5 (endpoint verification is a Phase 5 spike)

**Context.** Calculated times differ from the times Malaysian mosques, schools
and offices actually follow. A praying user who sees a time that disagrees with
their mosque will not trust the app again.

**Decision.** `JakimSource` is the default for Malaysian locations;
`CalculatedSource` (`adhan`, Malaysia preset, Shafi'i for Asar) is the offline
and overseas option and the last link in the fallback chain. Both implement
`PrayerSource`; the scheduler sees times plus a source label, which is always
visible in the UI. [11-prayer-and-azan.md](11-prayer-and-azan.md) has the zone
mapping, the cache schedule and the fallback order.

**Consequences.** A third-party endpoint becomes a correctness dependency, so:
a fixture-based parser, a monthly cache refreshed weekly, a bundled dataset, an
attribution line in both languages, and a licence check before any dataset is
redistributed. The endpoint's exact shape is unverified until the Phase 5
spike, whose result is recorded as a new ADR.

---

## ADR-009 · Open-Meteo for weather

**Status:** Accepted · **Phase:** 6

**Context.** Weather-change notifications need a free, keyless, reliable
source, because the app has decided to hold no credentials
([ADR-011](#adr-011--no-secret-vault)).

**Decision.** Open-Meteo. Weather codes map to Fluent keys, so no API text is
ever displayed and both languages are covered by our own strings. Temperature
is °C in both languages.

**Consequences.** A TTL cache and an explicit stale marker, so an offline run
shows old data honestly rather than nothing. Coordinates are rounded before
they are sent ([06-security-and-privacy.md](06-security-and-privacy.md)).

---

## ADR-010 · An in-app updater over a signed manifest

**Status:** Accepted · **Phase:** 9

**Context.** The app is portable, so there is no installer and no package
manager to update it. The owner requires an in-app updater rather than asking
users to re-download a zip.

**Decision.** Every release publishes a fixed asset set the updater reads by
name: `OmniToaster.exe` (the bare executable), `OmniToaster-<version>-windows-x64.zip`
(exe + README + LICENSE + notes), `manifest.json` (version, channel, and each
asset with its size and SHA-256), the checksums, and an rsign signature. The
updater verifies the signature and the hash before replacing anything, and the
replacement is staged so a failed update leaves the working exe in place.
[specs/updater.md](specs/updater.md) has the flow.

**Consequences.** The asset names are a contract: a release missing them reads
to the app as "no installable release", and users stay where they are. Both
release workflows and `scripts/verify-release.ps1` enforce the set. Renaming an
asset is a breaking change requiring a new ADR.

---

## ADR-011 · No secret vault

**Status:** Accepted · **Phase:** 0

**Context.** The scaffolding this project inherited assumed an encrypted vault
and `secret:<ulid>` references throughout. OmniToaster holds no credentials:
JAKIM, Open-Meteo and the IP lookup all need none, and there is no account.

**Decision.** No vault is built. If an API key ever becomes necessary it goes
in the Windows Credential Manager through `ot_platform`, never in a file and
never in the repository. Security effort goes instead into input validation,
path canonicalization, parameterized SQL and keeping crash data local
([06-security-and-privacy.md](06-security-and-privacy.md)).

**Consequences.** Any future feature that needs a credential reopens this
decision with a new ADR rather than quietly adding a storage format. Nothing in
the rules or the templates should mention a vault.

---

## ADR-013 · One raster master for the icon, generated with ImageMagick, embedded with `winresource`

**Status:** Accepted · **Phase:** 0

**Context.** `.claude/rules/build-and-run.md` asked for the icon sizes to be
generated from **one source SVG** by a committed script. The owner supplied the
artwork as a 512 × 512 RGBA PNG, not an SVG, and it is a detailed illustration —
three stacked cards, a bell and its rays — rather than something that would
redraw well as vector shapes.

**Options.**

| Option | For | Against |
|---|---|---|
| **Keep the PNG master and downscale** | 512 is the largest size a Windows `.ico` uses, so nothing is ever upscaled; the artwork arrives exactly as the owner drew it | Cannot be re-rendered larger later; small sizes need help to stay legible |
| Trace it to SVG first | Matches the original rule; re-renders at any size | A trace of this illustration would not be the same icon, and the gradients and soft shadows would have to be approximated |

**Decision.** The master is `assets/icon/omnitoaster-512.png`.
`scripts/make-icons.ps1` generates 256, 128, 64, 48, 32, 24 and 16 px PNGs plus
a seven-entry `omnitoaster.ico`, using ImageMagick 7 with a Lanczos filter.
Sizes at or below 32 px additionally get an unsharp pass: without it the
composition collapses into a coloured blob at 16 px, which is the size the tray
and the title bar use. `crates/omnitoaster/build.rs` embeds the `.ico` and the
VERSIONINFO block with `winresource`, guarded so it is a no-op on non-Windows
targets and Linux clippy still passes.

**Consequences.** The rule's "source SVG" wording is corrected to "source image,
SVG where there is one". The master must stay square and at least 512 px — the
script refuses otherwise, so nobody can quietly introduce an upscale. Nothing
in `assets/icon/` except the master may be edited by hand; `make-icons.ps1
-Check` verifies the generated set is present. `build.rs` needs the Windows
SDK's `rc.exe`, which comes with the MSVC build tools, and panics with that
sentence if it is missing rather than producing a default-looking exe. **16 px
remains the weakest size** even sharpened; if it matters more than keeping one
composition at every size, the alternative is a cropped bell-only variant for
16 and 24, which would be a new ADR because it changes what the icon *is* at
those sizes.

---

## ADR-012 · Two forges: Forgejo authoritative, GitHub mirrored

**Status:** Accepted · **Phase:** 0

**Context.** The owner runs a self-hosted Forgejo instance and wants the
project public on GitHub. Only one of them can own the version number.

**Decision.** `origin` is the Forgejo instance and holds version authority; its
`release.yml` owns `main` and publishes every release. GitHub is a public
mirror, fast-forwarded by the release job and by
`scripts/push-to-mirror.ps1`; changes made on GitHub come back through
`scripts/sync-from-github.ps1`, fast-forward only. Forgejo's runner is the
owner's own Windows machine, so every whole-workspace build there is `-j 4`;
the `.github/` copies run on GitHub-hosted machines and are deliberately
unbounded.

**Consequences.** Never set a Forgejo push mirror while GitHub is writable —
the two would fight. Don't run local cargo commands while a CI job is running,
because it is the same machine. `.github/` jobs are guarded so Forgejo Actions,
which also reads `.github/workflows/`, does not try to run them.
[09-ci-cd-and-releases.md](09-ci-cd-and-releases.md) has the pipeline.

## ADR-014 · No bundled UI font: egui's `default_fonts` is off

**Status:** Accepted · **Phase:** 1

**Context.** `egui` enables its `default_fonts` feature by default, which pulls
in `epaint_default_fonts` — Ubuntu Light, Hack and Noto Emoji — under
`OFL-1.1 AND Ubuntu-font-1.0`. Neither licence is on the project's allow list,
so adding `egui` to `ot-ui` failed `cargo deny check` on the licence pass. The
fonts also add about a megabyte to a portable exe that is meant to stay small,
and Ubuntu Light is not the font a Windows app should be drawing in.

**Decision.** `egui` is taken with `default-features = false`. OmniToaster
draws in the Windows UI font, loaded from the system at startup by the
settings window (FEAT-0014). An icon font, when one is needed, is a bundled
asset with its own entry in `THIRD_PARTY_NOTICES.md` and its licence added to
`deny.toml` deliberately rather than inherited from a default feature.

**Consequences.** Any crate adding an `egui` or `eframe` feature must keep
`default-features = false`, or the font licences come back. `eframe` is taken
with `glow` (the renderer) and `accesskit` (which is what gives the settings
window to a screen reader, and is an egui default worth keeping); `persistence`
stays off, because window geometry belongs under `OmniToasterData/` and not
wherever eframe would put it (ADR-004). That dependency set brings the Boost
licence (BSL-1.0) through `arboard`'s Windows clipboard, which is permissive
and now on the allow list. A build that cannot
find the system font has no glyphs at all, so the loader must fail loudly
rather than render an empty window. The unmaintained-`ttf-parser` advisory
(RUSTSEC-2026-0192) still applies, because `epaint` rasterises through
`ab_glyph` whichever font it is given; it is ignored in `deny.toml` with the
reason that the app only ever parses fonts it chooses to load.

## ADR-015 · A toast's rounded corners are cut with a window region, not painted transparent

**Status:** Implemented Phase 2 · **Phase:** 2

**Context.** A rounded rectangle or pill toast leaves its corners unpainted
and relies on the window being transparent there. eframe's `glow` renderer
chooses one OpenGL pixel format for every window, and on the reference
machine's Intel Iris Xe driver no format the desktop compositor treats as
transparent is offered, even when asked for: the corners came out black
(BUG-0007). Moving to `wgpu` would bring a large dependency tree and its own
uncertainty about transparent swap chains on Windows.

**Decision.** Each toast window is cut to its painted shape with a Win32
window region (`SetWindowRgn`), built corner by corner from the same radii
the renderer paints with (`ot_toast::render::corner_radii`), in the window's
own pixels. The code lives in `ot_platform::toast_window` with the rest of the
Win32 toast-window handling. The viewports still ask to be transparent, which
costs nothing where a driver supports it.

**Consequences.** Corners are see-through on any driver. A region's edge is
whole pixels, so a curve is stair-stepped rather than anti-aliased. Nothing
outside the shape can show, so a drop shadow beyond the toast is not possible
this way — none was visible before either, the window being the toast's own
size; a shadow is therefore Windows' own (frame extended by one pixel), which
the compositor draws only for an uncut window, so only a Rectangle toast has
one (CR-0006). Opacity uses the window's alpha (`WS_EX_LAYERED`), which Windows
composites against the desktop on any driver; it fades the text with the
background, so the owner set its floor at 70% (CR-0005). Painting the toast's
colours translucent instead only blended them against black.

**Revisited by ADR-016 (2026-11-17):** the driver does give per-pixel alpha once the frame is extended into the client area; the region, the opacity and the shadow above stay as they are by the owner's choice.

---

## ADR-016 · Toasts get per-pixel alpha from DWM, glass from the accent policy, and are painted by the root's frame

**Status:** Accepted 2026-11-17 · **Phase:** 3 · **Revisits:** ADR-015's premise

**Context.** Phase 3 needs three things Phase 2 had concluded this machine
could not give: see-through pixels (dissolve, glow, alpha to solid, a pop that
overshoots its own edge, a Genie warp), a blurred glass behind the toast, and
every toast animating at 60 fps at once. A spike (FEAT-0028) measured each on
the reference machine — Intel Iris Xe, driver 32.0.101.7079, 60 Hz, Windows 11
26H2 build 26300, transparency effects on — by capturing the screen over a
window of red and blue stripes.

**What the spike found.**

| Question | Result |
|---|---|
| Unpainted pixels, cleared to alpha 0, frame **not** extended | Black. ADR-015 was right about this much |
| The same with `DwmExtendFrameIntoClientArea(-1)` and no backdrop | **See-through**, and alpha 128 is half see-through. The GL format has no `WGL_TRANSPARENT_ARB` (so eframe still logs "Cannot create transparent window"), but DWM composites the surface's alpha channel once the frame covers the client area. ADR-015's "the driver offers no transparent window" was the wrong conclusion from the right symptom |
| Windows 11 backdrops (`DWMWA_SYSTEMBACKDROP_TYPE` Mica, Acrylic, Tabbed) | A flat colour — Windows draws the inactive fallback, and a toast is never active. No blur |
| Accent policy `ACCENT_ENABLE_BLURBEHIND` (`SetWindowCompositionAttribute`) | Black on this build |
| Accent policy `ACCENT_ENABLE_ACRYLICBLURBEHIND` | **Blurs**, with its tint, on an inactive window, with or without the corner region |
| A Rectangle toast with Windows' shadow on (non-client rendering enabled) | Opaque black in every mode: the shadow and per-pixel alpha exclude each other |
| Three **deferred** viewports, each asking to repaint | Only the newest (topmost) is ever repainted — 60 fps for it, **none** after the second frame for the others, with or without vsync, whoever asks for the repaint |
| Three **immediate** viewports, painted from the root's frame, every window moved each frame | All three repainted together: 54–56 fps in a **debug** build with vsync |

**Options.**

| Option | For | Against |
|---|---|---|
| **Extended frame for alpha, accent-policy Acrylic for glass, immediate viewports** | Everything Phase 3 asks for works on the reference machine; no new dependency | `SetWindowCompositionAttribute` is undocumented, so it is loaded by name and a refusal means the solid fallback |
| `window-vibrancy` crate | Named in `PROPOSAL.md` | Its Mica and Windows 11 Acrylic are the backdrops that go flat on a toast; its Windows 10 Acrylic is the same accent call, a dependency for twenty lines |
| `wgpu` renderer | Transparent swap chains on some drivers | A large tree, and the alpha problem was never the renderer |

**Decision.**
- Every toast window has its frame extended into the whole client area and is
  cleared to alpha 0, so the renderer's alpha reaches the desktop. The corner
  region of ADR-015 stays (the owner kept Phase 2's look, 2026-11-17), and so
  do window-alpha opacity (CR-0005) and the Rectangle-only shadow (CR-0006).
- **Glass is the accent policy's Acrylic, three ways** (the owner's choice,
  2026-11-17): Blur is the blur with a near-transparent tint, Acrylic is tinted
  with the toast's background, Mica-like is tinted heavily. A refused call, or
  Windows older than 10 1809, falls back to the solid background, logged.
  Glass turns Windows' shadow off, since the two cannot coexist.
- Toasts become **immediate** viewports, painted inside the root's frame, which
  is also what the parked settings window keeps alive (BUG-0008). The
  animation clock is that frame.
- The **reference machine** for the 60 fps exit criterion is the one above,
  measured in a **release** build.

**Consequences.** One Win32 call more per toast, in `ot-platform`. The root's
frame rate is every toast's frame rate, so the settings window's own cost
counts against the animation budget — the frame-time fallback measures the
whole frame. A Rectangle toast with glass has no shadow, and Settings says so.
The undocumented call can disappear in a future Windows; the fallback is the
plan for that day, not an error.

**Amended 2026-11-18 (FEAT-0030).** Building glass showed two things the spike
did not. The accent blur covers the **whole window rectangle** and ignores the
corner region — the spike's "with or without the corner region" was judged on
the blur, not on the corners — so the owner chose **glass for the Rectangle
only**, greyed out for the other shapes like the shadow. And a pure black
accent tint draws the window black, so the accent's own tint is a faint grey
and the toast paints its tint over it. The owner also set Blur's tint to 45%
rather than near-transparent, because text chosen for the toast's colour
vanished over a desktop of the opposite lightness (`docs/specs/toast.md` §4.2).

**Amended 2026-11-18 (FEAT-0031): no vsync.** With the timeline built, the
debug build animated three toasts at 11–16 fps, not the spike's 54–56. Frames
arrived every 62 or 104 ms — whole multiples of the 16.7 ms refresh — while the
app's own work took 5–10 ms: with vsync, the root and each toast window
presented in turn, each waiting for the next refresh. A minimised settings
window made it worse (12 fps against 32). eframe now runs with `vsync: false`;
the compositor already keeps a windowed app from tearing, and the app paces
itself, asking for a frame every 1/60 s while anything moves. Measured on the
reference machine, three toasts, release build: every in and out animation at
137–444 fps, with the settings window open or minimised; 62 fps in a debug
build with it open. The toast windows' own events drive frames beyond the 60
asked for; that costs 6% of one core while three toasts are on screen, against
1% with none, which is accepted for the seconds a toast is up.

## ADR-017 · Dates and times with `jiff`, not `chrono`

**Status:** Accepted 2026-11-19 · **Phase:** 4 · **Replaces:** the `chrono` +
`chrono-tz` row of `PROPOSAL.md` §4

**Context.** RUSTSEC-2026-0543 (2026-11-17) marks `chrono` unmaintained, with
no upgrade; the advisory names `jiff` and `time`. It was not a vulnerability —
`chrono` parsed no outside input here — so it was ignored in `deny.toml` with
CR-0007 to replace it before Phase 4, because the scheduler and the prayer
times are where a date library becomes hard to replace. What they need:

- **Named time zones.** JAKIM's times are `Asia/Kuala_Lumpur` whatever the PC
  is set to (`docs/11-prayer-and-azan.md` §4), and Windows ships no IANA
  database.
- **Local wall-clock arithmetic that is safe across DST.** "Every day at 07:00"
  stays 07:00; a time a DST gap skips, or a fold repeats, has one defined
  answer (`docs/specs/reminders.md` §7).
- **Absolute instants** for the queue and for a pause, and plain civil dates
  and times for the formatter.

**Decision.** `jiff` 1.0, default features: `tz-system` reads the Windows zone,
`tzdb-bundle-platform` bundles the IANA database on Windows, so
`Asia/Kuala_Lumpur` resolves on every machine without a download. Licence MIT
or Unlicense; `deny.toml` admits MIT. `chrono` leaves the tree entirely —
nothing else in the dependency graph used it — and its advisory ignore goes.

**Why not `time`.** It has fixed offsets but no named zones and no DST rules
without a second crate, so the scheduler would have to assemble what `jiff`
already does.

**Consequences.** `ot_i18n::format` takes a `jiff::civil::DateTime`; a pause
holds a `jiff::Timestamp` and is shown in the system zone. Crash-report and
log-file timestamps use `jiff`'s `strftime`, which they are allowed to:
they are diagnostics, not user-visible text, so `ot-i18n`'s formatter rule does
not cover them. The executable grows by the bundled zone database.

---

## ADR-018 · The e-Solat endpoint, as the Phase 5 spike found it

**Status:** Accepted 2026-11-20 · **Phase:** 5 · **Settles:** ADR-008's
unverified endpoint; `docs/11-prayer-and-azan.md` §12 questions 1 and 2

**Context.** ADR-008 made JAKIM e-Solat the default prayer source in Malaysia
before anyone had looked at the endpoint, which is not a documented public
API. Phase 5 starts by verifying it against three zones — SGR01, WLY01 and
JHR01 — before anything is built on it. The raw responses are kept in
`crates/ot-prayer/tests/fixtures/esolat-2026-11-20/`, captured 2026-11-20.

**What the endpoint is.**

- `https://www.e-solat.gov.my/index.php?r=esolatApi/takwimsolat&zone=<ZONE>&period=<P>`,
  HTTPS, no key, no account, answered from CloudFront in Singapore in under
  half a second. A lowercase zone is accepted.
- `period=today` (one day), `week` (today and seven more), `month` (the
  calendar month), `year` (the calendar year, 365 days, 77 KB).
- `period=duration` takes `datestart` and `dateend` (`yyyy-mm-dd`) **in a POST
  body**; the same values as query parameters give HTTP 500. A range may span
  months, but **only within a published year**: today, January 2027 has no
  data, and a December-to-January range stops at 31 December.

**What it answers.**

```json
{
  "prayerTime": [
    { "hijri": "1448-06-10", "date": "20-Nov-2026", "day": "Friday",
      "imsak": "05:39:00", "fajr": "05:49:00", "syuruk": "06:58:00",
      "dhuha": "07:23:00", "dhuhr": "13:02:00", "asr": "16:24:00",
      "maghrib": "18:59:00", "isha": "20:12:00" }
  ],
  "status": "OK!", "serverTime": "2026-11-20 09:58:09", "periodType": "month",
  "lang": "ms_my", "zone": "SGR01", "bearing": "291&#176; 7&#8242; 23&#8243;"
}
```

- Times are `HH:MM:SS` (seconds always `00`), not the `HH:MM` the spec
  guessed. All 365 days of the year's response had eight well-formed times.
- `date` is `DD-Mon-YYYY` with **Malay month abbreviations**: `Jan Feb Mac Apr
  Mei Jun Jul Ogos Sep Okt Nov Dis` — August is four letters. `day` is in
  English. `serverTime` is Malaysian local time.
- No data — an unknown zone, or a year not yet published — is **HTTP 200**
  with `status: "NO_RECORD!"` and `prayerTime` an **object**
  (`{"data": ["No data available …"]}`), not an array.
- A malformed request is HTML: a missing zone is HTTP 400 `text/html`; the
  duration-as-GET mistake is HTTP 500 with `Content-Type: application/json`
  and an HTML body. The content type cannot be trusted.

**Asar is Shafi'i.** On 20 November, standard solar formulas give Shafi'i Asar
(shadow 1) at 16:23 for Shah Alam, 16:22 for Kuala Lumpur and 16:11 for Pulau
Aur, and Hanafi (shadow 2) at 17:19, 17:18 and 17:08. JAKIM publishes 16:24,
16:24 and 16:12: Shafi'i, a minute or two later — the same precaution its
Dhuhr shows against solar noon (13:02 against 12:59.9 for SGR01).

**Decision.**

- The parser reads the shape above: `status` must be `OK!` and `prayerTime` an
  array, or the response is "no data" (not an error to retry); dates parse with
  both the Malay and the English month abbreviations; times accept `HH:MM:SS`
  and `HH:MM`; a missing or malformed field invalidates its whole day, and
  unknown fields are ignored. The body is capped before parsing (a year is 77
  KB; 256 KB is the cap) and must parse as JSON whatever its content type.
- The weekly fetch asks for **today to the end of next month** with one POST
  `period=duration` request — or to 31 December, when next month is in a year
  JAKIM has not published. From mid-December the chain's later links (bundled
  data, then Calculated) cover the gap, and a fetch is retried daily until the
  new year appears.
- Calculated mode keeps Shafi'i as its default Asar, now confirmed.

**Consequences.** The fixtures are real responses, never edited; a format
change means a new dated capture beside them and a `GOTCHAS.md` entry. The
year boundary is a known, yearly gap the fallback chain must handle and the UI
must label, not an outage. The endpoint stays undocumented — nothing here
promises it will keep this shape.

---

## ADR-019 · `ureq` with the Windows TLS stack, for the prayer fetch

**Status:** Accepted 2026-11-26 · **Phase:** 5 · **Builds on:** ADR-002 (blocking
`ureq` on worker threads)

**Context.** ADR-002 chose blocking `ureq` without saying which TLS stack it
would use, and `ureq` 3 builds for `rustls` unless told otherwise. FEAT-0052 is
the first HTTP in the app, and the weather fetch, the geolocation lookup and
the update check will reuse the choice. `deny.toml` bans `openssl-sys`, and the
exe is one static file that runs from a folder.

**Options.** `rustls` with its bundled root list (what `ureq` ships by
default) · `rustls` with the platform verifier · `native-tls`, which on Windows
is SChannel and the Windows certificate store.

**Decision.** `ureq` 3.5.2 with `default-features = false` and
`native-tls-no-default`. The agent is configured **explicitly**:
`TlsProvider::NativeTls` and `RootCerts::PlatformVerifier`, plus
`https_only(true)`, a ten-second global timeout, `User-Agent:
OmniToaster/<version>`, `http_status_as_error(false)` (the client names a 500
itself, because e-Solat's 500 says JSON and sends HTML), and a body limit of
`jakim::MAX_BODY` applied as the body is read. Gzip stays off: the answers are
small.

**Why Windows TLS.** The owner chose it when FEAT-0052 began. There is no second
crypto library in the exe, no C or assembly build step for one (`ring`,
`aws-lc`), and certificates are checked against the store Windows keeps up to
date and the user's organisation may extend, rather than a root list frozen into
a release. The cost is that the exe is not self-contained about TLS: it is as
current as the Windows it runs on.

**Consequences.**

- **The configuration is not optional.** `ureq` never picks the provider up by
  itself, and with `native-tls-no-default` its default root list
  (`RootCerts::WebPki`) is compiled out and panics at runtime when used. A
  build that compiles proves nothing, so the live three-zone test
  (`cargo test -p ot-prayer -- --ignored`) is the check, and the client is
  built in one place, `ot_prayer::fetch::Client`.
- **`openssl-sys` stays banned**, and the reason in `deny.toml` is reworded: it
  said "use rustls", which this decision contradicts. `deny.toml` checks the
  Windows target only, where `native-tls` uses SChannel and no OpenSSL. On
  Linux `native-tls` would pull in `openssl-sys`; nothing in CI builds there,
  and a Linux build of the workspace is not supported by this decision.
- **Cancel is bounded, not instant.** Blocking `ureq` cannot interrupt a read in
  progress. A cancelled fetch is stopped before it sends and its result is
  dropped before it is delivered, and the ten-second timeout bounds the time in
  between. The worker has no path to the cache, so a late answer cannot be
  written (docs/11-prayer-and-azan.md §4).
- **Accepted by the owner (2026-11-26):** the bounded cancel, as the working of
  `threads-and-scheduling.md`'s "cancel must actually stop the work" with blocking
  `ureq`, and a Windows-only workspace, as the working of `rust.md`'s "clippy must
  pass on Linux".
- **Note, 2026-11-26 (FEAT-0057): the basis of the owner's acceptance changed.**
  "The worker has no path to the cache" was true when the owner accepted a bounded
  cancel. The Prayer pane's resolve thread now writes the cache through the chain.
  `Chain::resolve` checks the cancel flag before every store and a test holds it
  (a `Fetch` that cancels mid-call and still answers stores nothing). A cancel
  landing between the check and the write can still leave one complete range in
  the cache. The owner is told, and the acceptance stands unless they say otherwise.
- Later HTTP callers (weather, geolocation, updates) build their agent the same
  way and cite this ADR.

---

## ADR-020 · `adhan` for calculated prayer times, and a Malaysia preset at 18°

**Status:** Accepted 2026-11-26 · **Phase:** 5 · **Builds on:** ADR-008 (calculated
times as the last resort), ADR-017 (`jiff`, not `chrono`)

**Context.** Calculated mode is the offline and overseas source and the last link
of the fallback chain. ADR-008 named the `adhan` crate before anyone had looked at
one, and the spec fixed the Malaysia preset at Fajr 20.0° and Isha 18.0°.

**Options.** The `adhan` crate (0.2.0) · `salah`, the older port · our own
implementation of Meeus's astronomical algorithms.

**Decision.** `adhan` 0.2.0. It has **no dependencies**, forbids `unsafe`, is
Apache-2.0 (already allowed by `deny.toml`), and reports its own checking against
the adhan-js reference library, the implementation most prayer apps use. `salah`
depends on `chrono`, which ADR-017 rules out. Writing the astronomy ourselves would
be the one place in the app where a quiet error moves a prayer, and a published,
reference-checked implementation is the better thing to lean on.

**What we checked ourselves, because the crate is young** (a 0.2 from a small
organisation, not a long-established one):

- **The three non-Malaysian presets** against 60 rows of adhan-js output the crate
  ships as test fixtures (`tests/fixtures/adhan_js_presets.tsv`): five places east
  and west of Greenwich, two dates, both Asar schools. This checks our wiring, not
  the crate's astronomy against an independent timetable.
- **The Malaysia preset against JAKIM itself**: the spike's three zones, 30 days
  each. This is the independent check, and it changed the preset.

**The Malaysia preset is Fajr 18.0° and Isha 18.0°, not 20.0° and 18.0°.** At a
Fajr of 20.0° every calculated Fajr is 10 to 12 minutes earlier than JAKIM's in all
three zones; the data fit about 17.5°, and 18.0° keeps every time within three
minutes. Isha at 18.0° was right. The owner chose 18.0° on 2026-11-26 over 17.5°
(a value fitted to one month of data, not a published figure, under a name that
says "JAKIM angles") and over keeping 20.0°. The spec, `PROPOSAL.md` and both
READMEs say 18.0°.

**Consequences.**

- JAKIM publishes Dhuhr, Asar, Maghrib and Isha one to three minutes after the
  calculation and Syuruk within three minutes either way. **No offsets are added**:
  they are a fitted guess, and the label "approximate" stays true.
- `adhan` gives a time as a UTC clock reading with the day taken off, so
  `calculated::place_on_day` puts it on the neighbouring UTC day nearest the
  place's solar noon.
- Inside the polar circles a time can be absent. It is `None`, never a made-up
  number, and the fallback chain (FEAT-0055) must treat a day with a missing time
  as a gap.
- Imsak and Dhuha are JAKIM's own and are not calculated.

---

## ADR-022 · Audio with `rodio`, on a thread of its own

**Status:** Accepted 2026-11-27 · **Phase:** 5 · **Builds on:** ADR-002 (threads, no
async), `docs/specs/announcements.md` §7

**Context.** The Azan, the chime and later the spoken announcements need sound,
and the spec named `rodio` before anyone looked. A clip the user imports is a
file from outside, so it follows the rules a picture does.

**Decision.** `rodio` 0.23 with `default-features = false` and `playback`, `wav`,
`mp3` and `vorbis`: WASAPI through `cpal`, and WAV, MP3 and Ogg Vorbis through
`symphonia`. No recording, dithering, FLAC or MP4. `cargo deny` admits the tree
as it stands.

- **One thread owns the one output device** (`ot_media::Player`), opened on the
  first clip so a machine with no speakers costs nothing until something plays. A
  new clip replaces the one playing: an Azan is never mixed. A device that cannot
  be opened is logged, reported by `has_no_device`, and silent — never a panic.
- **Imports follow the picture rules** (`ot_media::audio::check`): at most 20 MB
  before a byte is read; the format from the bytes (RIFF/WAVE, an ID3 tag or an
  MPEG frame sync, `OggS`), never the name; and a **bounded decode** of the whole
  clip up to ten minutes — the persistent-toast cap — so a corrupt file or one
  that never ends is refused with a message. The copy under a hashed name is the
  store's job and never leaves `assets/`. Decoding in the worker process is the
  Phase 8 hardening.
- **The clips that ship** are embedded: the stand-in chime, the Azan and the Azan
  for Subuh, from `assets/sound/`. They are the owner's, with no licence
  established (`assets/README.md`); **Phase 9's terms check decides whether they
  may ship**, and the default for a prayer is the short chime so the full Azan is
  a choice, not something a release plays unasked.

**Consequences.** The exe grows by the clips (about five megabytes) and by the
decoders. Playback cannot be proved by a test: the player's tests show it starts
and stops, or that the machine has no output and says so, and the audible check
is the app's.

**Update 2026-11-26 (CR-0008): the default is now the Azan.** The owner asked that
every prayer's call be the Azan, Subuh's own for Fajr, and chose to do it before
the licence is settled. A prayer whose template chose no sound now plays
`azan.mp3` (`azan-subuh.mp3` for Fajr), and the chime is a choice. **Syuruk is the
exception**: it is the sunrise, which has no call to prayer, so it keeps the chime.
The licence question above is **unchanged and still open**: a release now plays
clips whose terms Phase 9 has not checked, so that check must happen before the
release that should stand behind this default. A template saved before this
change with the chime chosen stored "nothing" (the chime was the default, so it
was not written down), and so now follows the new default; it cannot be told apart
from one that never chose, and the user chooses the chime again if they want it.

---

## ADR-021 · How the app learns where the user is

**Status:** Accepted 2026-11-26 · **Phase:** 5 · **Builds on:** ADR-019 (HTTP),
`docs/06-security-and-privacy.md`

**Context.** Prayer times, the zone and later the weather all need a place. A
place is personal data, and an IP lookup shows the user's address to a third
party, so the app asks before it looks, and it has to work for someone who says no.

**Probed 2026-11-26** (shapes only; the answers were the developer's own place):
`https://ipapi.co/json/` and `https://ipwho.is/` both answer over HTTPS with no
key, with coordinates, city, region, country code and an IANA zone; Open-Meteo's
`geocoding-api.open-meteo.com/v1/search` answers a city name with up to ten
places, each with coordinates and an IANA zone, and with no `results` key at all
when nothing matches.

**Decision.**

- **IP lookup: ipapi.co**, chosen by the owner over `ipwho.is`: better known, and
  its free allowance is far above one lookup a day. Only after the user consents,
  and at most once a day. Its terms for a free desktop app are to be re-read at
  the Phase 9 terms check.
- **City search: Open-Meteo geocoding.** It is already an allowed outbound service
  (the weather), it returns the zone, and it is queried only when the user
  searches, with only the text typed.
- **Coordinates typed by hand:** inside Malaysia the zone is `Asia/Kuala_Lumpur`
  (all of Malaysia is UTC+8). Elsewhere there is no free offline
  coordinate-to-zone lookup, so the zone is the PC's own and is shown for the
  user to confirm. The owner accepted this on 2026-11-26.
- **Consent** is `[location] ip_consent` in `settings.toml` — absent until asked.
  `Lookup::detect` refuses without it and sends nothing, so the guarantee does not
  depend on the UI calling it correctly.
- **Rounding:** every place is rounded to two decimals (about 1.1 km) on arrival,
  by whichever route, before it is stored or sent.
- **No IP is kept:** the IP answer carries one, and `Located` has no field for it.
- **The saved `location` row is the daily cache.** A place the user chose is
  `manual` and a detection never overwrites it; withdrawing consent keeps the
  saved place and stops looking.

**Consequences.**

- Typed coordinates abroad can be in the wrong zone if the PC's zone is not the
  place's; the confirmation step is the guard, and the user can search by city
  instead, which carries the right zone.
- Declining the lookup costs nothing but typing a city: city search sends the
  typed text and the IP address to Open-Meteo, as the weather will, and that is
  not covered by the IP-lookup consent. `docs/06` lists it as its own row.
- Two third parties now see an IP address at the user's request: ipapi.co once a
  day with consent, Open-Meteo when searching.

---

## ADR-023 · What Open-Meteo is, as verified, and how the app asks

**Status:** Accepted 2026-11-26 · **Phase:** 6 · **Builds on:** ADR-009 (Open-Meteo),
ADR-011 (no credentials), ADR-019 (blocking `ureq` over Windows TLS)

**Context.** ADR-009 chose Open-Meteo before anyone had looked at its answers.
FEAT-0065 does what FEAT-0049 did for e-Solat: ask the live service, keep the
raw responses (`crates/ot-weather/tests/fixtures/open-meteo-2026-11-26/`, captured
2026-11-26, with a README of how each was made) and build only on what they show.

**What the service is.**

- `GET https://api.open-meteo.com/v1/forecast`, HTTPS, no key, no account. A
  request with a few variables costs one call against 600 a minute, 5,000 an hour
  and 10,000 a day (`X-RateLimit-*` headers and the pricing page).
- The request the app makes: `latitude`, `longitude` (rounded to two decimals),
  `current=temperature_2m,weather_code`, `hourly=temperature_2m`,
  `past_hours=6`, `forecast_hours=1`, `timezone=GMT`. About 560 bytes back. The
  `past_hours` rows are the history a temperature threshold needs; without them
  there is only the current value.
- **`timezone=GMT` is deliberate.** With `timezone=auto` the times are local with
  no offset in the string, and London in winter has an offset of 0 while being a
  named zone. Asking for GMT makes every time UTC and lets the client refuse any
  answer whose `utc_offset_seconds` is not 0, instead of reading a local time as
  UTC.
- The answer's coordinates are the model grid cell's (3.1282952 for 3.14), not
  the ones sent; the app does not use them.
- `current.time` moves in 15-minute steps (`interval: 900`), which is why a poll
  faster than 15 minutes (the spec's minimum) buys nothing.
- Weather codes are WMO codes, a documented list of 29: 0, 1, 2, 3, 45, 48, 51,
  53, 55, 56, 57, 61, 63, 65, 66, 67, 71, 73, 75, 77, 80, 81, 82, 85, 86, 95, 96,
  97, 99. Codes 96 and 99 appear only with models that forecast hail. The API
  returns only numbers; every word is ours (FEAT-0066).
- **Failures.** A bad request is HTTP 400 with `{"error":true,"reason":"…"}`; the
  reason is the service's text and may leak its internals, so it is logged and
  never shown. **A request with no coordinates is HTTP 200 with an empty body**,
  so a 200 is not proof of an answer. Hourly values can be `null`.

**Decision.** `ot_weather::fetch::Client`: one blocking attempt on a worker thread,
the agent of ADR-019 (Windows TLS, ten seconds, `OmniToaster/<version>`, status
codes left to the caller), a 64 KB cap applied as the body is read, a `Cancel`
checked before the request and before an outcome is delivered, and the retry
schedule of 1, 5 and 15 minutes as a pure function. `ot_weather::observation::parse`
checks every field: a missing or odd one fails the whole answer, extra fields are
ignored, a temperature outside −100…70 °C and a code over 99 are refused, and
nothing panics. The coordinates are rounded where the URL is built, so no caller
can send more.

**Consequences.**

- `weather_cache` already exists in migration 001, so FEAT-0067 may need no new
  migration; it will say so.
- The agent builder and `Cancel` are written a second time here, because a
  service crate may not depend on another (`docs/03-architecture.md`). If a third
  service needs them they belong in a shared place; that is a change request, not
  a quiet third copy.
- **The free API is for non-commercial use, has no uptime guarantee, and the data
  is CC BY 4.0, which asks for credit** (the pricing page, read 2026-11-26).
  OmniToaster as a personal portable tool fits the free tier; shipping it to others
  commercially would need Open-Meteo's paid customer API, with a key, which
  reopens ADR-011. **The owner decides this before a release that includes
  weather is relied on**, and the About pane should credit Open-Meteo whatever is
  decided (FEAT-0070).
- 10,000 calls a day is far above one poll every 15 to 30 minutes (at most 96).

**Update 2026-11-27 (CR-0009).** The updater is the third service that needs the
agent and `Cancel`, which is the point at which the consequence above said to put
them somewhere shared. They now live in `crates/ot-net` — the agent of ADR-019 (with
a variant that takes a longer timeout, for a download) and the cancel flag — and
`ot-prayer` and `ot-weather` re-export `Cancel` and call `ot_net::agent`, so no call
site changed. `ot-net` holds no policy and reads no answer, so it is not a service
and the rule that services do not depend on each other is untouched.

---

## ADR-024 · The update source is fixed when the copy is built

**Status:** Accepted 2026-11-27 · **Phase:** 9 · **Builds on:** ADR-010 (the updater),
ADR-019, ADR-023

**Context.** OmniToaster is published from two forges, the self-hosted Forgejo at
`git.zacx.net` and a GitHub mirror of the same history, and each builds its own
release (`scripts/push-to-mirror.ps1` hands the tag to GitHub, whose `release.yml`
builds that version itself). The owner asked that a copy downloaded from GitHub
update from GitHub and one downloaded from git.zacx.net update from git.zacx.net.
The sibling `lock-screen` already does this.

**Options.** A setting the user switches (cannot know where the download came from,
and a setting can be pointed anywhere by anyone who can edit the file) · detecting
the download's origin from the file's Mark-of-the-Web (fragile: lost on copy, on
extraction by some tools, on any non-browser download) · **the release script bakes
the repository into the exe when it builds it**.

**Decision.** `OMNITOASTER_RELEASE_REPO` is set by `scripts/build-and-package.ps1`
to the `-RepoUrl` it was given — the forge it is building a release for — and
`ot_update::source` reads it with `option_env!`; a build with nothing set, which is
every local build, reads the Forgejo. The latest-release API is derived from it:
`https://api.github.com/repos/<o>/<r>/releases/latest` for GitHub and
`https://<host>/api/v1/repos/<o>/<r>/releases/latest` for a Forgejo, both answering
in the same JSON. A malformed value is refused (HTTPS, `host/owner/repo`, nothing
else) and falls back to the Forgejo with an error in the log. The settings pane
shows which source a copy uses and does not let it be changed. The check lives in
a new crate, `ot-update`, which depends on `ot-net` (CR-0009) and `semver`.

**Consequences.**

- The two forges' `OmniToaster.exe` differ by this one string, so their manifests
  list different digests; each release's manifest is self-consistent, and a copy
  verifies only what its own forge published.
- A release must carry `manifest.json`, `manifest.json.minisig` and
  `OmniToaster.exe`; one missing any is found but "not installable", and the user is
  pointed at its page.
- Only stable releases are offered (`releases/latest` excludes prereleases); the
  beta channel is a later step.
- Moving the project to another forge means one `-RepoUrl`, nothing in the source.

**Update 2026-11-27 (FEAT-0078): verifying.** `ot-update` verifies with
`minisign-verify` 0.3 (verification only, no dependencies of its own, MIT), against
the key committed as `keys/minisign.pub` and **embedded in the crate** with
`include_str!`. The signature is checked before the manifest is read as anything but
bytes; a failure fails closed and nothing is downloaded. Only the prehashed
signatures `rsign2` makes by default are accepted. The tests sign with a throwaway key
made at run time by the `minisign` crate (a dev-dependency, MIT), so no private key is
ever committed, and one test verifies the **real** v0.18.0 manifest and signature
(captured in `tests/fixtures/real-0.18.0/`) against the embedded key.
