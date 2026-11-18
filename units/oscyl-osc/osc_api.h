/*
 * Stands in for the logue SDK's osc_api.h, which userosc.h includes: the
 * engine calls no firmware routine (it keeps free of them so the same code
 * runs on the computer), and the real header pulls in CMSIS.
 */
#ifndef __osc_api_h
#define __osc_api_h
/* The real header brings these in (through float_math.h); _unit.c uses them. */
#include <stddef.h>
#endif
