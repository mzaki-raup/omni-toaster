# Spec — In-app updater

**Status:** approved · **Last updated:** 2026-11-08 · **ADR:** [ADR-010](../08-decisions.md)
**Owner crates:** `ot-core`, `ot-platform`, `ot-ui` · **Phase:** 9

## 1. Purpose

The app is portable: no installer, no package manager, nothing that would
update it from outside. The owner requires an in-app updater rather than asking
users to re-download a zip, so the updater is a **required Phase 9
deliverable**, not a nice-to-have.

## 2. User stories

- As **anyone**, I want the app to tell me a new version exists and to install
  it for me.
- As **the tinkerer**, I want an update to leave my `OmniToasterData/` folder
  untouched.
- As **anyone**, I want a failed update to leave me with a working app, not a
  half-replaced one.

## 3. The asset contract

Every release publishes exactly these, and the updater reads them **by name**
([09-ci-cd-and-releases.md](../09-ci-cd-and-releases.md)):

| Asset | The updater uses it for |
|---|---|
| `manifest.json` | The version, the channel, `min_supported_version`, `data_schema`, and each asset's size and SHA-256 |
| `manifest.json.minisig` | The minisign signature over `manifest.json` — checked **first**, against the public key embedded in the exe (`keys/minisign.pub`) |
| `OmniToaster.exe` | The bare executable it downloads and stages (the manifest's `assets["windows-x64"]`) |
| `OmniToaster-<version>-windows-x64.zip` | Not used by the updater; the manual path |
| `SHA256SUMS.txt` | Not used by the updater; the manual path |
| `CHANGELOG-<version>.md` | Not fetched; `notes_url` in the manifest is what the UI links to |

A release missing the first three reads as **"no installable release"** and the
user stays where they are. Renaming an asset is a breaking change needing a new
ADR.

## 4. The flow

```
1. Check      GET manifest.json for the channel          (on a schedule, or on demand)
2. Compare    manifest version vs the running version    (semver; never "newer string")
3. Verify     rsign signature over manifest.json          → fail closed
4. Download   OmniToaster.exe to  <data>/cache/update/    → size cap from the manifest
5. Verify     SHA-256 against the manifest                → fail closed, delete the file
6. Stage      write it beside the running exe as OmniToaster.exe.new
7. Hand over  the supervisor swaps and restarts the worker
8. Confirm    the new worker reports its version; on failure, roll back
```

**Fail closed at every verification step.** A signature or hash mismatch
deletes the download, logs the reason, and tells the user the update was
rejected — it never falls back to installing it anyway.

## 5. Why the supervisor does the swap

A running exe cannot replace itself on Windows — the file is locked while the
process runs, which is the same reason a release build fails with "Access is
denied" while an instance is open ([GOTCHAS.md](../GOTCHAS.md)). So:

- The **worker** downloads, verifies and stages `OmniToaster.exe.new`.
- It then asks the **supervisor** to apply it.
- The supervisor stops the worker, renames the current exe to
  `OmniToaster.exe.old`, renames `.new` into place, and starts the new worker.
- If the new worker fails to report its version within a timeout, the
  supervisor restores `.old` and restarts, then tells the user the update was
  rolled back.
- `.old` is deleted on the first successful run of the new version.

This makes the supervisor — which already exists for crash recovery
([specs/supervisor.md](supervisor.md)) — the only thing that touches the
executable.

## 6. What an update must never do

- Touch `OmniToasterData/` beyond `cache/update/`. Settings, reminders, the
  database, imported media and logs survive every update untouched.
- Run a migration. Migrations are the app's job on next start
  ([05-data-model-and-storage.md](../05-data-model-and-storage.md)), not the
  updater's.
- Require elevation. The folder is the user's; if it is not writable, the
  updater says so and offers the manual zip instead.
- Update silently. The user is told what version is being installed, and asked,
  unless they have turned on automatic updates.

## 7. Settings

Settings ▸ Logs and Updates: the current version, the channel (stable ·
prerelease), **Check Now**, the last check time and result, automatic updates
on or off, and a link to the release notes. The whole check can be turned off,
which stops the only outbound request this feature makes
([06-security-and-privacy.md](../06-security-and-privacy.md)).

## 8. Network behaviour

HTTPS only, a timeout, a response size cap, retry with backoff, and no polling
loop — a daily check at most, on a worker thread with blocking `ureq`, and
cancellable. The request sends the current version and channel and nothing
else.

## 9. Fluent keys

Prefix `update-`: available · up to date · checking · downloading with
progress · verifying · rejected (signature) · rejected (hash) · staged and
awaiting restart · rolled back · folder not writable · check disabled. Every
one in both languages, with sentence case
([10-i18n.md](../10-i18n.md)).

## 10. Verification

| Check | Asserts |
|---|---|
| Version comparison | semver ordering, including prereleases; an equal or older manifest offers nothing |
| Signature | a tampered `manifest.json` is rejected and nothing is downloaded |
| Hash | a corrupted download is rejected and deleted |
| Size cap | a response larger than the manifest claims is abandoned |
| Staging | `.new`, `.old` and the swap sequence, with the data folder untouched |
| Rollback | a new worker that fails to report its version is rolled back |
| Not writable | a read-only folder produces the manual-zip message, not a crash |
| Disabled | no outbound request is made when the check is off |

Then against a **real release** (Phase 9's exit criterion): the updater moves a
genuine published release forward, verifies its signature, and the data folder
is byte-identical afterwards except for `cache/update/`.

```bash
pwsh -File scripts/verify-release.ps1 -Tag v0.1.0
```

## 11. Open questions

1. ~~Whether the rsign public key is embedded in the exe or read from a file
   beside it.~~ **Settled:** embedded, and the same key committed as
   `keys/minisign.pub` for manual verification. A file beside the exe would be
   attacker-writable in a portable folder. `keys/README.md` covers rotation:
   ship the new public key one release early, signed with the old key.
2. Whether a prerelease channel is worth shipping in v1.0 or can wait. The
   `beta` channel already exists in the manifest and in
   `scripts/build-and-package.ps1`; the open part is only whether the UI
   exposes it.
3. ~~Whether the manifest should carry a minimum-version floor.~~ **Settled:**
   it does — `min_supported_version`, defaulting to `0.1.0`, bumped only when
   the updater protocol changes. An older copy is sent to the zip.
4. What `data_schema` in the manifest should make the updater do. It records
   the on-disk format version the build writes
   ([05-data-model-and-storage.md](../05-data-model-and-storage.md)); whether a
   forward jump should warn before updating is undecided.

## What is implemented (2026-11-08)

Nothing. Phase 9 has not started; this file is the requirement.
