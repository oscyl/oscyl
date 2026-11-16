# Oscyl on mobile

The mobile apps (Compose Multiplatform: Android, and iOS not yet run on a
device) use the same Rust core as the Linux app, through `crates/oscyl-ffi`
(UniFFI).

The first sections say how to run, test and translate the apps. The rest
describe how the code works, layer by layer.

## Run the app

The app is `apps/mobile` (Compose Multiplatform): `shared/` holds the UI
and the bindings, `androidApp/` the Android entry, USB-MIDI and manifest.
It needs a JDK 21, the Android SDK (`ANDROID_HOME`), the NDK and cargo-ndk
(see Tools); Gradle builds the Rust core itself.

On the emulator (any x86_64 AVD):

```sh
emulator -avd <name> -no-window &   # or with a window
cd apps/mobile && ./gradlew :androidApp:installDebug
adb shell am start -n io.github.oscyl/.android.MainActivity
```

The emulator has no USB. Pick a simulated synth on the welcome screen, or
later on Synth with "Try the demo".

On a phone: enable USB debugging, then `./gradlew :androidApp:installDebug`.
The app has not yet run with a synth on a real phone; testers are welcome.
Plug the synth into the phone's USB-C port through an OTG adapter or a
USB-C to USB-B cable (the phone powers the bus); Oscyl offers itself when
the synth is plugged in and connects it on its own. `adb logcat -s
OscylMidi` shows the port names Android reports (see Transport).

Tests: `./gradlew :shared:check :androidApp:testDebugUnitTest` (behaviour,
Robolectric, the real Rust core built for this machine) and
`:androidApp:verifyRoborazziDebug` (screenshots: every `@Preview` is a
golden; `recordRoborazziDebug` rewrites them). The accessibility (ATF)
checks run on a headless emulator Gradle manages:
`./gradlew :androidApp:pixel6Api34DebugAndroidTest` (needs KVM). Where the
emulator's software GPU crashes (emulator 36.6 on Linux), put
`android.testoptions.manageddevices.emulator.gpu=host` in
`~/.gradle/gradle.properties`. `scripts/ci.sh` runs all of these when
`ANDROID_HOME` is set (the managed device only when `/dev/kvm` exists).

## Run the app on iOS

### What you need

- A Mac with Xcode and JDK 21. The build phase finds JDK 21 with
  `/usr/libexec/java_home`, or uses `JAVA_HOME`.
- rustup, with the iOS targets:
  `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`.

### Build and run

1. Open `apps/mobile/iosApp/iosApp.xcodeproj`.
2. Pick a simulator, then Run.
3. Xcode's "Compile Kotlin" phase runs
   `./gradlew :shared:embedAndSignAppleFrameworkForXcode`. That builds the
   Rust core for the target and the `Shared` framework.

The phase lists its inputs in `Configuration/KotlinInputs.xcfilelist`, so
Xcode skips it when no Kotlin or Rust source changed. When you add or
remove a source file, run `scripts/mobile-assets.sh --inputs`; `ci.sh`
checks that the list is current.

The simulator has no USB: open Synth and choose "Try the demo".

### On an iPhone

1. Set your Apple developer team in `Configuration/Config.xcconfig`
   (`TEAM_ID=`). The bundle id becomes `io.github.oscyl<TEAM_ID>`, so a
   free personal team can sign it.
2. Plug the synth into the iPhone's port: USB-C directly, or through
   Apple's Lightning to USB 3 Camera Adapter on Lightning phones.
3. Console.app, filtered on `OscylMidi`, shows the port names CoreMIDI
   reports (see Transport).

For App Store builds, the owner needs to set:

- the bundle id without the team suffix;
- an App Store provisioning profile;
- nothing for the numbers: `MARKETING_VERSION` and
  `CURRENT_PROJECT_VERSION` follow the release (`scripts/version.sh` gives
  both, and `scripts/release-check.sh` checks them), so each version and
  each rc is one upload.

### Icon, launch screen and inputs

- The icon comes from the Linux app's SVG, and the launch colour (the
  ground, light and dark) from `design/tokens.json`.
- `scripts/mobile-assets.sh` regenerates the icon, the launch colour and the
  input list. It needs rsvg-convert and ImageMagick.
- `--check` validates what is committed: the plists, the project file's
  brackets, the asset catalogs, the icon and the launch colour.

### Privacy manifest

`iosApp/PrivacyInfo.xcprivacy` declares no tracking and no collected data.
It also gives a reason for each required-reason API the app uses:

- **File timestamps, C617.1:** the Rust core stats files and reads their
  modification times in its own folders, for example to list backups
  newest first. This was checked with `llvm-nm` on the iOS static library:
  `std::fs::metadata` and `Metadata::modified`.
- **System boot time, 35F9.1:** used to measure elapsed time. This covers
  Rust's `Instant`, and the Kotlin/Native and Compose runtimes' monotonic
  clocks. Those runtimes can't be inspected on Linux.

App Store Connect's upload validation is the final check. If it names
another API, for example `UserDefaults`, add its reason there.

The project builds arm64 only. `Config.xcconfig` excludes the x86_64
simulator slice, which neither the framework nor the Rust core has.

### Tests

- On macOS: `./gradlew :shared:iosSimulatorArm64Test
  :iosPlatform:iosSimulatorArm64Test`.
- `.github/workflows/ios.yml` also builds the app, launches it in a
  simulator and keeps a screenshot.
- On Linux, `ci.sh` proves what it can: clippy of the Rust core for both
  iOS targets, the `:iosPlatform` klibs, and the files above.

## Translating

The app's own strings are Compose resources:
`apps/mobile/shared/src/commonMain/composeResources/values/strings.xml`
(English) and `values-<lang>/strings.xml` beside it (Italian today). Every
key in English needs one in each language (`StringsCompleteTest`). Use
positional arguments (`%1$s`), plurals for counts, and a comment above
any string whose context is not obvious. Only the launcher label lives in
`apps/mobile/androidApp/src/main/res/values/strings.xml` (the brand name,
not translated); the MIDI errors Android and iOS share are Compose
resources too.

Debug and test builds also carry a generated pseudo-locale, `en-XA`
(accented, about 40% longer, bracketed), to catch clipping and
concatenation: pick "English (XA)" in the developer options' languages,
or see the `pseudoLocale_xa` screenshot goldens. Release builds
(`-PreleaseBuild=true`) leave it out.

Licence names and texts on the About page are never translated.

## Third-party licences

`scripts/licences.sh` writes the Rust crates' licences for both apps
(`apps/linux/data/licences/rust.json`,
`apps/mobile/shared/src/commonMain/composeResources/files/licences/rust.json`)
with cargo-about; run it after changing dependencies (`scripts/ci.sh`
checks with `--check`). The Kotlin and Android libraries' notices are
exported by the AboutLibraries Gradle plugin on every build.
## Rust libraries by hand

Gradle builds the Rust core itself. This script builds the same libraries
and bindings on their own, which is what `scripts/ci.sh` checks.

```sh
scripts/mobile-rust.sh            # output in target/mobile
scripts/mobile-rust.sh some/dir   # or elsewhere
```

It produces:

- `jniLibs/arm64-v8a/liboscyl_ffi.so` and `jniLibs/x86_64/liboscyl_ffi.so`
  (phone and emulator), laid out as Android's `jniLibs` expects;
- `kotlin/io/github/oscyl/core/oscyl_ffi.kt`, the stock UniFFI Kotlin
  bindings. They only check that the library's metadata generates: the app
  takes its bindings from the Kotlin Multiplatform plugin
  `ch.ubique.uniffi.plugin` 1.3.x, which needs UniFFI exactly 0.32.0 (hence
  the pinned `uniffi = "=0.32.0"`). The Kotlin package is set in
  `crates/oscyl-ffi/uniffi.toml`.

`scripts/ci.sh` runs it too (stage "ffi bindings"). Without cargo-ndk or an
NDK it builds the bindings only and says it skipped the Android libraries.

## Tools

- Android NDK (found through `ANDROID_NDK_HOME`, else the newest one under
  `$ANDROID_HOME/ndk` or `~/Android/Sdk/ndk`);
- `cargo install cargo-ndk`;
- `rustup target add aarch64-linux-android x86_64-linux-android`.

## Transport

`oscyl-midi`'s ALSA code (midir, hotplug, port scanning) sits behind its
default feature `midir`; `oscyl-ffi` builds without it. On mobile the
platform owns the USB MIDI ports (Android `MidiManager`, CoreMIDI) and moves
the bytes:

- the platform finds ports and asks `candidates(portNames)` which drivers
  may answer on them;
- `Synth.openLink(driverId, port, out)` opens one: Rust sends through the
  platform's `MidiOut.send(bytes)` (an exception there disconnects the
  synth), and the returned `MidiInputSink` takes what the synth sends:
  `push(bytes)` in any fragments (SysEx is framed in Rust), `disconnect()`
  when the port goes away (idempotent; a late `push` is ignored). Its
  Kotlin `close()` is UniFFI's `AutoCloseable.close()`: it releases the
  object, after which any call throws, so call it only once no receiver
  can push any more;
- `port` names one physical port and must be unique among the open ones
  (on Android, include the `MidiDeviceInfo` id, e.g. `"42:minilogue xd"`):
  a second `openLink` on a port already open or opening ends in
  `ConnectFailed` and its sink is never read;
- the device inquiry confirms the synth, as on Linux; `Synth.gone(port)`
  drops it on unplug. Releasing a `MidiInputSink` without `disconnect()`
  disconnects it too.

On Android, `AndroidMidi` names each port as ALSA does, `<product> _ <port
name>` (or the product alone when the USB port has no name), so the drivers'
hints such as `minilogue xd _ SOUND` can match; it logs the raw values under
the `OscylMidi` tag. This is unverified until the first run with a phone and
an xd.

Events (`SynthEvent`) reach the app's `SynthListener` on a Rust thread of
their own, in order; hop to the main dispatcher there. `Synth.openDemo`
connects a driver's simulated synth.

On unplug, the closed input can end the synth before `gone(port)` arrives:
then an `Error` for that id (saying the synth was disconnected) comes right
before its `Disconnected`. Treat such an `Error` followed by `Disconnected`
for the same id as status, not as an error dialog.

Shutdown: call `Synth.quit()` when done (`ViewModel.onCleared`, `close`).
After it, every call is a no-op. Dropping the `Synth` also quits, but only
as a fallback: the event thread holds the listener and the worker holds each
`MidiOut`, so if either refers back to the `Synth` (a ViewModel listening to
its own synth) the drop never comes, and only `quit()` stops the threads and
releases the device.

## Libraries and documents

Library logic lives in `oscyl-app`; the bindings only carry it across.

- `Libraries.open(dir)` opens (and makes) Oscyl's libraries folder: on
  Android `filesDir/libraries`. It converts libraries in the old
  `.mnlgxdlib` format once, keeping the originals in `before-oscyllib/`,
  and empties the last run's trash. Every `Libraries` method reads or
  writes that folder, so call it on `Dispatchers.IO`. `outline()` gives
  the tree depth first (folders and libraries, whether each is shown, a
  library's synth and program count, or why it can't be read).
  `create`/`add`/`newFolder`/`rename`/`duplicate`/`place`/`step`/`setOpen`
  change it; `delete` returns a `Trashed` for `restore` (Undo) or `purge`
  (the undo has gone). `newLibraryName()`/`newFolderName()` give "Library
  1", "Folder 1"… not taken.
- `Libraries.read(id)` gives a `Document` to edit; `save(id, doc)` writes
  it back (the app saves after every change, as on desktop).
  `Document.synth(driverId, title)` is a synth's slots: hand it each
  `SynthEvent.Program` with `receive(slot, data)`.
- `Document` is cheap to call from the main thread: `rows()` gives the
  whole list in one call (slot, name, edited, read-only factory slot,
  read yet); `setName`, `newProgram`, `remove`, `reorder`, `copy` (a
  `Clip`), `paste`/`pasteOver`, `revertProgram`, `clearProgram`,
  `undo`/`redo` change it, each change one undo step; `revision()`
  changes with every edit.
- `programMenu(doc, index, libraries, libraryId, clipboard)` says what a
  program's menu holds, in order, what is enabled, and which libraries
  Copy to Library offers.
- Files cross as bytes; the platform owns pickers and URIs.
  `importFile(fileName, bytes)` reads an Oscyl library or a synth's own
  format (by extension, else by contents) and refuses more programs than
  a library holds; `exportFormats(driverId)` lists what a library exports
  to (the synth's usual format first, `.oscyllib` last) and
  `exportFile(doc, ext)` writes one. Both parse or build the whole file:
  call them off the main thread.
- Errors are `FfiError`: `Failed` (a file or the folder, with the reason)
  and `Refused` (a factory slot, a name the synth can't hold). Reasons
  the core words go through its translation hook; file system reasons
  keep the system's words.

## Preferences and language

- `Settings.load(path)` reads the preferences (`oscyl-app` settings, the
  file the desktop uses too, here `filesDir/settings.json`): theme, the
  app's own animation switch, haptics, demo mode's driver, the first-run
  answer. Setters change them; `save()` writes them (blocking).
- `setLocale(tag)` makes core and driver words (page and field names,
  values, summaries, refusals, worker errors) follow the app's language,
  from the `po/*.po` catalogs built into the library (`oscyl-app`
  `catalog`); call it at start and when the language changes. The app's
  own words stay Compose resources.

## Synth work

- `Synth.readAll`/`cancelRead` read a synth again or stop; `readCurrent`
  reads the program it plays (a `Current` event). `Document.row(i)`
  gives one row, so a read updates the list slot by slot.
- User units (logue family): `readUnit(driverId, module, bytes)` checks
  a unit file for a section; `Synth.uploadUnit` loads it,
  `Synth.clearSlot` backs the slot up to a file (`unitBackupPath`) and
  clears it, never without the backup. `unitExt`, `unitSection`.
- `backups(dir)` lists the backups folder, newest first; `readFile`
  reads one. `keepUnsynced`/`unsynced`/`forgetUnsynced` keep a
  connected synth's unsynced edits on disk between runs.
  `newLibraryDriver` picks a new library's synth, as on the desktop.
- `ProgramEditor.update(pages, points)` and `facts()` give what an edit
  changed and the top row's state in one call each.

## Generate and the welcome

- `Generator(driverId, seed)` holds the core's hands (`oscyl-app`
  `generator`, shared with the desktop): `characters`, `macros`,
  `pages`, `deal`, `vary`, `breed`, `step`, `select`, `markKept`,
  `selectedProgram` and `hand(points)` (tiles with their pictures). The
  same seed deals the same bytes.
- `CharacterInfo.family` ("keys", "bass", "lead", "pad", "rhythm",
  "effect") groups the characters; the page's family row filters by it.
  `Generator.surprise()` and `dealCode(code)` deal and return what they
  chose (`Dealt`: character and dials) for the page to show; `morph(other)`
  deals six steps between two sounds; `code()` is the picked sound's
  recipe code (none for a variation, a child or a morph step).
  `Files.shareText` offers the code to another app.
- `Document.put(index, data)` puts a program over a slot or at the end
  of a library (refused when read-only or full).
- `Settings.onboarded`/`role` keep the welcome's answer; `role` picks the
  start page (Organise: Library; the others: the editor).
- `ProgramEditor.startPage(role)` is the page a program opens at: the
  oscillators for sound design, the voice for a performance.
- `Document.demo(driverId, title)` is a synth's demo library. The driver
  makes the sounds, so no vendor data is shipped. The welcome keeps it as
  a library.
- `Generator.hand` draws each tile from the page whose pictures differ
  most across the hand, so the tiles tell the sounds apart.
- On iOS, `Files.opened` carries files other apps open in Oscyl
  (`onOpenURL`). Oscyl owns `.oscyllib`, and synths' library files open
  as an alternate handler.

## The editor and the review

Editor logic lives in `oscyl-app` (`editor`, `review`); the GTK app uses
it too.

- `Document.editor(index)` opens program `index` as a `ProgramEditor`
  (a new one after the document reorders or removes programs).
  `pages()` gives the stages (title, the Linux app's icon id, intro,
  sections with their fields); `fields(page)`/`field(f)` each field as
  the program shows it now (label, `ControlKind` dial/segments/menu/Off-On,
  raw and saved raw, option and saved option, value texts, relevant,
  edited, help): query again after an edit, the control and the label
  can change with other fields. `stages()` gives every stage's summary,
  edit count and which sections apply; `graphs(page, points)` the
  stage's picture as plain marks (`GraphShape`) now and, when it
  differs, as saved; `handles(page)` its draggable points.
- Edits: `set(field, raw)`, `choose(field, option)` (segments, menus,
  Off/On), `revertField(field)`; each is one undo step of the document
  until `seal()` (call it when a gesture ends). They are refused
  (`FfiError.Refused`) on a factory slot or a slot not read yet, and
  return what to play: `Audition.Live` (send with `Synth.live`) or
  `Audition.Program` (`Synth.audition`). `dragTo(page, handle, x, y)`
  sets every field a handle moves, the whole drag one undo step until
  `seal()` (`drag` only says which fields it would set).
  `setComparing(true)` shows the saved program everywhere and ignores
  edits; `program()` is the program shown, to audition.
- `Document.review()` lists each edited program's changes in words;
  `markSaved()` keeps a library's edits (nothing shows as edited);
  `syncPlan()` says what a sync writes (refused until every edited slot
  has been read), `writeBackup(dir, doc, indices)` backs up what those
  slots held (blocking), then `Synth.write`; on `Written`, hand each
  written program back with `doc.receive(slot, data)`: it becomes the
  slot's saved version, so an edit made while the write ran stays edited. `auditionTarget(synths,
  active, driverId)` picks the synth that plays a program.

On Android, `AndroidFiles` (in `androidApp`) opens and creates documents
through the Storage Access Framework and shares through the share sheet
(a `FileProvider` over one file in the app's cache); the shared
`Files` interface takes it, and iOS has a stub until M7. On the
emulator, `adb push some.mnlgxdlib /sdcard/Download/`, then Import file
in Library.

## iOS platform

- `:iosPlatform` holds the iOS platform code that needs no Rust core.
  - **CoreMIDI** (`CoreMidi`): one client. Its setup-changed notification
    lists the ports again, and each source is paired with a destination of
    the same entity. Bytes go out with `MIDISend`, and come in through an
    input port's block.
  - **Files** (`Documents`): the document picker opens and exports a file.
    Export asks for the place first, then writes the bytes. The share sheet
    is a popover on iPad.
  - **Haptics:** UIKit's selection, impact and notification generators.
  - **Storage and language:** Application Support for storage, and
    `NSLocale.preferredLanguages` for the language.
- It has no cinterop, so `scripts/ci.sh` compiles its klibs on Linux
  (`kotlin.native.enableKlibsCrossCompilation`). `:shared`'s UniFFI
  bindings use cinterop, so `:shared` builds for iOS on macOS only.
- `shared/src/iosMain` adapts it to the shared contracts (`IosPlatform`).
- **Rust side:** the transport stays the platform's, as on Android. The
  same `Synth.openLink`, `MidiOut` and `MidiInputSink` carry the bytes.
  `midir` (CoreMIDI) was not used because it gives no hot-plug
  notifications: the port list would need polling.
- **Shared by both platforms:** `MidiLink` (early bytes, then the
  close-order rules), `midiPortName`, `AppStorage` and the MIDI error
  words live in `commonMain`. Android uses them too.

## Editor drag performance

The plan's target is an editor drag at p90 16 ms or less on a release
build. Measured on 2026-11-13 with `dumpsys gfxinfo`: a release build
(`-PreleaseBuild=true`) on the Pixel_10_b emulator (gfxstream, host
GPU), dragging Cutoff in its mid range so the filter graph redraws
every frame, three runs of about 220 frames each:

- p50 21–22 ms, p90 27 ms, p95 27–28 ms.
- GPU p90 23 ms.
- The UI thread was slow on 0–2 frames per run.

The UI thread work is done: the graph keeps its paths, dashes and
strokes from frame to frame, and an edit asks the core once.

### Smoothness run (2026-11-13)

Release build on the same emulator, frame stats from `dumpsys gfxinfo`
per gesture, warm (after the first pass).

| Gesture | p50 | p90 | p95 | Slow UI-thread frames | GPU p90 |
|---|---|---|---|---|---|
| 500-slot list, flings both ways | 16 ms | 16 ms | 16 ms | 0 of 118–294 | 14 ms |
| Editor drag (Cutoff, mid range) | 22–24 ms | 27–28 ms | 27–30 ms | 0–1 of ~220 | 23 ms |
| Stage-rail scrub (hold, then drag through every stage and back) | 21 ms | 25–31 ms | 32 ms | 3–4 of ~410 | 19–21 ms |

- **List:** smooth. Its first, cold pass composed new rows and was slower
  (20 slow UI frames of 75), so each row's words are now looked up once
  per list instead of once per row.
- **Editor:** the drag and the scrub are GPU-bound, as above. The p90 is
above target because the GPU takes the rest. This is the emulator's
GPU, so the numbers are not representative of a phone. Measure on a
real device before setting a new target; the owner decides on any new
target.


### Redesign, on real phones (2026-11-13)

The phone-first redesign measured on the owner's Pixel 7 Pro (120 Hz)
and Pixel 2 XL (60 Hz) over adb, with `dumpsys gfxinfo` per journey.
These are **debug builds** (debuggable, no R8, no Baseline Profile)
installed beside the owner's own Oscyl as `io.github.oscyl.redesign`:

```sh
cd apps/mobile && ./gradlew :androidApp:assembleDebug -Poscyl.idSuffix=.redesign
adb -s <serial> install -r androidApp/build/outputs/apk/debug/androidApp-debug.apk
```

Debug Compose runs several times slower than release, so these are an
upper bound, not the spec's verdict.

| Journey | Pixel 7 Pro p50 / p90 / p95 | Pixel 2 XL p50 / p90 / p95 |
|---|---|---|
| Editor scroll (6 flings) | 10 / 22 / 26 ms | 12 / 36 / 109 ms |
| Knob drag (slow and fast, up and down) | 9 / 13 / 14 ms | 24 / 34 / 40 ms |
| Destination switch (5 switches) | 6 / 11 / 17 ms | 8 / 21 / 30 ms |
| Graph grow and Back (3 times) | 10 / 25 / 40 ms | 32 / 53 / 73 ms |

The spec's targets (p95 8.3 ms on the 7 Pro, 16.7 ms on the 2 XL) are
for a release build with R8 full mode and Baseline Profiles. That part
of the plan is still to do: the `:baselineprofile` module, a release
variant signed for local benchmarking, the Macrobenchmark
`FrameTimingMetric` journeys, then tuning what misses. On the debug
build the slowest spots are the knob drag on the 2 XL (each value
recomposes the field and its section) and the graph's first growth (the
full-screen page composes its pager and the controls sheet).

The destination switch was recorded with `adb shell screenrecord` and
checked frame by frame: the old page fades out, the ground shows for a
few frames, the new one fades in; no frame mixes the two or flashes.

### Redesign, release-like (2026-11-13)

The benchmark variant (R8, the shipped Baseline Profile) installs as
`io.github.oscyl.bench`. The `:baselineprofile` module holds the profile
generator and the frame benchmarks:

```sh
cd apps/mobile
ANDROID_SERIAL=<serial> ./gradlew -PreleaseBuild=true :androidApp:generateBaselineProfile
ANDROID_SERIAL=<serial> ./gradlew -PreleaseBuild=true :baselineprofile:connectedBenchmarkReleaseAndroidTest \
  -Pandroid.testInstrumentationRunnerArguments.class=io.github.oscyl.baselineprofile.FrameBenchmarks#knobDrag \
  -Pandroid.testInstrumentationRunnerArguments.iterations=2
```

FrameTimingMetric, `frameDurationCpuMs` p50 / p90 / p95 in ms (5 iterations;
the 2 XL's knob, graph and scroll rows are later 2-iteration runs):

| Journey | Pixel 7 Pro (120 Hz, target p95 8.3) | Pixel 2 XL (60 Hz, target p95 16.7) |
|---|---|---|
| Editor scroll | 4.4 / 6.0 / 6.7 | 7.7 / 27.6 / 28.2 |
| Program list scroll | – | 4.4 / 8.8 / 9.5 |
| Knob drag | 6.2 / 9.9 / 10.7 | 12.7 / 27.3 / 27.8 |
| Graph grow and back | 7.3 / 14.4 / 15.4 | 26.0 / 62.4 / 73.3 |
| Destination switch | 2.6 / 6.7 / 11.2 | 7.9 / 17.1 / 20.7 |
| Synth sheet | 4.9 / 6.2 / 6.9 | 11.0 / 12.5 / 14.0 |

On the 7 Pro the frame overrun p95 is negative (on time) for scroll, knob
and sheet. On the 2 XL the render thread sets the pace (9–13 ms of GPU
command flush per full-screen frame on its QHD+ panel). Still above target:
the graph's growth and return, the 2 XL's knob and scroll, and the
destination switch.

### Smoothness pass (2026-11-13)

Judge smoothness on the release-like build only. `installDebug` runs Compose
unoptimised and is several times slower: it says nothing about what a user gets.

```sh
scripts/mobile-phone.sh                 # R8 build on every attached phone, profile compiled, started
scripts/mobile-frames.py -s <serial>    # frame numbers of every journey, about a minute
scripts/mobile-frames.py -s <serial> graph rail   # only these journeys
```

`mobile-frames.py` drives the installed app with adb and reads `gfxinfo` and
SurfaceFlinger: no Gradle, no instrumentation. Its last column is the one the eye
sees: refreshes the display missed out of the frames it showed. "late" is not
that: on Android 11 one slow frame leaves every later frame of the gesture queued
a refresh behind while the screen still gets one per refresh. The Macrobenchmark
(above) stays for numbers of record; while tuning, use this.

What was slow, found with `atrace` and `simpleperf record --app` on the 2 XL:

- **Blurred shadows on the UI thread.** `Modifier.dropShadow` blurs a mask on the
  CPU again for every new size, so a growing rail, a dragged sheet or a sliding
  plate cost 35 to 57 ms a frame. Every shadow is now `softShadow`
  (`ui/parts/SoftShadow.kt`): gradients the GPU shades at any size.
- **Panels composed inside a frame.** A fling, a rail jump or the way back from the
  full-screen graph composed whole sections of knobs (15 to 190 ms). The editor's
  list keeps the program composed around the page (`LazyLayoutCacheWindow`), filled
  in idle time; the list stays composed under the full-screen graph, and the
  graph's page stays composed (hidden) under the list.
- **The graph's growth** measured and drew the picture again at every size. It is
  now a transform of the page's picture on its layer (`Portrait` in
  `EditorScreen.kt`), with the page's ground and header fading and its sheet rising.
- **The controls sheet** changed its height per frame. It now slides by translation,
  rises under a drag anywhere on it before its list scrolls, and lowers from the
  list's top, on a tap on the picture, or on Back.
- **Each knob step** asked the core for the head and for five written values. The
  head is asked once per gesture; a turning knob's values are written once per turn.
- **Layout jumps.** "was N" is printed in the foot of the knob's scale; a selector's
  "was X" stands at the end of its legend's row. Nothing moves when a control
  becomes edited, and no empty line waits under a selector.

Refreshes missed / frames shown on the Pixel 2 XL (60 Hz), same journeys, before
and after (the graph journey opens and closes the full-screen graph three times):

| Journey | Before | After |
|---|---|---|
| Graph grow and back | 30 / 102 | 1 to 4 / 99 |
| Editor scroll | 2 to 7 / 127 | 1 to 3 / 127 |
| Knob drag | 0 to 5 / 127 | 0 to 2 / 127 |
| Stage rail, held and scrubbed | 4 to 6 / 127 | 0 to 1 / 127 |

On the Pixel 7 Pro (120 Hz) the graph journey went from frames of up to 83 ms to
1 or 2 missed of 58; knob and rail miss none. Its scroll figure is not comparable:
the panel changes its refresh rate while the page moves.

### Second pass (2026-11-14)

Found with `simpleperf record --app ... -g --trace-offcpu --clockid monotonic`, its samples
cut to the slow frames of `dumpsys gfxinfo framestats` (so time blocked counts too):

- **The profile was never compiled in.** `cmd package compile -m speed-profile` only uses
  a profile the app's installer has written; straight after `adb install` there is none,
  and the app ran interpreted. `scripts/mobile-phone.sh` now broadcasts
  `androidx.profileinstaller.action.INSTALL_PROFILE` first and fails if it is refused.
  Numbers taken before this date were of an interpreted app.
- **The Baseline Profile is older than the redesign** (no rule for `ScrollPeek`,
  `DockedSheet`, `SoftShadow`, `SheetLayer`): what it misses runs interpreted until the
  JIT has it. Generate it again before a release.
- **A fade through an offscreen copy** (`graphicsLayer { alpha }` below 1) makes that
  copy, as large as what fades, in the first frame of the motion: 30 ms and more of the
  render thread as a scroll began (the island's dim), at each destination switch (two
  whole screens) and as the graph page came in. Those layers now fade stroke by stroke
  (`CompositingStrategy.ModulateAlpha`). Use it for any fade of a large part.
- **A synth's read handled in one go.** The demo synth answers 500 slots at once and the
  model took them all in one turn of the main loop. They are now taken in 4 ms slices.
- **Work for a list nobody was looking at.** Each knob release bumped the document's
  revision: every row of the program list (composed under the editor) asked the core for
  its picture again, and the list's menu for the whole history. Pictures of the untouched
  rows are kept across an editor change; the history is read when the menu opens.
- **`RowWords` had no `equals`**, so every recomposition of the program list recomposed
  every row. A class passed to every item of a list must compare by value.
- **The graph page's sheet** composes its neighbours' knobs one section per frame while
  the pager rests, and keeps them measured: the change at a swipe's halfway composes nothing.

Refreshes missed / frames shown on the Pixel 2 XL, profile compiled in both, app warm:

| Journey | Before | After |
|---|---|---|
| Program list fling | 4 to 7 / 127 | 0 to 3 / 127 |
| Editor scroll | 3 to 5 / 127 | 0 to 1 / 127 (one pass of 4 and 5 right after an install) |
| Knob drag | 4 to 6 / 127 | 0 to 3 / 127 |
| Destination switch | 2 / 121 warm, 5 to 12 from a fresh start | 2 / 120 warm, 4 to 5 from a fresh start |
| Graph grow and back | 5 / 68 | 4 to 5 / 93 (the render thread's first frame of each growth) |

Passes differ by a few refreshes on this phone; the slow frames named above are gone in
every pass. Still there: one render-thread frame of 20 to 30 ms as the graph's page first
draws, and two frames of 20 to 40 ms as a destination comes back (its tree is placed and
drawn again; the motion waits for them, so nothing stutters, but the tap answers late).

### Fast loops

From fastest to slowest; use the first that can prove the change.

| Loop | Command (in `apps/mobile`) | Time |
|---|---|---|
| One test class | `./gradlew :androidApp:testDebugUnitTest -Poscyl.goldens=false --tests '*EditorBehaviourTest'` | 25 s |
| Every behaviour test | `./gradlew :androidApp:testDebugUnitTest -Poscyl.goldens=false` | 50 s |
| On the phones | `scripts/mobile-phone.sh frames graph` (from the repository root) | 1 min |
| Screenshots too | `./gradlew :androidApp:verifyRoborazziDebug` (`recordRoborazziDebug` after a wanted change) | 2 min |
| What changed | `scripts/ci.sh` | by change |
| Numbers of record | the Macrobenchmark, 5 iterations | 10 min a phone |

The preview screenshots are nine tenths of the unit suite's time. `scripts/ci.sh`
runs them only when a file under `apps/mobile`, `design` or `po` changed.
