# Build and run

You need Rust 1.92 or newer (`rustup`) for every platform.

### Linux (GNOME app and CLI)

You need GTK 4.20+, libadwaita 1.8+, ALSA (`alsa-lib`) and gettext, with their
development packages.

```sh
cargo run -p oscyl-gtk                        # the desktop app
cargo run -p oscyl-gtk -- --simulator         # with a simulated synth
cargo run -p oscyl-cli -- --help              # the command-line tool
cargo run -p oscyl-cli -- --simulator info    # try it without hardware
```

Connect the synth over USB. Oscyl finds its port by name. For the Flatpak
build, see [packaging/flatpak/README.md](../packaging/flatpak/README.md).

### macOS and Windows (CLI)

Rust is all you need:

```sh
cargo run -p oscyl-cli -- --help
```

### Android

You need a JDK 21, the Android SDK (`ANDROID_HOME`) with NDK r30, `cargo
install cargo-ndk`, and `rustup target add aarch64-linux-android
x86_64-linux-android`. Gradle builds the Rust core itself.

```sh
cd apps/mobile
./gradlew :androidApp:assembleDebug
adb install androidApp/build/outputs/apk/debug/androidApp-debug.apk
```

On the emulator, choose the simulated synth on the welcome screen. On a phone,
plug the synth into the USB-C port through an OTG adapter or a USB-C to USB-B
cable. The app has not yet run with a synth on a real phone, so testers are
welcome. Details: [docs/mobile/README.md](mobile/README.md).

### iOS

You need a Mac with Xcode. Open `apps/mobile/iosApp/iosApp.xcodeproj` and run
it. The iOS app has not been run on a device yet, and its first macOS CI run
is still to come. The steps and signing notes are in
[docs/mobile/README.md](mobile/README.md#run-the-app-on-ios).

## Repository layout

- `crates/oscyl-driver`: the driver contract, shared SysEx helpers and the
  request engine.
- `crates/drivers/`: one crate per synth (protocol, program model, files,
  simulator), plus `logue`, shared by the KORG logue family.
- `crates/oscyl-conformance`: the checks every driver must pass.
- `crates/oscyl-core`: the native `.oscyllib` library format.
- `crates/oscyl-midi`: the transports (midir on desktop, or bytes the
  platform moves) and the registered drivers.
- `crates/oscyl-app`: app logic shared by every app: documents, libraries,
  settings, the synth worker, the editor, the generator, translations.
- `crates/oscyl-ffi`: UniFFI bindings over `oscyl-app` for the mobile apps.
- `crates/oscyl-cli`: the `oscyl` command-line tool.
- `apps/linux`: the GNOME app (`oscyl-gtk`).
- `apps/mobile`: the Compose Multiplatform app (`androidApp`, `iosApp`,
  `shared`, `iosPlatform`).
- `po/`: translations ([how to translate](../po/README.md)).
- `packaging/`: the Flatpak manifest and install notes (the desktop entry
  and AppStream metadata are in `apps/linux/data/`).
- `docs/`: guides, decisions, protocol notes and research
  ([index](README.md)).
