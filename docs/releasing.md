# Releasing

A release is a signed tag `vX.Y.Z` on `main`. Pushing it runs
`.github/workflows/release.yml`, which builds every artifact, writes
`SHA256SUMS`, attests the build provenance, and opens a **draft** GitHub
Release whose notes are the version's `CHANGELOG.md` section. Nothing is
public until a maintainer publishes the draft.

## Checklist

1. **Hardware validation.** Every Tier 1 driver's maintainer has validated
   it on real hardware for this release ([GOVERNANCE.md](../GOVERNANCE.md)).
   Note any driver that missed it in the changelog.
2. **Bump the version** in a release PR:
   - `Cargo.toml`, `[workspace.package] version` (then `cargo check` to
     update `Cargo.lock`). It must be `X.Y.Z` or `X.Y.Z-rc.N`, with Y and Z
     under 100 and N from 1 to 98. `scripts/version.sh` is the one parser:
     it prints the version, iOS's numeric `marketing` version and the build
     `code`, which is ((X×100 + Y)×100 + Z)×100 + N for an rc, or + 99 for
     a final. So 0.4.0-rc.1 is 40001 and 0.4.0 is 40099: every rc gets its
     own store build, below its final. The Android app takes its
     `versionName` and `versionCode` from it at build time.
   - `apps/mobile/iosApp/Configuration/Config.xcconfig`: set
     `MARKETING_VERSION` to the `marketing` line and
     `CURRENT_PROJECT_VERSION` to the `code` line of `scripts/version.sh`.
     `scripts/release-check.sh` checks both.
   - `apps/linux/data/io.github.oscyl.Oscyl.metainfo.xml`: a new `<release>`
     first in `<releases>`.
3. **Changelog.** Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`, start
   a fresh empty `## [Unreleased]` above it, and update the compare links at
   the bottom (`[Unreleased]` compares `vX.Y.Z...HEAD`; add
   `[X.Y.Z]: …/compare/vPREV...vX.Y.Z`). Edit the prose for users: it is the
   release note.
4. **Check** with `scripts/release-check.sh X.Y.Z` (versions agree, the
   section exists) and `scripts/release-notes.sh X.Y.Z` (what the release
   will say). Merge the PR once CI is green.
5. **Dry run** (optional): run the Release workflow by hand on `main`
   (Actions → Release → Run workflow). It builds and attests everything and
   publishes nothing; the artifacts are on the run.
6. **Tag** the merge commit, signed, and push the tag:

   ```sh
   git switch main && git pull --ff-only
   git tag -s vX.Y.Z -m "Oscyl X.Y.Z"
   git tag -v vX.Y.Z   # check the signature before pushing
   git push origin vX.Y.Z
   ```

   A version with a pre-release suffix (`v1.0.0-rc.1`) becomes a
   pre-release.
7. **Approve** the `release` environment when the workflow asks (the Android
   signing and the publish job wait for it).
8. **Review the draft**: notes, assets, `SHA256SUMS`. Then publish it.

## What the workflow builds

| Artifact | Notes |
| --- | --- |
| `oscyl-X.Y.Z-{x86_64,aarch64}-unknown-linux-gnu.tar.gz` | The `oscyl` CLI for Linux (ALSA). No musl build: it would need a static libasound. Built on Ubuntu 24.04 (glibc 2.39 or newer). |
| `oscyl-X.Y.Z-{aarch64,x86_64}-apple-darwin.tar.gz` | The `oscyl` CLI for macOS (CoreMIDI). Not signed or notarised. |
| `oscyl-X.Y.Z-x86_64-pc-windows-msvc.zip` | The `oscyl` CLI for Windows (WinMM). Not signed. |
| `oscyl-X.Y.Z-desktop-windows-x86_64.zip` | The desktop app for Windows: a folder with `bin\Oscyl.exe` and the MSYS2 UCRT64 DLLs, data and licence texts it needs. Not signed. |
| `oscyl-X.Y.Z-desktop-macos-{aarch64,x86_64}.zip` | The desktop app for macOS: `Oscyl.app` with Homebrew's libraries inside, signed ad hoc, not notarised. Needs the macOS version of the runner that built it, or newer. |
| `oscyl-X.Y.Z-x86_64.flatpak` | The GNOME app, GNOME 49 runtime (installs it from Flathub). |
| `oscyl-X.Y.Z.apk`, `.aab` | Android, signed when the keystore secrets exist; otherwise `-unsigned` in the name. |
| `oscyl-X.Y.Z.tar.gz` | Source (`git archive` of the tag). |
| `SHA256SUMS` | Over all of the above. |

Each artifact has a SLSA build-provenance attestation. Check one with
`gh attestation verify <file> --repo oscyl/oscyl`, and the sums with
`sha256sum -c SHA256SUMS --ignore-missing`.

The workflow refuses a tag that is not on `main`, whose version differs
from the one in `Cargo.toml`, the Android app or the metainfo, or that has
no dated changelog section.

## Setting up (once)

- **Environment `release`** (Settings → Environments): required reviewers
  (the maintainers), deployment refs limited to tags `v*.*.*` and the `main` branch (for dry runs).
- **Android signing secrets**, as secrets of the `release` environment:
  - `ANDROID_KEYSTORE_BASE64`: `base64 -w0 release.jks`;
  - `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`.

  Keep the keystore and its passwords offline as well: a lost upload key
  needs a Play reset, and installed APKs only update from the same key.

## Last dry run

**0.4.0, 2026-11-13**, on an x86_64 Linux workstation.
Every Linux release build was run by hand, the way the workflow runs it.
Nothing was tagged, signed or published.

| Artifact | Result |
| --- | --- |
| `oscyl-0.4.0-x86_64-unknown-linux-gnu.tar.gz` | `cargo build --release --locked -p oscyl-cli`, 12 s warm. The binary reports `oscyl-cli 0.4.0`. |
| `oscyl-0.4.0-x86_64.flatpak` | `scripts/flatpak-sources.sh`, then flatpak-builder (the `org.flatpak.Builder` app) on GNOME 49 with rust-stable 25.08, and `flatpak build-bundle`. 1 min 22 s with warm sources. |
| `oscyl-0.4.0-unsigned.apk`, `.aab` | `./gradlew -PreleaseBuild=true :androidApp:assembleRelease :androidApp:bundleRelease`, 1 min 7 s. `aapt2` reads `versionName='0.4.0'` and, after the version scheme was settled, a rebuild reads `versionCode='40099'`. There is no `en-XA` pseudo-locale. |
| `oscyl-0.4.0.tar.gz` | `git archive --prefix=oscyl-0.4.0/`. Its `Cargo.toml` says 0.4.0. |
| `SHA256SUMS` | `sha256sum -- *` over the five files; `sha256sum -c` passes. |

The checks passed: `scripts/release-check.sh 0.4.0` says the versions and
the changelog agree, `scripts/release-notes.sh 0.4.0` prints the section,
and `appstreamcli validate --no-net` passes the metainfo.

Not covered by this dry run:
- the `aarch64-unknown-linux-gnu` CLI, which the workflow builds on an ARM
  runner;
- Android signing, since no keystore was used;
- attestations;
- the draft release itself.

Those run only in the Release workflow.
