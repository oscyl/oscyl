package io.github.oscyl.ui.parts

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes

/**
 * Progress as a tuning scale (DESIGN.md "Reading progress"): printed ticks in `rule-strong`
 * along a 6 dp strip, an ink line run out to [fraction] (0..1) and an index standing
 * at its end. The index glides to each new value on `base`; with no [fraction] the
 * strip is empty and keeps its place, so nothing moves when a read starts or ends.
 * Silent: the words beside it say how far.
 */
@Composable
fun ProgressScale(fraction: Float?, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    val motion = LocalOscylMotion.current
    val at by animateFloatAsState((fraction ?: 0f).coerceIn(0f, 1f), tween(if (motion) OscylTokens.baseMs else 0, easing = OscylTokens.easing))
    Box(
        modifier.fillMaxWidth().height(6.dp).drawBehind {
            if (fraction == null) return@drawBehind
            val rtl = layoutDirection == LayoutDirection.Rtl
            val w = 1.dp.toPx()
            val base = size.height - w
            fun x(f: Float) = if (rtl) size.width * (1f - f) else size.width * f
            // The printed scale: a tick every 6 dp or so, every fifth one taller.
            val ticks = (size.width / 6.dp.toPx()).toInt().coerceAtLeast(1)
            for (i in 0..ticks) {
                val tx = (size.width - w) * i / ticks + w / 2
                drawLine(c.ruleStrong, Offset(tx, if (i % 5 == 0) 0f else size.height / 2), Offset(tx, size.height), w)
            }
            drawLine(c.ink, Offset(x(0f), base), Offset(x(at), base), 2 * w, StrokeCap.Round)
            drawLine(c.ink, Offset(x(at), 0f), Offset(x(at), size.height), 2 * w, StrokeCap.Round)
        },
    )
}

@OscylPreviews
@Composable
private fun progressScale(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    Column(Modifier.width(240.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        ProgressScale(0f)
        ProgressScale(0.42f)
        ProgressScale(1f)
        ProgressScale(null)
    }
}
