# The mobile bindings

What `crates/oscyl-ffi` gives the Android and iOS apps, through UniFFI. The
logic lives in `oscyl-app`, and the bindings carry it across. The same calls
serve both platforms. The names below are the Kotlin names.

## Transport

`oscyl-midi`'s ALSA code (midir, hotplug and port scanning) is behind its
default feature `midir`, and `oscyl-ffi` builds without it. On mobile, the
platform owns the USB MIDI ports (Android's `MidiManager`, iOS's CoreMIDI)
and moves the bytes.

| Call | What it does |
|---|---|
| `candidates(portNames)` | Which drivers can answer on these ports. The platform finds the ports and asks. |
| `Synth.openLink(driverId, port, out)` | Opens one port. Rust sends through the platform's `MidiOut.send(bytes)`. The returned `MidiInputSink` takes what the synth sends. |
| `MidiOut.send(bytes)` | The platform sends bytes. An exception here disconnects the synth. |
| `MidiInputSink.push(bytes)` | The platform pushes received bytes, in fragments of any size. Rust frames the SysEx. |
| `MidiInputSink.disconnect()` | The port went away. A second call does nothing, and a later `push` is ignored. |
| `Synth.gone(port)` | Drops the synth on that port, on unplug. |
| `Synth.openDemo` | Connects a driver's simulated synth. |
| `Synth.quit()` | Ends the work and stops the threads. Call it when done (`ViewModel.onCleared`, `close`). After it, every call does nothing. |

Rules:

- `port` names one physical port, and it must be unique among the open
  ports. On Android, include the `MidiDeviceInfo` id, for example
  `"42:minilogue xd"`. A second `openLink` on a port that is open or
  opening ends in `ConnectFailed`, and its sink is never read.
- The device inquiry confirms the synth, as on Linux. Releasing a
  `MidiInputSink` without `disconnect()` also disconnects it.
- `MidiInputSink.close()` is UniFFI's `AutoCloseable.close()`. It releases
  the object, and every later call throws. Call it only when no receiver
  can push any more.
- Events (`SynthEvent`) reach the app's `SynthListener` in order, on a Rust
  thread of their own. Move to the main dispatcher there.
- On unplug, the closed input can end the synth before `gone(port)` comes.
  Then an `Error` for that id, which says that the synth disconnected, comes
  just before its `Disconnected`. Treat that pair as status, not as an
  error dialog.
- Dropping the `Synth` also quits, but only as a fallback. The event thread
  holds the listener and the worker holds each `MidiOut`. If either refers
  back to the `Synth`, the drop never comes. Only `quit()` then stops the
  threads and releases the device.

On Android, `AndroidMidi` names each port as ALSA does: `<product> _ <port
name>`, or the product alone when the port has no name. A device with
several unnamed ports numbers them: `<product> _ MIDI 2`. So the drivers'
hints, such as `minilogue xd _ SOUND`, match. It logs the raw values with
the tag `OscylMidi`.

## Libraries and documents

| Call | What it does |
|---|---|
| `Libraries.open(dir)` | Opens Oscyl's libraries folder, and makes it if needed (on Android, `filesDir/libraries`). It converts old `.mnlgxdlib` libraries once, keeps the originals in `before-oscyllib/`, and empties the last run's trash. |
| `Libraries.outline()` | The tree, depth first: folders and libraries, whether each one shows, a library's synth and program count, or why it cannot be read. |
| `create`, `add`, `newFolder`, `rename`, `duplicate`, `place`, `step`, `setOpen` | Change the tree. |
| `delete` | Returns a `Trashed`, for `restore` (Undo) or `purge` (the Undo is gone). |
| `newLibraryName()`, `newFolderName()` | "Library 1", "Folder 1" and so on, a name that is not taken. |
| `Libraries.read(id)` | A `Document` to edit. `save(id, doc)` writes it back. The app saves after every change. |
| `Document.synth(driverId, title)` | A synth's slots. Give it each `SynthEvent.Program` with `receive(slot, data)`. |
| `Document.rows()` | The whole list in one call: slot, name, edited, read-only factory slot, read yet. `Document.row(i)` gives one row. |
| `setName`, `newProgram`, `remove`, `reorder`, `copy`, `paste`, `pasteOver`, `revertProgram`, `clearProgram`, `undo`, `redo` | Change a document. Each change is one undo step. `copy` gives a `Clip`. `revision()` changes with every edit. |
| `Document.put(index, data)` | Puts a program over a slot, or at the end of a library. Refused when the document is read-only or full. |
| `programMenu(doc, index, libraries, libraryId, clipboard)` | What a program's menu holds, in order, which items are on, and which libraries "Copy to Library" offers. |
| `importFile(fileName, bytes)` | Reads an Oscyl library or a synth's own file (by extension, else by contents). Refuses more programs than a library holds. |
| `exportFormats(driverId)` | The formats that a library exports to: the synth's usual format first, `.oscyllib` last. |
| `exportFile(doc, ext)` | Writes one file. |

- Every `Libraries` method reads or writes the folder: call it on
  `Dispatchers.IO`. A `Document` is cheap to call from the main thread.
- Files cross as bytes; the platform owns pickers and URIs. `importFile` and
  `exportFile` parse or build the whole file: call them off the main thread.
- Errors are `FfiError`. `Failed` is a file or the folder, with the reason.
  `Refused` is a factory slot, or a name that the synth cannot hold. The
  core's own reasons go through its translation hook. File system reasons
  keep the system's words.

On Android, `AndroidFiles` (in `androidApp`) opens and creates documents
through the Storage Access Framework. It shares through the share sheet,
with a `FileProvider` over one file in the app's cache. The shared `Files`
interface takes it. On the emulator, run
`adb push some.mnlgxdlib /sdcard/Download/`, then choose Import file in
Library.

## Preferences and language

| Call | What it does |
|---|---|
| `Settings.load(path)` | Reads the preferences: the `oscyl-app` settings file that the desktop uses too, here `filesDir/settings.json`. They hold the theme, the app's own animation switch, haptics, demo mode's driver and the first-run answer. Setters change them, and `save()` writes them (blocking). |
| `Settings.onboarded`, `role` | Keep the welcome's answer. `role` picks the start page: Library for Organise, the editor for the others. |
| `setLocale(tag)` | Makes the core's and the drivers' words follow the app's language: page and field names, values, summaries, refusals and worker errors. They come from the `po/*.po` catalogs, built into the library (`oscyl-app` `catalog`). Call it at start and when the language changes. The app's own words stay Compose resources. |

## Synth work

| Call | What it does |
|---|---|
| `Synth.readAll`, `cancelRead` | Read a synth again, or stop. |
| `Synth.readCurrent` | Read the program that the synth plays (a `Current` event). |
| `readUnit(driverId, module, bytes)` | Checks a unit file for a section (logue synths). |
| `Synth.uploadUnit` | Loads a unit into a slot. |
| `Synth.clearSlot` | Backs the slot's unit up to a file (`unitBackupPath`), then clears the slot. It never clears without the backup. |
| `unitExt`, `unitSection` | A synth's unit file extension, and a section's name. |
| `backups(dir)` | The backups folder, newest first. `readFile` reads one backup. |
| `keepUnsynced`, `unsynced`, `forgetUnsynced` | Keep a connected synth's unsynced edits on disk between runs. |
| `newLibraryDriver` | Picks a new library's synth, as on the desktop. |
| `ProgramEditor.update(pages, points)`, `facts()` | What an edit changed, and the state of the top row, in one call each. |

## Generate and the welcome

| Call | What it does |
|---|---|
| `Generator(driverId, seed)` | Holds the core's hands (`oscyl-app` `generator`, shared with the desktop): `characters`, `macros`, `pages`, `deal`, `vary`, `breed`, `step`, `select`, `markKept`, `selectedProgram`. The same seed deals the same bytes. |
| `Generator.hand(points)` | The tiles with their pictures. Each tile is drawn from the page whose pictures differ most across the hand, so the tiles tell the sounds apart. |
| `CharacterInfo.family` | "keys", "bass", "lead", "pad", "rhythm" or "effect". The page's family row filters the characters by it. |
| `Generator.surprise()`, `dealCode(code)` | Deal, and return what they chose (`Dealt`: the character and the dials), for the page to show. |
| `Generator.morph(other)` | Deals six steps between two sounds. |
| `Generator.code()` | The recipe code of the picked sound. A variation, a child or a morph step has none. `Files.shareText` offers the code to another app. |
| `keepPalettes(path)` | Where each synth's user units are remembered, beside the preferences. The worker fills them in from every slot listing. |
| `Generator.units()`, `setUnitOn` | Generate's "My units". `setUnitOn` leaves one out for the session. A `SoundTile` names the `units` that it uses. |
| `keepWarning(onSynth)` | The warning to say once when a sound that uses a unit is kept. |
| `Shelf(dir)` | Keeps unit files: `list`, `add`, `remove`. `Synth.loadFromShelf` puts one into its section's first free slot. |
| `SlotInfo`, `ShelfUnitInfo` | Carry a unit's `params`, each with its range in words. In the editor, a known unit's Param 1 to 6 arrive named and ranged (`FieldState`). |
| `ProgramEditor.startPage(role)` | The page that a program opens at: the oscillators for sound design, the voice for a performance. |
| `Document.demo(driverId, title)` | A synth's demo library. The driver makes the sounds, so Oscyl ships no vendor data. The welcome keeps it as a library. |

On iOS, `Files.opened` carries the files that other apps open in Oscyl
(`onOpenURL`). Oscyl owns `.oscyllib`, and the synths' library files open in
Oscyl as an alternate handler.

## The editor and the review

The editor's logic lives in `oscyl-app` (`editor`, `review`), and the
desktop app uses it too.

| Call | What it does |
|---|---|
| `Document.editor(index)` | Opens program `index` as a `ProgramEditor`. Open a new one after the document reorders or removes programs. |
| `pages()` | The stages: title, the desktop app's icon id, intro, and sections with their fields. |
| `fields(page)`, `field(f)` | Each field as the program shows it now. A field has: label, `ControlKind` (dial, segments, menu, Off/On), raw and saved raw, option and saved option, value texts, relevant, edited, help. Ask again after an edit, because the control and the label can change with other fields. |
| `stages()` | Every stage's summary, edit count and which sections apply. |
| `graphs(page, points)` | The stage's picture as plain marks (`GraphShape`), now and, when it differs, as saved. |
| `handles(page)` | The picture's draggable points. |
| `set(field, raw)`, `choose(field, option)`, `revertField(field)` | Edits. `choose` is for segments, menus and Off/On. Each is one undo step of the document until `seal()`. Call `seal()` when a gesture ends. |
| `dragTo(page, handle, x, y)` | Sets every field that a handle moves. The whole drag is one undo step until `seal()`. `drag` only says which fields it would set. |
| `setComparing(true)` | Shows the saved program everywhere and ignores edits. `program()` is the program that shows, to audition. |
| `Document.review()` | Each edited program's changes, in words. |
| `markSaved()` | Keeps a library's edits: nothing shows as edited. |
| `syncPlan()` | What a sync writes. It is refused until every edited slot has been read. |
| `writeBackup(dir, doc, indices)` | Backs up what those slots hold (blocking). Then call `Synth.write`. |
| `auditionTarget(synths, active, driverId)` | The synth that plays a program. |

- An edit is refused (`FfiError.Refused`) on a factory slot, or on a slot
  that is not read yet.
- An edit returns what to play: `Audition.Live` (send it with `Synth.live`)
  or `Audition.Program` (send it with `Synth.audition`).
- On `Written`, give each written program back with `doc.receive(slot,
  data)`. It becomes the slot's saved version, so an edit made while the
  write ran stays edited.

## iOS platform

`:iosPlatform` holds the iOS code that needs no Rust core:

- **CoreMIDI** (`CoreMidi`): one client. Its setup-changed notification
  lists the ports again. Each source pairs with a destination of the same
  entity. Bytes go out with `MIDISend`, and come in through an input port's
  block.
- **Files** (`Documents`): the document picker opens a file and exports
  one. Export asks for the place first, then writes the bytes. The share
  sheet is a popover on iPad.
- **Haptics:** UIKit's selection, impact and notification generators.
- **Storage and language:** Application Support for storage, and
  `NSLocale.preferredLanguages` for the language.

`:iosPlatform` has no cinterop, so `scripts/ci.sh` compiles its klibs on
Linux (`kotlin.native.enableKlibsCrossCompilation`). The UniFFI bindings of
`:shared` use cinterop, so `:shared` builds for iOS on macOS only.
`shared/src/iosMain` adapts the platform code to the shared contracts
(`IosPlatform`).

On the Rust side, the transport stays the platform's, as on Android: the
same `Synth.openLink`, `MidiOut` and `MidiInputSink` carry the bytes. Oscyl
does not use midir's CoreMIDI backend because it gives no hotplug
notifications, so the port list would need polling.

`MidiLink` (early bytes, then the rules for the close order),
`midiPortName`, `AppStorage` and the MIDI error words are in `commonMain`,
for both platforms.
