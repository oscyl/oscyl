package io.github.oscyl.ui.editor

import io.github.oscyl.ui.shell.ChromeScale
import androidx.compose.ui.unit.LayoutDirection
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.theme.OscylThemeMode
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.ui.geometry.isSpecified
import kotlin.time.Duration.Companion.milliseconds
import androidx.compose.foundation.background
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicText
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.border
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculateCentroid
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.layout
import androidx.compose.ui.AbsoluteAlignment
import androidx.compose.foundation.layout.absoluteOffset
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChanged
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.runtime.derivedStateOf
import androidx.compose.ui.semantics.setProgress
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import io.github.oscyl.core.AxisWord
import io.github.oscyl.core.GraphShape
import io.github.oscyl.core.HandleInfo
import io.github.oscyl.core.MarkStyle
import io.github.oscyl.core.StageGraph
import io.github.oscyl.platform.LocalOscylHaptics
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.axis_attack
import io.github.oscyl.shared.generated.resources.axis_decay
import io.github.oscyl.shared.generated.resources.axis_hz100
import io.github.oscyl.shared.generated.resources.axis_khz1
import io.github.oscyl.shared.generated.resources.axis_khz10
import io.github.oscyl.shared.generated.resources.axis_note
import io.github.oscyl.shared.generated.resources.axis_percent0
import io.github.oscyl.shared.generated.resources.axis_percent100
import io.github.oscyl.shared.generated.resources.axis_percent50
import io.github.oscyl.shared.generated.resources.axis_pitch
import io.github.oscyl.shared.generated.resources.axis_release
import io.github.oscyl.shared.generated.resources.axis_sustain
import io.github.oscyl.shared.generated.resources.axis_time
import io.github.oscyl.shared.generated.resources.graph_fit
import io.github.oscyl.shared.generated.resources.graph_handle
import io.github.oscyl.shared.generated.resources.graph_move_down
import io.github.oscyl.shared.generated.resources.graph_move_left
import io.github.oscyl.shared.generated.resources.graph_move_right
import io.github.oscyl.shared.generated.resources.graph_move_up
import io.github.oscyl.shared.generated.resources.graph_zoom_in
import io.github.oscyl.theme.OscylColors
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.motionScale
import kotlin.math.roundToInt
import kotlinx.coroutines.launch
import org.jetbrains.compose.resources.stringResource

/** The deepest zoom, and where a double tap goes. */
private const val MaxZoom = 4f
private const val TapZoom = 2.5f

/** A screen reader's handle step, in graph widths. */
private const val KeyStep = 0.05f

/**
 * Where a shape is on its way: glides between two pictures of one kind point by
 * point (frame clock, `slow`, the one easing), and crossfades between others, as the
 * desktop graph does. Read in the draw phase only.
 */
@Stable
private class Glide(start: Pic?) {
    var from: Pic? by mutableStateOf(start)
    var to: Pic? by mutableStateOf(start)
    val t = Animatable(1f)

    /** The picture shown now, or null mid-crossfade (then draw [from] and [to] faded). */
    fun shown(): Pic? = lerp(from, to, t.value)
}

/** Paths and pens kept from frame to frame: a glide or a drag draws without allocating new ones. */
private class Paths {
    private val all = mutableListOf<Path>()
    private var used = 0
    private var kept: Pens? = null

    /** The pens at [density] (pixels per dp), made again only when it changes. */
    fun pens(density: Float): Pens = kept?.takeIf { it.density == density } ?: Pens(density).also { kept = it }

    fun rewind() {
        used = 0
    }

    fun next(): Path = (all.getOrNull(used) ?: Path().also { all += it }).also {
        it.rewind()
        used++
    }
}

/** The dashes and strokes a graph draws with, in pixels at [density]. */
private class Pens(val density: Float) {
    private fun dp(v: Float) = v * density
    val dash24 = PathEffect.dashPathEffect(floatArrayOf(dp(2f), dp(4f)))
    val dash23 = PathEffect.dashPathEffect(floatArrayOf(dp(2f), dp(3f)))
    val dash3 = PathEffect.dashPathEffect(floatArrayOf(dp(3f), dp(3f)))
    val line = Stroke(dp(1.75f), cap = StrokeCap.Round, join = StrokeJoin.Round)
    val faintLine = Stroke(dp(1.25f), cap = StrokeCap.Round, join = StrokeJoin.Round, pathEffect = PathEffect.dashPathEffect(floatArrayOf(dp(4f), dp(4f))))
    val outline = Stroke(dp(1f))
    val sweep = Stroke(dp(1.25f))
    val faintSweep = Stroke(dp(1.25f), pathEffect = dash3)
}

/** A picture with its marks' points unboxed, so a glide frame allocates only flat arrays. */
private class Pic(val shape: GraphShape, val points: List<FloatArray>)

private fun GraphShape.pic() = Pic(this, marks.map { it.points.toFloatArray() })

/** [a] [t] of the way to [b] when they share a kind and point counts; null otherwise. */
private fun lerp(a: Pic?, b: Pic?, t: Float): Pic? {
    if (a == null || b == null || t >= 1f) return if (t >= 1f) b else null
    val (x, y) = a.shape to b.shape
    if (x.kind != y.kind || x.marks.size != y.marks.size || x.ticks.size != y.ticks.size || x.rules.size != y.rules.size) return null
    if (x.marks.indices.any { a.points[it].size != b.points[it].size || x.marks[it].style != y.marks[it].style }) return null
    fun l(p: Float, q: Float) = p + (q - p) * t
    val shape = y.copy(
        rules = y.rules.mapIndexed { i, v -> l(x.rules[i], v) },
        cutoff = y.cutoff?.let { c -> x.cutoff?.let { l(it, c) } ?: c },
        ticks = y.ticks.mapIndexed { i, k -> k.copy(x = l(x.ticks[i].x, k.x), labelX = l(x.ticks[i].labelX, k.labelX), room = l(x.ticks[i].room, k.room)) },
    )
    return Pic(shape, b.points.mapIndexed { i, q -> val p = a.points[i]; FloatArray(q.size) { j -> l(p[j], q[j]) } })
}

/** The zoom: [scale] about the frame's origin, then [pan] (px, never past the edges). */
@Stable
private class Zoom {
    val scale = Animatable(1f)
    val panX = Animatable(0f)
    val panY = Animatable(0f)

    /** Never past the edges, and always a number, whatever a gesture or a frame not laid out yet hands in. */
    fun clampPan(x: Float, y: Float, s: Float, w: Float, h: Float): Offset {
        fun within(v: Float, room: Float) = if (v.isFinite()) v.coerceIn(minOf(-(s - 1f) * room, 0f), 0f) else 0f
        return Offset(within(x, w), within(y, h))
    }
}

/** Where data (x 0..1 left to right, y 0..1 bottom to top) lands in px. */
private class Frame(val rect: Rect, val scale: Float, val pan: Offset) {
    fun x(x: Float) = rect.left + x * rect.width * scale + pan.x
    fun y(y: Float) = rect.top + (1f - y) * rect.height * scale + pan.y
    fun at(x: Float, y: Float) = Offset(x(x), y(y))
    fun dataX(px: Float) = (px - rect.left - pan.x) / (rect.width * scale).coerceAtLeast(1f)
    fun dataY(py: Float) = 1f - (py - rect.top - pan.y) / (rect.height * scale).coerceAtLeast(1f)
}

/** The plot area inside a graph of [size] for [shape]: room for handles and the scale. */
private fun plot(size: Size, shape: GraphShape?, em: Float, pad: Float): Rect {
    val below = if (shape != null && shape.below && (shape.ticks.isNotEmpty() || shape.time)) 1.4f * em else 0f
    val side = if (shape != null && shape.levels.isNotEmpty()) 2.6f * em else 0f
    return Rect(pad + side, pad, (size.width - pad).coerceAtLeast(pad + side + 1f), (size.height - pad - below).coerceAtLeast(pad + 1f))
}

@Composable
private fun word(w: AxisWord): String = stringResource(
    when (w) {
        AxisWord.HZ100 -> Res.string.axis_hz100
        AxisWord.KHZ1 -> Res.string.axis_khz1
        AxisWord.KHZ10 -> Res.string.axis_khz10
        AxisWord.ATTACK -> Res.string.axis_attack
        AxisWord.DECAY -> Res.string.axis_decay
        AxisWord.SUSTAIN -> Res.string.axis_sustain
        AxisWord.RELEASE -> Res.string.axis_release
        AxisWord.NOTE -> Res.string.axis_note
        AxisWord.TIME -> Res.string.axis_time
        AxisWord.PITCH -> Res.string.axis_pitch
        AxisWord.PERCENT0 -> Res.string.axis_percent0
        AxisWord.PERCENT50 -> Res.string.axis_percent50
        AxisWord.PERCENT100 -> Res.string.axis_percent100
    },
)

/**
 * A stage's picture (DESIGN.md "Stage graph"): a 1 dp ruled frame with 10 dp corners,
 * the ink line with its soft fill, legends and rules in `legend`; while edited, the
 * saved shape dashed in `graphFaint` behind and the cutoff marked dashed in `act`.
 * Every new shape glides there (a handle's own drag, and a knob being turned, follow the finger at once).
 * Pinch or double-tap to zoom with eased glides, drag to pan when zoomed, double-tap
 * again to fit; handles stay 48 dp targets at any zoom, each with its name, value
 * and move actions for screen readers. Drawing and zoom run in the draw and layout
 * phases: a glide never recomposes.
 */
@Composable
fun StageGraphView(
    graph: StageGraph,
    handles: List<HandleInfo>,
    description: String,
    onDrag: (handle: Int, x: Float, y: Float) -> Unit,
    onRelease: () -> Unit,
    modifier: Modifier = Modifier,
    /** The frame's height; null fills the height it is given (the full-screen graph). */
    height: Dp? = GraphHeight,
    /** Full screen: larger handles, and the value in a plate tag beside the finger while dragging. */
    large: Boolean = false,
    /** A tap on the picture (not a handle): the stage's graph opens full screen. */
    onTap: (() -> Unit)? = null,
    /** Two fingers are on the picture, or it is zoomed in: what holds it should keep still. */
    onZooming: ((Boolean) -> Unit)? = null,
    /** Previews only: the handle shown as held. */
    startDragging: Int? = null,
    /** The picture came from a control being held ([PanelState.following]): it is drawn at once, never glided to. */
    following: () -> Boolean = { false },
) = ChromeScale {
    val c = OscylTheme.colors
    val motion = LocalOscylMotion.current
    val density = LocalDensity.current
    val type = MaterialTheme.typography
    val em = with(density) { type.bodyMedium.fontSize.toPx() }
    val legendStyle = type.bodySmall.copy(color = c.legend)
    val measurer = rememberTextMeasurer()
    val words = AxisWord.entries.associateWith { word(it) }
    val glide = remember { Glide(graph.now?.pic()) }
    val saved = remember(graph.saved) { graph.saved?.pic() }
    val paths = remember { Paths() }
    val savedPaths = remember { Paths() }
    val zoom = remember { Zoom() }
    var size by remember { mutableStateOf(IntSize.Zero) }
    var dragging by remember { mutableStateOf(startDragging) }
    var finger by remember { mutableStateOf(Offset.Zero) }
    // The picture as it is now: what a handle's drag draws, in the very frame its value lands.
    val nowPic = remember(graph.now) { graph.now?.pic() }
    // When the last drag let go: the picture its release settles on arrives at once too.
    val letGo = remember { arrayOfNulls<kotlin.time.TimeMark>(1) }
    val pad = with(density) { 12.dp.toPx() }
    val scope = rememberCoroutineScope()
    val drag = rememberUpdatedState(onDrag)
    val release = rememberUpdatedState(onRelease)

    LaunchedEffect(graph.now) {
        val target = nowPic
        val scale = motionScale(motion)
        // A handle's own drag follows the finger, to its last step and the release's own
        // update (within a moment of letting go); anything else glides there.
        val settling = letGo[0]?.let { it.elapsedNow() < 400.milliseconds } == true
        if (scale == 0f || dragging != null || following() || settling || glide.to == null) {
            glide.from = target
            glide.to = target
            glide.t.snapTo(1f)
            return@LaunchedEffect
        }
        glide.from = glide.shown() ?: glide.to
        glide.to = target
        glide.t.snapTo(0f)
        glide.t.animateTo(1f, tween((OscylTokens.slowMs * scale).roundToInt().coerceAtLeast(1), easing = OscylTokens.easing))
    }

    fun frame(): Frame {
        val r = plot(Size(size.width.toFloat(), size.height.toFloat()), glide.to?.shape, em, pad)
        return Frame(r, zoom.scale.value, Offset(zoom.panX.value, zoom.panY.value))
    }

    suspend fun zoomTo(s: Float, focus: Offset) {
        val f = frame()
        val target = s.coerceIn(1f, MaxZoom)
        // Keep the data under the finger where it is.
        val dx = f.dataX(focus.x)
        val dy = f.dataY(focus.y)
        val pan = zoom.clampPan(
            focus.x - f.rect.left - dx * f.rect.width * target,
            focus.y - f.rect.top - (1f - dy) * f.rect.height * target,
            target, f.rect.width, f.rect.height,
        )
        // One clock for the three: they start and land in the same frames.
        val (s0, x0, y0) = Triple(zoom.scale.value, zoom.panX.value, zoom.panY.value)
        val run: suspend (Float) -> Unit = { t ->
            zoom.scale.snapTo(s0 + (target - s0) * t)
            zoom.panX.snapTo(x0 + (pan.x - x0) * t)
            zoom.panY.snapTo(y0 + (pan.y - y0) * t)
        }
        if (!motion) run(1f) else androidx.compose.animation.core.animate(0f, 1f, animationSpec = tween(OscylTokens.slowMs, easing = OscylTokens.easing)) { t, _ -> scope.launch(start = kotlinx.coroutines.CoroutineStart.UNDISPATCHED) { run(t) } }
    }

    val zoomIn = stringResource(Res.string.graph_zoom_in)
    val fit = stringResource(Res.string.graph_fit)
    val shape = RoundedCornerShape(OscylTokens.radiusPanel)
    // Placed from the top left in every layout direction: the picture is never mirrored, so
    // neither are its handles (right to left they stood mirrored, off their curve).
    Box(
        contentAlignment = AbsoluteAlignment.TopLeft,
        modifier = modifier
            .fillMaxWidth()
            .then(if (height != null) Modifier.height(height) else Modifier.fillMaxHeight())
            .border(OscylTokens.ruleWidth, c.rule, shape)
            .onSizeChanged { size = it }
            .semantics {
                contentDescription = description
                customActions = listOf(
                    CustomAccessibilityAction(if (zoom.scale.value > 1f) fit else zoomIn) {
                        val f = frame()
                        scope.launch { zoomTo(if (zoom.scale.value > 1f) 1f else TapZoom, f.rect.center) }
                        true
                    },
                )
            }
            .pointerInput(em, pad, motion, onTap != null) {
                detectTapGestures(
                    onTap = if (onTap != null) ({ onTap() }) else null,
                    // In the list a tap grows the picture at once; double tap zooms only full screen.
                    onDoubleTap = if (!large) null else { at ->
                        val out = zoom.scale.value > 1.01f
                        onZooming?.invoke(!out)
                        scope.launch { zoomTo(if (out) 1f else TapZoom, at) }
                    },
                )
            }
            .pointerInput(em, pad, motion) {
                // Two fingers zoom and pan; one finger pans only while zoomed (else the page scrolls).
                awaitEachGesture {
                    awaitFirstDown(requireUnconsumed = false)
                    do {
                        val event = awaitPointerEvent()
                        val fingers = event.changes.count { it.pressed }
                        val zoomed = zoom.scale.value > 1.01f
                        onZooming?.invoke(fingers >= 2 || zoomed)
                        // The fingers' middle: there is none on the event that lifts the last one
                        // (it is unspecified, not a number). Zooming about it sent the pan, and
                        // with it every drawn coordinate, to NaN: the page then died in its draw.
                        val centroid = event.calculateCentroid()
                        if (fingers >= 1 && centroid.isSpecified && (fingers >= 2 || (zoomed && event.changes.any { it.positionChanged() && !it.isConsumed }))) {
                            val f = frame()
                            val s = (zoom.scale.value * event.calculateZoom().let { z -> if (z.isFinite() && z > 0f) z else 1f }).coerceIn(1f, MaxZoom)
                            val pan = event.calculatePan()
                            val dx = f.dataX(centroid.x)
                            val dy = f.dataY(centroid.y)
                            val to = zoom.clampPan(
                                centroid.x + pan.x - f.rect.left - dx * f.rect.width * s,
                                centroid.y + pan.y - f.rect.top - (1f - dy) * f.rect.height * s,
                                s, f.rect.width, f.rect.height,
                            )
                            scope.launch {
                                zoom.scale.snapTo(s)
                                zoom.panX.snapTo(to.x)
                                zoom.panY.snapTo(to.y)
                            }
                            event.changes.forEach { it.consume() }
                        }
                    } while (event.changes.any { it.pressed })
                    onZooming?.invoke(zoom.scale.value > 1.01f)
                }
            },
    ) {
        // The picture is clipped to the frame; the handles are not, so a handle at the
        // edge keeps its whole 48 dp target.
        val area = Modifier.size(with(density) { size.width.toDp() }, with(density) { size.height.toDp() }).clipToBounds()
        // The saved shape, dashed, on a layer of its own: it is still while a knob or a
        // handle moves, so its layer is kept and the dashes are not drawn again each frame.
        if (saved != null) {
            Canvas(area.graphicsLayer { compositingStrategy = CompositingStrategy.Offscreen }) {
                savedPaths.rewind()
                val f = frame()
                clipRect(f.rect.left - pad, 0f, f.rect.right + pad, f.rect.bottom + 1f) {
                    drawShape(f, saved, c.graphFaint, faint = true, edited = false, c, savedPaths)
                }
            }
        }
        // Its own layer too: a scroll moves the picture without drawing its curve again.
        Canvas(area.graphicsLayer { compositingStrategy = CompositingStrategy.Offscreen }) {
            paths.rewind()
            val f = frame()
            val edited = graph.saved != null
            // Dragged, the curve is the value's own picture: it never waits a frame for
            // the glide's effect to hear of it, so dot and line move as one.
            // A knob turned (or the Compare fader slid) does the same: each step restarting
            // a glide left the line a fifth of a second behind its own handles.
            val shown = if (dragging != null || (following() && glide.to !== nowPic)) nowPic else glide.shown()
            drawScale(f, (shown ?: glide.to)?.shape, c, measurer, legendStyle, words, em, paths.pens(this.density))
            clipRect(f.rect.left - pad, 0f, f.rect.right + pad, f.rect.bottom + 1f) {
                if (shown != null) {
                    drawShape(f, shown, c.ink, faint = false, edited = edited, c, paths)
                } else {
                    val t = glide.t.value
                    glide.from?.let { drawShape(f, it, c.ink.copy(alpha = c.ink.alpha * (1f - t)), faint = false, edited = edited, c, paths) }
                    glide.to?.let { drawShape(f, it, c.ink.copy(alpha = c.ink.alpha * t), faint = false, edited = edited, c, paths) }
                }
            }
            // A held handle's place on the scales: dashed guides to the left and bottom edges.
            dragging?.let { handles.getOrNull(it) }?.let { h ->
                val p = f.at(h.x, h.y)
                val dash = paths.pens(this.density).dash23
                val hair = 1.dp.toPx()
                drawLine(c.legend, p, Offset(f.rect.left, p.y), hair, pathEffect = dash)
                drawLine(c.legend, p, Offset(p.x, f.rect.bottom), hair, pathEffect = dash)
            }
        }
        handles.forEachIndexed { i, h ->
            // Zoomed out of view: no target, nothing for a screen reader to find.
            val inView by remember(h) {
                derivedStateOf { frame().let { f -> f.rect.inflate(1f).contains(f.at(h.x, h.y)) } }
            }
            if (!inView) return@forEachIndexed
            Handle(
                h,
                large = large,
                dragging = dragging == i,
                at = { frame().at(h.x, h.y) },
                frameSize = { size },
                onStart = { touch ->
                    dragging = i
                    // Where the finger is, in px: the drag adds up from here, never from
                    // the handle's snapped place, so slow moves are never lost.
                    val current = handles.getOrNull(i) ?: h
                    finger = frame().at(current.x, current.y) + touch
                },
                onMove = { delta ->
                    val f = frame()
                    val p = finger + delta
                    finger = p
                    drag.value(i, f.dataX(p.x).coerceIn(0f, 1f), f.dataY(p.y).coerceIn(0f, 1f))
                },
                onStep = { dx, dy ->
                    drag.value(i, (h.x + dx).coerceIn(0f, 1f), (h.y + dy).coerceIn(0f, 1f))
                    release.value()
                },
                onEnd = {
                    // The glide stands where the drag left the picture, before the drag is over.
                    glide.from = nowPic
                    glide.to = nowPic
                    letGo[0] = kotlin.time.TimeSource.Monotonic.markNow()
                    dragging = null
                    release.value()
                },
            )
        }
    }
}

/** How tall a stage's picture stands. */
private val GraphHeight = 176.dp

/**
 * One handle: a 48 dp target centred [at] its point (placed in the layout phase), a
 * `ground` disc with an ink ring (solid ink while dragged, a focus ring when focused).
 */
@Composable
private fun Handle(
    h: HandleInfo,
    large: Boolean,
    dragging: Boolean,
    at: () -> Offset,
    /** The graph frame's size: the value plate stays inside it. */
    frameSize: () -> IntSize,
    /** The drag began this far from the handle's centre. */
    onStart: (Offset) -> Unit,
    onMove: (Offset) -> Unit,
    onStep: (Float, Float) -> Unit,
    onEnd: () -> Unit,
) {
    val c = OscylTheme.colors
    val haptics = LocalOscylHaptics.current
    val interaction = remember { MutableInteractionSource() }
    val focused by interaction.collectIsFocusedAsState()
    val target = if (large) 64.dp else 48.dp
    val left = stringResource(Res.string.graph_move_left)
    val right = stringResource(Res.string.graph_move_right)
    val up = stringResource(Res.string.graph_move_up)
    val down = stringResource(Res.string.graph_move_down)
    val name = stringResource(Res.string.graph_handle, h.name, h.value)
    val move = rememberUpdatedState(onMove)
    val end = rememberUpdatedState(onEnd)
    val start = rememberUpdatedState(onStart)
    Box(
        Modifier
            .absoluteOffset {
                val p = at()
                val half = target.roundToPx() / 2
                IntOffset(p.x.roundToInt() - half, p.y.roundToInt() - half)
            }
            .size(target)
            .semantics {
                contentDescription = name
                stateDescription = h.value
                // A one-axis handle is adjustable like a slider; a two-axis one moves by actions.
                if (h.movesX != h.movesY) {
                    val v = if (h.movesX) h.x else h.y
                    progressBarRangeInfo = ProgressBarRangeInfo(v, 0f..1f)
                    setProgress { to -> if (h.movesX) onStep(to - h.x, 0f) else onStep(0f, to - h.y); true }
                }
                customActions = listOfNotNull(
                    if (h.movesX) CustomAccessibilityAction(left) { onStep(-KeyStep, 0f); true } else null,
                    if (h.movesX) CustomAccessibilityAction(right) { onStep(KeyStep, 0f); true } else null,
                    if (h.movesY) CustomAccessibilityAction(up) { onStep(0f, KeyStep); true } else null,
                    if (h.movesY) CustomAccessibilityAction(down) { onStep(0f, -KeyStep); true } else null,
                )
            }
            .focusable(interactionSource = interaction)
            .pointerInput(Unit) {
                detectDragGestures(
                    onDragStart = { at ->
                        start.value(at - Offset(size.width / 2f, size.height / 2f))
                        haptics.tick()
                    },
                    onDragEnd = { end.value() },
                    onDragCancel = { end.value() },
                ) { change, delta ->
                    change.consume()
                    move.value(delta)
                }
            }
            .drawBehind {
                val r = if (large) 10.dp.toPx() else maxOf(6.dp.toPx(), 0.35f * 14.dp.toPx())
                drawCircle(if (dragging) c.ink else c.ground, r)
                drawCircle(c.ink, r, style = Stroke(2.dp.toPx()))
                if (focused) drawCircle(c.focus, r + 4.dp.toPx(), style = Stroke(2.dp.toPx()))
            },
    )
    // While dragged the value rides beside the finger on a plate, kept inside the frame:
    // at the handle's end side and above it, else at its start side, else below.
    if (dragging) {
        BasicText(
            h.value,
            Modifier
                .layout { measurable, constraints ->
                    val plate = measurable.measure(constraints.copy(minWidth = 0, minHeight = 0))
                    layout(0, 0) {
                        val p = at()
                        val half = target.roundToPx() / 2
                        val frame = frameSize()
                        val px = p.x.roundToInt()
                        var y = p.y.roundToInt() - target.roundToPx()
                        val x = when {
                            px + half + plate.width <= frame.width -> px + half
                            px - half - plate.width >= 0 -> px - half - plate.width
                            else -> {
                                // Too wide for either side: centred over the handle, clear of it vertically.
                                y = if (p.y.roundToInt() - half - plate.height >= 0) p.y.roundToInt() - half - plate.height else p.y.roundToInt() + half
                                (px - plate.width / 2).coerceIn(0, maxOf(0, frame.width - plate.width))
                            }
                        }
                        if (y < 0) y = p.y.roundToInt() + half
                        plate.place(x, y)
                    }
                }
                .background(c.plate, RoundedCornerShape(OscylTokens.radius))
                .padding(horizontal = 8.dp, vertical = 4.dp),
            style = MaterialTheme.typography.bodyMedium.copy(color = c.onPlate, fontWeight = FontWeight.SemiBold, fontFeatureSettings = "tnum"),
        )
    }
}

/** The rules, ticks and legends of [shape] (DESIGN.md: `rule` hairlines, `legend` words). */
private fun DrawScope.drawScale(
    f: Frame,
    shape: GraphShape?,
    c: OscylColors,
    measurer: TextMeasurer,
    style: TextStyle,
    words: Map<AxisWord, String>,
    em: Float,
    pens: Pens,
) {
    shape ?: return
    val hair = 1.dp.toPx()
    val left = f.rect.left
    val right = f.rect.right
    for (y in shape.rules) {
        val py = f.y(y).roundToInt() + 0.5f
        drawLine(c.rule, Offset(left, py), Offset(right, py), hair)
    }
    fun text(t: String, x: Float, y: Float, centre: Boolean, end: Boolean = false) {
        // The picture is never mirrored: its words read left to right in every layout direction.
        val layout = measurer.measure(t, style, layoutDirection = LayoutDirection.Ltr)
        val w = layout.size.width.toFloat()
        val px = when {
            centre -> x - w / 2
            end -> x - w
            else -> x
        }
        // Kept inside the frame: the last legend never runs off the edge.
        val inside = px.coerceIn(0f, maxOf(0f, size.width - w - 6.dp.toPx()))
        drawText(layout, topLeft = Offset(inside.roundToInt().toFloat(), (y - layout.size.height / 2f).roundToInt().toFloat()))
    }
    val row = f.rect.bottom + 0.85f * em
    if (shape.below) {
        val dash = pens.dash24
        for (t in shape.ticks) {
            val x = f.x(t.x).roundToInt() + 0.5f
            if (x < left - 1f || x > right + 1f) continue
            drawLine(c.legend, Offset(x, f.rect.bottom), Offset(x, f.rect.bottom + 0.3f * em), hair)
            val w = t.word ?: continue
            val label = words.getValue(w)
            // A legend shows only where its span leaves it room (a short attack does not).
            if (t.room * f.rect.width * f.scale < measurer.measure(label, style, layoutDirection = LayoutDirection.Ltr).size.width + 0.4f * em) continue
            text(label, f.x(t.labelX), row, centre = true)
        }
        if (f.rect.height * f.scale > 12 * em) {
            // Zoomed in or tall: a dashed grid at tenths.
            for (i in 1 until 10) {
                val x = f.x(i / 10f)
                if (x in left..right) drawLine(c.rule, Offset(x, f.rect.top), Offset(x, f.rect.bottom), hair, pathEffect = dash)
            }
        }
        if (shape.time) text(words.getValue(AxisWord.TIME), right, row, centre = false, end = true)
        if (shape.pitch) text(words.getValue(AxisWord.PITCH), left + 0.4f * em, f.rect.top + 0.5f * em, centre = false)
    }
    if (shape.levels.isNotEmpty()) {
        val dash = pens.dash24
        for (l in shape.levels) {
            val y = f.y(l.y)
            if (l.y > 0f) drawLine(c.rule, Offset(left, y), Offset(right, y), hair, pathEffect = dash)
            text(words.getValue(l.word), left - 0.4f * em, y, centre = false, end = true)
        }
    }
}

/** One shape; [faint] is the saved one behind an edit: thin, dashed, no fills or marker. */
private fun DrawScope.drawShape(f: Frame, pic: Pic, pen: Color, faint: Boolean, edited: Boolean, c: OscylColors, paths: Paths) {
    val shape = pic.shape
    val pens = paths.pens(density)
    val dash3 = pens.dash3
    val line = if (faint) pens.faintLine else pens.line
    fun path(points: FloatArray, close: Boolean = false): Path = paths.next().apply {
        for (i in points.indices step 2) {
            val p = f.at(points[i], points[i + 1])
            if (i == 0) moveTo(p.x, p.y) else lineTo(p.x, p.y)
        }
        if (close) close()
    }
    for ((n, m) in shape.marks.withIndex()) {
        val pts = pic.points[n]
        if (pts.size < 2) continue
        when (m.style) {
            MarkStyle.LINE -> drawPath(path(pts), pen, style = line)
            MarkStyle.AREA -> {
                if (!faint) {
                    val area = floatArrayOf(pts[0], 0f) + pts + floatArrayOf(pts[pts.size - 2], 0f)
                    drawPath(path(area, close = true), pen.copy(alpha = pen.alpha * 0.08f))
                }
                drawPath(path(pts), pen, style = line)
                val cutoff = shape.cutoff
                if (!faint && cutoff != null) {
                    val x = f.x(cutoff)
                    drawLine(
                        if (edited) c.act else c.legend,
                        Offset(x, f.rect.top),
                        Offset(x, f.rect.bottom),
                        1.dp.toPx(),
                        pathEffect = pens.dash23,
                    )
                }
            }
            MarkStyle.BAR -> {
                val (x0, top) = f.at(pts[0], pts[1])
                val (x1, base) = f.at(pts[2], pts[3])
                val rect = Rect(x0, top, x1, maxOf(base, top + 1f))
                if (faint) {
                    drawRect(pen, rect.topLeft, rect.size, style = line)
                } else {
                    drawRect(pen.copy(alpha = pen.alpha * 0.22f), rect.topLeft, rect.size)
                    drawLine(pen, Offset(x0, top), Offset(x1, top), line.width, StrokeCap.Round)
                }
            }
            MarkStyle.TAP -> drawLine(
                pen,
                f.at(pts[0], 0f),
                f.at(pts[0], pts[1]),
                (if (faint) 1.25f else 2f).dp.toPx(),
                StrokeCap.Round,
                pathEffect = if (faint) dash3 else null,
            )
            MarkStyle.WASH -> if (faint) {
                drawPath(path(pts, close = true), pen, style = pens.outline)
            } else {
                drawPath(path(pts, close = true), pen.copy(alpha = pen.alpha * 0.22f))
            }
            MarkStyle.SWEEP -> drawPath(path(pts), pen, style = if (faint) pens.faintSweep else pens.sweep)
            MarkStyle.NOTE -> {
                val thick = (f.rect.height * f.scale * 0.07f).coerceIn(2.5f, 9f)
                val (x0, y) = f.at(pts[0], pts[1])
                val (x1, _) = f.at(pts[2], pts[3])
                val topLeft = Offset(x0 + 1f, y - thick / 2)
                val sz = Size(maxOf(x1 - x0 - 2f, thick), thick)
                if (faint) {
                    drawRoundRect(pen, topLeft, sz, CornerRadius(1f), style = pens.outline)
                } else {
                    drawRoundRect(pen, topLeft, sz, CornerRadius(1f))
                }
            }
        }
    }
}

/**
 * A small picture of [shape] with no scale or handles, as a stage tab or a generated
 * sound's tile shows it, in [pen].
 */
@Composable
fun MiniGraph(shape: GraphShape, pen: Color, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    val pic = remember(shape) { shape.pic() }
    val paths = remember { Paths() }
    val savedPaths = remember { Paths() }
    Canvas(modifier) {
        paths.rewind()
        val inset = 4.dp.toPx()
        val f = Frame(Rect(inset, inset, size.width - inset, size.height - inset), 1f, Offset.Zero)
        drawShape(f, pic, pen, faint = false, edited = false, c, paths)
    }
}

/** A handle held, in the list's size and full screen: its value on a plate, its guides to the scales. */
@OscylPreviews
@Composable
private fun graph_dragging(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    val panel = remember { samplePanel(start = 3) }
    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        StageGraphView(panel.graph(3)!!, panel.handles(3), "", onDrag = { _, _, _ -> }, onRelease = {}, startDragging = 0)
        StageGraphView(panel.graph(3)!!, panel.handles(3), "", onDrag = { _, _, _ -> }, onRelease = {}, height = 320.dp, large = true, startDragging = 0)
    }
}

@OscylA11yPreviews
@Composable
private fun graph_dragging_a11y() = PreviewTheme {
    val panel = remember { samplePanel(start = 3) }
    StageGraphView(panel.graph(3)!!, panel.handles(3), "", onDrag = { _, _, _ -> }, onRelease = {}, startDragging = 0)
}
