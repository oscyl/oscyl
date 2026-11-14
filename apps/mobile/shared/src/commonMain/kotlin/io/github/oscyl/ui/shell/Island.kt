package io.github.oscyl.ui.shell

import io.github.oscyl.shared.generated.resources.sheet_job_review
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.foundation.combinedClickable
import androidx.compose.ui.unit.Density
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.ui.platform.LocalDensity
import io.github.oscyl.ui.parts.FloatingRadius
import androidx.compose.runtime.getValue
import androidx.compose.animation.core.animateFloatAsState
import io.github.oscyl.shared.generated.resources.island_sync
import io.github.oscyl.shared.generated.resources.island_save
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.fillMaxHeight
import io.github.oscyl.platform.LocalOscylHaptics
import androidx.compose.ui.platform.testTag
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.VisibilityThreshold
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.layout
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.motionScale
import kotlin.math.roundToInt
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
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
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.boundsInParent
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.text
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.action_save
import io.github.oscyl.shared.generated.resources.action_save_edited
import io.github.oscyl.shared.generated.resources.action_sync
import io.github.oscyl.shared.generated.resources.action_sync_edited
import io.github.oscyl.shared.generated.resources.state_synced
import io.github.oscyl.shared.generated.resources.state_saved
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.floating
import io.github.oscyl.ui.parts.plateSpring
import io.github.oscyl.ui.parts.touchStates
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PHONE
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.parts.ProgressScale
import androidx.compose.ui.unit.sp
import io.github.oscyl.shared.generated.resources.ic_save
import io.github.oscyl.shared.generated.resources.ic_saved
import io.github.oscyl.shared.generated.resources.ic_sync
import androidx.compose.ui.graphics.CompositingStrategy
import io.github.oscyl.shared.generated.resources.island_saved
import io.github.oscyl.shared.generated.resources.island_synced
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.pluralStringResource
import org.jetbrains.compose.resources.stringResource

/** Corners of the plates inside a floating part. */
private val InnerShape = RoundedCornerShape(OscylTokens.radiusPanel)

/**
 * The floating island at the foot of a compact window: the three destinations on a
 * sliding plate, and beside it, 8 dp off its end, the Save key. The island takes
 * the rest of the width; at large text its tabs stand two over two, never ellipsised.
 */
@Composable
fun Island(
    current: Destination,
    onSelect: (Destination) -> Unit,
    saveCount: Int,
    onSave: () -> Unit,
    modifier: Modifier = Modifier,
    sync: Boolean = false,
    /** A hold on the Save key: the review of every change first. */
    onReview: () -> Unit = onSave,
    /** A program list is in sight: the search key stands at the start. */
    search: ProgramSearch? = null,
    /** A save or a sync under way, how far (0..1): the Save key's printed scale. */
    progress: Float? = null,
    /** The "Saved" notice stands: the clean key says so in ink. */
    confirmed: Boolean = false,
) {
    // One height for all: the search and Save keys stand as tall as the island between them.
    ChromeScale {
    BoxWithConstraints(modifier.fillMaxWidth()) {
        // The Save key keeps its word while the tabs have room for theirs beside it; on the
        // narrowest island (a small phone at the largest display size, the search key standing
        // too) the tabs stand as drawings alone, and so does it.
        val words = maxWidth - (if (search != null) 56.dp else 0.dp) >= 236.dp
        Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            if (search != null) SearchKey(search, Modifier.fillMaxHeight())
            // The tabs keep their words while they fit one row, else the drawings alone.
            Box(Modifier.weight(1f).floating(OscylTheme.colors).padding(6.dp)) { DestinationTabs(current, onSelect, vertical = false) }
            SavePlate(saveCount, sync, onSave, Modifier.fillMaxHeight().widthIn(min = 48.dp), onReview = onReview, words = words, progress = progress, confirmed = confirmed)
        }
    }
    }
}

/** How far the chrome's text grows with the system's font size. */
const val ChromeFontScaleMax = 1.3f

/**
 * The chrome's text (the island, the top bars, a picture's axis words) grows with the
 * system's font size up to [ChromeFontScaleMax], as the big apps' bars do: past that the
 * island breaks over two rows, titles cut to a few letters and axis words leave the
 * picture. A screen reader still says every label; the content grows in full.
 */
@Composable
fun ChromeScale(content: @Composable () -> Unit) {
    val density = LocalDensity.current
    if (density.fontScale <= ChromeFontScaleMax) {
        content()
    } else {
        CompositionLocalProvider(LocalDensity provides Density(density.density, ChromeFontScaleMax), content = content)
    }
}

/**
 * The destinations as a single-choice tab group. The current one sits on a sunken
 * `ground` plate with a 1 dp rule (one tone below the raised island, as in the Front
 * Panel sketch), which slides between tabs on the plate spring. The plate is drawn
 * in the draw phase, so its glide never recomposes the tabs.
 */
@Composable
internal fun DestinationTabs(
    current: Destination,
    onSelect: (Destination) -> Unit,
    vertical: Boolean,
    modifier: Modifier = Modifier,
    /** False: the drawings alone (each said by name). In a row they also go when the words do not fit. */
    labels: Boolean = true,
) {
    val c = OscylTheme.colors
    val bounds = remember { mutableStateMapOf<Destination, Rect>() }
    val haptics = LocalOscylHaptics.current
    val plate = remember { Animatable(Rect.Zero, Rect.VectorConverter, Rect.VisibilityThreshold) }
    val target = bounds[current]
    LaunchedEffect(target) {
        when {
            target == null -> Unit
            plate.value.isEmpty -> plate.snapTo(target)
            else -> plate.animateTo(target, plateSpring(Rect.VisibilityThreshold))
        }
    }
    val track = modifier.selectableGroup().drawBehind {
        // Until the first snap lands, draw straight at the current tab: no plateless first frame.
        val r = plate.value.takeUnless { it.isEmpty } ?: bounds[current] ?: return@drawBehind
        val radius = CornerRadius(OscylTokens.radiusPanel.toPx())
        drawRoundRect(c.ground, r.topLeft, r.size, radius)
        val half = OscylTokens.ruleWidth.toPx() / 2
        drawRoundRect(
            c.rule,
            r.topLeft + Offset(half, half),
            Size(r.width - 2 * half, r.height - 2 * half),
            CornerRadius(radius.x - half),
            style = Stroke(OscylTokens.ruleWidth.toPx()),
        )
    }
    // Whether the row keeps its words: decided in the row's own measure (below), before the
    // tabs are measured, and read by each tab's layout. Not state: one measure pass sets and reads it.
    val words = remember { TabWords() }
    val tab = @Composable { destination: Destination, size: Modifier ->
        DestinationTab(
            destination,
            selected = destination == current,
            onClick = {
                // A tick as the plate moves: the switch is felt as well as seen.
                if (destination != current) haptics.tick()
                onSelect(destination)
            },
            modifier = size.testTag("tab-${destination.name}").onPlaced { bounds[destination] = it.boundsInParent() },
            label = labels,
            words = words,
        )
    }
    if (vertical) {
        words.on = labels
        Column(track.width(IntrinsicSize.Max), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Destination.entries.forEach { tab(it, Modifier.fillMaxWidth()) }
        }
    } else {
        Layout({ Destination.entries.forEach { tab(it, Modifier) } }, track) { measurables, constraints ->
            // One row, each tab its width plus an even share of the rest. The words stay
            // while every labelled tab fits; otherwise the drawings alone (each said by name):
            // a label never breaks mid-word and the island never grows a second row.
            words.on = labels
            val labelled = measurables.map { it.maxIntrinsicWidth(Constraints.Infinity) }
            val bounded = constraints.hasBoundedWidth
            words.on = labels && (!bounded || labelled.sum() <= constraints.maxWidth)
            val natural = if (words.on) labelled else measurables.map { it.maxIntrinsicWidth(Constraints.Infinity) }
            val width = if (bounded) maxOf(constraints.maxWidth, natural.sum()) else natural.sum()
            val n = measurables.size
            val spare = width - natural.sum()
            val widths = natural.mapIndexed { i, w -> w + spare / n + if (i < spare % n) 1 else 0 }
            val height = measurables.indices.maxOf { measurables[it].minIntrinsicHeight(widths[it]) }
            // The tabs read the decision as they measure: the two modes ask with different
            // constraints, so a cached measure of the other mode is never reused.
            val placeables = measurables.mapIndexed { i, m ->
                m.measure(if (words.on) Constraints.fixed(widths[i], height) else Constraints(widths[i], widths[i], height, Constraints.Infinity))
            }
            layout(width, height) {
                var x = 0
                placeables.forEach { it.placeRelative(x, 0); x += it.width }
            }
        }
    }
}

/** Whether the island's tabs show their words this measure. */
private class TabWords { var on = true }

@Composable
private fun DestinationTab(destination: Destination, selected: Boolean, onClick: () -> Unit, modifier: Modifier, label: Boolean, words: TabWords) {
    val c = OscylTheme.colors
    val interaction = remember { MutableInteractionSource() }
    val tint = if (selected) c.ink else c.legend
    val word = stringResource(destination.label)
    Layout(
        {
            Image(painterResource(destination.icon), null, Modifier.size(20.dp), colorFilter = ColorFilter.tint(tint))
            if (label) BasicText(
                word,
                style = MaterialTheme.typography.labelSmall.copy(
                    color = tint,
                    fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Medium,
                    textAlign = TextAlign.Center,
                ),
            )
        },
        modifier
            .selectable(selected, interaction, indication = null, role = Role.Tab, onClick = onClick)
            // Said by name whether its word shows or not.
            .clearAndSetSemantics { text = AnnotatedString(word) }
            .touchStates(interaction, InnerShape, c.press)
            .padding(horizontal = 4.dp, vertical = 6.dp),
    ) { measurables, constraints ->
        val shown = if (words.on) measurables else measurables.take(1)
        val gap = 2.dp.roundToPx()
        val loose = constraints.copy(minWidth = 0, minHeight = 0)
        val placeables = shown.map { it.measure(loose) }
        val content = placeables.sumOf { it.height } + gap * (placeables.size - 1)
        val width = maxOf(constraints.minWidth, (if (shown.size > 1) 56.dp else 40.dp).roundToPx() - 8.dp.roundToPx(), placeables.maxOf { it.width })
        val height = maxOf(constraints.minHeight, 52.dp.roundToPx() - 12.dp.roundToPx(), content)
        layout(width, height) {
            var y = (height - content) / 2
            placeables.forEach { it.placeRelative((width - it.width) / 2, y); y += it.height + gap }
        }
    }
}

/**
 * The Save key, a floating part 8 dp off the island's end, as tall as the island, in the
 * island's make and a tab's anatomy: a drawing over its word. With programs to write, the
 * memory card and the verb ("Save", or "Sync" for a synth's slots) in `ink`, the orange
 * count on the drawing's end corner ("99+" above 99; it widens toward the start, over the
 * drawing, never toward the key's edge), announced as "Save 4 edited programs"; with
 * none, the card with a check on it and "Saved" in `legend`, disabled. There is no plate
 * on it: the plate is the selection's. Both faces are laid out, one over the other, and
 * crossfade on `base`, so the key keeps one size whatever it says; a count change pops
 * the count on `fast`.
 *
 * While a save or a sync runs, [progress] (0..1) runs along a printed scale at the key's
 * foot and the face waits, dimmed. [confirmed] (the "Saved" notice stands) shows the
 * clean face in `ink`. Without [words] (the narrowest island) the drawings stand alone, as
 * the tabs' do.
 */
@Composable
fun SavePlate(
    count: Int,
    sync: Boolean,
    onSave: () -> Unit,
    modifier: Modifier = Modifier,
    /** A hold: the review of every change before it is kept. */
    onReview: () -> Unit = onSave,
    words: Boolean = true,
    progress: Float? = null,
    confirmed: Boolean = false,
) {
    val c = OscylTheme.colors
    val motion = LocalOscylMotion.current
    val interaction = remember { MutableInteractionSource() }
    val dirty = count > 0
    val description = if (dirty) {
        pluralStringResource(if (sync) Res.plurals.action_sync_edited else Res.plurals.action_save_edited, count, count)
    } else {
        stringResource(if (sync) Res.string.state_synced else Res.string.state_saved)
    }
    val base = tween<Float>(if (motion) OscylTokens.baseMs else 0, easing = OscylTokens.easing)
    val edited by animateFloatAsState(if (dirty) 1f else 0f, base)
    val waiting by animateFloatAsState(if (progress != null) 1f else 0f, base)
    val sure by animateFloatAsState(if (confirmed) 1f else 0f, base)
    val pop = remember { Animatable(1f) }
    LaunchedEffect(count) {
        if (count == 0 || motionScale(motion) == 0f) return@LaunchedEffect
        pop.snapTo(0f)
        pop.animateTo(1f, tween((OscylTokens.fastMs * motionScale(motion)).roundToInt(), easing = OscylTokens.easing))
    }
    val shape = RoundedCornerShape(FloatingRadius)
    val reviewWord = stringResource(Res.string.sheet_job_review)
    // All four words hold the key's width, so it is one width on a synth's slots and in a
    // library (Sync and Save): the tabs never move between destinations.
    val room = listOf(Res.string.island_save, Res.string.island_sync, Res.string.island_saved, Res.string.island_synced).map { stringResource(it) }
    val haptics = LocalOscylHaptics.current
    Box(
        modifier
            .floating(c, shape)
            .then(if (dirty) Modifier.combinedClickable(interaction, indication = null, role = Role.Button, onLongClick = { haptics.confirm(); onReview() }, onClick = { haptics.tick(); onSave() }) else Modifier)
            .clearAndSetSemantics {
                contentDescription = description
                role = Role.Button
                if (dirty) {
                    onClick { onSave(); true }
                    customActions = listOf(CustomAccessibilityAction(reviewWord) { onReview(); true })
                } else {
                    disabled()
                }
            }
            .touchStates(interaction, shape, c.press),
        contentAlignment = Alignment.Center,
    ) {
        // Something to write: the card, its count, the verb. It waits, dimmed, while it is written.
        SaveFace(
            if (sync) Res.drawable.ic_sync else Res.drawable.ic_save,
            stringResource(if (sync) Res.string.island_sync else Res.string.island_save),
            c.ink, FontWeight.SemiBold, words, room,
            // Faded stroke by stroke: a copy to fade through would cut the count off at the face's edge.
            Modifier.graphicsLayer { alpha = edited * (1f - 0.45f * waiting); compositingStrategy = CompositingStrategy.ModulateAlpha },
        ) {
            SaveCount(
                count.coerceAtLeast(1),
                // On the drawing's end corner, 11 dp past it: it widens toward the start, over the drawing.
                Modifier.align(Alignment.TopEnd).offset(x = 11.dp, y = (-6).dp).graphicsLayer {
                    val t = pop.value
                    scaleX = 0.8f + 0.2f * t
                    scaleY = scaleX
                    alpha = t
                },
            )
        }
        // Nothing to write: the card with its check and the state's word, in ink while the notice says so.
        SaveFace(
            Res.drawable.ic_saved,
            stringResource(if (sync) Res.string.island_synced else Res.string.island_saved),
            androidx.compose.ui.graphics.lerp(c.legend, c.ink, sure), FontWeight.Medium, words, room,
            Modifier.graphicsLayer { alpha = 1f - edited; compositingStrategy = CompositingStrategy.ModulateAlpha },
        )
        // Its place is kept: nothing moves when a save starts or ends.
        // (As wide as the key is, never widening it: the scale fills what it is given.)
        Box(Modifier.matchParentSize().padding(start = 10.dp, end = 10.dp, bottom = 5.dp).graphicsLayer { alpha = waiting }, contentAlignment = Alignment.BottomCenter) {
            ProgressScale(progress)
        }
    }
}

/** One face of the Save key, built as a tab is: the drawing (with what stands [on] it) over its word. */
@Composable
private fun SaveFace(
    drawing: org.jetbrains.compose.resources.DrawableResource,
    word: String,
    tint: androidx.compose.ui.graphics.Color,
    weight: FontWeight,
    words: Boolean,
    /** Every word the key can say: laid out unseen, so the face is as wide as the widest. */
    room: List<String>,
    modifier: Modifier,
    on: @Composable androidx.compose.foundation.layout.BoxScope.() -> Unit = {},
) {
    Column(modifier.padding(horizontal = 8.dp), verticalArrangement = Arrangement.spacedBy(2.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        Box {
            Image(painterResource(drawing), null, Modifier.size(20.dp), colorFilter = ColorFilter.tint(tint))
            on()
        }
        if (words) {
            val style = MaterialTheme.typography.labelSmall.copy(color = tint, fontWeight = FontWeight.SemiBold)
            Box(contentAlignment = Alignment.Center) {
                room.forEach { BasicText(it, Modifier.graphicsLayer { alpha = 0f }.clearAndSetSemantics {}, style = style, maxLines = 1) }
                BasicText(word, style = style.copy(fontWeight = weight), maxLines = 1)
            }
        }
    }
}

/** The count on the Save key's drawing: the rail's own count box (16 dp, 11 sp tabular figures), "99+" above 99. */
@Composable
private fun SaveCount(count: Int, modifier: Modifier) {
    val c = OscylTheme.colors
    Box(
        modifier.heightIn(min = 16.dp).widthIn(min = 16.dp).background(c.act, RoundedCornerShape(4.dp)).padding(horizontal = 3.dp),
        contentAlignment = Alignment.Center,
    ) {
        BasicText(
            // Latin digits, as the slots and values around it (one digit system on a page).
            if (count > 99) "99+" else count.toString(),
            style = MaterialTheme.typography.labelSmall.copy(color = c.onAct, fontSize = 11.sp, lineHeight = 11.sp, fontWeight = FontWeight.Bold, fontFeatureSettings = "tnum", letterSpacing = 0.sp),
            maxLines = 1,
        )
    }
}

@OscylPreviews
@Composable
private fun island_compact(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    Island(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {})
}

@OscylA11yPreviews
@Composable
private fun island() = PreviewTheme {
    Island(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {})
}

/** A count past 99, a save under way, and the notice's "Saved". */
@OscylPreviews
@Composable
private fun island_states(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Island(Destination.Editor, onSelect = {}, saveCount = 140, onSave = {})
        Island(Destination.Editor, onSelect = {}, saveCount = 12, onSave = {}, progress = 0.6f)
        Island(Destination.Editor, onSelect = {}, saveCount = 0, onSave = {}, confirmed = true)
        Island(Destination.Library, onSelect = {}, saveCount = 3, onSave = {}, sync = true)
        Island(Destination.Library, onSelect = {}, saveCount = 0, onSave = {}, sync = true)
    }
}

@Preview(name = "italian", locale = "it", device = PHONE)
@Composable
private fun island_italian() = PreviewTheme {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Island(Destination.Editor, onSelect = {}, saveCount = 140, onSave = {})
        Island(Destination.Editor, onSelect = {}, saveCount = 0, onSave = {})
        Island(Destination.Library, onSelect = {}, saveCount = 2, onSave = {}, sync = true)
        Island(Destination.Library, onSelect = {}, saveCount = 0, onSave = {}, sync = true)
    }
}

@Preview(name = "saved", device = PHONE)
@Composable
private fun island_clean() = PreviewTheme {
    Island(Destination.Library, onSelect = {}, saveCount = 0, onSave = {})
}

/** A program list in sight: the search key at the island's start. */
@OscylPreviews
@Composable
private fun island_search(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    Island(Destination.Library, onSelect = {}, saveCount = 3, onSave = {}, search = remember { ProgramSearch().apply { available = true } })
}

/** The island as the search field, holding words. */
@OscylPreviews
@Composable
private fun island_searching(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    SearchIsland(remember { ProgramSearch().apply { available = true; active = true; query = "str" } })
}
