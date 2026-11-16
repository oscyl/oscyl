package io.github.oscyl.ui.parts

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.ClipOp
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.graphics.drawscope.inset
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.input.pointer.changedToUpIgnoreConsumed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.util.lerp
import io.github.oscyl.platform.LocalOscylHaptics
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.ic_stage_effects
import io.github.oscyl.shared.generated.resources.ic_stage_envelope
import io.github.oscyl.shared.generated.resources.ic_stage_filter
import io.github.oscyl.shared.generated.resources.ic_stage_lfo
import io.github.oscyl.shared.generated.resources.ic_stage_mixer
import io.github.oscyl.shared.generated.resources.ic_stage_oscillator
import io.github.oscyl.shared.generated.resources.ic_stage_voice
import io.github.oscyl.shared.generated.resources.stage_description
import io.github.oscyl.shared.generated.resources.stage_description_edited
import io.github.oscyl.theme.OscylColors
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import kotlin.math.abs
import kotlin.math.floor
import kotlin.math.roundToInt
import org.jetbrains.compose.resources.DrawableResource
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.pluralStringResource
import org.jetbrains.compose.resources.stringResource

/** One stage on the rail: its name, one-line summary, edit count and icon (from the Linux app's symbolic set). */
data class StageItem(val title: String, val summary: String, val edits: Int, val icon: DrawableResource)

/** The stage icon for a driver page's symbolic icon id; a page without one gets the modulation wave. */
fun stageIcon(id: String?): DrawableResource = when (id) {
    "oscyl-voice-symbolic" -> Res.drawable.ic_stage_voice
    "oscyl-oscillator-symbolic" -> Res.drawable.ic_stage_oscillator
    "oscyl-mixer-symbolic" -> Res.drawable.ic_stage_mixer
    "oscyl-filter-symbolic" -> Res.drawable.ic_stage_filter
    "oscyl-envelope-symbolic" -> Res.drawable.ic_stage_envelope
    "oscyl-effects-symbolic" -> Res.drawable.ic_stage_effects
    else -> Res.drawable.ic_stage_lfo
}

// At rest each stage is a 48 dp square target (the owner's 48 dp bar over the sketch's 44).
private val RestWidth = 48.dp
/** The rail's width at rest: what a page beside it keeps clear. */
internal val StageRailRestWidth get() = RestWidth
private val RestRow = 48.dp
/** A row of the rail that stands open for good (a wide window). */
private val RoomyRow = 56.dp
private val GrownMaxWidth = 300.dp
/** Above the first row and below the last. */
private val Pad = 4.dp
/** The plate floats this far inside its row, across and along. */
private val PlateInsetX = 4.dp
private val PlateInsetY = 2.dp
/** Between the rail and the flag that names a touched stage. */
private val FlagGap = 8.dp

/**
 * The stage rail, an index of the sound's stages (owner, 2026-11-14): a 48 dp floating
 * rail of the [stages]' drawings, the lifted plate on [current] and an orange count box
 * on each edited stage.
 *
 * It answers the moment a finger lands: a tick, the row dims, the plate comes to it and
 * a flag beside the rail names the stage (its name and summary). Lifted there, it is a
 * tap: [onJump], then [onTap]; the flag stays while the page is [landing]. Moved along
 * the rail instead, it is a glide, with nothing to wait for: the rail opens toward the
 * panel into the labelled list, the plate rides the finger, each stage passed ticks and
 * calls [onScrub]; release calls [onJump] and the rail closes on the plate spring.
 *
 * [open] keeps it open at rest (the first look at the editor; a wide window, whose rows
 * are [roomy]). While open and asked to [hint], the plate nods once to the next stage
 * and back: the glide, shown by the part itself.
 *
 * Place it at the end edge over the content: it opens from there toward the start
 * ([atStart]: at the start edge, the stage view's strip, opening toward the end). Its
 * own size never animates: it is the rest column (or, [open], the whole list) and what
 * opens is drawn, clipped and translated in the draw phase. A phone's rows keep 48 dp
 * open or closed, so no stage moves under a finger.
 */
@Composable
fun StageRail(
    stages: List<StageItem>,
    current: Int,
    onJump: (Int) -> Unit,
    onScrub: (Int) -> Unit,
    modifier: Modifier = Modifier,
    open: Boolean = false,
    roomy: Boolean = false,
    landing: Boolean = false,
    hint: Boolean = false,
    onTap: () -> Unit = {},
    atStart: Boolean = false,
) = StageRail(stages, current, onJump, onScrub, modifier, heldAt = null, pressedAt = null, open, roomy, landing, hint, onTap, atStart)

/** [heldAt] starts it gliding on that stage, open; [pressedAt] starts it pressed there, its flag out (previews). */
@Composable
internal fun StageRail(
    stages: List<StageItem>,
    current: Int,
    onJump: (Int) -> Unit,
    onScrub: (Int) -> Unit,
    modifier: Modifier,
    heldAt: Int?,
    pressedAt: Int? = null,
    open: Boolean = false,
    roomy: Boolean = false,
    landing: Boolean = false,
    hint: Boolean = false,
    onTap: () -> Unit = {},
    atStart: Boolean = false,
) = io.github.oscyl.ui.shell.ChromeScale {
    // Chrome: its names and counts grow with the system font to 130%, so the counts stay in the 48 dp rail.
    val c = OscylTheme.colors
    val haptics by rememberUpdatedState(LocalOscylHaptics.current)
    val motion = LocalOscylMotion.current
    val currentNow by rememberUpdatedState(current)
    // The stage under a gliding finger, and the one under a finger that has only landed.
    var scrub by remember { mutableStateOf(heldAt) }
    var tap by remember { mutableStateOf(pressedAt) }
    val gliding = scrub != null
    val shown = scrub ?: tap ?: current
    val grow = remember { Animatable(if (heldAt != null || open) 1f else 0f) }
    val plate = remember { Animatable(shown.toFloat()) }
    // Where the plate rides while gliding, in rows: the finger's own place, no spring between them.
    val ride = remember { mutableFloatStateOf(Float.NaN) }
    val press = remember { Animatable(if (pressedAt != null) 1f else 0f) }
    var pressRow by remember { mutableIntStateOf(pressedAt ?: -1) }
    // The flag: the stage it names, and the tap whose page is still on its way.
    var flagRow by remember { mutableIntStateOf((pressedAt ?: current).coerceIn(0, (stages.size - 1).coerceAtLeast(0))) }
    var tapped by remember { mutableStateOf(false) }
    val flagOut = (tap != null || (tapped && landing)) && !open && !gliding
    val flag = remember { Animatable(if (flagOut) 1f else 0f) }
    val nodded = remember { booleanArrayOf(false) }

    LaunchedEffect(gliding, open) {
        val to = if (gliding || open) 1f else 0f
        if (motionScale(motion) == 0f) grow.snapTo(to) else grow.animateTo(to, plateSpring(0.001f))
    }
    LaunchedEffect(shown, gliding, hint && open) {
        // Gliding, the plate is the finger's (ride): nothing to animate.
        if (gliding) return@LaunchedEffect
        val scale = motionScale(motion)
        // Let go: it settles from where the finger left it.
        ride.floatValue.let { if (!it.isNaN()) { plate.snapTo(it); ride.floatValue = Float.NaN } }
        if (scale == 0f) return@LaunchedEffect plate.snapTo(shown.toFloat())
        plate.animateTo(shown.toFloat(), plateSpring(0.001f))
        // The glide, shown once by the part itself: a nod to the next stage and home.
        if (hint && open && !nodded[0] && stages.size > 1) {
            nodded[0] = true
            plate.animateTo((if (shown < stages.lastIndex) shown + 1 else shown - 1).toFloat(), plateSpring(0.001f))
            plate.animateTo(shown.toFloat(), plateSpring(0.001f))
        }
    }
    LaunchedEffect(tap != null) {
        // There the moment the finger lands; it lets go over `base`.
        if (tap != null || motionScale(motion) == 0f) press.snapTo(if (tap != null) 1f else 0f)
        else press.animateTo(0f, tween((OscylTokens.baseMs * motionScale(motion)).roundToInt(), easing = OscylTokens.easing))
    }
    LaunchedEffect(landing) { if (!landing) tapped = false }
    LaunchedEffect(flagOut) {
        val scale = motionScale(motion)
        val to = if (flagOut) 1f else 0f
        if (scale == 0f) flag.snapTo(to)
        else flag.animateTo(to, tween(((if (flagOut) OscylTokens.fastMs else OscylTokens.baseMs) * scale).roundToInt(), easing = OscylTokens.easing))
    }
    val jump = rememberUpdatedState(onJump)
    val scrubTo = rememberUpdatedState(onScrub)
    val tapDone = rememberUpdatedState(onTap)
    val rows = remember { RowGeometry() }
    val grown = { grow.value }
    val at = { ride.floatValue.let { if (it.isNaN()) plate.value else it } }

    Layout(
        content = {
            // The floating body, the dimmed row and the plate: drawn, never measured again.
            Box(Modifier.railBody(c, rows, atStart, grown, at, { press.value }, { pressRow }))
            // Every row twice: in the rail's inks, and in the plate's own where the plate
            // stands over it. The plate rides a finger through them without a composition.
            for (onPlate in listOf(false, true)) {
                stages.forEachIndexed { i, stage ->
                    StageRow(stage, onPlate, roomy, atStart, grown, Modifier.clearAndSetSemantics {}.railRow(rows, i, onPlate, atStart, grown, at))
                }
            }
            StageFlag(stages.getOrNull(flagRow), atStart, { flag.value })
            // Each tab as screen readers see it: the row's slot as wide as the rail is now.
            stages.forEachIndexed { i, stage ->
                val description = if (stage.edits > 0) {
                    pluralStringResource(Res.plurals.stage_description_edited, stage.edits, stage.title, stage.summary, stage.edits)
                } else {
                    stringResource(Res.string.stage_description, stage.title, stage.summary)
                }
                Box(
                    Modifier.clearAndSetSemantics {
                        role = Role.Tab
                        selected = i == current
                        contentDescription = description
                        onClick { jump.value(i); true }
                    },
                )
            }
        },
        modifier = modifier
            .testTag("stage-rail")
            .selectableGroup()
            .pointerInput(stages.size) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    var at = rows.at(down.position.y)
                    // The answer to the touch itself, before anyone knows what it will be.
                    tap = at
                    pressRow = at
                    flagRow = at
                    tapped = false
                    haptics.tick()
                    var glide = false
                    var landed = false
                    try {
                        while (true) {
                            // Taken by another gesture or gone: back to rest without landing.
                            val change = awaitPointerEvent().changes.firstOrNull { it.id == down.id } ?: break
                            if (change.isConsumed) break
                            if (change.changedToUpIgnoreConsumed()) {
                                change.consume()
                                landed = true
                                break
                            }
                            val y = change.position.y
                            if (!glide && abs(y - down.position.y) > viewConfiguration.touchSlop) {
                                // Moved along the rail: a glide, from this event on.
                                glide = true
                                ride.floatValue = rows.fractionAt(y)
                                scrub = at
                                tap = null
                                if (at != currentNow) scrubTo.value(at)
                            }
                            if (!glide) continue
                            change.consume()
                            ride.floatValue = rows.fractionAt(y)
                            val next = rows.at(y)
                            if (next != at) {
                                at = next
                                scrub = next
                                haptics.tick()
                                scrubTo.value(next)
                            }
                        }
                    } finally {
                        if (landed) {
                            jump.value(at)
                            if (!glide) {
                                tapped = true
                                tapDone.value()
                            }
                        }
                        scrub = null
                        tap = null
                    }
                }
            },
    ) { measurables, constraints ->
        val n = stages.size
        val pad = Pad.roundToPx()
        val restWidth = RestWidth.roundToPx()
        val restRow = RestRow.roundToPx()
        val bodyM = measurables.first()
        val inkMs = measurables.subList(1, 1 + n)
        val plateMs = measurables.subList(1 + n, 1 + 2 * n)
        val flagM = measurables[1 + 2 * n]
        val targetMs = measurables.subList(2 + 2 * n, measurables.size)
        // The open width: the widest row, within the window and a hand's width.
        val maxWidth = minOf(constraints.maxWidth, GrownMaxWidth.roundToPx())
        val grownWidth = (inkMs.maxOfOrNull { it.maxIntrinsicWidth(Constraints.Infinity) } ?: 0)
            .coerceIn(restWidth, maxOf(restWidth, maxWidth))
        // A phone's rows are 48 dp, open or closed: nothing moves under a finger. Roomy
        // rows take their words' height, sharing the window's when they would overflow it.
        val rowMax = if (constraints.hasBoundedHeight && n > 0) maxOf(restRow, (constraints.maxHeight - 2 * pad) / n) else Constraints.Infinity
        rows.update(inkMs.map { if (roomy) it.minIntrinsicHeight(grownWidth).coerceIn(restRow, maxOf(restRow, rowMax)) else restRow }, pad, restWidth)
        val height = (rows.tops.lastOrNull()?.let { it + rows.heights.last() } ?: pad) + pad
        // Its own size is one of two, never between: what opens is drawn.
        val width = if (open) grownWidth else restWidth
        val body = bodyM.measure(Constraints.fixed(grownWidth, height))
        val ink = inkMs.mapIndexed { i, m -> m.measure(Constraints.fixed(grownWidth, rows.heights[i])) }
        val onPlate = plateMs.mapIndexed { i, m -> m.measure(Constraints.fixed(grownWidth, rows.heights[i])) }
        val flagP = flagM.measure(Constraints(maxWidth = maxWidth))
        val targets = targetMs.mapIndexed { i, m -> m.measure(Constraints.fixed(width, rows.heights[i])) }
        val named = flagRow.coerceIn(0, (n - 1).coerceAtLeast(0))
        layout(width, height) {
            // The body and its rows end at the rail's end edge and run toward the start
            // (from its start edge toward the end, for a rail that stands there).
            val x = if (atStart) 0 else width - grownWidth
            body.placeRelative(x, 0)
            ink.forEachIndexed { i, p -> p.placeRelative(x, rows.tops[i]) }
            onPlate.forEachIndexed { i, p -> p.placeRelative(x, rows.tops[i]) }
            if (n > 0) {
                val y = rows.tops[named] + (rows.heights[named] - flagP.height) / 2
                val beside = if (atStart) restWidth + FlagGap.roundToPx() else width - restWidth - FlagGap.roundToPx() - flagP.width
                flagP.placeRelative(beside, y.coerceIn(0, maxOf(0, height - flagP.height)))
            }
            targets.forEachIndexed { i, p -> p.placeRelative(0, rows.tops[i]) }
        }
    }
}

/** The rows' slots, kept by the layout for the finger, the plate and what is drawn. */
private class RowGeometry {
    var tops = IntArray(0)
    var heights = IntArray(0)

    /** The rest column's width in px. */
    var rest = 0

    fun update(slots: List<Int>, pad: Int, rest: Int) {
        heights = slots.toIntArray()
        var y = pad
        tops = IntArray(slots.size) { i -> y.also { y += slots[i] } }
        this.rest = rest
    }

    /** The row under [y], the nearest end beyond them. */
    fun at(y: Float): Int {
        val i = tops.indices.indexOfFirst { y < tops[it] + heights[it] }
        return if (i < 0) tops.lastIndex.coerceAtLeast(0) else i
    }

    /** Where [y] stands among the rows' centres: 1.5 is half way from the second row to the third. */
    fun fractionAt(y: Float): Float {
        if (tops.isEmpty()) return 0f
        fun centre(i: Int) = tops[i] + heights[i] / 2f
        if (y <= centre(0)) return 0f
        for (i in 0 until tops.lastIndex) {
            val b = centre(i + 1)
            if (y < b) return i + (y - centre(i)) / (b - centre(i))
        }
        return tops.lastIndex.toFloat()
    }

    private fun mix(at: Float, values: IntArray): Int {
        if (values.isEmpty()) return 0
        val i = floor(at).toInt().coerceIn(0, values.lastIndex)
        val j = (i + 1).coerceAtMost(values.lastIndex)
        return lerp(values[i].toFloat(), values[j].toFloat(), at - i).roundToInt()
    }

    fun lerpTop(at: Float) = mix(at, tops)
    fun lerpHeight(at: Float) = mix(at, heights)

    /** How much of the open rail is out of sight at [grown] (0 at rest .. 1 open), in px from its start edge. */
    fun hidden(width: Float, grown: Float) = (1f - grown) * (width - rest)
}

/**
 * The rail's floating body as far as it is open ([grown]), the row a finger has landed on
 * ([press] 0..1 on [pressRow]) and the lifted plate at row [at]: all drawn, in the open
 * rail's own box, so opening, dimming and the plate's travel measure and place nothing.
 */
private fun Modifier.railBody(c: OscylColors, rows: RowGeometry, atStart: Boolean, grown: () -> Float, at: () -> Float, press: () -> Float, pressRow: () -> Int): Modifier = drawWithCache {
    val f = OscylTokens.Floating
    val radius = FloatingRadius.toPx()
    val inner = OscylTokens.radiusPanel.toPx()
    val casts = c.shadow.alpha != 0f
    val far = SoftShadow(c.shadow.copy(alpha = c.shadow.alpha * f.farAlpha), f.farRadius.toPx(), radius)
    val near = SoftShadow(c.shadow.copy(alpha = c.shadow.alpha * f.nearAlpha), f.nearRadius.toPx(), radius)
    val farBy = Offset(0f, f.farOffset.toPx().roundToInt().toFloat())
    val nearBy = Offset(0f, f.nearOffset.toPx().roundToInt().toFloat())
    val lifted = SoftShadow(c.shadow, 3.dp.toPx(), inner)
    val liftedBy = 1.dp.toPx().roundToInt().toFloat()
    val enamel = Brush.verticalGradient(listOf(lerp(c.plate, Color.White, 0.06f), lerp(c.plate, Color.Black, 0.05f)))
    val rim = c.floatingRim
    val w = OscylTokens.ruleWidth.toPx()
    val stroke = Stroke(w)
    val insetX = PlateInsetX.toPx()
    val insetY = PlateInsetY.toPx()
    onDrawBehind {
        val hide = rows.hidden(size.width, grown())
        val part = Size(size.width - hide, size.height)
        // What is out of sight is on the side it opens toward.
        val left = if ((layoutDirection == LayoutDirection.Ltr) != atStart) hide else 0f
        translate(left = left) {
            if (casts) {
                softShadow(far, part, farBy, 1f)
                softShadow(near, part, nearBy, 1f)
            }
            drawRoundRect(c.raised, size = part, cornerRadius = CornerRadius(radius))
            translate(w / 2, w / 2) { drawRoundRect(rim, size = Size(part.width - w, part.height - w), cornerRadius = CornerRadius(radius), style = stroke) }
            val across = part.width - 2 * insetX
            val row = pressRow()
            val p = press()
            if (p > 0f && row in rows.tops.indices) {
                drawRoundRect(c.press, Offset(insetX, rows.tops[row] + insetY), Size(across, rows.heights[row] - 2 * insetY), CornerRadius(inner), alpha = p)
            }
            if (rows.tops.isNotEmpty()) {
                val top = rows.lerpTop(at()) + insetY
                val tall = rows.lerpHeight(at()) - 2 * insetY
                if (casts) softShadow(lifted, Size(across, tall), Offset(insetX, top + liftedBy), 1f)
            }
        }
        if (rows.tops.isNotEmpty()) {
            val top = rows.lerpTop(at()) + insetY
            val tall = rows.lerpHeight(at()) - 2 * insetY
            // Inset, so the enamel's sweep runs over the plate, not over the rail.
            inset(left + insetX, top, size.width - left - part.width + insetX, size.height - top - tall) {
                drawRoundRect(enamel, cornerRadius = CornerRadius(inner))
            }
        }
    }
}

/**
 * Row [i] as far as the rail is open, and of its two copies the one that belongs here:
 * [onPlate] draws only where the plate stands over the row, the other only where it does not.
 */
private fun Modifier.railRow(rows: RowGeometry, i: Int, onPlate: Boolean, atStart: Boolean, grown: () -> Float, at: () -> Float): Modifier = drawWithContent {
    val hide = rows.hidden(size.width, grown())
    val ltr = (layoutDirection == LayoutDirection.Ltr) != atStart
    val insetY = PlateInsetY.toPx()
    val top = rows.lerpTop(at()) + insetY - (rows.tops.getOrNull(i) ?: 0)
    val bottom = top + rows.lerpHeight(at()) - 2 * insetY
    val over = bottom > 0f && top < size.height
    clipRect(if (ltr) hide else 0f, 0f, if (ltr) size.width else size.width - hide, size.height) {
        when {
            onPlate -> if (over) clipRect(top = top, bottom = bottom) { this@drawWithContent.drawContent() }
            over -> clipRect(top = top, bottom = bottom, clipOp = ClipOp.Difference) { this@drawWithContent.drawContent() }
            else -> this@drawWithContent.drawContent()
        }
    }
}

/**
 * A stage's row: its drawing in the rest column and, shown as the rail opens, its words.
 * A phone's row keeps the drawing at the end edge, under the thumb that reached for it,
 * its words toward the start and its count on the drawing, open or closed. A [roomy]
 * row (the column that stands open in a wide window) reads from the start: the drawing,
 * the words, the count at its end.
 */
@Composable
private fun StageRow(stage: StageItem, onPlate: Boolean, roomy: Boolean, atStart: Boolean, grown: () -> Float, modifier: Modifier) {
    val c = OscylTheme.colors
    Row(
        modifier.fillMaxWidth().heightIn(min = if (roomy) RoomyRow else RestRow).then(if (roomy) Modifier.padding(end = 10.dp) else Modifier),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (roomy) {
            StageDrawing(stage, onPlate, count = { 1f - grown() })
            StageWords(stage, onPlate, end = false, Modifier.weight(1f).padding(start = 4.dp, end = 8.dp, top = 6.dp, bottom = 6.dp).graphicsLayer { alpha = grown() })
            Box(Modifier.widthIn(min = 18.dp).graphicsLayer { alpha = grown() }) {
                if (stage.edits > 0) CountBox(stage.edits) else Spacer(Modifier.size(18.dp))
            }
        } else if (atStart) {
            StageDrawing(stage, onPlate, count = { 1f })
            StageWords(stage, onPlate, end = false, Modifier.weight(1f).padding(start = 2.dp, end = 14.dp, top = 4.dp, bottom = 4.dp).graphicsLayer { alpha = grown() }, oneLine = true)
        } else {
            StageWords(stage, onPlate, end = true, Modifier.weight(1f).padding(start = 14.dp, end = 2.dp, top = 4.dp, bottom = 4.dp).graphicsLayer { alpha = grown() })
            StageDrawing(stage, onPlate, count = { 1f })
        }
    }
}

/** The stage's drawing in its 48 dp cell and, on an edited stage, the orange count in the cell's corner. */
@Composable
private fun StageDrawing(stage: StageItem, onPlate: Boolean, count: () -> Float) {
    val c = OscylTheme.colors
    Box(Modifier.size(RestWidth, RestRow), contentAlignment = Alignment.Center) {
        Image(painterResource(stage.icon), null, Modifier.size(18.dp), colorFilter = ColorFilter.tint(if (onPlate) c.onPlate else c.legend))
        if (stage.edits > 0) {
            Box(
                Modifier
                    .align(Alignment.TopEnd)
                    // In the cell's corner, clear of the drawing, large enough to read.
                    .padding(top = 1.dp, end = 1.dp)
                    .graphicsLayer { alpha = count() }
                    .heightIn(min = 16.dp)
                    .widthIn(min = 16.dp)
                    .background(c.act, RoundedCornerShape(4.dp))
                    .padding(horizontal = 2.dp),
                contentAlignment = Alignment.Center,
            ) {
                BasicText(
                    // Latin digits, as the values and slots beside it (one digit system on a page).
                    if (stage.edits > 9) "9+" else stage.edits.toString(),
                    style = MaterialTheme.typography.labelSmall.copy(color = c.onAct, fontSize = 11.sp, lineHeight = 11.sp, fontWeight = FontWeight.Bold, fontFeatureSettings = "tnum", letterSpacing = 0.sp),
                )
            }
        }
    }
}

/**
 * The name over the summary, at the [end] edge or the start; where the two would stand
 * taller than the row (large text), the summary gives way and the name stays.
 */
@Composable
private fun StageWords(stage: StageItem, onPlate: Boolean, end: Boolean, modifier: Modifier, oneLine: Boolean = end) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    Layout(
        content = {
            BasicText(
                stage.title,
                style = type.bodyMedium.copy(color = if (onPlate) c.onPlate else c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content),
                maxLines = if (oneLine) 1 else Int.MAX_VALUE,
                overflow = TextOverflow.Ellipsis,
            )
            BasicText(
                stage.summary,
                style = type.bodySmall.copy(color = if (onPlate) c.plateLegend else c.legend, textDirection = TextDirection.Content),
                maxLines = if (oneLine) 1 else Int.MAX_VALUE,
                overflow = TextOverflow.Ellipsis,
            )
        },
        modifier = modifier,
    ) { (title, summary), constraints ->
        val loose = constraints.copy(minWidth = 0, minHeight = 0, maxHeight = Constraints.Infinity)
        val t = title.measure(loose)
        val s = summary.measure(loose)
        val both = t.height + s.height <= constraints.maxHeight
        val height = (if (both) t.height + s.height else t.height).coerceIn(constraints.minHeight, constraints.maxHeight)
        val width = maxOf(t.width, if (both) s.width else 0).coerceIn(constraints.minWidth, constraints.maxWidth)
        layout(width, height) {
            val top = (height - if (both) t.height + s.height else t.height) / 2
            t.placeRelative(if (end) width - t.width else 0, top)
            if (both) s.placeRelative(if (end) width - s.width else 0, top + t.height)
        }
    }
}

/**
 * The flag: the touched stage's name and summary on a small floating part beside the
 * rail, there on touch-down ([shown] 0..1: a fade and an 8 dp slide from the rail on
 * the one easing) and gone when the page has landed. No ripple, no pill.
 */
@Composable
private fun StageFlag(stage: StageItem?, atStart: Boolean, shown: () -> Float) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val toward = if ((LocalLayoutDirection.current == LayoutDirection.Ltr) != atStart) 1f else -1f
    Box(
        Modifier
            .clearAndSetSemantics {}
            .graphicsLayer {
                val a = shown()
                alpha = a
                translationX = (1f - a) * FlagGap.toPx() * toward
                compositingStrategy = CompositingStrategy.ModulateAlpha
            }
            .floating(c, RoundedCornerShape(OscylTokens.radiusPanel))
            .padding(horizontal = 12.dp, vertical = 6.dp),
    ) {
        if (stage != null) {
            androidx.compose.foundation.layout.Column(horizontalAlignment = if (atStart) Alignment.Start else Alignment.End) {
                BasicText(stage.title, style = type.bodyMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content), maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (stage.summary.isNotBlank()) {
                    BasicText(stage.summary, style = type.bodySmall.copy(color = c.legend, textDirection = TextDirection.Content), maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
        }
    }
}

/** Sample stages from the Front Panel sketch; the real ones come from the synth's driver. */
internal val SampleStages = listOf(
    StageItem("Voice", "Poly · 4 voices", 0, Res.drawable.ic_stage_voice),
    StageItem("Oscillators", "Saw + Sqr · Ring", 1, Res.drawable.ic_stage_oscillator),
    StageItem("Filter", "LP 2-pole · 618", 2, Res.drawable.ic_stage_filter),
    StageItem("Envelopes", "A 12 · R 640", 1, Res.drawable.ic_stage_envelope),
    StageItem("Modulation", "LFO tri · 2.4 Hz", 12, Res.drawable.ic_stage_lfo),
    StageItem("Effects", "Chorus · Hall 38%", 0, Res.drawable.ic_stage_effects),
)

@Composable
private fun RailPreview(heldAt: Int? = null, pressedAt: Int? = null) = Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.TopEnd) {
    StageRail(SampleStages, current = 2, onJump = {}, onScrub = {}, modifier = Modifier, heldAt = heldAt, pressedAt = pressedAt)
}

@OscylPreviews
@Composable
private fun stageRail_resting(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) { RailPreview() }

@OscylPreviews
@Composable
private fun stageRail_pressed(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) { RailPreview(pressedAt = 1) }

@OscylPreviews
@Composable
private fun stageRail_held_scrubbing(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    PreviewTheme(mode) { RailPreview(heldAt = 3) }

@OscylA11yPreviews
@Composable
private fun stageRail_a11y() = PreviewTheme { RailPreview() }

@OscylA11yPreviews
@Composable
private fun stageRail_pressed_a11y() = PreviewTheme { RailPreview(pressedAt = 1) }

@OscylA11yPreviews
@Composable
private fun stageRail_held_a11y() = PreviewTheme { RailPreview(heldAt = 3) }
