package io.github.oscyl.ui.editor

import androidx.compose.ui.text.toUpperCase
import androidx.compose.ui.text.intl.Locale
import io.github.oscyl.shared.generated.resources.programs_new
import io.github.oscyl.ui.preferences.Hints
import io.github.oscyl.ui.preferences.LocalTeaching
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.foundation.Image
import org.jetbrains.compose.resources.painterResource
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.foundation.layout.size
import io.github.oscyl.shared.generated.resources.editor_copy_to_library
import io.github.oscyl.shared.generated.resources.play_short
import io.github.oscyl.shared.generated.resources.play_open
import io.github.oscyl.shared.generated.resources.ic_pad
import io.github.oscyl.shared.generated.resources.history_title
import io.github.oscyl.ui.shell.LocalShellNavigation
import io.github.oscyl.ui.shell.LocalDestinationShown
import androidx.compose.ui.platform.testTag
import io.github.oscyl.ui.preview.CutoutFrame
import io.github.oscyl.ui.preview.FakeCutout
import io.github.oscyl.ui.preview.LandscapeCutouts
import io.github.oscyl.ui.preview.PHONE_LANDSCAPE
import androidx.compose.animation.core.animateFloat
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.zIndex
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.animation.core.SeekableTransitionState
import androidx.compose.animation.core.rememberTransition
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.backhandler.PredictiveBackHandler
import io.github.oscyl.shared.generated.resources.ic_redo
import io.github.oscyl.shared.generated.resources.ic_undo
import io.github.oscyl.shared.generated.resources.programs_redo
import io.github.oscyl.shared.generated.resources.programs_undo
import androidx.compose.foundation.rememberScrollState
import io.github.oscyl.shared.generated.resources.destination_editor
import io.github.oscyl.ui.parts.CollapsingTopBar
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.EmptyArt
import io.github.oscyl.ui.parts.EmptyState
import io.github.oscyl.ui.parts.collapsingTitle
import io.github.oscyl.ui.parts.rememberCollapsingBarState
import io.github.oscyl.ui.shell.TopBarActions
import io.github.oscyl.ui.parts.safeInsets
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.animateScrollBy
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfoV2
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.window.core.layout.WindowSizeClass
import io.github.oscyl.core.AxisWord
import io.github.oscyl.core.ControlKind
import io.github.oscyl.core.FieldState
import io.github.oscyl.core.GraphLevel
import io.github.oscyl.core.GraphMark
import io.github.oscyl.core.GraphShape
import io.github.oscyl.core.GraphTick
import io.github.oscyl.core.HandleInfo
import io.github.oscyl.core.MarkStyle
import io.github.oscyl.core.PageInfo
import io.github.oscyl.core.SectionInfo
import io.github.oscyl.core.StageGraph
import io.github.oscyl.core.StageState
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.editor_empty_action
import io.github.oscyl.shared.generated.resources.editor_empty_body
import io.github.oscyl.shared.generated.resources.editor_empty_title
import io.github.oscyl.shared.generated.resources.editor_not_read
import io.github.oscyl.shared.generated.resources.program_reading
import io.github.oscyl.shared.generated.resources.editor_read_only
import io.github.oscyl.shared.generated.resources.editor_saved_version
import io.github.oscyl.shared.generated.resources.editor_slot
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.library.LibraryScreen
import io.github.oscyl.ui.library.NoLibraryActions
import io.github.oscyl.ui.library.SampleUi
import io.github.oscyl.ui.preview.PHONE
import androidx.compose.ui.tooling.preview.Preview
import io.github.oscyl.ui.parts.Legend
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.StageItem
import io.github.oscyl.ui.parts.StageRail
import io.github.oscyl.ui.parts.motionScale
import io.github.oscyl.ui.parts.stageIcon
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.OscylSizePreviews
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.shell.Destination
import io.github.oscyl.ui.shell.LocalIslandClearance
import io.github.oscyl.ui.parts.StageRailRestWidth
import io.github.oscyl.ui.shell.FootSide
import io.github.oscyl.ui.shell.ReserveFoot
import io.github.oscyl.ui.shell.Shell
import kotlin.math.PI
import kotlin.math.exp
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlinx.coroutines.launch
import org.jetbrains.compose.resources.stringResource

/**
 * The Editor destination. In an expanded window (tablets, unfolded), the sources and
 * the program list stand beside the editor, as on the desktop; elsewhere the editor
 * has the window to itself.
 */
@Composable
fun EditorDestination(editor: EditorModel, library: LibraryModel, onOpenLibrary: () -> Unit, onOpenSynth: () -> Unit, modifier: Modifier = Modifier) {
    val head by editor.head.collectAsStateWithLifecycle()
    val libraryUi by library.ui.collectAsStateWithLifecycle()
    // A program can be made where a library is shown.
    val onNew = if (libraryUi.programs?.inLibrary == true) library::newAndOpen else null
    val expanded = currentWindowAdaptiveInfoV2().windowSizeClass.isWidthAtLeastBreakpoint(WindowSizeClass.WIDTH_DP_EXPANDED_LOWER_BOUND)
    if (expanded) {
        SideBySide(
            { LibraryScreen(library, onOpenSynth = onOpenSynth, modifier = it) },
            { m, alone -> EditorScreen(head, editor.panel, editor, onOpenLibrary = onOpenLibrary.takeIf { alone }, modifier = m, onNew = onNew) },
            modifier,
        )
    } else {
        EditorScreen(head, editor.panel, editor, onOpenLibrary, modifier, onNew)
    }
}

/**
 * The sources and the program list beside the editor, parted by a rule, where the
 * space leaves the editor room for its dials ([SideBySideMin]); narrower (an unfolded
 * foldable, a small tablet), the editor has it alone, as in a medium window.
 */
@Composable
private fun SideBySide(sources: @Composable (Modifier) -> Unit, editor: @Composable (Modifier, alone: Boolean) -> Unit, modifier: Modifier) {
    BoxWithConstraints(modifier.fillMaxSize()) {
        if (maxWidth < SideBySideMin) {
            editor(Modifier.fillMaxSize(), true)
        } else {
            Row(Modifier.fillMaxSize()) {
                sources(Modifier.width(SourcesWidth))
                Box(Modifier.width(OscylTokens.ruleWidth).fillMaxHeight().background(OscylTheme.colors.rule))
                editor(Modifier.weight(1f), false)
            }
        }
    }
}

/** The sources and list panes' width, and the least room for them and the editor together. */
private val SourcesWidth = 560.dp
private val SideBySideMin = 1100.dp

/** The editor pane width from which the stage rail stands open with titles. */
private val OpenRailMin = 560.dp

/**
 * The Front Panel editor (UX brief, "Editor"; DESIGN.md "Mobile"). A phone held
 * upright, or any taller window: the collapsing top bar (undo, redo, the chip and the menu trailing; the slot
 * legend and Albert Sans title rise into it as they leave, with the stage in view as
 * its subtitle), then every stage in one continuous scroll (heading and summary,
 * framed picture with handles, section panels). The stage rail floats low at the end
 * edge, its foot 16 dp above the island, with Hold to compare under it; panels end at
 * its edge. A stage's picture grows into the full-screen graph by container transform
 * (Back, partway or whole, drives it home). A phone on its side gets the stage view.
 * With no program open, a way to Library.
 */
@Composable
fun EditorScreen(
    head: EditorHead?,
    panel: PanelState?,
    actions: EditorActions,
    onOpenLibrary: (() -> Unit)?,
    modifier: Modifier = Modifier,
    /** With nothing open: make a program in the library shown and open it (null: no library to make it in). */
    onNew: (() -> Unit)? = null,
) {
    if (head == null || panel == null) {
        Empty(onOpenLibrary, modifier.fillMaxSize(), onNew)
        return
    }
    // The stage grown to full screen, and the stage view's stage. Both outlive rotation
    // and the process, so each is kept with the program it was said of: saved state comes
    // back whatever program is open by then (another one, after this screen left and
    // returned), and a graph page restored onto a program without that picture hid the
    // list under nothing. A value for another program, or out of this one's range, is no value.
    val program = "${head.model}|${head.slot}|${head.name}"
    var openOf by rememberSaveable { mutableStateOf<Pair<String, Int>?>(null) }
    var stageOf by rememberSaveable { mutableStateOf<Pair<String, Int>?>(null) }
    val pictured = remember(panel) { panel.graphPages() }
    val open = openOf?.takeIf { it.first == program && it.second in pictured }?.second
    val stage = stageOf?.takeIf { it.first == program && it.second in panel.pages.indices }?.second ?: panel.firstPage
    // Another program is here: what was said of the last one is dropped, not kept for its return.
    LaunchedEffect(program) {
        if (openOf?.first != program) openOf = null
        if (stageOf?.first != program) stageOf = null
    }
    BoxWithConstraints(modifier.fillMaxSize()) {
        if (maxHeight < StageViewMaxHeight && maxWidth > maxHeight) {
            StageView(head, panel, actions, open ?: stage, onStage = { stageOf = program to it; if (open != null) openOf = program to it }, Modifier.fillMaxSize())
        } else {
            Portrait(head, panel, actions, open, onOpen = { openOf = it?.let { p -> program to p } }, compact = maxWidth < OpenRailMin, room = maxWidth)
        }
    }
}

/** Viewports of panels kept composed before and after the page: more than any program is long. */
@OptIn(androidx.compose.foundation.ExperimentalFoundationApi::class)
private val EditorCacheWindow = androidx.compose.foundation.lazy.layout.LazyLayoutCacheWindow(aheadFraction = 40f, behindFraction = 40f)

/** Below this height (a phone on its side) the editor is the stage view. */
private val StageViewMaxHeight = 480.dp

@OptIn(ExperimentalComposeUiApi::class)
@Composable
private fun Portrait(head: EditorHead, panel: PanelState, actions: EditorActions, open: Int?, onOpen: (Int?) -> Unit, compact: Boolean, room: Dp = Dp.Infinity) {
    // The whole program stays composed around the page: its panels are composed in idle
    // time after it opens and kept, so a fling, a jump from the rail or the way back from
    // a full-screen graph lays out and draws but never composes a panel of knobs in a frame.
    @OptIn(androidx.compose.foundation.ExperimentalFoundationApi::class)
    val list = rememberLazyListState(cacheWindow = EditorCacheWindow)
    // Items: the title, then each stage's heading, picture and sections.
    val items = remember(panel) {
        buildList {
            add(-1 to "title")
            panel.pages.forEachIndexed { p, page ->
                add(p to "head")
                // A stage without a picture has no picture item (an empty item doubled its gap).
                if (panel.graph(p)?.now != null) add(p to "graph")
                page.sections.indices.forEach { add(p to "section:$it") }
            }
        }
    }
    // The container transform: a seekable transition, so predictive Back drives it.
    val seek = remember { SeekableTransitionState(open) }
    val openNow by rememberUpdatedState(open)
    val motionNow by rememberUpdatedState(LocalOscylMotion.current)
    LaunchedEffect(open) {
        if (open == null) {
            // Its picture is laid out in the list again before it lands there.
            val at = items.indexOfFirst { it.first == seek.currentState && it.second == "graph" }
            if (at >= 0 && list.layoutInfo.visibleItemsInfo.none { it.index == at }) list.requestScrollToItem(at)
        }
        if (motionScale(motionNow) == 0f) seek.snapTo(open) else seek.animateTo(open)
    }
    // Only while this destination is the one shown: kept composed under another, it must not take its Back.
    PredictiveBackHandler(enabled = open != null && LocalDestinationShown.current) { progress ->
        try {
            progress.collect { seek.seekTo(it.progress.coerceIn(0f, 1f), targetState = null) }
            onOpen(null)
        } catch (e: kotlin.coroutines.cancellation.CancellationException) {
            seek.animateTo(openNow)
            throw e
        }
    }
    val transition = rememberTransition(seek, label = "graph")
    // The full-screen graph's pages: the stages with a picture.
    val pages = remember(panel) { panel.graphPages() }
    val pager = androidx.compose.foundation.pager.rememberPagerState(initialPage = pages.indexOf(open).coerceAtLeast(0)) { pages.size }
    val pagerScope = rememberCoroutineScope()
    // Kept across the graph's visit: the list comes back at its own width, never reflowing in from the side.
    val railWidth = remember(compact) { androidx.compose.runtime.mutableIntStateOf(0) }
    val motion = LocalOscylMotion.current
    // How far the graph's page has come in over the list: 0 the list alone, 1 the page alone.
    val cover = transition.animateFloat(
        { tween(if (!motion) 0 else if (targetState != null) OscylTokens.baseMs else OscylTokens.fastMs, easing = OscylTokens.easing) },
        label = "cover",
    ) { if (it != null) 1f else 0f }
    // The container transform, by hand: each stage's picture as the list has it and as the
    // page has it (px in the window). On the way, the page's picture is moved and stretched
    // from the one to the other on its layer; nothing is measured or drawn again per frame.
    val inList = remember(panel) { HashMap<Int, Rect>() }
    val inPage = remember(panel) { HashMap<Int, Rect>() }
    // The page is composed a moment after the program opens and kept, hidden under the
    // list: a tap on a picture starts its growth at once, with nothing to compose.
    var warm by remember { mutableStateOf(open != null) }
    LaunchedEffect(Unit) {
        repeat(2) { androidx.compose.runtime.withFrameNanos {} }
        warm = true
    }
    // The stage the page shows, also on its way out.
    var last by remember(panel) { androidx.compose.runtime.mutableIntStateOf(open ?: pages.firstOrNull() ?: 0) }
    if (open != null) last = open
    val up = transition.targetState != null || transition.currentState != null
    // Only the stages on screen keep their pictures current while values move. One owner
    // of that set: the list's visible stages, and while the graph's page is up (the list
    // lies composed under it) also the stage the pager shows and the neighbours it keeps
    // composed. Without them a stage reached by a swipe stood still under its own drag.
    var listShown by remember(panel) { mutableStateOf(emptySet<Int>()) }
    LaunchedEffect(panel) {
        snapshotFlow {
            val paged = seek.targetState != null || seek.currentState != null
            if (!paged) {
                listShown
            } else {
                val around = listOf(pager.currentPage, pager.targetPage, pager.settledPage).flatMap { listOf(it - 1, it, it + 1) }
                listShown + around.mapNotNull { pages.getOrNull(it) }
            }
        }.collect { actions.showing(it) }
    }
    Box(Modifier.fillMaxSize()) {
        EditorList(
            head, panel, actions, list, items, compact, room,
            onExpand = { p ->
                // The page turns to this stage before it shows: its picture is the one that grows.
                val at = pages.indexOf(p)
                if (at >= 0 && at != pager.currentPage) pagerScope.launch(start = kotlinx.coroutines.CoroutineStart.UNDISPATCHED) { pager.scrollToPage(at) }
                onOpen(p)
            },
            frame = { p ->
                Modifier
                    .onGloballyPositioned { inList[p] = it.boundsInRoot() }
                    // While the page's picture is out, it is the only one.
                    .graphicsLayer { alpha = if (p == last && cover.value > 0f && inPage[p] != null) 0f else 1f }
            },
            railWidthState = railWidth,
            onShowing = { listShown = it },
            modifier = Modifier
                .drawWithContent { if (cover.value < 1f) drawContent() }
                // Under the graph's page it is not there for a screen reader or a key either.
                .then(if (transition.targetState != null) Modifier.clearAndSetSemantics {} else Modifier),
        )
        if (warm && pages.isNotEmpty()) {
            val settled by remember { derivedStateOf { cover.value == 1f } }
            GraphFullScreen(
                head, panel, last, actions,
                onPage = { if (openNow != null) onOpen(it) },
                onClose = { onOpen(null) },
                settled = settled,
                pager = pager,
                reveal = { cover.value },
                modifier = Modifier
                    // Hidden, it lies under the list: touches and screen readers find the list.
                    .zIndex(if (up) 1f else -1f)
                    .drawWithContent { if (cover.value > 0f) drawContent() }
                    .then(if (up) Modifier.pointerInput(Unit) {} else Modifier.clearAndSetSemantics {}),
                frame = { p ->
                    Modifier
                        .onGloballyPositioned { inPage[p] = it.boundsInRoot() }
                        .graphicsLayer {
                            val t = cover.value
                            val from = inList[p]
                            val to = inPage[p]
                            if (t < 1f && p == last && from != null && to != null && to.width > 0f && to.height > 0f) {
                                transformOrigin = androidx.compose.ui.graphics.TransformOrigin(0f, 0f)
                                scaleX = from.width / to.width + (1f - from.width / to.width) * t
                                scaleY = from.height / to.height + (1f - from.height / to.height) * t
                                translationX = (from.left - to.left) * (1f - t)
                                translationY = (from.top - to.top) * (1f - t)
                            }
                        }
                },
            )
        }
    }
}

@Composable
private fun EditorList(
    head: EditorHead,
    panel: PanelState,
    actions: EditorActions,
    list: androidx.compose.foundation.lazy.LazyListState,
    items: List<Pair<Int, String>>,
    compact: Boolean,
    room: Dp,
    onExpand: (Int) -> Unit,
    frame: @Composable (Int) -> Modifier,
    railWidthState: androidx.compose.runtime.MutableIntState,
    /** The stages its visible items show. */
    onShowing: (Set<Int>) -> Unit,
    modifier: Modifier = Modifier,
) {
    val scope = rememberCoroutineScope()
    val motion = LocalOscylMotion.current
    val density = LocalDensity.current
    val bar = rememberCollapsingBarState(list)
    val starts = remember(items) { panel.pages.indices.map { p -> items.indexOfFirst { it.first == p } } }
    // The stage being read: the last whose heading has risen into the top third of the
    // page, else the one the page opens on.
    val current by remember(items) {
        derivedStateOf {
            val info = list.layoutInfo
            val line = info.viewportStartOffset + (info.viewportEndOffset - info.viewportStartOffset) / 3
            val visible = info.visibleItemsInfo
            val head = visible.lastOrNull { items.getOrNull(it.index)?.second == "head" && it.offset <= line }
            val item = head ?: visible.firstOrNull()
            (item?.index?.let { items.getOrNull(it)?.first } ?: 0).coerceAtLeast(0)
        }
    }
    // The stages in the list's view; its owner (the page) says which keep their pictures current.
    LaunchedEffect(panel) {
        snapshotFlow { list.layoutInfo.visibleItemsInfo.mapNotNull { items.getOrNull(it.index)?.first?.takeIf { p -> p >= 0 } }.toSet() }
            .collect { onShowing(it) }
    }
    // The program opens on the driver's first stage (the oscillators).
    LaunchedEffect(panel) { if (panel.firstPage > 0 && list.firstVisibleItemIndex == 0 && list.firstVisibleItemScrollOffset == 0) list.scrollToItem(starts[panel.firstPage]) }
    var railWidth by railWidthState
    val clearance = LocalIslandClearance.current
    val compare = remember { CompareState() }
    // Something to compare: edits on a program that can change (and, while comparing, still so).
    val canCompare = head.loaded && !head.readOnly &&
        (head.comparing || panel.pages.indices.any { (panel.stage(it)?.edits ?: 0u) > 0u })
    // The rail rests open (its stages' names shown) on the first look at the editor: six
    // drawings are not words until they have been. A touch on it keeps it open, and it closes
    // once it has taken the page somewhere; a first touch on the page itself closes it too.
    val teaching = LocalTeaching.current
    var railTaught by remember { mutableStateOf(teaching.taught(RAIL_HINT)) }
    val railUsed = {
        if (!railTaught) {
            railTaught = true
            teaching.teach(RAIL_HINT)
        }
    }
    // Gliding along the rail, taught once by the open rail's own plate (a nod) until done.
    var glideTaught by remember { mutableStateOf(teaching.taught(Hints.RAIL_GLIDE)) }
    // The stage a tap on the rail is gliding to, until the list lands there.
    var jumpingTo by remember { mutableStateOf<Int?>(null) }

    Box(modifier.fillMaxSize()) {
        Column(
            // The page under the rail, not the rail: a touch on the rail never reaches here.
            Modifier.fillMaxSize().then(
                if (railTaught) Modifier else Modifier.pointerInput(Unit) {
                    awaitEachGesture {
                        awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
                        railUsed()
                    }
                },
            ),
        ) {
            CollapsingTopBar(
                bar,
                title = if (head.loaded) head.name else stringResource(Res.string.program_reading),
                // While comparing the bar says so, wherever the page stands.
                subtitle = if (head.comparing) stringResource(Res.string.editor_saved_version) else panel.pages.getOrNull(current)?.title,
                // The one rule of every bar: the page's own actions, the chip, the menu (no parent here: no Back).
                trailing = {
                    val history = LocalShellNavigation.current.history
                    // Held, undo opens the history: every step, and a tap to jump to one.
                    IconAction(
                        Res.drawable.ic_undo, stringResource(Res.string.programs_undo), actions::undo, enabled = head.canUndo,
                        onLongClick = history, onLongClickLabel = stringResource(Res.string.history_title),
                    )
                    IconAction(Res.drawable.ic_redo, stringResource(Res.string.programs_redo), actions::redo, enabled = head.canRedo)
                    TopBarActions(compact = true, history = true)
                },
            )
            LazyColumn(
                Modifier.fillMaxSize().testTag("editor-list").windowInsetsPadding(safeInsets.only(WindowInsetsSides.Horizontal)),
                state = list,
                contentPadding = run {
                    // The rail and its margins stay clear. On a phone that is the rail at rest, fixed:
                    // the first look's grown rail (its names taught) floats over the panels, which never reflow.
                    val end = if (compact) StageRailRestWidth + RailAir else maxOf(16.dp, with(density) { railWidth.toDp() })
                    // A tablet's panels keep a reading width, centred in the room beside the rail.
                    val spare = if (room == Dp.Infinity) 0.dp else ((room - 16.dp - end - EditorMaxWidth) / 2).coerceAtLeast(0.dp)
                    PaddingValues(start = 16.dp + spare, end = end + spare, bottom = 24.dp + clearance)
                },
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                items.forEach { (page, kind) ->
                    // Keyed by program too: another program's graphs start unzoomed.
                    item(key = "${panel.id}:$page:$kind") {
                        when {
                            page < 0 -> Title(head, panel.comparing, Modifier.collapsingTitle(bar), actions::copyToLibrary)
                            kind == "head" -> StageHeading(panel, page, onRoll = { actions.roll(page) }.takeIf { !head.readOnly && head.loaded }, onRollTapped = { actions.rollTapped(page) })
                            kind == "graph" -> StageGraphSlot(panel, page, actions, onExpand = { onExpand(page) }, frame = frame(page))
                            else -> SectionPanel(panel, page, kind.removePrefix("section:").toInt(), actions)
                        }
                    }
                }
            }
        }
        // The rail floats low at the end edge, in thumb reach, Hold to compare at its foot:
        // a notice ends before it, so the Compare part a hint names stays in sight.
        ReserveFoot(FootSide.End, 12.dp + StageRailRestWidth + 8.dp)
        Column(
            Modifier
                .align(Alignment.BottomEnd)
                .windowInsetsPadding(safeInsets.only(WindowInsetsSides.End + WindowInsetsSides.Top))
                // The island's end margin: the rail stands in one column with the Save part under it.
                .padding(top = 64.dp, end = 12.dp, bottom = clearance + OscylTokens.railAnchor)
                // Its resting width: held, the rail grows over the panels, which never reflow for it.
                .onSizeChanged { val w = it.width + with(density) { RailAir.roundToPx() }; if (railWidth == 0 || w < railWidth) railWidth = w },
            horizontalAlignment = Alignment.End,
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Rail(
                panel,
                { jumpingTo ?: current },
                onJump = { i ->
                    // The plate goes straight to the tapped stage and stays while the list
                    // glides past the others, rather than stepping through each of them.
                    jumpingTo = i
                    scope.launch {
                        try {
                            list.glideTo(starts[i], motionScale(motion))
                        } finally {
                            jumpingTo = null
                            // It has taken the page somewhere: the open rail closes around its plate.
                            railUsed()
                        }
                    }
                },
                onScrub = { i ->
                    if (!glideTaught) {
                        glideTaught = true
                        teaching.teach(Hints.RAIL_GLIDE)
                    }
                    scope.launch { list.scrollToItem(starts[i]) }
                },
                modifier = Modifier,
                open = !compact || !railTaught,
                roomy = !compact,
                landing = jumpingTo != null,
                hint = !glideTaught && head.loaded && LocalDestinationShown.current,
                // A plain tap: the line that says the rail can be glided along, once.
                onTap = actions::railTapped,
            )
            RailCompare(
                head.comparing, actions::compare, actions::morph, { panel.blend },
                state = compare,
                enabled = canCompare,
                onShortTap = actions::compareTapped,
            )
        }
        // The fader beside Compare: its own fixed place, composed once, shown while held.
        CompareFader(
            compare,
            { panel.blend },
            Modifier
                .align(Alignment.BottomEnd)
                .windowInsetsPadding(safeInsets.only(WindowInsetsSides.End))
                .padding(end = 12.dp + 48.dp + 8.dp, bottom = clearance + OscylTokens.railAnchor),
        )
    }
}

/** The one-time look at the stage rail's names (Preferences keep it shown once per install). */
private const val RAIL_HINT = "stage-rail"

/** The widest the panels run (a tablet's window is far wider than a stage needs). */
private val EditorMaxWidth = 840.dp

/** Between the rail and the panels beside it. */
private val RailAir = 20.dp

/** The stage rail over the panel's stages; it alone redraws as summaries and counts move. */
@Composable
internal fun Rail(
    panel: PanelState,
    current: () -> Int,
    onJump: (Int) -> Unit,
    onScrub: (Int) -> Unit,
    modifier: Modifier,
    open: Boolean,
    roomy: Boolean = false,
    landing: Boolean = false,
    hint: Boolean = false,
    onTap: () -> Unit = {},
    atStart: Boolean = false,
) {
    val stages = panel.pages.mapIndexed { i, page ->
        val s = panel.stage(i)
        StageItem(page.title, s?.shortSummary.orEmpty(), s?.edits?.toInt() ?: 0, stageIcon(page.icon))
    }
    StageRail(stages, current(), onJump, onScrub, modifier, open, roomy, landing, hint, onTap, atStart)
}

/**
 * Glide to item [index] over 320 ms on the one easing (the desktop's jump), from an
 * estimate of the distance when it is out of view, then settle on it exactly.
 */
private suspend fun LazyListState.glideTo(index: Int, scale: Float) {
    if (scale == 0f) return scrollToItem(index)
    val target = layoutInfo.visibleItemsInfo.firstOrNull { it.index == index }
    // Off screen, one glide that finds its target on the way (an estimate and then a snap
    // to correct it read as two jumps).
    if (target == null) return animateScrollToItem(index)
    animateScrollBy(target.offset.toFloat(), tween((320 * scale).roundToInt(), easing = OscylTokens.easing))
}

/** The slot legend, the program title and, while comparing, the "Saved version" tag (its space kept). */
@Composable
private fun Title(head: EditorHead, comparing: Boolean, modifier: Modifier = Modifier, copyToLibrary: () -> Unit = {}) {
    val c = OscylTheme.colors
    val motion = LocalOscylMotion.current
    val tag by animateFloatAsState(if (comparing) 1f else 0f, tween(if (motion) OscylTokens.baseMs else 0, easing = OscylTokens.easing))
    Column(modifier.fillMaxWidth().padding(top = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        // The slot legend, and at its end the way to the play page, with its word.
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            // While comparing, "SAVED VERSION" in ink takes the slot legend's place (one place, nothing moves).
            // One line in every language (a long translation of Perform beside it cuts it short):
            // the title block keeps one height, and comparing swaps one line for one line.
            Box(Modifier.weight(1f).padding(end = 8.dp)) {
                Legend(stringResource(Res.string.editor_slot, head.slot, head.model), Modifier.graphicsLayer { alpha = 1f - tag }.then(if (comparing) Modifier.clearAndSetSemantics {} else Modifier), maxLines = 1)
                BasicText(
                    stringResource(Res.string.editor_saved_version).toUpperCase(Locale.current),
                    Modifier.graphicsLayer { alpha = tag }.then(if (comparing) Modifier else Modifier.clearAndSetSemantics {}),
                    style = MaterialTheme.typography.labelMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold),
                    maxLines = 1,
                    overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                )
            }
            OutlinedAction(
                stringResource(Res.string.play_short), LocalShellNavigation.current.play, Modifier.testTag("play-open"),
                leading = { Image(painterResource(Res.drawable.ic_pad), null, Modifier.size(18.dp), colorFilter = ColorFilter.tint(OscylTheme.colors.ink)) },
            )
        }
        run {
            BasicText(
                // A slot not read yet holds nothing known: no placeholder name.
                if (head.loaded) head.name else stringResource(Res.string.program_reading),
                // The bar's title is the page's heading, wherever it shows.
                Modifier.clearAndSetSemantics {},
                style = OscylTheme.type.programTitle.copy(color = c.ink, textDirection = TextDirection.Content),
            )
        }
        if (head.readOnly) {
            BasicText(stringResource(Res.string.editor_read_only), style = MaterialTheme.typography.bodyMedium.copy(color = c.legend))
            OutlinedAction(stringResource(Res.string.editor_copy_to_library), copyToLibrary)
        } else if (!head.loaded) {
            BasicText(stringResource(Res.string.editor_not_read), style = MaterialTheme.typography.bodyMedium.copy(color = c.legend))
        }
    }
}

/** No program open: the bar every destination has (the chip, the menu), the panel at rest, and the way to the Library. */
@Composable
private fun Empty(onOpenLibrary: (() -> Unit)?, modifier: Modifier, onNew: (() -> Unit)? = null) {
    Column(modifier) {
        // Nothing scrolls here: the bar stays the page, its title said for screen readers.
        CollapsingTopBar(rememberCollapsingBarState(rememberScrollState()), stringResource(Res.string.destination_editor), subtitle = null, trailing = { TopBarActions() })
        Box(
            Modifier.fillMaxSize().windowInsetsPadding(safeInsets.only(WindowInsetsSides.Horizontal)).padding(bottom = LocalIslandClearance.current),
            contentAlignment = Alignment.Center,
        ) {
            EmptyState(
                stringResource(Res.string.editor_empty_title),
                stringResource(Res.string.editor_empty_body),
                art = EmptyArt.Panel,
                action = if (onOpenLibrary == null && onNew == null) null else ({
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        onOpenLibrary?.let { OutlinedAction(stringResource(Res.string.editor_empty_action), it) }
                        onNew?.let { OutlinedAction(stringResource(Res.string.programs_new), it, Modifier.testTag("editor-new")) }
                    }
                }),
            )
        }
    }
}

// Sample state for the previews and the device checks: plain values, no model, no core calls.

private fun dial(f: Int, label: String, raw: Int, saved: Int = raw, max: Int = 1023, large: Boolean = false) = FieldState(
    f.toUInt(), label, ControlKind.Dial(0u, max.toUInt(), large), raw.toUInt(), saved.toUInt(), null, null,
    raw.toString(), saved.toString(), relevant = true, edited = raw != saved, help = "",
)

private fun choice(f: Int, label: String, options: List<String>, option: Int, saved: Int = option, menu: Boolean = false) = FieldState(
    f.toUInt(), label, if (menu) ControlKind.Menu(options) else ControlKind.Segments(options), option.toUInt(), saved.toUInt(),
    option.toUInt(), saved.toUInt(), options[option], options[saved], relevant = true, edited = option != saved, help = "",
)

private fun section(title: String?, vararg fields: Int) = SectionInfo(title, fields.map { it.toUInt() })

/** A low-pass response: flat, a resonant bump at [cutoff], then the slope. */
private fun response(cutoff: Float, res: Float): GraphShape {
    val pts = (0..96).flatMap { i ->
        val x = i / 96f
        val d = (x - cutoff) * 12f
        val y = if (x < cutoff) 0.7f + res * exp(-d * d) else (0.7f + res * exp(-d * d)) - (x - cutoff) * 2.2f
        listOf(x, y.coerceIn(0f, 1f))
    }
    return GraphShape(
        1u, listOf(GraphMark(MarkStyle.AREA, pts)), listOf(0f, 0.7f), cutoff, true,
        listOf(GraphTick(0.33f, AxisWord.HZ100, 0.33f, 1f), GraphTick(0.67f, AxisWord.KHZ1, 0.67f, 1f), GraphTick(1f, AxisWord.KHZ10, 1f, 1f)),
        emptyList(), time = false, pitch = false,
    )
}

private val Wave = GraphShape(
    0u, listOf(GraphMark(MarkStyle.LINE, (0..96).flatMap { i -> val x = i / 96f; listOf(x, 0.5f + 0.4f * sin(2 * PI.toFloat() * 2 * x)) })),
    listOf(0.5f), null, false, emptyList(), emptyList(), time = false, pitch = false,
)

private val Envelope = GraphShape(
    2u, listOf(GraphMark(MarkStyle.LINE, listOf(0f, 0f, 0.08f, 1f, 0.3f, 0.6f, 0.7f, 0.6f, 1f, 0f))), listOf(0f), null, true,
    listOf(
        GraphTick(0f, AxisWord.ATTACK, 0.04f, 0.08f), GraphTick(0.08f, AxisWord.DECAY, 0.19f, 0.22f),
        GraphTick(0.3f, AxisWord.SUSTAIN, 0.5f, 0.4f), GraphTick(0.7f, AxisWord.RELEASE, 0.85f, 0.3f), GraphTick(1f, null, 1f, 1f),
    ),
    emptyList(), time = false, pitch = false,
)

private val Levels = GraphShape(
    3u, listOf(0.8f, 0.55f, 0.2f).mapIndexed { i, l -> GraphMark(MarkStyle.BAR, listOf(i / 3f + 0.066f, l, i / 3f + 0.266f, 0f)) },
    emptyList(), null, false, emptyList(),
    listOf(GraphLevel(0f, AxisWord.PERCENT0), GraphLevel(0.5f, AxisWord.PERCENT50), GraphLevel(1f, AxisWord.PERCENT100)), time = false, pitch = false,
)

/** A minilogue xd–like program in the editor, the filter edited, as in the Front Panel sketch. */
fun samplePanel(longNames: Boolean = false, start: Int = 0): PanelState {
    val pages = listOf(
        PageInfo("Voice", "oscyl-voice-symbolic", "", listOf(section(null, 0, 1))),
        PageInfo("Oscillators", "oscyl-oscillator-symbolic", "", listOf(section("VCO 1", 2, 3, 4), section("VCO 2", 5, 6))),
        PageInfo("Mixer", "oscyl-mixer-symbolic", "", listOf(section(null, 7, 8, 9))),
        PageInfo("Filter", "oscyl-filter-symbolic", "", listOf(section(null, 10, 11, 12, 13))),
        PageInfo("Envelopes", "oscyl-envelope-symbolic", "", listOf(section(if (longNames) "Amplifier envelope generator and its velocity" else "Amp EG", 14, 15, 16, 17))),
        PageInfo("Effects", "oscyl-effects-symbolic", "", listOf(section("Delay", 18, 19, 20))),
    )
    val sub = if (longNames) "Sub-oscillator waveform selection" else "Wave"
    val fields = listOf(
        choice(0, "Voice mode", listOf("Poly", "Unison", "Chord", "Arp"), 0),
        dial(1, "Depth", 512),
        choice(2, sub, listOf("Saw", "Triangle", "Square"), 0),
        dial(3, "Pitch", 512, large = true), dial(4, "Shape", 300, saved = 120),
        choice(5, "Octave", listOf("16'", "8'", "4'", "2'"), 1),
        dial(6, "Shape", 0),
        dial(7, "VCO 1", 1023, large = true), dial(8, "VCO 2", 640), dial(9, "Multi", 0),
        dial(10, "Cutoff", 618, saved = 431, large = true), dial(11, "Resonance", 347), dial(12, "EG Int", 512),
        choice(13, "Drive", listOf("0%", "50%", "100%"), 1, saved = 0),
        dial(14, "Attack", 12, large = true), dial(15, "Decay", 400), dial(16, "Sustain", 700), dial(17, "Release", 640),
        choice(18, "Delay type", listOf("Stereo", "Mono", "Ping pong", "Hi-pass", "Tape", "One tap", "Stereo BPM", "Mono BPM"), 2, menu = true),
        dial(19, "Time", 380, large = true), dial(20, "Depth", 210),
    )
    val stages = listOf(
        StageState("Poly · 8 voices", "Poly", 0u, listOf(true)),
        StageState("Saw + Sqr · Ring", "Saw + Sqr", 1u, listOf(true, true)),
        StageState("VCO 1 100% · VCO 2 63%", "100 · 63", 0u, listOf(true)),
        StageState("LP 2-pole · 618 · res 34%", "LP · 618", 2u, listOf(true)),
        StageState("A 12 · D 400 · S 70% · R 640", "A 12 · R 640", 0u, listOf(true)),
        StageState("Ping pong · 380 ms", "Ping pong", 0u, listOf(true)),
    )
    val graphs = listOf(
        StageGraph(null, null), StageGraph(Wave, null), StageGraph(Levels, null),
        StageGraph(response(0.62f, 0.25f), response(0.45f, 0.25f)), StageGraph(Envelope, null), StageGraph(null, null),
    )
    val handles = listOf(
        emptyList(), emptyList(), emptyList(),
        listOf(HandleInfo("Cutoff and resonance", 0.62f, 0.95f, true, true, "Cutoff 618 · Resonance 347")),
        listOf(HandleInfo("Attack", 0.08f, 1f, true, false, "Attack 12")), emptyList(),
    )
    return PanelState(pages, start, fields, stages, graphs, handles)
}

val SampleHead = EditorHead("Glass Choir", "004", "minilogue xd", canUndo = true, canLive = true, live = true, saveCount = 4)

@Composable
private fun EditorPreview(
    mode: OscylThemeMode = OscylThemeMode.Dark,
    head: EditorHead? = SampleHead,
    comparing: Boolean = false,
    longNames: Boolean = false,
    start: Int = 0,
) = OscylTheme(mode) {
    Shell(Destination.Editor, onSelect = {}, saveCount = head?.saveCount ?: 0, onSave = {}) { modifier ->
        val panel = remember { samplePanel(longNames, start).also { it.comparing = comparing } }
        EditorScreen(head?.copy(comparing = comparing), panel, NoEditorActions, onOpenLibrary = {}, modifier)
    }
}

@OscylPreviews
@Composable
private fun editor(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = EditorPreview(mode)

@OscylPreviews
@Composable
private fun editor_comparing(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = EditorPreview(mode, comparing = true)

/** Scrolled to the filter: its picture with the handle, the edited dials and selectors. */
@OscylPreviews
@Composable
private fun editor_filter(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = EditorPreview(mode, start = 3)

@OscylPreviews
@Composable
private fun editor_readOnly(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    EditorPreview(mode, SampleHead.copy(name = "Factory Strings", slot = "F1 012", model = "Prophet Rev2", readOnly = true, canUndo = false, canLive = false, saveCount = 0))

@OscylPreviews
@Composable
private fun editor_empty(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = EditorPreview(mode, head = null)

@OscylA11yPreviews
@Composable
private fun editor_a11y() = EditorPreview(longNames = true, start = 3)

@OscylA11yPreviews
@Composable
private fun editor_longName_a11y() = EditorPreview(head = SampleHead.copy(name = "Twelve chars", slot = "500"), longNames = true)

@OscylSizePreviews
@Composable
private fun editor_sizes() = EditorPreview()

@Preview(name = "italian", locale = "it", device = PHONE)
@Composable
private fun editor_italian() = EditorPreview()

/** A tablet in landscape: sources, programs and the editor side by side. */
@Preview(name = "expanded_tablet_light", device = "spec:width=1280dp,height=800dp,dpi=240")
@Composable
private fun editor_tablet() = OscylTheme(OscylThemeMode.Light) {
    Shell(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {}) { modifier ->
        val panel = remember { samplePanel() }
        SideBySide(
            { LibraryScreen(SampleUi, NoLibraryActions, onOpenSynth = {}, modifier = it) },
            { m, alone -> EditorScreen(SampleHead, panel, NoEditorActions, onOpenLibrary = {}.takeIf { alone }, modifier = m) },
            modifier,
        )
    }
}

/** An unfolded foldable or a small tablet in landscape (900 dp): the editor alone, its rail open. */
@Preview(name = "expanded_900dp", device = "spec:width=900dp,height=600dp,dpi=240")
@Composable
private fun editor_expanded900() = OscylTheme(OscylThemeMode.Light) {
    Shell(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {}) { modifier ->
        val panel = remember { samplePanel(start = 3) }
        SideBySide(
            { LibraryScreen(SampleUi, NoLibraryActions, onOpenSynth = {}, modifier = it) },
            { m, alone -> EditorScreen(SampleHead, panel, NoEditorActions, onOpenLibrary = {}.takeIf { alone }, modifier = m) },
            modifier,
        )
    }
}

/** A phone on its side, the cutout on either short edge: the stage view clears it, the island on end too. */
@Preview(name = "stageView", device = PHONE_LANDSCAPE)
@Composable
private fun editor_stageView(@PreviewParameter(LandscapeCutouts::class) cutout: FakeCutout) = OscylTheme(OscylThemeMode.Dark) {
    CutoutFrame(cutout) {
        Shell(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {}) { modifier ->
            val panel = remember { samplePanel(start = 3) }
            EditorScreen(SampleHead, panel, NoEditorActions, onOpenLibrary = {}, modifier)
        }
    }
}

/** The stage view at 200% text: nothing clips the title or a value. */
@Preview(name = "stageView_fontScale2", device = PHONE_LANDSCAPE, fontScale = 2f)
@Composable
private fun editor_stageView_a11y() = EditorPreview(start = 3)

/** The filter's picture grown to the whole page, its controls peeking at the foot. */
@OscylPreviews
@Composable
private fun editor_fullScreenGraph(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = OscylTheme(mode) {
    Shell(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {}) { modifier ->
        val panel = remember { samplePanel(start = 3) }
        GraphFullScreen(SampleHead, panel, 3, NoEditorActions, onPage = {}, onClose = {}, modifier)
    }
}

@OscylA11yPreviews
@Composable
private fun editor_fullScreenGraph_a11y() = OscylTheme(OscylThemeMode.Dark) {
    Shell(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {}) { modifier ->
        val panel = remember { samplePanel(start = 3) }
        GraphFullScreen(SampleHead, panel, 3, NoEditorActions, onPage = {}, onClose = {}, modifier)
    }
}

