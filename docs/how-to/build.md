# Build and run Oscyl

This guide builds and runs the desktop app and the command line tool on
each system. For the Android and iOS apps, see
[Build, run and test the mobile apps](mobile.md).

Every system needs Rust 1.92 or newer, through [rustup](https://rustup.rs).

## Linux: the desktop app and the command line

1. Install the development packages of GTK 4.20 or newer, libadwaita 1.8
   or newer, ALSA (`alsa-lib`) and gettext.
2. Run the app or the command line:

   ```sh
   cargo run -p oscyl-gtk                        # the desktop app
   cargo run -p oscyl-gtk -- --simulator         # with simulated synths
   cargo run -p oscyl-cli -- --help              # the command line tool
   cargo run -p oscyl-cli -- --simulator info    # try it without a synth
   ```

3. Connect the synth over USB. Oscyl finds its port by name.

To build the Flatpak, see
[packaging/flatpak](https://github.com/oscyl/oscyl/blob/main/packaging/flatpak/README.md).

## macOS and Windows: the command line

Rust is all that you need:

```sh
cargo run -p oscyl-cli -- --help
```

## macOS: the desktop app

1. Install the libraries with [Homebrew](https://brew.sh):

   ```sh
   brew install pkgconf gtk4 libadwaita gettext adwaita-icon-theme
   ```

2. Run the app:

   ```sh
   GETTEXT_DIR="$(brew --prefix gettext)" cargo run -p oscyl-gtk -- --simulator
   ```

To make `Oscyl.app`, also install `dylibbundler` and `librsvg`, then run:

```sh
GETTEXT_DIR="$(brew --prefix gettext)" cargo build --release -p oscyl-gtk
packaging/desktop/bundle-macos.sh target/release/oscyl-gtk 0.0.0 dist
```

The release's `Oscyl.app` is signed ad hoc but not notarised, so macOS
refuses to open it the first time. Do one of these:

- In Finder, Control-click the app, choose Open, then choose Open in the
  dialog.
- On macOS 15 and later, try to open the app. Then go to System Settings,
  Privacy & Security, and choose Open Anyway.
- Clear the quarantine flag: `xattr -dr com.apple.quarantine Oscyl.app`.

macOS has no hotplug yet. If you plug the synth in after Oscyl starts,
choose Connect.

## Windows: the desktop app

1. In an [MSYS2](https://www.msys2.org) UCRT64 shell, install the
   toolchain and the libraries:

   ```sh
   pacman -S --needed mingw-w64-ucrt-x86_64-{toolchain,rust,pkgconf,gtk4,libadwaita,gettext-tools}
   ```

2. Run the app:

   ```sh
   cargo run -p oscyl-gtk -- --simulator
   ```

To make the folder that a release ships (`bin/Oscyl.exe` runs it):

```sh
cargo build --release -p oscyl-gtk
packaging/desktop/bundle-windows.sh target/release/oscyl-gtk.exe "$MSYSTEM_PREFIX" dist/Oscyl
```

To make the installer from that folder, install
`mingw-w64-ucrt-x86_64-nsis`, then run:

```sh
eval "$(scripts/version.sh)"
makensis -DVERSION="$version" -DNUMERIC="$marketing" -DSRC="$(cygpath -w "$PWD/dist/Oscyl")" \
    -DOUT="$(cygpath -w "$PWD/dist/oscyl-setup.exe")" packaging/desktop/oscyl.nsi
```

The installer is for the current user only, and needs no admin rights. It
puts Oscyl in `%LOCALAPPDATA%\Programs\Oscyl` and adds it to the Start
menu. To remove Oscyl, open Settings > Apps > Installed apps, choose Oscyl,
then choose Uninstall. Your settings and libraries stay.

You can also build the Windows app on Linux:

1. Install `mingw-w64-gcc`.
2. Unpack the same MSYS2 packages into a directory.
3. Point `PKG_CONFIG_SYSROOT_DIR`, `PKG_CONFIG_LIBDIR` and `MSYSTEM_PREFIX`
   into it, and set `PKG_CONFIG_ALLOW_CROSS=1`.
4. Run `cargo build -p oscyl-gtk --target x86_64-pc-windows-gnu`.

The installer and the app are not signed, so SmartScreen can warn the
first time you run one of them. Choose More info, then Run anyway. As on macOS, if you plug the synth
in after Oscyl starts, choose Connect.

To try the app on a Windows PC with a synth, from Linux, see
[Test on Windows hardware from Linux](windows-hardware.md).

## The optional tools of the checks

`scripts/ci.sh` uses these tools when they are installed, and skips their
stages when they are not. [Run the checks](run-the-checks.md) says what
each stage does.

- [actionlint](https://github.com/rhysd/actionlint) and
  [zizmor](https://github.com/zizmorcore/zizmor): the workflows.
- Ruby: the issue forms.
- [typos](https://github.com/crate-ci/typos): spelling.
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny): the licences
  and advisories of the dependencies (`cargo install cargo-deny --locked`).
- mdBook, Vale and lychee: the book, its prose and its links.
  `scripts/docs-tools.sh DIR` downloads the versions that CI uses, for
  x86_64 Linux.
