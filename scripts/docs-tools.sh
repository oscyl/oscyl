#!/usr/bin/env bash
# Download the documentation tools (mdBook, Vale, lychee) for x86_64 Linux
# into DIR, at pinned versions, and check each archive's SHA-256 first.
# CI uses it; on another system, install the same versions another way.
# Usage: scripts/docs-tools.sh DIR   (then put DIR on PATH)
set -euo pipefail

dir=${1:?usage: scripts/docs-tools.sh DIR}
mkdir -p "$dir"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fetch() { # url sha256
    local file="$tmp/${1##*/}"
    curl -fsSL --retry 3 -o "$file" "$1"
    echo "$2  $file" | sha256sum -c --quiet
}

fetch https://github.com/rust-lang/mdBook/releases/download/v0.5.4/mdbook-v0.5.4-x86_64-unknown-linux-gnu.tar.gz \
    3f28de05dafca9d0f2eab99c662116b0e37b89b1d96a08f8f430b9eeae958cd7
fetch https://github.com/errata-ai/vale/releases/download/v3.27.0/vale_3.27.0_Linux_64-bit.tar.gz \
    c80d62f6bce87b81843ac5b512a26e63213927b957adba0a64fc594570da2000
fetch https://github.com/lycheeverse/lychee/releases/download/lychee-v0.24.2/lychee-x86_64-unknown-linux-gnu.tar.gz \
    1f4e0ef7f6554a6ed33dd7ac144fb2e1bbed98598e7af973042fc5cd43951c9a

tar -xzf "$tmp/mdbook-v0.5.4-x86_64-unknown-linux-gnu.tar.gz" -C "$dir" mdbook
tar -xzf "$tmp/vale_3.27.0_Linux_64-bit.tar.gz" -C "$dir" vale
tar -xzf "$tmp/lychee-x86_64-unknown-linux-gnu.tar.gz" -C "$dir" --strip-components=1 \
    lychee-x86_64-unknown-linux-gnu/lychee
"$dir/mdbook" --version
"$dir/vale" --version
"$dir/lychee" --version
