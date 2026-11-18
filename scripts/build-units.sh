#!/usr/bin/env bash
# Build Oscyl's runtime units (units/*) for every logue platform they run on
# and keep the payloads next to their source, so no app needs a toolchain.
# Writes units/oscyl-osc/payload/<platform>.bin and SOURCE, the cksum of
# the sources the payloads were built from (a test fails when it is stale).
#
# Needs arm-none-eabi-gcc (GCC_BIN_PATH, default /usr/bin). The SDK files in
# units/oscyl-osc/sdk are from KORG's logue-sdk (BSD-3-Clause, see
# sdk/LICENSE): platform/minilogue-xd/inc/userosc.h, each platform's
# inc/userprg.h, dummy-osc/tpl/_unit.c and dummy-osc/ld/{userosc,rules}.ld,
# unchanged. Flags follow the SDK's oscillator Makefile.
set -euo pipefail

cd "$(dirname "$0")/.."
unit=units/oscyl-osc
bin=${GCC_BIN_PATH:-/usr/bin}

# The files a payload is made from, in the order SOURCE hashes them; the
# test (crates/oscyl-osc-host) reads the same list.
sources=(
    "$unit/engine.h" "$unit/engine.c" "$unit/unit.c" "$unit/osc_api.h"
    "$unit/sdk/userosc.h" "$unit/sdk/_unit.c" "$unit/sdk/ld/userosc.ld" "$unit/sdk/ld/rules.ld"
    "$unit/sdk/minilogue-xd/userprg.h" "$unit/sdk/prologue/userprg.h"
    scripts/build-units.sh
)

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

cflags=(
    -mcpu=cortex-m4 -mthumb -mno-thumb-interwork -DTHUMB_NO_INTERWORKING -DTHUMB_PRESENT
    -Os -mlittle-endian -mfloat-abi=hard -mfpu=fpv4-sp-d16 -fsingle-precision-constant
    -std=c11 -W -Wall -Wextra
    -DSTM32F401xC -DCORTEX_USE_FPU=TRUE -DARM_MATH_CM4 -D__FPU_PRESENT
)

for platform in minilogue-xd prologue; do
    objs=()
    for c in "$unit/sdk/_unit.c" "$unit/engine.c" "$unit/unit.c"; do
        o="$out/$platform-$(basename "$c" .c).o"
        "$bin/arm-none-eabi-gcc" "${cflags[@]}" -I"$unit" -I"$unit/sdk" -I"$unit/sdk/$platform" -c "$c" -o "$o"
        objs+=("$o")
    done
    "$bin/arm-none-eabi-gcc" "${cflags[@]}" -nostartfiles "${objs[@]}" \
        -Wl,--library-path="$unit/sdk/ld",--script="$unit/sdk/ld/userosc.ld" -lm -o "$out/$platform.elf"
    "$bin/arm-none-eabi-objcopy" -O binary "$out/$platform.elf" "$unit/payload/$platform.bin"
    chmod 644 "$unit/payload/$platform.bin"
    "$bin/arm-none-eabi-size" "$out/$platform.elf"
done

cat "${sources[@]}" | cksum > "$unit/payload/SOURCE"
echo "wrote $unit/payload/{minilogue-xd,prologue}.bin and SOURCE"
