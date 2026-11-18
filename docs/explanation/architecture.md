# Architecture

Oscyl is a set of Rust crates with thin apps around them. This page explains
how the parts divide the work, and why. [A tour of the code](../start/code-tour.md)
names the parts, and [Repository layout](../reference/layout.md) lists the
folders.

## Generic code and synth code

Oscyl draws one line through everything: what is the same for every synth,
and what belongs to one synth.

- **The same for every synth:** the library, sync, auditions, the editor's
  drawing, Generate's hands, the files on disk. This code is in
  `oscyl-core`, `oscyl-app` and the apps.
- **One synth's:** its protocol, the meaning of its program bytes, its file
  formats, its quirks, and how its editor pages look. This code is in its
  driver.

The two meet in the driver contract, the traits of `oscyl-driver`. Generic
code sees a driver only as `&dyn Driver`. It never asks which synth it has:
a driver declares what its synth can do (`Capabilities`), and the apps show
only that. So a new driver gets every feature of every app, and no app
changes for it.

## Why drivers are compiled in

A driver is a crate in this repository, compiled into each app. Oscyl has
no plugins that load at run time. [ADR 0002](../adr/0002-driver-contract.md)
records the decision. These are the reasons:

- A driver writes to a synth's memory. Oscyl must trust it, and CI tests
  every driver on every commit.
- iOS does not let an app load code at run time, and every app shares the
  same drivers.
- The contract is young. When it changes, every driver changes in the same
  commit, so `main` always builds ([ADR 0001](../adr/0001-monorepo.md)).

A driver outside this repository is not supported yet. That can change
when the contract crate is published and stable.

## Sans IO

A driver's session and simulator never touch a port, a thread or a clock.
The session takes bytes and the time from its caller, and gives back bytes
to send and events. This is *sans-IO* code.

This choice pays several times:

- The same session runs on Linux, macOS and Windows (through midir), on
  Android and iOS (where the platform moves the bytes), and in tests.
- A test runs a session against its simulator on a virtual clock. Time jumps
  to the next deadline, so a lost reply times out at once, and no test
  sleeps.
- The conformance suite can run every driver through the same checks,
  faults and all, in milliseconds.

Nothing in Oscyl waits for a fixed time. A step ends on the synth's reply,
or on a timeout that only finds a dead link. The worker thread of
`oscyl-app` sleeps until a synth sends bytes, a command comes, or the
earliest deadline of a session passes.

## The request engine

Most synths answer one request at a time, and their replies often do not
say which request they answer. So `oscyl_driver::engine` runs a protocol
with one request in flight:

1. A driver's `Protocol` expands each operation into steps. A step is a
   message to send and the reply that completes it.
2. The engine sends one step, and waits.
3. Each complete SysEx message from the synth goes to the protocol, which
   says whether it completes the step, fails it, or is unrelated.
4. The first failure, or a timeout, ends the operation. The engine does not
   send the rest of its steps.
5. The engine reports progress after each step, then `Done` or `Failed`.

An unrelated message is common: a port that echoes Oscyl's own output, or a
dump that someone sends from the synth's panel. The engine ignores it and
keeps waiting, so neither can complete a write by accident.

## Safe writes

A write changes the only copy of someone's sounds. These rules keep it safe:

- **A write is done only when the synth says so.** If the synth sends no
  completion signal, the driver reads the slot back and compares it. The
  conformance suite checks, with every known fault, that no write reports a
  false success.
- **A backup comes first.** Sync backs up the slots that it will change, and
  writes nothing if the backup fails. `oscyl restore` backs up the whole
  synth first.
- **Factory slots are never written.** A bank says whether it is writable,
  and a session refuses a write to a factory slot before it sends anything.
- **The driver plans the commit.** A KORG logue synth saves its flash memory
  later than it answers. Its driver pads each batch so that the last write
  forces the save before the synth answers. Then the next read is safe at
  once, with no timing involved.

## What runs where

| Platform | App | Synth link |
|---|---|---|
| Linux | The GTK app (`apps/linux`) on `oscyl-app` | `oscyl-midi` with midir on ALSA, with hotplug |
| macOS | The same GTK app | midir on CoreMIDI; choose Connect after you plug the synth in |
| Windows | The same GTK app | midir on WinMM; choose Connect after you plug the synth in |
| Android | The Compose app (`apps/mobile`), on `oscyl-app` through `oscyl-ffi` | Android's `MidiManager` moves the bytes |
| iOS | The same Compose app | CoreMIDI, in the app's own code, moves the bytes |
| Every desktop | The command line, `oscyl-cli` | `oscyl-midi` with midir, one operation at a time |

On every platform, the same `oscyl_midi::Device` holds a driver's session
and a link. Only the link differs: a midir port, bytes that the platform
pushes, or a simulator.

## Further reading

- [What happens when you plug in a synth](../start/plug-in.md): the same
  design, from the user's side.
- [The driver contract](../reference/driver-contract.md): every method.
- [Decision records](../adr/): the decisions behind this page.
