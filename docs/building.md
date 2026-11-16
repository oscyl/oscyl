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

### macOS (desktop app)

With [Homebrew](https://brew.sh):

```sh
brew install pkgconf gtk4 libadwaita gettext adwaita-icon-theme
GETTEXT_DIR="$(brew --prefix gettext)" cargo run -p oscyl-gtk -- --simulator
```

To make `Oscyl.app` (also `dylibbundler` and `librsvg`):

```sh
GETTEXT_DIR="$(brew --prefix gettext)" cargo build --release -p oscyl-gtk
packaging/desktop/bundle-macos.sh target/release/oscyl-gtk 0.0.0 dist
```

The release's `Oscyl.app` is signed ad hoc but not notarised, so macOS
refuses to open it the first time. Open it once from Finder with
Control-click, Open, then Open in the dialog (on macOS 15 and later: try to
open it, then System Settings, Privacy & Security, Open Anyway). Or clear
the quarantine flag: `xattr -dr com.apple.quarantine Oscyl.app`.

There is no hotplug on macOS yet: a synth plugged in after Oscyl started
connects when you choose Connect.

### Windows (desktop app)

In an [MSYS2](https://www.msys2.org) UCRT64 shell:

```sh
pacman -S --needed mingw-w64-ucrt-x86_64-{toolchain,rust,pkgconf,gtk4,libadwaita,gettext-tools}
cargo run -p oscyl-gtk -- --simulator
```

To make the folder the release ships (`bin/Oscyl.exe` runs it):

```sh
cargo build --release -p oscyl-gtk
packaging/desktop/bundle-windows.sh target/release/oscyl-gtk.exe "$MSYSTEM_PREFIX" dist/Oscyl
```

From Linux, `cargo build -p oscyl-gtk --target x86_64-pc-windows-gnu` also
works with `mingw-w64-gcc` and the same MSYS2 packages unpacked into a
directory (`PKG_CONFIG_SYSROOT_DIR`, `PKG_CONFIG_LIBDIR` and
`MSYSTEM_PREFIX` pointing into it, `PKG_CONFIG_ALLOW_CROSS=1`).

The app is not signed, so SmartScreen may warn on first launch: More info,
then Run anyway. As on macOS, a synth plugged in later connects when you
choose Connect.

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
