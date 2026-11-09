# OmniToaster — work ledger

Every unit of work has an **id**, lives under exactly one **task version**, and
carries its own **checklist**.

```
/dev-task v0.0.1       every item under that task version
/dev-task FEAT-0001    that one item, and nothing else
```

## Task versions are not app versions

A heading here is **my own batch label** — a bag of work I want done together.
It is not the application's release number, it does not have to match it, and
nothing in this file moves `[workspace.package] version`. The app's number is
decided by the release pipeline from the Conventional Commit types, whatever this file is called.

So `## v0.0.1` here means "task batch v0.0.1". A batch may ship across two
app releases, or three batches may ship in one. Where it is worth recording
which release carried a batch, the heading gets a `> **Shipped in:**` note
after the fact.

Companion files, and what each is for:

| File | Holds |
|---|---|
| **TASKS.md** (this file) | what is to be done, by id and by task version, with its checklist |
| `CHANGELOG.md` | what was done, one line per change, under Unreleased until the app releases |
| `CHECKLIST.md` | how far the roadmap phases have got |

---

## Format

### Ids

`FEAT-0001` · `BUG-0001` · `CR-0001` — four digits, allocated once, **never
reused and never renumbered**, because commits, branches and `CHANGELOG` lines
point at them.

| Prefix | Means |
|---|---|
| `FEAT` | new capability the user can see |
| `BUG` | behaviour that contradicts its spec or crashes |
| `CR` | change request: a deliberate change to existing behaviour that is neither new capability nor a defect (a changed default, a renamed label, a tightened guard, a dropped option) |

The next free number per prefix:

```bash
grep -oE '^### (FEAT|BUG|CR)-[0-9]{4}' TASKS.md | sort | tail -5
```

### Task version headings

```
  ## v0.7.1 — Planned
  > **Shipped in:** app v0.3.0, 2026-11-04
```

(Indented here only so this example isn't itself matched as a heading — real
headings start at column 0.)

- The label is mine to choose: `v0.7.1`, `v3`, `sprint-11`, `2026-11`. One
  token, no spaces. Keep one style across the file so sorting stays sane.
- Status is `Planned`, `In-Progress`, or `Done <YYYY-MM-DD>` — "done" meaning
  every item in the batch is `Completed`, not that the app released. A
  reconstructed batch whose finish date nobody recorded says `Done` with no
  date rather than inventing one.
- The optional `> **Shipped in:**` line records which app release carried the
  batch, filled in afterwards. The task version is **never** renamed to match
  the app's number, and `[workspace.package] version` is never edited by hand.
- Unscheduled work sits under `## Backlog`. An item belongs to exactly one
  heading; moving it means moving the whole `###` block.

### An item

```markdown
  ### FEAT-0012 · Mica-like glass on the toast window
  - **State:** Pending · **Spec:** docs/specs/toast.md · **Phase:** 3
  - **Branch:** feat/phase-3-glass
  - **Depends on:** FEAT-0011
  - [ ] <step>
  - [ ] <step>
  - [ ] Test: <what it asserts>
  - [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
  - [ ] App: `--data-dir ./tmp-data` — <what to click>
  - [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
  - **Notes:** <anything that would otherwise be lost>
```

Rules that keep the file honest:

- **State** is `Pending`, `In-Progress`, `Completed` or `Blocked` — the same
  vocabulary as `CHECKLIST.md`, plus `Blocked`. `Completed` only once every box
  is ticked, with one exception: when I close something with boxes still open,
  the boxes **stay open** and the heading or the item's **Notes** says I closed
  it and what went unchecked. The State is my call; the boxes are the record of
  what was actually verified, and nothing may tick them on my say-so.
- The last four boxes (**Test**, **Gate**, **App**, **Docs**) are on every
  item **that is still to be done**. A box is ticked only when that check
  actually ran and passed. A check that cannot be made on this machine (no
  server here) is left unticked with the reason in **Notes** — never ticked
  optimistically.
- `Blocked` always has a reason in **Notes**.
- **Depends on** names ids. A batch runs dependencies first.
- `BUG` items get a reproducing test before the fix, so their **Test** box is
  the first thing that goes green.
- Optional fields: **Spec** (the document in `docs/` that the item must match —
  a numbered spec such as `docs/11-prayer-and-azan.md` or an area spec such as
  `docs/specs/toast.md`), **Phase** (`0`–`9`, the delivery phases of
  `PROPOSAL.md` §13), **Branch**, **Depends on**, **Source**
  (where the request came from), **Kind** (`major`, on the items that moved the
  minor digit) and **Verified** (what the `Completed` claim rests on).

### Commits

Reference the id in the footer so history maps back to the ledger:

```
fix(ot-prayer): keep the zone selection across a failed fetch

Refs: BUG-0003

```
### Latest Vacant/Free Task

```
Version : v0.0.2
Features : FEAT-0017
Bugs : BUG-0003
Change Request : CR-0002
```

---

## v0.2.0 — Planned

Phase 2, Toast Core. The Phase 2 exit criteria from `docs/07-roadmap.md`:
every shape renders correctly at 100%, 150% and 200% DPI in both languages
with no overflow, and the preview matches the real toast.

Ordered by dependency: FEAT-0017 is the model everything else draws, so it
comes first; FEAT-0018 gives it a window to live in and FEAT-0019 paints it;
FEAT-0020 and FEAT-0021 fill it with content; FEAT-0022 decides where it goes;
FEAT-0023 shows it inside Settings, which is the half of the exit criteria
that cannot be met by the renderer alone; FEAT-0024 is the evidence.

**The rule that governs the whole phase**: the renderer draws a `ToastSpec`
and nothing else. The preview and a real toast therefore cannot diverge, and
anything that renders without going through a `ToastSpec` is a defect, not a
shortcut (`docs/specs/toast.md` §3).

### FEAT-0017 · `ToastSpec`, and its limits enforced in the model
- **State:** Completed · **Spec:** docs/specs/toast.md §3, §4 · **Phase:** 2
- **Branch:** feat/phase-2-toast
- **Verified:** the gate — 11 tests over the model, including the hand-edited-file case
- [x] `ToastSpec` with its four parts — content, style, animation, behaviour — as `docs/specs/toast.md` §3 names them
- [x] Every limit in §4's table enforced **on construction and on load**, not only in the UI: width 280–600, height 80–300, radius 0–40, opacity 40–100%, border 0–4, duration 3–60 s, persistent capped at 10 minutes, 1–5 visible toasts, 1–3 actions
- [x] A settings file edited by hand to width 20 is clamped, with a logged warning naming the field and both values
- [x] Defaults that match the spec's table, so a spec with nothing set is a usable toast
- [x] Test: every field clamps at both ends; a round trip through TOML keeps what it was given; a hand-edited out-of-range value loads clamped rather than refused
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 (131 tests) · deny check · --self-test
- [x] App: not applicable on its own — nothing renders yet. The App box belongs to FEAT-0019
- [x] Docs: spec · CHANGELOG
- **Notes:** Clamping belongs in the model because the model is what the renderer trusts. A UI that clamps and a file that does not is the same defect wearing a hat.
  **A NaN is replaced by the default rather than clamped.** `f32::clamp` panics on a NaN bound and quietly keeps a NaN value, so a NaN width would reach the renderer intact and draw a toast of no size at all. Every non-finite value takes the field's default and says so in the log.
  **The spec's first open question is answered here:** a pill is its own shape, not radius 40 under another name. Its radius is half its height, so it stays a pill as the toast grows; a fixed 40 stops being half the height the moment it does. `a_pill_stays_a_pill_at_any_height_and_a_rectangle_stays_square` is the test, and `docs/specs/toast.md` §12 now records the answer.

### FEAT-0018 · The toast viewport
- **State:** In-Progress · **Spec:** docs/specs/toast.md §1, §7 · **Phase:** 2
- **Branch:** feat/phase-2-toast
- **Depends on:** FEAT-0017
- **Verified:** the gate, plus the real window's extended style read back from Win32 while a toast was on screen
- [x] A borderless, transparent, always-on-top viewport that **never steals focus** and appears in neither the taskbar nor Alt+Tab
- [x] Opening and closing one on demand, with more than one alive at once — a deferred viewport per toast, keyed on the toast's own id so it keeps one window rather than getting a new one each frame
- [x] It is not a Windows Action Center notification: the app paints its own
- [x] Test: the viewport builder carries the flags the spec names, so a later edit that drops one fails rather than being noticed by a user. Plus the queue, the hover pause and dismissal, which are the behaviours around it
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 (139 tests) · deny check · --self-test
- [ ] App: a toast appears over a full-screen window, takes no focus from what is being typed into, and is absent from Alt+Tab. **Two of the three are done:** the window's extended style was read back from Win32 with a toast on screen — `WS_EX_TOPMOST` set, `WS_EX_TOOLWINDOW` set, `WS_EX_APPWINDOW` clear, `WS_EX_NOACTIVATE` set, and the foreground window was not the toast. **Over a full-screen window** needs someone to put one there, so it stays open
- [x] Docs: spec · CHANGELOG · GOTCHAS
- **Notes:** eframe's second viewport is the obvious route; whether an always-on-top no-focus window is reachable through `ViewportBuilder` alone or needs a Win32 call in `ot-platform` is the thing to establish first, and the answer belongs in the spec.
  **Answered: `ViewportBuilder` gets most of the way and not all of it.** Measured on the built window — borderless, transparent, topmost and unfocused all came out right, but `WS_EX_TOOLWINDOW` was **clear** and `WS_EX_APPWINDOW` was **set**, which is a toast in Alt+Tab and a taskbar button. `with_taskbar(false)` removes the taskbar *button* through the shell's taskbar list; Alt+Tab membership is decided by the extended style, so the style is what has to change. `ot_platform::toast_window::make_unobtrusive` sets it on the real window, found by title within this process. The spec's §1 now says so rather than leaving the next person to find it with Alt+Tab.
  **The title is the app's own name and never changes with the language**, which is what makes that lookup safe: a toast window is found by a string that a language switch cannot move.
  **Hovering pauses the countdown** (§6), which means a paused toast is one whose clock is not running rather than one running slowly — `elapsed()` adds nothing while the pointer is over it.
  **The surplus queues rather than being dropped**, and a queued toast starts its countdown when it *appears*, not when it was pushed. Otherwise a toast that waited behind four others would arrive already expired and flash past unread.

### FEAT-0019 · Shape, radius, colour, opacity, border and shadow
- **State:** In-Progress · **Spec:** docs/specs/toast.md §4 · **Phase:** 2
- **Branch:** feat/phase-2-toast
- **Depends on:** FEAT-0018
- **Verified:** the gate, plus every shape photographed at three UI scales in Malay
- [x] Rectangle, rounded rectangle and pill, each drawn from the `ToastSpec` alone
- [x] Corner radius 0–40, solid and gradient backgrounds, opacity 40–100%, border 0–4 px, shadow off / soft / strong
- [x] Nothing hard-codes a colour: the palette comes from the theme and the spec's accent overrides exactly what the template says it does
- [ ] Test: `egui_kittest` snapshots per shape, in both languages, with the longest Malay strings — **FEAT-0024's job**, which is the item that brings `egui_kittest` in. What this item has instead is the arithmetic: opacity, the content rect per shape, and the shadow settings
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 (145 tests) · deny check · --self-test
- [ ] App: every shape at 100%, 150% and 200% DPI, in both languages, with no overflow. **Done for Malay, which is the language that decides a layout** — all three shapes at all three scales, nothing overflowing, the longest label fitting, the accent scaling with the rest. Two gaps: English has only been seen at 100%, and the scales come from `--ui-scale`, which scales the window *and* its content exactly as display scaling does but is not literally the Windows setting. Changing that setting is the owner's, and it is what closes this box
- [x] Docs: spec · CHANGELOG · GOTCHAS. **README screenshot waits** for FEAT-0020's header and body: a screenshot of a toast showing only its title would be out of date within the phase
- **Notes:** Settles the spec's first open question — whether pill earns its place beside rounded rectangle, or is radius 40 under another name. FEAT-0017 answered it in the model; this item is where it shows: a pill stays a pill at 120 px and at 240 px, which a fixed radius does not.
  **A pill insets its content further than a rectangle**, because its ends are round and text inset by the same padding would have its first glyph clipped by the curve.
  **The accent is a bar down the leading edge** rather than a repaint of the whole toast: it is the one place a category's colour shows without taking the rest of the palette with it.
  **A gradient is a mesh over a rounded fill.** egui has no rounded gradient fill, so the shape is filled with the end colour first and the mesh paints the body inset by the corner radius — anything the mesh does not reach keeps the fill, which is what makes the corners survive.
  **`Color32` stores premultiplied alpha**, so reading a channel back after `from_rgba_unmultiplied` gives the premultiplied value and not the one that went in — 0x3b at 40% reads as 24. The test compares against the colour egui itself would build rather than against the channels; the first version of it failed for this reason and the behaviour was right all along.

### FEAT-0020 · Header, text and the ellipsis rules
- **State:** In-Progress · **Spec:** docs/specs/toast.md §4 · **Phase:** 2
- **Branch:** feat/phase-2-toast
- **Depends on:** FEAT-0019
- **Verified:** the gate, plus both languages photographed with a body long enough to wrap
- [x] Header: icon, title, app name, timestamp, each toggleable, the timestamp through `ot_i18n`'s one formatter — **formatted by the caller**, because a spec holds text and never keys, and `ot-i18n` owns the only thing that may turn a time into text. Visible in the captures: English reads `9:37 pm`, Malay `21:37`
- [x] Body text with bold and italic. **As a whole, not per segment:** a markup language inside a notification body is a thing to add when something asks for it
- [x] Boxes size to content in both languages; a long word wraps or truncates with an ellipsis, and **nothing overflows the toast**
- [x] An ellipsis is for user text that can be any length, never for the app's own labels while there is room — the app name and the time are laid out first and get their room; the title takes what is left and is the thing that truncates
- [x] Test: the longest Malay strings in every slot; a single unbroken 200-character word; the pseudo-locale (about 40% longer) catching what Malay does not
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 (151 tests) · deny check · --self-test
- [ ] App: a long Malay title, a long body and a long app name together, at each DPI. **Both languages are done at 100%** with a body long enough to wrap twice; the three-scale sweep was run for FEAT-0019's frame and is worth repeating once the content is final rather than twice
- [x] Docs: spec · CHANGELOG · GOTCHAS
- **Notes:** Malay runs up to 30% wider than English, so the Malay layout is the one that decides the design, not the English one.
  **`break_anywhere` is not the answer to a long word.** It sounds like it — a 200-character word has nowhere to wrap — but it applies to *everything*, so ordinary prose came out broken mid-word: "with your curren / t settings". egui already breaks a word that cannot fit a line of its own, so the flag is off and both cases are covered.
  **The first version of these tests proved nothing.** `FontDefinitions::default()` is empty in this build, because `default_fonts` is off (ADR-014) — so every glyph measured zero wide, nothing ever wrapped, and the assertions passed on text that had not been laid out. The test context now installs the machine's own UI font, and `the_test_context_has_a_font_that_measures_something` is there so the same mistake cannot come back quietly.

### FEAT-0021 · Images on a toast
- **State:** In-Progress · **Spec:** docs/specs/toast.md §4, docs/06-security-and-privacy.md · **Phase:** 2
- **Branch:** feat/phase-2-toast
- **Depends on:** FEAT-0020
- **Verified:** the gate — nine tests over the import path, one per rule it enforces
- [x] The import path in the order security.md fixes it: size limit **before reading**, format by **magic bytes** not extension, canonicalized source and destination, copy in under a content-hashed name, bounded decode at 4096 × 4096
- [x] A thumbnail cached under the data folder and treated as derived data, never as the source of truth
- [x] Decoding and resizing happen off the UI thread; the result arrives through a bounded channel drained at frame start. The Windows Open dialog runs on a worker thread too, because `IFileOpenDialog::Show` runs its own modal loop on whatever thread calls it; the import follows on the same thread. Checked: the settings window kept drawing frames while the dialog was open
- [x] An oversize, corrupt or mislabelled file is rejected with a localized message naming the reason — it must not hang or exhaust memory
- [x] Test: a `.png` that is not a PNG is refused; a 6 MB file is refused before it is read; a traversal path cannot land outside `OmniToasterData/assets/`; a decode bomb is bounded
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 (174 tests) · deny check · --self-test
- [ ] App: import a real image, see it on a toast, and see a rejected one explain itself in both languages. **Two of three done:** a picture chosen through the dialog was imported, decoded and drawn — as the header icon and as the body image, separately — on the preview and on a real toast. The refusal message has not been seen on screen: the scripted dialog proved too unreliable to deliver a chosen file, and the rules are proven by test instead (a fake PNG, an oversize file, a decode bomb, a traversal, a RIFF that is not a WebP)
- [ ] Docs: spec · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** The first feature in the project that takes a file from outside. Every step above is a requirement from `docs/06-security-and-privacy.md`, not a suggestion, and the decoder choice has to keep the static build and `cargo deny` clean.
  **The decoder is `image` with default features off and only `png`, `jpeg` and `webp` on** (the owner's call, 2026-11-09). Pure Rust, so `+crt-static` and the portable exe are unaffected; MIT/Apache-2.0, so `deny.toml` needed nothing; and the exe carries no decoder for a format the app refuses anyway, which makes the attack surface the three formats we validate rather than the twenty the crate can read. `sha2` comes with it for the content-hashed names the schema already expects.
  **The bomb is refused on its header, not its pixels.** A PNG claiming 50000 × 50000 is a few dozen bytes on disk and 10 GB decoded; `into_dimensions` reads the header alone, so the refusal costs nothing and nothing ever allocates for it. The decode check runs *before* the copy, so a bomb never reaches the data folder at all.
  **"RIFF" alone is not a WebP** — a RIFF wave file starts with the same four bytes, so the payload tag at offset 8 is what settles it. Sniffing the container alone would accept an audio file as a picture.
  **The keys are `error-import-*`, not `import-error-*`.** The project's own key-prefix test refused the first spelling: `import-` is not a documented prefix and an import failure is an error, so it belongs under the prefix that already exists. The test said so before a human had to.

### FEAT-0022 · Placement, DPI, monitors and stacking
- **State:** In-Progress · **Spec:** docs/specs/toast.md §7 · **Phase:** 2
- **Branch:** feat/phase-2-toast
- **Depends on:** FEAT-0019
- **Verified:** the gate, plus three live toasts whose window rects were read back from Win32 and matched the arithmetic to the pixel
- [x] A chosen screen, worked out from **that monitor's work area** and never from the primary monitor's rect
- [x] **The nine presets** of `docs/specs/toast.md` §7.1 — four corners, four edges and the centre — with Bottom Right the default, a 16 px DPI-scaled margin against an edge, and no margin on an axis the preset centres on
- [ ] **A custom point** (§7.2): set by dragging the preview onto a map of the chosen monitor *and* by typing X and Y in percent, the two driving one value. **The model and the arithmetic are done**; the two ways of setting it are controls, so they land with the Appearance pane in FEAT-0023
- [x] A custom point is stored as a **fraction of the work area**, not as pixels, so it means the same place after a resolution change, a taskbar move, or a move to another monitor
- [x] The fraction addresses the toast's **anchor corner** — the corner nearest the point — so a toast placed near an edge grows inward rather than off the screen
- [x] A custom point is clamped so the whole toast stays inside the work area, clamped **at paint time** against the monitor in use rather than once at save time
- [x] **One custom point, not one per monitor** (the owner's call, 2026-11-09): it is a fraction, so it already means the same place on any screen, and moving the toast to another monitor keeps it in the same relative spot there
- [x] Never covers the taskbar; stacks toward the centre of the work area, per §7.3's table
- [x] When the chosen monitor disappears, toasts move to the primary one and say so in the log
- [ ] 1–5 visible with a configurable gap; the surplus queues and appears as earlier ones retire
- [ ] Correct from 100% to 250% DPI, including a DPI change mid-session and a monitor unplugged while a toast is visible. **Not checked:** this machine has one monitor at 100% (`\\.\DISPLAY2`, work area 1920×984 at scale 1.0), so a DPI change and an unplug are the owner's to make. The work area is re-read every frame rather than cached, which is what makes an unplug survivable
- [x] Test: the placement arithmetic against a table of work areas, positions and DPI scales, including a monitor whose origin is negative — the second-monitor case that catches a primary-rect assumption
- [x] Test: each of the nine presets lands against the edges it names and is centred on the axes it centres on — `Top` centred horizontally against the top, `Left` centred vertically against the left, `Centre` centred on both
- [x] Test: a custom fraction resolves to the same relative place on a 1920×1080 and a 3840×2160 work area, and a fraction of 1.0 still leaves the whole toast inside it
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: each of the nine presets at 100%, 150% and 200%; a custom point placed by dragging and by typing; a stack of five; **a second monitor and a 250% display are checks this machine cannot make** — it has one monitor at 100%, which is now a recorded fact rather than a guess. What was checked: three live toasts at the default Bottom Right, their window rects read back from Win32 — right edge 1904 (1920 − 16), bottom 968 (984 − 16), stacking upward 128 apart (120 + 8), none over the taskbar. Picking the other presets needs the Appearance pane (FEAT-0023)
- [ ] Docs: spec · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** All monitor and work-area queries are Win32, so they live in `ot-platform` behind a capability-named interface, with the arithmetic in `ot-toast` where it can be tested without a screen.
  **Nine presets — four corners, four edges and the centre — and a custom point are the owner's requirement, asked for on 2026-11-09**, and `docs/specs/toast.md` §7 was rewritten to carry it before any of it was built. The spec previously said only "a chosen screen and corner".
  **The custom point is a fraction, not a pixel pair.** `(1850, 1010)` is wrong the moment the resolution changes, the taskbar moves or the toast appears on another monitor; `(0.96, 0.94)` means the same place on all of them. The fraction addresses the anchor *corner* rather than the toast's top-left, which is what keeps a toast placed near the bottom-right growing up and left instead of off the screen.
  **Two ways in, one value out.** Dragging the preview and typing a percentage set the same fraction, so neither can disagree with the other — the same reason the preview and the real toast share one `ToastSpec`.

### FEAT-0023 · The Settings ▸ Appearance preview
- **State:** In-Progress · **Spec:** docs/specs/toast.md §3, docs/04-ui-design.md · **Phase:** 2
- **Branch:** feat/phase-2-toast
- **Depends on:** FEAT-0019, FEAT-0020
- **Verified:** the gate, plus the preview and a real toast photographed side by side at the same non-default style
- [x] An Appearance pane: shape, radius, width, opacity, border, shadow, background and accent colours, header toggles, and the position picker with its map — each the control `docs/04-ui-design.md` names for its value. **Height** has no control yet: toasts size to their content, and a fixed height is the one style option nobody has asked for
- [x] A live preview drawn by **the same renderer, from the same `ToastSpec`**, so it cannot diverge from the real thing — `Prefs::spec` is the one function both call, and the preview uses `render::frame` and `content::draw` exactly as a toast window does
- [x] A style option the preview cannot show is a design bug, not a preview limitation — there is none: every control on the pane changes the preview
- [x] Test: the pane's strings all resolve in both catalogues; building a spec from the pane and from a template produce the same spec for the same inputs (`the_preview_and_a_real_toast_are_the_same_spec_for_the_same_choices`)
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 (180 tests) · deny check · --self-test
- [x] App: change a style, look at the preview, fire the real toast, compare them side by side. **Done at width 478 and 77% opacity** — same size, same layout, same text positions, same header, mark and accent bar. The only difference is what shows *through* them at 77%: the window behind the preview and the desktop behind the toast, which is correct for a translucent toast
- [ ] Docs: spec · CHANGELOG · CHECKLIST done. **README screenshot waits** for the image picker, so the pane is photographed once, complete
- **Corrected by the owner, 2026-11-10: icon is icon, image is image, and both belong to the notification.** The first version of this pane had one global "Image" setting drawn in the header's icon slot — wrong twice over. `Content` now carries an **icon** (header, beside the title, the app's own icon when absent), an **image** (inside the content, beside the body, with an optional border), and an **accent** that overrides the default; every notification may carry different ones. Appearance keeps only global style, and a **Test Toast Content** section — in memory, never saved — lets the preview and the test toast show real pictures. Checked on screen: the icon and image in their places on the preview and on a real toast, the image kept wide rather than squashed, the notification's green accent replacing the default blue with nothing written to `settings.toml`.
  **The image border is drawn in the theme's dim-text colour**, not the toast's border colour. The border colour was tried and measured: `#3a3a42` on `#26262c` was findable at 300% zoom and not at 100%, so the switch changed nothing anyone would see. The dim-colour version was not re-photographed, because the scripted dialog could not reliably load a picture a second time.
- **Still to come on this item:** a look at the pane in Malay and at the narrow minimum
- **Notes:** The preview is the reason `ToastSpec` exists. Building one from a reminder plus its template is one function used by both the preview and the scheduler, so there is one path and not two.
  **`Prefs` lives in `ot-toast`, and `settings.toml` carries it as a raw `[toast]` table.** `ot-core` depends on nothing in the workspace, so it cannot name `Style` or `Behaviour`; it holds the table and `ot_toast::Prefs` reads it. A table that does not fit falls back to the defaults with a line in the log, so a hand-edit cannot stop toasts appearing.
  **Saved on release, not on change.** A slider drag changes the prefs every frame; they are written when the pointer comes up, or a sweep across the range would rewrite the file a hundred times.
  **The test-toast bell drew as `?` at first.** Segoe UI has no bell, no clock and no pictographs; the tray shows them only because Windows falls back to Segoe UI Symbol behind the scenes, and egui falls back to nothing it was not given. `ot_platform::ui_font::symbols` loads that same fallback, last in both families, so it only answers for glyphs the UI font lacks.
  **A false alarm worth recording:** a run saved `position = "centre"` that nobody chose. It did not reproduce under a controlled sequence, and a second run showed why — an automation click meant for empty space landed on the Opacity slider because the pane was scrolled. The first was the same thing on the map's centre dot. My clicks, not the app.

### FEAT-0024 · Snapshot and length evidence
- **State:** Pending · **Spec:** docs/specs/toast.md §11 · **Phase:** 2
- **Branch:** feat/phase-2-toast
- **Depends on:** FEAT-0019, FEAT-0020, FEAT-0023
- [ ] `egui_kittest` snapshots for every shape, in both languages, with the longest Malay strings
- [ ] A pseudo-locale (about 40% longer than English) built and used to catch layout breakage Malay does not
- [ ] The snapshots run in CI and fail on a visual change, so a layout regression is caught by the gate rather than by a user
- [ ] Test: that is what this item is
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: not applicable — this item is the evidence for the others
- [ ] Docs: spec · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** Synthetic clicks never form double-clicks, so any double-click path stays an in-app check. Snapshots are a floor, not a substitute for looking: the phase's exit criteria say *in the running app*.

### FEAT-0025 · The image library
- **State:** Pending · **Spec:** docs/specs/toast.md §8.1 · **Phase:** 2
- **Branch:** feat/phase-2-toast
- **Depends on:** FEAT-0021
- **Source:** the owner, 2026-11-10 — upload pictures once and use them in notification templates: a meeting picture, a mosque, an alarm, the hourly announcement
- [ ] A library of uploaded pictures: upload through FEAT-0021's import path unchanged, list with thumbnails, a label the user gives each one, remove
- [ ] The `asset` table records each upload with its label, so the library is a query rather than a folder listing — `docs/05-data-model-and-storage.md` now carries the `label` column, which needs a migration
- [ ] One list for both uses: any picture may be an icon in one template and an image in another
- [ ] Removing a picture still in use asks first and names the templates that use it; they then fall back to the app icon or no image rather than pointing at a missing file
- [ ] The Test Toast Content pickers choose from the library, with "Upload a New Picture" at the end of the list
- [ ] Test: two uploads of the same file are one library entry; a label round-trips; removal of a used picture is refused without confirmation; the list survives a restart
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: upload three pictures, label them, use two in the test toast, remove one in use and see the warning — in both languages
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST
- **Notes:** The asset store was built content-addressed for exactly this: the same picture uploaded twice is already one file. What this adds is the *list* — names, thumbnails, and knowing what uses what.
  **The template pickers themselves arrive with each template's phase** — reminder types in Phase 4, the Azan per prayer in Phase 5, the hourly announcement in Phase 7. This item builds the library they all choose from, and the Test Toast Content pickers are its first user.

## v0.1.0 — Planned

Phase 1, Shell. The Phase 1 exit criteria from `docs/07-roadmap.md`: the tray
works, the theme switches, autostart survives a reboot, and a language switch
updates the tray and the settings window in under one second.

Ordered by dependency: FEAT-0011 to FEAT-0013 are self-contained `ot-platform`
work and come first; FEAT-0014 needs a theme to render; FEAT-0015 needs a
window to open; FEAT-0016 needs both so it has something to re-render.

### FEAT-0011 · Single instance
- **State:** Completed · **Spec:** docs/specs/tray-and-settings.md §5 · **Phase:** 1
- **Branch:** feat/phase-1-shell
- **Verified:** the gate, plus a cross-process test that runs two real executables
- [x] `ot_platform::single_instance` taking a named mutex, with a distinct name per role so the supervisor and its worker cannot collide
- [x] A second launch does not start a second tray icon or a second scheduler; it exits 0 with a localized notice. **Signalling the first instance is deferred to FEAT-0014**, which is where a window exists to focus
- [x] The guard releases on exit, including an abnormal one, so a killed process does not lock the app out
- [x] Test: taking the guard twice in one process fails the second time; the name differs per role; dropping it lets a later take succeed
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [x] App: two overlapping processes — the second bows out and the database is left alone
- [x] Docs: spec · CHANGELOG · CHECKLIST
- **Notes:** Windows-only code, so it lives in `ot-platform` behind a capability-named interface with a non-Windows stub (`docs/03-architecture.md`). Focusing the existing window is FEAT-0014's job; this item only has to refuse the second start.
  **A named mutex, not a lock file**, because the kernel releases it when the process ends however it ends — a lock file left by a killed process would lock the app out of its own slot until someone deleted it. The names are `Local\`-scoped so two users logged into the same machine each get their own instance; `Global\` would let one user's copy lock another out.
  The unit tests can only prove the guard within one process, which is not the thing that matters. `a_second_instance_bows_out_instead_of_starting` in `tests/cli.rs` runs two real executables overlapping, asserts the second exits **0** with the notice — not a failure code, since a double-click on a running app is not an error — and asserts it never reached the database. It then starts a third after the first exits, proving the guard is released.
  That test needed a debug-only `--hold-ms`, because until FEAT-0014 there is no window keeping the process alive, so the first copy would exit before a second could start. It is compiled out of release and goes away when the window lands.

### FEAT-0012 · Start with Windows
- **State:** In-Progress · **Spec:** docs/specs/tray-and-settings.md §7 · **Phase:** 1
- **Branch:** feat/phase-1-shell
- **Verified:** the gate, a real-registry round trip, and the refresh path driven end to end
- [x] `ot_platform::autostart` writing a single HKCU `Run` value, only when the user enables it
- [x] Refreshed on every launch, so a moved or renamed portable folder still starts
- [x] Disabling removes the value; the toggle **reads the registry** rather than trusting `settings.toml`, so a value removed by other means shows as off
- [x] Test: enable writes the value with the current exe path; a moved exe rewrites it; disable removes it; reading an absent value reports off rather than erroring
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: enable it, reboot, confirm the app starts; move the folder, launch, confirm the value updated. **The refresh half is done** — a stale path was seeded under the real value name, the app rewrote it to this folder, and the value was removed afterwards. The **enable and reboot** half needs the settings window's toggle (FEAT-0014) and a reboot, so it stays open
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS — README waits until the toggle is visible
- **Notes:** The **one** registry write the app is allowed (ADR-004), so the test must prove nothing else is written. Tests touch the real HKCU hive, so they use a value name scoped to the test run and remove it afterwards — never the production `OmniToaster` value, or running the suite would disable a user's autostart.

### FEAT-0013 · The three themes and system detection
- **State:** In-Progress · **Spec:** docs/04-ui-design.md · **Phase:** 1
- **Branch:** feat/phase-1-shell
- **Verified:** the gate, a cross-check of the reading against the live registry, and the System path driven through every reading
- [x] `ot_platform::system_theme` reading `AppsUseLightTheme`, rechecked every 5 seconds so a Windows change is followed without a restart
- [x] `ot_ui::theme` with the semantic tokens from `docs/04-ui-design.md` — `surface`, `surface_raised`, `text`, `text_dim`, `accent`, `border`, `danger` — for Dark and Light
- [x] Dark is the default; System follows Windows; nothing hard-codes a colour
- [x] Hover changes fill or border only — never size, position or glyph scale
- [x] Test: each mode resolves to the expected palette; System follows a change in the detected value; every token is defined in both palettes
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: switch all three modes and change the Windows setting with System selected. **The first half is done** — once FEAT-0014 gave it a window, Dark, Light and System were each selected in the running app and each drew correctly, with the stored value matching the choice every time (`dark`, `light`, `system`). The **second half** — changing the Windows setting while System is selected and watching the app follow within five seconds — stays open: it means writing to the user's own `Personalize` key, which is their setting to change, not this session's
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS — README waits until a themed screen exists to describe
- **Notes:** The recheck is a poll because Windows offers no cheap notification for this worth a message loop here. 5 seconds is the spec's number.
  **`AppsUseLightTheme`, not `SystemUsesLightTheme`**: the second is the taskbar and Start menu, the two can differ, and following the wrong one would put OmniToaster in light mode while every other app stayed dark. The name reads backwards from how it is spoken about — 1 means light — so the test cross-checks the reading against `reg query` rather than only asserting it does not panic.
  A reading Windows declines (an older build, a profile that never set it) is not an error: it falls back to Dark, the app's default. Guessing light would flash a white window at someone who chose dark everywhere.
  `poll` is split from a pure `resolve(mode, reading)` plus a private `adopt(reading)`, so the test can drive System through Light, Dark and absent without changing the machine's own Windows setting — a test that only read the live value could not prove the app follows a change.
  `widget.expansion = 0.0` on all five widget states: egui grows widgets under the pointer by default, which `.claude/rules/ui.md` forbids outright, and it is the kind of thing only a human notices, so it is asserted.
  Adding `egui` failed `cargo deny` twice over — `epaint_default_fonts` ships the Ubuntu font under licences not on the allow list, and `ab_glyph` reaches the unmaintained `ttf-parser`. ADR-014 turns `default_fonts` off rather than allowing a font licence the app does not need; the advisory is ignored with its reason.

### FEAT-0014 · The settings window
- **State:** In-Progress · **Spec:** docs/04-ui-design.md · **Phase:** 1
- **Branch:** feat/settings-window
- **Depends on:** FEAT-0013
- **Verified:** the gate, plus the running app driven on a scratch `--data-dir` — both languages, Dark, Light and System, at the default size and at the minimum
- [x] An eframe window: a left rail of sections, a content pane, a status line
- [x] The **General** pane only: language, theme, start with Windows, and the status line. The other panes belong to later phases and must not be stubbed in as empty tabs
- [x] Closing the window does not quit the app — closed by FEAT-0015, which gave it a tray to be reached from. Checked in the running app: the window was closed and the process was still there afterwards, drawing frames
- [x] Every string through Fluent in both catalogues; Title Case for labels, sentence case for hints
- [x] Test: the General pane's strings all resolve; no literal reaches a widget — `the_general_pane_names_only_keys_that_exist_in_both_catalogues` walks every key the window can ask for, in both catalogues
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 (110 tests) · deny check · --self-test
- [x] App: `--data-dir ./tmp-data` — the window opens; the language and theme pickers both work and persist; Dark, Light and System were each selected and each drew correctly; the stored language is honoured at the next start; the layout holds at 880×560 and at the 560×400 minimum, in English and in Malay
- [ ] App, the part that could not be done from here: the **Start with Windows** checkbox. Clicking it writes the HKCU `Run` value, which this session's permission guard refuses as persistence. The code path is the one FEAT-0012 already proved against the real registry; the click itself needs the owner
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS · screenshots
- **Notes:** `egui_kittest` covers widget snapshots, but synthetic clicks never form double-clicks, so any double-click path is exercised in the running app. The window is where FEAT-0011's "focus the existing instance" lands — still FEAT-0015's job, since the tray is what raises it.
  **Three pieces had to come with it.** `ot_core::settings` reads and writes `config/settings.toml`, which did not exist yet and is the spec's source of truth for configuration (ADR-007); `ot_platform::ui_font` loads Segoe UI and Consolas out of the Windows font folder, because ADR-014 turned egui's bundled fonts off and without a font nothing renders at all; and `ot_ui::layout` holds the measurements and the one row helper, so the rules in `.claude/rules/ui.md` are implemented once rather than per pane.
  **The language choice is honoured at startup**, which needed the stored value to be read *before* `ot_i18n::init` — the data folder's own failure messages are localized, and the choice lives in that folder. `ot_core::settings::stored_language` reads it without creating or validating anything, and treats every failure as "no choice recorded". Changing the language still needs a restart (the hint says so); the live switch is FEAT-0016.
  **`--worker` now opens a window**, so `worker_is_accepted_but_does_nothing_yet` was renamed and given `--hold-ms 0`. No test may open the window: CI runs as a service in session 0, where no window can open at all. That is also why `--hold-ms` stays.
  **That fix was incomplete, and CI found it.** Run 39 hung on `a_second_instance_bows_out_instead_of_starting` and was cancelled — its *third* instance, the one that proves the guard is released after the first exits, had no flag because it had never needed one. Every `Command::new(exe())` in `tests/cli.rs` now carries `--hold-ms`. The local gate had reported 110 passing beforehand, and it was wrong: a copy of the app was still running, so cargo could not replace `target/debug/OmniToaster.exe` ("Access is denied") and the integration tests ran the previous build. Both are now in `docs/GOTCHAS.md`; the suite is run under a `timeout` so a hang is an exit code rather than a number.
  **Known gap, not fixed here:** in Light mode the window's **title bar stays dark**, because that is Windows chrome and needs `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE)` against the window handle. It is visible in `docs/screenshots/` if you look. Filed as CR-0002 rather than bolted on, since it needs the HWND plumbed out of eframe.

### CR-0002 · The title bar ignores the theme
- **State:** Pending · **Spec:** docs/04-ui-design.md · **Phase:** 1
- **Source:** seen while checking FEAT-0014 in the running app
- [ ] Spec first: `docs/04-ui-design.md` says what the window chrome follows
- [ ] `ot_platform` sets `DWMWA_USE_IMMERSIVE_DARK_MODE` on the window, behind a capability-named interface, taking the handle rather than finding it
- [ ] `ot-ui` calls it when the appearance resolves, including when System follows a Windows change while the window is open
- [ ] Test: the call is made for each appearance; the non-Windows path is a no-op
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: Light with a light title bar, Dark with a dark one, and System following a change in Windows while the window is open
- [ ] Docs: spec · CHANGELOG · screenshots retaken
- **Notes:** The window is light and its title bar is dark, which is the most visible thing wrong with the first screen the product has. Getting the handle means plumbing `raw-window-handle` out of eframe's `CreationContext`, which is why it is its own item rather than a line in FEAT-0014.

### FEAT-0015 · Tray icon and menu
- **State:** In-Progress · **Spec:** docs/specs/tray-and-settings.md §3, §4 · **Phase:** 1
- **Branch:** feat/tray-icon
- **Depends on:** FEAT-0014
- **Verified:** the gate, plus the running app — the icon is created, closing the window hides it and the process keeps drawing frames
- [x] `tray-icon` + `muda`, using the icon already embedded in the exe. `muda` arrives **through** `tray-icon` and is used as `tray_icon::menu`, so the two cannot drift apart; adding it separately would have pinned a second version of the same types
- [ ] Left-click opens or focuses the settings window. **Written and not clicked from here** — see the App box
- [x] Right-click menu: Open Settings · Pause All Notifications (30 min · 1 hour · until resumed) · Test Toast · Snooze Last · Quit — each with a glyph, the same glyph for the same verb, chosen in code rather than written into the locale files
- [x] The menu is **rebuilt** on a language change rather than created once — and on a pause starting or lapsing, which changes which items belong in it
- [x] Pause state and its expiry visible in the tooltip and the status line; Quit confirms while a pause is active
- [ ] A stale icon left by a killed process is removed on the next start. **Not done and not faked:** `tray-icon` removes the icon when it is dropped, which a killed process never does. Windows clears the ghost when someone next moves the pointer over it, and doing better needs a stable `GUID` identity on the `Shell_NotifyIcon` call, which this crate does not expose. Filed as CR-0003
- [x] Test: the menu's labels all resolve in both languages — every tray key is in the window's key-coverage test; the pause windows compute the right expiry, including across midnight (`a_pause_started_before_midnight_ends_the_next_day`)
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 (119 tests) · deny check · --self-test
- [ ] App: every menu item, both languages, and the icon at 100% and 200% DPI. **This is the owner's to do.** Windows 11 puts a new tray icon in the hidden-icons overflow, and that flyout cannot be reached from this session — not by clicking (the chevron has no findable position), not through UI Automation (the notification area is a XAML island that exposes nothing), and not by Win+B. What *was* checked here: the icon is created without error in both languages (a failure would show in the status line and it does not), closing the window hides it rather than quitting, and the process keeps drawing while hidden
- [x] Docs: spec · CHANGELOG · CHECKLIST · GOTCHAS. **README screenshot waits** on the menu being photographable, which is the same blocker as the App box
- **Notes:** Test Toast and Snooze Last have nothing to act on until Phases 2 and 4; they are present and disabled rather than hidden, so the menu's shape does not change under the user later. Pause suppresses toasts and announcements but **never Azan** — that is Settings ▸ Prayer's job.
  **The tray shares the window's thread.** It is created inside the eframe app, so winit's message pump delivers its clicks, and the window asks to be repainted every 120 ms because nothing wakes egui when an event lands in the tray's channel. The question that decided the design was whether a *hidden* window still gets updated: it does. Measured with a temporary counter — 18 frames in the 6 seconds after the window was closed, about 3 a second, Windows throttling it from the 8 the interval asks for. That is far inside what a menu click needs, so there is no second thread and no second message loop.
  **Labels cross the crate boundary as strings.** `ot-platform` owns Windows, not language: the caller resolves every Fluent key and hands over a `Labels`, which is also what makes a language change an ordinary rebuild rather than a special case. The same trade appears in `PauseWindow` versus `ot_core::PauseFor` — two enums naming the same three things so that `ot-platform` need not know about clocks. `the_pause_windows_map_across_the_crate_boundary_without_crossing_over` is there because swapping two of them would mute the app for the wrong length of time and nothing else would notice.

### CR-0003 · A killed worker leaves a ghost tray icon
- **State:** Pending · **Spec:** docs/specs/tray-and-settings.md §3 · **Phase:** 1
- **Source:** FEAT-0015 — the box it could not tick
- [ ] Spec first: say what identity the icon takes and what the next start does with a stale one
- [ ] Give the icon a stable `GUID` so Windows recognises the new instance as the same icon and replaces it, instead of leaving the dead one until the pointer passes over it
- [ ] Test: two runs in a row produce one icon, not two
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: kill the process, start it again, confirm one icon
- [ ] Docs: spec · CHANGELOG · GOTCHAS
- **Notes:** `tray-icon` 0.24 has no way to pass a `GUID` to `Shell_NotifyIcon`, so this is either a patch upstream or the one Win32 call made directly in `ot-platform`. Worth doing before Phase 8, when the supervisor starts restarting workers and every crash would otherwise leave another ghost.

### FEAT-0016 · Live language switch
- **State:** In-Progress · **Spec:** docs/10-i18n.md · **Phase:** 1
- **Branch:** feat/live-language
- **Depends on:** FEAT-0014, FEAT-0015
- **Verified:** the gate, plus the running app — English to Malay to System, with the switch timing itself
- [x] Changing the language re-renders the settings window and **rebuilds** the tray menu, with no restart. The window's *title* needed its own command: it is set on the viewport rather than drawn, so it kept the old language until it was told
- [x] The whole switch completes in under one second — **measured at 2.4 ms** (English to Malay) and 1.4 ms (Malay to System), catalogues and tray menu included. The switch logs its own duration, so the claim is checkable on a user's machine
- [x] `system` follows the Windows display language; an explicit choice overrides it and persists. Resolved against Windows each time it is chosen rather than once at startup, and stored as the *absence* of a language key, which is what makes "follow Windows" survive a restart
- [x] Test: the active language changes what `text()` returns, including under substitution; the setting round-trips through `settings.toml`; a second `init` is refused rather than becoming a silent way to switch
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 (120 tests) · deny check · --self-test
- [ ] App: switch English to Malay to System and watch both surfaces change, timed. **The window half is done and photographed** — every label, the hint, the status line and the title bar, in both directions. The **tray menu** half cannot be seen from this session for the same reason as FEAT-0015: Windows 11 hides a new icon in the overflow. The rebuild is called on the measured path and its labels come from the same catalogue the test checks
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** The global Fluent loader was a `OnceLock` set at startup (FEAT-0004), which a live switch cannot re-set. It is now an `RwLock<Option<Active>>` holding the loader, the language it is serving and the override folder it was built with — so a user who has overridden their Malay text does not lose it by switching away and back.
  **The loader is built before the write lock is taken.** Loading parses both catalogues; doing that under the lock would stall every string lookup in the app for the duration, and a frame that is drawing would block a switch as surely as a switch would block a frame. Readers hold the lock only long enough to clone an `Arc`.
  **A poisoned lock is recovered from, not propagated.** A panic somewhere else in the process is no reason for every label in the app to turn into its key.
  **The switching tests live in their own integration test — their own process.** A unit test that asserts English and a test that switches to Malay cannot share one, and it is the same global-state race that made a release flaky earlier in this project. For the same reason it is *one* test and not several: `cargo test` runs the tests in one file in parallel threads.

---

## v0.0.1 — In-Progress

Phase 0, Foundation. The Phase 0 exit criteria from `PROPOSAL.md` §13: the app
starts in English and in Malay, writes a log, the locale key parity check
passes, and CI is green. Nothing here is `Completed` until all of that holds.

**All four exit criteria hold as of 2026-11-08, and FEAT-0001 to FEAT-0007 are
`Completed`.** The batch is still `In-Progress` rather than `Done`, because the
batch closes only when *every* item in it is `Completed` and two are not:
FEAT-0009 (the release packaging) is `In-Progress` with the Pester tests and
`THIRD_PARTY_NOTICES.md` outstanding, and BUG-0001 is `Blocked` on the owner.
Neither is an exit criterion, so Phase 1 is not held up by them.

> **Shipped in:** app v0.1.0 and v0.2.0, 2026-11-08 — the batch spans two
> releases, which is exactly what this file's own Format section says a task
> version may do.

The items are ordered by dependency: FEAT-0001 first, then 0002 and 0003 on top
of it, then 0004, 0005 and 0006 in any order.

### FEAT-0001 · Cargo workspace skeleton
- **State:** Completed · **Spec:** docs/03-architecture.md · **Phase:** 0
- **Branch:** feat/phase-0-foundation
- **Verified:** the gate below, run 2026-11-08 on this machine, plus the built exe
- [x] Root `Cargo.toml` with `[workspace]`, `[workspace.package]` (version, edition 2024, licence MIT, repository) and `[workspace.dependencies]` for everything shared
- [x] `crates/omnitoaster` — package `omnitoaster`, `[[bin]] name = "OmniToaster"`, a `main` that prints its version and exits
- [x] The ten library crates, each compiling with a doc comment and nothing else: `ot-core`, `ot-i18n`, `ot-store`, `ot-platform`, `ot-ui`, `ot-toast`, `ot-sched`, `ot-prayer`, `ot-weather`, `ot-media`
- [x] `.cargo/config.toml` with `+crt-static` so the MSVC build stays portable
- [x] `rust-toolchain.toml` pinning the stable channel and the MSVC target
- [x] `deny.toml` so `cargo deny check` has something to read
- [x] `.gitignore` covering `target/`, `tmp-data/` and `OmniToasterData/`
- [x] Test: `cargo metadata` lists all eleven members; a test in `omnitoaster` asserts `env!("CARGO_PKG_VERSION")` is non-empty
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check
- [x] App: `cargo build --release -p omnitoaster` produces `target/release/OmniToaster.exe`, and running it prints the version
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** `ot-platform` is the only crate allowed to contain Windows-only code, so every other crate's clippy must pass on Linux too. No tokio — `PROPOSAL.md` §5 and the architecture spec settle this. Two deliberate deviations from the boxes as written, both recorded in `docs/03-architecture.md`: `[workspace.dependencies]` is present but **empty**, because populating it would have meant inventing twenty version numbers that `--locked` would then enforce — each later item adds its own with `cargo add`, so the versions come from the registry; and `crates/omnitoaster` declares **no** path dependencies yet for the same reason an unused dependency is dead weight, with `cargo build --workspace` still compiling every member so the gate covers them. `.gitignore` landed earlier, in the realignment batch. The release profile sets `panic = "abort"`, which is a design choice the supervisor depends on and is written up in the architecture spec. `+crt-static` was confirmed, not assumed: the built exe imports only `KERNEL32.dll` and `ntdll.dll`, with no `VCRUNTIME`, `msvcp` or `api-ms-win-crt` entries. The gate's `--self-test` step is not in this item's box because the flag arrives with FEAT-0006.

### FEAT-0002 · Portable path resolution and the writability check
- **State:** Completed · **Spec:** docs/05-data-model-and-storage.md · **Phase:** 0
- **Branch:** feat/phase-0-paths
- **Depends on:** FEAT-0001
- **Verified:** the gate below plus both halves of the App box, run 2026-11-08
- [x] `ot-core::paths` resolves the data root: `--data-dir <dir>` when given, otherwise `OmniToasterData/` beside the exe — never `%APPDATA%`, `%TEMP%` or the registry
- [x] Create the subtree on first run: `config/`, `data/`, `locales/`, `assets/{images,audio/azan,audio/chimes,icons}`, `logs/crash/`, `cache/`
- [x] Writability check at startup: write and delete a probe file in the root; on failure return a typed error that the caller can render bilingually, and exit without touching existing data
- [x] Path canonicalization helper that refuses any path resolving outside the data root
- [x] Test: a resolved root under a temp dir gets the whole subtree; a read-only root produces the writability error and creates nothing; `../` and an absolute path outside the root are both refused
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check — 17 tests pass. `--self-test` is **not** in this run because the flag does not exist until FEAT-0006
- [x] App: `--data-dir ./tmp-data` — the folder appears with every subdirectory; point `--data-dir` at a read-only folder and the app reports it and exits instead of panicking
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** The read-only case did **not** need the escape hatch this item anticipated: `icacls /inheritance:d` followed by a `(OI)(CI)(W,AD)` deny produced a genuinely unwritable folder, PowerShell confirmed it refused a write, and the app reported `the data folder … is not writable: Access is denied. (os error 5)`, printed that nothing was changed, exited 1 and — with the ACL restored afterwards to check — had created nothing inside it. The folder and its ACL were cleaned up. `PathError` is a `thiserror` enum with four variants kept separate because their user-facing advice differs; FEAT-0004 turns the binary's plain-English rendering into `error-` Fluent keys. Two things found while building it, both now in `docs/GOTCHAS.md` under "Storage and Migrations": `fs::canonicalize` returns a `\\?\` extended-length path on Windows that leaks into messages and subprocess arguments, so it is stripped unless the path is at or past 260 characters where the prefix is load-bearing; and writability must be proved by writing, since a permission check still says yes on a full disk or a read-only mount. `join()` also re-checks with `canonicalize` where the target exists, so a symlink inside the root that points outside it is refused too. **Scope note:** minimal `--data-dir` parsing landed in `crates/omnitoaster/src/main.rs` here rather than in FEAT-0006, because this item's App box cannot be exercised without it; FEAT-0006 still owns `--self-test`, `--worker` and the bilingual `--help`.

### FEAT-0003 · Logging and the panic hook
- **State:** Completed · **Spec:** docs/03-architecture.md · **Phase:** 0
- **Branch:** feat/phase-0-paths
- **Depends on:** FEAT-0002
- **Verified:** the gate below plus both halves of the App box, run 2026-11-08
- [x] `tracing` + `tracing-appender` writing `logs/app-YYYY-MM-DD.log`, daily rotation, retention configurable with a 14-day default
- [x] A log level from settings, overridable by `--log-level` for a session — the `--log-level` half is done; reading the default from `settings.toml` waits for the settings file itself, which no item has created yet
- [x] `std::panic::set_hook` writing `logs/crash/<timestamp>.txt` with the app version, the OS build, the panic message, the location and the last 200 log lines — and no personal data
- [x] Retention sweep at startup deletes log files past the retention window
- [x] Test: a span and an event reach the log file; the panic hook writes a crash file whose contents contain the panic message and no absolute user path; the sweep deletes an aged file and keeps a fresh one
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check — 27 tests pass. `--self-test` is not in this run because the flag does not exist until FEAT-0006
- [x] App: `--data-dir ./tmp-data` — the log file appears and grows; a deliberate panic behind a debug-only trigger leaves a crash file
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** The supervisor that restarts after a crash is Phase 8; this item only captures the crash. Verified in the app: `app.2026-11-08` appeared with the startup and `startup complete` lines, and `--panic-now` (compiled only under `debug_assertions`) produced `logs/crash/2026-11-08T12-37-36.553.txt` carrying the version, `os: windows x86_64`, the panic message, the source location, and the recent log line with the data folder **redacted to `<data>`** — grepped afterwards for an absolute user path and found none. Three decisions worth knowing. `RecentLines` is a bounded in-memory ring of the last 200 formatted lines, so a crash report has context without re-reading the log while the process is already failing; its mutex recovers from poisoning rather than panicking again inside the logging layer, since a panic is exactly when it is needed. `HostInfo` is passed in rather than discovered, because reading the Windows build number is platform code and `ot-core` depends on nothing — for now it reports `windows x86_64`, and Phase 1 can supply the real build through `ot-platform`. And `LevelFilter` was chosen over `EnvFilter`: per-module levels would be nice but `EnvFilter` pulls regex machinery into a binary that ships as one portable exe, and nothing asks for them. **Found a real bug while doing this** — see the paths note below.

### FEAT-0004 · Fluent setup, both locale files, and locale detection
- **State:** Completed · **Spec:** docs/10-i18n.md · **Phase:** 0
- **Branch:** feat/phase-0-paths
- **Depends on:** FEAT-0001
- **Verified:** the gate below plus the App box in both languages, run 2026-11-08
- [x] `locales/en-US/omni_toaster.ftl` and `locales/ms-MY/omni_toaster.ftl`, embedded with `rust-embed`, holding the Phase 0 keys: the startup and writability errors, the log-path notice, the language names, and `format-` keys
- [x] `ot-i18n` builds the bundles with `i18n-embed`; a missing Malay key falls back to English and logs a warning. **The `fl!()` path is deliberately not used** — see Notes and the ADR-006 amendment
- [x] Locale detection: `GetUserDefaultLocaleName` in `ot-platform`, `ms-*` → `ms-MY`, everything else → `en-US`; an explicit `--lang` overrides it
- [x] User locale overrides loaded from `<data>/locales/` and validated; a broken override is ignored with a logged warning and the embedded text is used
- [x] The single `format_datetime(dt, prefs)` formatter — the only function in the workspace that formats a date, time or number
- [x] Test: key parity — the `en-US` and `ms-MY` key sets are identical, and the test fails naming the keys that are missing from each side; both bundles parse with no syntax errors; a missing Malay key falls back to English; `format_datetime` produces the expected string in both languages for 12-hour and 24-hour
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check — 44 tests pass. `--self-test` is not in this run because the flag does not exist until FEAT-0006. **This box was first ticked on a run that passed by luck:** two tests raced on the global Fluent loader and one unwrapped `init`, so the suite's result depended on thread scheduling. It went green locally, then failed the v0.1.0 release job with `AlreadyInitialised`. Re-ticked after the fix, proven with 20 consecutive parallel runs, a `--test-threads=1` run, and each racing test in isolation
- [x] App: `--data-dir ./tmp-data` in both languages, including the unwritable-folder error path
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** **Edit both `.ftl` files with the Edit tool, never a heredoc** — an unquoted heredoc eats `{ $placeholders }` and backticks and silently drops the message. 16 tests in `ot-i18n` cover parity (naming the missing keys per side), bundle parsing, key casing and prefixes, the OS-locale mapping, override validation, and the Malay-vs-English fallback.
  **Three design changes this forced, each with its document updated in the same commit.** (1) `fl!` and `i18n-embed-fl` are dropped: both derive the catalogue filename from the *crate* name, so they look for `ot-i18n.ftl`, while the locale files are named after the product as the data-model spec requires. `i18n-config` cannot override the domain, so the loader is built with `FluentLanguageLoader::new("omni_toaster", …)` and the compile-time key check is replaced by the parity, prefix and casing tests — ADR-006 carries the amendment and the condition for revisiting it. (2) `format_datetime` lost its `lang` parameter: the words come from the active bundle and `TimePrefs::for_language` carries the only language-dependent choice, so a `lang` argument could only have been ignored. `docs/03` and `docs/10` are corrected. (3) `publish = false` is now inherited by every member, which is true (the deliverable is the exe) and is what lets `cargo deny` accept versionless intra-workspace path dependencies.
  **Two bugs the App check caught that the tests had not**, both now in GOTCHAS: `load_languages` takes the *active* language first and fallbacks after, so "English first for the fallback" made every lookup answer in English while the UI reported Malay — invisible to parity tests and to the language-name key, which reads the same in both files; and Fluent's U+2068/U+2069 isolating marks were reaching users as stray boxes in `Logs are kept in ⁨D:\…⁩.`, now stripped in `text_with`, the one place every substituted string passes through. A regression test covers each.
  `cargo deny` also went red twice: `basic-toml` is flagged **unmaintained** (not vulnerable, no upgrade available) and arrives only inside the `i18n-embed` proc-macro at build time, so it is ignored in `deny.toml` with that reasoning written down.

### FEAT-0005 · SQLite store and migration `001_init.sql`
- **State:** Completed · **Spec:** docs/05-data-model-and-storage.md · **Phase:** 0
- **Branch:** feat/phase-0-store
- **Depends on:** FEAT-0002
- **Verified:** the gate below plus the App box, run 2026-11-08
- [x] `ot-store` opens `data/omnitoaster.db` with `rusqlite`, WAL journal, foreign keys on
- [x] Migration runner tracked with `PRAGMA user_version`, applying numbered files in order inside one transaction each
- [x] `001_init.sql`: `settings`, `location`, `category`, `reminder`, `snooze_log`, `notification_log`, `asset`, `weather_cache`, per `PROPOSAL.md` §9
- [x] Every statement parameterized; no string-built SQL anywhere in the crate
- [x] Test: a fresh database migrates to `user_version = 1` and has every table; running the migration twice is a no-op; an existing database at version 1 is left alone; a settings round trip returns what was written
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check — 52 tests pass. `--self-test` still does not exist; FEAT-0006 owns it
- [x] App: `--data-dir ./tmp-data` — `data/omnitoaster.db` appears on first run, `PRAGMA user_version` reads 1, and a second run does not re-migrate
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** Migrations `002_locale_keys.sql` and `003_jakim.sql` belong to Phases 4 and 5 and are **not** written here — `category` therefore has no key columns and no `authored_lang` yet, which is what `docs/05` assigns to `002`. 8 tests cover the migration, the table set, idempotence, the settings round trip, and three things the schema would otherwise only claim: that foreign keys are actually enforced (SQLite defaults them **off**, which would make every `ON DELETE` clause decorative), that a delete cascades, and that `STRICT` rejects a text value in an INTEGER column. A database at a *higher* `user_version` than the build knows is refused with a message telling the user to update, rather than being written to with an older schema.
  **Decisions.** `rusqlite` uses the `bundled` feature, so SQLite is compiled from source into the exe and a portable folder can never meet a missing or ancient system `sqlite3.dll`; it also makes `STRICT` tables safe to rely on, since the bundled engine is new enough. Verified that this does not break portability: the release exe has **zero** dynamic CRT imports, so the C compilation picked up `+crt-static` correctly, and the binary grew from 970 KB to 2.6 MB — still one file. `Store` has a hand-written `Debug` that prints only the schema version, because the contents are the user's reminders and should not reach a log line or a crash report by accident. Cargo selected `rusqlite` 0.40.2 rather than 0.41.1 because the newer one needs Rust 1.89 and the workspace declares 1.88; worth revisiting only if 0.40 picks up an advisory. **It did, on 2026-11-09** — RUSTSEC-2026-0459 against the SQLite it bundles — and BUG-0004 took the upgrade and the MSRV with it.
  **A false positive worth knowing about:** `scripts/verify-release.ps1` reports "`--version` and `--self-test` OK" for v0.1.0, but `--self-test` does not exist yet, so the flag is ignored and the exit code is 0 for the wrong reason. That check only becomes real with FEAT-0006.

### FEAT-0006 · The `--self-test` and `--data-dir` flags
- **State:** Completed · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Branch:** feat/phase-0-cli
- **Depends on:** FEAT-0002, FEAT-0003, FEAT-0004, FEAT-0005
- **Verified:** the gate plus the release exe, run 2026-11-08
- [x] CLI parsing in `omnitoaster` for `--data-dir <dir>`, `--self-test`, `--worker` and `--version`, with a bilingual `--help`
- [x] `--self-test` runs the headless checks and prints one flushed line per check with `ok` or `FAILED`: paths resolve and are writable, the log file is written, both locale bundles parse, key parity holds, the database migrates to the expected version, the date formatter returns a value in each language
- [x] Exit code 0 only when every check printed `ok`. **The panic half of this box could not be built as written** — see Notes; the name is printed before the check instead, so an abort still names what died
- [x] `--worker` is accepted and recognised now but does nothing until Phase 8
- [x] Test: `--self-test` against a temp data dir exits 0 and its output contains every check name; the broken-locale case is covered, with a correction — see Notes
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · **--self-test on the release exe** — 61 tests, and the self-test's six checks all `ok`
- [x] App: `--self-test` exits 0 with every line `ok`; `--data-dir ./tmp-data` still starts normally; `--help` reads correctly in both languages
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** **The `--self-test` gate box on FEAT-0001 to FEAT-0005 was left unticked because the flag did not exist. It does now, and it passes on the release exe**, so that caveat on those five items is resolved — their boxes stay as they were ticked, since they record what ran at the time.
  **One box could not be built as written.** "A panic inside a check becomes a `FAILED` line naming it" needs `catch_unwind`, and the release profile sets `panic = "abort"` so the supervisor sees a non-zero exit rather than a half-unwound process — nothing unwinds, so nothing can be caught. Rather than weaken the panic strategy for the benefit of the self-test, the check's **name is printed and flushed before the check runs** and the verdict after it, so an abort leaves the dying check's name as the last line of output. Same diagnostic value, different mechanism; `docs/09-ci-cd-and-releases.md` now specifies the ordering and why, and a test asserts the name precedes the verdict.
  **A correction to this item's own test box.** It asked for "a deliberately broken locale makes it exit non-zero and name the locale check". That contradicts `docs/05-data-model-and-storage.md`, which requires a broken *override* to be ignored with a logged warning so a user cannot break their app by mis-editing one. Both cannot be true. The spec wins: the test now asserts the broken override is survived, the locale check still runs, and nothing crashes or hangs. A broken *embedded* catalogue would fail the check, but it cannot be produced at runtime — it would fail `cargo test` first.
  **Beyond the boxes, one addition worth naming:** an argument starting `--` that is not a known flag is now rejected and named. A silently ignored `--data-dirr` typo would have started the app against the **real** `OmniToasterData/`, which is the kind of thing that eats someone's reminders. A test keeps `--help`'s list and the accepted set from drifting apart.
  `ot-i18n` gained public `message_keys`, `check_key_parity` and `probe_language` so the self-test and the unit test share one parity implementation rather than each parsing the catalogues their own way.

### FEAT-0007 · CI skeleton
- **State:** Completed · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Branch:** the realignment batch, then `feat/phase-0-cli` for this bookkeeping
- **Depends on:** FEAT-0006
- **Verified:** nine green `ci.yml` runs and two published releases — the run numbers are in Notes
- [x] `.forgejo/workflows/ci.yml` on the owner's machine: fmt, clippy `-D warnings`, `test --workspace --locked -j 4`, locale key parity, `cargo deny check`, release build, then `--version` and `--self-test` on the built exe — every cargo step bounded with `-j 4`
- [x] `.github/workflows/ci.yml` running the same checks on the mirror, guarded with `if: github.server_url == 'https://github.com'`
- [x] `ci.yml` does not run on `main`; documentation-only pushes do not trigger a release
- [x] Test: not applicable — a workflow is verified by a green run, recorded in Notes
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [x] App: not applicable — this item ships no user-visible behaviour
- [x] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** **This item sat `Pending` while its work was finished and its CHECKLIST row read `Completed`** — the two disagreed for most of Phase 0, which is the exact drift `task-auditor` exists to catch, and it was my oversight rather than anything about the work. The pipeline itself was built in the realignment batch and has been exercised continuously since.
  The evidence this box rests on: `ci.yml` green on runs 3, 4, 9, 10, 11, 13, 15, 17 and 19 across six branches; `release.yml` published v0.1.0 (run 14) and v0.2.0 (run 18), both then verified from a clean download by `scripts/verify-release.ps1`. Docs-only pushes to `main` stayed green and published nothing while no `Cargo.toml` existed (runs 1 and 2), which is the behaviour the third box asks for. Two release runs failed first — run 5 at the version bump with no signing key configured, run 12 at the tests step on a flaky test — and both failed **safely**, before the push and publish steps, leaving no tag and no release.
  The runner **is** this machine, so no local cargo command may run while a job is; that rule held throughout. The asset contract and the updater are Phase 9. **BUG-0001 is open against this pipeline** — the forge intermittently stops creating runs — and it is a fault in the forge rather than in these workflows, which is why it does not block this item.

### FEAT-0010 · The app icon, in every size and embedded in the exe
- **State:** Completed · **Spec:** docs/08-decisions.md (ADR-013) · **Phase:** 0
- **Branch:** feat/app-icon
- **Source:** the owner supplied the artwork and asked for it on the app, the README and the executable
- **Verified:** the generated set, the built exe's resources, and the small sizes looked at rather than assumed
- [x] `assets/icon/omnitoaster-512.png` as the master, committed as supplied
- [x] `scripts/make-icons.ps1` generating 256, 128, 64, 48, 32, 24 and 16 px PNGs and a seven-entry `omnitoaster.ico`, with a `-Check` mode and a refusal to upscale
- [x] `crates/omnitoaster/build.rs` embedding the `.ico` and the VERSIONINFO block, a no-op on non-Windows so Linux clippy still passes
- [x] README shows the icon and explains the master-and-generated arrangement
- [x] ADR-013 records the tool and why the master is raster rather than SVG, which the rule had assumed
- [x] Test: not unit-testable — verified by inspecting the artefacts, recorded in Notes
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [x] App: the exe's icon and metadata read back correctly; the small sizes inspected visually
- [x] Docs: ADR · README · rules · CHANGELOG · CHECKLIST
- **Notes:** The `.ico` holds 16, 24, 32, 48, 64, 128 and 256, the 256 entry PNG-compressed. Windows reads 16 in the tray and title bar, 32 in the taskbar, 48 in Explorer, 256 for Alt-Tab. Confirmed the embedding is verbatim rather than assumed: the 32 px extracted back out of `OmniToaster.exe` is pixel-identical to `omnitoaster-32.png`, and the PNG-compressed 256 entry is present in the binary. Properties ▸ Details now reads ProductName `OmniToaster`, FileDescription "OmniToaster — portable toast notifications", FileVersion `0.3.0`, taken from `CARGO_PKG_VERSION` so it cannot drift from the release.
  **One real quality problem, found by looking instead of assuming.** A plain Lanczos downscale to 16 px turned the artwork into a coloured blob — the card stack and the bell indistinguishable, the rays gone entirely — and 16 px is the tray and title-bar size, the one users see most. An unsharp pass on sizes at or below 32 px recovers the card edges, the bell silhouette and the rays. **16 px is still the weakest size**, because three stacked cards plus a bell and rays is a lot of detail for 256 pixels. The alternative, a cropped bell-only variant at 16 and 24, would change what the icon *is* at those sizes, so it is the owner's call and would need its own ADR.
  The exe grew from 2.68 MB to 2.90 MB, almost all of it the 256 px entry. `build.rs` needs the Windows SDK's `rc.exe`; it panics naming that requirement rather than silently producing a default-looking exe.

### FEAT-0009 · Release packaging, signing keys and release-planner tests
- **State:** In-Progress · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Branch:** feat/phase-0-foundation
- **Depends on:** FEAT-0001
- **Verified:** 2026-11-08, a signed local packaging run of 0.1.0, checked independently afterwards. `rsign verify -p keys/minisign.pub -x manifest.json.minisig manifest.json` exits 0 with "Signature and comment signature verified" and the trusted comment `OmniToaster-0.1.0-stable`, so **the committed public key and the owner's private key are a matching pair** and the key is unencrypted — an encrypted one would have hung the 60-second timeout instead. The signing ran through the `$env:OT_MINISIGN_KEY` path from the raw file contents, so the script's two-line normalisation is exercised as the forge will exercise it. All six assets were produced. `manifest.json` reads schema 1, product `OmniToaster`, version `0.1.0`, channel `stable`, tag `v0.1.0`, `min_supported_version` `0.1.0`, `data_schema` 1; both asset entries' recorded size and SHA-256 match the real files; `SHA256SUMS.txt` covers the other five files, every hash agrees, and it correctly omits itself. The zip holds `OmniToaster.exe`, `README.txt`, `LICENSE` and `CHANGELOG-0.1.0.md`. The manifest's asset URLs point at the Forgejo download path, which is right for a Forgejo release — the GitHub pipeline passes `-RepoUrl` explicitly for exactly this reason.
- [x] `scripts/build-and-package.ps1` — builds the release exe, assembles `OmniToaster-<version>-windows-x64.zip` (exe + `README.txt` + `LICENSE` + `THIRD_PARTY_NOTICES.md` + notes), writes `manifest.json`, writes `SHA256SUMS.txt` and `CHANGELOG-<version>.md`, signs the manifest with rsign, and fails if any contract member is missing. Written 2026-11-08; **parsed, never executed** — there is no workspace to build yet
- [x] `keys/README.md` — generation, where the public key is embedded, the `OT_MINISIGN_KEY` forge secret, and rotation. Written 2026-11-08
- [x] `keys/minisign.pub` — the owner generated the keypair 2026-11-08; the public half is committed. Shape verified: two lines, `untrusted comment: minisign public key`, a 56-character `RWQ…` body, and no secret material. The private half sits at `./omnitoaster-minisign.key`, which `.gitignore` covers — `git add` refuses it, confirmed, and `git status --ignored` shows it as `!!`
- [ ] `THIRD_PARTY_NOTICES.md` — the packaging script copies it into the zip only `if (Test-Path)`, and it does not exist, so the 0.1.0 zip shipped without it. Harmless today because the workspace has no third-party dependencies; it must exist before the first release that does. Generate it from `cargo about` or `cargo-license` rather than by hand
- [ ] `scripts/tests/` — Pester 5 tests for the release planner against throwaway git repositories, covering each Conventional Commit type's bump and a documentation-only push producing no release
- [ ] Remove the `Test-Path scripts/tests` guard added to `.forgejo/workflows/ci.yml` once the tests exist
- [x] Rename the forge secrets: the workflows read `OT_MINISIGN_KEY` and `OT_FORGE_UPLOAD_URL`, and the GitHub mirror `OT_SIGNING_KEY_EXTRA`. **Proven by release run 14**, which signed `manifest.json` on the runner and published v0.1.0; `scripts/verify-release.ps1 -Tag v0.1.0` then verified that signature against the committed public key from a clean download
- [ ] Test: `pwsh -c 'Invoke-Pester scripts/tests'` green; `scripts/build-and-package.ps1` produces the full asset set and `scripts/verify-release.ps1` accepts a published release. **Two of the three now hold:** packaging produces the full set, and `verify-release.ps1 -Tag v0.1.0` passed against the real published release — downloaded anonymously, signature verified against `keys/minisign.pub`, every size and SHA-256 matched. The box stays unticked **only** because `scripts/tests/` does not exist. The script's "`--self-test` OK" line **was** a false positive from v0.1.0 to v0.2.0 — the flag was unimplemented, so it was ignored and the exe exited 0 for the wrong reason. FEAT-0006 fixed it, and `verify-release.ps1 -Tag v0.3.0` now prints the six check lines inside its own output, so that line means what it says
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: not applicable — this item ships no user-visible behaviour
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** Filed 2026-11-08 during the scaffolding realignment, when both `release.yml` files called `scripts/build-and-package.ps1` and referenced `keys/README.md` while neither existed. Both were written the same day and are ticked above; they are **parsed, not executed**, because there is no workspace to build until FEAT-0001. What is still open is `keys/minisign.pub`, the Pester tests, and the forge secret rename. **Now confirmed, and `Blocked` on the owner.** The prediction came true the moment FEAT-0001 merged: release run 5 on `main` failed. Before FEAT-0001 the release job's "Plan the release" step short-circuited on `if (-not (Test-Path Cargo.toml))` and emitted an empty version, so every later step skipped and a docs push to `main` stayed green. A `Cargo.toml` now exists and a `feat:` commit is in history, so the planner computes a minor bump to `0.1.0` and the job proceeds to the signing step, which throws because `OT_MINISIGN_KEY` is unset and `keys/minisign.pub` is absent. The failure was contained: it dies before the push and publish steps, so **no tag was pushed, no release was created, and `main` is untouched** at `71a5ac3` — verified with `git ls-remote --tags` (empty), the releases API (0 releases) and a remote/local `main` comparison. There is no tag yet, so **every further push to `main` will re-attempt `0.1.0` and fail the same way** until two things the owner must do themselves are done: generate the minisign keypair (`rsign generate --unencrypted`, per `keys/README.md`) and commit `keys/minisign.pub`, then set the `OT_MINISIGN_KEY` secret on both forges, along with `OT_FORGE_UPLOAD_URL` and the mirror's `OT_SIGNING_KEY_EXTRA`. **I did not generate the keypair:** a release signing key is the owner's to create and hold, and nothing should ever put its private half where an agent has handled it. `.forgejo/workflows/ci.yml`'s Pester step was unguarded and would have failed the very first branch push; it is guarded now, and this item removes the guard. The asset names are a contract the updater reads — see `docs/specs/updater.md` and ADR-010; renaming one needs a new ADR.

---

## Backlog

### CR-0001 · The product is "OmniToaster", one word, everywhere a user sees it
- **State:** Completed · **Spec:** docs/01-vision-and-scope.md · **Phase:** 0
- **Branch:** cr/product-name-omnitoaster
- **Source:** the owner, on seeing `OmniToaster --version` print `Omni Toaster 0.1.0`
- **Verified:** the gate plus the app, run 2026-11-08
- [x] Spec first: `docs/01-vision-and-scope.md` states the canonical spelling and what keeps the hyphenated repository name
- [x] Both locale catalogues, so the Malay strings do not keep a spelling the English ones dropped
- [x] The version line, the startup and error messages, the crash report header
- [x] The packaged `README.txt`, the release title and `TAGGER_NAME` on both forges
- [x] Test: no message in either `.ftl` contains the two-word spelling, naming the offending lines
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check — 53 tests
- [x] App: `--version` prints `OmniToaster 0.1.0`; both languages and the crash report header checked
- [x] Docs: spec · README · CHANGELOG · CHECKLIST
- **Notes:** 66 occurrences across 32 files, done as a literal replacement with no regex or escapes — the only kind of scripted edit this project trusts (`docs/GOTCHAS.md`). The real defect was that the product had **two** spellings: the exe, the data folder, the package, `docs/CHANGELOG.md` and `docs/CHECKLIST.md` already said `OmniToaster`, while the proposal, the specs and every user-visible string said `Omni Toaster`. Two things were deliberately left alone: `Feasibility Study.txt`, which records the owner's own original words and is a historical input rather than a document we maintain; and the released `0.1.0` section of `docs/CHANGELOG.md`, which belongs to the release workflow and is never hand-edited — it happens to contain no occurrence anyway, since its entries are commit subjects. **v0.1.0 shipped with the old spelling** and cannot be changed retrospectively; v0.2.0 carries the fix.
  **A second mistake, found afterwards and worth more than the first:** this was committed as `style:`, which `scripts/next-release.ps1` lists in `$NoReleaseTypes` — no version bump and **no line in the release notes**. So the rename reached users in v0.2.0 carried by the `feat(store)` commit in the same push, and v0.2.0's notes do not mention it. Released sections are never hand-edited, so that cannot be corrected now. A change to user-visible text is `fix:` or `feat:` however cosmetic the diff looks; `docs/09` and `docs/GOTCHAS.md` now say so. The same review also found `docs/09`'s bump table wrongly listing `refactor:` and `build:` as patch bumps when the script treats both as no-release.

### BUG-0004 · The bundled SQLite carried CVE-2026-119375
- **State:** Completed · **Spec:** docs/06-security-and-privacy.md · **Phase:** 1
- **Branch:** feat/live-language
- **Source:** `cargo deny check` went red mid-branch on RUSTSEC-2026-0459, with no change of our own — exactly the case the testing rules describe
- **Verified:** the gate, plus the bundled header now reading `SQLITE_VERSION "3.54.1"`
- [x] Establish whether the code reaches the vulnerable path. **It barely does, and the fix was taken anyway:** SQLite before 3.54.0 can return uninitialized page-cache memory when a database is opened alongside a `-wal` file whose page size differs, and only a process that opens a database a less trusted party could have written is exposed. OmniToaster opens one database, the one it created, in its own portable folder. But saying "unreachable" and shipping a known-vulnerable SQLite is not the same as not shipping one
- [x] `rusqlite` 0.40.2 → 0.41.1 and `libsqlite3-sys` 0.38.2 → 0.39.1, which bundles SQLite 3.54.1. No API change was needed
- [x] MSRV 1.88 → 1.89, which `rusqlite` 0.41 requires. The declared floor follows the versions that carry the security fixes rather than the fixes waiting on the floor
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 (120 tests) · deny check · --self-test
- [ ] App: not applicable — no behaviour changed. The migration path is covered by the existing fresh-and-existing database tests, which pass against the new engine
- [x] Docs: CHANGELOG · TASKS
- **Notes:** The advisory landed while an unrelated feature branch was open, which is exactly why `cargo deny check` is in the local gate: its result does not depend on the branch, so one that was green yesterday goes red today with nothing of its own changed.
  Worth saying plainly rather than implying a shipped vulnerability: **the app has never opened a database it did not create**, so no release was exposed in a way that mattered. The upgrade removes the code from the binary regardless.

### BUG-0003 · A path dependency pinned `0.4.0`, so the first minor release could not resolve
- **State:** Completed · **Spec:** docs/08-decisions.md (ADR-001) · **Phase:** 0
- **Branch:** bug/path-dep-pins-a-version
- **Source:** the v0.5.0 release run went red immediately after Phase 1's merge — `failed to select a version for the requirement ot-platform = "^0.4.0"`, candidate versions found which didn't match: 0.5.0
- **Verified:** the gate, plus a simulated version bump to 0.5.0 that now resolves and builds
- [x] Test first: `crates/omnitoaster/tests/manifests.rs` reads every manifest in the workspace and fails on any dependency line carrying both a `path` and a `version`, naming file and line. It failed on the offending line before the fix
- [x] Fix: `crates/omnitoaster/Cargo.toml` takes `ot-platform` by path alone, like its three siblings
- [x] Proved against a real bump: `[workspace.package] version` set to `0.5.0` in a scratch copy, `cargo build` resolves and compiles, then reverted
- [x] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: not applicable — a manifest defect with no runtime behaviour. The release run going green is the proof, and it is the step that failed
- [x] Docs: CHANGELOG · GOTCHAS · docs/09-ci-cd-and-releases.md
- **Notes:** The version lives once, in `[workspace.package] version` (ADR-001), and `scripts/update-version.ps1` moves it. The pin kept the literal `0.4.0` while the crate it pointed at moved with the workspace.
  **Why it stayed hidden through four releases:** `version = "0.4.0"` is a caret requirement, so it accepted 0.4.1 and every patch release resolved. The first **minor** bump is what breaks it — and that is the release least likely to be watched for a resolver error, since nothing about the change touches the manifest. v0.5.0 is the release that found it; v0.1.0 to v0.4.1 were all patch or same-minor bumps from a version the pin accepted.
  The failure surfaces inside `scripts/build-and-package.ps1`, which made it look like a packaging fault. It is not: the script reported a `cargo build` exit code 101 faithfully, and PowerShell's error rendering mangled the throwing line into `. XITCODE -ne 0) { … }`, which is cosmetic and not a defect in the script.

### BUG-0002 · The GitHub mirror handoff skips silently, so no release reaches GitHub
- **State:** Completed · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Source:** the owner asked why GitHub had built no release
- **Verified:** v0.5.0, 2026-11-09 — the first release to mirror itself with nothing driven by hand
- [x] Establish what is actually missing
- [x] Set `OT_GITHUB_MIRROR_URL` and `OT_GITHUB_MIRROR_TOKEN` as repository secrets on the Forgejo instance, and `OT_MINISIGN_KEY` on GitHub so its release job can sign — the owner set all three on 2026-11-08. **Now verified:** v0.5.0's Forgejo run pushed the tag to GitHub and dispatched its release workflow, which signed and published six assets without anything being driven from here. All three secrets are therefore present and sufficient
- [x] Back-fill v0.1.0 to v0.4.0, which GitHub never received. **Done 2026-11-08:** the owner pushed the four tags, which started nothing on their own as predicted, and all four builds were dispatched from here and went green — five releases on GitHub, six assets each. One correction was needed afterwards: back-filling in ascending order left `/releases/latest` answering v0.4.0, a downgrade for a v0.4.1 user, so v0.4.1 was re-marked latest. Previously: needed one command from the owner, then this session can finish it. All four tag commits are already reachable from GitHub's `main`, so only the refs are missing — no objects to transfer. Creating them from here is refused by this session's permission guard, both as a `git push` of the tags and as a `gh api` ref creation, which is the right call for an action that creates public surface. The owner runs:
  ```
  git push https://github.com/mzaki-raup/omni-toaster.git v0.1.0 v0.2.0 v0.3.0 v0.4.0
  ```
  That pushes the annotated tags as they exist on Forgejo, and starts nothing on its own: every tagged commit is a `chore(release): … [skip ci]`, which GitHub honours. Dispatching the builds afterwards is **not** blocked — `gh workflow run release.yml -f tag=<tag>` worked for v0.4.1 — so the builds can be driven from here once the refs exist
- [x] Make the skip **loud**: every exit path in `scripts/push-to-mirror.ps1` now reports through `Write-Outcome`, which emits a `::warning::` annotation and a line into `$GITHUB_STEP_SUMMARY`. The unconfigured case still exits 0 — a fork or a forge without a mirror must be able to release — but it can no longer pass for success. Exercised both paths locally: the annotation prints and the summary line is written
- [x] Decide whether an unconfigured mirror should fail the release outright once the secrets exist. **It should not, and does not need to:** once the secrets exist the script gets past the guard, and a push that is refused already throws. The only case that still exits 0 is genuinely-absent configuration, which is the fork case. A warning covers it
- [x] Test: a release run reports what it did with the mirror either way. Forgejo run 30 carried the reporting change and its mirror step worked end to end — the tag reached GitHub and the release workflow was dispatched. The summary line's *rendering* is unverified, since Forgejo's job logs need a token this session does not have; the step's behaviour is proven by its effects
- [x] Docs: spec · CHANGELOG · GOTCHAS. `docs/CHECKLIST.md` gets nothing: it tracks the roadmap's phases, and a fault in the pipeline is not a phase row — FEAT-0009 already carries the release packaging
- **Closed by v0.5.0** (2026-11-09): the end-to-end proof this item was waiting for. Forgejo published v0.5.0 with six assets, its mirror step pushed the tag and dispatched GitHub's release workflow, that run signed and published six assets of its own, and `/releases/latest` on the mirror answers v0.5.0. Nothing in that chain was dispatched by hand, which is what every earlier release needed.
- **Notes:** Found 2026-11-08. GitHub has **no tags and no releases**, while Forgejo has v0.1.0, v0.2.0 and v0.3.0. `.github/workflows/release.yml` triggers on `push: tags: ['v*']`, so with no tag it has never run — GitHub's 10 Actions runs are all `ci.yml`, all green.
  The tags never arrive because the Forgejo release job's last step runs `scripts/push-to-mirror.ps1`, which opens with: if `OT_GITHUB_MIRROR_URL` or `OT_GITHUB_MIRROR_TOKEN` is unset, print "GitHub mirror not configured … skipping" and **`exit 0`**. The job stays green having mirrored nothing. That silence is the actual defect — the missing secrets are just configuration.
  `main` *is* current on GitHub, which is what makes this easy to miss: `origin` carries GitHub as a second **push url**, so `git push origin main` reaches both. That pushes the refs named on the command line, never tags, so branches mirror as a side effect of ordinary pushes while tags depend entirely on the step that is skipping.
  **I may have caused it.** Earlier the same day I renamed every `HS_*` pipeline variable to `OT_*`, including these two. If they had been configured under the old names, that rename broke the handoff — and because the script exits 0, nothing said so. I cannot read secret names from here to tell whether they were ever set for this repository, so the honest position is that the rename is a candidate cause rather than a proven one.
  **Fixed and proven, 2026-11-08.** After the owner replaced the secret with the actual secret key (timestamp moved 15:21:57Z → 15:28:33Z), a re-dispatch went green on every step, including *Publish the release* and *Verify the published release as a user would*. GitHub now carries **v0.4.1 with all six contract assets**, signed and self-verified. The full chain works: Forgejo publishes, pushes the tag to GitHub, dispatches GitHub's workflow, and GitHub builds, signs and publishes its own copy.
  Three distinct faults sat on top of each other here, which is why it took four attempts: the mirror secrets were absent so the step skipped silently; then the token lacked Contents/Workflows/Actions write so the tag push 403'd; then `OT_MINISIGN_KEY` was set under the wrong name, then set to the public key rather than the secret key. Each one masked the next.

  **Second GitHub attempt: the secret exists now, and is the wrong key.** `gh secret list` confirmed `OT_MINISIGN_KEY` was created at 15:21:57Z, eight minutes *after* the failed run started at 15:13:10Z — so the earlier one-second guard failure was simply the secret not existing yet. The repository also carries `OT_SIGNING_KEY` from 04:06, which is the wrong name and is what the first attempt set.
  Re-dispatching the workflow got much further: **the build succeeded in 1m04s**, which proves `build.rs` and the icon's resource embedding work on a GitHub-hosted runner — a genuine unknown until now, since GitHub had never built any release. It then failed at `rsign sign` with **`failed to fill whole buffer`**, rsign's way of saying the key material is shorter than a secret key. The likeliest cause is that the secret holds the **public** key: both halves are two-line files beginning `untrusted comment:`, so the script's shape check passes and only rsign notices. The discriminator is the second line's length — 56 characters for the public key, roughly 140 to 210 for an unencrypted secret key.
  Whatever is pasted must also be the **same** secret key Forgejo signs with, because `keys/minisign.pub` is committed and `build-and-package.ps1` runs `rsign verify` against it straight after signing.
  **Tooling note that should have come first:** `gh` is installed and authenticated here as `mzaki-raup` with `repo` and `workflow` scopes. That makes secret *names*, job logs and `workflow run` all readable or runnable from this session. Two diagnostic rounds were spent inferring from step timings and asking the owner to read logs before this was checked.

  **The mirror itself is fixed (2026-11-08).** After the owner widened the token, Forgejo release run 30 went green, `v0.4.1` reached GitHub, and the script dispatched GitHub's release workflow — which is the only way it starts, since the tagged commit is `chore(release): … [skip ci]` and GitHub honours that marker. So the 403 was the whole of the mirror problem and all three permissions were needed.
  **GitHub's own release build then failed, which is a separate fault.** Its job steps are public even though its logs are not: fmt, clippy, tests, the `rsign2` install and the release notes all passed, and **"Build, package and sign" failed after 1 second**. That timing is the diagnosis — a cargo build takes minutes and the rsign password timeout is 60 s, so it threw before doing any work. The first statement in that step is `if (-not $env:OT_MINISIGN_KEY) { throw … }`, so **`OT_MINISIGN_KEY` is arriving empty in the GitHub job**. The repository is not a fork and the run was a `workflow_dispatch` on `main`, so neither of the usual reasons secrets go missing applies; the likely causes are that it was set as a *variable* rather than a *secret*, at *environment* scope when the job declares no environment, on the wrong repository, or under a slightly different name. It must also be the **same** key as Forgejo's, because `keys/minisign.pub` is committed and `build-and-package.ps1` runs `rsign verify` against it immediately after signing — a separate GitHub keypair would sign fine and fail verification.

  **Root cause, from run 27's log: a 403 on the tag push.**

```
Pushing v0.4.0 to the GitHub mirror failed; GitHub will not build this release.
remote: Permission to mzaki-raup/omni-toaster.git denied to mzaki-raup.
fatal: unable to access '***/': The requested URL returned error: 403
```

  Two things follow. The mirror step **ran** rather than skipping, so the secrets exist and the silent-skip symptom is gone. And `OT_GITHUB_MIRROR_URL` is **correct** — a 403 means git reached the right repository and authenticated as the right account; a wrong URL returns 404. What is wrong is the token's permissions.
  `scripts/push-to-mirror.ps1` states the requirement in its own header: write access to Contents, to Workflows (a pushed history can touch `.github/workflows/`, and this repository has them) and to Actions (to dispatch the release run). A fine-grained PAT needs this repository listed under Repository access — they grant nothing by default — with Contents, Workflows and Actions all read/write; a classic PAT needs `repo` **and** `workflow`.

  **v0.4.0 was the test, and it is inconclusive — but it moved.** Release run 27 published v0.4.0 to Forgejo with all six assets, and `scripts/verify-release.ps1 -Tag v0.4.0` passes from the desktop: signature, every size and hash, and the exe's six self-test checks. So the release itself is sound. The run still went **red**, GitHub still has no tags, and no Release run appeared in GitHub's Actions.
  Which of the last two steps failed is unknown from here. "Verify the published release" runs **before** "Hand the release to the GitHub mirror", so a verify failure would explain the mirror never running — and GOTCHAS already records that step going red on the runner as LocalSystem in session 0 while the same release verifies fine on the desktop, which is exactly the shape of what happened. The alternative is that the mirror step ran with the new secrets and failed on the push, which would point at the token's scope or the URL rather than at the names. Both fit the evidence equally.
  The forge's job-log endpoints return nothing without a token, so this needs one look at run 27 in the forge UI. That is twice now a diagnosis has stalled on not being able to read a job log; making the mirror step report its outcome into the run summary — already an open box above — would make this self-diagnosing instead.

  Note that GitHub's own release job will also need `OT_MINISIGN_KEY` to sign, and uses `GITHUB_TOKEN` plus the `gh` CLI for the rest.

### BUG-0001 · A branch push intermittently creates no `ci.yml` run
- **State:** Blocked · **Spec:** docs/09-ci-cd-and-releases.md · **Phase:** 0
- **Blocked on:** the owner — this is the forge's Actions dispatch, not anything in this repository
- **Reopened:** 2026-11-08, after a second and third occurrence. The earlier closure called it a one-off transient outage; it is a recurring one, which is a different thing
- [x] Reproduce: a branch push creates no workflow run, while the forge, the runner and the git push itself are all healthy
- [x] Rule out the workflow's `on.push` filter — a later branch push triggered `ci.yml` within seconds with the filter untouched
- [ ] Establish why the forge stops creating runs. First occurrence came with `502 Bad Gateway` in the runner's task fetch; the second and third came with **no error at all** in the runner log, so a 502 is a symptom rather than the cause
- [ ] Decide what the project does about it: whether `workflow_dispatch` is the manual nudge, or whether the branch gate needs a documented "CI did not run" path so work is not blocked indefinitely
- [ ] Docs: CHANGELOG · CHECKLIST · GOTCHAS
- [x] Reproduce: push a branch and check whether a workflow run is created
- [x] Establish the cause
- [x] Confirm `ci.yml` still does not run on `main`
- [x] Test: not unit-testable — settled by a real branch push
- [x] Docs: CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** My first diagnosis — the `on.push` filter's `!` negation — was wrong, and no code was changed for it. My second — "a transient outage, closed" — was also wrong, just less so.
  **Occurrence 1, 05:26.** `docs/realign-scaffolding` reached Forgejo at `2a720c0` (`git ls-remote` confirmed it) and 20 minutes of polling showed `total_count` still 1. The runner logged `fetching a task timed out after 5s` and `failed to fetch task: unavailable: 502 Bad Gateway` at 05:40. A later push of `feat/phase-0-foundation` triggered `ci.yml` in seconds with the filter untouched, which is what ruled the filter out.
  **Occurrences 2 and 3, ~13:30 onward.** `feat/phase-0-store` at `c084482` and `cr/product-name-omnitoaster` at `e7ec0f0` both reached the remote and neither produced a run. This time the diagnostic came back completely clean: forge responding on `/api/v1/version` with `9.0.3+gitea-1.22.0`, `gitea-runner` service **Running**, and **no error of any kind** in the runner log — whose last task fetch was 13:12:45, consistent with no task existing to fetch rather than a failure to fetch one. So the 502 in occurrence 1 was a symptom of the same underlying stall, not its cause.
  **Occurrence 4, 2026-11-08 evening.** `cab96a0` on `bug/mirror-skips-silently` reached the remote — `git ls-remote` confirms it — and no run was created; `total_count` stayed at 28. Same clean diagnostic: forge answering on `/api/v1/version`, `gitea-runner` **Running**. This time the runner log does carry a network error, `failed to fetch task … wsarecv: An existing connection was forcibly closed by the remote host`, against `104.21.50.102` — a Cloudflare address, not the origin. So the pattern across four occurrences is a **connection between the runner and the forge being dropped by whatever sits in front of it**, which shows up as a 502, a forcible close, or nothing at all depending on timing. That points at the proxy or tunnel rather than at Forgejo itself, which is worth checking before anything in the forge's own configuration.

  **Occurrence 5, 2026-11-09 ~09:25Z.** `f8878dc` on `feat/tray-icon` reached the remote — `git ls-remote` shows it on `refs/heads/feat/tray-icon` — and no run was created; the task list stayed at run 41 for a quarter of an hour. Forge answering on `/api/v1/version`. **New detail worth having:** the runner log shows it was occupied with a job for a *different* repository at the time (`repository=owner/hyper-stack`, run 555, started 17:18 local), and it carries `UpdateLog error: unavailable: 502 Bad Gateway, retrying until reconnected` followed by a reconnect a couple of seconds later. A busy runner should still leave a run *created and waiting* rather than no row at all, so this does not explain it on its own — but it is the first occurrence where the forge was demonstrably mid-conversation with the runner about something else, and the 502 pattern from occurrences 1 and 4 is there again. Everything still points at the proxy in front of the forge.
  **It also means the runner was in use by another project while this session worked**, which is the situation the rules warn about from the other side: the CI runner is this machine, so a local `cargo` run and someone else's CI job can collide.

  **Why this matters beyond the annoyance:** the project's mandated flow is branch → push → CI green → merge, and it cannot be followed while runs are not created. Work either waits indefinitely or merges unverified. That decision is the owner's, which is why this is `Blocked` rather than carried further here.
  The diagnostic order itself is worth keeping and is in `docs/GOTCHAS.md`: confirm the commit is on the remote, read the runner log, then check the forge — before touching a workflow that is probably innocent.

### FEAT-0008 · In-app updater
- **State:** Pending · **Spec:** docs/specs/updater.md · **Phase:** 9
- [ ] Fetch `manifest.json` over HTTPS on a schedule and on demand from Settings → About
- [ ] Verify the detached rsign signature against a public key embedded in the exe **before** reading the manifest's contents as trustworthy
- [ ] Compare the manifest version with `CARGO_PKG_VERSION` by semver, not by string
- [ ] Download the bare `OmniToaster.exe` (not the zip — the zip is the manual path, see `docs/specs/updater.md` §3), check its SHA-256 against the manifest, and stage it beside the running exe as `OmniToaster.exe.new`
- [ ] Replace the running exe on restart, leaving `OmniToasterData/` untouched
- [ ] Never update silently: the user is told what version, what size and what changed, and chooses
- [ ] A failed signature or digest is reported and abandoned — never retried against an unsigned asset, never fallen back to an unverified download
- [ ] Test: a manifest with a bad signature is refused; a good manifest with a mismatched digest is refused; an equal or older version offers no update; a newer one does; the staged-replace path is exercised against a fixture
- [ ] Gate: fmt · clippy -D warnings · test --workspace --locked -j 4 · deny check · --self-test
- [ ] App: `--data-dir ./tmp-data` — install the previous release on a clean machine, let it find the current one, accept, and confirm the new version starts with its data intact; then point it at a tampered manifest and confirm it refuses
- [ ] Docs: spec · README · CHANGELOG · CHECKLIST · GOTCHAS
- **Notes:** The owner has made this a requirement, not an option: it is a Phase 9 deliverable with its own exit criterion in `PROPOSAL.md` §13, and the asset contract it depends on is in §12. It stays in the Backlog until Phase 9 opens as a task version.

---

