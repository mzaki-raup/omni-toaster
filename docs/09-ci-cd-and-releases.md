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

`cargo deny` is in the **local** gate too, not only in CI, because its
advisories come from the RustSec database and move on their own: a branch that
was green yesterday can fail today with no change of its own.

## Versioning

Semantic versioning. The version lives **once**, under `[workspace.package]` in
the root `Cargo.toml`, and is moved only by `scripts/update-version.ps1` —
never by hand. The release pipeline decides the number from the Conventional
Commit types since the previous tag:

| Commit type | Bump |
|---|---|
| `feat:` | minor |
| `fix:`, `perf:`, `security:`, `refactor:`, `build:` | patch |
| `!` or a `BREAKING CHANGE:` footer | major |
| `docs:`, `chore:`, `test:`, `ci:`, `style:` | none — a docs-only push to `main` triggers no release |

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
| `OmniToaster-<version>-windows-x64.zip` | Manual update: exe + `README.txt` + `LICENSE` + `THIRD_PARTY_NOTICES.md` + notes |
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
