/*
 * Oscyl's oscillator engine: fixed DSP blocks that a patch wires up.
 * Copyright (c) 2026 the Oscyl contributors. MIT OR Apache-2.0.
 *
 * The same code runs in the logue SDK unit (unit.c) and, compiled for the
 * computer, in Oscyl's tests (crates/oscyl-osc-host). Oscyl writes the
 * patch (crates/drivers/logue/src/osc.rs, which documents the format
 * and holds a Rust port of this file for the apps' pictures).
 *
 * Patch: a 20-byte head (marker "OSCYLPAT", version, block, mod and table
 * counts, body length u16, reserved u16, CRC-32 of the body), then the body:
 * globals (8 bytes), names (96 bytes, unread here), blocks (8 bytes each:
 * type and seven constants), mods (4 bytes each: source, block, constant,
 * signed amount) and tables (256 int16 each). A patch that does not check
 * out, or wires no sound source, plays a plain saw.
 */
#ifndef OSCYL_ENGINE_H
#define OSCYL_ENGINE_H

#include <stdint.h>

#define OSCYL_PATCH_SIZE 4608
#define OSCYL_HEAD 20
#define OSCYL_GLOBALS 8
#define OSCYL_NAMES 96
#define OSCYL_MAX_BLOCKS 8
#define OSCYL_MAX_MODS 16
#define OSCYL_MAX_TABLES 8
#define OSCYL_TABLE_LEN 256
#define OSCYL_CHUNK 64
#define OSCYL_RATE 48000.0f

enum {
  OSCYL_SAW = 1,
  OSCYL_WAVETABLE,
  OSCYL_FM,
  OSCYL_SUPERSAW,
  OSCYL_SUB,
  OSCYL_NOISE,
  OSCYL_PHASE_DIST,
  OSCYL_SYNC,
  OSCYL_FOLD,
  OSCYL_CRUSH,
  OSCYL_LOWPASS,
  OSCYL_FORMANT,
  OSCYL_NUM_TYPES
};

enum {
  OSCYL_SRC_SHAPE = 0,
  OSCYL_SRC_SHIFT_SHAPE,
  OSCYL_SRC_LFO,
  OSCYL_SRC_ENV,
  OSCYL_SRC_PARAM1, /* .. PARAM6 = 9 */
  OSCYL_SRC_KEY = 10,
  OSCYL_NUM_SRC
};

typedef struct {
  uint8_t type;
  float base[7];   /* constants, 0..1 */
  float v[7];      /* this call's values, modulated */
  float k[4];      /* this call's derived coefficients */
  /* state */
  float s[6];
  uint32_t n;
} oscyl_block_t;

typedef struct {
  uint8_t src, block, param;
  float amount;
} oscyl_mod_t;

typedef struct {
  /* wiring */
  oscyl_block_t blocks[OSCYL_MAX_BLOCKS];
  oscyl_mod_t mods[OSCYL_MAX_MODS];
  const uint8_t *tables;
  uint8_t n_blocks, n_mods, n_tables;
  float gain, lfo_rate, env_attack, env_decay, env_sustain;
  uint8_t lfo_shape;
  /* controls */
  float shape, shift_shape, params[6];
  /* state */
  float phase, sub_phase, lfo_phase, lfo_value, env, dc_x, dc_y;
  uint8_t env_stage;
  uint32_t noise;
} oscyl_engine_t;

/* Wire `e` from `patch` (`size` bytes); a bad patch wires a saw. */
void oscyl_engine_load(oscyl_engine_t *e, const uint8_t *patch, uint32_t size);
void oscyl_engine_note_on(oscyl_engine_t *e);
/* A logue SDK parameter: 0-5 the unit's six (0-100), 6 Shape, 7 Shift-Shape
 * (both 0-1023). */
void oscyl_engine_param(oscyl_engine_t *e, uint16_t index, uint16_t value);
/* `frames` (at most OSCYL_CHUNK) samples at `pitch` (note << 8 | fine),
 * with the synth's LFO on Shape (`shape_lfo`, Q31), each within -1..1. */
void oscyl_engine_render(oscyl_engine_t *e, uint16_t pitch, int32_t shape_lfo,
                         float *out, uint32_t frames);

#endif
