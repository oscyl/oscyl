# Supported synths

The synths that Oscyl has a driver for, and what Oscyl does with each one.

## What works on each synth

| | KORG minilogue xd | KORG prologue (8 and 16) | Sequential Prophet Rev2 |
|---|---|---|---|
| Librarian: read, organise, write programs | Yes | Yes | Yes |
| Editor, with a graph for each stage | Yes | Yes | Yes |
| Backup and `oscyl restore` | Yes | Yes | Yes |
| Live: edits play at once | Yes | Yes | Yes |
| Live: the synth's knobs and program changes move the app | Yes | No | No |
| Generate | Yes | Yes | Yes |
| User units: list, load, clear, shelf | Yes | Yes | No |
| The oscillator unit | Yes | Yes | No |
| The driver's own words in Italian | Yes | No | No |

## The drivers

| | KORG minilogue xd | KORG prologue | Sequential Prophet Rev2 |
|---|---|---|---|
| Driver id | `korg-minilogue-xd` | `korg-prologue` | `sequential-prophet-rev2` |
| Crate | `oscyl-minilogue-xd` | `oscyl-korg-prologue` | `oscyl-prophet-rev2` |
| Tier | 2 | 2 | 2 |
| Program slots | 500, all writable | 500, all writable | U1 to U4 (512, writable); F1 to F4 (512, factory, never written) |
| User unit slots | Oscillators 16, modulation effects 16, delays 8, reverbs 8 | Oscillators 16, modulation effects 16, delays 8, reverbs 8 | None |
| File formats | `.mnlgxdlib`, `.mnlgxdpreset`, `.mnlgxdprog` | `.prlglib` | `.syx` |
| Unit files | `.mnlgxdunit` | `.prlgunit` | None |
| Port name hints | `minilogue xd _ SOUND`, `minilogue xd _ MIDI 2` | `prologue _ SOUND`, `prologue _ MIDI 2` | `Prophet Rev2` |

Every format above keeps each program's author and comment, except `.syx`.
Oscyl also reads and writes every synth's programs as `.oscyllib`.

## What was checked on hardware

- **minilogue xd:** checked on the synth, firmware 2.10, with the desktop
  app and the command line on Linux. The Android app finds it on a Pixel 7
  Pro. On Windows 11, `oscyl info` finds it.
- **prologue and Prophet Rev2:** proved against their simulators only.
  Hardware testers are welcome.
- **The oscillator unit:** not yet heard on a synth.

The desktop app on macOS and Windows, and the iOS app, have not run with a
synth yet.

## Tiers

[GOVERNANCE](https://github.com/oscyl/oscyl/blob/main/GOVERNANCE.md#support-tiers)
defines the tiers. A driver reports its tier with `Driver::tier`.

No driver is Tier 1 today. The minilogue xd was checked on the synth, but
Tier 1 also needs a named maintainer who checks the driver for each
release. `.github/CODEOWNERS` names the maintainers' team for each driver,
not one person, so every driver is Tier 2.

| Tier | What it takes |
|---|---|
| Tier 3 (community) | It compiles. |
| Tier 2 (conformance-proven) | It passes the conformance suite against its simulator on every commit. |
| Tier 1 (hardware-validated) | Tier 2, and a named maintainer checks it on the real synth for each release. A Tier 1 failure blocks the release. |

To add a synth, see [Add a driver for a new synth](../how-to/add-a-synth.md).
