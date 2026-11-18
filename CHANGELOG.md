# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **The documentation is a book**, online at
  [oscyl.github.io/oscyl/docs](https://oscyl.github.io/oscyl/docs/). It
  starts with four short pages for newcomers: what Oscyl is and is not, and
  what it reads from and writes to a synth. A tutorial builds a first driver
  step by step, and the API reference of the driver crates is on the same
  site.

### Changed
- **The minilogue xd driver is Tier 1.** Its maintainer checks it on the
  synth for each release, and a failure of that check blocks the release.
- **Two driver crates have new names.** `oscyl-minilogue-xd` is now
  `oscyl-korg-minilogue-xd`, and `oscyl-prophet-rev2` is now
  `oscyl-sequential-prophet-rev2`, so every driver package is
  `oscyl-<vendor>-<model>`. A project that uses them from Git changes the
  names in its `Cargo.toml` and its `use` lines.

## [0.5.0] - 2026-11-16

Generate goes deeper, your units become part of it, Oscyl makes
oscillators of its own, and the desktop app comes to Windows and macOS.

### Added
- **The desktop app on Windows and macOS.** Each release carries the
  GTK app as a zipped folder for Windows (x86_64) and as `Oscyl.app`
  for macOS (Apple silicon and Intel), with the libraries it needs
  inside, next to the Linux Flatpak. Neither is signed or notarised
  yet, and neither has met a real synth: testers are welcome. There is
  no hotplug there yet; a synth plugged in later connects with Connect.
- **Generate goes deeper.** The minilogue xd has 28 kinds of sound, the
  prologue 18 and the Rev2 16, grouped in families, with a quality gate
  for every driver. Surprise picks a kind for you, Morph blends two
  sounds, and a recipe code copies a sound as a short code that deals it
  again anywhere. On the desktop, Android and the new `oscyl generate`.
- **Your units in Generate.** The user oscillators and effects in a
  synth's slots become Generate's palette, so about one sound in three
  uses one, within each parameter's declared range. A chip row turns
  units off for the session. The editor names a user oscillator's
  parameters and their ranges after the unit in its slot.
- **A shelf of unit files.** Oscyl keeps unit files for each synth, and
  one tap loads one into the first free slot of its section. Also
  `oscyl units shelf add|list|remove|load` and `oscyl units info`.
- **New oscillators.** Oscyl makes oscillators of its own for the
  minilogue xd and prologue: one prebuilt engine unit (wavetables, phase
  distortion, FM, folding, sync, supersaw, formants, noise, crush) that
  a generated patch wires up. Generate deals six as tiles with their
  waves; vary and breed them, keep one to load it, or deal sounds that
  use a new one in one tap. Also `oscyl units make`. Not yet heard on a
  real synth; testers are welcome.

### Changed
- **One look on every screen.** The desktop app now speaks the phone
  app's design language: section names engraved into their frames, a
  stage header with a live summary, the saved value above each dial,
  segment ticks, Generate's chips and sound pictures, the phone's empty
  states and toasts. It fits a window down to 360 px.
- **The Android app is signed** with Oscyl's release key.
- With Live on, a knob turned on the synth moves in the app at once, as
  fast as a finger on the screen; only big jumps still glide.

### Fixed
- Android finds a minilogue xd or prologue whose USB ports have no names,
  and talks to its sound port rather than the simulator.
- A knob turned on the synth while the phone's screen is off lands in the
  program it was turned on, and is saved.
- On Linux, a unit bigger than about 4 KB now reaches the synth: the ALSA
  sequencer dropped it, so the upload timed out. Oscyl writes to the
  synth's rawmidi device instead.
- The Rev2's generated modulation goes to the destinations its manual
  lists.

## [0.4.0] - 2026-11-14

Oscyl now runs on Android, with an iOS app ready for its first Mac build,
and supports two more synths: the KORG prologue and the Sequential Prophet
Rev2.

### Added
- **The `oscyl` command-line tool builds on macOS and Windows** (CoreMIDI
  and WinMM through `midir`). CI tests it there, and a release publishes
  it for both. It has not yet run with a synth on either; testers are
  welcome. The GNOME app stays Linux only.
- **The Android app** (Compose Multiplatform, `apps/mobile`). It does what
  the desktop app does, on phones, foldables and tablets. It has been tested
  on the emulator with the simulated synth, but not yet with a synth on a
  real phone; testers are welcome.
  - **Library:** the synths' programs as they are read, and Oscyl's own
    libraries in folders. Rename, duplicate, move and delete, with Undo.
    The program menu copies, pastes over, copies to a library, moves,
    reverts, removes or clears. Synth and Oscyl library files import and
    export through the system's file picker or share to other apps.
    Factory slots are marked and never changed.
  - **Editor:** every stage of a program in one scroll, with zoomable
    pictures and draggable handles. Dials and selectors keep the saved
    value in view. A stage rail follows the scroll and scrubs when held.
    Live audition, hold to compare, and undo and redo.
  - **Review and sync:** the island's Save count opens every edited
    program with its changes in words and a revert for each. Sync backs
    up the synth's slots first, then writes only the edited ones. A
    failure keeps every edit and says why.
  - **Synth:** read again or stop a read. Keep the program the synth
    plays. Load and clear user oscillators and effects on logue synths:
    cleared slots are backed up first and can be put back. The backups
    list opens a backup as a library or shares it.
  - **Generate:** pick a character and four shape dials to deal six
    sounds. Vary them, breed two, step back through earlier hands, and
    keep one over the program open or in a library.
  - **First run:** it asks what you mainly do, offers a simulated synth
    and a demo library, and opens programs where your answer suggests.
  - **Guide and Preferences:** a guide covers connecting, editing,
    auditioning, generating, syncing and units. Preferences set the theme
    (AMOLED black included), the app's own animation switch, haptics and
    the demo mode.
  - **Your edits are safe:** unsynced edits on a connected synth survive
    the app being closed, and a synth unplugged with edits leaves them as
    a new library.
  - **For everyone:** English and Italian, screen readers, large text,
    every window size, and switchable haptics.
- **The iOS app**, ready for its first Mac build. It has the same app as
  Android, with USB MIDI through CoreMIDI, the document picker, the share
  sheet, haptics and opening Oscyl files from other apps. It has not run
  on a device yet, and its CI build and simulator launch run on macOS.
- **Two more synths, both at Tier 2.** Each passes the conformance suite
  against its simulator, and hardware testers are wanted.
  - **KORG prologue** (8 and 16): the two-timbre program with every field,
    writes committed safely, audition, live edits, `.prlglib` files, user
    units, the panel's value meanings, graphs and Generate. Programs from
    firmware 1.x keep their "Not set" fields.
  - **Sequential Prophet Rev2:** both layers, writes confirmed by reading
    them back, audition, `.syx` files, and read-only factory banks
    (F1-F4).
- **Generate in the Linux app** (Ctrl+G): it deals six sounds from twelve
  characters, from power chords, fuzz bass and scream leads to bells and
  space effects, shaped by four dials (bright, movement, grit, ambience).
  Picking a sound plays it on the synth. Keep puts it over the selected slot
  as one undoable edit. Vary and Breed With deal variations and crosses, and
  Back and Forward walk the last four hands. Stages of the selected program
  can be kept, to give a sound of your own a new filter or new effects.
- **Libraries in the Linux app**: Oscyl keeps its own libraries, in folders
  you can nest and move by drag. New programs can be added, and programs
  put in order by dragging them.
- **Several synths at once in the Linux app**, two of one model included.
  The sidebar picks which one to show and sync with, and a library sends
  to a synth of its model. When the Prophet Rev2 does not answer, the app
  names its MIDI SysEx Cable setting.
- **The CLI works with any supported synth.** The file's format or
  `--synth` picks it, and `--simulator` opens its demo synth.
- **A driver guide** (`docs/drivers.md`) for adding a synth.
- Opening a file whose extension Oscyl doesn't know reads it as an Oscyl
  library when its contents are one.
- Third-party licence pages, in the mobile app and in the desktop app's
  About dialog.
- Project governance (roles, support tiers, RFCs and ADRs), a code of
  conduct, a security policy, a support page, issue forms and a pull request
  template.
- CI for every driver's conformance, `cargo deny` and the docs, mirrored by
  `scripts/ci.sh`, and release automation that builds the CLI, the Flatpak
  and the Android app with checksums and build provenance.

### Changed
- Libraries are kept in Oscyl's own format, `.oscyllib`, which records the
  synth each one belongs to. Existing libraries convert the first time
  Oscyl starts. The original files are kept in `before-oscyllib/` next to
  them, and a file that cannot be read stays there unchanged.
- The Linux app and the CLI read everything about a synth from its driver:
  pages and controls, values, graphs, Generate, file formats, slots and
  banks. The program list's filter is the driver's (the minilogue xd's
  voice mode).
- Factory (read-only) slots can be looked at but not edited.
- The desktop's licence sections have short titles, and a cleared unit's
  backup loads back into a slot.
- For driver authors: the driver contract describes libraries neutrally,
  with vendor file import and export that must round-trip, and each format
  says whether it keeps a program's author and comment. It gained an
  optional demo library, demo simulator, list facet, firmware text and
  setup hint.
- The KORG logue drivers share `oscyl-logue`; the minilogue xd behaves
  byte for byte as before.

### Fixed
- The read-lag check no longer reports a lagging synth as healthy.
- The MIDI link sends each SysEx message of a buffer on its own: midir's
  ALSA backend sent only the first.
- An edit made while a sync runs no longer shows as synced (desktop).
- Opening Generate while a library is shown no longer crashes the desktop
  app.

## [0.3.0] - 2026-11-11

### Changed
- Editor redesigned as one ruled chart band with a single signal line through
  six stages, live graphs you can drag (filter peak, envelope points, mixer
  levels), a quiet red tick as the only edit mark, and no cards.
- Program list grouped under category headers; every demo program renamed.
- Help, guide and onboarding copy rewritten: plain, accurate, no filler.
- Faster editing: dial and graph drags update only what changed.
- Sync now saves a backup of the slots it replaces first.

### Fixed
- Wrong help facts (four voices; VCO 1 modulates VCO 2).
- Firmware version no longer hard-coded; developer flags are not saved.
- Timers removed from list scrolling and screenshot capture.

## [0.2.0] - 2026-11-11

### Added
- `oscyl-gtk`, the GNOME desktop app:
  - offline-first library editing;
  - the six-stage editor with live graphs and a zoomable inspector;
  - live audition;
  - sync that previews its changes and commits them deterministically;
  - unit slot management with automatic backups;
  - undo/redo;
  - light, dark and AMOLED themes;
  - interface zoom and keyboard shortcuts;
  - an in-app guide and a first-run setup;
  - phone-to-desktop layouts;
  - translations.
- A simulated minilogue xd (`oscyl_core::sim`), a 120-program demo library,
  and `--simulator` in the CLI and the app.
- Plain-language help for every parameter, and presentation tables (stages,
  sections, value names, relevance) shared by all front ends.
- Packaging: app icon, desktop entry, AppStream metadata and a Flatpak
  manifest.

### Changed
- Program writes are committed deterministically: each batch is padded so the
  firmware saves synchronously before its last acknowledgement
  (`store::plan_commit`). The fixed 2 s post-write quiet period is gone.
- The synth's port is selected by name ("minilogue xd _ SOUND") instead of
  probing every port.

### Added
- Automatic recovery from the firmware's reply-lag fault without a power cycle.

## [0.1.0] - 2026-11-11

### Added
- `oscyl-core`: sans-IO minilogue xd protocol (SysEx framing, KORG 7-in-8
  packing, message codec), typed 1024-byte program model with all 138
  documented fields, Librarian file formats (`.mnlgxdlib`, `.mnlgxdprog`,
  `.mnlgxdpreset`), logue SDK unit files, and a diff-based write planner.
- `oscyl-midi`: ALSA transport via `midir` with automatic device discovery.
- `oscyl` CLI: `ports`, `info`, `current`, `backup`, `diff`, `restore`
  (writes only changed programs, after an automatic backup), `audition`,
  `units list|upload|clear`.

### Fixed
- Avoids and detects a firmware 2.10 fault where reading during a flash save
  leaves the synth answering every request with the previous request's data.

[Unreleased]: https://github.com/oscyl/oscyl/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/oscyl/oscyl/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/oscyl/oscyl/releases/tag/v0.4.0
