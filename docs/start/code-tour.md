# A tour of the code

Oscyl is one Cargo workspace with a few apps around it. This page names
each part, follows one program from the synth to the screen, and says where
to start reading for each kind of work.

## The parts

| Part | What it does |
|---|---|
| [`crates/oscyl-driver`](https://github.com/oscyl/oscyl/tree/main/crates/oscyl-driver) | The driver contract: the traits that every driver implements. Also the SysEx helpers and the request engine. |
| [`crates/drivers/`](https://github.com/oscyl/oscyl/tree/main/crates/drivers) | One crate for each synth model, and `logue`, the code that the two KORG logue drivers share. |
| [`crates/oscyl-conformance`](https://github.com/oscyl/oscyl/tree/main/crates/oscyl-conformance) | The checks that every driver passes against its simulator. |
| [`crates/oscyl-core`](https://github.com/oscyl/oscyl/tree/main/crates/oscyl-core) | The `.oscyllib` library format, and the parts of Generate that no driver owns. |
| [`crates/oscyl-midi`](https://github.com/oscyl/oscyl/tree/main/crates/oscyl-midi) | The transport: MIDI ports on the desktop, or bytes that Android and iOS move. It also holds the list of drivers that the apps know, `DRIVERS`. |
| [`crates/oscyl-app`](https://github.com/oscyl/oscyl/tree/main/crates/oscyl-app) | The app logic that every app shares: documents with undo, libraries, settings, the editor, Generate, and the worker thread that talks to the synths. It has no UI. |
| [`crates/oscyl-cli`](https://github.com/oscyl/oscyl/tree/main/crates/oscyl-cli) | The command line tool, `oscyl`. |
| [`crates/oscyl-ffi`](https://github.com/oscyl/oscyl/tree/main/crates/oscyl-ffi) | The bindings that give `oscyl-app` to the mobile apps, through UniFFI. |
| [`crates/oscyl-osc-host`](https://github.com/oscyl/oscyl/tree/main/crates/oscyl-osc-host) | A test crate. It compiles the oscillator unit's C for the computer, so tests can hear a patch. Nothing ships it. |
| [`apps/linux`](https://github.com/oscyl/oscyl/tree/main/apps/linux) | The desktop app (GTK 4 and libadwaita), for Linux, Windows and macOS. |
| [`apps/mobile`](https://github.com/oscyl/oscyl/tree/main/apps/mobile) | The Android and iOS apps (Compose Multiplatform). |
| [`units/oscyl-osc`](https://github.com/oscyl/oscyl/tree/main/units/oscyl-osc) | The oscillator unit: its C source and its prebuilt payloads. |

[Repository layout](../reference/layout.md) lists every top-level folder.

## One program, from the synth to the screen

When the desktop app reads slot 5 of a synth, the bytes go through these
steps.

1. The app asks the worker thread in `oscyl_app::synth` to read every slot.
2. The worker gives the operation to the synth's `oscyl_midi::Device`. The
   device holds the driver's session and the MIDI port.
3. The session turns "read slot 5" into a SysEx request. The
   [request engine](glossary.md#request-engine) sends it and waits for the
   answer.
4. The synth answers with a program dump. The device gives the bytes to the
   session, which collects one whole SysEx message.
5. The session checks that the message answers slot 5. It reports
   `SessionEvent::Program(5, bytes)`.
6. The worker sends the program to the app. The app puts it in the synth's
   document, a list of programs with undo.
7. The editor reads each value with `Driver::get` and its text with
   `Driver::format_value`. The driver's `curation` says on which page and in
   which section each field goes.

A write goes the same way in the other direction. On Android and iOS, step 2
is different. The platform owns the USB port and moves the bytes, and
`oscyl-ffi` hands them to the same device and session.

[Architecture](../explanation/architecture.md) tells why the parts are
split this way.

## Where to start reading

- **A driver for a new synth:** the tutorial
  [Your first driver](../tutorials/first-driver.md), then the Prophet Rev2
  driver in
  [`crates/drivers/sequential-prophet-rev2`](https://github.com/oscyl/oscyl/tree/main/crates/drivers/sequential-prophet-rev2).
  It is the smallest complete driver.
- **The contract itself:**
  [`contract.rs`](https://github.com/oscyl/oscyl/blob/main/crates/oscyl-driver/src/contract.rs)
  and [The driver contract](../reference/driver-contract.md).
- **The desktop app:**
  [`apps/linux/src/app.rs`](https://github.com/oscyl/oscyl/blob/main/apps/linux/src/app.rs),
  which holds the app's state and its answers to the synth's events.
- **The mobile apps:** the screens under
  [`apps/mobile/shared/src/commonMain`](https://github.com/oscyl/oscyl/tree/main/apps/mobile/shared/src/commonMain),
  and [The mobile bindings](../reference/mobile-bindings.md).
- **Generate:**
  [`crates/oscyl-app/src/generator.rs`](https://github.com/oscyl/oscyl/blob/main/crates/oscyl-app/src/generator.rs),
  then a driver's own `generate.rs`.
- **The library format:**
  [`crates/oscyl-core/src/native.rs`](https://github.com/oscyl/oscyl/blob/main/crates/oscyl-core/src/native.rs).
- **The command line:**
  [`crates/oscyl-cli/src/main.rs`](https://github.com/oscyl/oscyl/blob/main/crates/oscyl-cli/src/main.rs).

To build and run each app, read [Build and run](../how-to/build.md).
