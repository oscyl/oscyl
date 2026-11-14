#!/usr/bin/env bash
# The third-party Rust crates each app ships, with their licence texts, for
# the About pages: the Linux app (apps/linux) and the mobile core
# (crates/oscyl-ffi). Writes the two rust.json files; with --check, fails
# when the committed ones are stale.
#
# Needs cargo-about (cargo install --locked cargo-about --features cli) and jq.
set -euo pipefail

cd "$(dirname "$0")/.."

check=false
[ "${1:-}" = "--check" ] && check=true

if ! command -v jq >/dev/null; then
    echo "scripts/licences.sh needs jq" >&2
    exit 1
fi
if ! command -v cargo-about >/dev/null; then
    cargo install --locked cargo-about --features cli
fi

# cargo-about's model reduced to what the pages show: crates (sorted, each
# pointing at its licence texts) and the texts. Oscyl's own crates (path
# dependencies) are left out: they are Oscyl's licence.
reduce='
  [.licenses[] | select(any(.used_by[]; .crate.source != null))] as $texts
  | {
      crates: ([.crates[] | select(.package.source != null)]
        | unique_by([.package.name, .package.version])
        | map(.package as $p | {
            name: $p.name,
            version: $p.version,
            licence: .license,
            url: ($p.repository // $p.homepage),
            texts: [$texts | to_entries[]
              | select(any(.value.used_by[]; .crate.name == $p.name and .crate.version == $p.version))
              | .key]
          })),
      licences: [$texts[] | {id, name, text}]
    }'

stale=0
generate() { # <manifest> <output>
    # tmp stays global so the EXIT trap still sees it after an abort.
    tmp=$(mktemp)
    trap 'rm -f "$tmp"' EXIT
    cargo about generate --format json --manifest-path "$1" | jq --indent 1 "$reduce" > "$tmp"
    if $check; then
        if ! cmp -s "$tmp" "$2"; then
            echo "$2 is stale: run scripts/licences.sh" >&2
            stale=1
        fi
        rm "$tmp"
    else
        mkdir -p "$(dirname "$2")"
        cat "$tmp" > "$2"
        rm "$tmp"
        echo "wrote $2"
    fi
}

generate apps/linux/Cargo.toml apps/linux/data/licences/rust.json
generate crates/oscyl-ffi/Cargo.toml apps/mobile/shared/src/commonMain/composeResources/files/licences/rust.json
exit $stale
