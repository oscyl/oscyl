#!/usr/bin/env bash
# The release version and the numbers the stores need, from one parser.
# Usage: scripts/version.sh [VERSION]
# Without VERSION, reads `version` from Cargo.toml's [workspace.package] table
# (and only from there). Accepts X.Y.Z or X.Y.Z-rc.N, with Y, Z under 100 and
# N from 1 to 98; anything else fails with a message. Prints:
#   version=X.Y.Z[-rc.N]   the version as written
#   marketing=X.Y.Z        iOS MARKETING_VERSION (always numeric)
#   code=NNNN              Android versionCode and iOS CFBundleVersion:
#                          ((X*100+Y)*100+Z)*100 + N for an rc, + 99 for a final,
#                          so every rc sorts before its final and after the last one.
set -euo pipefail

if [ $# -gt 0 ]; then
    version=$1
else
    cargo_toml="$(dirname "$0")/../Cargo.toml"
    version=$(awk '
        /^\[/ { in_package = ($0 == "[workspace.package]") ; next }
        in_package && /^version *= *"/ { sub(/^version *= *"/, ""); sub(/".*/, ""); print; exit }
    ' "$cargo_toml")
    if [ -z "$version" ]; then
        echo "Cargo.toml: [workspace.package] has no version" >&2
        exit 1
    fi
fi

if [[ ! $version =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-rc\.([0-9]+))?$ ]]; then
    echo "version '$version' is not X.Y.Z or X.Y.Z-rc.N" >&2
    exit 1
fi
major=$((10#${BASH_REMATCH[1]}))
minor=$((10#${BASH_REMATCH[2]}))
patch=$((10#${BASH_REMATCH[3]}))
rc=${BASH_REMATCH[5]:-}
if [ "$minor" -ge 100 ] || [ "$patch" -ge 100 ]; then
    echo "version '$version': minor and patch must be under 100" >&2
    exit 1
fi
if [ -n "$rc" ]; then
    rc=$((10#$rc))
    if [ "$rc" -lt 1 ] || [ "$rc" -ge 99 ]; then
        echo "version '$version': the rc number must be from 1 to 98 (99 is the final's)" >&2
        exit 1
    fi
    build=$rc
else
    build=99
fi
echo "version=$version"
echo "marketing=$major.$minor.$patch"
echo "code=$(( ((major * 100 + minor) * 100 + patch) * 100 + build ))"
