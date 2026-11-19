#!/usr/bin/env bash
# Check that every place naming the release version agrees with X.Y.Z, and
# that CHANGELOG.md has its dated section. Run before tagging; the release
# workflow runs it on the tag.
# Usage: scripts/release-check.sh X.Y.Z
set -euo pipefail

cd "$(dirname "$0")/.."
version=${1:?usage: release-check.sh X.Y.Z}
fail=0
check() { if [ "$2" != "$version" ]; then echo "$1 says '$2', expected $version" >&2; fail=1; fi; }

# One parser for every version rule (scripts/version.sh): the tag's version
# must be one it accepts, and Cargo.toml must say the same.
if ! info=$(scripts/version.sh "$version"); then exit 1; fi
marketing=$(sed -n 's/^marketing=//p' <<< "$info")
code=$(sed -n 's/^code=//p' <<< "$info")
check "Cargo.toml [workspace.package] version" \
    "$(scripts/version.sh | sed -n 's/^version=//p')"
# Android takes both its numbers from scripts/version.sh (apps/mobile/build.gradle.kts).
for wired in 'versionName = rootProject.extra\["oscylVersion"\]' 'versionCode = rootProject.extra\["oscylVersionCode"\]'; do
    grep -q "$wired" apps/mobile/androidApp/build.gradle.kts \
        || { echo "apps/mobile/androidApp no longer takes its version from scripts/version.sh ($wired)" >&2; fail=1; }
done
expect() { if [ "$2" != "$3" ]; then echo "$1 says '$2', expected $3" >&2; fail=1; fi; }
expect "apps/mobile/iosApp MARKETING_VERSION" \
    "$(sed -n 's/^MARKETING_VERSION *= *\(.*\)/\1/p' apps/mobile/iosApp/Configuration/Config.xcconfig)" "$marketing"
expect "apps/mobile/iosApp CURRENT_PROJECT_VERSION" \
    "$(sed -n 's/^CURRENT_PROJECT_VERSION *= *\(.*\)/\1/p' apps/mobile/iosApp/Configuration/Config.xcconfig)" "$code"
check "the metainfo's newest release" \
    "$(sed -n 's/^ *<release version="\([^"]*\)".*/\1/p' apps/linux/data/io.github.oscyl.Oscyl.metainfo.xml | head -n 1)"
# The guide to the crates names the newest final release (a release
# candidate leaves it on the last final).
if [[ $version != *-rc.* ]]; then
    for tag in $(grep -o 'tag = "[^"]*"' docs/how-to/use-the-crates.md | sed 's/tag = "\(.*\)"/\1/' | sort -u); do
        check "docs/how-to/use-the-crates.md's tag" "${tag#v}"
    done
fi
scripts/release-notes.sh "$version" >/dev/null || fail=1

[ "$fail" = 0 ] && echo "release $version: versions and changelog agree"
exit "$fail"
