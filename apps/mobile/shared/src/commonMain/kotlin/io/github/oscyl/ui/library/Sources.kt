package io.github.oscyl.ui.library

import io.github.oscyl.shared.generated.resources.name_new_library
import io.github.oscyl.shared.generated.resources.name_new_folder
import io.github.oscyl.shared.generated.resources.search_done
import androidx.compose.runtime.LaunchedEffect
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.shared.generated.resources.program_more
import io.github.oscyl.shared.generated.resources.ic_more
import androidx.compose.foundation.layout.widthIn
import io.github.oscyl.ui.parts.ProgressScale
import io.github.oscyl.ui.parts.startAlign
import androidx.compose.ui.platform.testTag
import androidx.compose.foundation.lazy.rememberLazyListState
import io.github.oscyl.shared.generated.resources.destination_library
import io.github.oscyl.ui.parts.CollapsingTopBar
import io.github.oscyl.ui.parts.LargeTitle
import io.github.oscyl.ui.parts.rememberCollapsingBarState
import io.github.oscyl.ui.shell.TopBarActions
import io.github.oscyl.ui.parts.safeInsets
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import io.github.oscyl.core.OutlineLine
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.ic_chevron
import io.github.oscyl.shared.generated.resources.ic_library
import io.github.oscyl.shared.generated.resources.library_actions
import io.github.oscyl.ui.parts.EmptyState
import io.github.oscyl.shared.generated.resources.library_connect_synth
import io.github.oscyl.shared.generated.resources.library_empty_body
import io.github.oscyl.shared.generated.resources.library_empty_title
import io.github.oscyl.shared.generated.resources.library_folder_closed
import io.github.oscyl.shared.generated.resources.library_folder_open
import io.github.oscyl.shared.generated.resources.library_import
import io.github.oscyl.shared.generated.resources.library_legend_libraries
import io.github.oscyl.shared.generated.resources.library_legend_synth
import io.github.oscyl.shared.generated.resources.library_new
import io.github.oscyl.shared.generated.resources.library_new_folder
import io.github.oscyl.shared.generated.resources.library_no_synth
import io.github.oscyl.shared.generated.resources.library_programs
import io.github.oscyl.shared.generated.resources.library_reading
import io.github.oscyl.shared.generated.resources.library_unreadable
import io.github.oscyl.shared.generated.resources.library_unsaved
import io.github.oscyl.shared.generated.resources.node_move_down
import io.github.oscyl.shared.generated.resources.node_move_up
import io.github.oscyl.shared.generated.resources.synth_state_simulated
import io.github.oscyl.shared.generated.resources.synth_state_connected
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.Lamp
import io.github.oscyl.ui.parts.Legend
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.touchStates
import io.github.oscyl.ui.shell.LocalIslandClearance
import io.github.oscyl.ui.synth.SynthCard
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.pluralStringResource
import org.jetbrains.compose.resources.stringResource

/** Each level of the tree steps in this far, over a 1 dp guide (DESIGN.md Navigation). */
private val Indent = 18.dp

/**
 * The sources column: connected synths, then Oscyl's libraries as a folder tree.
 * A tap shows a source's programs; a folder's tap opens or closes it; a long press
 * (or a screen reader's actions) offers renaming, exporting, moving and deleting.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun Sources(
    ui: LibraryUi,
    selected: String?,
    actions: LibraryActions,
    onSelect: (String) -> Unit,
    onOpenSynth: () -> Unit,
    modifier: Modifier = Modifier,
    topBarActions: Boolean = true,
    /** Shown under the empty state while the person has not said what they mainly do. */
    roleHint: (@Composable () -> Unit)? = null,
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    var menuFor by rememberSaveable { mutableStateOf<String?>(null) }
    var naming by remember { mutableStateOf<List<String>?>(null) }
    var renaming by rememberSaveable { mutableStateOf<String?>(null) }
    val list = rememberLazyListState()
    val bar = rememberCollapsingBarState(list)
    val title = stringResource(Res.string.destination_library)
    Column(modifier.fillMaxSize().background(c.sidebar)) {
    // Beside the programs (whose bar carries the chip and menu) its bar has nothing to hold: no empty 56 dp row.
    CollapsingTopBar(bar, title, subtitle = null, trailing = { if (topBarActions) TopBarActions() }, minHeight = if (topBarActions) 56.dp else 0.dp)
    LazyColumn(
        Modifier
            .fillMaxSize()
            .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Horizontal)),
        state = list,
        contentPadding = PaddingValues(start = 8.dp, end = 8.dp, bottom = 16.dp + LocalIslandClearance.current),
    ) {
        item { LargeTitle(title, bar, Modifier.padding(start = 8.dp, end = 8.dp, bottom = 16.dp)) }
        item { Legend(stringResource(Res.string.library_legend_synth), Modifier.padding(start = 8.dp, end = 8.dp, bottom = 8.dp)) }
        if (ui.synths.isEmpty()) {
            item {
                FlowRow(
                    Modifier.fillMaxWidth().padding(start = 8.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    itemVerticalAlignment = Alignment.CenterVertically,
                ) {
                    BasicText(
                        stringResource(Res.string.library_no_synth),
                        Modifier.padding(end = 8.dp),
                        style = type.bodyMedium.copy(color = c.legend, textDirection = TextDirection.Content),
                    )
                    OutlinedAction(stringResource(Res.string.library_connect_synth), onOpenSynth)
                }
            }
        }
        items(ui.synths, key = { "synth ${it.id}" }) { card ->
            val key = synthSource(card.id)
            SourceRow(
                title = card.model,
                caption = card.read?.let { stringResource(Res.string.library_reading, it.done, it.total) }
                    // Always a line, so a read starting or ending never changes the row's height.
                    ?: stringResource(if (card.simulated) Res.string.synth_state_simulated else Res.string.synth_state_connected),
                selected = key == selected,
                onClick = { onSelect(key) },
                leading = { Lamp(lit = true) },
                scale = true,
                progress = card.read?.let { if (it.total > 0) it.done.toFloat() / it.total else 0f },
            )
        }
        item { Legend(stringResource(Res.string.library_legend_libraries), Modifier.padding(start = 8.dp, top = 24.dp, end = 8.dp, bottom = 8.dp)) }
        if (ui.lines.isNotEmpty()) {
            item {
                FlowRow(Modifier.fillMaxWidth().padding(horizontal = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedAction(stringResource(Res.string.library_new), { naming = ui.lines.map { it.id }; actions.newLibrary() })
                    OutlinedAction(stringResource(Res.string.library_new_folder), { naming = ui.lines.map { it.id }; actions.newFolder() })
                    OutlinedAction(stringResource(Res.string.library_import), actions::importFile)
                }
            }
        } else {
            item {
                Column(Modifier.fillMaxWidth().padding(8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                    // Its next steps are its own actions, under its words.
                    EmptyState(
                        stringResource(Res.string.library_empty_title),
                        stringResource(Res.string.library_empty_body),
                        action = {
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                OutlinedAction(stringResource(Res.string.library_new), { naming = ui.lines.map { it.id }; actions.newLibrary() })
                                OutlinedAction(stringResource(Res.string.library_import), actions::importFile)
                            }
                        },
                    )
                    roleHint?.invoke()
                }
            }
        }
        items(ui.lines.filter { it.shown }, key = { it.id }) { line ->
            TreeRow(
                line,
                selected = librarySource(line.id) == selected,
                unsaved = line.id in ui.unsaved,
                actions = actions,
                onClick = { if (line.isFolder) actions.setOpen(line.id, !line.open) else onSelect(librarySource(line.id)) },
                onMenu = { menuFor = line.id },
            )
        }
    }
    }
    // A New that failed names nothing later.
    LaunchedEffect(ui.notice) { if (ui.notice is Notice.Failed) naming = null }
    // What was there before New library or New folder: the line that appears is named at once.
    LaunchedEffect(ui.lines, naming) {
        val before = naming ?: return@LaunchedEffect
        val made = ui.lines.firstOrNull { it.id !in before } ?: return@LaunchedEffect
        naming = null
        renaming = made.id
    }
    ui.lines.firstOrNull { it.id == renaming }?.let { line ->
        NameDialog(
            stringResource(if (line.isFolder) Res.string.name_new_folder else Res.string.name_new_library),
            line.name,
            onDismiss = { renaming = null },
            confirmWord = stringResource(Res.string.search_done),
        ) { renaming = null; actions.renameNode(line.id, it) }
    }
    val menuLine = ui.lines.firstOrNull { it.id == menuFor }
    if (menuLine != null) {
        NodeMenu(menuLine, ui.lines, actions, onDismiss = { menuFor = null })
    }
}

/** A library or folder line, indented to its depth with a guide per level. */
@Composable
private fun TreeRow(
    line: OutlineLine,
    selected: Boolean,
    unsaved: Boolean,
    actions: LibraryActions,
    onClick: () -> Unit,
    onMenu: () -> Unit,
) {
    val c = OscylTheme.colors
    val folder = line.isFolder
    val moveUp = stringResource(Res.string.node_move_up)
    val moveDown = stringResource(Res.string.node_move_down)
    val programs = line.programs?.let { pluralStringResource(Res.plurals.library_programs, it.toInt(), it.toInt()) }
    val state = when {
        folder -> stringResource(if (line.open) Res.string.library_folder_open else Res.string.library_folder_closed)
        unsaved -> stringResource(Res.string.library_unsaved)
        else -> null
    }
    val motion = LocalOscylMotion.current
    val turn by animateFloatAsState(
        if (line.open) 90f else 0f,
        if (motion) tween(OscylTokens.slowMs, easing = OscylTokens.easing) else tween(0),
    )
    val mirrored = LocalLayoutDirection.current == LayoutDirection.Rtl
    SourceRow(
        modifier = if (folder) Modifier else Modifier.testTag("library-row"),
        title = line.name,
        caption = if (line.broken != null) stringResource(Res.string.library_unreadable) else programs,
        selected = selected,
        onClick = onClick,
        onLongClick = onMenu,
        // The same ⋯ as a program's row, in the same place: a library's menu is found, not guessed.
        onMenu = onMenu,
        unsaved = unsaved,
        state = state,
        depth = line.depth.toInt(),
        customActions = listOfNotNull(
            CustomAccessibilityAction(stringResource(Res.string.library_actions)) { onMenu(); true },
            if (!line.first) CustomAccessibilityAction(moveUp) { actions.stepNode(line.id, -1); true } else null,
            if (!line.last) CustomAccessibilityAction(moveDown) { actions.stepNode(line.id, 1); true } else null,
        ),
        leading = {
            Image(
                painterResource(if (folder) Res.drawable.ic_chevron else Res.drawable.ic_library),
                null,
                Modifier.size(16.dp).then(if (folder) Modifier.rotate(if (mirrored) -turn else turn) else Modifier),
                colorFilter = ColorFilter.tint(c.legend),
            )
        },
    )
}

/**
 * One source: a leading mark, the name (600 when selected) and a caption in `legend`,
 * an orange dot when [unsaved]. Selected, it sits on the flat `raised` plate.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun SourceRow(
    title: String,
    caption: String?,
    selected: Boolean,
    onClick: () -> Unit,
    leading: @Composable () -> Unit,
    onLongClick: (() -> Unit)? = null,
    unsaved: Boolean = false,
    state: String? = null,
    depth: Int = 0,
    customActions: List<CustomAccessibilityAction> = emptyList(),
    modifier: Modifier = Modifier,
    /** A synth's row: it keeps a strip for its read's tuning scale, shown at [progress] while it is read. */
    scale: Boolean = false,
    progress: Float? = null,
    /** Its menu, behind a ⋯ at the row's end (in `legend`; ink on the selected row). */
    onMenu: (() -> Unit)? = null,
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    val actionsLabel = stringResource(Res.string.library_actions)
    Row(
        modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .then(if (selected) Modifier.background(c.raised, shape) else Modifier)
            .drawBehind {
                // One guide per level, under the chevron of the folder above.
                for (d in 0 until depth) {
                    val x = (Indent * d + 16.dp).toPx()
                    val at = if (layoutDirection == LayoutDirection.Rtl) size.width - x else x
                    drawLine(c.rule, Offset(at, 0f), Offset(at, size.height), OscylTokens.ruleWidth.toPx())
                }
            }
            .combinedClickable(
                interactionSource = interaction,
                indication = null,
                onLongClickLabel = if (onLongClick != null) actionsLabel else null,
                onLongClick = onLongClick,
                onClick = onClick,
            )
            .semantics(mergeDescendants = true) {
                this.selected = selected
                state?.let { stateDescription = it }
                if (customActions.isNotEmpty()) this.customActions = customActions
            }
            .touchStates(interaction, shape, c.press)
            .padding(end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (depth > 0) Spacer(Modifier.width(Indent * depth))
        Box(Modifier.padding(start = 8.dp, end = 12.dp).size(16.dp), contentAlignment = Alignment.Center) { leading() }
        Column(Modifier.weight(1f).padding(vertical = 6.dp)) {
            BasicText(
                title,
                Modifier.fillMaxWidth(),
                style = type.bodyLarge.copy(
                    color = c.ink,
                    // One weight, selected or not: the plate says it, and no name changes width.
                    fontWeight = FontWeight.Medium,
                    textDirection = TextDirection.Content,
                    textAlign = startAlign(),
                ),
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            if (caption != null) {
                BasicText(caption, Modifier.fillMaxWidth(), style = type.bodySmall.copy(color = c.legend, fontFeatureSettings = "tnum", textDirection = TextDirection.Content, textAlign = startAlign()))
            }
            if (scale) ProgressScale(progress, Modifier.padding(top = 4.dp).widthIn(max = 240.dp))
        }
        if (unsaved) Box(Modifier.padding(start = 8.dp).size(6.dp).background(c.act, CircleShape).clearAndSetSemantics {})
        if (onMenu != null) IconAction(Res.drawable.ic_more, stringResource(Res.string.program_more, title), onMenu, tint = if (selected) null else c.legend)
    }
}
