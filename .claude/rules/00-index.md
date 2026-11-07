# Rules index

OmniToaster is a portable Rust + egui desktop workspace: a Cargo workspace of
small crates, one per tool, shipping as a single static exe that keeps all its
data in `OmniToasterData/` beside itself. `README.md` has the crate layout.

The root [CLAUDE.md](../../CLAUDE.md) holds the behavioural guidelines that
apply to any project. **These files hold OmniToaster's own rules** — read the one
for the area you are about to change.

| File | Read it before you… |
|---|---|
| | |


Always also read the matching section of [docs/GOTCHAS.md](../../docs/GOTCHAS.md)

## The non-negotiables, in one place

- **Portable:** all app data under `OmniToasterData/` beside the exe. Never the
  registry, `%APPDATA%` or `%TEMP%`. Never commit `OmniToasterData/`.
- **Secrets:** never plaintext. Encrypted vault, referenced as `secret:<ulid>`.
  Redacted in logs and history. A test that writes a secret greps the data
  folder afterwards.
- **The UI thread never blocks.** All I/O on the tokio runtime; results through
  bounded channels drained at frame start; anything over ~200 ms is a Job with
  progress and cancel. Never `block_on` on the UI thread.
- **Specs are requirements.** A design change updates its document in the same
  commit.
- **Hyper Stack** (`../hyper-stack`) is a reference only. Don't copy code
  and don't add it as a dependency.

