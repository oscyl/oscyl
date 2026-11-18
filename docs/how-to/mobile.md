# Build, run and test the mobile apps

This guide builds, runs and tests the Android and iOS apps. They are in
`apps/mobile` (Compose Multiplatform) and use the same Rust core as the
desktop app, through `crates/oscyl-ffi`. Gradle builds the Rust core
itself. [Repository layout](../reference/layout.md) names the app's
modules, and [The mobile bindings](../reference/mobile-bindings.md) lists
what the Rust core gives the app.

## What you need for Android

- A JDK 21.
- The Android SDK, with `ANDROID_HOME` set.
- The NDK r30 (`ndk;30.0.16248370`). Oscyl finds it through
  `ANDROID_NDK_HOME`, or takes the newest one under `$ANDROID_HOME/ndk` or
  `~/Android/Sdk/ndk`.
- cargo-ndk: `cargo install cargo-ndk`.
- The Rust targets: `rustup target add aarch64-linux-android x86_64-linux-android`.

## Run the Android app on the emulator

Use any x86_64 virtual device:

```sh
emulator -avd <name> -no-window &   # or with a window
cd apps/mobile && ./gradlew :androidApp:installDebug
adb shell am start -n io.github.oscyl/.android.MainActivity
```

The emulator has no USB. Pick a simulated synth on the welcome screen, or
later choose "Try the demo" on the Synth page.

## Run the Android app on a phone

1. Turn on USB debugging on the phone, and connect it.
2. Install the app:

   ```sh
   cd apps/mobile && ./gradlew :androidApp:installDebug
   ```

3. Plug the synth into the phone's USB-C port, through an OTG adapter or a
   USB-C to USB-B cable. The phone powers the synth's USB port.
4. Android offers Oscyl for the synth. Oscyl connects it.

`adb logcat -s OscylMidi` shows the port names that Android reports. The
app finds a minilogue xd on a Pixel 7 Pro. Testers with other phones and
synths are welcome.

To make an APK file instead, run `./gradlew :androidApp:assembleDebug` in
`apps/mobile`. The APK is
`androidApp/build/outputs/apk/debug/androidApp-debug.apk`.

## Run the iOS app

You need a Mac with Xcode and a JDK 21. The build finds the JDK with
`/usr/libexec/java_home`, or uses `JAVA_HOME`. Add the Rust targets:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
```

1. Open `apps/mobile/iosApp/iosApp.xcodeproj`.
2. Pick a simulator, then choose Run.

Xcode's "Compile Kotlin" phase runs
`./gradlew :shared:embedAndSignAppleFrameworkForXcode`. That builds the
Rust core for the target and the `Shared` framework. The simulator has no
USB: open Synth and choose "Try the demo". The project builds arm64 only.

The iOS app has not run on an iPhone yet. To try it on one:

1. Set your Apple developer team in `Configuration/Config.xcconfig`
   (`TEAM_ID=`). The bundle ID becomes `io.github.oscyl<TEAM_ID>`, so a free
   personal team can sign it.
2. Plug the synth into the iPhone: USB-C directly, or Apple's Lightning to
   USB 3 Camera Adapter on an iPhone with Lightning.
3. In Console.app, filter on `OscylMidi` to see the port names that CoreMIDI
   reports.

For an App Store build, set the bundle ID without the team suffix, and an
App Store provisioning profile. The version numbers follow the release:
[Cut a release](release.md) sets them.

### When you add or remove a source file

The "Compile Kotlin" phase lists its inputs in
`Configuration/KotlinInputs.xcfilelist`, so Xcode skips the phase when no
Kotlin or Rust source changed. When you add or remove a source file, run:

```sh
scripts/mobile-assets.sh --inputs
```

`scripts/ci.sh` fails when the list is out of date.

### The icon and the launch screen

`scripts/mobile-assets.sh` makes the iOS icon from the desktop app's SVG,
the launch colours from `design/tokens.json`, and the input list again. It
needs rsvg-convert and ImageMagick. `scripts/mobile-assets.sh --check`
checks the committed files: the plists, the project file, the asset catalogs,
the icon, the launch colour and the input list.

### The privacy manifest

`iosApp/PrivacyInfo.xcprivacy` declares no tracking and no collected data.
It also gives a reason for each required-reason API that the app uses:

- **File timestamps, C617.1:** the Rust core reads the modification times
  of files in its own folders, for example to list backups newest first.
  `llvm-nm` on the iOS static library finds `std::fs::metadata` and
  `Metadata::modified`.
- **System boot time, 35F9.1:** to measure elapsed time. This covers Rust's
  `Instant` and the clocks of the Kotlin/Native and Compose runtimes. Those
  runtimes cannot be inspected on Linux.

App Store Connect checks the manifest when you upload a build. If it names
another API, for example `UserDefaults`, add that API's reason.

## Test the apps

| What | Command, in `apps/mobile` |
|---|---|
| Behaviour tests (Robolectric, with the real Rust core built for this computer) | `./gradlew :shared:check :androidApp:testDebugUnitTest` |
| Screenshot goldens | `./gradlew :androidApp:verifyRoborazziDebug` (see [Update screenshots and goldens](screenshots.md)) |
| Accessibility checks (ATF), on an emulator that Gradle manages | `./gradlew :androidApp:pixel6Api34DebugAndroidTest` (needs KVM) |
| iOS tests, on macOS | `./gradlew :shared:iosSimulatorArm64Test :iosPlatform:iosSimulatorArm64Test` |

If the emulator's software GPU crashes (emulator 36.6 on Linux), put
`android.testoptions.manageddevices.emulator.gpu=host` in
`~/.gradle/gradle.properties`.

`scripts/ci.sh` runs the Android tests when `ANDROID_HOME` is set, and the
accessibility checks only when `/dev/kvm` is usable. On Linux it also checks
what it can for iOS: clippy of the Rust core for both iOS targets, the
`:iosPlatform` klibs and the files above. The iOS workflow
(`.github/workflows/ios.yml`) builds the app on macOS, starts it in a
simulator and keeps a screenshot.

### Fast loops

From fastest to slowest. Use the first loop that can prove the change.

| Loop | Command (in `apps/mobile`) | Time |
|---|---|---|
| One test class | `./gradlew :androidApp:testDebugUnitTest -Poscyl.goldens=false --tests '*EditorBehaviourTest'` | 25 s |
| Every behaviour test | `./gradlew :androidApp:testDebugUnitTest -Poscyl.goldens=false` | 50 s |
| On the phones | `scripts/mobile-phone.sh frames graph` (from the repository root) | 1 min |
| Screenshots too | `./gradlew :androidApp:verifyRoborazziDebug` (`recordRoborazziDebug` after a change that you want) | 2 min |
| What changed | `scripts/ci.sh` | depends on the change |
| Numbers of record | the Macrobenchmark, 5 iterations | 10 min for each phone |

The preview screenshots take nine tenths of the time of the unit tests.
`scripts/ci.sh` runs them only when a file under `apps/mobile`, `design` or
`po` changed.

## Judge smoothness

Judge smoothness on the release-like build only. `installDebug` runs Compose
without optimisation, several times slower, so it says nothing about what a
user gets.

```sh
scripts/mobile-phone.sh                 # R8 build on every attached phone, profile compiled, started
scripts/mobile-frames.py -s <serial>    # frame numbers of every journey, about a minute
scripts/mobile-frames.py -s <serial> graph rail   # only these journeys
```

`mobile-frames.py` drives the installed app with adb and reads `gfxinfo` and
SurfaceFlinger. It needs no Gradle and no instrumentation. Its last column
is the one that the eye sees: the refreshes that the display missed, out of
the frames that it showed.

The column "late" is something else. On Android 11, one slow frame queues
every later frame of the gesture one refresh behind. The screen still gets
one frame for each refresh. While you tune, use `mobile-frames.py`. For
numbers of record, use the Macrobenchmark:

```sh
cd apps/mobile
ANDROID_SERIAL=<serial> ./gradlew -PreleaseBuild=true :androidApp:generateBaselineProfile
ANDROID_SERIAL=<serial> ./gradlew -PreleaseBuild=true :baselineprofile:connectedBenchmarkReleaseAndroidTest \
  -Pandroid.testInstrumentationRunnerArguments.class=io.github.oscyl.baselineprofile.FrameBenchmarks#knobDrag \
  -Pandroid.testInstrumentationRunnerArguments.iterations=2
```

The benchmark build (R8, with the shipped Baseline Profile) installs as
`io.github.oscyl.bench`. To install a debug build beside your own Oscyl,
add `-Poscyl.idSuffix=.redesign` (or another suffix) to the Gradle command.

[Mobile smoothness](../explanation/mobile-smoothness.md) records what was
slow, why, and the numbers before and after.

## Build the Rust libraries by hand

Gradle builds the Rust core itself. `scripts/mobile-rust.sh` builds the same
libraries and bindings alone. `scripts/ci.sh` runs it in the stage "ffi
bindings".

```sh
scripts/mobile-rust.sh            # output in target/mobile
scripts/mobile-rust.sh some/dir   # or elsewhere
```

It makes:

- `jniLibs/arm64-v8a/liboscyl_ffi.so` and `jniLibs/x86_64/liboscyl_ffi.so`
  (phone and emulator), as Android's `jniLibs` expects them;
- `kotlin/io/github/oscyl/core/oscyl_ffi.kt`, the stock UniFFI Kotlin
  bindings.

The stock bindings only check that the library's metadata generates. The
app takes its bindings from the Kotlin Multiplatform plugin
`ch.ubique.uniffi.plugin` 1.3.x, which needs UniFFI 0.32.0 exactly. So
`Cargo.toml` pins `uniffi = "=0.32.0"`. The Kotlin package is set in
`crates/oscyl-ffi/uniffi.toml`.

Without cargo-ndk or an NDK, the script builds the bindings only, and says
that it skipped the Android libraries.
