#!/usr/bin/env bash
# Checks scripts/release-notes.sh against a small changelog.
set -euo pipefail

cd "$(dirname "$0")"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

cat > "$tmp/CHANGELOG.md" <<'MD'
# Changelog

## [Unreleased]

### Added
- Next.

## [1.2.0] - 2026-01-02

### Fixed
- One. [1.2.0] is not a heading.

## [1.1.0] - 2026-01-01

### Added
- Two.

[Unreleased]: https://example.org/compare/v1.2.0...HEAD
[1.2.0]: https://example.org/compare/v1.1.0...v1.2.0
MD

expect() { [ "$1" = "$2" ] || { printf 'expected:\n%s\ngot:\n%s\n' "$2" "$1" >&2; exit 1; }; }
expect "$(./release-notes.sh 1.2.0 "$tmp/CHANGELOG.md")" $'### Fixed\n- One. [1.2.0] is not a heading.'
# The last section stops at the link references.
expect "$(./release-notes.sh 1.1.0 "$tmp/CHANGELOG.md")" $'### Added\n- Two.'
for bad in 9.9.9 Unreleased 1.2 1.2.0.1; do
    if ./release-notes.sh "$bad" "$tmp/CHANGELOG.md" >/dev/null 2>&1; then
        echo "release-notes.sh $bad should fail" >&2
        exit 1
    fi
done
# The real changelog: 0.3.0 is exactly its section.
expect "$(./release-notes.sh 0.3.0)" "$(sed -n '/^## \[0\.3\.0\] - /,/^## \[0\.2\.0\]/p' ../CHANGELOG.md | sed '1,2d;$d' | sed '$d')"
echo "release-notes.sh: ok"
