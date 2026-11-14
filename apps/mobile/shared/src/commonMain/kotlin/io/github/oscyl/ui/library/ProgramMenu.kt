package io.github.oscyl.ui.library

import io.github.oscyl.ui.parts.disabled
import io.github.oscyl.ui.shell.LocalShellFrame
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.exclude
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.navigationBars
import io.github.oscyl.shared.generated.resources.program_play
import io.github.oscyl.shared.generated.resources.ic_play
import androidx.compose.ui.Alignment
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.WindowInsetsSides
import io.github.oscyl.ui.parts.safeInsets
import io.github.oscyl.ui.parts.FloatingShape
import io.github.oscyl.ui.parts.FloatingRadius
import io.github.oscyl.shared.generated.resources.ic_library
import io.github.oscyl.shared.generated.resources.ic_undo
import io.github.oscyl.shared.generated.resources.ic_keep
import io.github.oscyl.shared.generated.resources.ic_add
import io.github.oscyl.shared.generated.resources.ic_share
import io.github.oscyl.shared.generated.resources.ic_export
import io.github.oscyl.shared.generated.resources.ic_folder
import io.github.oscyl.shared.generated.resources.ic_remove
import io.github.oscyl.shared.generated.resources.ic_down
import io.github.oscyl.shared.generated.resources.ic_up
import io.github.oscyl.shared.generated.resources.ic_duplicate
import io.github.oscyl.shared.generated.resources.ic_paste
import io.github.oscyl.shared.generated.resources.ic_copy
import io.github.oscyl.shared.generated.resources.ic_rename
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.DrawableResource
import kotlinx.coroutines.launch
import io.github.oscyl.ui.parts.motionScale
import io.github.oscyl.ui.parts.LocalOscylMotion
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.Image
import androidx.compose.animation.core.tween
import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.input.TextFieldLineLimits
import androidx.compose.foundation.text.input.rememberTextFieldState
import androidx.compose.foundation.text.input.setTextAndSelectAll
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.produceState
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.tooling.preview.PreviewParameter
import io.github.oscyl.core.MenuEntry
import io.github.oscyl.core.OutlineLine
import io.github.oscyl.core.ProgramAction
import io.github.oscyl.core.ProgramMenu
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.core.ProgramRow
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.action_cancel
import io.github.oscyl.shared.generated.resources.action_rename
import io.github.oscyl.shared.generated.resources.library_copy_name
import io.github.oscyl.shared.generated.resources.menu_clear
import io.github.oscyl.shared.generated.resources.menu_copy
import io.github.oscyl.shared.generated.resources.menu_copy_to
import io.github.oscyl.shared.generated.resources.menu_duplicate
import io.github.oscyl.shared.generated.resources.menu_move_down
import io.github.oscyl.shared.generated.resources.menu_move_up
import io.github.oscyl.shared.generated.resources.menu_new
import io.github.oscyl.shared.generated.resources.menu_new_library
import io.github.oscyl.shared.generated.resources.menu_paste_over
import io.github.oscyl.shared.generated.resources.menu_remove
import io.github.oscyl.shared.generated.resources.menu_rename
import io.github.oscyl.shared.generated.resources.menu_revert
import io.github.oscyl.shared.generated.resources.name_field
import io.github.oscyl.shared.generated.resources.name_folder
import io.github.oscyl.shared.generated.resources.name_library
import io.github.oscyl.shared.generated.resources.name_program
import io.github.oscyl.shared.generated.resources.node_delete
import io.github.oscyl.shared.generated.resources.node_duplicate
import io.github.oscyl.shared.generated.resources.node_export
import io.github.oscyl.shared.generated.resources.node_export_format
import io.github.oscyl.shared.generated.resources.node_share
import io.github.oscyl.shared.generated.resources.node_move_down
import io.github.oscyl.shared.generated.resources.node_move_to
import io.github.oscyl.shared.generated.resources.node_move_up
import io.github.oscyl.shared.generated.resources.node_new_folder_here
import io.github.oscyl.shared.generated.resources.node_new_library_here
import io.github.oscyl.shared.generated.resources.node_rename
import io.github.oscyl.shared.generated.resources.node_top_level
import io.github.oscyl.shared.generated.resources.program_heading
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.floating
import io.github.oscyl.ui.parts.touchStates
import org.jetbrains.compose.resources.StringResource
import org.jetbrains.compose.resources.stringResource

/** Each program action's label. */
internal val ProgramAction.label: StringResource
    get() = when (this) {
        ProgramAction.RENAME -> Res.string.menu_rename
        ProgramAction.DUPLICATE -> Res.string.menu_duplicate
        ProgramAction.COPY -> Res.string.menu_copy
        ProgramAction.PASTE_OVER -> Res.string.menu_paste_over
        ProgramAction.COPY_TO_LIBRARY -> Res.string.menu_copy_to
        ProgramAction.NEW_PROGRAM -> Res.string.menu_new
        ProgramAction.MOVE_UP -> Res.string.menu_move_up
        ProgramAction.MOVE_DOWN -> Res.string.menu_move_down
        ProgramAction.REVERT -> Res.string.menu_revert
        ProgramAction.REMOVE -> Res.string.menu_remove
        ProgramAction.CLEAR -> Res.string.menu_clear
    }

/** Each program action's glyph. */
internal val ProgramAction.icon: DrawableResource
    get() = when (this) {
        ProgramAction.RENAME -> Res.drawable.ic_rename
        ProgramAction.DUPLICATE -> Res.drawable.ic_duplicate
        ProgramAction.COPY -> Res.drawable.ic_copy
        ProgramAction.PASTE_OVER -> Res.drawable.ic_paste
        ProgramAction.COPY_TO_LIBRARY -> Res.drawable.ic_keep
        ProgramAction.NEW_PROGRAM -> Res.drawable.ic_add
        ProgramAction.MOVE_UP -> Res.drawable.ic_up
        ProgramAction.MOVE_DOWN -> Res.drawable.ic_down
        ProgramAction.REVERT -> Res.drawable.ic_undo
        ProgramAction.REMOVE -> Res.drawable.ic_remove
        ProgramAction.CLEAR -> Res.drawable.ic_remove
    }

private enum class Page { Main, CopyTo, Rename }

/**
 * Program [index]'s menu as the core decides it: its sections in order, what is not
 * possible now shown but inert. Copy to library and Rename open a second page in the
 * same panel.
 */
@Composable
internal fun ProgramMenuDialog(
    row: ProgramRow,
    index: Int,
    actions: LibraryActions,
    onDismiss: () -> Unit,
    /** The program moved (up or down): the menu stays open on it, at its new place. */
    onMoved: (Int) -> Unit = {},
    /** Plays it on its synth (the row's swipe, said in words), when one plays it. */
    onPlay: (() -> Unit)? = null,
    /** A how-to line under the heading (the one-time word on holding and moving a row). */
    note: String? = null,
) {
    // Asked off the main thread; the dialog opens once the core has answered.
    val menu = produceState<ProgramMenu?>(null, index, row) { value = actions.menu(index) }.value ?: return
    var page by rememberSaveable { mutableStateOf(Page.Main) }
    val heading = stringResource(Res.string.program_heading, row.slot, row.name)
    when (page) {
        Page.Rename -> NameDialog(stringResource(Res.string.name_program), row.name, onDismiss) {
            actions.renameProgram(index, it)
            onDismiss()
        }
        Page.CopyTo -> MenuPanel(stringResource(Res.string.menu_copy_to), onDismiss) {
            menu.targets.forEach { target ->
                MenuItem(target.name, icon = Res.drawable.ic_library) { actions.copyTo(index, target.id); onDismiss() }
            }
            MenuItem(stringResource(Res.string.menu_new_library), icon = Res.drawable.ic_add) { actions.copyTo(index, null); onDismiss() }
        }
        Page.Main -> MenuPanel(heading, onDismiss) {
            if (note != null) {
                BasicText(
                    note,
                    Modifier.padding(start = 16.dp, end = 16.dp, bottom = 8.dp),
                    style = MaterialTheme.typography.bodySmall.copy(color = OscylTheme.colors.legend),
                )
            }
            if (onPlay != null) {
                MenuItem(stringResource(Res.string.program_play), icon = Res.drawable.ic_play) { onPlay(); onDismiss() }
                Rule()
            }
            ProgramMenuEntries(menu) { action ->
                when (action) {
                    ProgramAction.RENAME -> page = Page.Rename
                    ProgramAction.COPY_TO_LIBRARY -> page = Page.CopyTo
                    // Moving a program is often several steps: the menu stays, on the moved row.
                    ProgramAction.MOVE_UP -> { actions.act(index, action); onMoved(index - 1) }
                    ProgramAction.MOVE_DOWN -> { actions.act(index, action); onMoved(index + 1) }
                    else -> {
                        actions.act(index, action)
                        onDismiss()
                    }
                }
            }
        }
    }
}

/** The menu's sections, ruled apart. */
@Composable
private fun ProgramMenuEntries(menu: ProgramMenu, onPick: (ProgramAction) -> Unit) {
    menu.sections.forEachIndexed { s, section ->
        if (s > 0) Rule()
        section.forEach { entry ->
            MenuItem(stringResource(entry.action.label), enabled = entry.enabled, icon = entry.action.icon) { onPick(entry.action) }
        }
    }
}

private enum class NodePage { Main, MoveTo, Rename, Export }

/** A library's or folder's menu, as the desktop's: rename, duplicate, export, move, delete. */
@Composable
internal fun NodeMenu(
    line: OutlineLine,
    lines: List<OutlineLine>,
    actions: LibraryActions,
    onDismiss: () -> Unit,
) {
    var page by rememberSaveable { mutableStateOf(NodePage.Main) }
    val done = { f: () -> Unit -> f(); onDismiss() }
    val copyName = stringResource(Res.string.library_copy_name, line.name)
    when (page) {
        NodePage.Rename -> NameDialog(
            stringResource(if (line.isFolder) Res.string.name_folder else Res.string.name_library),
            line.name,
            onDismiss,
        ) { done { actions.renameNode(line.id, it) } }
        NodePage.MoveTo -> MenuPanel(stringResource(Res.string.node_move_to), onDismiss) {
            MenuItem(stringResource(Res.string.node_top_level), icon = Res.drawable.ic_library) { done { actions.moveNode(line.id, null) } }
            lines.filter { it.isFolder }.forEach { folder ->
                MenuItem(folder.name, indent = folder.depth.toInt(), icon = Res.drawable.ic_folder) { done { actions.moveNode(line.id, folder.id) } }
            }
        }
        NodePage.Export -> MenuPanel(stringResource(Res.string.node_export), onDismiss) {
            actions.exportFormats(line.id).forEach { format ->
                MenuItem(stringResource(Res.string.node_export_format, format.name, format.ext), icon = Res.drawable.ic_export) { done { actions.export(line.id, format.ext) } }
            }
        }
        NodePage.Main -> MenuPanel(line.name, onDismiss) {
            if (line.isFolder) {
                MenuItem(stringResource(Res.string.node_new_library_here), icon = Res.drawable.ic_add) { done { actions.newLibrary(line.id) } }
                MenuItem(stringResource(Res.string.node_new_folder_here), icon = Res.drawable.ic_folder) { done { actions.newFolder(line.id) } }
                Rule()
            }
            MenuItem(stringResource(Res.string.node_rename), icon = Res.drawable.ic_rename) { page = NodePage.Rename }
            if (!line.isFolder) {
                MenuItem(stringResource(Res.string.node_duplicate), icon = Res.drawable.ic_duplicate) { done { actions.duplicateLibrary(line.id, copyName) } }
                // A library Oscyl can't read has no programs to write out.
                val readable = line.broken == null
                MenuItem(stringResource(Res.string.node_export), enabled = readable, icon = Res.drawable.ic_export) { page = NodePage.Export }
                MenuItem(stringResource(Res.string.node_share), enabled = readable, icon = Res.drawable.ic_share) { done { actions.share(line.id) } }
            }
            Rule()
            MenuItem(stringResource(Res.string.node_move_up), enabled = !line.first, icon = Res.drawable.ic_up) { done { actions.stepNode(line.id, -1) } }
            MenuItem(stringResource(Res.string.node_move_down), enabled = !line.last, icon = Res.drawable.ic_down) { done { actions.stepNode(line.id, 1) } }
            if (!line.isFolder) MenuItem(stringResource(Res.string.node_move_to), icon = Res.drawable.ic_folder) { page = NodePage.MoveTo }
            Rule()
            MenuItem(stringResource(Res.string.node_delete), icon = Res.drawable.ic_remove) { done { actions.deleteNode(line.id) } }
        }
    }
}

/** Puts the panel it is asked in away with its exit, then runs what follows (outside a panel: at once). */
internal val LocalPanelLeave = staticCompositionLocalOf<(() -> Unit) -> Unit> { { it() } }

/**
 * A menu as the app's sheets are: it rises from the foot over a dimmed page, under the
 * thumb (8 dp off the sides and foot, its lower corners following the display's, at most
 * 640 dp wide), and scrolls when it must. Back, a tap outside or an action put it away.
 */
@Composable
internal fun MenuPanel(title: String, onDismiss: () -> Unit, content: @Composable ColumnScope.() -> Unit) {
    val motion = LocalOscylMotion.current
    val scope = rememberCoroutineScope()
    // How much of it is still there on the way out (1 at rest), and whether it is leaving.
    val there = remember { Animatable(1f) }
    val leave: (() -> Unit) -> Unit = remember(motion) {
        { then ->
            // Put away by Back, a tap outside or Cancel: it goes as it came, over `fast`, then is gone.
            if (!there.isRunning) {
                scope.launch {
                    val scale = motionScale(motion)
                    if (scale > 0f) there.animateTo(0f, tween((OscylTokens.fastMs * scale).toInt().coerceAtLeast(1), easing = OscylTokens.easing))
                    then()
                }
            }
        }
    }
    val rise = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        val scale = motionScale(motion)
        if (scale == 0f) rise.snapTo(1f) else rise.animateTo(1f, tween((OscylTokens.slowMs * scale).toInt().coerceAtLeast(1), easing = OscylTokens.easing))
    }
    Dialog(onDismissRequest = { leave(onDismiss) }, properties = io.github.oscyl.platform.panelDialogProperties()) {
        CompositionLocalProvider(LocalPanelLeave provides leave) {
            Box(
                Modifier
                    .fillMaxSize()
                    .pointerInput(Unit) { detectTapGestures { leave(onDismiss) } }
                    // The keyboard (past the navigation bar the panel keeps inside) places the
                    // panel, never resizes it: nothing changes size per frame as it rises.
                    .windowInsetsPadding(WindowInsets.ime.exclude(WindowInsets.navigationBars)),
                contentAlignment = Alignment.BottomCenter,
            ) {
                val frame = LocalShellFrame.current
                MenuSurface(
                    title,
                    Modifier
                        .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))
                        // Clear of a side island, which it would cover.
                        .padding(start = frame?.sideStart ?: 0.dp, end = frame?.sideEnd ?: 0.dp)
                        // Edge to edge like the sheets: its foot runs under the navigation bar.
                        .padding(top = 48.dp)
                        .widthIn(max = 640.dp)
                        .graphicsLayer {
                            // Rises from the foot; leaves the way it came, faster.
                            val t = rise.value * there.value
                            translationY = (1f - t) * size.height
                            alpha = if (t <= 0f) 0f else 1f
                        }
                        // Its own taps stay its own.
                        .pointerInput(Unit) { detectTapGestures {} },
                    RoundedCornerShape(topStart = FloatingRadius, topEnd = FloatingRadius),
                    content,
                )
            }
        }
    }
}

/** The panel itself: the floating part, its heading, then [content]. */
@Composable
private fun MenuSurface(title: String, modifier: Modifier = Modifier, shape: androidx.compose.ui.graphics.Shape = FloatingShape, content: @Composable ColumnScope.() -> Unit) {
    val c = OscylTheme.colors
    Column(
        modifier
            .fillMaxWidth()
            .floating(c, shape, edgeToEdge = true)
            .verticalScroll(rememberScrollState())
            .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Bottom))
            .padding(vertical = 8.dp),
    ) {
        BasicText(
            title,
            Modifier.padding(horizontal = 16.dp, vertical = 8.dp).semantics { heading() },
            style = MaterialTheme.typography.titleMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content),
        )
        content()
    }
}

@Composable
private fun Rule() {
    Spacer(Modifier.padding(vertical = 4.dp).fillMaxWidth().height(OscylTokens.ruleWidth).background(OscylTheme.colors.rule))
}

/**
 * One menu entry, 48 dp tall: its glyph, then its words (the ⋮ menu's own row). Not
 * [enabled], it stays in place in `legend` and does nothing.
 */
@Composable
internal fun MenuItem(text: String, enabled: Boolean = true, indent: Int = 0, icon: DrawableResource? = null, onClick: () -> Unit) {
    val c = OscylTheme.colors
    val interaction = remember { MutableInteractionSource() }
    val pen = if (enabled) c.ink else c.disabled
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .clickable(interaction, indication = null, enabled = enabled, role = Role.Button, onClick = onClick)
            .touchStates(interaction, RoundedCornerShape(0.dp), c.press)
            .padding(start = 16.dp + 18.dp * indent, end = 16.dp, top = 12.dp, bottom = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(14.dp),
        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
    ) {
        if (icon != null) Image(painterResource(icon), null, Modifier.size(20.dp), colorFilter = ColorFilter.tint(pen))
        BasicText(
            text,
            style = MaterialTheme.typography.bodyLarge.copy(
                color = pen,
                fontWeight = FontWeight.Medium,
                textDirection = TextDirection.Content,
            ),
        )
    }
}

/** Asks for a name: an outlined field holding [initial], selected, then Cancel and Rename. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun NameDialog(title: String, initial: String, onDismiss: () -> Unit, confirmWord: String? = null, onConfirm: (String) -> Unit) {
    val c = OscylTheme.colors
    val state = rememberTextFieldState()
    val focus = remember { FocusRequester() }
    val fieldLabel = stringResource(Res.string.name_field)
    LaunchedEffect(Unit) { state.setTextAndSelectAll(initial) }
    val confirm = { onConfirm(state.text.toString().trim()) }
    MenuPanel(title, onDismiss) {
        // Inside the dialog's window, once the field is there.
        LaunchedEffect(Unit) { focus.requestFocus() }
        BasicTextField(
            state,
            Modifier
                .padding(horizontal = 16.dp, vertical = 8.dp)
                .fillMaxWidth()
                .heightIn(min = 48.dp)
                .border(OscylTokens.ruleWidth, c.ruleStrong, RoundedCornerShape(OscylTokens.radius))
                .padding(horizontal = 12.dp, vertical = 12.dp)
                .focusRequester(focus)
                .semantics { contentDescription = fieldLabel },
            textStyle = MaterialTheme.typography.bodyLarge.copy(color = c.ink, textDirection = TextDirection.Content),
            cursorBrush = SolidColor(c.ink),
            lineLimits = TextFieldLineLimits.SingleLine,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
            onKeyboardAction = { confirm() },
        )
        // The outlines on the 16 dp the title, words and field stand on.
        FlowRow(
            Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, bottom = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp, androidx.compose.ui.Alignment.End),
        ) {
            val leave = LocalPanelLeave.current
            OutlinedAction(stringResource(Res.string.action_cancel), { leave(onDismiss) })
            OutlinedAction(confirmWord ?: stringResource(Res.string.action_rename), confirm)
        }
    }
}

/** Asks before something that can't be taken back here: [body], then Cancel and [confirm]. */
@Composable
internal fun ConfirmDialog(title: String, body: String, confirm: String, onDismiss: () -> Unit, onConfirm: () -> Unit) {
    MenuPanel(title, onDismiss) {
        BasicText(
            body,
            Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
            style = MaterialTheme.typography.bodyMedium.copy(color = OscylTheme.colors.ink, textDirection = TextDirection.Content),
        )
        // The outlines on the 16 dp the title, words and field stand on.
        FlowRow(
            Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, bottom = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp, androidx.compose.ui.Alignment.End),
        ) {
            val leave = LocalPanelLeave.current
            OutlinedAction(stringResource(Res.string.action_cancel), { leave(onDismiss) })
            OutlinedAction(confirm, onConfirm)
        }
    }
}

private val SampleMenu = ProgramMenu(
    sections = listOf(
        listOf(ProgramAction.RENAME to true, ProgramAction.COPY to true, ProgramAction.PASTE_OVER to false, ProgramAction.COPY_TO_LIBRARY to true),
        listOf(ProgramAction.NEW_PROGRAM to true, ProgramAction.DUPLICATE to true),
        listOf(ProgramAction.MOVE_UP to true, ProgramAction.MOVE_DOWN to true),
        listOf(ProgramAction.REVERT to true, ProgramAction.REMOVE to true),
    ).map { section -> section.map { (a, on) -> MenuEntry(a, on) } },
    targets = emptyList(),
)

@OscylPreviews
@Composable
private fun programMenu(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    MenuSurface(stringResource(Res.string.program_heading, "002", "Glass Bells")) { ProgramMenuEntries(SampleMenu) {} }
}

@OscylA11yPreviews
@Composable
private fun programMenu_a11y() = PreviewTheme {
    MenuSurface(stringResource(Res.string.program_heading, "002", "Glass Bells")) { ProgramMenuEntries(SampleMenu) {} }
}
