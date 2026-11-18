# Glossary

The words that Oscyl's apps, code and documentation use, in alphabetical
order. Each entry links to the page that tells more.

### Audition

To play a program on the synth without storing it. Oscyl sends the program
to the [edit buffer](#edit-buffer). See
[What happens when you plug in a synth](plug-in.md).

### Backup

A library file that holds what a synth's slots held before Oscyl wrote to
them. Sync and `oscyl restore` make one first and write nothing when they
cannot. See [The command line](../reference/cli.md).

### Bank

A numbered run of [slots](#slot) on a synth, for example U1 on the Prophet
Rev2. A bank is writable, or it is a factory bank that Oscyl never writes.
See [The driver contract](../reference/driver-contract.md).

### Character

A kind of sound that Generate can make, such as "fuzz bass" or "power chord". Each
driver lists its own characters. See
[What Oscyl is](what-is-oscyl.md#what-oscyl-does).

### Conformance suite

The checks that every driver must pass against its
[simulator](#simulator), in the crate `oscyl-conformance`. CI runs them on
every commit. See [The driver contract](../reference/driver-contract.md).

### Driver

The code that teaches Oscyl one synth model: its messages, its program bytes,
its files and its editor pages. A driver is a Rust crate under
`crates/drivers/`. See [Architecture](../explanation/architecture.md).

### Edit buffer

The synth's working copy of the program that it plays now. Oscyl changes it
to [audition](#audition) a program. The synth forgets it when you pick
another program, and its stored programs do not change.

### Field

One parameter of a program, such as a filter's cutoff, with its range of
raw values. A driver lists its fields in a fixed order. See
[The driver contract](../reference/driver-contract.md).

### Library

A list of programs for one synth model, kept in a file on your computer or
phone. Oscyl's own format is `.oscyllib`. Oscyl also reads and writes each
synth's own file formats. See
[The .oscyllib library format](../reference/oscyllib-format.md).

### Live

The switch in the apps that plays every edit on the synth at once, in the
[edit buffer](#edit-buffer). On a minilogue xd, Live also follows the
synth's knobs and program changes. See
[What happens when you plug in a synth](plug-in.md#you-edit-and-live-plays-the-edit).

### Oscillator unit

One prebuilt logue SDK unit, made by Oscyl, whose sound comes from data.
Oscyl fills it with a new oscillator and loads it into a minilogue xd or a
prologue, only when you keep one. It is optional, and it is not a synth
engine. See
[The oscillator unit](../explanation/oscillator-unit.md).

### Program

One sound of a synth: all its parameter values and its name, as the synth
stores them in one [slot](#slot). KORG calls it a program. Other makers say
patch or preset.

### Recipe code

A short text that names a sound from Generate: its character, its four
dials and its seed, for example `power-chord-ygy6c734s69jn`. The same code
makes the same sound again, on any synth that has that character.

### Request engine

The part of `oscyl-driver` that runs a synth's protocol. It keeps one
request in flight, matches each reply to it, and fails a request whose reply
does not come. It is sans-IO: it never touches a port or a clock. See
[Architecture](../explanation/architecture.md#the-request-engine).

### Session

A driver's protocol for one connected synth, behind the trait
`DriverSession`. Most sessions use the [request engine](#request-engine).
See [The driver contract](../reference/driver-contract.md).

### Shelf

The unit files that Oscyl keeps for each synth, ready to load into a free
[unit slot](#user-unit). See [The command line](../reference/cli.md).

### Simulator

A driver's model of its synth, in code. It answers messages as the synth
does, with no sound. Tests, the conformance suite and the apps' demo mode
use it. See [Try Oscyl without a synth](../tutorials/try-without-a-synth.md).

### Slot

One numbered place for a program in the synth's memory. The apps number
slots from 1, the code from 0.

### Sync

The action that writes your edited programs to the synth, after a
[backup](#backup). See
[What happens when you plug in a synth](plug-in.md#sync-writes-after-a-backup).

### Tier

How far Oscyl trusts a driver. Tier 3 compiles, and Tier 2 passes the
[conformance suite](#conformance-suite). Tier 1 is also checked on the real
synth for each release. See [Supported synths](../reference/synths.md).

### User unit

A user oscillator or effect, made with KORG's logue SDK, in one of the unit
slots of a minilogue xd or a prologue. Oscyl lists them, loads them and
clears them. See [Supported synths](../reference/synths.md).
