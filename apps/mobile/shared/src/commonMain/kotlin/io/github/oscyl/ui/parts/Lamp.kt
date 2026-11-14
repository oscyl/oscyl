package io.github.oscyl.ui.parts

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes

/**
 * The 7 dp function lamp: lit, it fills `on` (connected, synced, live) over `base`;
 * off, a 1 dp `legend` ring. Always paired with a word, which carries the meaning.
 */
@Composable
fun Lamp(lit: Boolean, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    // Fill and ring change together (at once with the app's animations off).
    val spec = tween<Color>(if (LocalOscylMotion.current) OscylTokens.baseMs else 0, easing = OscylTokens.easing)
    val fill by animateColorAsState(if (lit) c.on else c.on.copy(alpha = 0f), spec)
    val ring by animateColorAsState(if (lit) c.on else c.legend, spec)
    Box(modifier.size(7.dp).background(fill, CircleShape).border(1.dp, ring, CircleShape))
}

@OscylA11yPreviews
@Composable
private fun lamp_a11y() = lamp(OscylThemeMode.Dark)

@OscylPreviews
@Composable
private fun lamp(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Lamp(lit = false)
        Lamp(lit = true)
    }
}
