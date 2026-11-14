package io.github.oscyl.ui.parts

import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.runtime.remember
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.snap
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.SpringSpec
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.interaction.InteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.MotionDurationScale
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import io.github.oscyl.theme.OscylColors
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import kotlinx.coroutines.currentCoroutineContext

/**
 * The app's one authored motion: a selection plate sliding on a critically damped
 * spring that carries its speed into a new target. Same parameters as the Linux
 * plate (`apps/linux/src/widgets/plate.rs`: damping ratio 1, mass 1, stiffness 600).
 * Give the animated value a visibility threshold (half a pixel) or the spring
 * keeps running long after the plate has visibly landed.
 */
fun <T> plateSpring(visibilityThreshold: T? = null): SpringSpec<T> =
    spring(dampingRatio = Spring.DampingRatioNoBouncy, stiffness = PlateStiffness, visibilityThreshold = visibilityThreshold)

/** The desktop plate's spring stiffness. */
const val PlateStiffness = 600f

/**
 * The app's own motion switch (Preferences, still to come): false stills every Oscyl
 * animation, on top of the system's animator duration scale. Provide it around the app.
 */
val LocalOscylMotion = staticCompositionLocalOf { true }

/**
 * How fast motion runs in this coroutine: the system animator duration scale (Compose's
 * [MotionDurationScale]), or 0 when the app's switch ([motion]) is off. At 0, snap.
 */
suspend fun motionScale(motion: Boolean): Float =
    if (motion) currentCoroutineContext()[MotionDurationScale]?.scaleFactor ?: 1f else 0f

/**
 * A lifted enamel plate: [fill] swept from 6% lighter at the top to 5% darker at the
 * bottom, on a soft 1 dp shadow (DESIGN.md, "Lifted plate").
 */
fun Modifier.liftedPlate(colors: OscylColors, shape: Shape, fill: Color = colors.plate): Modifier =
    softShadow(shape, 3.dp, DpOffset(0.dp, 1.dp), colors.shadow)
        .background(Brush.verticalGradient(listOf(lerp(fill, Color.White, 0.06f), lerp(fill, Color.Black, 0.05f))), shape)

/**
 * Touch and keyboard states without a ripple: [press] is there the moment the finger
 * lands (a held part answers at once, and the quickest tap still shows) and lets go
 * over `base`; keyboard focus draws the 2 dp ink ring. With the app's motion off it
 * lets go at once too.
 */
@Composable
fun Modifier.touchStates(interactionSource: InteractionSource, shape: Shape, press: Color): Modifier {
    val pressed by interactionSource.collectIsPressedAsState()
    val focused by interactionSource.collectIsFocusedAsState()
    val overlay by animateColorAsState(
        if (pressed) press else press.copy(alpha = 0f),
        if (pressed || !LocalOscylMotion.current) snap() else tween(OscylTokens.baseMs, easing = OscylTokens.easing),
    )
    val ring = if (focused) Modifier.border(2.dp, OscylTheme.colors.focus, shape) else Modifier
    return background(overlay, shape).then(ring)
}

@OscylA11yPreviews
@Composable
private fun plates_a11y() = plates(OscylThemeMode.Dark)

@OscylPreviews
@Composable
private fun plates(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    val c = OscylTheme.colors
    Row {
        Box(Modifier.size(width = 120.dp, height = 64.dp).floating(c))
        Spacer(Modifier.width(16.dp))
        Box(Modifier.size(width = 80.dp, height = 44.dp).liftedPlate(c, RoundedCornerShape(OscylTokens.radiusPanel)))
    }
}
