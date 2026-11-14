#!/usr/bin/env bash
# Print the body of CHANGELOG.md's `## [X.Y.Z] - YYYY-MM-DD` section (the
# release notes), without its heading or surrounding blank lines.
# Usage: scripts/release-notes.sh X.Y.Z [changelog]
# Exits non-zero when the version has no dated section.
set -euo pipefail

version=${1:?usage: release-notes.sh X.Y.Z [changelog]}
changelog=${2:-$(dirname "$0")/../CHANGELOG.md}

awk -v v="$version" '
    BEGIN { heading = "## [" v "] - " }
    index($0, heading) == 1 && substr($0, length(heading) + 1) ~ /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/ { found = 1; next }
    found && (/^## / || /^\[[^]]+\]: /) { exit }
    found { lines[++n] = $0 }
    END {
        if (!found) { print "no dated section for " v " in the changelog" > "/dev/stderr"; exit 1 }
        first = 1; while (first <= n && lines[first] ~ /^[[:space:]]*$/) first++
        while (n >= first && lines[n] ~ /^[[:space:]]*$/) n--
        if (first > n) { print "the section for " v " is empty" > "/dev/stderr"; exit 1 }
        for (i = first; i <= n; i++) print lines[i]
    }
' "$changelog"
