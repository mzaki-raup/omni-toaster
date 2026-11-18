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
