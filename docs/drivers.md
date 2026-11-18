# Writing a synth driver

A driver teaches Oscyl one synth: how to talk to it, what its program bytes
mean, how its files are laid out, and how the editor should show it. Each
app gets every feature for free once the driver is in: the library, sync,
audition, the editor, Generate, the CLI and the mobile apps.

This guide uses the Sequential Prophet Rev2 driver
(`crates/drivers/sequential-prophet-rev2`) as its worked example. It is a
single synth with a published protocol, no shared family crate and no user
units, so it is the simplest complete driver. The KORG prologue shows how two
synths share a family crate (`crates/drivers/logue`).

## The contract

The traits live in `crates/oscyl-driver/src/contract.rs`. Why they look the
way they do is in [ADR 0002](adr/0002-driver-contract.md). A driver is a
compiled-in crate that exposes `pub static DRIVER`. Apps only ever see
`&dyn Driver`.

- **`Driver`:**
  - **Identity:** `info` (an id such as `sequential-prophet-rev2`, the
    vendor, the model), `recognize` (does this identity reply come from your
    synth) and `port_hint` (port names to try first).
  - **For the synth picker:** `tier` (its GOVERNANCE.md tier; 3 by
    default) and `voices` (the polyphony of the models it covers). The
    picker's nameplate is drawn from the driver's own pages and fields
    (`oscyl_app::panel`), so there is nothing to supply.
  - **What the synth has:** `capabilities` (banks and their slots, which
    ones are writable, the edit buffer, live changes, unit slots).
  - **The program:** `fields` (the schema: name, group, range),
    `init_program`, `check_program`, `get`/`set`, and `name`/`set_name`
    with `name_rules`.
  - **Files:** `file_formats`, `import` and `export`. Exporting what you
    imported must give back the same library.
  - **The editor:** `curation` (pages and sections). Also optional:
    `control`, labels, `format_value`, `page_summary`, `graphs` and
    `handles`/`drag`, `help`, `relevant`, `category`. Every one of these
    has a sensible default.
  - **Generate** (optional): `characters`, `macro_info`, `generate`, `vary`,
    `breed` and `generated_name`.
  - **Live edits:** `live_messages` (the CC or NRPN for one field, for
    live audition).
  - **The protocol:** `session` and `simulator`.
- **`DriverSession`** is the protocol, sans IO. You `submit` an operation
  (read a slot, write a batch, audition). Oscyl feeds it bytes
  (`handle_input`) and time (`handle_timeout`), and takes what it must send
  (`poll_transmit`) and what happened (`poll_event`). It never touches a
  port or a clock, so it runs the same on desktop, Android and iOS, and in
  tests.
- **`Simulator`** is the synth itself, sans IO. It takes a SysEx message and
  returns the replies, keeps programs and an edit buffer, and can `inject`
  the faults seen on the real hardware (`faults` lists them). It powers the
  conformance suite and the demo mode.

## Step by step

### 1. Research the synth

Work from public documents: the maker's MIDI implementation chart, the
manuals, and open references with a compatible licence. Then watch your own
synth: record what it sends and answers, and compare dumps before and after
one change. Send it read requests only until you know what a write does.

Name your sources in the crate's module docs and in the pull request.

### 2. Make the crate

Create `crates/drivers/<vendor>-<model>`. The workspace already includes
every crate under `crates/drivers/`. Copy the Rev2's `Cargo.toml`: the only
dependency is `oscyl-driver`, and `oscyl-conformance` is a dev-dependency.

The Rev2 splits its code like this, and a new driver usually can too:

| Module | What it holds |
|---|---|
| `msg.rs` | SysEx messages: build them, decode replies |
| `program.rs` | The program bytes, packing, the name |
| `fields.rs` | The parameter schema, with each field's offset and range |
| `syx.rs` | The file formats (`.syx` for the Rev2) |
| `session.rs` | The `DriverSession`: reads, writes and their completion signals |
| `sim.rs` | The `Simulator`, with its faults (`lose_reply`, `answer_before_store`) |
| `curate.rs` | The editor's pages and sections |
| `generate.rs` | Generate's characters and recipes (optional) |
| `driver.rs` | `impl Driver` and `pub static DRIVER` |

Rules that save trouble:

- Only synthetic test fixtures. Never commit vendor firmware, factory
  programs or files from the internet. Keep local copies under `private/`,
  which git ignores.
- A write is done only when the synth says so. If the synth gives no
  completion signal, the session reads the slot back. The conformance suite
  checks that no fault ever reports false success.
- The simulator must reproduce the faults you saw on the hardware, so the
  suite can prove the session survives them.

### 3. Pass the conformance suite

Add `tests/conformance.rs`:

```rust
#[test]
fn prophet_rev2_conforms() {
    oscyl_conformance::run(&oscyl_prophet_rev2::DRIVER);
}
```

`oscyl_conformance::run` checks, among other things:

- **The program:** init programs, fields and names.
- **Bad input is refused:** wrong program lengths and bad targets.
- **Files:** they round-trip.
- **The protocol:** write and read back, audition and progress. No fault
  reports a false success, and lost replies fail.
- **The editor:** the curation covers every field, and every value
  formats.
- **The rest:** presentation, generation (deterministic and valid), live
  messages (well formed), and the app hooks.

Run it:

```sh
cargo test -p oscyl-prophet-rev2 --test conformance
```

### 4. Register the driver

- Add it to `DRIVERS` in `crates/oscyl-midi/src/device.rs`, and as a
  dependency in `crates/oscyl-midi/Cargo.toml`. Every app and the CLI find
  drivers there.
- Add the package to the `conformance` matrix in `.github/workflows/ci.yml`.
  `scripts/ci.sh` finds it on its own, and fails if `tests/conformance.rs`
  is missing.
- Name its maintainer in `.github/CODEOWNERS` for the crate and its notes.
- Add a row to the README's supported synths table, at Tier 2.

Then try it end to end with its simulator:

```sh
cargo run -p oscyl-cli -- --simulator --synth sequential-prophet-rev2 info
cargo run -p oscyl-gtk -- --simulator
```

### 5. Translations (optional)

A driver's user-facing strings (page titles, labels, value names, help) can
be translated. List the driver's id in `TRANSLATED` in
`apps/linux/src/pot.rs`, then regenerate the template (see
[po/README.md](https://github.com/oscyl/oscyl/blob/main/po/README.md)). Today only the minilogue xd is translated;
other drivers stay English.

## Hardware validation and tiers

[GOVERNANCE.md](https://github.com/oscyl/oscyl/blob/main/GOVERNANCE.md) defines the tiers. What each one means for
a driver:

| Tier | What it takes |
|---|---|
| Tier 3 (community) | It compiles. |
| Tier 2 (conformance-proven) | Steps 1 to 4 above. The conformance suite passes against the simulator on every commit. |
| Tier 1 (hardware-validated) | Tier 2, plus a named maintainer who validates it on the real synth for every release. |

For hardware validation:

1. Back up the whole synth before any write: `oscyl backup <file>`.
2. Write only to scratch slots, and restore them afterwards.
3. Check connect, reading one program, a full backup, writing one program
   until its completion signal, and audition.
4. Finish with `oscyl diff <backup>`, and record the firmware version.

Never commit captures or dumps; keep them in `private/`, which git ignores.

A Tier 1 failure blocks the release ([releasing.md](releasing.md)).
