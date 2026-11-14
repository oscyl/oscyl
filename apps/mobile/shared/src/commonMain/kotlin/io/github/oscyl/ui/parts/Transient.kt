package io.github.oscyl.ui.parts

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.oscyl.theme.OscylTokens
import kotlin.math.roundToInt

/**
 * How far a transient part (a notice, a popup, a foot key, the search filters) has come
 * out, 0..1, and whether it is [up]: still composed for its way back. The one way such a
 * part arrives and leaves (DESIGN.md "Transient parts"): in over `base`, out over `fast`,
 * on the one easing; at once with motion off.
 */
@Stable
class TransientState internal constructor(visible: Boolean) {
    internal val t = Animatable(if (visible) 1f else 0f)
    val value: Float get() = t.value
    var up by mutableStateOf(visible)
        internal set
}

/** A transient part's state for [visible]: it stays [TransientState.up] until its exit ends. */
@Composable
fun rememberTransient(visible: Boolean): TransientState {
    val s = remember { TransientState(visible) }
    val motion = LocalOscylMotion.current
    // At rest and away (a knob's stepper, closed) it costs no coroutine.
    if (visible || s.up) {
        LaunchedEffect(visible) {
            if (visible) s.up = true
            s.t.go(if (visible) 1f else 0f, if (visible) OscylTokens.baseMs else OscylTokens.fastMs, motionScale(motion))
            if (!visible) s.up = false
        }
    }
    return s
}

/** The part kept laid out (its place never changes), only drawn as it comes and goes. */
@Composable
fun transientAlpha(visible: Boolean): State<Float> {
    val s = rememberTransient(visible)
    return remember(s) { androidx.compose.runtime.derivedStateOf { s.value } }
}

/**
 * The transient look on the part's layer, at [t] (0 gone, 1 there): alpha, a scale from
 * [scaleFrom] about [origin], and a rise from [rise] lower (the parts at the foot).
 * Placement and alpha only: nothing is measured again.
 */
fun Modifier.transientLayer(t: () -> Float, scaleFrom: Float = 0.96f, rise: Dp = 0.dp, origin: TransformOrigin = TransformOrigin.Center): Modifier =
    graphicsLayer {
        val v = t()
        alpha = v
        if (scaleFrom != 1f) {
            transformOrigin = origin
            scaleX = scaleFrom + (1f - scaleFrom) * v
            scaleY = scaleX
        }
        if (rise != 0.dp) translationY = (1f - v) * rise.toPx()
    }

/**
 * [value] while there is one, arriving and leaving as every transient part does; the last
 * one stays on its way out. A new value replacing another waits for the old one to leave
 * over `fast`, then rises over `base`: no frame shows both.
 */
@Composable
fun <T : Any> Transient(
    value: T?,
    modifier: Modifier = Modifier,
    scaleFrom: Float = 0.96f,
    rise: Dp = 0.dp,
    content: @Composable (T) -> Unit,
) {
    val motion = LocalOscylMotion.current
    var held by remember { mutableStateOf(value) }
    val t = remember { Animatable(if (value != null) 1f else 0f) }
    LaunchedEffect(value) {
        val scale = motionScale(motion)
        if (held != null && held != value) t.go(0f, OscylTokens.fastMs, scale)
        held = value
        if (value != null) t.go(1f, OscylTokens.baseMs, scale)
    }
    val shown = held ?: return
    // Keyed: a new value starts afresh (a swiped notice's offset is not the next one's).
    Box(modifier.transientLayer({ t.value }, scaleFrom, rise)) { key(shown) { content(shown) } }
}

private suspend fun Animatable<Float, *>.go(target: Float, ms: Int, scale: Float) {
    val d = (ms * scale).roundToInt()
    if (d <= 0) snapTo(target) else animateTo(target, tween(d, easing = OscylTokens.easing))
}
