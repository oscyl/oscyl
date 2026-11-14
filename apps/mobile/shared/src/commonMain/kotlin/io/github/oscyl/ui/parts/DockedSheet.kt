package io.github.oscyl.ui.parts

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import io.github.oscyl.ui.shell.LocalShellFrame
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import io.github.oscyl.shared.generated.resources.sheet_close
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.backhandler.PredictiveBackHandler
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Velocity
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import org.jetbrains.compose.resources.stringResource
import kotlin.coroutines.cancellation.CancellationException
import kotlin.math.abs
import kotlin.math.roundToInt

/** How far the sheet sinks under a Back swipe held all the way, as a part of its height. */
private const val BackSink = 0.35f

/**
 * The sheets in sight (rising, up or sinking). A sheet asked for while another is still in
 * sight waits until that one has landed below the foot: never two veils at once.
 */
private val inSight = mutableStateListOf<Any>()

/**
 * The one docked sheet (the synths, the review, the edit history; DESIGN.md "Sheets"): a floating sheet that rises from
 * the foot, edge to edge on a phone (a hand's width in a wide window), its top corners
 * rounded, a grip at its head, over a veiled page. It is as tall as its [content], up
 * to most of the window; then the content scrolls, and only once the sheet is fully
 * up. A drag down on it, a flick, a tap on the page or on its grip, or Back (which it follows
 * with the finger) put it away ([onClose]).
 *
 * It moves by placement only (one offset, read where it is placed: nothing is measured
 * again), on `slow` and the one easing. Its content is composed a moment after the app
 * opens and kept, unseen and silent below the foot, so opening it composes nothing;
 * [content] is told whether it is in sight.
 */
@Composable
fun DockedSheet(
    open: Boolean,
    onClose: () -> Unit,
    /** What a screen reader calls the pane. */
    title: String,
    modifier: Modifier = Modifier,
    content: @Composable (visible: Boolean) -> Unit,
) {
    val layer = LocalSheetLayer.current
    if (layer == null) return SheetBody(open, onClose, title, modifier, content)
    // Asked for inside a page: it rises in the app's sheet layer, over the island too.
    val openNow = rememberUpdatedState(open)
    val closeNow = rememberUpdatedState(onClose)
    val titleNow = rememberUpdatedState(title)
    val contentNow = rememberUpdatedState(content)
    val body: @Composable () -> Unit = remember { { SheetBody(openNow.value, closeNow.value, titleNow.value, modifier, contentNow.value) } }
    DisposableEffect(layer) {
        layer.sheets.add(body)
        onDispose { layer.sheets.remove(body) }
    }
}

/** What a sheet's content keeps clear at its end (the navigation bar it runs under): pad a scroll's content by it. */
val LocalSheetFoot = staticCompositionLocalOf { 0.dp }

/** Where the sheets rise: over the shell, island and all. [SheetLayerHost] draws them. */
class SheetLayer {
    internal val sheets = mutableStateListOf<@Composable () -> Unit>()
}

val LocalSheetLayer = staticCompositionLocalOf<SheetLayer?> { null }

/** The sheets asked for under [layer], drawn here (put it over the island). */
@Composable
fun SheetLayerHost(layer: SheetLayer) {
    layer.sheets.forEach { key(it) { it() } }
}

@OptIn(ExperimentalComposeUiApi::class)
@Composable
private fun SheetBody(
    open: Boolean,
    onClose: () -> Unit,
    title: String,
    modifier: Modifier,
    content: @Composable (visible: Boolean) -> Unit,
) {
    val c = OscylTheme.colors
    val motion = LocalOscylMotion.current
    val scope = rememberCoroutineScope()
    val density = LocalDensity.current
    // How far it is let down, as a part of its own height: 0 up, 1 away below the foot.
    val away = remember { Animatable(if (open) 0f else 1f) }
    // Its measured height (px): read by the gesture handlers only, never by composition.
    val height = remember { intArrayOf(0) }
    val openNow by rememberUpdatedState(open)
    val closeNow by rememberUpdatedState(onClose)
    val motionNow by rememberUpdatedState(motion)
    suspend fun settle(to: Float) {
        val scale = motionScale(motionNow)
        if (scale == 0f) away.snapTo(to) else away.animateTo(to, tween((OscylTokens.slowMs * scale).roundToInt(), easing = OscylTokens.easing))
    }
    // In sight until it has landed below the foot.
    var visible by remember { mutableStateOf(open) }
    val me = remember { Any() }
    DisposableEffect(Unit) { onDispose { inSight.remove(me) } }
    LaunchedEffect(open) {
        if (open) {
            // Another sheet on its way down lands first.
            snapshotFlow { inSight.none { it !== me } }.first { it }
            if (me !in inSight) inSight.add(me)
            visible = true
        }
        settle(if (open) 0f else 1f)
        if (!open) {
            visible = false
            inSight.remove(me)
        }
    }
    // Composed in idle time after the first frames, then kept.
    var warm by remember { mutableStateOf(open) }
    LaunchedEffect(Unit) {
        repeat(2) { withFrameNanos {} }
        warm = true
    }
    PredictiveBackHandler(enabled = open) { progress ->
        try {
            progress.collect { away.snapTo(OscylTokens.easing.transform(it.progress.coerceIn(0f, 1f)) * BackSink) }
            onClose()
        } catch (e: CancellationException) {
            scope.launch { settle(0f) }
            throw e
        }
    }
    val flick = with(density) { 400.dp.toPx() }
    /** Moves with the finger; returns what it took of [dy]. */
    fun pull(dy: Float): Float {
        val h = height[0].toFloat()
        if (h <= 0f || !openNow) return 0f
        val to = (away.value + dy / h).coerceIn(0f, 1f)
        val taken = (to - away.value) * h
        if (taken != 0f) scope.launch(start = CoroutineStart.UNDISPATCHED) { away.snapTo(to) }
        return taken
    }
    /** Lets go at [velocity] px/s (down positive): away on a flick down or past a third, else back up. */
    fun letGo(velocity: Float) {
        if (if (abs(velocity) > flick) velocity > 0f else away.value > 1f / 3) closeNow() else scope.launch { settle(0f) }
    }
    // The content scrolls only while the sheet is fully up, and lowers it from its top.
    val nested = remember {
        object : NestedScrollConnection {
            override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset =
                if (source == NestedScrollSource.UserInput && available.y < 0f && away.value > 0f) Offset(0f, pull(available.y)) else Offset.Zero

            override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset =
                if (source == NestedScrollSource.UserInput && available.y > 0f) Offset(0f, pull(available.y)) else Offset.Zero

            override suspend fun onPreFling(available: Velocity): Velocity {
                if (away.value <= 0f) return Velocity.Zero
                letGo(available.y)
                return available
            }
        }
    }
    // Edge to edge: full width, its foot under the navigation bar (its content keeps clear
    // of it), only its top corners rounded. In a wide window a hand's width, centred.
    val shape = RoundedCornerShape(topStart = FloatingRadius, topEnd = FloatingRadius)
    BoxWithConstraints(modifier.fillMaxSize()) {
        val room = maxHeight
        // The veil over the page: a tap there closes it; screen readers use Close or Back.
        Box(
            Modifier
                .fillMaxSize()
                .drawBehind {
                    val a = 1f - away.value
                    if (a > 0f) drawRect(c.shadow.copy(alpha = maxOf(c.shadow.alpha, 0.35f)), alpha = a)
                }
                .then(if (open && visible) Modifier.pointerInput(onClose) { detectTapGestures { onClose() } } else Modifier),
        )
        if (!warm) return@BoxWithConstraints
        // Beside a side island it centres in the room left, so it never stands over the island
        // (and its Save): the veil still covers it.
        val frame = LocalShellFrame.current
        Column(
            Modifier
                .align(Alignment.BottomCenter)
                .padding(start = frame?.sideStart ?: 0.dp, end = frame?.sideEnd ?: 0.dp)
                .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Top))
                .padding(top = 24.dp)
                .widthIn(max = 640.dp)
                .fillMaxWidth()
                .heightIn(max = room * 0.92f)
                .layout { measurable, constraints ->
                    val placeable = measurable.measure(constraints)
                    height[0] = placeable.height
                    // Away is below the foot.
                    layout(placeable.width, placeable.height) { placeable.place(0, (away.value * placeable.height).roundToInt()) }
                }
                .drawWithContent { if (away.value < 1f) drawContent() }
                .floating(c, shape, edgeToEdge = true)
                .nestedScroll(nested)
                .draggable(rememberDraggableState { pull(it) }, Orientation.Vertical, enabled = open, onDragStopped = { letGo(it) })
                // Its own taps stay its own: none falls through to the veil.
                .pointerInput(Unit) { detectTapGestures {} }
                .then(if (visible) Modifier.semantics { paneTitle = title } else Modifier.clearAndSetSemantics {}),
        ) {
            // The grip: drag it down, or tap it, to put the sheet away. It is the sheet's one
            // Close (no ✕ beside the title): a screen reader finds it as the Close button.
            val closeLabel = stringResource(Res.string.sheet_close)
            Box(
                Modifier
                    // A 48 dp target that takes only 28 dp of the sheet: the title row stands
                    // over its lower 20 dp, and a touch there that the row does not take is the grip's.
                    .layout { measurable, constraints ->
                        val p = measurable.measure(constraints.copy(minHeight = GripTarget.roundToPx(), maxHeight = GripTarget.roundToPx()))
                        layout(p.width, GripRoom.roundToPx()) { p.place(0, 0) }
                    }
                    .fillMaxWidth()
                    .testTag("sheet-grip")
                    // A tap, not a drag: a pull that starts on it (or under the title) moves the sheet,
                    // and letting go short of a third leaves it open.
                    .pointerInput(Unit) {
                        awaitEachGesture {
                            awaitFirstDown(requireUnconsumed = false)
                            // The sheet follows a pull, so the finger never leaves the grip: a pull
                            // is told by the sheet's drag taking the movement.
                            var pulled = false
                            do {
                                val event = awaitPointerEvent(PointerEventPass.Final)
                                if (event.changes.any { it.isConsumed && it.pressed }) pulled = true
                            } while (event.changes.any { it.pressed })
                            if (!pulled) closeNow()
                        }
                    }
                    .semantics {
                        contentDescription = closeLabel
                        role = Role.Button
                        onClick(closeLabel) { closeNow(); true }
                    }
                    .drawBehind {
                        val w = 36.dp.toPx()
                        val h = 4.dp.toPx()
                        // Where it always stood: centred in the 28 dp the sheet gives it.
                        drawRoundRect(c.ruleStrong, Offset((size.width - w) / 2, (GripRoom.toPx() - h) / 2), Size(w, h), CornerRadius(h / 2))
                    },
            )
            // Edge to edge: the sheet runs under the navigation bar and its content scrolls under
            // it too; the content pads its own end by [LocalSheetFoot] (inside its scroll).
            val foot = safeInsets.only(WindowInsetsSides.Bottom).asPaddingValues().calculateBottomPadding()
            Box(Modifier.windowInsetsPadding(safeInsets.only(WindowInsetsSides.Horizontal))) {
                CompositionLocalProvider(LocalSheetFoot provides foot) { content(visible) }
            }
        }
    }
}

/**
 * A docked sheet's title row: [leading] (a sub-page's Back), the title as a heading, then
 * [trailing]. No Close: the grip is the sheet's one way out besides a drag, a tap on the
 * page and Back.
 */
@Composable
fun SheetTitle(
    title: String,
    modifier: Modifier = Modifier,
    leading: (@Composable RowScope.() -> Unit)? = null,
    trailing: @Composable RowScope.() -> Unit = {},
) {
    val c = OscylTheme.colors
    Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).padding(start = if (leading != null) 4.dp else 16.dp, end = 4.dp).then(modifier), verticalAlignment = Alignment.CenterVertically) {
        leading?.invoke(this)
        BasicText(
            title,
            Modifier.weight(1f).semantics { heading() },
            style = MaterialTheme.typography.titleMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content, textAlign = startAlign()),
        )
        trailing()
    }
}

/** The grip's place in the sheet, and its touch target (the rest under the title row). */
private val GripRoom = 28.dp
private val GripTarget = 48.dp
