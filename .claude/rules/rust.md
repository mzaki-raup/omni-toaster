# Rust rules

## Shape of a change

- Smallest change that works. No speculative features, no abstractions for
  single-use code, no refactors of working code, no dependency upgrades unless
  the change requires them.
- Surgical edits: don't touch adjacent code, comments or formatting. Match the
  surrounding style. Remove only the imports and functions *your* change made
  unused. Mention unrelated dead code; don't delete it.
- Every changed line must trace to the request.

## Language and crate rules

- Edition 2024: `gen` is a reserved keyword — name the identifier `generate`.
- Return `Result` and propagate with `?`. Avoid `unwrap()`/`expect()` outside
  tests; where unavoidable, `expect("invariant")`.
- Avoid `unsafe`; if it is needed, add a `// SAFETY:` comment.
- No crate-wide allow attributes. Use a scoped `#[allow(...)]` with a reason.
  Clippy must pass with `-D warnings`, including on Linux.
- Insert new functions **before** the `#[cfg(test)] mod tests` block, never
  after it, and never by appending with `cat >>`.

## Crate boundaries

- One concern per library crate: `ot-core`, `ot-i18n`, `ot-store`,
  `ot-platform`, `ot-ui`, `ot-toast`, `ot-sched`, `ot-prayer`, `ot-weather`,
  `ot-media`. `crates/omnitoaster` is the binary and the only crate that wires
  them together.
- **All Windows-only code lives in `ot-platform`.** Anything in another crate
  that would not compile without `#[cfg(windows)]` — a `windows` crate call, a
  registry read, an HWND — belongs behind an `ot-platform` interface instead.
  Clippy must pass on Linux, which is what catches this.
- Modules don't call each other: they publish and receive over the event bus.
  A library crate reaching sideways into another's internals is a finding.
- The scheduler owns time, `ot-i18n` owns formatting, `ot-store` owns SQL. Don't
  reimplement any of the three elsewhere.

## Reuse before writing

Search first: ripgrep for the function name. If logic is needed twice, extract a shared function
instead of copying. Don't change a public signature without updating every call
site.

## Performance

- Cache derived data on a hash of its inputs. Never recompute diffs, layouts,
  regexes or parses every frame inside an immediate-mode `ui()`. Test with a
  large document.
- Don't allocate in hot loops: pre-size with `Vec::with_capacity`, reuse
  buffers, prefer iterators over intermediate vectors.
- Share large data with `Arc`, references or channels, not clones. Anything
  crossing a channel between the UI, the scheduler and a worker thread is
  `Send`, so caches shared across threads use `Arc`, never `Rc`.
- Measure before optimising (`criterion`, `cargo flamegraph`).

## Build hygiene on this machine

- Use `cargo test -j 4`. A full parallel build can exhaust the page file and
  corrupt `.rmeta`/`.rlib` files; if that happens, delete `target/debug` and
  rebuild.
- Delete `target/debug/incremental` after long build sessions. Never set
  `CARGO_PROFILE_DEV_DEBUG=0` on the command line.
- Keep `.cargo/config.toml` (`+crt-static`) so the MSVC build stays portable.
- A running `OmniToaster.exe` locks its own file, so a release build fails with
  "Access is denied" while an instance is open. Stop it first:

```bash
Stop-Process -Name OmniToaster -Force
```
