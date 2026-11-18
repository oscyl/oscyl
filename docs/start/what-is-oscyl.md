# What Oscyl is, and what it is not

Oscyl is a librarian and sound editor for hardware synths. It reads the
programs on your synth, and lets you organise and edit them on a computer or
a phone. Then it writes back only the programs you changed.

## What Oscyl does

A synth keeps its sounds as *programs* in numbered *slots*. Oscyl does two
jobs with them.

- **The librarian** keeps programs in *libraries*: files on your computer or
  phone. You back up a synth, sort and rename programs, copy them between
  libraries, and send them back to the synth.
- **The sound editor** shows every parameter of a program on screen. The
  pages follow the signal: voice, oscillators, mixer, filter, envelopes and
  effects. Each page has a graph of its stage, and each control shows its
  saved value next to the new one.

Oscyl also works with the synth unplugged. You open a library, edit it, and
sync it to the synth later.

**[Generate](glossary.md#generate)** makes new programs. You pick a
character, for example "fuzz bass" or "bells". You set four dials: Bright,
Movement, Grit and Ambience. Oscyl then makes six new programs for your
synth. You can vary them, cross two of them, keep the one you like, or share
it as a short recipe code.

Oscyl runs on Linux, Windows, macOS, Android and iOS, and as a command line
tool, `oscyl`. Not every platform has run with a real synth yet:

- **Checked with a synth:** the desktop app and the command line on Linux,
  the Android app, and `oscyl info` on Windows.
- **Built in CI, not yet run with a synth:** the desktop app on Windows and
  macOS, the command line on macOS, and the iOS app. You build the iOS app
  from source.

Today Oscyl supports the KORG minilogue xd, the KORG prologue and the
Sequential Prophet Rev2. [Supported synths](../reference/synths.md) says
what works on each one.

## Why it exists

KORG makes librarian software for its synths on Windows and macOS only.
Owners on Linux or Android have none, and owners of several synths need one
tool for each synth. Oscyl is one librarian and editor for every platform
and for synths of different makers.

Oscyl also touches the synth as little as it can. It reads the synth once
when the synth connects, and then works from that copy. It writes only the
slots you changed, and it backs them up first.
[What happens when you plug in a synth](plug-in.md) describes the whole
sequence.

The minilogue xd and the prologue run *user units*: oscillators and effects
that anyone can write with KORG's [logue SDK](glossary.md#logue-sdk). The
synth keeps them in its unit slots. Oscyl lists and loads them. It can also
make new user oscillators with its
[oscillator unit](../explanation/oscillator-unit.md): one prebuilt logue SDK
unit whose sound comes from data that Oscyl writes.

## What Oscyl is not

- **Not a DAW and not a sequencer.** Oscyl does not record audio and does
  not play notes. You play the synth.
- **Not a preset converter.** A program belongs to one synth model. Oscyl
  does not convert a minilogue xd program into a Prophet Rev2 program, and it
  will not.
- **Not a synth engine.** Oscyl makes no sound of its own. The simulated
  synth that you can try answers like a real synth, but it is silent.
- **Not a platform for synth engines.** The oscillator unit is a logue SDK
  unit for two KORG synths. It is not a third kind of synth and not a
  target that you write sounds for.
- **Not a firmware tool.** Oscyl never changes a synth's firmware.

## Next

- [What happens when you plug in a synth](plug-in.md)
- [Glossary](glossary.md)
- [A tour of the code](code-tour.md)
