/*
 * Oscyl's oscillator engine. See engine.h for the patch format.
 * Copyright (c) 2026 the Oscyl contributors. MIT OR Apache-2.0.
 *
 * Kept free of libc and libm, so the unit and the host build compute the
 * same thing; crates/drivers/logue/src/engine/render.rs ports it to Rust
 * operation for operation (a test holds the two together).
 */
#include "engine.h"

static const float ONE_255 = 1.0f / 255.0f;

static float absf(float x) { return x < 0.0f ? -x : x; }

static float clamp01(float x) { return x < 0.0f ? 0.0f : (x > 1.0f ? 1.0f : x); }

static int32_t floor_i(float x) {
  int32_t i = (int32_t)x;
  return (float)i > x ? i - 1 : i;
}

static float frac(float x) { return x - (float)floor_i(x); }

/* sin(2 pi p): a parabola, refined; error about 0.001. */
static float sin2pi(float p) {
  p = frac(p);
  float sign = 1.0f;
  if (p >= 0.5f) {
    p -= 0.5f;
    sign = -1.0f;
  }
  float s = 16.0f * p * (0.5f - p);
  s = 0.225f * (s * s - s) + s;
  return sign * s;
}

/* 2^x for -126 < x < 127. */
static float exp2_(float x) {
  int32_t i = floor_i(x);
  float f = x - (float)i;
  float m = 1.0f + f * (0.69606564f + f * (0.22449433f + f * 0.07944023f));
  if (i < -126) i = -126;
  if (i > 127) i = 127;
  union {
    uint32_t u;
    float f;
  } scale;
  scale.u = (uint32_t)(i + 127) << 23;
  return m * scale.f;
}

static float blep(float t, float dt) {
  if (dt > 0.5f) dt = 0.5f;
  if (t < dt) {
    t /= dt;
    return t + t - t * t - 1.0f;
  }
  if (t > 1.0f - dt) {
    t = (t - 1.0f) / dt;
    return t * t + t + t + 1.0f;
  }
  return 0.0f;
}

static float saw(float t, float dt) { return 2.0f * t - 1.0f - blep(t, dt); }

static uint32_t next_noise(uint32_t *x) {
  uint32_t v = *x;
  v ^= v << 13;
  v ^= v >> 17;
  v ^= v << 5;
  *x = v;
  return v;
}

static float noise_f(uint32_t *x) { return (float)(next_noise(x) >> 8) * (2.0f / 16777216.0f) - 1.0f; }

static uint32_t crc32(const uint8_t *p, uint32_t n) {
  uint32_t c = 0xFFFFFFFFu;
  while (n--) {
    c ^= *p++;
    for (int k = 0; k < 8; k++) c = (c >> 1) ^ (0xEDB88320u & (0u - (c & 1u)));
  }
  return ~c;
}

static int is_source(uint8_t t) { return t >= OSCYL_SAW && t <= OSCYL_NOISE; }

/* Seconds from a time constant byte: 1 ms to 2 s. */
static float seconds(uint8_t b) { return 0.001f * exp2_((float)b * ONE_255 * 11.0f); }

static const float VOWELS[5][2] = {
    {730.0f, 1090.0f}, {530.0f, 1840.0f}, {270.0f, 2290.0f}, {570.0f, 840.0f}, {300.0f, 870.0f}};

static const float SPREAD[5] = {-1.0f, -0.42f, 0.0f, 0.42f, 1.0f};

static void reset(oscyl_engine_t *e) {
  e->phase = 0.0f;
  e->sub_phase = 0.0f;
  e->env = 0.0f;
  e->env_stage = 0;
  for (int b = 0; b < OSCYL_MAX_BLOCKS; b++) {
    oscyl_block_t *k = &e->blocks[b];
    for (int i = 0; i < 6; i++) k->s[i] = 0.0f;
    k->n = 0;
    if (k->type == OSCYL_SUPERSAW)
      for (int i = 0; i < 5; i++) k->s[i] = 0.2f * (float)i;
  }
}

void oscyl_engine_load(oscyl_engine_t *e, const uint8_t *p, uint32_t size) {
  uint8_t *z = (uint8_t *)e;
  for (uint32_t i = 0; i < sizeof *e; i++) z[i] = 0;
  e->noise = 0x12345678u;

  /* The marker, each byte one higher: the payload holds the marker only
   * where the patch begins. */
  static const uint8_t MARK[8] = {'P', 'T', 'D', 'Z', 'M', 'Q', 'B', 'U'};
  int ok = size >= OSCYL_HEAD;
  for (int i = 0; ok && i < 8; i++) ok = p[i] + 1 == MARK[i];
  uint32_t nb = 0, nm = 0, nt = 0, len = 0;
  if (ok) {
    nb = p[9];
    nm = p[10];
    nt = p[11];
    len = (uint32_t)p[12] | (uint32_t)p[13] << 8;
    ok = p[8] == 1 && nb <= OSCYL_MAX_BLOCKS && nm <= OSCYL_MAX_MODS && nt <= OSCYL_MAX_TABLES &&
         len == OSCYL_GLOBALS + OSCYL_NAMES + 8 * nb + 4 * nm + 2 * OSCYL_TABLE_LEN * nt &&
         OSCYL_HEAD + len <= size;
  }
  if (ok) {
    uint32_t crc = (uint32_t)p[16] | (uint32_t)p[17] << 8 | (uint32_t)p[18] << 16 | (uint32_t)p[19] << 24;
    ok = crc == crc32(p + OSCYL_HEAD, len);
  }
  int source = 0;
  if (ok) {
    const uint8_t *g = p + OSCYL_HEAD;
    e->gain = (float)g[0] * (1.0f / 64.0f);
    e->lfo_rate = 0.05f * exp2_((float)g[1] * ONE_255 * 9.0f);
    e->lfo_shape = g[2];
    e->env_attack = seconds(g[3]);
    e->env_decay = seconds(g[4]);
    e->env_sustain = (float)g[5] * ONE_255;
    const uint8_t *b = g + OSCYL_GLOBALS + OSCYL_NAMES;
    for (uint32_t i = 0; i < nb; i++, b += 8) {
      e->blocks[i].type = b[0];
      for (int j = 0; j < 7; j++) e->blocks[i].base[j] = (float)b[1 + j] * ONE_255;
      source |= is_source(b[0]);
    }
    for (uint32_t i = 0; i < nm; i++, b += 4) {
      e->mods[i].src = b[0];
      e->mods[i].block = b[1];
      e->mods[i].param = b[2];
      e->mods[i].amount = (float)(int8_t)b[3] * (1.0f / 64.0f);
    }
    e->tables = b;
    e->n_blocks = (uint8_t)nb;
    e->n_mods = (uint8_t)nm;
    e->n_tables = (uint8_t)nt;
  }
  if (!source) {
    /* Nothing that checks out, or nothing that sounds: a plain saw. */
    for (int i = 0; i < OSCYL_MAX_BLOCKS; i++) e->blocks[i].type = 0;
    e->blocks[0].type = OSCYL_SAW;
    e->blocks[0].base[0] = 1.0f;
    e->n_blocks = 1;
    e->n_mods = 0;
    e->n_tables = 0;
    e->gain = 0.5f;
    e->lfo_rate = 1.0f;
    e->env_attack = 0.001f;
    e->env_decay = 0.001f;
    e->env_sustain = 1.0f;
  }
  reset(e);
}

void oscyl_engine_note_on(oscyl_engine_t *e) { reset(e); }

void oscyl_engine_param(oscyl_engine_t *e, uint16_t index, uint16_t value) {
  if (index < 6)
    e->params[index] = clamp01((float)value * 0.01f);
  else if (index == 6)
    e->shape = clamp01((float)value * (1.0f / 1023.0f));
  else if (index == 7)
    e->shift_shape = clamp01((float)value * (1.0f / 1023.0f));
}

static float table_at(const uint8_t *t, float phase) {
  float idx = phase * (float)OSCYL_TABLE_LEN;
  int32_t j = floor_i(idx);
  float f = idx - (float)j;
  const uint8_t *a = t + 2 * (j & 255), *b = t + 2 * ((j + 1) & 255);
  float s0 = (float)(int16_t)(a[0] | a[1] << 8), s1 = (float)(int16_t)(b[0] | b[1] << 8);
  return (s0 + (s1 - s0) * f) * (1.0f / 32768.0f);
}

/* Everything that holds for one render call: modulation and coefficients. */
static void resolve(oscyl_engine_t *e, float note, float w0, float shape_lfo, uint32_t frames) {
  float dt = (float)frames * (1.0f / OSCYL_RATE);
  e->lfo_phase += e->lfo_rate * dt;
  if (e->lfo_phase >= 1.0f) {
    e->lfo_phase = frac(e->lfo_phase);
    if (e->lfo_shape >= 2) e->lfo_value = noise_f(&e->noise);
  }
  if (e->lfo_shape == 0)
    e->lfo_value = sin2pi(e->lfo_phase);
  else if (e->lfo_shape == 1)
    e->lfo_value = 4.0f * absf(e->lfo_phase - 0.5f) - 1.0f;
  if (e->env_stage == 0) {
    e->env += dt / e->env_attack;
    if (e->env >= 1.0f) {
      e->env = 1.0f;
      e->env_stage = 1;
    }
  } else {
    float c = 1.0f - dt / e->env_decay;
    if (c < 0.0f) c = 0.0f;
    e->env = e->env_sustain + (e->env - e->env_sustain) * c;
  }

  float src[OSCYL_NUM_SRC];
  src[OSCYL_SRC_SHAPE] = clamp01(e->shape + shape_lfo);
  src[OSCYL_SRC_SHIFT_SHAPE] = e->shift_shape;
  src[OSCYL_SRC_LFO] = e->lfo_value;
  src[OSCYL_SRC_ENV] = e->env;
  for (int i = 0; i < 6; i++) src[OSCYL_SRC_PARAM1 + i] = e->params[i];
  src[OSCYL_SRC_KEY] = (note - 60.0f) * (1.0f / 64.0f);

  for (int b = 0; b < e->n_blocks; b++)
    for (int j = 0; j < 7; j++) e->blocks[b].v[j] = e->blocks[b].base[j];
  for (int m = 0; m < e->n_mods; m++) {
    const oscyl_mod_t *d = &e->mods[m];
    if (d->src < OSCYL_NUM_SRC && d->block < e->n_blocks && d->param < 7)
      e->blocks[d->block].v[d->param] += d->amount * src[d->src];
  }
  float f0 = w0 * OSCYL_RATE;
  for (int i = 0; i < e->n_blocks; i++) {
    oscyl_block_t *b = &e->blocks[i];
    float *v = b->v, *k = b->k;
    for (int j = 0; j < 7; j++) v[j] = clamp01(v[j]);
    switch (b->type) {
    case OSCYL_WAVETABLE: {
      float pos = v[0] * (float)(e->n_tables > 0 ? e->n_tables - 1 : 0);
      int32_t i0 = floor_i(pos);
      if (e->n_tables > 1 && i0 >= e->n_tables - 1) i0 = e->n_tables - 2;
      if (i0 < 0) i0 = 0;
      k[0] = (float)i0;
      k[1] = e->n_tables > 1 ? pos - (float)i0 : 0.0f;
      break;
    }
    case OSCYL_FM:
      k[0] = (float)(1 + floor_i(v[0] * 7.99f));
      k[1] = v[1] * v[1] * 0.8f;
      k[2] = v[2] * 0.2f;
      break;
    case OSCYL_SUPERSAW:
      k[0] = v[0] * 0.04f;
      break;
    case OSCYL_NOISE:
      k[0] = (float)(1 + floor_i(v[1] * v[1] * 63.0f));
      break;
    case OSCYL_PHASE_DIST: {
      float d = 0.5f - 0.48f * v[0];
      k[0] = d;
      k[1] = 0.5f / d;
      k[2] = 0.5f / (1.0f - d);
      break;
    }
    case OSCYL_SYNC:
      k[0] = 1.0f + 7.0f * v[0];
      break;
    case OSCYL_FOLD:
      k[0] = 0.25f * (1.0f + 7.0f * v[0]);
      k[1] = 0.125f * v[1];
      break;
    case OSCYL_CRUSH:
      k[0] = exp2_(2.0f + (1.0f - v[0]) * 10.0f);
      k[1] = (float)(1 + floor_i(v[1] * v[1] * 31.0f));
      break;
    case OSCYL_LOWPASS: {
      float fc = f0 * exp2_(v[0] * 7.0f);
      if (fc > 20000.0f) fc = 20000.0f;
      float w = 6.2831853f * fc * (1.0f / OSCYL_RATE);
      k[0] = w / (1.0f + w);
      break;
    }
    case OSCYL_FORMANT: {
      float pos = v[0] * 4.0f;
      int32_t a = floor_i(pos);
      if (a > 3) a = 3;
      float f = pos - (float)a, shift = exp2_(v[3] - 0.5f);
      for (int n = 0; n < 2; n++) {
        float hz = (VOWELS[a][n] + (VOWELS[a + 1][n] - VOWELS[a][n]) * f) * shift;
        k[n] = 2.0f * sin2pi(0.5f * hz * (1.0f / OSCYL_RATE));
      }
      k[2] = 1.0f - 0.9f * v[1];
      break;
    }
    default:
      break;
    }
  }
}

void oscyl_engine_render(oscyl_engine_t *e, uint16_t pitch, int32_t shape_lfo, float *out,
                         uint32_t frames) {
  float note = (float)(pitch >> 8) + (float)(pitch & 0xFF) * (1.0f / 256.0f);
  float w0 = 440.0f * exp2_((note - 69.0f) * (1.0f / 12.0f)) * (1.0f / OSCYL_RATE);
  if (w0 > 0.45f) w0 = 0.45f;
  resolve(e, note, w0, (float)shape_lfo * (1.0f / 2147483648.0f), frames);

  for (uint32_t i = 0; i < frames; i++) {
    float ph = e->phase + w0;
    if (ph >= 1.0f) {
      ph -= 1.0f;
      e->sub_phase = e->sub_phase >= 0.5f ? 0.0f : 0.5f;
    }
    e->phase = ph;
    float rp = ph, dt = w0, x = 0.0f;
    for (int n = 0; n < e->n_blocks; n++) {
      oscyl_block_t *b = &e->blocks[n];
      const float *v = b->v, *k = b->k;
      float *s = b->s;
      switch (b->type) {
      case OSCYL_SAW:
        x += v[0] * saw(rp, dt);
        break;
      case OSCYL_WAVETABLE: {
        float y;
        if (e->n_tables == 0) {
          y = sin2pi(rp);
        } else {
          const uint8_t *t = e->tables + (uint32_t)k[0] * 2 * OSCYL_TABLE_LEN;
          y = table_at(t, rp);
          if (k[1] > 0.0f) y += (table_at(t + 2 * OSCYL_TABLE_LEN, rp) - y) * k[1];
        }
        x += v[1] * y;
        break;
      }
      case OSCYL_FM: {
        float m = sin2pi(rp * k[0] + k[2] * s[0]);
        s[0] = m;
        x += v[3] * sin2pi(rp + k[1] * m);
        break;
      }
      case OSCYL_SUPERSAW: {
        float y = 0.0f;
        for (int j = 0; j < 5; j++) {
          float d = w0 * (1.0f + k[0] * SPREAD[j]);
          float p = s[j] + d;
          if (p >= 1.0f) p -= 1.0f;
          s[j] = p;
          y += saw(p, d);
        }
        x += v[1] * y * 0.36f;
        break;
      }
      case OSCYL_SUB: {
        float sp = e->sub_phase + 0.5f * ph;
        float sq = sp < 0.5f ? 1.0f : -1.0f;
        x += v[0] * (sin2pi(sp) + (sq - sin2pi(sp)) * v[1]);
        break;
      }
      case OSCYL_NOISE:
        if ((float)b->n >= k[0] - 0.5f || b->n == 0) {
          s[0] = noise_f(&e->noise);
          b->n = 0;
        }
        b->n++;
        x += v[0] * s[0];
        break;
      case OSCYL_PHASE_DIST:
        rp = rp < k[0] ? rp * k[1] : 0.5f + (rp - k[0]) * k[2];
        break;
      case OSCYL_SYNC:
        rp = frac(rp * k[0]);
        dt *= k[0];
        break;
      case OSCYL_FOLD: {
        float y = sin2pi(k[0] * x + k[1]);
        x += (y - x) * v[2];
        break;
      }
      case OSCYL_CRUSH: {
        if ((float)b->n >= k[1] - 0.5f || b->n == 0) {
          float q = x * k[0];
          s[0] = (float)(int32_t)(q + (q >= 0.0f ? 0.5f : -0.5f)) / k[0];
          b->n = 0;
        }
        b->n++;
        x += (s[0] - x) * v[2];
        break;
      }
      case OSCYL_LOWPASS:
        s[0] += k[0] * (x - s[0]);
        x = s[0];
        break;
      case OSCYL_FORMANT: {
        float y = 0.0f;
        for (int j = 0; j < 2; j++) {
          float *lp = &s[2 * j], *bp = &s[2 * j + 1];
          *lp += k[j] * *bp;
          float hp = x - *lp - k[2] * *bp;
          *bp += k[j] * hp;
          y += *bp * (j == 0 ? 1.0f : 0.7f);
        }
        x += (y * k[2] - x) * v[2];
        break;
      }
      default:
        break;
      }
    }
    /* Gain, a DC blocker, then a soft knee above 0.75. */
    float y = x * e->gain;
    float hp = y - e->dc_x + 0.995f * e->dc_y;
    e->dc_x = y;
    e->dc_y = hp;
    float a = absf(hp);
    if (a > 0.75f) {
      float z = (a - 0.75f) * 4.0f;
      float t = z >= 3.0f ? 1.0f : z * (27.0f + z * z) / (27.0f + 9.0f * z * z);
      a = 0.75f + 0.25f * t;
      hp = hp < 0.0f ? -a : a;
    }
    out[i] = hp;
  }
}
