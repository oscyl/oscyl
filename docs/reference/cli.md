# The command line

`oscyl` is Oscyl's command line tool, for every desktop system. It comes
with each release, or runs from the source as `cargo run -p oscyl-cli --`.
This page lists every command and option, as `oscyl --help` and each
command's `--help` give them.

```text
oscyl [OPTIONS] <COMMAND>
```

## Global options

| Option | What it does |
|---|---|
| `--port <PORT>` | Only consider MIDI ports whose name contains this text. |
| `--synth <SYNTH>` | Which kind of synth, by driver id, when no file says. `oscyl ports` lists the ids. |
| `--simulator` | Use a simulated synth, with its demo programs, instead of MIDI hardware. The environment variable `OSCYL_SIMULATOR` does the same. |
| `-h`, `--help` | Print the help. |
| `-V`, `--version` | Print the version. |

A command that needs a synth looks for exactly one synth on the ports that
a driver claims. When it finds more than one, give `--synth` or a more
exact `--port`.

## Files that Oscyl keeps

Oscyl keeps its files in `$XDG_DATA_HOME/oscyl`, or in
`~/.local/share/oscyl` when `XDG_DATA_HOME` is not set:

- `backups/`: the automatic backups of `restore`, and the unit backups of
  `units upload --force` and `units clear`;
- `units/`: the shelf.

## Commands that read

### `oscyl ports`

List the MIDI input and output ports, then the synths that Oscyl supports.
Needs no synth.

```sh
oscyl ports
```

### `oscyl info`

Show the connected synth's model, port, MIDI channel and firmware.

```sh
oscyl info
```

### `oscyl current`

Print the edit buffer: the program that the synth plays now, which can be
unsaved. It prints the name, then one line for each field.

| Option | What it does |
|---|---|
| `--raw` | Print the program's bytes as hex instead. |

```sh
oscyl current --raw
```

### `oscyl backup <OUT>`

Read every program and save them to a library: `.oscyllib`, or one of the
synth's own formats, by the extension of `OUT`. On a synth with user units
(minilogue xd, prologue), the backup also holds the global settings, the
tuning and the units.

```sh
oscyl backup my-xd.oscyllib
```

### `oscyl diff <LIBRARY>`

List the programs that differ between a library and the synth.

```sh
oscyl diff my-xd.oscyllib
```

## Commands that write to the synth

### `oscyl restore <LIBRARY>`

Write a library to the synth. Oscyl reads the synth, saves an automatic
backup of it, then writes only the programs that differ. It writes nothing
when the backup fails. It never writes a factory slot.

| Option | What it does |
|---|---|
| `--all` | Write every program, also the unchanged ones. |

```sh
oscyl restore my-xd.oscyllib
```

### `oscyl audition <FILE>`

Play a program from a file, in the edit buffer, without storing it.

| Option | What it does |
|---|---|
| `--index <INDEX>` | Which program of a library, from 0. Default: 0. |

```sh
oscyl audition my-xd.oscyllib --index 12
```

## Generate

### `oscyl generate`

Deal new sounds, each with a recipe code that makes it again. Needs no
synth, except for `--audition` and `--units`. Give one of `--character`,
`--code`, `--vary` or `--breed`.

| Option | What it does |
|---|---|
| `--list` | List the characters that the synth (`--synth`) can generate. |
| `--character <CHARACTER>` | The character, by id (see `--list`). |
| `--bright <BRIGHT>` | Filter openness, 0 to 100. Default: 50. |
| `--movement <MOVEMENT>` | Modulation, 0 to 100. Default: 50. |
| `--grit <GRIT>` | Drive, noise and other dirt, 0 to 100. Default: 50. |
| `--space <SPACE>` | Delay, reverb and other ambience, 0 to 100. Default: 50. |
| `--seed <SEED>` | The first sound's seed. The others follow from it. Random when left out. |
| `--code <CODE>` | Deal the sound that a recipe code names, first. |
| `--count <COUNT>` | How many sounds. Default: 6, or 1 with `--code`. |
| `--out <OUT>` | Write each sound into this folder as a program file. |
| `--audition` | Play the first sound on the synth, without storing it. |
| `--units` | Let the sounds use the user units that the synth holds (about one sound in three). Needs the synth, or `--simulator`. |
| `--vary <FILE>` | Deal variations of the program in a file instead. |
| `--breed <A> <B>` | Deal children of the programs in two files instead. |

```sh
oscyl generate --character fuzz-bass --seed 7 --count 3
```

## User units

### `oscyl units`

Manage the logue SDK user oscillators and effects of a minilogue xd or a
prologue. Without `--synth`, the commands work on the first driver with user
units, the minilogue xd. Slot numbers start at 1.

| Command | What it does | Writes |
|---|---|---|
| `units list` | Show what each user slot holds. | Nothing |
| `units upload <FILE> --slot <SLOT>` | Load a unit file into a slot. `--force` replaces a unit already there, after a backup of it. | One slot |
| `units clear --module <MODULE> --slot <SLOT>` | Empty a slot, after a backup of its unit. `MODULE` is `osc`, `modfx`, `delfx` or `revfx`. | One slot |
| `units info <FILE>` | Show what a unit file says about itself: section, name and parameters. Needs no synth. | Nothing |
| `units shelf add <FILES>...` | Copy unit files onto the shelf. | Nothing |
| `units shelf list` | List the shelf, each unit with its parameters. | Nothing |
| `units shelf remove <NAME>` | Take a unit off the shelf, by name. | Nothing |
| `units shelf load <NAME>` | Load a unit from the shelf into the first free slot of its section. | One empty slot |
| `units make` | Make a new oscillator with Oscyl's oscillator unit and show it. Needs no synth, except for `--load`. | With `--load`, one empty slot |

`units make` takes these options:

| Option | What it does |
|---|---|
| `--family <FAMILY>` | What the oscillator is built around: `wavetable`, `fm`, `fold`, `formant`, `noise` or `hybrid`. Default: any, by the seed. |
| `--seed <SEED>` | The same seed and family make the same oscillator. Random when left out. |
| `--load` | Put the oscillator on the shelf and load it into the first free oscillator slot. |
| `--out <OUT>` | Write it as a unit file. |

```sh
oscyl units make --family fm --seed 3 --out fm3.mnlgxdunit
oscyl --synth korg-prologue units list
```
