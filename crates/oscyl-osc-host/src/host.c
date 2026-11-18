/* One note of a patch, as the synth would play it. */
#include "dsp.h"

void oscyl_host_render(const uint8_t *patch, uint32_t size, uint16_t pitch, const uint16_t knobs[8],
                       float *out, uint32_t frames) {
  oscyl_dsp_t e;
  oscyl_dsp_load(&e, patch, size);
  for (uint16_t i = 0; i < 8; i++) oscyl_dsp_param(&e, i, knobs[i]);
  oscyl_dsp_note_on(&e);
  for (uint32_t done = 0; done < frames; done += OSCYL_CHUNK) {
    uint32_t n = frames - done < OSCYL_CHUNK ? frames - done : OSCYL_CHUNK;
    oscyl_dsp_render(&e, pitch, 0, out + done, n);
  }
}
