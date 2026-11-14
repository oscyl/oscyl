# Contributing

Thank you for helping. This page covers setting up, running the checks,
updating screenshot goldens and translations, and sending a change. Project
roles and decisions are in [GOVERNANCE.md](GOVERNANCE.md), and everyone
follows the [Code of Conduct](CODE_OF_CONDUCT.md). Report security problems
privately, as described in [SECURITY.md](SECURITY.md).

## Set up

Every platform needs Rust 1.92 or newer, through `rustup`.

- **Linux app and CLI:** GTK 4.20+, libadwaita 1.8+, ALSA (`alsa-lib`) and
  gettext, with their development packages. Then `cargo run -p oscyl-gtk`.
- **Android app:**
  - a JDK 21 and the Android SDK, with `ANDROID_HOME` set;
  - NDK r30 (`ndk;30.0.16248370`);
  - `cargo install cargo-ndk`;
  - `rustup target add aarch64-linux-android x86_64-linux-android`.

  Then `cd apps/mobile && ./gradlew :androidApp:assembleDebug`. See
  [docs/mobile/README.md](docs/mobile/README.md).
- **iOS app:** a Mac with Xcode, plus
  `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`, then open
  `apps/mobile/iosApp/iosApp.xcodeproj`. On Linux you can still check the
  iOS code that builds there (see below).
- **Optional tools** that `scripts/ci.sh` uses when they are installed
  (Ruby too, for the issue forms):
  - [actionlint](https://github.com/rhysd/actionlint);
  - [zizmor](https://github.com/zizmorcore/zizmor);
  - [typos](https://github.com/crate-ci/typos);
  - [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
    (`cargo install cargo-deny --locked`).

## Run the checks

`scripts/ci.sh` runs what CI runs, in this order, and stops at the first
failure. It runs only the stages whose files changed since its last green
run (or since the branch left `main`), so a Kotlin-only change skips the Rust
stages. `scripts/ci.sh --all` runs everything:

1. Workflow lint (actionlint, zizmor), the issue forms (with Ruby), and
   spelling (typos).
2. The release scripts: the notes test, every changelog section and its
   compare link, and the version parser's test (`scripts/version-test.sh`).
3. `cargo fmt --check`, then clippy with warnings as errors.
4. The workspace tests, the build without midir, and each driver's
   conformance test.
5. The docs, with warnings as errors.
6. The FFI bindings.
7. What Linux can prove for iOS: clippy for the iOS targets, and the
   project files and assets (`scripts/mobile-assets.sh --check`).
8. The licence pages.
9. The mobile app: unit tests, screenshot goldens, the debug build and the
   iOS platform klibs.
10. The accessibility checks on an emulator.
11. `cargo deny`.

The heavy stages skip themselves when their tools are missing, and say so:

- No `ANDROID_HOME`: no mobile app (stages 9 and 10). To skip them on
  purpose, run `env -u ANDROID_HOME scripts/ci.sh`.
- No usable `/dev/kvm`: no accessibility checks (stage 10).
- No iOS Rust targets: no iOS clippy.
- No cargo-ndk or no NDK: only the bindings in the FFI stage.

For a faster loop while working, run the part you touched, for example
`cargo test -p oscyl-app` or
`cd apps/mobile && ./gradlew :androidApp:testDebugUnitTest`.

Hardware tests are marked `#[ignore]`. Run them with
`cargo test -- --ignored` while the synth is connected.

## Screenshot goldens (mobile)

Every `@Preview` in the mobile app is a screenshot golden, under
`apps/mobile/androidApp/src/test/snapshots/`. The previews are the only
source of the app's screenshots.

1. Change the UI and its previews.
2. Record: `cd apps/mobile && ./gradlew :androidApp:recordRoborazziDebug`.
3. Look at every golden that changed (`git status`), in each theme, size,
   font scale, mirrored and in the pseudo-locale. A golden nobody looked at
   is a bug waiting.
4. Verify, as CI does: `./gradlew :androidApp:verifyRoborazziDebug`. When
   it fails, the comparison images are in
   `apps/mobile/androidApp/build/outputs/roborazzi/`.

For the Linux app, design screenshots come from the app itself:
`cargo run -p oscyl-gtk -- --demo demo:filter:edited --screenshot shot.png`.
Other scripted states are listed at `run_demo` in
`apps/linux/src/onboarding.rs`.

## Translations

- **Desktop and core words** (the Linux app, the drivers' labels and help,
  and the core's messages, which the mobile apps show too) use gettext:
  `po/oscyl.pot` and `po/<lang>.po`. Regenerate the template with
  `cargo run -p oscyl-gtk -- --write-pot po/oscyl.pot`; a test fails when
  it is stale. See [po/README.md](po/README.md).
- **The mobile app's own words** are Compose resources:
  `apps/mobile/shared/src/commonMain/composeResources/values*/strings.xml`.
  Every English key needs one in each language (`StringsCompleteTest`).
- **Italian** is the second language, and every new string needs it. Ask
  for a review from someone who reads Italian when you are not sure, and
  say so in the pull request.

## Send a change

- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
  with a scope, for example `feat(core): add program model`.
- Versions follow [Semantic Versioning](https://semver.org/).
- **The changelog rule:** every user-visible change adds a line to
  `CHANGELOG.md` under `## [Unreleased]`, written for users (what they can
  now do, not how the code changed).
- Never add vendor firmware, factory presets, librarian binaries or
  content scraped from third-party sites. Test fixtures must be synthetic
  or written by you.

## Add a synth

Read the [driver guide](docs/drivers.md). It covers the contract, the
simulator, the conformance suite and
hardware validation for each tier. The acceptance checklist is under
"Driver acceptance" in [GOVERNANCE.md](GOVERNANCE.md). To ask for a synth
without writing the driver, open a "New synth request" issue.

## RFCs

Changes to the driver contract, changes that touch more than one driver,
and changes to the `.oscyllib` format start with an RFC. Copy
`docs/rfcs/0000-template.md` and follow
[docs/rfcs/README.md](docs/rfcs/README.md). Decisions already made are
recorded in [docs/adr/](docs/adr/README.md).

## Releases

Maintainers release by pushing a signed tag. The steps are in
[docs/releasing.md](docs/releasing.md).

## Licence

Unless you state otherwise, any contribution you submit is dual-licensed as
MIT OR Apache-2.0, without additional terms.
