package io.github.oscyl.ui.parts

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.AbsoluteAlignment
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import io.github.oscyl.platform.LocalOscylHaptics
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.pad_description
import io.github.oscyl.shared.generated.resources.pad_less
import io.github.oscyl.shared.generated.resources.pad_more
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import org.jetbrains.compose.resources.stringResource
import kotlin.math.floor
import kotlin.math.roundToInt

/**
 * An XY pad on two parameters (DESIGN.md "Play page"): a ruled frame with a grid printed
 * at tenths (every fifth `rule-strong`), the axes' legends at the foot and the start
 * edge, the point as a graph handle's ring in a 64 dp target and, while edited, an
 * orange line from the saved point (a dotted ghost ring) to it. A touch anywhere takes
 * the point there (it glides over `fast`, then follows the finger); while held, its
 * values ride on a plate beside it with dashed guides to the scales. A tick per tenth
 * crossed, a bump at an edge. The first finger owns it; [onRelease] once when it lifts.
 * Keyboard: arrows step 1%, page keys 10%. Screen readers: More and Less for each axis,
 * the state "{xLabel} {xText}, {yLabel} {yText}". [x] and [y] run 0..1, y upward;
 * right to left, x grows toward the start.
 */
@Composable
fun XyPad(
    x: Float,
    y: Float,
    savedX: Float?,
    savedY: Float?,
    xLabel: String,
    yLabel: String,
    xText: String,
    yText: String,
    onMove: (x: Float, y: Float) -> Unit,
    onRelease: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    /** A double tap: both fields back to saved (one step), as a knob's double tap. */
    onRestore: (() -> Unit)? = null,
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val haptics = LocalOscylHaptics.current
    val motion = LocalOscylMotion.current
    val rtl = LocalLayoutDirection.current == LayoutDirection.Rtl
    val scope = rememberCoroutineScope()
    val now by rememberUpdatedState(x to y)
    val move by rememberUpdatedState(onMove)
    val release by rememberUpdatedState(onRelease)
    val restore by rememberUpdatedState(onRestore)
    val held = remember { Animatable(0f) }
    val moreX = stringResource(Res.string.pad_more, xLabel)
    val lessX = stringResource(Res.string.pad_less, xLabel)
    val moreY = stringResource(Res.string.pad_more, yLabel)
    val lessY = stringResource(Res.string.pad_less, yLabel)
    val description = stringResource(Res.string.pad_description, xLabel, yLabel)
    fun step(dx: Float, dy: Float) {
        move((now.first + dx).coerceIn(0f, 1f), (now.second + dy).coerceIn(0f, 1f))
        release()
    }
    BoxWithConstraints(
        modifier
            .fillMaxWidth()
            .aspectRatio(1f)
            .semantics {
                contentDescription = description
                stateDescription = "$xLabel $xText, $yLabel $yText"
                if (!enabled) {
                    disabled()
                } else {
                    customActions = listOf(
                        CustomAccessibilityAction(moreX) { step(0.1f, 0f); true },
                        CustomAccessibilityAction(lessX) { step(-0.1f, 0f); true },
                        CustomAccessibilityAction(moreY) { step(0f, 0.1f); true },
                        CustomAccessibilityAction(lessY) { step(0f, -0.1f); true },
                    )
                }
            }
            .onKeyEvent { e ->
                if (!enabled || e.type != KeyEventType.KeyDown) return@onKeyEvent false
                val sx = if (rtl) -1f else 1f
                when (e.key) {
                    Key.DirectionRight -> step(0.01f * sx, 0f)
                    Key.DirectionLeft -> step(-0.01f * sx, 0f)
                    Key.DirectionUp -> step(0f, 0.01f)
                    Key.DirectionDown -> step(0f, -0.01f)
                    Key.PageUp -> step(0f, 0.1f)
                    Key.PageDown -> step(0f, -0.1f)
                    else -> return@onKeyEvent false
                }
                true
            }
            .focusable(enabled)
            .testTag("xy-pad"),
    ) {
        val inset = 16.dp
        Canvas(
            Modifier
                .fillMaxSize()
                .pointerInput(enabled, rtl) {
                    if (!enabled) return@pointerInput
                    val pad = inset.toPx()
                    fun at(p: Offset): Pair<Float, Float> {
                        val w = size.width - 2 * pad
                        val h = size.height - 2 * pad
                        val fx = ((p.x - pad) / w).coerceIn(0f, 1f)
                        return (if (rtl) 1f - fx else fx) to (1f - (p.y - pad) / h).coerceIn(0f, 1f)
                    }
                    var lastTenth = -1 to -1
                    // The last short tap: a second one soon after, near it, restores the saved point.
                    var lastTap = Long.MIN_VALUE / 2
                    var lastTapAt = Offset.Zero
                    fun feel(v: Pair<Float, Float>) {
                        val tenth = floor(v.first * 10).toInt() to floor(v.second * 10).toInt()
                        val edge = v.first == 0f || v.first == 1f || v.second == 0f || v.second == 1f
                        if (tenth != lastTenth) {
                            if (edge) haptics.bump() else haptics.tick()
                            lastTenth = tenth
                        }
                    }
                    awaitEachGesture {
                        val down = awaitFirstDown()
                        down.consume()
                        val again = restore
                        if (again != null && down.uptimeMillis - lastTap < viewConfiguration.doubleTapTimeoutMillis &&
                            (down.position - lastTapAt).getDistance() < viewConfiguration.touchSlop * 3
                        ) {
                            lastTap = Long.MIN_VALUE / 2
                            haptics.confirm()
                            again()
                            do {
                                val e = awaitPointerEvent()
                                e.changes.forEach { it.consume() }
                            } while (e.changes.any { it.id == down.id && it.pressed })
                            return@awaitEachGesture
                        }
                        var travelled = 0f
                        scope.launch { held.snapTo(1f) }
                        var finger = at(down.position)
                        lastTenth = floor(now.first * 10).toInt() to floor(now.second * 10).toInt()
                        // The point glides to the touch over `fast`, toward where the finger is by then.
                        val from = now
                        var glide: Job? = scope.launch {
                            val ms = if (motionScale(motion) == 0f) 0 else (OscylTokens.fastMs * motionScale(motion)).roundToInt()
                            if (ms > 0) {
                                Animatable(0f).animateTo(1f, tween(ms, easing = OscylTokens.easing)) {
                                    val t = value
                                    move(from.first + (finger.first - from.first) * t, from.second + (finger.second - from.second) * t)
                                }
                            }
                            move(finger.first, finger.second)
                        }
                        try {
                            while (true) {
                                val event = awaitPointerEvent()
                                // The first finger owns the pad: any other is ignored.
                                val ch = event.changes.firstOrNull { it.id == down.id } ?: break
                                event.changes.forEach { it.consume() }
                                if (!ch.pressed) break
                                travelled += ch.positionChange().getDistance()
                                finger = at(ch.position)
                                feel(finger)
                                if (glide?.isActive != true) {
                                    glide = null
                                    move(finger.first, finger.second)
                                }
                            }
                        } finally {
                            glide?.let {
                                it.cancel()
                                move(finger.first, finger.second)
                            }
                            scope.launch { held.snapTo(0f) }
                            release()
                            if (travelled < viewConfiguration.touchSlop) {
                                lastTap = down.uptimeMillis
                                lastTapAt = down.position
                            }
                        }
                    }
                },
        ) {
            val pad = inset.toPx()
            val w = size.width - 2 * pad
            val h = size.height - 2 * pad
            fun px(fx: Float) = pad + w * (if (rtl) 1f - fx else fx)
            fun py(fy: Float) = pad + h * (1f - fy)
            val hair = OscylTokens.ruleWidth.toPx()
            // The frame and the printed grid at tenths.
            drawRoundRect(c.rule, Offset(hair / 2, hair / 2), size.copy(size.width - hair, size.height - hair), CornerRadius(OscylTokens.radiusPanel.toPx()), style = Stroke(hair))
            for (i in 0..10) {
                val pen = if (i % 5 == 0) c.ruleStrong else c.rule
                drawLine(pen, Offset(pad + w * i / 10f, pad), Offset(pad + w * i / 10f, pad + h), hair)
                drawLine(pen, Offset(pad, pad + h * i / 10f), Offset(pad + w, pad + h * i / 10f), hair)
            }
            val p = Offset(px(x), py(y))
            val r = 10.dp.toPx()
            if (savedX != null && savedY != null && (savedX != x || savedY != y)) {
                val s = Offset(px(savedX), py(savedY))
                drawLine(c.act, s, p, 2.dp.toPx())
                drawCircle(c.legend, r, s, style = Stroke(1.5.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(2.dp.toPx(), 3.dp.toPx()))))
            }
            if (held.value > 0f) {
                val dash = PathEffect.dashPathEffect(floatArrayOf(2.dp.toPx(), 3.dp.toPx()))
                drawLine(c.legend, p, Offset(if (rtl) pad + w else pad, p.y), hair, pathEffect = dash)
                drawLine(c.legend, p, Offset(p.x, pad + h), hair, pathEffect = dash)
            }
            drawCircle(if (held.value > 0f) c.ink else c.ground, r, p)
            drawCircle(if (enabled) c.ink else c.legend, r, p, style = Stroke(2.dp.toPx()))
        }
        val legend = type.labelMedium.copy(color = c.legend)
        BasicText(xLabel, Modifier.align(Alignment.BottomEnd).padding(end = inset + 4.dp, bottom = inset + 4.dp), style = legend)
        BasicText(yLabel, Modifier.align(Alignment.TopStart).padding(start = inset + 4.dp, top = inset + 4.dp), style = legend)
        // While held, the values on a plate beside the point, kept inside the frame.
        if (held.value > 0f) {
            BasicText(
                "$xText · $yText",
                Modifier
                    // From the top left in every direction: its place is computed in absolute terms.
                    .align(AbsoluteAlignment.TopLeft)
                    .layout { m, cons ->
                        val plate = m.measure(cons.copy(minWidth = 0, minHeight = 0))
                        layout(0, 0) {
                            val pad = inset.toPx()
                            val w = cons.maxWidth - 2 * pad
                            val h = cons.maxHeight - 2 * pad
                            val fx = if (rtl) 1f - x else x
                            val pxv = (pad + w * fx).roundToInt()
                            val pyv = (pad + h * (1f - y)).roundToInt()
                            val off = 32.dp.roundToPx()
                            var px0 = pxv + off
                            if (px0 + plate.width > cons.maxWidth) px0 = pxv - off - plate.width
                            var py0 = pyv - off - plate.height / 2
                            if (py0 < 0) py0 = pyv + off
                            plate.place(px0.coerceIn(0, maxOf(0, cons.maxWidth - plate.width)), py0)
                        }
                    }
                    .background(c.plate, RoundedCornerShape(OscylTokens.radius))
                    .padding(horizontal = 8.dp, vertical = 4.dp),
                style = type.bodyMedium.copy(color = c.onPlate, fontWeight = FontWeight.SemiBold, fontFeatureSettings = "tnum"),
            )
        }
    }
}

@Composable
private fun XyPadSample(edited: Boolean, enabled: Boolean = true) = XyPad(
    x = 0.62f, y = 0.4f,
    savedX = if (edited) 0.42f else 0.62f, savedY = if (edited) 0.25f else 0.4f,
    xLabel = "Cutoff", yLabel = "Resonance", xText = "618", yText = "34%",
    onMove = { _, _ -> }, onRelease = {}, enabled = enabled,
)

@OscylPreviews
@Composable
private fun xyPad(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) { XyPadSample(edited = true) }

@OscylA11yPreviews
@Composable
private fun xyPad_a11y() = PreviewTheme { XyPadSample(edited = true) }
