# Flatpak

`io.github.oscyl.Oscyl.json` builds `oscyl-gtk` on the GNOME 49 runtime with
the `rust-stable` SDK extension. The build is offline, so the crate sources
must be vendored first into `cargo-sources.json` (not committed; regenerate
whenever `Cargo.lock` changes):

```sh
scripts/flatpak-sources.sh   # from the repo root
```

It runs flatpak-cargo-generator (pinned commit, hash-checked) with `uv` or
`pipx`, which install its Python dependencies.

Build and install locally:

```sh
cd packaging/flatpak
flatpak install flathub org.gnome.Sdk//49 org.freedesktop.Sdk.Extension.rust-stable//25.08
flatpak-builder --user --install --force-clean build-dir io.github.oscyl.Oscyl.json
flatpak run io.github.oscyl.Oscyl
```

CI builds the manifest in the GNOME runtime container and runs its
`test-commands` (`cargo test -p oscyl-gtk`) with a display. flatpak-builder
runs them only for a module with `"run-tests": true`, which the CI action
sets; add it to the module to run them locally.

Permissions: `--device=all` is required because the synth is reached through
ALSA MIDI (`/dev/snd`); there is no narrower Flatpak permission for it. No
audio, network or file-system permissions are requested: files are opened
through the portal file chooser.

Translations: every `po/<lang>.po` is compiled with `msgfmt` into
`/app/share/locale/<lang>/LC_MESSAGES/oscyl.mo`, and the binary is built with
`OSCYL_LOCALEDIR=/app/share/locale` so it finds them.
