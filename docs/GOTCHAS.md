# GOTCHAS.md

Known mistakes and their fixes. Read the section for your area before starting work, and add new entries under the matching heading as `- [YYYY-MM-DD] Symptom → Correct approach`.

An entry names the trap in bold, then gives the symptom as it actually appeared, then the fix. A section with no entries says so; do not invent one to fill it.

The entries below were carried over from the owner's previous Rust project on
this same machine and toolchain. They are kept because they are true of this
Windows box, this GPG setup, this Forgejo runner and the Rust edition in use,
and because re-learning any of them costs hours. The project-level entries from
that codebase were dropped — nothing here refers to code Omni Toaster does not
have.

## Git, Merges and Releases
- **`origin` has no fetch refspec, so `git fetch origin` refreshes nothing.** `remote.origin.fetch` is unset in this checkout: `origin/main` only moves when *you* push to it, never when the release workflow commits on `main`. A branch cut from `origin/main` after a release was based on the previous release's commit and built without the work merged in between. Before branching or merging, fetch with the refspec spelled out - `git fetch origin +refs/heads/main:refs/remotes/origin/main` - or compare against the forge's `branches/main` API.
- **A merge resolution and its commit must be one `&&` chain, and the marker count a hard gate inside it.** A resolution script died on a `/tmp` path, the `;` after it let `git add && git commit && git push` run anyway, and a merge with `<<<<<<<` in `CHANGELOG.md` reached `main`. The shape that cannot do this is `resolve && test "$(grep -c '^<<<<<<<\|^=======\|^>>>>>>>' CHANGELOG.md)" = 0 && git commit …` - every step a gate, nothing after a `;`.
- **Never read a push's result through `| grep -v remote | tail -1`.** A push is a gate: check its exit code (`git push … || echo PUSH FAILED`) and `git log -1 origin/main` after it, exactly as a `cargo` gate is checked by its exit code and not by eye.
- **Commit gates must check exit codes.** `cargo clippy ... | grep ... ; git commit` committed code with lint errors. Use `cargo clippy ... >/dev/null 2>&1 && git commit`, not a check on printed output.
- **Never use `--no-verify` or `--no-gpg-sign`.** If GPG signing fails, fix the agent (see the gpg-agent note below). Don't bypass it.
- **gpg-agent cache.** `%APPDATA%\gnupg\gpg-agent.conf` sets `default-cache-ttl 14400`. The agent re-reads it only after `gpgconf --reload gpg-agent`. `gpgconf --kill gpg-agent` drops the cache.
- **Two GnuPG installations share one `~/.gnupg` here.** `gpg` on the Git Bash PATH is MSYS GnuPG 2.4.5 and cannot read the keyboxd `pubring.db` that `common.conf`'s `use-keyboxd` selects: it reports `skipped "<key>": No secret key` and lists no keys at all, which looks like a lost key and is not. Only `C:\Program Files\GnuPG\bin\gpg.exe` (2.5.16) sees them, and it is what `gpg.program` points git at. Probe signing with that full path, never the bare `gpg`. The harmless-looking `sending fd … to keyboxd: Input/output error` it prints is not the failure; read the `[GNUPG:]` status lines for `SIG_CREATED` or `Timeout`.
- **CHANGELOG conflict markers.** A merge of a release into a feature branch left `<<<<<<<`, `=======`, and `>>>>>>>` in `CHANGELOG.md` for five releases, and nothing caught it. Run `grep -n '^<<<<<<<\|^=======\|^>>>>>>>' CHANGELOG.md` before any merge commit. CI fails on markers in tracked text.
- **Resolving a CHANGELOG conflict.** Never keep "both". The released section belongs to the release workflow, so keep its generated text as is. Unreleased keeps only items not yet released. Confirm with `grep -c '^<<<<<<<\|^=======\|^>>>>>>>' CHANGELOG.md` (must print `0`).
- **Writing Unreleased notes before a release lands** guarantees that conflict. Expect it, and resolve it as above.
- **New workspace member on a branch + merged release bump.** The version script can't see a crate that exists only on the branch, so `Cargo.lock` keeps its old version and every `--locked` command fails with "cannot update the lock file". Run a plain `cargo check`, then commit the one-line lock fix. The release job may already have tagged `main`.
- **Diff base.** `git diff <base>..HEAD` for "the last N days" needs the right base commit. A wrong base produced empty diffs for six crates in review.

## Release Pipeline and Updater
- **CI must pass `-j 4` like everything else on this machine.** The Forgejo runner *is* the owner's Windows box, so an unbounded `cargo test --workspace` there exhausts the paging file exactly as it does from a shell: a run died with `rustc-LLVM ERROR: out of memory` / `STATUS_ILLEGAL_INSTRUCTION` compiling one crate's lib test, on a commit that changed only Markdown and PNGs. The rules have mandated `-j 4` for local work all along; the Forgejo workflows must too. The `.github/` copies are deliberately left unbounded - those run on GitHub-hosted machines.
- **The "nothing to release yet" guard stops protecting you the moment `Cargo.toml` exists.** `release.yml`'s plan step short-circuits on `if (-not (Test-Path Cargo.toml))` and emits an empty version, which skips every later step - so while Phase 0 was unstarted, documentation pushes to `main` stayed green and published nothing. The commit that created the workspace removed that protection in the same breath: with a `Cargo.toml` present and a `feat:` commit in history the planner computed a minor bump and the job walked on into signing, which threw on the unset `OT_MINISIGN_KEY`. Nothing was damaged, because the version bump and tag are local until a step *after* signing pushes them - confirm with `git ls-remote --tags` and the releases API rather than assuming. The lesson for the ordering: keep publish-affecting steps after everything that can fail, and have the signing key and `keys/minisign.pub` in place **before** the first `feat:` reaches `main`, or every push re-attempts the same version and fails.
- **A push that creates no workflow run at all is usually the forge, not your `on:` filter.** A branch pushed at 05:26 reached Forgejo - `git ls-remote` showed the commit on `refs/heads/<branch>` - but `/actions/tasks` still reported `total_count` 1 twenty minutes later. The `on.push` filter looked guilty (`branches: ['**', '!main', '!beta']` uses `!` negation, which not every Forgejo version honours), and it was innocent: the next branch push triggered `ci.yml` within seconds with the filter untouched. The runner log is what settles it - `C:\Apps\gitea-runner\service-err.log` had `fetching a task timed out after 5s` and `failed to fetch task: unavailable: 502 Bad Gateway` in exactly that window. Check three things before editing a workflow: the commit really is on the remote, the runner log for fetch errors, and `/api/v1/version` for the forge itself. A `000` from `curl` against the forge means the probe failed, not that the forge is down - repeated rapid probes get rate-limited by the proxy in front of it, while `git` over the same host keeps working.
- **An out-of-memory CI failure looks like a code failure.** It is reported as `could not compile <crate>`, and the crate named is simply whichever one was unlucky, not the one at fault. Before debugging the code, check whether anything else was running on the machine - a GUI app, a screenshot harness, another cargo - and re-run on a quiet machine.
- **A red release run after the publish step is a verification failure, not a failed release.** The release is live by then; the step that follows downloads it and runs the exe's `--self-test` *on the runner*, as LocalSystem in session 0, where a GUI or terminal probe can trip on things that work fine on the desktop. Re-verify the downloaded asset from the desktop before treating the release as broken. Flush each self-test line as it prints, give any probe a second and longer try, and turn a panic inside a check into a `FAILED` line - so a red verify names its reason instead of just stopping.
- **Tag and mirror.** The release job's last step fast-forwards GitHub to the version commit (never forced), pushes the tag, and dispatches GitHub's workflow. Never set a Forgejo push mirror while GitHub is writable.
- **Changes made on GitHub** come back with the sync script (fast-forward only).
- **Every GitHub job** is guarded with `if: github.server_url == 'https://github.com'`. Forgejo Actions also reads `.github/workflows/` and can't fetch published actions.
- **Release notes** are generated from commit subjects. Hand-edit only the Unreleased section.

## Patch Scripts and Text Editing
- **`grep -c` the markers after any re-run.** Checking "new text present" isn't enough when the new text starts with the old text. Make each patch step idempotent, and test for a marker that exists only *after* that step - re-running a half-applied patch has duplicated whole functions.
- **Unicode escapes in Python.** `\u{b7}` or `\u{2192}` in a Python string is an error ("truncated \uXXXX escape"). Put the character itself in the text, or build it with `chr()`. Doubling the backslash writes a literal `\\u` into Rust, which compiles to the wrong thing.
- **Windows paths in Python.** Use raw strings (`r"C:\Users\..."`). `"C:\Users"` fails on `\U`.
- **Windows paths in TOML.** Use single-quoted literal strings (`audio_path = 'C:\Users\...'`). A double-quoted TOML string processes `\U`, and an invalid escape fails the whole file. The symptom was only a "TOML parse error" and a setting that never appeared.
- **Heredocs.** Use `<<'EOF'` (quoted). An unquoted heredoc expands `` `backticks` `` (blanking names in doc comments) and `$name` (turning Fluent text into `Remove { }?`, a parse error that dropped the whole message). Better still, write the script to a file with the Write tool and run it.
- **Quoting accidents.** Two lost patches came from quoting, not logic. Write scripts to files.
- **Python on Windows writes CRLF.** `open(p, 'w')` translates `\n` to `\r\n`, so a patched LF file comes back with every line changed (git warns "CRLF will be replaced by LF"). Open with `newline=''` for reading and writing. Fix a damaged file with `sed -i 's/\r$//'`.
- **After a release, CHANGELOG's Unreleased has no subheadings.** The release commit resets it to `- (nothing yet)`, so a script that finds `### Fixed` (or any heading) by searching forward from `## Unreleased` lands in the version just released and files the line under it. Bound the search at the next `## ` heading, and replace the placeholder when it is there.
- **The Bash tool can refuse a long heredoc** ("unexpected EOF while looking for matching `'`") even when quoted, depending on the text. Nothing in it ran. Write the script to a file with the Write tool and run the file.

## Windows Environment and Build
- **Disk filled by `target/debug`.** Delete `target/debug/incremental` (cache only; `debug/deps` stays, and the next build isn't a full rebuild). Check after long sessions. Check free space *first* when a build misbehaves - a full disk does not announce itself, it shows up as `LNK1140: limit exceeded for program database` (the linker cannot write the PDB).
- **`LNK1140: limit exceeded for program database`.** The PDB outgrew its format limit. No code change causes or fixes it. Don't set `CARGO_PROFILE_DEV_DEBUG=0` on the command line. That changes every fingerprint, rebuilds the whole workspace, and fills drive space. If it recurs, set the debug level in `Cargo.toml` so the fingerprint is stable.
- **Paging file exhaustion.** `cargo test --workspace` can fail with "the paging file is too small" (os error 1455), then "found invalid metadata files for crate …" on every later run. The parallel build exhausts the commit limit, not RAM. Delete `target/debug` and use `-j 4`. A rustc killed mid-write leaves corrupt `.rmeta`/`.rlib` files.
- **Two cargo invocations on one target folder** serialise on the build lock.
- **A background `cargo test` started before a new crate is registered** fails on the missing module. Re-run after registering.
- **Release build fails with "Access is denied".** A running `OmniToaster.exe` keeps its own file locked. Run `Stop-Process -Name OmniToaster -Force` before `cargo build --release`. During development, always use `--data-dir ./tmp-data` so the real `OmniToasterData/` is never touched.

## Build and Dependency Gotchas (Rust)
- **CI runs `cargo deny check`, and its result does not depend on the branch.** Advisories come from the RustSec database and that moves on its own, so a branch whose `fmt`, `clippy`, tests and `--self-test` all passed yesterday can go red today with no change of its own. Run `cargo deny check` as part of the gate, and before a push if the branch has been open a while. The fix is usually `cargo update -p <crate>` for a patch bump - but check whether the code actually reaches the vulnerable path and say so either way, rather than implying a vulnerability shipped. The other CI step the local gate skips is the release build plus `--version` / `--self-test` on the exe.
- **Edition 2024 reserved keyword.** `gen` fails to parse ("expected identifier, found reserved keyword"), which broke 18 places from one field. Name it `generate`.
- **Items after a test module.** Appending functions to a file puts them after `#[cfg(test)] mod tests`, which clippy refuses ("items after a test module"). Insert before the test module. `cat >>` is safe only for files without one.

## Clippy and Lints
- **Don't let lints pile up.** Fix them, or use a scoped `#[allow(...)]` with a reason. Never a crate-wide allow.
- **Clippy must pass with `-D warnings`** on every crate, including Linux. Only `ot-platform` may hold Windows-only code; a platform dependency anywhere else breaks the Linux run.

## egui and Toast Rendering
Nothing recorded yet.

## Localization and Fluent
- **Edit `locales/**/*.ftl` with the Edit tool, never a heredoc.** An unquoted heredoc expands `` `backticks` `` and `$name`, which turns `Remove { $name }?` into `Remove { }?` - a Fluent parse error that silently drops the whole message. The only symptom is a "locale … has N syntax errors" line in the log and a failing key-parity or bundle-parse test. Quote the heredoc (`<<'EOF'`) if you must script it, or better, write the script to a file.
- **A key added to one locale only** is caught by the key-parity test, not by the compiler, and only if the test names the missing keys on each side. A parity test that just asserts the counts are equal passes happily when each file is missing a different key. Add every new key to **both** `.ftl` files in the same commit.

## Scheduler and Time
Nothing recorded yet.

## Prayer Times and JAKIM
Nothing recorded yet.

## Weather and HTTP
Nothing recorded yet.

## Audio and Speech
Nothing recorded yet.

## Tray, Startup and Supervisor
Nothing recorded yet.

## Storage and Migrations
Nothing recorded yet.

## Testing
Nothing recorded yet.
