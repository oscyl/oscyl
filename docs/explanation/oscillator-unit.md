# The oscillator unit

The KORG minilogue xd and prologue load *user oscillators*: small programs,
made with KORG's logue SDK, that the synth plays in its Multi Engine. A user
oscillator is normally C code, built with an ARM compiler for one synth.
Oscyl makes new user oscillators on a phone, with no compiler. This page
explains how, and why the oscillator unit lives in this repository.

## One prebuilt unit, many oscillators

Oscyl has one logue SDK oscillator of its own, the *oscillator unit*, in
[`units/oscyl-osc`](https://github.com/oscyl/oscyl/tree/main/units/oscyl-osc).
Its C holds a fixed set of DSP blocks. Some make a sound: wavetables, FM,
a supersaw, a sub oscillator and noise. Others shape it: phase distortion,
sync, a wavefolder, a bit crusher, a low-pass filter and formant filters.
The unit is built once for each synth. The builds are in the repository, as
`payload/minilogue-xd.bin` and `payload/prologue.bin`.

The blocks do nothing on their own. A *patch* wires them up. A patch is
data, and it says:

- which blocks run, in what order, and with which constants;
- what Shape, Shift-Shape, an LFO, an envelope and the six parameters move;
- up to eight single-cycle tables.

The unit keeps a marked region of 4,608 bytes for the patch. It reads the
patch from there when the synth starts the unit. As built, the region holds
only its marker, and the unit plays a plain saw.

To make a new oscillator, Oscyl writes a patch into a copy of the payload.
Then it adds a manifest: the oscillator's name, its parameters and an id of
its own. The result is an ordinary unit file, `.mnlgxdunit` or
`.prlgunit`. The synth loads it like any other user oscillator, into a free
oscillator slot.

So the sound is data, and every app can make it. The desktop app, the
phone apps and `oscyl units make` all use the same Rust code,
[`oscyl_logue::osc`](https://github.com/oscyl/oscyl/blob/main/crates/drivers/logue/src/osc.rs).
It makes patches in six families: wavetable, FM, fold, formant, noise and
hybrid. It varies and crosses them as Generate does with programs.

## An extra, not a requirement

The oscillator unit is optional. Oscyl puts one on the synth only when you
keep a new oscillator, and only into an empty oscillator slot. The
librarian, the editor and Generate's sounds all work without it, and the
Prophet Rev2 has no user units at all.

## How Oscyl knows what a patch sounds like

Oscyl draws one cycle of each new oscillator's wave, and checks that no
patch is silent or too loud. For this, `oscyl_logue::osc::render` runs the
unit's DSP in Rust, operation for operation.

A Rust copy of C code can drift from the C. So the test crate
`oscyl-osc-host` compiles the unit's C for the computer, and its tests hold
the two together:

- every family's oscillators make a sound, do not clip, and carry no DC
  offset (a constant shift of the whole wave);
- the Rust port plays what the C plays;
- a bad or empty patch plays a plain saw;
- the committed payloads come from the sources as they are. The file
  `payload/SOURCE` holds the checksum of the sources and of
  `scripts/build-units.sh`, and a test compares it.

## Why it lives in this repository

The unit, its patch format and its Rust code change together. A new block
needs C, a Rust port, a patch field and new tests, in one commit. The KORG
drivers load the unit's payloads, so the unit stays beside them.
[ADR 0001](../adr/0001-monorepo.md) gives the same reason for the whole
repository: a change and everything that it affects land together.

`scripts/build-units.sh` rebuilds the payloads, with
`arm-none-eabi-gcc`. A new payload must be checked on the synths before a
release, so nobody builds them again without a reason.

## What it is not

- **Not a synth engine on your computer.** The unit runs on the synth.
  Oscyl's apps make no sound.
- **Not a target platform.** It is one oscillator for two KORG synths. It is
  not a third kind of synth next to them, and not a place to run other
  engines.
- **Not a converter.** It does not turn a program of one synth into a sound
  of another.

The oscillator unit has not yet been heard on a real synth. Testers with a
minilogue xd or a prologue are welcome.
