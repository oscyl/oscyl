package io.github.oscyl.ui.shell

import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.listSaver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.snapshots.SnapshotStateList
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.layoutId
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.Constraints
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.motionScale
import kotlin.math.roundToInt

/** The app's screen-change motion (DESIGN.md "Motion"), one easing and three tweens. */
object OscylTransitions {
    /** A destination leaving fades out over `fast` (90 ms). */
    const val outMs = OscylTokens.fastMs

    /** The new one arrives over `base` (150 ms). */
    const val inMs = OscylTokens.baseMs

}

/**
 * The app root's shared-transition scope, for container transforms between parts
 * that live in different places (a row into the editor, the first run's card into the chip).
 * Null outside the app (previews).
 */
val LocalSharedTransitionScope = staticCompositionLocalOf<SharedTransitionScope?> { null }

/**
 * Whether the destination a part stands in is the one shown. Destinations stay composed
 * under each other, so a part that takes Back says so only while this is true.
 */
val LocalDestinationShown = androidx.compose.runtime.compositionLocalOf { true }

/** The shared key of the chip in this destination's bar (the first run's card lands in it). */
val LocalChipKey = staticCompositionLocalOf { "chip" }

/** The chip's key for [destination]. */
fun chipKey(destination: Destination) = "chip:${destination.name}"

/**
 * [this] as one end of a shared element [key] (the chip, the first run's
 * card): its bounds morph into the other end's when [visible] flips. A plain modifier
 * outside the app (previews).
 */
@OptIn(androidx.compose.animation.ExperimentalSharedTransitionApi::class)
@Composable
fun Modifier.sharedPart(key: String, visible: Boolean): Modifier {
    val scope = LocalSharedTransitionScope.current ?: return this
    return with(scope) {
        // The content keeps its own (final) size and is clipped to the morphing bounds: it never reflows mid-flight.
        // Only in flight: at rest the part draws its own edge (a clip there shaved a smaller-cornered card's outline).
        this@sharedPart.sharedElementWithCallerManagedVisibility(rememberSharedContentState(key), visible)
            .then(if (isTransitionActive) Modifier.clip(androidx.compose.foundation.shape.RoundedCornerShape(io.github.oscyl.ui.parts.FloatingRadius)) else Modifier)
            .skipToLookaheadSize()
    }
}

/**
 * Where the destinations are between two of them: [from] leaving, [to] arriving, [t]
 * from 0 to 1 over the leaving (`fast`) and then the arriving (`base`). At rest both
 * are the one shown and [t] is 1. A switch runs it; predictive Back seeks it with the
 * finger and lets it finish or return.
 */
@androidx.compose.runtime.Stable
class DestinationMotion(start: Destination) {
    var from by androidx.compose.runtime.mutableStateOf(start)
        private set
    var to by androidx.compose.runtime.mutableStateOf(start)
        private set
    val t = Animatable(1f)

    /** The finger is [progress] (0..1) of the way through Back from [current] to [target]. */
    suspend fun seek(current: Destination, target: Destination, progress: Float) {
        if (from != current || to != target) {
            move++
            from = current
            to = target
        }
        // The finger shows where Back leads, most of the way; letting go finishes it.
        t.snapTo(progress.coerceIn(0f, 1f) * 0.85f)
    }

    /** Counts the moves begun (a switch, a Back gesture): a late end of one never undoes the next. */
    private var move = 0

    /**
     * The gesture was let go without going back: [from] returns. If a switch has begun
     * since (a tab tapped as the finger lifted), this does nothing: the switch owns the
     * pages now, and putting [from] back would show one page under another's name.
     */
    suspend fun release(scale: Float) {
        val mine = move
        if (scale > 0f) t.animateTo(0f, tween((OscylTransitions.outMs * scale).roundToInt(), easing = OscylTokens.easing))
        if (move != mine) return
        to = from
        t.snapTo(1f)
    }

    /**
     * [current] is the destination now: go there (from wherever a gesture left it).
     * [settle] waits, before anything moves, for the arriving page's first frame (it may
     * be composing and laying out for the first time, under the page still shown): the
     * motion then plays whole, instead of jumping past the frames that work swallowed.
     */
    suspend fun run(current: Destination, scale: Float, settle: suspend () -> Unit = {}) {
        if (to != current) {
            move++
            from = to
            to = current
            t.snapTo(0f)
            if (scale > 0f) settle()
        }
        val left = ((OscylTransitions.outMs + OscylTransitions.inMs) * scale * (1f - t.value)).roundToInt()
        if (left <= 0) t.snapTo(1f) else t.animateTo(1f, tween(left, easing = androidx.compose.animation.core.LinearEasing))
        from = to
    }

    /** How far [from] has left (0..1) and [to] has arrived (0..1) at [t], each on the one easing. */
    internal fun left(): Float = OscylTokens.easing.transform((t.value / Out).coerceIn(0f, 1f))
    // The arriving one sets out when the leaving one is half gone: no frame is empty between them.
    internal fun arrived(): Float = OscylTokens.easing.transform(((t.value - Out / 2) / (1f - Out / 2)).coerceIn(0f, 1f))

    private companion object {
        /** The leaving's share of the whole. */
        const val Out = OscylTransitions.outMs.toFloat() / (OscylTransitions.outMs + OscylTransitions.inMs)
    }
}

/**
 * The destinations, each kept composed once visited (its scroll, its pane, its
 * fields), with only what shows placed. They stand side by side in the island's order:
 * a switch moves along it. The old one leaves toward its side over 90 ms, fading; then
 * the new one arrives from its side over 150 ms, [axisPx] along the axis, on the page's
 * ground, setting out as the old one is half gone: no frame is blank (the new one is
 * composed and measured from the first frame). Only layers move. With the app's motion off or the
 * system's animator scale at 0, a switch is an instant cut. Hidden destinations are
 * silent to screen readers and take no touches (they are not placed).
 */
@Composable
fun DestinationHost(
    current: Destination,
    modifier: Modifier = Modifier,
    motion: DestinationMotion = remember { DestinationMotion(current) },
    axisPx: Int = 0,
    /** A page or a sheet stands over the destinations: none is in sight, none takes Back. */
    covered: Boolean = false,
    content: @Composable (Destination) -> Unit,
) {
    val holder = rememberSaveableStateHolder()
    val visited = rememberSaveable(saver = VisitedSaver) { mutableStateListOf(current) }
    // A destination is composed from its first visit on (and in that same frame); one a
    // Back gesture is showing counts as visited.
    val wanted = listOf(current, motion.to).distinct()
    val composed = visited + wanted.filter { it !in visited }
    SideEffect { wanted.forEach { if (it !in visited) visited.add(it) } }
    val animate by rememberUpdatedState(LocalOscylMotion.current)
    LaunchedEffect(current) { motion.run(current, motionScale(animate)) { repeat(2) { androidx.compose.runtime.withFrameNanos {} } } }
    Layout(
        content = {
            composed.forEach { d ->
                key(d) {
                    Box(
                        Modifier
                            .layoutId(d)
                            .then(if (d == current && motion.to == current) Modifier else Modifier.clearAndSetSemantics {})
                            .graphicsLayer {
                                // A page fades stroke by stroke: through an offscreen copy, each switch
                                // made two copies of the whole screen in its first frame.
                                compositingStrategy = CompositingStrategy.ModulateAlpha
                                val (from, to) = motion.from to motion.to
                                // Toward the end edge when the new one stands after the old one.
                                val dir = if (to.ordinal >= from.ordinal) 1f else -1f
                                when {
                                    from == to -> Unit
                                    d == to -> {
                                        val v = motion.arrived()
                                        alpha = v
                                        translationX = dir * (1f - v) * axisPx
                                    }
                                    d == from -> {
                                        val u = motion.left()
                                        alpha = 1f - u
                                        translationX = -dir * u * axisPx / 2f
                                    }
                                }
                            },
                    ) {
                        holder.SaveableStateProvider(d.name) {
                            androidx.compose.runtime.CompositionLocalProvider(LocalChipKey provides chipKey(d), LocalDestinationShown provides (d == current && !covered)) { content(d) }
                        }
                    }
                }
            }
        },
        modifier = modifier,
    ) { measurables, constraints ->
        val fixed = Constraints.fixed(constraints.maxWidth, constraints.maxHeight)
        // Only what shows is measured and placed: the one arriving, and the one leaving.
        val (from, to) = motion.from to motion.to
        val placeables = measurables.mapNotNull { m ->
            val d = m.layoutId as Destination
            if (d == to || d == from) d to m.measure(fixed) else null
        }
        layout(constraints.maxWidth, constraints.maxHeight) {
            placeables.forEach { (d, p) -> if (d != to) p.place(0, 0) }
            placeables.forEach { (d, p) -> if (d == to) p.place(0, 0, zIndex = 1f) }
        }
    }
}

private val VisitedSaver = listSaver<SnapshotStateList<Destination>, String>(
    save = { list -> list.map { it.name } },
    restore = { names -> names.mapNotNull { n -> Destination.entries.firstOrNull { it.name == n } }.toMutableStateList() },
)

private fun <T> List<T>.toMutableStateList() = mutableStateListOf<T>().also { it.addAll(this) }
