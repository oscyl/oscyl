# Release artifacts

What the release workflow, `.github/workflows/release.yml`, builds and
checks. [Cut a release](../how-to/release.md) says how to start it.

## What a release holds

| Artifact | Notes |
| --- | --- |
| `oscyl-X.Y.Z-{x86_64,aarch64}-unknown-linux-gnu.tar.gz` | The `oscyl` command line for Linux (ALSA). Built on Ubuntu 24.04, so it needs glibc 2.39 or newer. There is no musl build: it would need a static libasound. |
| `oscyl-X.Y.Z-{aarch64,x86_64}-apple-darwin.tar.gz` | The `oscyl` command line for macOS (CoreMIDI). Not signed or notarised. |
| `oscyl-X.Y.Z-x86_64-pc-windows-msvc.zip` | The `oscyl` command line for Windows (WinMM). Not signed. |
| `oscyl-X.Y.Z-desktop-windows-x86_64.zip` | The desktop app for Windows: a folder with `bin\Oscyl.exe`, and the MSYS2 UCRT64 DLLs, data and licence texts that it needs. Not signed. |
| `oscyl-X.Y.Z-desktop-macos-{aarch64,x86_64}.zip` | The desktop app for macOS: `Oscyl.app`, with Homebrew's libraries inside. Signed ad hoc, not notarised. It needs the macOS version of the runner that built it, or newer. |
| `oscyl-X.Y.Z-x86_64.flatpak` | The GNOME app on the GNOME 49 runtime, which installs from Flathub. |
| `oscyl-X.Y.Z.apk`, `.aab` | The Android app, signed when the keystore secrets exist. Otherwise `-unsigned` is in the name. |
| `oscyl-X.Y.Z.tar.gz` | The source (`git archive` of the tag). |
| `SHA256SUMS` | The checksums of all of the above. |

Each artifact has a SLSA build-provenance attestation. To check a file and
the sums:

```sh
gh attestation verify <file> --repo oscyl/oscyl
sha256sum -c SHA256SUMS --ignore-missing
```

## What the workflow refuses

The workflow refuses a tag when:

- the tag is not on `main`;
- its version differs from the one in `Cargo.toml`, in the Android app or
  in the metainfo;
- `CHANGELOG.md` has no dated section for the version.

## Last dry run

**0.4.0, 2026-11-13**, on an x86_64 Linux workstation. Every Linux release
build ran by hand, as the workflow runs it. Nothing was tagged, signed or
published.

| Artifact | Result |
| --- | --- |
| `oscyl-0.4.0-x86_64-unknown-linux-gnu.tar.gz` | `cargo build --release --locked -p oscyl-cli`, 12 s warm. The binary reports `oscyl-cli 0.4.0`. |
| `oscyl-0.4.0-x86_64.flatpak` | `scripts/flatpak-sources.sh`, then flatpak-builder (the `org.flatpak.Builder` app) on GNOME 49 with rust-stable 25.08, and `flatpak build-bundle`. 1 min 22 s with warm sources. |
| `oscyl-0.4.0-unsigned.apk`, `.aab` | `./gradlew -PreleaseBuild=true :androidApp:assembleRelease :androidApp:bundleRelease`, 1 min 7 s. `aapt2` reads `versionName='0.4.0'`. After the version scheme was settled, a rebuild reads `versionCode='40099'`. There is no `en-XA` pseudo-locale. |
| `oscyl-0.4.0.tar.gz` | `git archive --prefix=oscyl-0.4.0/`. Its `Cargo.toml` says 0.4.0. |
| `SHA256SUMS` | `sha256sum -- *` over the five files. `sha256sum -c` passes. |

The checks passed: `scripts/release-check.sh 0.4.0` says that the versions
and the changelog agree, `scripts/release-notes.sh 0.4.0` prints the
section, and `appstreamcli validate --no-net` passes the metainfo.

The dry run did not cover:

- the `aarch64-unknown-linux-gnu` command line, which the workflow builds on
  an ARM runner;
- Android signing, because it used no keystore;
- the attestations;
- the draft release itself.

Only the release workflow runs those.
