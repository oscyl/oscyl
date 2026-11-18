#!/usr/bin/env bash
# The app as a user gets it, on every attached phone: R8, the Baseline Profile compiled
# in, not debuggable. Judge smoothness on this build, never on installDebug (a debug
# build runs Compose interpreted, several times slower). Installs as io.github.oscyl.bench
# beside any other Oscyl, with its own data.
#   scripts/mobile-phone.sh            build, install, compile the profile, start
#   scripts/mobile-phone.sh frames ... then scripts/mobile-frames.py on each phone
set -euo pipefail
cd "$(dirname "$0")/../apps/mobile"

# Gradle's daemon outlives a build. A CARGO_TARGET_DIR that an earlier build
# set (a worktree that shares the main checkout's build) reaches the next
# build as an empty string, and cargo refuses it. Always pass a real one.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$(cd ../.. && pwd)/target}"
pkg=io.github.oscyl.bench
./gradlew -q -PreleaseBuild=true :androidApp:assembleBenchmarkRelease
apk=androidApp/build/outputs/apk/benchmarkRelease/androidApp-benchmarkRelease.apk
phones=$(adb devices | awk 'NR > 1 && $2 == "device" && $1 !~ /^emulator-/ { print $1 }')
[ -n "$phones" ] || { echo "no phone attached" >&2; exit 1; }
for s in $phones; do
    (
        adb -s "$s" install -r "$apk" >/dev/null
        # What an install from a store does in the background: compile the profile now.
        # The profile travels in the APK and is not the system's until the app's installer
        # has written it; compiled without that, the app runs interpreted and every number lies.
        adb -s "$s" shell am broadcast -a androidx.profileinstaller.action.INSTALL_PROFILE \
            "$pkg/androidx.profileinstaller.ProfileInstallReceiver" | grep -q 'result=1' ||
            { echo "$s: the Baseline Profile was not installed" >&2; exit 1; }
        adb -s "$s" shell cmd package compile -f -m speed-profile "$pkg" >/dev/null
        adb -s "$s" shell am start -S -W -n "$pkg/io.github.oscyl.android.MainActivity" >/dev/null
        echo "$s: installed"
    ) &
done
wait
if [ "${1:-}" = frames ]; then
    shift
    for s in $phones; do ../../scripts/mobile-frames.py -s "$s" "$@"; done
fi
