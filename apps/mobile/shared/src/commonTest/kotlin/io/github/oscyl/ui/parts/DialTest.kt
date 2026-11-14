package io.github.oscyl.ui.parts

import androidx.compose.ui.unit.dp
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class DialTest {
    private val fast = 1f
    private val slow = 0.05f

    @Test
    fun aFastDragOf220dpRunsTheRange() {
        val range = 0..1023
        assertEquals(1023f, knobDrag(0f, 220.dp, fast, range))
        assertEquals(0f, knobDrag(1023f, (-220).dp, fast, range))
        assertEquals(511.5f, knobDrag(0f, 110.dp, fast, range))
        // It clamps at both ends.
        assertEquals(1023f, knobDrag(900f, 400.dp, fast, range))
        assertEquals(0f, knobDrag(10f, (-50).dp, fast, range))
    }

    @Test
    fun aSlowDragTurnsAtTheFineGain() {
        val range = 0..1000
        // The same 220 dp, slowly: about 0.35 of the range.
        assertEquals(350f, knobDrag(0f, 220.dp, slow, range), 0.5f)
        // Just under and just over the threshold.
        assertEquals(35f, knobDrag(0f, 22.dp, 0.149f, range), 0.5f)
        assertEquals(100f, knobDrag(0f, 22.dp, 0.151f, range), 0.5f)
    }

    @Test
    fun feelsAreTicksDetentsAndBumps() {
        val range = 0..100
        val saved = 52
        val feels = (1..100).map { knobFeel(it - 1, it, saved, range) }
        // A bump at the end, detents at the centre (50) and saved (52), ticks at the other 18 inner printed steps.
        assertEquals(Feel.Bump, feels.last())
        assertEquals(2, feels.count { it == Feel.Detent })
        assertEquals(Feel.Detent, knobFeel(49, 50, saved, range))
        assertEquals(Feel.Detent, knobFeel(51, 52, saved, range))
        assertEquals(18, feels.count { it == Feel.Tick })
        // A flick across the scale in one move: one feel, the firmest (the end).
        assertEquals(Feel.Bump, knobFeel(0, 100, saved, range))
        assertEquals(Feel.Bump, knobFeel(1, 0, saved, range))
        assertEquals(Feel.None, knobFeel(3, 4, saved, range))
        assertTrue(Feel.Bump > Feel.Detent && Feel.Detent > Feel.Tick)
    }

    @Test
    fun theSpringSettlesWithoutOvershoot() {
        var x = 0f
        var v = 0f
        var peak = 0f
        var settled = -1
        for (frame in 1..60) {
            val (nx, nv) = springStep(x, v, 1f, 1f / 120, 1100f)
            x = nx
            v = nv
            peak = maxOf(peak, x)
            if (settled < 0 && abs(1f - x) < 0.01f) settled = frame
        }
        assertTrue(peak <= 1.0001f, "no overshoot, peak $peak")
        // Critically damped at ω ≈ 33/s: within 1% in about 200 ms, never at once.
        assertTrue(settled in 6..30, "settled at frame $settled")
        // A dropped frame (50 ms) stays stable.
        val (jx, _) = springStep(0f, 0f, 1f, 0.05f, 1100f)
        assertTrue(jx in 0f..1f)
    }
}
