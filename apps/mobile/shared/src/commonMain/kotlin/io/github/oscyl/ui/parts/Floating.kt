package io.github.oscyl.ui.parts

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.drawOutline
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import io.github.oscyl.theme.OscylColors
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes

/** Outer corners of the floating parts (island, rail, sheets); the plates inside them use `radiusPanel`. */
val FloatingRadius = OscylTokens.Floating.radius

internal val FloatingShape = RoundedCornerShape(FloatingRadius)

/** The one ink of a control that cannot act now (DESIGN.md "Off controls": `legend` at 60%). */
val OscylColors.disabled: androidx.compose.ui.graphics.Color get() = legend.copy(alpha = 0.6f)

/**
 * A floating part (DESIGN.md "Floating"): raised enamel lifted by two shadow layers,
 * a close 3 dp one and a soft 28 dp one, with a 1 dp highlight along its inner top
 * edge and the 1 dp `floating-rim` around it (tokens.json, shared with the desktop's
 * notices: the rule on light, near `rule-strong` on dark, `rule-strong` on AMOLED, which
 * casts no shadow, so there the rim is the lift).
 *
 * [lift] (0..1) fades the whole lift in, read in the draw phase only: the top bar
 * rises from the page this way as its title scrolls into it, with no recomposition.
 */
fun Modifier.floating(colors: OscylColors, shape: Shape = FloatingShape, lift: (() -> Float)? = null, edgeToEdge: Boolean = false): Modifier {
    val f = OscylTokens.Floating
    val rim = colors.floatingRim
    val shadows = this
        .softShadow(shape, f.farRadius, DpOffset(0.dp, f.farOffset), colors.shadow.copy(alpha = colors.shadow.alpha * f.farAlpha)) { lift?.invoke() ?: 1f }
        .softShadow(shape, f.nearRadius, DpOffset(0.dp, f.nearOffset), colors.shadow.copy(alpha = colors.shadow.alpha * f.nearAlpha)) { lift?.invoke() ?: 1f }
    return shadows
        .drawWithCache {
            // Outlines made once per size: the lift's frames only draw.
            val fill = shape.createOutline(size, layoutDirection, this)
            val w = OscylTokens.ruleWidth.toPx()
            val inner = shape.createOutline(Size(size.width - w, size.height - w), layoutDirection, this)
            val stroke = Stroke(w)
            onDrawBehind {
                val a = lift?.invoke() ?: 1f
                if (a <= 0f) return@onDrawBehind
                drawOutline(fill, colors.raised, alpha = a)
                // Edge to edge (a sheet), only its head is an edge: the sides and foot run off the
                // screen and a rim there drew a frame along the display's own edges and corners.
                if (edgeToEdge) {
                    clipRect(bottom = FloatingRadius.toPx() + w) { translate(w / 2, w / 2) { drawOutline(inner, rim, alpha = a, style = stroke) } }
                } else {
                    translate(w / 2, w / 2) { drawOutline(inner, rim, alpha = a, style = stroke) }
                }
            }
        }
}

@OscylPreviews
@Composable
private fun floating(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    val c = OscylTheme.colors
    Row(horizontalArrangement = Arrangement.spacedBy(24.dp)) {
        Box(Modifier.size(width = 120.dp, height = 64.dp).floating(c))
        Box(Modifier.size(width = 120.dp, height = 64.dp).floating(c, lift = { 0.5f }))
    }
}
