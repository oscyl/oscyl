package io.github.oscyl.ui.parts

import androidx.compose.ui.graphics.graphicsLayer
import io.github.oscyl.platform.LocalOscylHaptics
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.VisibilityThreshold
import androidx.compose.foundation.border
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.boundsInParent
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.segment_saved
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import kotlin.math.roundToInt
import org.jetbrains.compose.resources.stringResource

/** The orange notch under the saved option: 12 x 3 dp, 3 dp below the frame. */
private val NotchGap = 3.dp
private val NotchHeight = 3.dp

/**
 * A choice of [options] (DESIGN.md "Segments"): one ruled frame, the chosen option on
 * the lifted plate, which slides between options on the plate spring (in the layout
 * phase, so a glide never recomposes). While another option is chosen, the [saved] one
 * carries the orange notch under the frame; its space is always kept, and every other option
 * a printed tick there (the chosen one longer, in ink). Options too long
 * for one row (large text, long names) stack in a column, the notch inside the option.
 */
@Composable
fun Segments(
    options: List<String>,
    selected: Int,
    saved: Int,
    onSelect: (Int) -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    /** A program's value: printed ticks and the saved notch under it. False for a setting with no saved value (no marks, no room kept for them). */
    index: Boolean = true,
    /** False on a part that has its own edge (a floating plate): no frame is drawn, that edge is the only one. */
    framed: Boolean = true,
    /** The chosen option's plate corners: concentric with what holds it (its corner less the gap). */
    innerRadius: Dp = OscylTokens.radiusInner,
    /** Each option's least height (a floating plate's options stand as tall as the island's tabs). */
    optionHeight: Dp = 48.dp,
) {
    val c = OscylTheme.colors
    val motion = LocalOscylMotion.current
    val haptics = LocalOscylHaptics.current
    val inner = RoundedCornerShape(innerRadius)
    val bounds = remember(options.size) { mutableStateMapOf<Int, Rect>() }
    val plate = remember { Animatable(Rect.Zero, Rect.VectorConverter, Rect.VisibilityThreshold) }
    val target = bounds[selected]
    val shape = remember { SegmentsShape() }
    LaunchedEffect(target) {
        if (target == null) return@LaunchedEffect
        // A new size (rotation, a resized window) moves the plate with the layout, at once.
        val resized = shape.width != shape.plateWidth
        shape.plateWidth = shape.width
        if (plate.value.isEmpty || resized || motionScale(motion) == 0f) {
            plate.snapTo(target)
        } else {
            plate.animateTo(target, plateSpring(Rect.VisibilityThreshold))
        }
    }
    val savedChoice = stringResource(Res.string.segment_saved)
    val notchGap = NotchGap
    Box(
        modifier
            // Off: drawn at 60%, so it does not look choosable.
            .graphicsLayer { alpha = if (enabled) 1f else 0.6f }
            .padding(bottom = if (index) NotchGap + NotchHeight else 0.dp)
            .then(if (framed) Modifier.border(OscylTokens.ruleWidth, c.ruleStrong, RoundedCornerShape(OscylTokens.radius)).padding(2.dp) else Modifier),
    ) {
        // The plate, under the options, placed where it has glided to.
        Box(
            Modifier
                .matchParentSize()
                .layout { measurable, constraints ->
                    val r = plate.value.takeUnless { it.isEmpty } ?: bounds[selected] ?: Rect.Zero
                    val p = measurable.measure(Constraints.fixed(r.width.roundToInt(), r.height.roundToInt()))
                    layout(constraints.maxWidth, constraints.maxHeight) { p.place(r.left.roundToInt(), r.top.roundToInt()) }
                },
        ) {
            Box(Modifier.liftedPlate(c, inner).matchParentSize())
        }
        Layout(
            content = {
                options.forEachIndexed { i, option ->
                    val chosen = i == selected
                    val interaction = remember { MutableInteractionSource() }
                    Box(
                        Modifier
                            .heightIn(min = optionHeight)
                            .onPlaced { bounds[i] = it.boundsInParent() }
                            .selectable(chosen, interaction, indication = null, enabled = enabled, role = Role.RadioButton) { if (!chosen) haptics.tick(); onSelect(i) }
                            .then(if (i == saved && !chosen) Modifier.semantics { stateDescription = savedChoice } else Modifier)
                            .touchStates(interaction, inner, c.press)
                            .padding(horizontal = 11.dp, vertical = 4.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        BasicText(
                            option,
                            style = MaterialTheme.typography.bodyMedium.copy(
                                color = if (chosen) c.onPlate else c.legend,
                                fontWeight = if (chosen) FontWeight.SemiBold else FontWeight.Medium,
                                textAlign = TextAlign.Center,
                                textDirection = TextDirection.Content,
                            ),
                        )
                    }
                }
            },
            modifier = Modifier.selectableGroup().drawBehind {
                if (!index) return@drawBehind
                // A printed tick per option: `legend` 1 x 4 dp, the chosen one `ink` 1.5 x 6 dp;
                // the saved option, while another is chosen, carries the notch instead.
                val ltr = layoutDirection == LayoutDirection.Ltr
                bounds.forEach { (i, r) ->
                    if (i >= options.size || (i == saved && i != selected)) return@forEach
                    val chosen = i == selected
                    val w = (if (chosen) 1.5.dp else 1.dp).toPx()
                    val len = (if (chosen) 6.dp else 4.dp).toPx()
                    val ink = if (chosen) c.ink else c.legend
                    if (shape.stacked) {
                        val x0 = if (ltr) r.left else r.right - len
                        // Inside the option: on the plate when chosen, so in the plate's ink.
                        drawRect(if (chosen) c.onPlate else ink, Offset(x0, r.center.y - w / 2), Size(len, w))
                    } else {
                        drawRect(ink, Offset(r.center.x - w / 2, size.height + 2.dp.toPx()), Size(w, len))
                    }
                }
                val at = bounds[saved]
                if (selected == saved || at == null) return@drawBehind
                val long = minOf(12.dp.toPx(), at.width)
                val thick = NotchHeight.toPx()
                val radius = CornerRadius(thick / 2)
                if (shape.stacked) {
                    // Stacked, the notch stands inside the saved option, at its end.
                    val x = if (layoutDirection == LayoutDirection.Ltr) at.right - 8.dp.toPx() - thick else at.left + 8.dp.toPx()
                    drawRoundRect(c.act, Offset(x, at.center.y - long / 2), Size(thick, long), radius)
                } else {
                    val y = size.height + 2.dp.toPx() + notchGap.toPx()
                    drawRoundRect(c.act, Offset(at.center.x - long / 2, y), Size(long, thick), radius)
                }
            },
        ) { measurables, constraints ->
            // One row of equal options while the longest fits its share; then each its own
            // width plus a share of the rest; past that (200% text, long names) one column,
            // so a word never breaks.
            if (measurables.isEmpty()) return@Layout layout(0, 0) {}
            val natural = measurables.map { it.maxIntrinsicWidth(Constraints.Infinity) }
            val width = if (constraints.hasBoundedWidth) constraints.maxWidth else natural.sum()
            val n = measurables.size
            shape.stacked = natural.sum() > width
            shape.width = width
            if (shape.stacked) {
                val placeables = measurables.map { it.measure(Constraints.fixedWidth(width)) }
                layout(width, placeables.sumOf { it.height }) {
                    var y = 0
                    placeables.forEach { it.placeRelative(0, y); y += it.height }
                }
            } else {
                val spare = width - if (natural.max() * n <= width) natural.max() * n else natural.sum()
                val base = natural.map { if (natural.max() * n <= width) natural.max() else it }
                val widths = base.mapIndexed { i, w -> w + spare / n + if (i < spare % n) 1 else 0 }
                val height = measurables.indices.maxOf { measurables[it].minIntrinsicHeight(widths[it]) }
                val placeables = measurables.mapIndexed { i, m -> m.measure(Constraints.fixed(widths[i], height)) }
                layout(width, height) {
                    var x = 0
                    placeables.forEach { it.placeRelative(x, 0); x += it.width }
                }
            }
        }
    }
}

/** How the options were last laid out, for the notch (written in layout, read in draw). */
private class SegmentsShape {
    var stacked = false
    var width = 0
    /** The width the plate last landed at. */
    var plateWidth = -1
}

@Composable
private fun SegmentsSamples() {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Segments(listOf("2-pole", "4-pole"), selected = 1, saved = 0, onSelect = {})
        Segments(listOf("Saw", "Triangle", "Square"), selected = 0, saved = 0, onSelect = {})
        Segments(listOf("Off", "On"), selected = 0, saved = 1, onSelect = {})
        // Stacks at large text, the notch moving inside the saved option.
        Segments(listOf("Sawtooth", "Triangle", "Square", "Noise"), selected = 2, saved = 0, onSelect = {})
        Segments(listOf("Mono"), selected = 0, saved = 0, onSelect = {})
    }
}

/** Twelve options stack into a column on a phone: the ticks move to each option's start. */
@OscylPreviews
@Composable
private fun segments_twelve(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    Segments((1..12).map { "$it" }, selected = 6, saved = 3, onSelect = {})
}

@OscylPreviews
@Composable
private fun segments_notch(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) { SegmentsSamples() }

@OscylA11yPreviews
@Composable
private fun segments_a11y() = PreviewTheme { SegmentsSamples() }
