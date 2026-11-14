package io.github.oscyl.ui.parts

import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.VisibilityThreshold
import androidx.compose.ui.geometry.Rect
import kotlin.test.Test
import kotlin.test.assertTrue

class PlateSpringTest {
    /** The time the plate takes to land on a neighbouring tab, in ms. */
    private fun settleMs(distancePx: Float): Long {
        val converter = Rect.VectorConverter
        val from = Rect(0f, 0f, 160f, 156f)
        val spec = plateSpring(Rect.VisibilityThreshold).vectorize(converter)
        val start = converter.convertToVector(from)
        val end = converter.convertToVector(from.translate(distancePx, 0f))
        val still = converter.convertToVector(Rect.Zero)
        return spec.getDurationNanos(start, end, still) / 1_000_000
    }

    @Test
    fun plateLandsLikeTheDesktopPlate() {
        // One island tab on a 3x phone is ~250 px; the side island moves ~150 px.
        for (distance in listOf(150f, 250f, 600f)) {
            val ms = settleMs(distance)
            assertTrue(ms in 250..420, "plate settles a ${distance.toInt()} px move in $ms ms")
        }
    }
}
