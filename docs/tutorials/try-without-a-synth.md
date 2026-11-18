# Try Oscyl without a synth

In this tutorial you run Oscyl against a simulated minilogue xd. You read
its programs, back it up, make new sounds with Generate, and then edit and
sync a program in the desktop app. You need no synth: the simulator answers
like the real one, with no sound.

It takes about 20 minutes, most of it for the first build.

## What you need

- A clone of the repository and Rust 1.92 or newer, through
  [rustup](https://rustup.rs).
- On Linux, the development packages of ALSA (`alsa-lib`). For the desktop
  app in step 6, also GTK 4.20 or newer, libadwaita 1.8 or newer, and gettext.

The command line parts work on Linux, macOS and Windows. The desktop app
part uses Linux. [Build and run](../how-to/build.md) gives the commands for
the other systems.

## 1. Get the code

```sh
git clone https://github.com/oscyl/oscyl
cd oscyl
```

## 2. See which synths Oscyl knows

```sh
cargo run -p oscyl-cli -- ports
```

The first build takes a few minutes. Then Oscyl lists the MIDI ports of
your computer, and after them the synths it supports:

```text
synth: korg-minilogue-xd (KORG minilogue xd)
synth: sequential-prophet-rev2 (Sequential Prophet Rev2)
synth: korg-prologue (KORG prologue)
```

The first word after `synth:` is the driver's id. You use it with `--synth`.

## 3. Connect the simulated synth

```sh
cargo run -p oscyl-cli -- --simulator info
```

`--simulator` connects a simulated synth instead of a MIDI port. The first
driver, the minilogue xd's, makes it:

```text
synth: KORG minilogue xd
port: simulated minilogue xd
channel: 1
firmware: 2.10
```

Now ask for the program that the synth plays, its edit buffer:

```sh
cargo run -p oscyl-cli -- --simulator current
```

Oscyl prints the program's name and then every field with its value:

```text
name: Glass Choir
Voice / Octave: 2
Voice / Portamento: 0
Voice / Key Trig: 0
```

## 4. Back up the synth

```sh
cargo run -p oscyl-cli -- --simulator backup my-synth.oscyllib
```

Oscyl reads all 500 programs, then the synth's other data, and saves them
in one library file:

```text
read everything in 38.4ms
saved my-synth.oscyllib
```

Compare the file with the synth:

```sh
cargo run -p oscyl-cli -- --simulator diff my-synth.oscyllib
```

Nothing changed since the backup, so Oscyl says:

```text
0 of 500 programs differ
```

## 5. Make new sounds

Generate needs no synth. List the characters that the minilogue xd driver
can make:

```sh
cargo run -p oscyl-cli -- generate --list
```

Each line is a character's id, its family and what it sounds like:

```text
power-chord  lead    Driven fifths for riffs
fuzz-bass    bass    Low, fuzzy and tight
```

Deal three fuzz basses from seed 7:

```sh
cargo run -p oscyl-cli -- generate --character fuzz-bass --seed 7 --count 3
```

Each sound has a name and a recipe code:

```text
Deep Thud     fuzz-bass-000001v4s69jf
Dirt Growl    fuzz-bass-ygy6c734s69jc
Tar Fuzz      fuzz-bass-qarjm0k4s69j2
```

A recipe code makes its sound again. Write the second one to a file:

```sh
cargo run -p oscyl-cli -- generate --code fuzz-bass-ygy6c734s69jc --out sounds
```

Oscyl writes `sounds/Dirt Growl.mnlgxdprog`, a program file that KORG's own
software also opens. Play it on the simulated synth:

```sh
cargo run -p oscyl-cli -- --simulator audition "sounds/Dirt Growl.mnlgxdprog"
```

```text
auditioning "Dirt Growl" (366µs)
```

The simulated synth now holds "Dirt Growl" in its edit buffer. A real synth
would play it, and store nothing.

## 6. Edit and sync in the desktop app

Start the desktop app with simulated synths:

```sh
cargo run -p oscyl-gtk -- --simulator
```

The window opens on the simulated minilogue xd. Under its name, the sidebar
says "Simulated, no sound". The middle column lists its 500 programs, and
the editor shows the first one, "Glass Choir".

1. In the row of stages at the top of the editor, click **Filter**.
2. Drag the **Cutoff** dial up.
3. Look at the dial. Under it, Oscyl shows "was" and the value that the
   synth holds. The Filter stage and the program in the list get a mark.
4. Click **Sync** at the top right. A dialog names the one slot that will
   change.
5. Click **Sync** in the dialog.

Oscyl backs up the slot and writes it to the simulated synth. The button
then says **Synced**. The backup is a new file in
`~/.local/share/oscyl/backups`.

## What you did

You connected a synth, read it, backed it up, and compared it with a
library. You made sounds with Generate and played one. You edited a program
and synced it, and Oscyl backed up the slot before it wrote.

Next:

- [What happens when you plug in a synth](../start/plug-in.md) explains
  what you saw: reads, the edit buffer, Live and Sync.
- [Your first driver](first-driver.md) adds a synth of your own.
- [The command line](../reference/cli.md) lists every command.
