# Use Oscyl's crates in your own project

This guide adds Oscyl's crates to another Rust project, from Git. Use it
for a tool that reads a synth or a library file with Oscyl's drivers.

Oscyl's crates are not on crates.io yet. A driver for a new synth goes in
this repository: see [Add a driver for a new synth](add-a-synth.md).

## Choose the crates

| Crate | Use it to |
|---|---|
| `oscyl-driver` | Use the contract: `Driver`, `SessionOp`, `SessionEvent` and the other types |
| `oscyl-midi` | Find and open a synth, and run operations on it. It holds `DRIVERS`, every driver that Oscyl has |
| `oscyl-core` | Read and write `.oscyllib` library files |
| `oscyl-conformance` | Test a driver against its simulator |
| A driver crate, such as `oscyl-korg-minilogue-xd` | Use one synth's own types directly |

The driver packages are named `oscyl-<vendor>-<model>` after Oscyl 0.5.0.
At the tag v0.5.0, two of them have older names: `oscyl-minilogue-xd` and
`oscyl-prophet-rev2`.

`oscyl-midi` depends on every driver crate. Cargo clones the whole
repository once, then builds only the crates that you name and their
dependencies. On Linux, the default feature `midir` of `oscyl-midi` needs
the ALSA development package (`alsa-lib`).

## Add the dependencies

Name the same tag for every Oscyl crate. A tag is a release, and its
crates are tested together:

```toml
[dependencies]
oscyl-driver = { git = "https://github.com/oscyl/oscyl", tag = "v0.5.0" }
oscyl-midi = { git = "https://github.com/oscyl/oscyl", tag = "v0.5.0" }
```

## Read a program

This `main.rs` connects a simulated minilogue xd, reads the program that it
plays, and prints its name and four fields:

```rust
//! Read the program that a synth plays, and print its first fields.

use oscyl_driver::{SessionEvent, SessionOp};
use oscyl_midi::Device;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let driver = oscyl_midi::driver("korg-minilogue-xd").ok_or("no such driver")?;
    // A simulated synth. For a real one: oscyl_midi::scan() and Device::open.
    let mut synth = Device::demo(driver)?;
    let mut program = None;
    synth.run(SessionOp::ReadCurrent, |event| {
        if let SessionEvent::Current(bytes) = event {
            program = Some(bytes.clone());
        }
    })?;
    let program = program.ok_or("the synth sent no program")?;
    println!("{}", driver.name(&program)?);
    for (i, field) in driver.fields().iter().enumerate().take(4) {
        let value = driver.format_value(&program, i);
        println!("{} / {}: {value}", field.group, field.name);
    }
    Ok(())
}
```

`cargo run` prints the program's name, then one line for each field. This
is the output with Oscyl 0.5.0:

```text
Glass Choir
Voice / Octave: 0
Voice / Portamento: 0
Voice / Key Trig: Off
Voice / Voice Mode Depth: Off
```

`format_value` gives each value as the editor shows it. `Driver::get` gives
the raw number that the synth stores, as `oscyl current` prints it: Octave
2, Key Trig 0.

## Open a real synth

`oscyl_midi::scan()` lists the MIDI ports that a driver claims, each with
its driver. `Device::open` connects one. It sends the device inquiry, and
fails unless the driver's synth answers:

```rust
let found = oscyl_midi::scan()?;
let candidate = found.first().ok_or("no synth found")?;
let mut synth = Device::open(candidate)?;
println!("{} on {}", synth.driver().info().model, synth.port_name());
```

Then run operations on `synth` as above. A scan needs the feature `midir`,
which is on by default.

## What may change

Oscyl is before version 1.0. Any release can change any public item of
any crate. A change to the driver contract needs an RFC first
([GOVERNANCE](https://github.com/oscyl/oscyl/blob/main/GOVERNANCE.md#when-an-rfc-is-required)),
and [the changelog](https://github.com/oscyl/oscyl/blob/main/CHANGELOG.md)
names each release's changes.

- Name a tag, never a branch, so that your build does not change under you.
- Read the changelog before you move to a new tag.
- The `.oscyllib` format has a version number, and new readers keep reading
  old files. See [The .oscyllib library format](../reference/oscyllib-format.md#versioning).

The API reference of the crates is at
[oscyl.github.io/oscyl/api](https://oscyl.github.io/oscyl/api/).
