#!/usr/bin/env bash
# Build the Rust core for the mobile apps:
#   <out>/kotlin   Kotlin bindings generated from the host library (a smoke
#                  check that the UniFFI metadata generates; the app uses the
#                  Kotlin Multiplatform plugin's own bindings)
#   <out>/jniLibs  liboscyl_ffi.so for arm64-v8a and x86_64, when cargo-ndk
#                  and an Android NDK are found (else skipped)
# Usage: scripts/mobile-rust.sh [out]   (out defaults to target/mobile)
set -euo pipefail

cd "$(dirname "$0")/.."
out=${1:-target/mobile}

cargo build -p oscyl-ffi --features cli
built=${CARGO_TARGET_DIR:-target}/debug
rm -rf "$out/kotlin"
"$built/uniffi-bindgen" generate --library "$built/liboscyl_ffi.so" \
    --language kotlin --no-format --out-dir "$out/kotlin"

if [ -z "${ANDROID_NDK_HOME:-}" ]; then
    sdk=${ANDROID_HOME:-$HOME/Android/Sdk}
    newest=$(ls -d "$sdk"/ndk/*/ 2>/dev/null | sort -V | tail -n 1 || true)
    if [ -n "$newest" ]; then export ANDROID_NDK_HOME=${newest%/}; fi
fi
if ! cargo ndk --version >/dev/null 2>&1 || [ -z "${ANDROID_NDK_HOME:-}" ]; then
    echo "cargo-ndk or the Android NDK not found, skipping the Android libraries"
    exit 0
fi
cargo ndk -t arm64-v8a -t x86_64 -o "$out/jniLibs" build --release -p oscyl-ffi

find "$out" -type f \( -name '*.so' -o -name '*.kt' \) | sort
