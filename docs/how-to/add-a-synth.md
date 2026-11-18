# Add a driver for a new synth

This guide takes a real synth from its first SysEx dump to a driver that
the apps list. It assumes that you know Rust and MIDI, and that you did
[Your first driver](../tutorials/first-driver.md) or read its code.

Use the Prophet Rev2 driver,
[`crates/drivers/sequential-prophet-rev2`](https://github.com/oscyl/oscyl/tree/main/crates/drivers/sequential-prophet-rev2),
as your model. It is one synth with a published protocol, no shared family
crate and no user units. For two synths that share code, see the KORG
prologue and the family crate
[`crates/drivers/logue`](https://github.com/oscyl/oscyl/tree/main/crates/drivers/logue).

## 1. Learn the synth's protocol

1. Collect public documents: the maker's MIDI implementation chart, the
   manuals, and open references with a compatible licence.
2. Record what your synth sends and answers. Compare dumps from before and
   after one change on the panel.
3. Send only read requests until you know what a write does.
4. Note the firmware version that you work with.

Name your sources in the crate's module docs and in the pull request. Keep
your captures and dumps in `private/`, which git ignores. Never commit
vendor firmware, factory programs or files from the internet.

## 2. Find the synth's port names

A driver finds its synth by the names of its MIDI ports (`port_hint`). The
same USB port gets a different name on each platform. Oscyl spells the
names alike where it can.

| Platform | How the name looks | Example |
|---|---|---|
| Linux (ALSA) | `<product> _ <port>`, then the client and port numbers | `minilogue xd:minilogue xd _ SOUND 24:1` |
| Android | `<product> _ <port>`. Android names no port, so Oscyl numbers them: `<product> _ MIDI <n>` | `minilogue xd _ MIDI 2` |
| Windows | The first port has the product name. Windows names later ports `MIDIIN2 (<product>)` and `MIDIOUT2 (<product>)`, which `oscyl_midi::port_label` reads as `<product> _ MIDI 2` | `minilogue xd _ MIDI 2` |
| macOS, iOS | The names that CoreMIDI reports | |

To see the names:

- **Linux, macOS and Windows:** run `oscyl ports`.
- **Android:** run `adb logcat -s OscylMidi` while you plug the synth in.
- **iOS:** filter Console.app on `OscylMidi`.

Give `port_hint` one substring for each name that the synth's port can have.
The minilogue xd driver gives `"minilogue xd _ SOUND"` and
`"minilogue xd _ MIDI 2"`.

## 3. Make the crate

1. Make `crates/drivers/<vendor>-<model>`. The workspace includes every
   crate under `crates/drivers/`.
2. Copy the Rev2's `Cargo.toml`. Change the name and the description. The
   only dependency is `oscyl-driver`, and `oscyl-conformance` is a
   dev-dependency.
3. Split the code into modules as the Rev2 does:

| Module | What it holds |
|---|---|
| `msg.rs` | The SysEx messages: build them, decode the replies |
| `program.rs` | The program bytes, their packing and the name |
| `fields.rs` | The parameters, with each field's offset and range |
| `syx.rs` | The synth's file formats (`.syx` for the Rev2) |
| `session.rs` | The `DriverSession`: reads, writes and their completion signals |
| `sim.rs` | The `Simulator`, with the faults seen on the hardware |
| `curate.rs` | The editor's pages and sections |
| `generate.rs` | Generate's characters and recipes (optional) |
| `driver.rs` | `impl Driver` and `pub static DRIVER` |

Obey these rules:

- A write is done only when the synth says so. If the synth sends no
  completion signal, the session reads the slot back and compares it.
- The simulator reproduces every fault that you saw on the hardware, so the
  conformance suite can prove that the session survives it.
- The simulator has the fault `lose_reply`: every reply is lost.
- Test fixtures are synthetic. You make them; you do not copy them.

[The driver contract](../reference/driver-contract.md) lists every method
and which ones you can leave out.

## 4. Pass the conformance suite

Add `tests/conformance.rs`:

```rust
#[test]
fn prophet_rev2_conforms() {
    oscyl_conformance::run(&oscyl_prophet_rev2::DRIVER);
}
```

Run it until it passes:

```sh
cargo test -p oscyl-prophet-rev2 --test conformance
```

A failure names the driver, the check and the problem.
[The driver contract](../reference/driver-contract.md#what-the-conformance-suite-checks)
lists the checks.

## 5. Register the driver

1. Add the driver to `DRIVERS` in
   [`crates/oscyl-midi/src/device.rs`](https://github.com/oscyl/oscyl/blob/main/crates/oscyl-midi/src/device.rs),
   and as a dependency in `crates/oscyl-midi/Cargo.toml`. Every app and the
   command line find drivers there.
2. Add the package to the `conformance` matrix in
   `.github/workflows/ci.yml`. `scripts/ci.sh` finds the test on its own.
3. Name the driver's maintainer for the crate in `.github/CODEOWNERS`.
4. Add a row for the synth to the README's table of supported synths and to
   [Supported synths](../reference/synths.md), at Tier 2.
5. Set `Driver::tier` to 2.

Then try the driver with its simulator:

```sh
cargo run -p oscyl-cli -- --simulator --synth sequential-prophet-rev2 info
cargo run -p oscyl-gtk -- --simulator
```

## 6. Translate its words (optional)

A driver's page titles, labels, value names and help can be translated:

1. Add the driver's id to `TRANSLATED` in
   [`apps/linux/src/pot.rs`](https://github.com/oscyl/oscyl/blob/main/apps/linux/src/pot.rs).
2. Make the template again, as [Translate Oscyl](translate.md) says.

Today only the minilogue xd is translated. The other drivers stay in
English.

## 7. Check it on the synth

Follow [Check a driver on hardware](hardware-check.md) before you ask for a
review.

## 8. Send it

1. Open a "Driver submission" issue. It tells the maintainers about the
   driver.
2. Open the pull request, and link it from the issue. Its template has a
   checklist for a new driver.

[GOVERNANCE](https://github.com/oscyl/oscyl/blob/main/GOVERNANCE.md#driver-acceptance)
lists what a driver needs before it is accepted.
