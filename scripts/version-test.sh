#!/usr/bin/env bash
# Tests for scripts/version.sh: the numbers it gives, and what it refuses.
set -euo pipefail
cd "$(dirname "$0")/.."

fail=0
expect() {
    local got
    got=$(scripts/version.sh "$1" | tr '\n' ' ')
    if [ "$got" != "$2 " ]; then echo "version.sh $1: got '$got', expected '$2'" >&2; fail=1; fi
}
refuse() {
    if scripts/version.sh "$1" >/dev/null 2>&1; then echo "version.sh accepted '$1'" >&2; fail=1; fi
}

expect 0.4.0 "version=0.4.0 marketing=0.4.0 code=40099"
expect 0.4.0-rc.1 "version=0.4.0-rc.1 marketing=0.4.0 code=40001"
expect 1.2.3 "version=1.2.3 marketing=1.2.3 code=1020399"
expect 0.99.99 "version=0.99.99 marketing=0.99.99 code=999999"
for bad in "" 0.5 0.5.x 1.2.3.4 0.1.100 0.100.0 0.4.0-beta.1 0.4.0-rc 0.4.0-rc.0 0.4.0-rc.99 v0.4.0 " 0.4.0"; do
    refuse "$bad"
done
# A final always outranks its rcs, and an rc outranks the release before.
[ "$(scripts/version.sh 0.4.0 | sed -n 's/^code=//p')" -gt "$(scripts/version.sh 0.4.0-rc.98 | sed -n 's/^code=//p')" ] || { echo "final <= rc" >&2; fail=1; }
[ "$(scripts/version.sh 0.4.0-rc.1 | sed -n 's/^code=//p')" -gt "$(scripts/version.sh 0.3.9 | sed -n 's/^code=//p')" ] || { echo "rc <= previous" >&2; fail=1; }

# Cargo.toml: only [workspace.package]'s version counts.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/scripts"
cp scripts/version.sh "$tmp/scripts/"
printf '[workspace.package]\nedition = "2024"\n\n[workspace.dependencies]\nversion = "9.9.9"\n' > "$tmp/Cargo.toml"
if "$tmp/scripts/version.sh" >/dev/null 2>&1; then echo "version.sh read a version outside [workspace.package]" >&2; fail=1; fi
printf '[package]\nversion = "8.8.8"\n\n[workspace.package]\nversion = "0.4.0"\n' > "$tmp/Cargo.toml"
[ "$("$tmp/scripts/version.sh" | head -n 1)" = "version=0.4.0" ] || { echo "version.sh missed [workspace.package]" >&2; fail=1; }

[ "$fail" = 0 ] && echo "version.sh: ok"
exit "$fail"
