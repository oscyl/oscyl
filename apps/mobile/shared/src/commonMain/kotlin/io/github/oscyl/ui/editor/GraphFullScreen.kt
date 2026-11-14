package io.github.oscyl.ui.editor

import androidx.compose.ui.graphics.CompositingStrategy
import io.github.oscyl.ui.shell.LocalDestinationShown
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.Spacer
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.offset
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.backhandler.BackHandler
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.Velocity
import kotlin.math.abs
import kotlinx.coroutines.CoroutineStart
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.PagerState
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.graph_close
import io.github.oscyl.shared.generated.resources.graph_controls_less
import io.github.oscyl.shared.generated.resources.graph_controls_more
import io.github.oscyl.shared.generated.resources.graph_description
import io.github.oscyl.shared.generated.resources.graph_position
import io.github.oscyl.shared.generated.resources.ic_collapse
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.FloatingRadius
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.floating
import io.github.oscyl.ui.parts.motionScale
import io.github.oscyl.ui.parts.safeInsets
import io.github.oscyl.ui.shell.LocalIslandClearance
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.collectLatest
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.layoutId
import org.jetbrains.compose.resources.stringResource
import kotlin.math.roundToInt

/** The stages that have a picture: the full-screen graph pages through these. */
internal fun PanelState.graphPages(): List<Int> = pages.indices.filter { graph(it)?.now != null }

/**
 * A stage's graph grown to the whole page (DESIGN.md "Full-screen graph"): a header
 * (the close chevron, the stage and the program, Hold to compare), the picture filling
 * the page with larger handles and the value riding beside the finger, a pager across
 * the stages that have a picture, and the stage's controls on a sheet docked at the
 * foot that peeks one row and drags up for the rest. [page] is the stage shown;
 * paging reports the new one ([onPage]). A swipe down on the header, Back or the
 * chevron closes it ([onClose]). [frame] marks each picture for the container transform.
 * A picture being pinched or zoomed keeps the pager still: the fingers move the picture.
 */
@Composable
fun GraphFullScreen(
    head: EditorHead,
    panel: PanelState,
    page: Int,
    actions: EditorActions,
    onPage: (Int) -> Unit,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
    frame: @Composable (Int) -> Modifier = { Modifier },
    /** The page's own way in is over: its sheet composes its knobs. */
    settled: Boolean = true,
    pages: List<Int> = remember(panel) { panel.graphPages() },
    pager: PagerState = rememberPagerState(initialPage = pages.indexOf(page).coerceAtLeast(0)) { pages.size },
    /** How far the page has come in (0..1), read in the draw phase: its ground and header fade, its sheet rises. */
    reveal: () -> Float = { 1f },
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val report by rememberUpdatedState(onPage)
    LaunchedEffect(pager, pages) { snapshotFlow { pager.settledPage }.collect { pages.getOrNull(it)?.let(report) } }
    val shown = pages.getOrElse(pager.currentPage) { page }
    // The sheet's knobs belong to the stage the page shows: they fade out as a swipe leaves
    // it, change at the halfway mark (where they are invisible) and fade in as the next
    // picture lands. Waiting for the pager to settle left the sheet blank for its whole glide.
    val docked = shown
    val close by rememberUpdatedState(onClose)
    val density = androidx.compose.ui.platform.LocalDensity.current
    // The ground comes first (whole by half way), the header after it: at no frame do the
    // editor bar's words and this header's stand over each other.
    Column(modifier.fillMaxSize().drawBehind { drawRect(c.ground, alpha = (reveal() * 2f).coerceAtMost(1f)) }) {
        // Chrome: its words grow with the system font to 130%.
        io.github.oscyl.ui.shell.ChromeScale {
            Row(
                Modifier
                    .fillMaxWidth()
                    // Faded stroke by stroke (here and on the knobs below): a fade through an offscreen
                    // copy makes that copy, as large as what fades, in the first frame of the motion.
                    .graphicsLayer { alpha = ((reveal() - 0.5f) * 2f).coerceIn(0f, 1f); compositingStrategy = CompositingStrategy.ModulateAlpha }
                    .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))
                    .heightIn(min = 56.dp)
                    .padding(horizontal = 4.dp)
                    // A swipe down on the header puts the graph back.
                    .pointerInput(Unit) {
                        var pulled = 0f
                        detectVerticalDragGestures(
                            onDragStart = { pulled = 0f },
                            onDragEnd = { if (pulled > with(density) { 72.dp.toPx() }) close() },
                        ) { change, dy ->
                            change.consume()
                            pulled += dy
                        }
                    },
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconAction(Res.drawable.ic_collapse, stringResource(Res.string.graph_close), onClose, Modifier.testTag("graph-close"))
                Column(Modifier.weight(1f).padding(horizontal = 8.dp)) {
                    BasicText(
                        panel.pages[shown].title,
                        Modifier.semantics { heading() },
                        style = type.titleMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    BasicText(
                        "${head.name} · " + stringResource(Res.string.graph_position, pages.indexOf(shown) + 1, pages.size),
                        style = type.bodySmall.copy(color = c.legend, fontFeatureSettings = "tnum", textDirection = TextDirection.Content),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                CompareHold(head.comparing, compact = false, onCompare = actions::compare, short = true, onMorph = actions::morph, blend = { panel.blend })
                Spacer(Modifier.width(8.dp))
            }
        }
        BoxWithConstraints(Modifier.weight(1f).fillMaxWidth()) {
            val room = maxHeight
            var zooming by remember { mutableStateOf(false) }
            // The neighbours are composed ahead: a swipe only moves pictures that are there.
            HorizontalPager(pager, Modifier.fillMaxSize().testTag("graph pager"), key = { pages[it] }, userScrollEnabled = !zooming, beyondViewportPageCount = 1) { i ->
                val p = pages[i]
                val graph = panel.graph(p)
                if (graph?.now != null) {
                    StageGraphView(
                        graph,
                        panel.handles(p),
                        stringResource(Res.string.graph_description, panel.pages[p].title, panel.stage(p)?.summary.orEmpty()),
                        onDrag = { h, x, y -> actions.drag(p, h, x, y) },
                        onRelease = actions::release,
                        // Above the sheet's peek, so its handles stay clear of it.
                        modifier = Modifier.padding(start = 16.dp, top = 8.dp, end = 16.dp, bottom = SheetPeek + LocalIslandClearance.current + 16.dp).then(frame(p)),
                        height = null,
                        large = true,
                        onZooming = { zooming = it },
                        following = { panel.following },
                    )
                }
            }
            // The sheet comes up from the foot as the page comes in.
            // The stages a swipe can reach from here: their knobs are made ready while the pager rests.
            val around = remember(pages, pager.currentPage) { listOfNotNull(pages.getOrNull(pager.currentPage - 1), pages.getOrNull(pager.currentPage + 1)) }
            ControlsSheet(
                panel, docked, actions, room, settled,
                Modifier.align(Alignment.BottomCenter).graphicsLayer { translationY = (1f - reveal()) * SheetPeek.toPx() },
                around = around,
                still = { !pager.isScrollInProgress },
                dim = { 1f - (abs(pager.currentPageOffsetFraction) * 2f).coerceAtMost(1f) },
            )
        }
    }
}

/** How much of the controls sheet shows at rest: its grip and about a row of knobs. */
private val SheetPeek = 196.dp

/** The band at the sheet's head where its grip stands. */
private val SheetGrip = 28.dp

/**
 * The stage's controls on a sheet docked at the foot, edge to edge: it peeks a row.
 * It rises to its content, never past it: a drag up anywhere on it raises it until its
 * last row shows (at most to most of [room], where its list scrolls); at the list's top
 * a drag down lowers it again. A stage whose controls all show in the peek has nowhere
 * to rise: no grip is drawn, a drag moves nothing and a screen reader hears no action.
 * Its first row stands where every other stage's does. A tap on its grip, a tap on the picture above
 * it, Back, or the screen reader's action do the same. It moves by translation only
 * (nothing is measured again while it slides) and settles on the plate's easing, the
 * way a flick was going. [settled] says the page's own transition is over: its knobs
 * are composed then, never in a frame of the growth.
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun ControlsSheet(
    panel: PanelState,
    page: Int,
    actions: EditorActions,
    room: androidx.compose.ui.unit.Dp,
    settled: Boolean,
    modifier: Modifier,
    /** How clearly the knobs show (0..1, read in the draw phase): they dim while the pager turns to another stage. */
    dim: () -> Float = { 1f },
    /** The stages a swipe leads to: their knobs are composed and measured ahead, unseen, while the pager is [still]. */
    around: List<Int> = emptyList(),
    still: () -> Boolean = { true },
) {
    val c = OscylTheme.colors
    val motion = LocalOscylMotion.current
    val density = androidx.compose.ui.platform.LocalDensity.current
    val clearance = LocalIslandClearance.current
    val peek = with(density) { (SheetPeek + clearance).toPx() }
    // Each stage's controls as tall as they were last measured (the neighbours' ahead of a swipe).
    val heights = remember(panel) { mutableStateMapOf<Int, Int>() }
    // The grip, the controls and the foot's padding; not measured yet, it is taken to fit.
    val content = heights[page]?.let { it + with(density) { (SheetGrip + 12.dp + clearance).toPx() } } ?: peek
    // Something is below the peek: the sheet can rise. Less than a control's height there
    // hides no control (a single row's frame ends just under the peek): that sheet stays.
    val rises = content - peek >= with(density) { 48.dp.toPx() }
    val full = if (rises) minOf(with(density) { (room * 0.82f).toPx() }, content) else peek
    // How far the sheet rises above its peek.
    val span = full - peek
    var asked by rememberSaveable { mutableStateOf(false) }
    val raised = asked && rises
    // How far it stands above its peek: 0 peeking, [span] raised. (Counted from the peek, so a
    // stage with more or fewer controls coming in never moves a sheet that rests there.)
    val lift = remember { Animatable(0f) }
    val scope = rememberCoroutineScope()
    val flick = with(density) { 400.dp.toPx() }
    val scroll = rememberScrollState()
    fun settle(up: Boolean) {
        asked = up
        // Lowered, it shows its first row again.
        if (!up) scope.launch { if (motionScale(motion) == 0f) scroll.scrollTo(0) else scroll.animateScrollTo(0, tween(OscylTokens.slowMs, easing = OscylTokens.easing)) }
        scope.launch {
            val scale = motionScale(motion)
            val to = if (up) span else 0f
            if (scale == 0f) lift.snapTo(to) else lift.animateTo(to, tween((OscylTokens.slowMs * scale).roundToInt(), easing = OscylTokens.easing))
        }
    }
    /** Lets go at [velocity] px/s (down positive): where the flick was going, else the nearer rest. */
    fun letGo(velocity: Float) = settle(if (abs(velocity) > flick) velocity < 0f else lift.value > span / 2)
    /** Moves with the finger; returns what it took of [dy]. */
    fun pull(dy: Float): Float {
        val to = (lift.value - dy).coerceIn(0f, span)
        val taken = lift.value - to
        if (taken != 0f) scope.launch(start = CoroutineStart.UNDISPATCHED) { lift.snapTo(to) }
        return taken
    }
    LaunchedEffect(span) { lift.snapTo(if (raised) span else 0f) }
    // The list inside scrolls only once the sheet is raised, and lowers it from its top.
    val nested = remember(span) {
        object : NestedScrollConnection {
            override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset =
                if (source == NestedScrollSource.UserInput && available.y < 0f) Offset(0f, pull(available.y)) else Offset.Zero

            override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset =
                if (source == NestedScrollSource.UserInput && available.y > 0f) Offset(0f, pull(available.y)) else Offset.Zero

            override suspend fun onPreFling(available: Velocity): Velocity {
                if (lift.value >= span || lift.value <= 0f && available.y <= 0f) return Velocity.Zero
                letGo(available.y)
                return available
            }

            override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
                if (lift.value > 0f && lift.value < span) letGo(available.y)
                return Velocity.Zero
            }
        }
    }
    BackHandler(enabled = raised && LocalDestinationShown.current) { settle(false) }
    // Raised, the picture above waits behind a veil: a tap there lowers the sheet.
    if (raised) {
        Box(
            Modifier
                .fillMaxSize()
                .drawBehind { drawRect(c.shadow.copy(alpha = maxOf(c.shadow.alpha, 0.35f)), alpha = (lift.value / span.coerceAtLeast(1f)).coerceAtMost(1f) * 0.6f) }
                .pointerInput(Unit) { detectTapGestures { settle(false) } },
        )
    }
    val shape = RoundedCornerShape(topStart = FloatingRadius, topEnd = FloatingRadius)
    val toggleLabel = stringResource(if (raised) Res.string.graph_controls_less else Res.string.graph_controls_more)
    Column(
        modifier
            .fillMaxWidth()
            .height(with(density) { full.toDp() })
            .offset { IntOffset(0, (span - lift.value).coerceAtLeast(0f).roundToInt()) }
            .floating(c, shape, edgeToEdge = true)
            .testTag("controls-sheet")
            .nestedScroll(nested)
            .draggable(
                rememberDraggableState { pull(it) },
                Orientation.Vertical,
                enabled = rises,
                onDragStopped = { letGo(it) },
            ),
    ) {
        // The grip: drag it, or tap it, to raise or lower the sheet. Its band stays when
        // there is nothing to raise (the knobs keep their place); the grip itself does not.
        Box(
            Modifier
                .fillMaxWidth()
                .height(SheetGrip)
                .then(
                    if (!rises) Modifier else Modifier
                        .testTag("controls-grip")
                        .clickable(remember { MutableInteractionSource() }, indication = null, role = Role.Button, onClickLabel = toggleLabel) { settle(!raised) }
                        .drawBehind {
                            val w = 36.dp.toPx()
                            val h = 4.dp.toPx()
                            // With the knobs: it comes and goes where a swipe has them unseen.
                            drawRoundRect(c.ruleStrong, Offset((size.width - w) / 2, (size.height - h) / 2), Size(w, h), CornerRadius(h / 2), alpha = dim())
                        },
                ),
        )
        Column(
            Modifier
                .weight(1f)
                .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Horizontal))
                .verticalScroll(scroll)
                .padding(start = 16.dp, end = 16.dp, bottom = 12.dp + clearance),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            // Composed once the page has landed, then kept; they fade in on the plate's easing.
            var ready by remember { mutableStateOf(false) }
            if (settled) ready = true
            val shown by animateFloatAsState(if (ready) 1f else 0f, tween(if (motion) OscylTokens.fastMs else 0, easing = OscylTokens.easing))
            // The neighbours' knobs, made ready one section per frame once the pager rests and kept
            // beside the ones shown: the change at a swipe's halfway mark composes nothing (a
            // stage of knobs composed there cost that frame and more). How many of each
            // stage's sections are composed so far:
            val kept = remember(panel) { mutableStateMapOf<Int, Int>() }
            val wanted by rememberUpdatedState(around + page)
            LaunchedEffect(panel, ready) {
                if (!ready) return@LaunchedEffect
                snapshotFlow { wanted to still() }.collectLatest { (want, rest) ->
                    if (!rest) return@collectLatest
                    kept.keys.filter { it !in want }.forEach { kept.remove(it) }
                    for (p in want) {
                        val all = panel.pages[p].sections.size
                        while ((kept[p] ?: 0) < all) {
                            withFrameNanos {}
                            kept[p] = (kept[p] ?: 0) + 1
                        }
                    }
                }
            }
            // Keyed by stage: another stage's knobs start at their own values, never glide from these.
            if (ready) {
                val composed = (kept.keys + page).sorted()
                Layout(
                    content = {
                        composed.forEach { p ->
                            key(p) {
                                Column(Modifier.layoutId(p).onSizeChanged { heights[p] = it.height }.graphicsLayer { alpha = shown * dim(); compositingStrategy = CompositingStrategy.ModulateAlpha }, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                                    // The stage shown has every section, whatever was made ahead.
                                    val sections = panel.pages[p].sections.size
                                    repeat(if (p == page) sections else minOf(kept[p] ?: 0, sections)) { key(it) { SectionPanel(panel, p, it, actions) } }
                                }
                            }
                        }
                    },
                ) { measurables, constraints ->
                    // All are measured; only the stage shown is placed (the others draw nothing and take no touch).
                    val placed = measurables.map { it.layoutId to it.measure(constraints) }.first { it.first == page }.second
                    layout(placed.width, placed.height) { placed.place(0, 0) }
                }
            }
        }
    }
}
