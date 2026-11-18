# What happens when you plug in a synth

This page follows a synth from the moment you plug it in to the moment
Oscyl writes to it. It also says what Oscyl writes, and when.

```text
 plug in
    |
    v
 find the synth -------- device inquiry; the answer names the synth
    |
    v
 read every slot ------- the synth's memory does not change
    |
    v
 edit in Oscyl --------- Oscyl's copy changes
    |        \
    |         `-- Live on: the change plays in the edit buffer only
    v
 Sync ------------------ back up the slots, then write only those slots
```

## Oscyl finds the synth

Each [driver](glossary.md#driver) knows the names of the MIDI ports its synth
shows. When a port with such a name appears, Oscyl sends it the MIDI *device
inquiry*. The synth answers with its identity, and the driver checks that
the answer comes from its synth.

On Linux the desktop app sees a synth when you plug it in. On macOS and
Windows, you choose **Connect** after you plug it in. On Android, the
system offers Oscyl when the synth is plugged in, and Oscyl connects it.

## Oscyl reads the synth

Oscyl then reads every program slot, one slot at a time, in the
background. You can work while it reads. On a minilogue xd or a prologue,
Oscyl also lists the user units in the synth's unit slots when a page needs
them.

Reading changes nothing on the synth. From now on, Oscyl works from the copy
it read.

## You edit, and Live plays the edit

An edit changes Oscyl's copy of the program and nothing else. The program
in the list shows that it is edited, and each control shows the value it
had before.

When **Live** is on, every edit also plays on the synth. Oscyl sends the
change to the synth's *edit buffer*: the working copy of the program that
the synth plays now. For most parameters Oscyl sends one MIDI Control Change
or NRPN, so held notes keep sounding. For the others, it sends the whole
program to the edit buffer. **Hold to compare** plays the saved program
while you hold it.

The edit buffer is not the synth's memory. The synth forgets the edit
buffer when you pick another program on it, and its stored programs do not
change.

On a minilogue xd, Live also works the other way. A knob that you turn on
the synth moves the same control in Oscyl, as an edit. A program that you
pick on the synth opens in Oscyl.

## Sync writes, after a backup

**Sync** is the only action in the apps that writes programs to the synth.

1. Sync lists every slot that it will change and asks you to confirm.
2. Oscyl saves a backup of those slots as it read them, in its backups
   folder. If the backup fails, Oscyl writes nothing.
3. Oscyl writes only those slots, one at a time.
4. A slot counts as written only when the synth confirms it. When a synth
   sends no confirmation, the driver reads the slot back and compares it.

No slot that you did not edit changes, and Oscyl never writes a factory
slot. The minilogue xd and the prologue save to flash memory later than they
answer. So their driver can send a program twice, or send a neighbouring
slot the bytes it already holds. Then the synth saves before it answers.

When you unplug the synth, your edits that are not synced stay in Oscyl.
The phone app keeps them on disk, so they stay even when the app closes.

## What can write to the synth

| What you do | What changes on the synth |
|---|---|
| Plug in, read, edit | Nothing |
| Live on, edit or pick a program | The edit buffer only |
| Hold to compare | The edit buffer only |
| Pick a sound in Generate | The edit buffer only |
| `oscyl audition`, `oscyl generate --audition` | The edit buffer only |
| Sync | The edited slots, after a backup |
| Load a user unit (minilogue xd, prologue) | One empty unit slot |
| Keep a new oscillator | One empty oscillator slot |
| Clear a unit slot | That slot, after a backup of its unit |
| `oscyl restore` | The slots that differ, after a backup of the whole synth |
| `oscyl units upload`, `clear`, `shelf load`, `make --load` | One unit slot; a unit is backed up before it is replaced or cleared |

The apps load a unit only into an empty slot. The command line replaces a
unit only with `--force`. `oscyl backup` also reads the global settings and
the tuning of a minilogue xd or a prologue, but no command writes them back.

## Where to go next

- [Glossary](glossary.md): the words on this page.
- [The command line](../reference/cli.md): every command that reads or writes.
- [Architecture](../explanation/architecture.md): how the driver, the
  session and the apps share this work.
