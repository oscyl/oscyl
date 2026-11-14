package io.github.oscyl.ui.parts

import io.github.oscyl.shared.generated.resources.dial_was_restore
import io.github.oscyl.shared.generated.resources.ic_undo
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.clickable
import io.github.oscyl.shared.generated.resources.dial_no_such_value
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.runtime.setValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.IntrinsicSize
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.theme.OscylThemeMode
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.Image
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Popup
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.window.PopupPositionProvider
import androidx.compose.ui.window.PopupProperties
import io.github.oscyl.platform.LocalOscylHaptics
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.dial_decrease
import io.github.oscyl.shared.generated.resources.dial_increase
import io.github.oscyl.shared.generated.resources.ic_add
import io.github.oscyl.shared.generated.resources.ic_minus
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import org.jetbrains.compose.resources.DrawableResource
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.stringResource

/** Held, a step repeats from this rate (steps a second) ... */
private const val RepeatFrom = 8f
/** ... up to this one ... */
private const val RepeatTo = 40f
/** ... reached this long (seconds) after the repeat starts. */
private const val RepeatRamp = 1.5f

/**
 * A knob's exact value (DESIGN.md "Dial"): a floating plate under the value with minus,
 * the value, plus. A press steps by one; held, it repeats after the long-press timeout,
 * speeding from 8 to 40 steps a second over 1.5 s on the frame clock, and stops at the
 * ends (where that button is disabled). [onDone] when the finger lifts: one undo step per
 * press or hold. Back or a tap outside closes it ([onDismiss]). [typed] (Plan B) makes the
 * value a field to type into; null shows it as text.
 */
@Composable
fun ValueStepper(
    value: Int,
    range: IntRange,
    text: String,
    label: String,
    onChange: (Int) -> Unit,
    onDone: () -> Unit,
    onDismiss: () -> Unit,
    typed: ((String) -> Int?)? = null,
    /** The saved value as written, while the knob is edited: a row under the steps restores it. */
    savedText: String? = null,
    onRestore: () -> Unit = {},
    /** How far it has come out (0..1): its caller keeps it up while it leaves. */
    shown: () -> Float = { 1f },
) {
    Popup(popupPositionProvider = Below, onDismissRequest = onDismiss, properties = PopupProperties(focusable = true)) {
        // Grown from under the value it sets.
        Box(Modifier.transientLayer(shown, origin = TransformOrigin(0.5f, 0f))) {
            StepperPlate(value, range, text, label, onChange, onDone, typed, savedText = savedText, onRestore = onRestore)
        }
    }
}

/** The stepper's plate: minus, the value, plus. */
@Composable
private fun StepperPlate(
    value: Int,
    range: IntRange,
    text: String,
    label: String,
    onChange: (Int) -> Unit,
    onDone: () -> Unit,
    typed: ((String) -> Int?)? = null,
    startError: Boolean = false,
    savedText: String? = null,
    onRestore: () -> Unit = {},
) {
    val c = OscylTheme.colors
    // What is typed; it starts again from the value whenever the value is written anew.
    var draft by remember(text) { mutableStateOf(TextFieldValue(text, TextRange(0, text.length))) }
    var error by remember(text) { mutableStateOf(startError) }
    Column(Modifier.floating(c).padding(4.dp).testTag("stepper"), horizontalAlignment = Alignment.CenterHorizontally) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            StepButton(Res.drawable.ic_minus, stringResource(Res.string.dial_decrease), -1, value, range, onChange, onDone, "stepper-minus")
            val style = MaterialTheme.typography.titleMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold, fontFeatureSettings = "tnum", textAlign = TextAlign.Center)
            val valueModifier = Modifier
                .widthIn(min = 64.dp, max = 160.dp)
                .padding(horizontal = 4.dp)
                .semantics {
                    contentDescription = label
                    stateDescription = text
                }
                .testTag("stepper-value")
            if (typed == null) {
                BasicText(text, valueModifier, style = style, maxLines = 1)
            } else {
                val submit = {
                    val v = typed(draft.text)
                    if (v == null) {
                        error = true
                    } else {
                        error = false
                        onChange(v.coerceIn(range))
                        onDone()
                    }
                }
                BasicTextField(
                    draft,
                    { draft = it; error = false },
                    valueModifier.width(IntrinsicSize.Min),
                    textStyle = style,
                    singleLine = true,
                    cursorBrush = SolidColor(c.ink),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal, imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { submit() }),
                )
            }
            StepButton(Res.drawable.ic_add, stringResource(Res.string.dial_increase), 1, value, range, onChange, onDone, "stepper-plus")
        }
        if (savedText != null) {
            // "↶ Back to 431": the knob's restore, where its exact value is.
            val interaction = remember { MutableInteractionSource() }
            Row(
                Modifier
                    .heightIn(min = 48.dp)
                    .clickable(interaction, indication = null, role = Role.Button, onClick = onRestore)
                    .touchStates(interaction, RoundedCornerShape(OscylTokens.radius), c.press)
                    .padding(horizontal = 12.dp)
                    .testTag("stepper-restore"),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Image(painterResource(Res.drawable.ic_undo), null, Modifier.size(16.dp), colorFilter = ColorFilter.tint(c.act))
                BasicText(
                    stringResource(Res.string.dial_was_restore, savedText),
                    style = MaterialTheme.typography.bodyMedium.copy(color = c.act, fontWeight = FontWeight.Medium, fontFeatureSettings = "tnum"),
                )
            }
        }
        if (error) {
            BasicText(
                stringResource(Res.string.dial_no_such_value),
                Modifier.padding(bottom = 4.dp).semantics { liveRegion = LiveRegionMode.Polite },
                style = MaterialTheme.typography.bodySmall.copy(color = c.legend),
            )
        }
    }
}

/** Under its anchor, centred, kept inside the window. */
private object Below : PopupPositionProvider {
    override fun calculatePosition(anchorBounds: IntRect, windowSize: IntSize, layoutDirection: LayoutDirection, popupContentSize: IntSize): IntOffset {
        val x = (anchorBounds.center.x - popupContentSize.width / 2).coerceIn(0, maxOf(0, windowSize.width - popupContentSize.width))
        val below = anchorBounds.bottom
        val y = if (below + popupContentSize.height <= windowSize.height) below else (anchorBounds.top - popupContentSize.height).coerceAtLeast(0)
        return IntOffset(x, y)
    }
}

/** Minus or plus: a press steps once, a hold repeats; disabled at its end. */
@Composable
private fun StepButton(
    icon: DrawableResource,
    label: String,
    sign: Int,
    value: Int,
    range: IntRange,
    onChange: (Int) -> Unit,
    onDone: () -> Unit,
    tag: String,
) {
    val c = OscylTheme.colors
    val haptics = LocalOscylHaptics.current
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    val enabled = if (sign > 0) value < range.last else value > range.first
    val enabledNow by rememberUpdatedState(enabled)
    val now by rememberUpdatedState(value)
    val change by rememberUpdatedState(onChange)
    val done by rememberUpdatedState(onDone)
    val step = { from: Int -> (from + sign).coerceIn(range) }
    Box(
        Modifier
            .size(48.dp)
            .testTag(tag)
            .semantics {
                contentDescription = label
                role = Role.Button
                if (!enabled) disabled()
                onClick {
                    if (!enabled) return@onClick false
                    change(step(now))
                    done()
                    true
                }
            }
            // Keyed on the range only: reaching an end mid-hold must not cut the gesture short.
            .pointerInput(range) {
                coroutineScope {
                    awaitEachGesture {
                        val down = awaitFirstDown()
                        if (!enabledNow) return@awaitEachGesture
                        down.consume()
                        val press = PressInteraction.Press(down.position)
                        interaction.tryEmit(press)
                        var v = step(now)
                        change(v)
                        haptics.tick()
                        val hold = viewConfiguration.longPressTimeoutMillis
                        // The repeat runs on the frame clock while the finger stays down.
                        val repeat = launch {
                            var start = -1L
                            var last = 0L
                            var owed = 0f
                            while (true) {
                                withFrameNanos { t ->
                                    if (start < 0) start = t
                                    val ms = (t - start) / 1_000_000
                                    if (ms >= hold) {
                                        val ramp = ((ms - hold) / 1000f / RepeatRamp).coerceIn(0f, 1f)
                                        val rate = RepeatFrom + (RepeatTo - RepeatFrom) * ramp
                                        owed += rate * (if (last == 0L) 0f else (t - last) / 1e9f)
                                        last = t
                                        while (owed >= 1f) {
                                            owed -= 1f
                                            val next = step(v)
                                            if (next != v) {
                                                v = next
                                                change(v)
                                                if (v == range.first || v == range.last) haptics.bump() else haptics.tick()
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        try {
                            waitForUpOrCancellation()
                        } finally {
                            repeat.cancel()
                            interaction.tryEmit(PressInteraction.Release(press))
                            done()
                        }
                    }
                }
            }
            .padding(4.dp)
            .touchStates(interaction, shape, c.press),
        contentAlignment = Alignment.Center,
    ) {
        Image(painterResource(icon), null, Modifier.size(20.dp), colorFilter = ColorFilter.tint(if (enabled) c.ink else c.legend.copy(alpha = 0.5f)))
    }
}

@OscylPreviews
@Composable
private fun valueStepper(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        StepperPlate(618, 0..1023, "618", "Cutoff", {}, {})
        StepperPlate(1023, 0..1023, "1023", "Cutoff", {}, {})
    }
}

@OscylA11yPreviews
@Composable
private fun valueStepper_a11y() = PreviewTheme { StepperPlate(0, 0..1023, "-12 c", "Pitch", {}, {}) }

/** A typed value the field cannot be: the plate says so under it. */
@OscylPreviews
@Composable
private fun valueStepper_typed(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    StepperPlate(618, 0..1023, "618", "Cutoff", {}, {}, typed = { null }, startError = true)
}
