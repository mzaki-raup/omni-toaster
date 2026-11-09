# 09 — CI/CD and releases

**Status:** approved · **Last updated:** 2026-11-08 · **Source:** [PROPOSAL.md](../PROPOSAL.md) CI section, [ADR-010](08-decisions.md), [ADR-012](08-decisions.md)

## Two forges

| Remote | Is | Owns |
|---|---|---|
| `origin` — `git.zacx.net` (Forgejo) | Authoritative | The version number, `main`, and every published release |
| `github` — `github.com/mzaki-raup/omni-toaster` | Public mirror | Nothing. Fast-forwarded from Forgejo |

`scripts/push-to-mirror.ps1` pushes the mirror; `scripts/sync-from-github.ps1`
brings changes made on GitHub back, fast-forward only. Never set a Forgejo push
mirror while GitHub is writable.

**The mirror needs two secrets on the Forgejo instance**, and GitHub needs
`OT_MINISIGN_KEY` of its own to sign.

| Secret | Value |
|---|---|
| `OT_GITHUB_MIRROR_URL` | `https://github.com/mzaki-raup/omni-toaster.git` — a plain HTTPS clone URL with no credentials in it. The `.git` suffix and a trailing slash are both optional |
| `OT_GITHUB_MIRROR_TOKEN` | A GitHub PAT for the account that owns the mirror. **Fine-grained:** this repository listed under Repository access (they grant nothing by default), with Contents, Workflows and Actions all **read/write**. **Classic:** `repo` **and** `workflow` |

The Workflows permission is not optional padding: the pushed history can touch
`.github/workflows/`, and GitHub refuses such a push without it. Actions is
needed because the script dispatches the mirror's release run — a tag push
alone will not start it, since the tagged commit is Forgejo's
`chore(release): vX.Y.Z [skip ci]` and GitHub honours that marker.

A **403** on the tag push means the URL is right and the token is short of
permissions; a **404** would mean the URL is wrong. Without them the handoff step **skips and exits 0**, so the release goes
green having mirrored nothing — v0.1.0 through v0.3.0 reached Forgejo while
GitHub got no tags and no releases at all (BUG-0002).

That failure hides well, because `origin` carries GitHub as a second *push
url*: an ordinary `git push origin main` updates both forges, so `main` looks
current on the mirror. A plain push only moves the refs it is given, never
tags — and `.github/workflows/release.yml` triggers on `push: tags: ['v*']`,
so no tag means it has never run.

**The Forgejo runner is the owner's own Windows machine.** Two consequences
that are not negotiable: every whole-workspace build on it passes `-j 4`, and
nobody runs a local `cargo` command while a job is running — check the forge's
`/actions/tasks?limit=4` first. The `.github/` workflows run on GitHub-hosted
machines and are deliberately left unbounded; their jobs are guarded with
`if: github.server_url == 'https://github.com'` so Forgejo Actions, which also
reads `.github/workflows/`, does not try to run them.

## Branch flow

1. Work on a branch.
2. Push it.
3. Wait for `ci.yml` to go green.
4. Merge into `main` and push.

Never merge red. `ci.yml` runs on every branch except `main` and `beta`;
`release.yml` owns `main`, because the single runner empties its working copy
on entry and two workflows on one commit would destroy each other's files.
`main` is only reached through a branch `ci.yml` has checked, and `release.yml`
re-runs the same checks before publishing anything.

## The pipeline

`ci.yml`, in order:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings`
3. `cargo test --workspace --locked -j 4`
4. Locale key parity: `en-US` and `ms-MY` must have identical key sets
5. The hard-coded-string lint: no user-facing literal outside a Fluent call
6. `cargo deny check` (and `cargo audit`)
7. Conflict-marker grep over the tracked text files
8. Release build, then `--version` and `--self-test` on the built exe

Until Phase 0 creates the workspace, the cargo steps are skipped by a
`has_cargo` guard and the job passes after checking that the scripts parse.

`--self-test` also asserts the portability rule: nothing named `OmniToaster*`
appeared under `%APPDATA%`, `%LOCALAPPDATA%` or `%TEMP%` during the run
([ADR-004](08-decisions.md)).

### What `--self-test` prints

One line per check, **flushed as it prints**, and exit code 0 only when every
line said `ok`. Six checks as of Phase 0: paths resolve and are writable, the
log file is written, both locale bundles parse, locale key parity holds, the
database migrates, and the date formatter answers in both languages.

The check's **name is printed before the check runs** and the verdict after it:

```
paths resolve and are writable ... ok
locale key parity holds ... FAILED: ms-MY is missing 2: help-usage, help-footer
```

That ordering is the contract, and it replaces an earlier plan to catch a
panic inside a check and turn it into a `FAILED` line. The release profile sets
`panic = "abort"` so the supervisor sees a non-zero exit rather than a
half-unwound process ([03-architecture.md](03-architecture.md)), and
`catch_unwind` cannot catch what does not unwind. Printing the name first gets
the same diagnostic value a different way: an abort leaves the dying check's
name as the last line of output. A buffered self-test that died mid-check would
name nothing at all, which [GOTCHAS.md](GOTCHAS.md) records as a lesson already
paid for.

A failing check does not stop the others, because a self-test that stops at the
first problem hides the rest.

`cargo deny` is in the **local** gate too, not only in CI, because its
advisories come from the RustSec database and move on their own: a branch that
was green yesterday can fail today with no change of its own.

## Versioning

Semantic versioning. The version lives **once**, under `[workspace.package]` in
the root `Cargo.toml`, and is moved only by `scripts/update-version.ps1` —
never by hand. **Once** means no crate repeats it: a dependency on a sibling
crate is `{ path = "../ot-thing" }` and never carries a `version`, which a
version bump would leave behind — a pinned `^0.4.0` accepts every patch
release and then fails to resolve on the first minor one (BUG-0003).
`crates/omnitoaster/tests/manifests.rs` fails the gate on such a line.

The release pipeline decides the number from the Conventional Commit types
since the previous tag:

| Commit type | Bump | Appears in the notes under |
|---|---|---|
| `!` or a `BREAKING CHANGE:` footer | major | Added |
| `feat:` | minor | Added |
| `fix:` | patch | Fixed |
| `security:` | patch | Security |
| `perf:` | patch | Changed |
| `docs:`, `chore:`, `refactor:`, `test:`, `ci:`, `style:`, `build:` | none | **nothing — the commit is not listed at all** |

Taken from `$MinorTypes`, `$PatchTypes` and `$NoReleaseTypes` in
`scripts/next-release.ps1`, which is the implementation. An earlier version of
this table had `refactor:` and `build:` as patch bumps, which was simply wrong.

**The type decides whether a change is published *and* whether anyone is told
about it.** A no-release type is skipped entirely: no bump, and no line in the
release notes. That is right for genuine no-ops, and wrong the moment a commit
labelled `style:` or `refactor:` changes something a user can see — the change
then ships silently, carried by whatever releasable commit happens to be in the
same push. CR-0001 did exactly that: it renamed the product in every visible
string under a `style:` subject, so it shipped in v0.2.0 with no mention in
v0.2.0's notes. **A change to user-visible text is `fix:` or `feat:`, never
`style:`**, whatever the diff looks like.

Preview the next release with:

```bash
pwsh -File scripts/next-release.ps1
```

A **task version** in [TASKS.md](TASKS.md) is the owner's batch label and has
nothing to do with this number. Never rename one to match the other.

## The release asset contract

Every release publishes exactly these, and the in-app updater reads them by
name ([ADR-010](08-decisions.md), [specs/updater.md](specs/updater.md)):

| Asset | Is |
|---|---|
| `OmniToaster.exe` | The bare executable, what the updater downloads and stages |
| `OmniToaster-<version>-windows-x64.zip` | Manual update: exe + `README.txt` + `LICENSE` + notes, and `THIRD_PARTY_NOTICES.md` once that file exists — the packaging script copies it only if present, and the workspace has no third-party dependencies yet |
| `manifest.json` | Schema, product, version, channel, tag, `notes_url`, `min_supported_version`, `data_schema`, and each asset with its size and SHA-256 |
| `manifest.json.minisig` | The minisign (Ed25519) signature over `manifest.json` — the trust anchor, checked before the manifest is believed |
| `SHA256SUMS.txt` | Checksums in `sha256sum -c` layout, for manual verification |
| `CHANGELOG-<version>.md` | The release notes, also used as the release body |

The verifying half of the key is committed as `keys/minisign.pub` and embedded
in the app; `keys/README.md` covers generation, the `OT_MINISIGN_KEY` forge
secret and rotation. `scripts/build-and-package.ps1` produces this set and
fails if any member is missing; `scripts/verify-release.ps1` re-checks it
against a published release.

A release that lacks them reads to the app as "no installable release" and
users stay where they are. **Renaming an asset is a breaking change** and needs
a new ADR.

Releases publish automatically from `main`. Nobody tags or publishes by hand.
After a release, verify it the way the app does:

```bash
pwsh -File scripts/verify-release.ps1 -Tag v0.1.0
```

A red release run **after** "Published vX.Y.Z with N assets" is a verification
failure, not a failed release — the release is already live. Confirm with that
script before treating it as broken.

## Release notes

Generated from the Conventional Commit subjects since the previous tag. Only
the **Unreleased** section of [CHANGELOG.md](CHANGELOG.md) is hand-edited;
released sections belong to the workflow and are never edited by hand. Writing
Unreleased notes while a release lands guarantees a conflict — expect it, and
resolve it by keeping the generated released section as it is and leaving only
unreleased items under Unreleased. A conflict-marker grep must print nothing
before any merge commit:

```bash
grep -nE '^(<<<<<<<|=======|>>>>>>>)' docs/CHANGELOG.md
```

## Signing

Release binaries are code-signed where a certificate is available; an unsigned
exe triggers SmartScreen. Until a certificate exists, releases ship published
SHA-256 checksums and the README documents how to verify them. The rsign
signature over `manifest.json` is independent of code signing and is what the
updater actually checks.

The signing secret is `OT_MINISIGN_KEY` on both forges, and the build fails
loudly rather than publishing unsigned if it is unset. It must be the **two-line
unencrypted** minisign secret key: a key made with `rsign generate -W` prompts
for a password on the console, which on a runner hangs until the 60-second
timeout kills it. `keys/README.md` has the generation and rotation procedure.
