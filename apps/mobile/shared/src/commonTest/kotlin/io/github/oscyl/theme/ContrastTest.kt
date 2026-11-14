package io.github.oscyl.theme

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.luminance
import kotlin.test.Test
import kotlin.test.assertTrue

/** Text on its ground reaches WCAG AA (4.5:1) in every theme, plate gradient included. */
class ContrastTest {
    private fun ratio(a: Color, b: Color): Float {
        val (hi, lo) = listOf(a.luminance(), b.luminance()).sortedDescending()
        return (hi + 0.05f) / (lo + 0.05f)
    }

    @Test
    fun textPairsReachAa() {
        for ((name, c) in listOf("light" to OscylTokens.Light, "dark" to OscylTokens.Dark, "amoled" to OscylTokens.Amoled)) {
            // The lifted plate's sweep (Plate.kt): 6% lighter at the top, 5% darker at the bottom.
            val plateTop = lerp(c.plate, Color.White, 0.06f)
            val plateBottom = lerp(c.plate, Color.Black, 0.05f)
            val pairs = listOf(
                "ink on ground" to (c.ink to c.ground),
                "ink on raised" to (c.ink to c.raised),
                "legend on ground" to (c.legend to c.ground),
                "legend on raised" to (c.legend to c.raised),
                "act on ground" to (c.act to c.ground),
                "act on raised" to (c.act to c.raised),
                "on-act on act" to (c.onAct to c.act),
                "on-plate on plate top" to (c.onPlate to plateTop),
                "on-plate on plate bottom" to (c.onPlate to plateBottom),
                "plate-legend on plate top" to (c.plateLegend to plateTop),
                "plate-legend on plate bottom" to (c.plateLegend to plateBottom),
            )
            for ((pair, colors) in pairs) {
                val r = ratio(colors.first, colors.second)
                assertTrue(r >= 4.5f, "$name: $pair is $r:1")
            }
        }
    }
}
