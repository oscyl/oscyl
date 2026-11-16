/*
 * The logue SDK oscillator around Oscyl's engine (engine.c). Oscyl writes
 * a patch into `oscyl_patch` in the payload, found by its marker; as built,
 * the region holds only the marker and so plays a plain saw.
 * Copyright (c) 2026 the Oscyl contributors. MIT OR Apache-2.0.
 */
#include "userosc.h"
#include "engine.h"

__attribute__((used, aligned(4))) const uint8_t oscyl_patch[OSCYL_PATCH_SIZE] = {
    'O', 'S', 'C', 'Y', 'L', 'P', 'A', 'T'};

static oscyl_engine_t s_engine;

void OSC_INIT(uint32_t platform, uint32_t api) {
  (void)platform;
  (void)api;
  /* Hide where the bytes come from, so the compiler reads the region as
   * Oscyl wrote it, not as it was compiled. */
  const uint8_t *p = oscyl_patch;
  __asm__("" : "+r"(p));
  oscyl_engine_load(&s_engine, p, OSCYL_PATCH_SIZE);
}

void OSC_CYCLE(const user_osc_param_t *const params, int32_t *yn, const uint32_t frames) {
  float buf[OSCYL_CHUNK];
  for (uint32_t done = 0; done < frames;) {
    uint32_t n = frames - done;
    if (n > OSCYL_CHUNK) n = OSCYL_CHUNK;
    oscyl_engine_render(&s_engine, params->pitch, params->shape_lfo, buf, n);
    for (uint32_t i = 0; i < n; i++) {
      float y = buf[i];
      if (y > 0.999999f) y = 0.999999f;
      if (y < -1.0f) y = -1.0f;
      yn[done + i] = (int32_t)(y * 2147483648.0f);
    }
    done += n;
  }
}

void OSC_NOTEON(const user_osc_param_t *const params) {
  (void)params;
  oscyl_engine_note_on(&s_engine);
}

void OSC_NOTEOFF(const user_osc_param_t *const params) { (void)params; }

void OSC_PARAM(uint16_t index, uint16_t value) { oscyl_engine_param(&s_engine, index, value); }
