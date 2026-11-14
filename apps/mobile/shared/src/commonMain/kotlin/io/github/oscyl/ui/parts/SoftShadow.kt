package io.github.oscyl.ui.parts

import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpOffset
import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.roundToInt

/**
 * A soft shadow under [shape] (a rectangle, a rounded rectangle or a circle), blurred
 * by [radius] and moved by [offset], drawn as gradients: four corners, four edges and
 * the core. The GPU shades it, at any size, so a part that grows or slides (the rail,
 * a sheet, a plate) casts it for nothing per frame. A blurred mask (`dropShadow`) is
 * blurred on the UI thread again for every new size: tens of ms on an older phone.
 *
 * [alpha] is read in the draw phase only.
 */
fun Modifier.softShadow(shape: Shape, radius: Dp, offset: DpOffset, color: Color, alpha: () -> Float = { 1f }): Modifier =
    if (color.alpha == 0f) this else drawWithCache {
        val corner = when (val outline = shape.createOutline(size, layoutDirection, this)) {
            is Outline.Rounded -> outline.roundRect.let { maxOf(it.topLeftCornerRadius.x, it.topRightCornerRadius.x, it.bottomLeftCornerRadius.x, it.bottomRightCornerRadius.x) }
            else -> 0f
        }.coerceAtMost(size.minDimension / 2)
        val shadow = SoftShadow(color, radius.toPx(), corner)
        val by = Offset(offset.x.toPx().roundToInt().toFloat(), offset.y.toPx().roundToInt().toFloat())
        onDrawBehind { softShadow(shadow, size, by, alpha()) }
    }

/** The brushes of one shadow: a blur of [blur] px round corners of [corner] px. */
internal class SoftShadow(color: Color, blur: Float, val corner: Float) {
    // A blur mask's standard deviation for its radius, as the platforms' mask filters have it.
    private val sigma = 0.57735f * blur + 0.5f

    /** From the corner's centre (or the edge's inner line) to where the shadow is gone. */
    val span = corner + 2.5f * sigma

    private val stops = Array(Stops + 1) { i ->
        val t = i.toFloat() / Stops
        t to color.copy(alpha = color.alpha * coverage((t * span - corner) / sigma))
    }
    val core = stops[0].second
    val round = Brush.radialGradient(*stops, center = Offset.Zero, radius = span)
    val up = Brush.linearGradient(*stops, start = Offset.Zero, end = Offset(0f, -span))
    val down = Brush.linearGradient(*stops, start = Offset.Zero, end = Offset(0f, span))
    val start = Brush.linearGradient(*stops, start = Offset.Zero, end = Offset(-span, 0f))
    val end = Brush.linearGradient(*stops, start = Offset.Zero, end = Offset(span, 0f))

    private companion object {
        const val Stops = 16
    }
}

/** How much of a blurred edge covers a point [x] standard deviations outside it (inside: negative). */
internal fun coverage(x: Float): Float {
    // erfc by Abramowitz and Stegun 7.1.26 (error under 2e-7).
    val z = abs(x) / 1.41421356f
    val t = 1f / (1f + 0.3275911f * z)
    val erfc = t * (0.254829592f + t * (-0.284496736f + t * (1.421413741f + t * (-1.453152027f + t * 1.061405429f)))) * exp(-z * z)
    return if (x >= 0f) 0.5f * erfc else 1f - 0.5f * erfc
}

/** Draws [shadow] under a part of [part] size, moved [by], at [alpha]. */
internal fun DrawScope.softShadow(shadow: SoftShadow, part: Size, by: Offset, alpha: Float) {
    if (alpha <= 0f) return
    val c = shadow.corner
    val q = shadow.span
    val w = part.width
    val h = part.height
    val square = Size(q, q)
    translate(by.x, by.y) {
        translate(c, c) { drawRect(shadow.round, Offset(-q, -q), square, alpha) }
        translate(w - c, c) { drawRect(shadow.round, Offset(0f, -q), square, alpha) }
        translate(c, h - c) { drawRect(shadow.round, Offset(-q, 0f), square, alpha) }
        translate(w - c, h - c) { drawRect(shadow.round, Offset.Zero, square, alpha) }
        if (w > 2 * c) {
            translate(c, c) { drawRect(shadow.up, Offset(0f, -q), Size(w - 2 * c, q), alpha) }
            translate(c, h - c) { drawRect(shadow.down, Offset.Zero, Size(w - 2 * c, q), alpha) }
        }
        if (h > 2 * c) {
            translate(c, c) { drawRect(shadow.start, Offset(-q, 0f), Size(q, h - 2 * c), alpha) }
            translate(w - c, c) { drawRect(shadow.end, Offset.Zero, Size(q, h - 2 * c), alpha) }
        }
        if (w > 2 * c && h > 2 * c) drawRect(shadow.core, Offset(c, c), Size(w - 2 * c, h - 2 * c), alpha)
    }
}
