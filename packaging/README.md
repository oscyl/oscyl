# Packaging

Everything a distribution or Flatpak build needs to install Oscyl. App ID:
`io.github.oscyl.Oscyl`; binary: `oscyl-gtk` (crate `apps/linux`).

| File | Installs to |
| --- | --- |
| `apps/linux/data/io.github.oscyl.Oscyl.desktop` | `share/applications/` |
| `apps/linux/data/io.github.oscyl.Oscyl.metainfo.xml` | `share/metainfo/` |
| `apps/linux/data/icons/hicolor/scalable/apps/io.github.oscyl.Oscyl.svg` | `share/icons/hicolor/scalable/apps/` |
| `apps/linux/data/icons/hicolor/symbolic/apps/io.github.oscyl.Oscyl-symbolic.svg` | `share/icons/hicolor/symbolic/apps/` |
| `po/<lang>.po` (compiled with `msgfmt`) | `share/locale/<lang>/LC_MESSAGES/oscyl.mo` |

Build the binary with `OSCYL_LOCALEDIR=<prefix>/share/locale` so it finds the
translations (the default is `/usr/share/locale`).

## Checks

```sh
desktop-file-validate apps/linux/data/io.github.oscyl.Oscyl.desktop
appstreamcli validate --no-net apps/linux/data/io.github.oscyl.Oscyl.metainfo.xml
```

The metainfo has no screenshots yet (see the TODO in the file); Flathub needs
them before submission. Add a `<release>` entry for every version.

## Icons

The app icon is a pen-stroke ring dial whose arc runs on into an oscillator
wave, ending in an ink "pen" dot. It sits on a chart-card ground (GNOME 128 px
grid: 96 px square with 8 px bottom depth). The symbolic icon is the same mark as
filled paths in `#2e3436`. Neither uses KORG marks or panel artwork.

Note that `apps/linux/build.rs` also bundles these SVGs into the GResource,
like every other SVG under `data/`.

## Flatpak

See [`flatpak/README.md`](flatpak/README.md).

## Windows and macOS

[`desktop/`](desktop/) bundles the app so it runs on a machine with nothing
else installed. `bundle-windows.sh` makes a folder (`bin/Oscyl.exe`, the
MSYS2 UCRT64 DLLs it loads, `share/`, `licences/`); `bundle-macos.sh` makes
`Oscyl.app` with Homebrew's dylibs relocated into it, signed ad hoc. Both
share `bundle-share.sh` (GTK's schemas, the Adwaita icons, the translations)
and ship the licence texts and exact versions of the libraries they carry
(`THIRD-PARTY.txt`, `PACKAGES.txt`). The release and `desktop-apps.yml`
workflows run them; [Build and run Oscyl](../docs/how-to/build.md) shows
how by hand.
