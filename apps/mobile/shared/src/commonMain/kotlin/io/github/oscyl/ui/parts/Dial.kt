package io.github.oscyl.ui.parts

import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.ui.semantics.disabled
import io.github.oscyl.shared.generated.resources.dial_set_exactly
import androidx.compose.ui.semantics.testTag
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.platform.testTag
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.setProgress
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.oscyl.i18n.formatInteger
import io.github.oscyl.platform.LocalOscylHaptics
import io.github.oscyl.platform.OscylHaptics
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.dial_decrease
import io.github.oscyl.shared.generated.resources.dial_increase
import io.github.oscyl.shared.generated.resources.dial_restore
import io.github.oscyl.shared.generated.resources.dial_state_edited
import io.github.oscyl.shared.generated.resources.dial_was
import io.github.oscyl.shared.generated.resources.ic_undo
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlinx.coroutines.flow.first
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.stringResource

/** A dial's printed scale diameter; Large for a section's leading parameters. On phones every dial is a [Knob]. */
enum class DialSize(internal val ring: Dp) { Small(72.dp), Large(92.dp) }

/** The phone knob's numbers (`mobile.knob` in design/tokens.json). */
private val Knob = OscylTokens.Knob

// The scale runs from 7:30 round to 4:30 (0° is 3 o'clock, clockwise), as on the desktop dial.
private const val StartDeg = 135f
private const val SweepDeg = 270f
/** Printed ticks over the sweep; every fifth is long. */
private const val Ticks = 21
/** Up to this text scale "was N" fits the scale's foot under a phone's knob. */
private const val WasInFootMaxFontScale = 1.3f
/** Below this many ms between haptics, a second one waits unless it is firmer (one per frame at 120 Hz). */
private const val HapticGapMs = 8L

/** Where [v] sits in [range], 0..1. */
internal fun fraction(v: Int, range: IntRange): Float =
    if (range.last <= range.first) 0f else (v.coerceIn(range) - range.first).toFloat() / (range.last - range.first)

/**
 * The unrounded value after the finger moved [up] (negative: down) from [acc] at
 * [speed] dp/ms, clamped to [range]: [Knob]'s travel runs the whole range, and a slow
 * finger (under its fine speed) turns at its fine gain.
 */
internal fun knobDrag(acc: Float, up: Dp, speed: Float, range: IntRange): Float {
    val gain = if (speed < Knob.fineSpeedDpPerMs) Knob.fineGain else 1f
    val delta = up / Knob.travel * (range.last - range.first) * gain
    return (acc + delta).coerceIn(range.first.toFloat(), range.last.toFloat())
}

/** A knob's feels, firmest last; one per move, the firmest winning. */
internal enum class Feel { None, Tick, Detent, Bump }

/**
 * What a move from [from] to [to] feels like: a bump on reaching either end, a detent
 * on reaching or passing [saved] or the centre, else a tick on reaching or passing a
 * printed scale step.
 */
internal fun knobFeel(from: Int, to: Int, saved: Int, range: IntRange): Feel {
    if (from == to) return Feel.None
    fun reached(v: Float) = if (to > from) v > from && v <= to else v < from && v >= to
    val span = range.last - range.first
    return when {
        to == range.first || to == range.last -> Feel.Bump
        reached(saved.toFloat()) || reached(range.first + span / 2f) -> Feel.Detent
        (1 until Ticks - 1).any { reached(range.first + it * span / (Ticks - 1f)) } -> Feel.Tick
        else -> Feel.None
    }
}

internal fun OscylHaptics.feel(feel: Feel) = when (feel) {
    Feel.None -> Unit
    Feel.Tick -> tick()
    Feel.Detent -> detent()
    Feel.Bump -> bump()
}

/**
 * One step of a critically damped spring pulling [x] (moving at [v] per s) to [target]
 * over [dt] s with [stiffness] (ω², 1/s²), in sub-steps short enough to stay stable
 * through a dropped frame. Returns the new position and speed.
 */
internal fun springStep(x: Float, v: Float, target: Float, dt: Float, stiffness: Float): Pair<Float, Float> {
    val w = kotlin.math.sqrt(stiffness)
    var pos = x
    var vel = v
    val steps = max(1, (dt / 0.004f).roundToInt())
    val h = dt / steps
    repeat(steps) {
        vel += (stiffness * (target - pos) - 2f * w * vel) * h
        pos += vel * h
    }
    return pos to vel
}

/** Whether this window is a phone's (compact width, or compact height on its side): knobs, no printed numerals, "was" inline. */
@Composable
private fun phoneWidth(): Boolean {
    val size = androidx.compose.material3.adaptive.currentWindowAdaptiveInfoV2().windowSizeClass
    return !size.isWidthAtLeastBreakpoint(androidx.window.core.layout.WindowSizeClass.WIDTH_DP_MEDIUM_LOWER_BOUND) ||
        !size.isHeightAtLeastBreakpoint(androidx.window.core.layout.WindowSizeClass.HEIGHT_DP_MEDIUM_LOWER_BOUND)
}

/**
 * The front-panel dial (DESIGN.md "Dial"): a printed scale of 21 ticks over 270°, an
 * enamel disc and an ink index. While edited, a dotted ghost marks [saved], an orange
 * arc runs from it to the value, a dot follows the [label] and "was N" restores.
 *
 * It is a physical knob. A touch on the disc or its ring grabs: from then until the
 * finger lifts the knob owns the gesture, so the page cannot scroll; the value and
 * label below do not grab, so the page scrolls between knobs. Up turns it up; 220 dp
 * runs the range, a slow finger turns at 0.35×. The index stays under the finger;
 * a change the finger did not make (a restore, the synth's own knob) reaches it
 * through a critically damped spring (heavier while [comparing]), in the draw phase.
 * The value goes to the synth at once. Haptics: a tick per printed step,
 * a detent at [saved] and the centre, a bump at an end, at most one per frame (the
 * firmest wins). A double tap restores [saved], with a confirmation.
 *
 * On a phone the disc is 64 dp in a 96 dp target, the printed 0/5/10 go and "was N"
 * sits inline after the value. Wider, the numerals and the "was" line under it stay.
 */
@Composable
fun Dial(
    value: Int,
    saved: Int,
    range: IntRange,
    label: String,
    format: (Int) -> String,
    onChange: (Int) -> Unit,
    size: DialSize = DialSize.Large,
    modifier: Modifier = Modifier,
    comparing: Boolean = false,
    onRelease: () -> Unit = {},
    /** False for a setting with no saved value (Generate's shape dials): no "was", and no space kept for one. */
    revertible: Boolean = true,
    /** Typed text to a value (Plan B); null: the stepper has no field to type into. */
    typed: ((String) -> Int?)? = null,
    /** False on a program that cannot change: the knob shows and says its value, nothing turns it. */
    enabled: Boolean = true,
) {
    val c = OscylTheme.colors
    val haptics by rememberUpdatedState(LocalOscylHaptics.current)
    val ltr = LocalLayoutDirection.current == LayoutDirection.Ltr
    val motion = LocalOscylMotion.current
    val phone = phoneWidth()
    val edited = value != saved

    // Held, the index is under the finger. Otherwise (a restore, the synth's own knob,
    // compare) it has weight: a critically damped spring pulls it to the value, frame by frame.
    var active by remember { mutableStateOf(false) }
    val shown = remember { mutableFloatStateOf(fraction(value, range)) }
    val target = rememberUpdatedState(fraction(value, range))
    val stiffness = rememberUpdatedState(if (comparing) Knob.springStiffnessComparing else Knob.springStiffness)
    LaunchedEffect(motion) {
        while (true) {
            snapshotFlow { target.value }.first { it != shown.floatValue }
            val scale = motionScale(motion)
            if (scale == 0f || active) {
                shown.floatValue = target.value
                continue
            }
            var last = -1L
            var speed = 0f
            while (shown.floatValue != target.value || speed != 0f) {
                withFrameNanos { now ->
                    val dt = if (last < 0) 1f / 120 else ((now - last) / 1e9f).coerceAtMost(0.05f)
                    last = now
                    val t = target.value
                    if (active) {
                        shown.floatValue = t
                        speed = 0f
                        return@withFrameNanos
                    }
                    // A slower animator scale stretches the spring's time.
                    val (x, v) = springStep(shown.floatValue, speed, t, dt, stiffness.value / (scale * scale))
                    if (abs(t - x) < 0.0005f && abs(v) < 0.01f) {
                        shown.floatValue = t
                        speed = 0f
                    } else {
                        shown.floatValue = x
                        speed = v
                    }
                }
            }
        }
    }

    val current = rememberUpdatedState(value)
    val change = rememberUpdatedState(onChange)
    val release = rememberUpdatedState(onRelease)
    val savedNow = rememberUpdatedState(saved)
    // A step from a key or a screen reader is a gesture of its own: it ends at once.
    fun set(v: Int) {
        val next = v.coerceIn(range)
        if (next != current.value) {
            change.value(next)
            release.value()
        }
    }
    val restore = {
        set(savedNow.value)
        haptics.confirm()
    }
    val restoreNow = rememberUpdatedState(restore)
    // Screen readers step one printed scale step, as their own slider gestures do.
    val step = max(1, ((range.last - range.first) / (Ticks - 1f)).roundToInt())

    val interaction = remember { MutableInteractionSource() }
    val focused by interaction.collectIsFocusedAsState()
    val edit by animateFloatAsState(if (edited) 1f else 0f, tween(if (motion) OscylTokens.baseMs else 0, easing = OscylTokens.easing))

    val valueText = format(value)
    val state = if (edited) stringResource(Res.string.dial_state_edited, valueText, format(saved)) else valueText
    val increase = stringResource(Res.string.dial_increase)
    val decrease = stringResource(Res.string.dial_decrease)
    val restoreLabel = stringResource(Res.string.dial_restore)
    val setExactly = stringResource(Res.string.dial_set_exactly)
    // The stepper under the value, from a long press there or the screen reader's action.
    var stepping by remember { mutableStateOf(false) }
    val density = LocalDensity.current
    val grab = with(density) { Knob.grab.toPx() / 2 }
    val centre = remember { FaceCentre() }
    // On a phone "was N" is printed in the scale's open foot, under the disc: nothing
    // moves when a knob becomes edited. At large text it has a line of its own, always kept.
    val wasText = if (phone && revertible) stringResource(Res.string.dial_was, format(saved)) else null
    val wasInFoot = density.fontScale <= WasInFootMaxFontScale
    // Printed in the foot, "was N" leads with the undo mark and a tap on it restores.
    val footRestores = rememberUpdatedState(wasText != null && wasInFoot && edited)

    // Off (a factory slot, a slot not read): drawn at 60%, so it does not look turnable.
    Column(modifier.width(IntrinsicSize.Max).graphicsLayer { alpha = if (enabled) 1f else 0.6f }, horizontalAlignment = Alignment.CenterHorizontally) {
        Column(
            Modifier
                .semantics(mergeDescendants = true) {
                    contentDescription = label
                    stateDescription = state
                    progressBarRangeInfo = ProgressBarRangeInfo(value.toFloat(), range.first.toFloat()..range.last.toFloat())
                    if (!enabled) {
                        disabled()
                        return@semantics
                    }
                    setProgress { set(it.roundToInt()); true }
                    customActions = listOfNotNull(
                        CustomAccessibilityAction(increase) { set(current.value + step); true },
                        CustomAccessibilityAction(decrease) { set(current.value - step); true },
                        if (edited) CustomAccessibilityAction(restoreLabel) { restore(); true } else null,
                        CustomAccessibilityAction(setExactly) { stepping = true; true },
                    )
                }
                .onKeyEvent { event ->
                    if (!enabled || event.type != KeyEventType.KeyDown) return@onKeyEvent false
                    when (event.key) {
                        // Right raises left to right, left raises right to left, as sliders do.
                        Key.DirectionUp -> set(current.value + 1)
                        Key.DirectionDown -> set(current.value - 1)
                        Key.DirectionRight -> set(current.value + if (ltr) 1 else -1)
                        Key.DirectionLeft -> set(current.value + if (ltr) -1 else 1)
                        Key.PageUp -> set(current.value + step)
                        Key.PageDown -> set(current.value - step)
                        else -> return@onKeyEvent false
                    }
                    true
                }
                .focusable(interactionSource = interaction),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            DialFace(
                size, phone, edited, saved, range, { shown.floatValue }, { active }, focused, centre,
                was = wasText?.let { "↶ $it" }.takeIf { wasInFoot && edited },
                wasAlpha = { edit },
                modifier = Modifier.testTag("knob").pointerInput(range, enabled) {
                    if (!enabled) return@pointerInput
                    var lastTap = Long.MIN_VALUE / 2
                    var lastTapAt = Offset.Zero
                    awaitEachGesture {
                        val down = awaitFirstDown(requireUnconsumed = false)
                        // Only the disc and its ring grab; elsewhere the page scrolls. A tap on
                        // "↶ was N" in the scale's foot restores, as the segments' line does.
                        if ((down.position - centre.at).getDistance() > grab) {
                            // Only around the words (not the face's whole lower band), and never the
                            // touch a flinging page took to stop itself.
                            val nearWords = kotlin.math.abs(down.position.x - centre.at.x) < 48.dp.toPx() && down.position.y > centre.at.y + grab * 0.6f
                            if (footRestores.value && nearWords && !down.isConsumed) {
                                val up = waitForUpOrCancellation()
                                if (up != null) {
                                    up.consume()
                                    restoreNow.value()
                                }
                            }
                            return@awaitEachGesture
                        }
                        down.consume()
                        if (down.uptimeMillis - lastTap < viewConfiguration.doubleTapTimeoutMillis &&
                            (down.position - lastTapAt).getDistance() < viewConfiguration.touchSlop * 3
                        ) {
                            lastTap = Long.MIN_VALUE / 2
                            restoreNow.value()
                            do {
                                val e = awaitPointerEvent()
                                e.changes.forEach { it.consume() }
                            } while (e.changes.any { it.id == down.id && it.pressed })
                            return@awaitEachGesture
                        }
                        var acc = current.value.toFloat()
                        var last = current.value
                        var travelled = 0f
                        var speed = Float.MAX_VALUE
                        var then = down.uptimeMillis
                        var felt = Long.MIN_VALUE / 2
                        var feltHow = Feel.None
                        var changed = false
                        active = true
                        try {
                            while (true) {
                                val event = awaitPointerEvent()
                                val ch = event.changes.firstOrNull { it.id == down.id } ?: break
                                if (!ch.pressed) {
                                    ch.consume()
                                    if (travelled < viewConfiguration.touchSlop) {
                                        lastTap = ch.uptimeMillis
                                        lastTapAt = ch.position
                                    }
                                    break
                                }
                                val dy = ch.positionChange().y
                                ch.consume()
                                if (dy == 0f) continue
                                travelled += abs(dy)
                                val dt = (ch.uptimeMillis - then).coerceAtLeast(1L)
                                then = ch.uptimeMillis
                                // The finger's speed, smoothed over a few events (dp/ms).
                                val now = abs(dy).toDp().value / dt
                                speed = if (speed == Float.MAX_VALUE) now else speed * 0.5f + now * 0.5f
                                acc = knobDrag(acc, (-dy).toDp(), speed, range)
                                val next = acc.roundToInt()
                                if (next != last) {
                                    val feel = knobFeel(last, next, savedNow.value, range)
                                    if (feel != Feel.None && (ch.uptimeMillis - felt >= HapticGapMs || feel > feltHow)) {
                                        haptics.feel(feel)
                                        felt = ch.uptimeMillis
                                        feltHow = feel
                                    }
                                    last = next
                                    changed = true
                                    change.value(next)
                                }
                            }
                        } finally {
                            active = false
                            if (changed) release.value()
                        }
                    }
                },
            )
            // The value and its name: one target (at least 48 dp tall), a tap or a long press
            // opens the stepper under it.
            Column(
                Modifier
                    .heightIn(min = 48.dp)
                    .pointerInput(enabled) {
                        if (enabled) {
                            detectTapGestures(
                                onTap = { stepping = true },
                                onLongPress = { haptics.tick(); stepping = true },
                            )
                        }
                    },
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Box(Modifier.clearAndSetSemantics { testTag = "dial-value" }) {
                    DialValue({ shown.floatValue }, range, format, label)
                    // It arrives and leaves as every transient part does.
                    val stepper = rememberTransient(stepping)
                    if (stepper.up) {
                        ValueStepper(
                            value, range, valueText, label,
                            shown = { stepper.value },
                            onChange = { change.value(it.coerceIn(range)) },
                            onDone = { release.value() },
                            onDismiss = { stepping = false },
                            typed = typed,
                            // Edited: the way back to saved is in the stepper too.
                            savedText = format(saved).takeIf { edited && revertible },
                            onRestore = { restore(); stepping = false },
                        )
                    }
                }
                Row(Modifier.clearAndSetSemantics {}, verticalAlignment = Alignment.CenterVertically) {
                    // The edit dot follows the label, as a selector's does; its twin space before
                    // keeps the label centred.
                    Spacer(Modifier.width(12.dp))
                    BasicText(
                        label,
                        style = MaterialTheme.typography.bodyMedium.copy(
                            color = c.legend,
                            textAlign = TextAlign.Center,
                            textDirection = TextDirection.Content,
                        ),
                    )
                    Spacer(Modifier.width(6.dp))
                    Canvas(Modifier.size(6.dp).graphicsLayer { alpha = edit }) { drawCircle(c.act) }
                }
            }
            // At large text, the same "↶ was N" line as wider windows: a target that restores.
            if (wasText != null && !wasInFoot) WasLine(format(saved), edited, { edit }, onRestore = restore)
        }
        if (revertible && !phone) WasLine(format(saved), edited, { edit }, onRestore = restore)
    }
}

/** Where the face's centre is (px, in the face), as it last drew: where a touch grabs. */
private class FaceCentre {
    var at = Offset.Zero
}

/** The scale, disc, index and, while edited, the ghost and the orange arc. */
@Composable
private fun DialFace(
    size: DialSize,
    phone: Boolean,
    edited: Boolean,
    saved: Int,
    range: IntRange,
    shown: () -> Float,
    active: () -> Boolean,
    focused: Boolean,
    centre: FaceCentre,
    /** "was N", printed in the scale's open foot while edited (a phone's knob); null prints nothing. */
    was: String?,
    wasAlpha: () -> Float,
    modifier: Modifier,
) {
    val c = OscylTheme.colors
    val density = LocalDensity.current
    // A phone's knob prints no numerals: only "was N" is laid out, when there is one.
    val measurer = rememberTextMeasurer(cacheSize = if (phone) 1 else 8)
    val wasStyle = MaterialTheme.typography.bodySmall.copy(color = c.act, fontWeight = FontWeight.Medium, fontFeatureSettings = "tnum")
    val wasLayout = remember(was, wasStyle) { was?.let { measurer.measure(it, wasStyle, softWrap = false, maxLines = 1) } }
    val numeralStyle = MaterialTheme.typography.bodyMedium.let {
        it.copy(fontSize = it.fontSize * 0.6f, fontWeight = FontWeight.Medium, color = c.legend, letterSpacing = 0.sp)
    }
    val numerals = if (phone) null else remember(measurer, numeralStyle) {
        Triple(measurer.measure(formatInteger(5), numeralStyle), measurer.measure(formatInteger(0), numeralStyle), measurer.measure(formatInteger(10), numeralStyle))
    }

    // Geometry in px (as the desktop `draw`): the "5" sits above the scale, "0" and "10"
    // below its end ticks; large text widens and deepens the face rather than clipping.
    // Every dial takes a large dial's height, its scale centred there, so values and
    // labels share one line across a row of mixed sizes.
    val g = with(density) {
        if (phone) {
            // The knob: the scale just inside the 96 dp target, round a 64 dp disc.
            val target = Knob.target.toPx()
            val ring = target - 4.dp.toPx()
            val rOut = ring / 2 - 1.dp.toPx()
            return@with DialGeometry(ring, (target - ring) / 2, ring, rOut, rOut - 3.dp.toPx(), Knob.disc.toPx() / 2, Size(target, target))
        }
        val (five, zero, ten) = numerals!!
        val top = five.size.height + 1f
        val numeral = Size(maxOf(zero.size.width, ten.size.width).toFloat(), maxOf(zero.size.height, ten.size.height).toFloat())
        fun extent(ring: Float): Pair<Float, Float> {
            val diag = 0.7071f * (ring / 2 - 4.dp.toPx())
            return maxOf(ring / 2, diag + numeral.width + 1f) to maxOf(ring / 2, diag + 1f + numeral.height)
        }
        val ring = size.ring.toPx()
        val large = DialSize.Large.ring.toPx()
        val rOut = ring / 2 - 1.dp.toPx()
        val half = extent(ring).first
        DialGeometry(ring, top, large, rOut, rOut - 3.dp.toPx(), (rOut * 0.7f).roundToInt().toFloat(), Size(2 * half, top + large / 2 + extent(large).second))
    }
    centre.at = Offset(g.cx, g.cy)
    Box(modifier.size(with(density) { g.canvas.width.toDp() }, with(density) { g.canvas.height.toDp() })) {
        // The disc's shadow, the printed scale and the disc on a layer of their own: kept
        // while the index turns and while the page scrolls.
        Box(Modifier.matchParentSize().graphicsLayer { compositingStrategy = CompositingStrategy.Offscreen }) {
        Box(
            Modifier
                .offset { IntOffset((g.cx - g.rDisc).roundToInt(), (g.cy - g.rDisc).roundToInt()) }
                .size(with(density) { (2 * g.rDisc).toDp() })
                .softShadow(CircleShape, 4.dp, DpOffset(0.dp, 1.5.dp), c.shadow),
        )
        Canvas(Modifier.matchParentSize()) {
            val center = Offset(g.cx, g.cy)
            fun at(f: Float, r: Float): Offset {
                val a = (StartDeg + SweepDeg * f) * PI.toFloat() / 180f
                return center + Offset(r * cos(a), r * sin(a))
            }
            val ruleWidth = OscylTokens.ruleWidth.toPx()
            for (i in 0 until Ticks) {
                val f = i / (Ticks - 1f)
                val r0 = g.rScale * if (i % 5 == 0) 0.80f else 0.86f
                drawLine(c.legend, at(f, r0), at(f, g.rScale), ruleWidth)
            }
            if (numerals != null) drawText(numerals.first, topLeft = Offset((g.cx - numerals.first.size.width / 2f).roundToInt().toFloat(), (g.cy - g.rOut - g.top).roundToInt().toFloat()))
            if (numerals != null) at(0f, g.rScale).let { drawText(numerals.second, topLeft = Offset((it.x - numerals.second.size.width - 1f).roundToInt().toFloat(), (it.y + 1f).roundToInt().toFloat())) }
            if (numerals != null) at(1f, g.rScale).let { drawText(numerals.third, topLeft = Offset((it.x + 1f).roundToInt().toFloat(), (it.y + 1f).roundToInt().toFloat())) }

            // The disc: raised enamel swept light to dark, darker while turned or focused.
            val fill = if (active()) lerp(c.raised, c.ink, 0.18f) else c.raised
            drawCircle(
                Brush.verticalGradient(
                    listOf(lerp(fill, Color.White, 0.07f), lerp(fill, Color.Black, 0.10f)),
                    startY = g.cy - g.rDisc,
                    endY = g.cy + g.rDisc,
                ),
                g.rDisc,
                center,
            )
            drawCircle(if (active()) c.legend else c.ruleStrong, g.rDisc - ruleWidth / 2, center, style = Stroke(ruleWidth))
            if (focused) drawCircle(c.focus, g.rOut + 3.dp.toPx(), center, style = Stroke(2.dp.toPx()))
            // While edited, the dotted ghost at the saved value (still while the knob turns).
            if (edited) {
                val s = fraction(saved, range)
                drawLine(
                    c.legend,
                    at(s, g.rDisc * 0.62f),
                    at(s, g.rDisc * 1.08f),
                    maxOf(g.ring * 0.03f, 2.dp.toPx()),
                    StrokeCap.Round,
                    PathEffect.dashPathEffect(floatArrayOf(0.1.dp.toPx(), 3.2.dp.toPx())),
                )
            }
            // "was N" in the foot of the scale, between its end ticks, centred under the disc.
            if (wasLayout != null) {
                val foot = g.cy + g.rDisc
                drawText(
                    wasLayout,
                    topLeft = Offset(
                        (g.cx - wasLayout.size.width / 2f).roundToInt().toFloat(),
                        (foot + (this.size.height - foot - wasLayout.size.height) / 2f).roundToInt().toFloat(),
                    ),
                    alpha = wasAlpha(),
                )
            }
        }
        }
        // The moving parts: the index and the ink arc, and while edited the ghost and the orange arc over it.
        Canvas(Modifier.matchParentSize()) {
            val center = Offset(g.cx, g.cy)
            fun at(f: Float, r: Float): Offset {
                val a = (StartDeg + SweepDeg * f) * PI.toFloat() / 180f
                return center + Offset(r * cos(a), r * sin(a))
            }
            val f = shown()
            val rArc = g.rOut - 0.5f
            // The value as an ink arc from zero; none at zero (a round cap would leave a dot).
            if (f > 0f) {
                drawArc(
                    c.ink,
                    startAngle = StartDeg,
                    sweepAngle = SweepDeg * f,
                    useCenter = false,
                    topLeft = center - Offset(rArc, rArc),
                    size = Size(2 * rArc, 2 * rArc),
                    style = Stroke(maxOf(g.ring * 0.02f, 1.5.dp.toPx()), cap = StrokeCap.Round),
                )
            }
            if (edited) {
                val s = fraction(saved, range)
                val (f0, f1) = minOf(s, f) to maxOf(s, f)
                if (f1 > f0) {
                    drawArc(
                        c.act,
                        startAngle = StartDeg + SweepDeg * f0,
                        sweepAngle = SweepDeg * (f1 - f0),
                        useCenter = false,
                        topLeft = center - Offset(rArc, rArc),
                        size = Size(2 * rArc, 2 * rArc),
                        style = Stroke(maxOf(g.ring * 0.028f, 2.dp.toPx()), cap = StrokeCap.Round),
                    )
                }
                drawCircle(c.act, maxOf(g.ring * 0.03f, 2.5.dp.toPx()), at(f, rArc))
            }
            drawLine(c.ink, at(f, g.rDisc * 0.38f), at(f, g.rDisc * 0.84f), maxOf(g.ring * 0.035f, 2.dp.toPx()), StrokeCap.Round)
        }
    }
}

private class DialGeometry(
    val ring: Float,
    val top: Float,
    large: Float,
    val rOut: Float,
    val rScale: Float,
    val rDisc: Float,
    val canvas: Size,
) {
    val cx = (canvas.width / 2).roundToInt().toFloat()
    val cy = (top + large / 2).roundToInt().toFloat()
}

/** The value under the disc, rolling with the index; only this line recomposes while it glides. */
@Composable
private fun DialValue(shown: () -> Float, range: IntRange, format: (Int) -> String, field: String) {
    val rolling = (range.first + shown() * (range.last - range.first)).roundToInt()
    val style = MaterialTheme.typography.bodyLarge.copy(
        color = OscylTheme.colors.ink,
        fontWeight = FontWeight.SemiBold,
        fontFeatureSettings = "tnum",
        textAlign = TextAlign.Center,
    )
    // The box is as wide as the field's widest value, so a turning number never moves the layout.
    val widest = widestValue(range, format, style, field)
    BasicText(format(rolling), Modifier.padding(top = 2.dp).width(widest), style = style)
}

/**
 * The widest of [range]'s values as [format] writes them in [style]: the ends, their
 * neighbours and the printed steps (where a sign, a unit or a word changes the width).
 * Formatting asks the core, so it is done once per field and kept for the process:
 * a dial scrolling back into view measures nothing.
 */
@Composable
private fun widestValue(range: IntRange, format: (Int) -> String, style: androidx.compose.ui.text.TextStyle, field: String): Dp {
    val measurer = rememberTextMeasurer()
    val density = LocalDensity.current
    // The ends as written now carry the language and the unit: a key no other field or locale shares.
    val key = "$field|$range|${format(range.first)}|${format(range.last)}|${style.fontSize}|${density.density}|${density.fontScale}"
    return remember(key) {
        WidestCache.getOrPut(key) {
            val span = range.last - range.first
            val values = ((0..Ticks).map { range.first + it * span / Ticks } + listOf(range.first + 1, range.last - 1)).distinct()
            // The longest strings are the widest in tabular figures; measure only those.
            val words = values.map(format)
            val longest = words.maxOf { it.length }
            val px = words.filter { it.length >= longest - 1 }.distinct().maxOf { measurer.measure(it, style).size.width }
            with(density) { px.toDp() } + 1.dp
        }
    }
}

/** Each field's widest value, by field, range, text size and density. */
private val WidestCache = HashMap<String, Dp>()

/** "↶ was 431" in orange while edited, restoring on a tap; laid out (invisibly) when not. */
@Composable
internal fun WasLine(savedText: String, edited: Boolean, alpha: () -> Float, onRestore: () -> Unit) {
    val c = OscylTheme.colors
    val interaction = remember { MutableInteractionSource() }
    val shape = RoundedCornerShape(OscylTokens.radius)
    val action = if (edited) {
        Modifier
            .clickable(interaction, indication = null, role = Role.Button, onClickLabel = stringResource(Res.string.dial_restore), onClick = onRestore)
            .touchStates(interaction, shape, c.press)
    } else {
        Modifier.clearAndSetSemantics {}
    }
    Row(
        // A 48 dp target, its height always kept below the dial.
        Modifier.graphicsLayer { this.alpha = alpha() }.then(action).heightIn(min = 48.dp).padding(horizontal = 6.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        val style = MaterialTheme.typography.bodySmall.copy(color = c.act, fontWeight = FontWeight.Medium)
        // The glyph grows with the text it leads.
        Image(
            painterResource(Res.drawable.ic_undo),
            null,
            Modifier.size(with(LocalDensity.current) { style.fontSize.toDp() }),
            colorFilter = ColorFilter.tint(c.act),
        )
        BasicText(stringResource(Res.string.dial_was, savedText), style = style)
    }
}

@Composable
private fun DialSamples() {
    // As in a section panel: dials flow onto more lines as text grows.
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Dial(618, saved = 431, range = 0..1023, label = "Cutoff", format = { formatInteger(it) }, onChange = {})
        Dial(347, saved = 347, range = 0..1023, label = "Resonance", format = { formatInteger(it) }, onChange = {}, size = DialSize.Small)
        Dial(0, saved = 812, range = 0..1023, label = "EG Int", format = { formatInteger(it) }, onChange = {}, size = DialSize.Small)
    }
}

@OscylPreviews
@Composable
private fun dial_edited(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) { DialSamples() }

@OscylA11yPreviews
@Composable
private fun dial_a11y() = PreviewTheme { DialSamples() }

@OscylPreviews
@Composable
private fun dial_ends(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Dial(0, saved = 0, range = 0..1023, label = "Minimum", format = { formatInteger(it) }, onChange = {})
        Dial(1023, saved = 1023, range = 0..1023, label = "Maximum", format = { formatInteger(it) }, onChange = {})
        Dial(200, saved = 700, range = 0..1023, label = "Below", format = { formatInteger(it) }, onChange = {})
    }
}
