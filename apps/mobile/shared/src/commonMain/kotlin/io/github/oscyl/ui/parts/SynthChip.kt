package io.github.oscyl.ui.parts

import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.Canvas
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.rememberInfiniteTransition
import io.github.oscyl.shared.generated.resources.sheet_add
import io.github.oscyl.platform.LocalOscylHaptics
import androidx.compose.foundation.combinedClickable
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.graphics.graphicsLayer
import kotlinx.coroutines.launch
import kotlin.math.roundToInt
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.size
import io.github.oscyl.core.PanelDrawing
import io.github.oscyl.ui.synth.PanelThumb
import io.github.oscyl.ui.synth.plate
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.chip_description
import io.github.oscyl.shared.generated.resources.chip_description_more
import io.github.oscyl.shared.generated.resources.chip_description_none
import io.github.oscyl.shared.generated.resources.chip_more
import io.github.oscyl.shared.generated.resources.chip_no_synth
import io.github.oscyl.shared.generated.resources.editor_live
import io.github.oscyl.shared.generated.resources.synth_reading_percent
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import org.jetbrains.compose.resources.stringResource

/**
 * What the synth chip shows: the first connected synth's [model] (null: none), how
 * many [more] are connected, whether its edits play [live], and how far a read has
 * come ([reading], 0..1, null when none is reading).
 */
@Immutable
data class SynthStatus(
    val model: String? = null,
    val more: Int = 0,
    val live: Boolean = false,
    val reading: Float? = null,
    /** Changes sent to the synth so far: the lamp blinks on each while Live. */
    val sends: () -> Int = { 0 },
    /** The first connected synth's nameplate drawing (its driver's), for the full chip. */
    val drawing: PanelDrawing? = null,
)

/** A model's short name for the chip: its last word when that is at most four letters ("xd", "Rev2"), else the whole name. */
fun shortModel(model: String): String {
    val last = model.trim().substringAfterLast(' ')
    return if (last.length in 1..4 && last != model.trim()) last else model.trim()
}

/** The synths as the app sees them, provided around the app; previews show none. */
val LocalSynthStatus = compositionLocalOf { SynthStatus() }

/**
 * The synth chip, top right on every destination: the lamp (lit while a synth is
 * connected), the synth's model, "+1" per other synth, and "Live" while edits play;
 * then each change sent blinks the lamp (25% to 100% over 240 ms, never restarted
 * mid-way), so a stream flickers like a MIDI activity light.
 * A read in progress draws a slim line along its foot. It opens the synths ([onClick]).
 * [compact] (and at large text), it is the synth drawn small, its lamp at the panel's
 * corner (no lamp and a `legend` outline with no synth), the count of others beside it
 * and a read's line under it; its words stay spoken.
 */
@Composable
fun SynthChip(onClick: () -> Unit, modifier: Modifier = Modifier, compact: Boolean = false, onLongClick: (() -> Unit)? = null) {
    val c = OscylTheme.colors
    val status = LocalSynthStatus.current
    // Large text: the lamp alone, so the bar's title keeps its room.
    val lampOnly = compact || androidx.compose.ui.platform.LocalDensity.current.fontScale > 1.3f
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    val label = MaterialTheme.typography.bodyMedium
    val model = status.model
    val description = when {
        model == null -> stringResource(Res.string.chip_description_none)
        status.more > 0 -> stringResource(Res.string.chip_description, stringResource(Res.string.chip_description_more, model, status.more))
        else -> stringResource(Res.string.chip_description, model)
    }
    val live = stringResource(Res.string.editor_live)
    val blink = remember { Animatable(1f) }
    val haptics = LocalOscylHaptics.current
    val addLabel = stringResource(Res.string.sheet_add)
    // While a read runs (and motion is on), the lamp breathes between full and 45%.
    val breathing = status.reading != null && LocalOscylMotion.current
    val breath = if (breathing) {
        rememberInfiniteTransition(label = "reading").animateFloat(
            1f, 0.45f, infiniteRepeatable(tween(OscylTokens.slowMs * 4, easing = OscylTokens.easing), RepeatMode.Reverse), label = "breath",
        )
    } else {
        remember { mutableFloatStateOf(1f) }
    }
    val motion by rememberUpdatedState(LocalOscylMotion.current)
    val sends by rememberUpdatedState(status.sends)
    val playing by rememberUpdatedState(status.live)
    LaunchedEffect(Unit) {
        snapshotFlow { sends() }.collect {
            if (!playing || blink.isRunning || it == 0) return@collect
            launch {
                val scale = motionScale(motion)
                if (scale == 0f) return@launch
                blink.snapTo(0.25f)
                blink.animateTo(1f, tween((240 * scale).roundToInt().coerceAtLeast(1), easing = OscylTokens.easing))
            }
        }
    }
    val reading = status.reading?.let { stringResource(Res.string.synth_reading_percent, (it * 100).toInt()) }
    val state = listOfNotNull(live.takeIf { status.live }, reading).joinToString().ifEmpty { null }
    Box(
        modifier
            .defaultMinSize(minWidth = 48.dp, minHeight = 48.dp)
            // A long press adds a synth straight away (the sheet's Add a synth is the same, one step on).
            .combinedClickable(
                interactionSource = interaction,
                indication = null,
                role = Role.Button,
                onLongClickLabel = if (onLongClick != null) addLabel else null,
                onLongClick = onLongClick?.let { add -> { haptics.confirm(); add() } },
                onClick = { haptics.tick(); onClick() },
            )
            .semantics {
                contentDescription = description
                role = Role.Button
                if (state != null) stateDescription = state
            },
        contentAlignment = Alignment.Center,
    ) {
        if (lampOnly) {
            // Compact: the synth itself, drawn small, its lamp where a panel carries one.
            Row(
                Modifier
                    .clearAndSetSemantics {}
                    .heightIn(min = 36.dp)
                    .touchStates(interaction, shape, c.press)
                    .padding(horizontal = 6.dp),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(Modifier.size(24.dp)) {
                    Canvas(Modifier.matchParentSize()) {
                        val u = 1.dp.toPx()
                        val pen = if (model != null) c.ink else c.legend
                        val line = Stroke(1.5f * u, cap = StrokeCap.Round, join = StrokeJoin.Round)
                        scale(if (layoutDirection == LayoutDirection.Rtl) -1f else 1f, 1f) {
                            // The body: whole with no synth; connected, its top end corner is left open for the lamp.
                            val body = Path().apply {
                                if (model == null) {
                                    addRoundRect(RoundRect(1.5f * u, 6.5f * u, 22.5f * u, 19.5f * u, CornerRadius(2f * u)))
                                } else {
                                    moveTo(14.5f * u, 6.5f * u)
                                    lineTo(3.5f * u, 6.5f * u)
                                    quadraticTo(1.5f * u, 6.5f * u, 1.5f * u, 8.5f * u)
                                    lineTo(1.5f * u, 17.5f * u)
                                    quadraticTo(1.5f * u, 19.5f * u, 3.5f * u, 19.5f * u)
                                    lineTo(20.5f * u, 19.5f * u)
                                    quadraticTo(22.5f * u, 19.5f * u, 22.5f * u, 17.5f * u)
                                    lineTo(22.5f * u, 12.5f * u)
                                }
                            }
                            drawPath(body, pen, style = line)
                            // Its keys.
                            drawLine(pen, Offset(1.5f * u, 13f * u), Offset(if (model == null) 22.5f * u else 22.5f * u, 13f * u), 1.5f * u, StrokeCap.Round)
                            for (x in listOf(6.75f, 12f, 17.25f)) drawLine(pen, Offset(x * u, 13f * u), Offset(x * u, 19.5f * u), 1.5f * u, StrokeCap.Round)
                            // A read: the line under it runs out to how far it has come.
                            status.reading?.let { f ->
                                drawLine(c.rule, Offset(1.5f * u, 23f * u), Offset(22.5f * u, 23f * u), 2f * u, StrokeCap.Round)
                                drawLine(c.ink, Offset(1.5f * u, 23f * u), Offset((1.5f + 21f * f.coerceIn(0f, 1f)) * u, 23f * u), 2f * u, StrokeCap.Round)
                            }
                        }
                    }
                    if (model != null) Lamp(lit = true, Modifier.align(Alignment.TopEnd).offset(y = 3.dp).graphicsLayer { alpha = blink.value * breath.value })
                }
                if (status.more > 0) {
                    BasicText(stringResource(Res.string.chip_more, status.more), style = label.copy(color = c.legend, fontFeatureSettings = "tnum"))
                }
            }
        } else {
            Row(
                Modifier
                    .clearAndSetSemantics {}
                    .heightIn(min = 34.dp)
                    .widthIn(max = 180.dp)
                    .border(OscylTokens.ruleWidth, c.ruleStrong, shape)
                    .touchStates(interaction, shape, c.press)
                    .drawBehind {
                        val f = status.reading ?: return@drawBehind
                        val h = 2.dp.toPx()
                        val inset = 10.dp.toPx()
                        val w = size.width - 2 * inset
                        val y = size.height - h - 3.dp.toPx()
                        val rtl = layoutDirection == LayoutDirection.Rtl
                        val run = w * f.coerceIn(0f, 1f)
                        drawRoundRect(c.rule, Offset(inset, y), Size(w, h), CornerRadius(h / 2))
                        drawRoundRect(c.ink, Offset(if (rtl) inset + w - run else inset, y), Size(run, h), CornerRadius(h / 2))
                    }
                    .padding(start = if (model != null) 11.dp else 12.dp, end = 12.dp, top = 5.dp, bottom = 5.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                // No synth: a calm word, no lamp (an unlit ring beside it read as a fault).
                // A read in progress: the lamp breathes on the slow curve, above its progress line.
                // The synth's nameplate in miniature, then its lamp and short name.
                val drawing = status.drawing
                if (model != null && drawing != null) PanelThumb(drawing, Modifier.size(34.dp, 20.dp), pad = 3.dp, minKnob = 1.5.dp)
                if (model != null) Lamp(lit = true, Modifier.graphicsLayer { alpha = blink.value * breath.value })
                BasicText(
                    model?.let { if (drawing != null) shortModel(it) else it } ?: stringResource(Res.string.chip_no_synth),
                    Modifier.weight(1f, fill = false),
                    style = label.copy(color = if (model != null) c.ink else c.legend, fontWeight = FontWeight.Medium, textDirection = TextDirection.Content),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                if (status.more > 0) {
                    BasicText(stringResource(Res.string.chip_more, status.more), style = label.copy(color = c.legend, fontFeatureSettings = "tnum"))
                }
                if (status.live) {
                    BasicText(live, style = MaterialTheme.typography.labelMedium.copy(color = c.legend))
                }
            }
        }
    }
}

@OscylA11yPreviews
@Composable
private fun synthChip_a11y() = synthChip(OscylThemeMode.Dark)

@OscylPreviews
@Composable
private fun synthChip(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    // Every state, full beside compact: none, connected, live, reading, several, a long name.
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        listOf(
            SynthStatus(),
            SynthStatus("minilogue xd", drawing = plate(6)),
            SynthStatus("minilogue xd", live = true, drawing = plate(6)),
            SynthStatus("Prophet Rev2", reading = 0.4f, drawing = plate(5)),
            SynthStatus("Prophet Rev2", more = 2, live = true, reading = 0.8f, drawing = plate(5)),
            SynthStatus("A very long model name indeed", more = 1),
        ).forEach { status ->
            androidx.compose.runtime.CompositionLocalProvider(LocalSynthStatus provides status) {
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                    SynthChip({}, compact = true)
                    SynthChip({})
                }
            }
        }
    }
}
