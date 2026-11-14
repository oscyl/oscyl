#!/usr/bin/env bash
# Vendor the crates for the offline Flatpak build into
# packaging/flatpak/cargo-sources.json (not committed; rerun after Cargo.lock
# changes). Fetches flatpak-cargo-generator at a pinned commit, checks its
# hash, and runs it with uv (or pipx), which install its inline dependencies.
set -euo pipefail

cd "$(dirname "$0")/.."

rev=9d86d336a39b9af38b86f114b6ce5820fe260f91
sum=0a2db6be87d75910facef28ab46d4d6460802e8419ab850d0caa6a364d26b380
gen=target/flatpak-cargo-generator.py

mkdir -p target
curl -fsSL -o "$gen" \
    "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/$rev/cargo/flatpak-cargo-generator.py"
echo "$sum  $gen" | sha256sum -c --quiet

if command -v uv >/dev/null 2>&1; then
    run=(uv run --script)
else
    run=(pipx run)
fi
"${run[@]}" "$gen" Cargo.lock -o packaging/flatpak/cargo-sources.json
