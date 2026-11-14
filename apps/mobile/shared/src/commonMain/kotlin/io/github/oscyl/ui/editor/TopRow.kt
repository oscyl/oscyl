package io.github.oscyl.ui.editor

import io.github.oscyl.shared.generated.resources.morph_slide
import io.github.oscyl.platform.excludeSystemGestures
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.draw.drawBehind
import androidx.compose.runtime.setValue
import androidx.compose.runtime.Stable
import androidx.compose.animation.core.animateFloatAsState
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.theme.OscylThemeMode
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.platform.LocalLayoutDirection
import kotlin.math.abs
import io.github.oscyl.ui.parts.ProgressScale
import io.github.oscyl.shared.generated.resources.state_saved
import io.github.oscyl.shared.generated.resources.morph_toward_saved
import io.github.oscyl.shared.generated.resources.morph_toward_edit
import io.github.oscyl.shared.generated.resources.morph_edit
import androidx.compose.ui.window.PopupPositionProvider
import androidx.compose.ui.window.Popup
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.semantics.setProgress
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.MutableState
import androidx.compose.foundation.layout.width
import androidx.compose.animation.core.animate
import io.github.oscyl.shared.generated.resources.editor_hold
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.state.ToggleableState
import androidx.compose.ui.semantics.toggleableState
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import io.github.oscyl.platform.LocalOscylHaptics
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.editor_compare
import io.github.oscyl.shared.generated.resources.editor_live
import io.github.oscyl.shared.generated.resources.ic_compare
import io.github.oscyl.shared.generated.resources.ic_redo
import io.github.oscyl.shared.generated.resources.ic_undo
import io.github.oscyl.shared.generated.resources.programs_redo
import io.github.oscyl.shared.generated.resources.programs_undo
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.Lamp
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.floating
import io.github.oscyl.ui.parts.liftedPlate
import io.github.oscyl.ui.parts.motionScale
import kotlinx.coroutines.launch
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.stringResource
import kotlin.math.roundToInt

/** An outlined button's frame, its 48 dp target kept around a 36 dp face. */
@Composable
private fun OutlinedFace(held: Boolean, modifier: Modifier, content: @Composable () -> Unit) {
    val c = OscylTheme.colors
    val shape = RoundedCornerShape(OscylTokens.radius)
    Box(modifier.defaultMinSize(minWidth = 48.dp, minHeight = 48.dp), contentAlignment = Alignment.Center) {
        Row(
            Modifier
                .heightIn(min = 36.dp)
                .background(if (held) c.press else c.press.copy(alpha = 0f), shape)
                .border(OscylTokens.ruleWidth, c.ruleStrong, shape)
                .padding(horizontal = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) { content() }
    }
}

/** How far a finger slides to go from saved all the way to the edit. */
private val MorphSpan = 160.dp

/**
 * What the Compare part is doing, for the fader beside it: held (compare on), and
 * sliding (morphing). Hoisted so the fader stands in its own fixed place, composed once,
 * and a press composes nothing and moves nothing.
 */
@Stable
class CompareState {
    var held by mutableStateOf(false)
    var sliding by mutableStateOf(false)
}

/**
 * Hold to compare: acts while pressed (a haptic tick on press and on release). With
 * [onMorph], sliding away from the edge it stands on ([towardEnd] when it stands at the
 * start edge) blends saved and edited: 0 to 160 dp is saved to the edit, read per pointer
 * event; letting go glides to the edit on `base` and ends it. A press too short to hear
 * anything calls [onShortTap] (the app says how it works). A screen reader's double tap
 * toggles compare, since it cannot hold, and adjusts the blend as a slider in tenths.
 * Off ([enabled] false) when there is nothing to compare.
 */
@Composable
private fun Modifier.compareHold(
    comparing: Boolean,
    onCompare: (Boolean) -> Unit,
    onMorph: ((Float?) -> Unit)? = null,
    blend: () -> Float = { if (comparing) 0f else 1f },
    state: CompareState? = null,
    towardEnd: Boolean = false,
    enabled: Boolean = true,
    onShortTap: (() -> Unit)? = null,
): Modifier {
    val haptics = LocalOscylHaptics.current
    val motion = LocalOscylMotion.current
    val scope = rememberCoroutineScope()
    val compare = rememberUpdatedState(onCompare)
    val morph = rememberUpdatedState(onMorph)
    val shortTap = rememberUpdatedState(onShortTap)
    val now = rememberUpdatedState(comparing)
    val rtlNow = rememberUpdatedState(LocalLayoutDirection.current == LayoutDirection.Rtl)
    val endNow = rememberUpdatedState(towardEnd)
    // The glide after a slide ends the morph; a new press takes over from it.
    val glide = remember { arrayOfNulls<Job>(1) }
    val label = stringResource(Res.string.editor_compare)
    val more = stringResource(Res.string.morph_toward_edit)
    val less = stringResource(Res.string.morph_toward_saved)
    return clearAndSetSemantics {
        contentDescription = label
        role = Role.Button
        selected = comparing
        if (!enabled) {
            disabled()
            return@clearAndSetSemantics
        }
        onClick { compare.value(!now.value); true }
        if (onMorph != null) {
            val t = blend()
            progressBarRangeInfo = ProgressBarRangeInfo(t, 0f..1f, steps = 9)
            stateDescription = "${(t * 100).roundToInt()}%"
            setProgress { to -> onMorph(if (to >= 1f) null else to.coerceIn(0f, 1f)); true }
            customActions = listOf(
                CustomAccessibilityAction(more) { onMorph((blend() + 0.1f).takeIf { it < 1f }); true },
                CustomAccessibilityAction(less) { onMorph((blend() - 0.1f).coerceAtLeast(0f)); true },
            )
        }
    }.excludeSystemGestures().pointerInput(enabled) {
        if (!enabled) return@pointerInput
        awaitEachGesture {
            val down = awaitFirstDown()
            down.consume()
            // A glide still running is cut short: this press owns the blend now.
            glide[0]?.cancel()
            glide[0] = null
            haptics.tick()
            compare.value(true)
            state?.held = true
            val span = MorphSpan.toPx()
            // Away from the edge the part stands on: toward the start, or the end at the start edge.
            val sign = (if (rtlNow.value) 1f else -1f) * (if (endNow.value) -1f else 1f)
            var toward = 0f
            var sliding = false
            var t = 0f
            var upAt = down.uptimeMillis
            try {
                while (true) {
                    val event = awaitPointerEvent()
                    val ch = event.changes.firstOrNull { it.id == down.id } ?: break
                    if (!ch.pressed) {
                        ch.consume()
                        upAt = ch.uptimeMillis
                        break
                    }
                    val dx = ch.positionChange().x
                    ch.consume()
                    val m = morph.value ?: continue
                    toward += sign * dx
                    if (!sliding && abs(toward) > viewConfiguration.touchSlop) {
                        sliding = true
                        state?.sliding = true
                    }
                    if (sliding) {
                        val before = t
                        t = (toward / span).coerceIn(0f, 1f)
                        m(t)
                        // Felt as a knob is: a tick per tenth, a detent at saved and at the edit.
                        if (t != before) {
                            if ((t == 0f || t == 1f)) haptics.detent()
                            else if ((t * 10).toInt() != (before * 10).toInt()) haptics.tick()
                        }
                    }
                }
            } finally {
                haptics.tick()
                state?.held = false
                val m = morph.value
                if (sliding && m != null) {
                    // Letting go glides on to the edit, then ends the morph: always, even if the
                    // part leaves the screen mid-glide; not when a new press cut it short.
                    var job: Job? = null
                    job = scope.launch(start = CoroutineStart.LAZY) {
                        try {
                            val scale = motionScale(motion)
                            if (scale > 0f) animate(t, 1f, animationSpec = tween((OscylTokens.baseMs * scale).roundToInt(), easing = OscylTokens.easing)) { v, _ -> m(v) }
                        } finally {
                            if (glide[0] === job) {
                                glide[0] = null
                                m(null)
                            }
                            state?.sliding = false
                        }
                    }
                    glide[0] = job
                    job.start()
                } else {
                    compare.value(false)
                    // Too short to hear the saved sound: say how the part works.
                    if (upAt - down.uptimeMillis < viewConfiguration.longPressTimeoutMillis) shortTap.value?.invoke()
                }
            }
        }
    }
}

/**
 * The Compare fader: a floating plate beside the Compare part while it is held, "Edited"
 * at the far end, the tuning scale from the part outward with its index at the blend,
 * "Saved" by the part; "Slide to blend" over the scale until the slide starts. Composed
 * once in its own fixed place and drawn only while held (alpha and translation on its
 * layer, `fast`): nothing around it moves. [towardEnd] mirrors it for a part at the start edge.
 */
@Composable
internal fun CompareFader(state: CompareState, blend: () -> Float, modifier: Modifier = Modifier, towardEnd: Boolean = false) {
    val c = OscylTheme.colors
    val motion = LocalOscylMotion.current
    val shown by animateFloatAsState(if (state.held || state.sliding) 1f else 0f, tween(if (motion) OscylTokens.fastMs else 0, easing = OscylTokens.easing))
    val hint by animateFloatAsState(if (state.sliding) 0f else 1f, tween(if (motion) OscylTokens.fastMs else 0, easing = OscylTokens.easing))
    val rtl = LocalLayoutDirection.current == LayoutDirection.Rtl
    Box(
        modifier.clearAndSetSemantics {}.graphicsLayer {
            alpha = shown
            // From behind the part: toward it while it fades in.
            val away = (1f - shown) * 24.dp.toPx()
            translationX = if ((towardEnd) != rtl) -away else away
        },
    ) {
        if (shown > 0f) MorphScalePlate(blend, hint = { hint }, towardEnd = towardEnd)
    }
}

@Composable
private fun MorphScalePlate(blend: () -> Float, hint: () -> Float = { 0f }, towardEnd: Boolean = false) {
    val c = OscylTheme.colors
    val style = MaterialTheme.typography.labelMedium.copy(color = c.legend)
    Row(
        Modifier
            .heightIn(min = 48.dp)
            .floating(c, RoundedCornerShape(OscylTokens.radiusPanel))
            .padding(horizontal = 12.dp)
            .testTag("morph-scale"),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // Away from the part is toward the edit: "Edited" at the far end, "Saved" by the part.
        val far = @Composable { BasicText(stringResource(Res.string.morph_edit), style = style) }
        val near = @Composable { BasicText(stringResource(Res.string.state_saved), style = style) }
        if (towardEnd) near() else far()
        Box(contentAlignment = Alignment.Center) {
            ProgressScale(blend(), Modifier.width(MorphSpan).graphicsLayer { if (!towardEnd) scaleX = -1f })
            BasicText(
                stringResource(Res.string.morph_slide),
                Modifier.graphicsLayer { alpha = hint() }.padding(bottom = 22.dp),
                style = MaterialTheme.typography.labelSmall.copy(color = c.legend),
            )
        }
        if (towardEnd) far() else near()
    }
}

/** While morphing where there is no fixed place for the fader: a popup beside the part. */
@Composable
private fun MorphScale(blend: () -> Float, towardEnd: Boolean = false) {
    Popup(popupPositionProvider = remember(towardEnd) { Beside(towardEnd) }) { MorphScalePlate(blend, towardEnd = towardEnd) }
}

/** Beside the anchor on the side the slide goes ([towardEnd]), centred on it; under it when there is no room. */
private class Beside(private val towardEnd: Boolean) : PopupPositionProvider {
    override fun calculatePosition(anchorBounds: IntRect, windowSize: IntSize, layoutDirection: LayoutDirection, popupContentSize: IntSize): IntOffset {
        val gap = 8
        val y = anchorBounds.center.y - popupContentSize.height / 2
        val left = (layoutDirection == LayoutDirection.Ltr) != towardEnd
        val x = if (left) anchorBounds.left - gap - popupContentSize.width else anchorBounds.right + gap
        if (x >= 0 && x + popupContentSize.width <= windowSize.width) return IntOffset(x, y.coerceIn(0, maxOf(0, windowSize.height - popupContentSize.height)))
        val cx = (anchorBounds.center.x - popupContentSize.width / 2).coerceIn(0, maxOf(0, windowSize.width - popupContentSize.width))
        return IntOffset(cx, anchorBounds.bottom + gap)
    }
}

/** Hold to compare as an outlined button, held in `press`: in a header, beside its other actions. */
@Composable
internal fun CompareHold(
    comparing: Boolean,
    compact: Boolean,
    onCompare: (Boolean) -> Unit,
    short: Boolean = false,
    onMorph: ((Float?) -> Unit)? = null,
    blend: () -> Float = { if (comparing) 0f else 1f },
) {
    val c = OscylTheme.colors
    val state = remember { CompareState() }
    // The popup's own layout node lives in this box, so it anchors here and moves nothing around it.
    Box {
        if (state.sliding) MorphScale(blend)
        OutlinedFace(comparing, Modifier.compareHold(comparing, onCompare, onMorph, blend, state)) {
            Image(painterResource(Res.drawable.ic_compare), null, Modifier.size(18.dp), colorFilter = ColorFilter.tint(c.ink))
            if (!compact) {
                val label = stringResource(if (short) Res.string.editor_hold else Res.string.editor_compare)
                BasicText(label, style = MaterialTheme.typography.bodyMedium.copy(color = c.ink, fontWeight = FontWeight.Medium))
            }
        }
    }
}

/**
 * Hold to compare at the rail's foot: one floating 48 dp part in the rail's own make,
 * the compare mark (the saved sound, dotted, half behind the sound now) and three printed
 * ridges at the side it slides toward. Held, it takes the `press` fill and the fader comes
 * out beside it ([state], placed by the caller); nothing around it moves. Off, its mark is
 * `legend` at half strength. Without a [state] (no fixed place for the fader) a popup shows it.
 */
@Composable
internal fun RailCompare(
    comparing: Boolean,
    onCompare: (Boolean) -> Unit,
    onMorph: ((Float?) -> Unit)? = null,
    blend: () -> Float = { if (comparing) 0f else 1f },
    state: CompareState? = null,
    towardEnd: Boolean = false,
    enabled: Boolean = true,
    onShortTap: (() -> Unit)? = null,
) {
    val c = OscylTheme.colors
    val own = remember { CompareState() }
    val s = state ?: own
    Box {
        if (state == null && s.sliding) MorphScale(blend, towardEnd)
        Box(
            Modifier
                .size(48.dp)
                .floating(c)
                .compareHold(comparing, onCompare, onMorph, blend, s, towardEnd, enabled, onShortTap)
                .drawBehind {
                    // The grip: three printed ridges on the side the part slides toward.
                    val ridge = 10.dp.toPx()
                    val pen = 1.dp.toPx()
                    val toStart = !towardEnd
                    val atLeft = toStart == (layoutDirection == LayoutDirection.Ltr)
                    for (k in 0..2) {
                        val off = 5.dp.toPx() + k * 3.dp.toPx()
                        val x = if (atLeft) off else size.width - off
                        drawLine(c.ruleStrong, Offset(x, (size.height - ridge) / 2), Offset(x, (size.height + ridge) / 2), pen)
                    }
                }
                .padding(4.dp)
                .then(if (comparing) Modifier.background(c.press, RoundedCornerShape(OscylTokens.radiusPanel)) else Modifier),
            contentAlignment = Alignment.Center,
        ) {
            Image(
                painterResource(Res.drawable.ic_compare),
                null,
                // Clear of the ridges: a little toward the side they are not on.
                Modifier.padding(start = if (towardEnd) 0.dp else 6.dp, end = if (towardEnd) 6.dp else 0.dp).size(20.dp),
                colorFilter = ColorFilter.tint(if (enabled) c.ink else c.legend.copy(alpha = 0.5f)),
            )
        }
    }
}

@OscylPreviews
@Composable
private fun morphScale(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    androidx.compose.foundation.layout.Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        MorphScalePlate({ 0f }, hint = { 1f })
        MorphScalePlate({ 0.6f })
    }
}

@OscylA11yPreviews
@Composable
private fun morphScale_a11y() = PreviewTheme { MorphScalePlate({ 0.6f }) }
