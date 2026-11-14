package io.github.oscyl.ui.library

import io.github.oscyl.ui.parts.rememberTransient
import io.github.oscyl.ui.parts.transientLayer
import io.github.oscyl.ui.shell.FootSide
import io.github.oscyl.ui.shell.ReserveFoot
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.ui.graphics.CompositingStrategy
import kotlinx.coroutines.flow.collectLatest
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.input.pointer.PointerEventPass
import io.github.oscyl.ui.preferences.Hints
import io.github.oscyl.shared.generated.resources.program_drag_hint
import io.github.oscyl.ui.preferences.LocalTeaching
import androidx.compose.foundation.layout.BoxWithConstraints
import io.github.oscyl.shared.generated.resources.read_stop
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.foundation.gestures.awaitLongPressOrCancellation
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.zIndex
import androidx.compose.ui.composed
import io.github.oscyl.shared.generated.resources.programs_new_key
import io.github.oscyl.shared.generated.resources.ic_add
import org.jetbrains.compose.resources.painterResource
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.foundation.clickable
import androidx.compose.foundation.Image
import io.github.oscyl.ui.shell.SearchKey
import io.github.oscyl.ui.shell.SearchIsland
import io.github.oscyl.ui.shell.SearchFilter
import io.github.oscyl.ui.shell.ProgramSearch
import io.github.oscyl.ui.shell.LocalProgramSearch
import io.github.oscyl.ui.parts.floating
import kotlinx.coroutines.flow.drop
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.AbsoluteAlignment
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.DisposableEffect
import io.github.oscyl.ui.shell.MenuEdits
import io.github.oscyl.shared.generated.resources.search_matches
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.theme.OscylThemeMode
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.Dp
import io.github.oscyl.shared.generated.resources.program_audition
import io.github.oscyl.shared.generated.resources.program_play
import kotlinx.coroutines.CoroutineStart
import io.github.oscyl.ui.parts.plateSpring
import io.github.oscyl.ui.parts.Lamp
import io.github.oscyl.platform.LocalOscylHaptics
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.Orientation
import kotlin.math.roundToInt
import io.github.oscyl.shared.generated.resources.synth_stop_reading
import io.github.oscyl.ui.parts.motionScale
import io.github.oscyl.ui.parts.ProgressScale
import io.github.oscyl.ui.parts.LocalOscylMotion
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.foundation.layout.width
import androidx.compose.animation.core.tween
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.Animatable
import androidx.compose.ui.semantics.testTag
import io.github.oscyl.ui.editor.MiniGraph
import io.github.oscyl.core.GraphShape
import kotlinx.coroutines.flow.first
import androidx.compose.runtime.snapshotFlow
import io.github.oscyl.ui.shell.LocalDestinationShown
import io.github.oscyl.shared.generated.resources.search_none_title
import io.github.oscyl.shared.generated.resources.search_none_body
import io.github.oscyl.shared.generated.resources.search_clear
import io.github.oscyl.shared.generated.resources.ic_close
import kotlinx.coroutines.launch
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.backhandler.BackHandler
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.foundation.lazy.items
import io.github.oscyl.ui.parts.startAlign
import io.github.oscyl.ui.parts.SkeletonRows
import io.github.oscyl.ui.parts.EmptyState
import androidx.compose.ui.platform.testTag
import io.github.oscyl.ui.parts.CollapsingBarState
import io.github.oscyl.ui.parts.CollapsingTopBar
import io.github.oscyl.ui.parts.LargeTitle
import io.github.oscyl.ui.parts.rememberCollapsingBarState
import io.github.oscyl.ui.shell.TopBarActions
import io.github.oscyl.ui.parts.safeInsets
import io.github.oscyl.shared.generated.resources.menu_move_down

import io.github.oscyl.shared.generated.resources.menu_move_up

import io.github.oscyl.core.ProgramAction

import androidx.compose.ui.semantics.customActions

import androidx.compose.ui.semantics.CustomAccessibilityAction

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.oscyl.core.ProgramRow
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.action_back
import io.github.oscyl.shared.generated.resources.ic_back
import io.github.oscyl.shared.generated.resources.ic_more
import io.github.oscyl.shared.generated.resources.library_actions
import io.github.oscyl.shared.generated.resources.library_programs
import io.github.oscyl.shared.generated.resources.library_reading
import io.github.oscyl.shared.generated.resources.program_edited
import io.github.oscyl.shared.generated.resources.program_factory
import io.github.oscyl.shared.generated.resources.program_factory_state
import io.github.oscyl.shared.generated.resources.program_more
import io.github.oscyl.shared.generated.resources.program_reading
import io.github.oscyl.shared.generated.resources.programs_choose_body
import io.github.oscyl.shared.generated.resources.programs_choose_title
import io.github.oscyl.shared.generated.resources.programs_empty_body
import io.github.oscyl.shared.generated.resources.programs_empty_title
import io.github.oscyl.shared.generated.resources.programs_new
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.touchStates
import io.github.oscyl.ui.shell.LocalIslandClearance
import org.jetbrains.compose.resources.pluralStringResource
import org.jetbrains.compose.resources.stringResource

/**
 * The program list of the source shown: its collapsing top bar (Back on a phone,
 * undo, redo, the synth chip and the menu), the source's large title with its count
 * and, in a library, New program, then one row per
 * program. Tapping a row selects it; a long press, or the selected row's ⋯, opens
 * its menu. With nothing chosen, it says how to start.
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun ProgramList(
    programs: ProgramsUi?,
    actions: LibraryActions,
    onBack: (() -> Unit)?,
    modifier: Modifier = Modifier,
    onOpen: () -> Unit = {},
    topBarActions: Boolean = true,
    /** A source is chosen but its programs are not here yet. */
    loading: Boolean = false,
) {
    val c = OscylTheme.colors
    val list = rememberLazyListState()
    val bar = rememberCollapsingBarState(list)
    val back = @Composable { if (onBack != null) IconAction(Res.drawable.ic_back, stringResource(Res.string.action_back), onBack) }
    Column(modifier.fillMaxSize().background(c.list)) {
        if (programs == null) {
            CollapsingTopBar(bar, "", subtitle = null, leading = { back() }, trailing = { if (topBarActions) TopBarActions() })
            // A source chosen, its programs on their way: their rows' places, never a blank page.
            if (loading) SkeletonRows(12, Modifier.padding(top = 56.dp)) else Message(stringResource(Res.string.programs_choose_title), stringResource(Res.string.programs_choose_body))
            return@Column
        }
        var menuFor by rememberSaveable(programs.source) { mutableStateOf<Int?>(null) }
        // The menu a hold opened on a row that could have been moved says so, once.
        var dragNote by remember { mutableStateOf(false) }
        val hints = LocalTeaching.current
        // Search lives in the island (DESIGN.md "Search"): the list says it is in sight and
        // reads the words and the filter. The rows keep their document indices, so every
        // action still names the right program. Without a shell the list keeps its own.
        val search = LocalProgramSearch.current ?: rememberSaveable(saver = ProgramSearch.Saver) { ProgramSearch() }
        val inSight = LocalDestinationShown.current
        // Nothing to search in an empty list: no key.
        val searchable = inSight && programs.rows.isNotEmpty()
        DisposableEffect(search, searchable) {
            search.available = searchable
            onDispose { search.available = false }
        }
        // A list long enough to want the search: its key says so once.
        val longList = searchable && programs.rows.size >= SCRUB_FROM
        DisposableEffect(search, longList) {
            search.long = longList
            onDispose { search.long = false }
        }
        // Another source starts a fresh search (none after a rotation: the source is not saved).
        LaunchedEffect(search, programs.source) {
            if (search.source != null && search.source != programs.source) search.close()
            search.source = programs.source
        }
        val hasFactory = programs.rows.any { it.readOnly }
        SideEffect { if (inSight) search.factory = hasFactory }
        // The row count now, for the wait below (the effect outlives this composition).
        val rowCount by rememberUpdatedState(programs.rows.size)
        val query = search.query
        val scope = rememberCoroutineScope()
        // Where the list stood when the search began: it goes back there when the search ends;
        // while it narrows, the list starts at the top.
        val before = rememberSaveable(programs.source) { intArrayOf(-1, 0) }
        LaunchedEffect(search, programs.source) {
            var was = search.query.isNotBlank() || search.filter != SearchFilter.All
            // A newer query cancels a pending wait.
            snapshotFlow { search.query.trim() to search.filter }.drop(1).collectLatest { (q, f) ->
                val now = q.isNotEmpty() || f != SearchFilter.All
                val wasFiltering = was
                was = now
                if (now && !wasFiltering) {
                    before[0] = list.firstVisibleItemIndex
                    before[1] = list.firstVisibleItemScrollOffset
                }
                if (now) {
                    list.scrollToItem(0)
                } else if (wasFiltering && before[0] >= 0) {
                    // Once the whole list is laid out again, not on the filtered one.
                    // The heading and every row: a filtered list that happens to be long enough does not count.
                    val (i, o) = before[0] to before[1]
                    before[0] = -1
                    snapshotFlow { list.layoutInfo.totalItemsCount }.first { it >= rowCount + 1 }
                    list.scrollToItem(i, o)
                }
            }
        }
        // Back (and the bar's arrow) closes a search before it leaves the list.
        BackHandler(enabled = search.active && inSight) { search.close() }
        // New programs and duplicates: the list goes to what was made.
        LaunchedEffect(actions) {
            actions.placed.collect { at ->
                search.close()
                // The heading is item 0; wait for the row to be in the list, then glide to it.
                snapshotFlow { list.layoutInfo.totalItemsCount }.first { it > at + 1 }
                list.animateScrollToItem(at + 1)
            }
        }
        val match = foldForSearch(query.trim())
        val shown = remember(programs.rows, match, search.filter) {
            programs.rows.withIndex().filter { (_, row) ->
                val kept = when (search.filter) {
                    SearchFilter.All -> true
                    SearchFilter.Edited -> row.edited
                    SearchFilter.User -> !row.readOnly
                    SearchFilter.Factory -> row.readOnly
                }
                kept && (match.isEmpty() || foldForSearch(row.name).contains(match) || foldForSearch(row.slot).contains(match))
            }
        }
        val filtering = match.isNotEmpty() || search.filter != SearchFilter.All
        val count = programs.rows.size
        val read = programs.read
        val caption = when {
            read != null -> stringResource(Res.string.library_reading, read.done, read.total)
            // Searching: how many of them match.
            filtering -> pluralStringResource(Res.plurals.search_matches, count, shown.size, count)
            else -> pluralStringResource(Res.plurals.library_programs, count, count)
        }
        CollapsingTopBar(
            bar,
            programs.title,
            caption,
            leading = { if (onBack != null) IconAction(Res.drawable.ic_back, stringResource(Res.string.action_back), { if (search.active) search.close() else onBack() }) },
            trailing = {
                // Undo and redo head the menu, with their step's words: the title keeps its room.
                // The history is read when the menu opens: read on every change, it cost each knob's
                // release a call into the core from a list nobody was looking at.
                val edits = remember(programs.revision, programs.source, programs.canUndo, programs.canRedo, actions) {
                    MenuEdits(
                        undo = { if (programs.canUndo) actions.history().lastOrNull { it.done }?.title else null },
                        redo = { if (programs.canRedo) actions.history().firstOrNull { !it.done }?.title else null },
                        onUndo = actions::undo,
                        onRedo = actions::redo,
                    )
                }
                // New program heads the menu in a library (and stays in the empty state).
                if (topBarActions) TopBarActions(compact = true, edits = edits, onNew = if (programs.inLibrary) actions::newProgram else null)
            },
        )
        // Looked up once for the whole list, not per row as rows scroll in.
        val words = rowWords()
        // A row held and dragged: where it came from, how far the finger went, where it would land.
        var dragFrom by remember(programs.source) { mutableStateOf<Int?>(null) }
        var finger by remember { mutableFloatStateOf(0f) }
        // Filtering or a read takes the lift away: nothing stays floating.
        val canLift = !filtering && programs.read == null
        LaunchedEffect(canLift) { if (!canLift) { dragFrom = null; finger = 0f } }
        val dragTo by remember(programs.source) {
            derivedStateOf {
                val from = dragFrom ?: return@derivedStateOf null
                val items = list.layoutInfo.visibleItemsInfo
                val me = items.firstOrNull { it.key == from } ?: return@derivedStateOf from
                val centre = me.offset + me.size / 2 + finger
                val rows = items.filter { it.key is Int }
                val over = rows.firstOrNull { centre >= it.offset && centre < it.offset + it.size }
                    ?: if (centre < (rows.firstOrNull()?.offset ?: 0)) rows.firstOrNull() else rows.lastOrNull()
                (over?.key as? Int) ?: from
            }
        }
        val clearance = LocalIslandClearance.current
        // Swiping a row to hear it is taught once: the first row bounces open on "Play"
        // and home whenever the list shows, until the first touch on the list.
        val teaching = LocalTeaching.current
        var swipeTaught by remember { mutableStateOf(teaching.taught(SWIPE_HINT)) }
        val hintRow = if (!swipeTaught && programs.canAudition && !filtering) shown.firstOrNull { it.value.loaded }?.index else null
        // What stands at the list's foot: the island (when it carries search), and the keys.
        val newKey = programs.inLibrary && !search.active
        // New fades as the search opens, rather than leaving in one frame.
        val newShown = rememberTransient(newKey)
        val ownSearch = !search.hosted && programs.rows.isNotEmpty()
        val keys = newKey || ownSearch
        val base = if (search.hosted) clearance else 0.dp
        val foot = base + if (keys) 56.dp + 12.dp else 0.dp
        // Where the keys stood while shown: leaving, they fade there (not on the risen search).
        val keysAt = remember { floatArrayOf(0f) }
        if (keys) keysAt[0] = base.value
        BoxWithConstraints(
            Modifier.weight(1f).fillMaxWidth().then(
                if (hintRow == null) Modifier else Modifier.pointerInput(Unit) {
                    awaitEachGesture {
                        awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
                        swipeTaught = true
                        teaching.teach(SWIPE_HINT)
                    }
                },
            ),
        ) {
        // A narrow list (a foldable's pane, the largest display size) or large text keeps
        // the name on one line: the picture gives way.
        val pictures = maxWidth >= 300.dp && LocalDensity.current.fontScale <= 1.3f
        LazyColumn(
            Modifier.fillMaxSize().testTag("program-list").windowInsetsPadding(safeInsets.only(WindowInsetsSides.Horizontal)),
            state = list,
            contentPadding = PaddingValues(start = 8.dp, end = 8.dp, bottom = 16.dp + foot),
        ) {
            item(key = "title") { Heading(programs, caption, bar, actions) }
            if (programs.rows.isEmpty()) {
                item(key = "empty") {
                    Message(
                        stringResource(Res.string.programs_empty_title),
                        stringResource(Res.string.programs_empty_body),
                        fill = false,
                        // New stands at the foot already: one New on the page.
                        action = if (programs.inLibrary && !newKey) ({ OutlinedAction(stringResource(Res.string.programs_new), actions::newProgram) }) else null,
                    )
                }
            }
            if (filtering && shown.isEmpty() && programs.rows.isNotEmpty()) {
                item(key = "no-match") {
                    Message(
                        stringResource(Res.string.search_none_title),
                        stringResource(Res.string.search_none_body, query.trim()),
                        fill = false,
                        action = { OutlinedAction(stringResource(Res.string.search_clear), search::close) },
                    )
                }
            }
            items(shown, key = { it.index }, contentType = { "program" }) { (i, row) ->
                // During a read nothing can be edited: a row's picture changes only when it is read,
                // not with every slot that arrives (each a new revision).
                val stamp = if (programs.read != null) 0uL else programs.revision
                val picture = remember(programs.source, stamp, i, row.loaded, pictures) { if (row.loaded && pictures) actions.thumbnail(i) else null }
                ProgramRowView(
                    modifier = Modifier.testTag("program-row"),
                    row = row,
                    thumbnail = picture,
                    onAudition = if (programs.canAudition && row.loaded) ({ actions.audition(i) }) else null,
                    words = words,
                    match = match,
                    selected = programs.selected == i,
                    onClick = {
                        actions.selectProgram(i)
                        onOpen()
                    },
                    // Its actions, without opening it (the editor's program stays).
                    onMenu = {
                        if (!(row.readOnly || filtering || programs.read != null)) hints.once(Hints.ROW_DRAG) { dragNote = true }
                        menuFor = i
                    },
                    // No reordering while filtered: a neighbour on screen is not the one in the list.
                    onMove = if (row.readOnly || filtering) null else ({ up -> actions.act(i, if (up) ProgramAction.MOVE_UP else ProgramAction.MOVE_DOWN) }),
                    // Held and dragged, it moves to where the ink line shows (as one undo step).
                    onLift = if (row.readOnly || filtering || programs.read != null) null else ({ dragFrom = i; finger = 0f }),
                    onDrag = { finger += it },
                    onDrop = {
                        val to = dragTo
                        dragFrom = null
                        finger = 0f
                        if (to != null && to != i) {
                            actions.move(i, to)
                            hints.teach(Hints.ROW_DRAG)
                        }
                    },
                    lifted = dragFrom == i,
                    hinting = hintRow == i,
                    liftY = { finger },
                    gap = dragTo.let { to -> val from = dragFrom; if (from == null || to == null || to == from || to != i) 0 else if (to < from) -1 else 1 },
                )
            }
        }
        // A long list: the slot scrubber at the end edge while it scrolls.
        // Not while searching (the keyboard leaves no room for a scale, and a filtered list
        // is short), and only with a scale's room between the top and the foot.
        if (!filtering && !search.active && programs.rows.size > SCRUB_FROM && maxHeight - foot >= 240.dp) {
            SlotScrubber(list, programs.rows, Modifier.align(Alignment.CenterEnd).windowInsetsPadding(safeInsets.only(WindowInsetsSides.End)).padding(top = 8.dp, end = 4.dp, bottom = 8.dp + foot))
        }
        // The foot's end, in the thumb's reach: New program in a library, and beside a side
        // island (or without a shell) the search key and field the island would carry.
        if (keys || newShown.up || (search.active && !search.hosted)) {
            Box(
                Modifier
                    .align(Alignment.BottomEnd)
                    .fillMaxWidth()
                    .windowInsetsPadding(safeInsets.only(if (search.hosted) WindowInsetsSides.Horizontal else WindowInsetsSides.Horizontal + WindowInsetsSides.Bottom))
                    .padding(start = 12.dp, end = 12.dp, bottom = 12.dp + if (keys) base else keysAt[0].dp),
                contentAlignment = Alignment.CenterEnd,
            ) {
                if (search.active && !search.hosted) {
                    SearchIsland(search)
                } else {
                    // Dimmed with the island while the list scrolls under them.
                    val peek by io.github.oscyl.ui.shell.peekAlpha()
                    // A notice stands above them.
                    ReserveFoot(FootSide.Keys, 56.dp, shown = keys)
                    Row(Modifier.height(56.dp).graphicsLayer { alpha = peek; compositingStrategy = CompositingStrategy.ModulateAlpha }, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        if (newShown.up) NewKey(actions::newProgram, Modifier.fillMaxHeight().transientLayer({ newShown.value }).then(if (newKey) Modifier else Modifier.clearAndSetSemantics {}), enabled = newKey)
                        if (ownSearch) SearchKey(search, Modifier.fillMaxHeight())
                    }
                }
            }
        }
        }
        val menuIndex = menuFor?.takeIf { it < programs.rows.size }
        if (menuIndex != null) {
            ProgramMenuDialog(
                programs.rows[menuIndex], menuIndex, actions,
                onDismiss = { menuFor = null; dragNote = false },
                onMoved = { at -> menuFor = at.coerceIn(0, programs.rows.size - 1) },
                onPlay = if (programs.canAudition && programs.rows[menuIndex].loaded) ({ actions.audition(menuIndex) }) else null,
                note = if (dragNote) stringResource(Res.string.program_drag_hint) else null,
            )
        }
    }
}

/**
 * The source's large title (it rises into the bar as it scrolls away), its count or
 * read progress, and in a library New program; then the rule the rows start on.
 */
@Composable
private fun Heading(programs: ProgramsUi, caption: String, bar: CollapsingBarState, actions: LibraryActions) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    Column(Modifier.fillMaxWidth()) {
        LargeTitle(programs.title, bar, Modifier.padding(horizontal = 8.dp))
        // One height with or without New program (a synth's slots have none): the rows start at one line.
        Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).padding(start = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            BasicText(
                caption,
                Modifier.weight(1f),
                style = type.bodySmall.copy(color = c.legend, fontFeatureSettings = "tnum", textDirection = TextDirection.Content, textAlign = startAlign()),
            )
            // A synth being read: the tuning scale beside its words, and Stop, in the row's
            // own 48 dp (a read starting or ending moves nothing).
            programs.read?.let { read ->
                ProgressScale(if (read.total > 0) read.done.toFloat() / read.total else 0f, Modifier.width(96.dp).padding(horizontal = 8.dp))
                // The word, not ✕: everywhere else ✕ closes.
                val said = stringResource(Res.string.synth_stop_reading)
                OutlinedAction(stringResource(Res.string.read_stop), actions::stopRead, Modifier.testTag("read-stop").semantics { contentDescription = said })
            }
        }
        Spacer(Modifier.fillMaxWidth().heightIn(min = OscylTokens.ruleWidth).background(c.rule))
    }
}

/**
 * New program, a floating key in the island's make at the list's foot end: the + glyph and
 * the word, 56 dp tall. It adds after the selected program and takes the list to it.
 */
@Composable
private fun NewKey(onNew: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true) = io.github.oscyl.ui.parts.FootKey(
    stringResource(Res.string.programs_new_key),
    Res.drawable.ic_add,
    onNew,
    modifier.testTag("program-new-key"),
    enabled = enabled,
    said = stringResource(Res.string.programs_new),
)

/**
 * A long press lifts (a confirm), then each move is handed on; let go after moving drops,
 * let go without moving opens the menu. Quick swipes and scrolls are left to the row and
 * the list: nothing is taken before the long press.
 */
private fun Modifier.liftOnHold(onLift: () -> Unit, onDrag: (Float) -> Unit, onDrop: () -> Unit, onMenu: () -> Unit): Modifier = composed {
    val haptics = LocalOscylHaptics.current
    val lift by rememberUpdatedState(onLift)
    val drag by rememberUpdatedState(onDrag)
    val drop by rememberUpdatedState(onDrop)
    val menu by rememberUpdatedState(onMenu)
    pointerInput(Unit) {
        awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false)
            awaitLongPressOrCancellation(down.id) ?: return@awaitEachGesture
            haptics.confirm()
            lift()
            // How far from where it lifted, at most: a steady finger's jitter is not a move.
            var net = 0f
            var farthest = 0f
            var finished = false
            try {
                while (true) {
                    val change = awaitPointerEvent().changes.firstOrNull { it.id == down.id } ?: break
                    if (!change.pressed) {
                        change.consume()
                        break
                    }
                    val dy = change.positionChange().y
                    change.consume()
                    if (dy != 0f) {
                        net += dy
                        farthest = maxOf(farthest, kotlin.math.abs(net))
                        drag(dy)
                    }
                }
                finished = true
            } finally {
                // Taken away (the system's gesture, the list changing under it): it lands where it was.
                drop()
            }
            if (finished && farthest <= viewConfiguration.touchSlop) menu()
        }
    }
}

/** The one-time hint of swiping a row to hear it. */
private const val SWIPE_HINT = "swipe-play"

/** From how many slots a list carries the scrubber. */
private const val SCRUB_FROM = 40

/** How opaque the scrubber stands while the list scrolls under it (whole once held). */
private const val ScrubberRest = 0.6f

/**
 * The slot scrubber (DESIGN.md "Search"): a floating scale on the end edge while the list
 * scrolls, its slots printed at five marks and the list's place as an ink line. Held, the
 * list follows the thumb: the slot and name stand in a plate beside it, and every bank of
 * ten ticks. It fades in at once and out a moment after the list stops; faded, it takes no
 * touch. A screen reader scrolls the list itself, so the scale is not said.
 */
@Composable
private fun SlotScrubber(list: LazyListState, rows: List<ProgramRow>, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val haptics = LocalOscylHaptics.current
    val motion = LocalOscylMotion.current
    val scope = rememberCoroutineScope()
    var held by remember { mutableStateOf<Int?>(null) }
    var thumbY by remember { mutableStateOf(0f) }
    val seen = remember { Animatable(0f) }
    val active = list.isScrollInProgress || held != null
    LaunchedEffect(active) {
        val scale = motionScale(motion)
        when {
            scale == 0f -> seen.snapTo(if (active) 1f else 0f)
            active -> seen.animateTo(1f, tween((OscylTokens.fastMs * scale).roundToInt(), easing = OscylTokens.easing))
            // A moment to reach it after a fling: the fade is the wait.
            else -> seen.animateTo(0f, tween((OscylTokens.baseMs * 3 * scale).roundToInt(), delayMillis = (600 * scale).roundToInt(), easing = OscylTokens.easing))
        }
    }
    // Over the rows' ⋯ column it stands dimmed while the list scrolls, and whole once held.
    val whole by animateFloatAsState(if (held != null) 1f else ScrubberRest, tween(if (motion) OscylTokens.baseMs else 0, easing = OscylTokens.easing), label = "scrubber")
    val visible by remember { derivedStateOf { seen.value > 0f } }
    if (!visible) return
    val n = rows.size
    val pad = with(LocalDensity.current) { 12.dp.toPx() }
    // Faded stroke by stroke: no offscreen copy is made as a scroll begins.
    Box(modifier.fillMaxHeight().graphicsLayer { alpha = seen.value * whole; compositingStrategy = CompositingStrategy.ModulateAlpha }) {
        // The rows can shrink under a held scale (an undo, a synth unplugged).
        val plate = held?.takeIf { it < rows.size }
        if (plate != null) {
            var plateH by remember { mutableStateOf(0) }
            Column(
                Modifier
                    .align(Alignment.TopEnd)
                    .padding(end = 52.dp + 8.dp)
                    .onSizeChanged { plateH = it.height }
                    .offset { IntOffset(0, (thumbY - plateH / 2f).roundToInt()) }
                    .widthIn(max = 200.dp)
                    .clearAndSetSemantics {}
                    .floating(c)
                    .padding(horizontal = 16.dp, vertical = 10.dp),
            ) {
                BasicText(rows[plate].slot, style = type.headlineSmall.copy(color = c.ink, fontFeatureSettings = "tnum", fontWeight = FontWeight.SemiBold))
                if (rows[plate].loaded) BasicText(rows[plate].name, style = type.bodyMedium.copy(color = c.ink, fontWeight = FontWeight.Medium), maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        Column(
            Modifier
                .align(Alignment.TopEnd)
                .fillMaxHeight()
                .width(52.dp)
                .clearAndSetSemantics { testTag = "slot-scrubber" }
                .floating(c)
                // It takes touches only while the list moves or it is held: fading after a
                // fling, a tap goes to the row's ⋯ under it.
                .then(if (list.isScrollInProgress || held != null) Modifier.scrubbing(n, pad, list, haptics, scope, { held }, { held = it }, { thumbY = it }) else Modifier)
                .drawBehind {
                    // The list's place: an ink line across the scale.
                    val at = if (n > 1) list.firstVisibleItemIndex.coerceAtMost(n).toFloat() / n else 0f
                    val y = pad + at * (size.height - 2 * pad)
                    drawLine(c.ink, Offset(4.dp.toPx(), y), Offset(size.width - 4.dp.toPx(), y), 3.dp.toPx())
                }
                .padding(vertical = 8.dp),
            verticalArrangement = Arrangement.SpaceBetween,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            for (k in 0..4) BasicText(rows[(n - 1) * k / 4].slot, style = type.labelSmall.copy(color = c.legend, fontFeatureSettings = "tnum"))
        }
    }
}

/** The scale's gesture: the row under the thumb, the list following, a tick per bank of ten. */
private fun Modifier.scrubbing(
    n: Int,
    pad: Float,
    list: LazyListState,
    haptics: io.github.oscyl.platform.OscylHaptics,
    scope: kotlinx.coroutines.CoroutineScope,
    held: () -> Int?,
    hold: (Int?) -> Unit,
    thumb: (Float) -> Unit,
): Modifier = pointerInput(n) {
    fun jump(y: Float) {
        thumb(y.coerceIn(0f, size.height.toFloat()))
        val row = (((y - pad) / (size.height - 2 * pad)) * n).toInt().coerceIn(0, n - 1)
        if (held()?.let { it / 10 } != row / 10) haptics.tick()
        hold(row)
        // The heading is item 0.
        scope.launch { list.scrollToItem(row + 1) }
    }
    awaitEachGesture {
        val down = awaitFirstDown()
        down.consume()
        try {
            jump(down.position.y)
            while (true) {
                val event = awaitPointerEvent()
                val change = event.changes.firstOrNull { it.id == down.id } ?: break
                if (!change.pressed) break
                change.consume()
                jump(change.position.y)
            }
        } finally {
            // Taken away or let go: the scale lets go too.
            hold(null)
        }
    }
}

/** [name] with each place it holds [match] (folded) underlined. */
fun underlined(name: String, match: String): AnnotatedString = buildAnnotatedString {
    append(name)
    if (match.isEmpty()) return@buildAnnotatedString
    val folded = foldForSearch(name)
    // Folding keeps one letter per letter, but lower-casing can grow a few; then no marks.
    if (folded.length != name.length) return@buildAnnotatedString
    var at = folded.indexOf(match)
    while (at >= 0) {
        addStyle(SpanStyle(textDecoration = TextDecoration.Underline), at, at + match.length)
        at = folded.indexOf(match, at + match.length)
    }
}

/** Lower case with the common Latin accents folded away, so "cafe" finds "Café". */
internal fun foldForSearch(s: String): String = buildString(s.length) { s.lowercase().forEach { append(Folds[it] ?: it) } }

// ponytail: a table of Latin accents, not full Unicode normalisation (none in common Kotlin); add letters as names need them.
private val Folds: Map<Char, Char> =
    "àáâãäåāăąçćčďèéêëēėęěìíîïīįłñńňòóôõöøōőřśšşťùúûüūůűųýÿźżž".zip("aaaaaaaaacccdeeeeeeeeiiiiiilnnnoooooooorssstuuuuuuuuyyzzz").toMap()

/** A heading and a line of guidance, then [action], centred in the pane at a reading width. */
@Composable
private fun Message(title: String, body: String, fill: Boolean = true, action: (@Composable () -> Unit)? = null) {
    val area = if (fill) Modifier.fillMaxSize().padding(bottom = LocalIslandClearance.current) else Modifier.fillMaxWidth()
    Box(area, contentAlignment = Alignment.Center) { EmptyState(title, body, action = action) }
}

/**
 * The words every program row uses, looked up once per list. Compared by its words: a
 * new one with the same words (the list composed again) leaves every row alone.
 */
@Immutable
internal data class RowWords(
    val reading: String,
    val edited: String,
    val factoryState: String,
    val factory: String,
    val moveUp: String,
    val moveDown: String,
    val actions: String,
)

@Composable
internal fun rowWords() = RowWords(
    reading = stringResource(Res.string.program_reading),
    edited = stringResource(Res.string.program_edited),
    factoryState = stringResource(Res.string.program_factory_state),
    factory = stringResource(Res.string.program_factory),
    moveUp = stringResource(Res.string.menu_move_up),
    moveDown = stringResource(Res.string.menu_move_down),
    actions = stringResource(Res.string.library_actions),
)

/**
 * One program: its slot in `legend` (tabular), its name, then an orange dot when
 * edited and a "Factory" legend for a read-only slot (said with the row, never by
 * colour alone). Rows sit on 1 dp rules; the selected one on the flat `raised` plate.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun ProgramRowView(
    row: ProgramRow,
    words: RowWords,
    selected: Boolean,
    onClick: () -> Unit,
    onMenu: () -> Unit,
    onMove: ((up: Boolean) -> Unit)? = null,
    modifier: Modifier = Modifier,
    /** The program's first picture, drawn small before the menu's place. */
    thumbnail: GraphShape? = null,
    /** A swipe toward the start (or the screen reader's Audition) plays it; null: no swipe. */
    onAudition: (() -> Unit)? = null,
    /** Previews only: the row drawn swiped this far. */
    startSwipe: Dp = 0.dp,
    /** The swipe's hint: the row rests [startSwipe] open on "Play" while true, and closes when it turns false. */
    hinting: Boolean = false,
    /** The search's words, folded: underlined where the name holds them. */
    match: String = "",
    /**
     * Held, the row lifts ([onLift]) and follows the finger ([onDrag], each step's
     * distance) until let go ([onDrop]); held and let go without moving, its menu opens.
     * Null: a long press opens the menu.
     */
    onLift: (() -> Unit)? = null,
    onDrag: (Float) -> Unit = {},
    onDrop: () -> Unit = {},
    /** Lifted: drawn [liftY] off its place, over the rows. */
    lifted: Boolean = false,
    liftY: () -> Float = { 0f },
    /** Where a lifted row would land, as a 2 dp ink line: -1 above this row, 1 under it, 0 none. */
    gap: Int = 0,
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    val name = if (row.loaded) row.name else words.reading
    // A slot read while the list is open fades its name and picture in once (none with motion off).
    val motion = LocalOscylMotion.current
    val appear = remember { Animatable(if (row.loaded) 1f else 0f) }
    LaunchedEffect(row.loaded) {
        if (!row.loaded) appear.snapTo(0f)
        else if (appear.value < 1f) {
            if (motionScale(motion) == 0f) appear.snapTo(1f) else appear.animateTo(1f, tween((OscylTokens.baseMs * motionScale(motion)).roundToInt(), easing = OscylTokens.easing))
        }
    }
    val states = listOfNotNull(if (row.edited) words.edited else null, if (row.readOnly) words.factoryState else null)
    val rule = c.rule.copy(alpha = c.rule.alpha * 0.7f)
    val moveUp = words.moveUp
    val moveDown = words.moveDown
    // A swipe toward the start slides the row's layer up to 96 dp over "Play" and its
    // lamp; past 64 dp a tick, the word turns ink, and letting go plays it. It claims
    // the gesture only after a horizontal slop, so the list still scrolls.
    val haptics = LocalOscylHaptics.current
    val teaching = LocalTeaching.current
    val scope = rememberCoroutineScope()
    val density = LocalDensity.current
    val rtl = LocalLayoutDirection.current == LayoutDirection.Rtl
    val reach = with(density) { 96.dp.toPx() }
    val trigger = with(density) { 64.dp.toPx() }
    val swipe = remember { Animatable(with(density) { startSwipe.toPx() }) }
    // Only a hint that was showing closes (a preview's [startSwipe] stays open).
    var hinted by remember { mutableStateOf(false) }
    LaunchedEffect(hinting) {
        if (hinting && motionScale(motion) > 0f) {
            // A bounce, never a rest: the row peeks at "Play" twice and comes home, so it
            // can never be left standing open. Once the row has faded in.
            hinted = true
            snapshotFlow { appear.value }.first { it >= 1f }
            val scale = motionScale(motion)
            listOf(40.dp, 20.dp).forEach { out ->
                swipe.animateTo(with(density) { out.toPx() }, tween((OscylTokens.slowMs * scale).roundToInt(), easing = OscylTokens.easing))
                swipe.animateTo(0f, spring(dampingRatio = 0.55f, stiffness = 500f))
            }
            hinted = false
        } else if (hinted) {
            hinted = false
            swipe.animateTo(0f, plateSpring(0.5f))
        }
    }
    val audition by rememberUpdatedState(onAudition)
    val playWord = stringResource(Res.string.program_play)
    val auditionWord = stringResource(Res.string.program_audition)
    Box(
        Modifier
            .fillMaxWidth()
            .then(if (lifted) Modifier.zIndex(1f).graphicsLayer { translationY = liftY() } else Modifier)
            .drawWithContent {
                drawContent()
                if (gap != 0) {
                    val y = if (gap < 0) 0f else size.height
                    drawLine(c.ink, Offset(0f, y), Offset(size.width, y), 2.dp.toPx())
                }
            },
    ) {
    if (onAudition != null) {
        // Under the row, at its end: drawn only while the row is moved off it.
        Row(
            Modifier
                .matchParentSize()
                .graphicsLayer { alpha = (swipe.value / trigger).coerceIn(0f, 1f) }
                .clearAndSetSemantics {}
                .padding(end = 20.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            val armed = swipe.value >= trigger
            Lamp(lit = armed)
            BasicText(playWord, style = type.labelLarge.copy(color = if (armed) c.ink else c.legend, fontWeight = FontWeight.SemiBold))
        }
    }
    Row(
        modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .graphicsLayer { translationX = swipe.value * if (rtl) 1f else -1f }
            .then(
                if (onAudition == null) {
                    Modifier
                } else {
                    Modifier.draggable(
                        rememberDraggableState { delta ->
                            val before = swipe.value
                            val next = (before + if (rtl) delta else -delta).coerceIn(0f, reach)
                            if ((before < trigger) != (next < trigger)) haptics.tick()
                            scope.launch(start = CoroutineStart.UNDISPATCHED) { swipe.snapTo(next) }
                        },
                        Orientation.Horizontal,
                        onDragStopped = {
                            if (swipe.value >= trigger) {
                                audition?.invoke()
                                // Done: the first row need not show it again.
                                teaching.teach(SWIPE_HINT)
                            }
                            swipe.animateTo(0f, plateSpring(0.5f))
                        },
                    )
                },
            )
            .then(if (lifted) Modifier.floating(c, shape) else Modifier.background(if (selected) c.raised else c.list, shape))
            .drawBehind {
                if (!selected) drawLine(rule, Offset(0f, size.height), Offset(size.width, size.height), OscylTokens.ruleWidth.toPx())
            }
            .combinedClickable(
                interactionSource = interaction,
                indication = null,
                onLongClickLabel = words.actions,
                // A slot not read yet has nothing to act on; a row that lifts holds its own long press.
                onLongClick = if (row.loaded && onLift == null) onMenu else null,
                onClick = onClick,
            )
            .then(if (onLift != null && row.loaded) Modifier.liftOnHold(onLift, onDrag, onDrop, onMenu) else Modifier)
            .semantics(mergeDescendants = true) {
                this.selected = selected
                if (states.isNotEmpty()) stateDescription = states.joinToString(", ")
                // Screen readers move a program without the drag (the core refuses what it can't).
                val acts = listOfNotNull(
                    onAudition?.let { a -> CustomAccessibilityAction(auditionWord) { a(); true } },
                    onMove?.let { m -> CustomAccessibilityAction(moveUp) { m(true); true } },
                    onMove?.let { m -> CustomAccessibilityAction(moveDown) { m(false); true } },
                )
                if (acts.isNotEmpty()) customActions = acts
            }
            .touchStates(interaction, shape, c.press)
            .padding(start = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        BasicText(row.slot, style = type.bodyMedium.copy(color = c.legend, fontFeatureSettings = "tnum"))
        if (!row.loaded) {
            // Where the name will be: a printed rule, said as "Reading…".
            Box(
                Modifier
                    .weight(1f)
                    .heightIn(min = 32.dp)
                    .semantics { contentDescription = name }
                    .drawBehind {
                        val y = size.height / 2
                        drawLine(c.legend.copy(alpha = 0.5f), Offset(0f, y), Offset(size.width * 0.6f, y), OscylTokens.ruleWidth.toPx())
                    },
            )
        } else BasicText(
            underlined(name, match),
            Modifier.weight(1f).padding(vertical = 8.dp).graphicsLayer { alpha = appear.value },
            style = type.bodyLarge.copy(
                color = if (row.loaded) c.ink else c.legend,
                // One weight, selected or not: the plate says it, and no name changes width.
                fontWeight = FontWeight.Medium,
                textDirection = TextDirection.Content,
                textAlign = startAlign(),
            ),
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
        )
        if (row.edited) Box(Modifier.size(6.dp).background(c.act, CircleShape).clearAndSetSemantics {})
        if (row.readOnly) {
            BasicText(
                words.factory,
                Modifier.clearAndSetSemantics {},
                style = type.labelSmall.copy(color = c.legend),
            )
        }
        // Last before the menu's place, so the pictures stand in one column whatever comes before.
        if (thumbnail != null) MiniGraph(thumbnail, c.legend, Modifier.size(56.dp, 28.dp).graphicsLayer { alpha = appear.value }.clearAndSetSemantics { testTag = "program-thumbnail" })
        // Every read row shows its ⋯ (in `legend`; ink on the selected one), in the place kept for it.
        if (row.loaded) {
            IconAction(Res.drawable.ic_more, stringResource(Res.string.program_more, name), onMenu, tint = if (selected) null else c.legend)
        } else {
            // The menu's place is kept: selecting a row moves neither its name's wrap nor its dot.
            Spacer(Modifier.size(48.dp))
        }
    }
    }
}

/** Rows swiped toward the start: short of the mark, and past it (the lamp lit, the word in ink). */
@OscylPreviews
@Composable
private fun library_row_swiped(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    val words = rowWords()
    Column {
        ProgramRowView(SampleRows[0], words, selected = false, onClick = {}, onMenu = {}, onAudition = {}, startSwipe = 40.dp)
        ProgramRowView(SampleRows[1], words, selected = false, onClick = {}, onMenu = {}, onAudition = {}, startSwipe = 80.dp)
        ProgramRowView(SampleRows[2], words, selected = true, onClick = {}, onMenu = {}, onAudition = {})
    }
}
