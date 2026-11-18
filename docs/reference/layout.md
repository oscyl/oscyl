# Repository layout

The top-level folders and files of the repository, and what each one holds.
[A tour of the code](../start/code-tour.md) says how the crates work
together.

## Crates

| Path | What it holds |
|---|---|
| `crates/oscyl-driver` | The driver contract, the shared SysEx helpers and the request engine. |
| `crates/drivers/` | One crate for each synth model: its protocol, program model, files and simulator. Also `logue`, which the KORG logue drivers share, and `tutorial-synth`, the driver of the tutorial "Your first driver". |
| `crates/oscyl-conformance` | The checks that every driver must pass. |
| `crates/oscyl-core` | The `.oscyllib` library format, and the parts of Generate that no driver owns. |
| `crates/oscyl-midi` | The transports (midir on the desktop, or bytes that the platform moves) and `DRIVERS`, the drivers that the apps know. |
| `crates/oscyl-app` | The app logic that every app shares: documents, libraries, settings, the synth worker, the editor, Generate, translations. |
| `crates/oscyl-ffi` | The UniFFI bindings over `oscyl-app`, for the mobile apps. |
| `crates/oscyl-cli` | The command line tool, `oscyl`. |
| `crates/oscyl-osc-host` | Tests of the oscillator unit: its C, compiled for the computer. |

## Apps

| Path | What it holds |
|---|---|
| `apps/linux` | The desktop app (`oscyl-gtk`), for Linux, Windows and macOS. Its desktop entry, AppStream metadata, icons and fonts are in `apps/linux/data/`. |
| `apps/mobile/shared` | The mobile apps' UI and the bindings, in common Kotlin. |
| `apps/mobile/androidApp` | The Android entry, USB-MIDI and the manifest. |
| `apps/mobile/iosApp` | The Xcode project of the iOS app. |
| `apps/mobile/iosPlatform` | The iOS platform code that needs no Rust core: CoreMIDI, files, haptics, storage and language. |
| `apps/mobile/baselineprofile` | The Baseline Profile generator and the frame benchmarks. |

## Everything else

| Path | What it holds |
|---|---|
| `units/oscyl-osc` | The oscillator unit: its C source, KORG's logue SDK files and the prebuilt payloads. |
| `docs/` | This book. `docs/images/` holds the pictures of the README and the website. |
| `site/` | The website's landing page. |
| `design/tokens.json` | The design tokens: every colour, shape, spacing and motion value of both apps. |
| `DESIGN.md` | The design system. |
| `po/` | The gettext translations. |
| `packaging/` | The Flatpak manifest, the bundle scripts for Windows and macOS, and the install notes. |
| `scripts/` | The local checks (`ci.sh`) and the build, release and asset scripts. |
| `fuzz/` | The fuzz targets of the parsers. |
| `.github/` | The workflows, the issue forms, the pull request template and the Vale styles. |
| `.vale.ini`, `lychee.toml` | The settings of the prose check and of the link check. |
| `_typos.toml`, `deny.toml`, `about.toml` | The settings of the spelling check, of cargo-deny and of cargo-about. |
