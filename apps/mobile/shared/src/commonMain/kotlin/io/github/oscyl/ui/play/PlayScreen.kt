package io.github.oscyl.ui.play

import io.github.oscyl.ui.parts.barLift
import io.github.oscyl.ui.parts.PickerChevron
import io.github.oscyl.shared.generated.resources.editor_not_read
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.oscyl.core.ControlKind
import io.github.oscyl.core.FieldState
import io.github.oscyl.core.PlayLayout
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.editor_read_only
import io.github.oscyl.shared.generated.resources.play_across
import io.github.oscyl.shared.generated.resources.play_assign
import io.github.oscyl.shared.generated.resources.play_choose
import io.github.oscyl.shared.generated.resources.play_choose_pad
import io.github.oscyl.shared.generated.resources.play_knob
import io.github.oscyl.shared.generated.resources.play_none
import io.github.oscyl.shared.generated.resources.play_title
import io.github.oscyl.shared.generated.resources.play_up
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.editor.EditorActions
import io.github.oscyl.ui.editor.EditorHead
import io.github.oscyl.ui.editor.EditorModel
import io.github.oscyl.ui.editor.NoEditorActions
import io.github.oscyl.ui.editor.PanelState
import io.github.oscyl.ui.editor.SampleHead
import io.github.oscyl.ui.editor.samplePanel
import io.github.oscyl.ui.parts.BackHeader
import io.github.oscyl.ui.parts.Dial
import io.github.oscyl.ui.parts.DialSize
import io.github.oscyl.ui.parts.DockedSheet
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.SheetTitle
import io.github.oscyl.ui.parts.XyPad
import io.github.oscyl.ui.parts.liftedPlate
import io.github.oscyl.ui.parts.safeInsets
import io.github.oscyl.ui.parts.stageIcon
import io.github.oscyl.ui.parts.startAlign
import io.github.oscyl.ui.parts.touchStates
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.OscylSizePreviews
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.shell.LocalIslandClearance
import io.github.oscyl.ui.shell.TopBarActions
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.stringResource
import kotlin.math.roundToInt

/** The play page over the app's models. */
@Composable
fun PlayScreen(model: PlayModel, editor: EditorModel, onBack: () -> Unit, modifier: Modifier = Modifier) {
    val layout by model.layout.collectAsStateWithLifecycle()
    val head by editor.head.collectAsStateWithLifecycle()
    PlayScreen(head, editor.panel, layout, editor, model::assign, onBack, modifier)
}

/**
 * A page above the editor for playing the open program (DESIGN.md "Play page"): a
 * large XY pad on two parameters and four macro knobs, each assignable and kept per
 * synth model. Edits go through the editor's own actions: the synth plays them, one
 * gesture is one undo step, and the review counts them. Portrait: the pad, then the
 * knobs in a row; landscape or wide: the pad at the start, the knobs two by two at the
 * end. A tap on an axis or a knob's legend opens the docked sheet of the program's
 * dials, by stage, "None" first. A field this program does not show is unassigned here
 * (the model's layout keeps it). A factory slot or one not read yet shows the values
 * with the controls off.
 */
@Composable
fun PlayScreen(
    head: EditorHead?,
    panel: PanelState?,
    layout: PlayLayout,
    actions: EditorActions,
    onAssign: (PlaySlot, UInt?) -> Unit,
    onBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    // The slot whose sheet is open (an ordinal; -1 none): kept across rotation.
    var assigning by rememberSaveable { mutableIntStateOf(-1) }
    val writable = head != null && head.loaded && !head.readOnly
    fun shown(f: UInt?): FieldState? = f?.let { panel?.field(it) }?.takeIf { it.relevant && it.control is ControlKind.Dial }
    // The stages the page reads stay current while it is open.
    val pages = remember(panel, layout) {
        val fields = listOfNotNull(layout.x, layout.y) + layout.macros.filterNotNull()
        panel?.pages?.indices?.filter { p -> panel.pages[p].sections.any { s -> s.fields.any { it in fields } } }?.toSet().orEmpty()
    }
    LaunchedEffect(pages) { actions.playing(pages) }
    DisposableEffect(Unit) { onDispose { actions.playing(emptySet()) } }
    Box(modifier.fillMaxSize()) {
        BoxWithConstraints(
            Modifier
                .fillMaxSize()
                .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))
                .padding(bottom = LocalIslandClearance.current),
        ) {
            val wide = maxWidth > maxHeight
            // Wide: the header and the axes' row stay in sight under the pad.
            val padSide = if (wide) maxHeight - 136.dp else minOf(maxWidth - 32.dp, maxHeight * 0.6f)
            // Large text or a phone's width: two knobs to a row, so no knob, label or its
            // "Knob n" key is cut or broken over two lines (four need about 110 dp each).
            val large = LocalDensity.current.fontScale > 1.3f || maxWidth < 480.dp
            // The page under the header lifts it as it scrolls, as every bar does.
            val scroll = rememberScrollState()
            val liftAt = with(LocalDensity.current) { 8.dp.toPx() }
            Column(Modifier.fillMaxSize()) {
                Row(Modifier.barLift(c) { if (wide) 0f else (scroll.value / liftAt).coerceIn(0f, 1f) }, verticalAlignment = Alignment.CenterVertically) {
                    BackHeader(head?.name ?: stringResource(Res.string.play_title), onBack, Modifier.weight(1f))
                    TopBarActions(compact = true)
                }
                if (head != null && !writable) {
                    BasicText(
                        stringResource(if (head.loaded) Res.string.editor_read_only else Res.string.editor_not_read),
                        Modifier.padding(horizontal = 16.dp),
                        style = type.bodySmall.copy(color = c.legend),
                    )
                }
                val pad = @Composable {
                    Pad(shown(layout.x), shown(layout.y), writable, actions, Modifier.widthIn(max = padSide)) { assigning = it.ordinal }
                }
                // Two knobs with one name ("Shape", "Shape") say whose: their section's or page's title first.
                val macroFields = layout.macros.map { shown(it) }
                val names = macroFields.map { f ->
                    f ?: return@map null
                    val twin = macroFields.count { it?.label == f.label } > 1
                    if (!twin) return@map f.label
                    val page = panel?.pages?.firstOrNull { p -> p.sections.any { f.field in it.fields } }
                    val where = page?.sections?.firstOrNull { f.field in it.fields }?.title ?: page?.title
                    if (where != null) "$where ${f.label}" else f.label
                }
                val knobs = @Composable { columns: Int ->
                    Macros(macroFields, names, head?.comparing == true, writable, actions, columns) { assigning = it.ordinal }
                }
                if (wide) {
                    Row(
                        Modifier.fillMaxSize().padding(horizontal = 16.dp),
                        horizontalArrangement = Arrangement.spacedBy(24.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        // Scrolls only when its axes' row wraps (large text in a short window).
                        Box(Modifier.weight(1f).fillMaxHeight().verticalScroll(rememberScrollState()), contentAlignment = Alignment.Center) { pad() }
                        Box(Modifier.weight(1f).fillMaxHeight().verticalScroll(rememberScrollState()), contentAlignment = Alignment.Center) { knobs(2) }
                    }
                } else {
                    Column(
                        Modifier.fillMaxSize().verticalScroll(scroll).padding(horizontal = 16.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(16.dp),
                    ) {
                        pad()
                        knobs(if (large) 2 else 4)
                    }
                }
            }
        }
        val slot = PlaySlot.entries.getOrNull(assigning)
        // Keep its words while it sinks: a closed sheet still draws its last title.
        val name = slot?.let { slotName(it) } ?: ""
        DockedSheet(open = slot != null, onClose = { assigning = -1 }, title = stringResource(Res.string.play_assign, name)) {
            Column(Modifier.fillMaxWidth()) {
                SheetTitle(stringResource(Res.string.play_assign, name))
                Column(Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState()).padding(start = 8.dp, end = 8.dp, bottom = 16.dp + io.github.oscyl.ui.parts.LocalSheetFoot.current)) {
                    val current = slot?.let { layout.of(it) }
                    FieldRow(stringResource(Res.string.play_none), chosen = slot != null && current == null, tag = "play-none") {
                        slot?.let { onAssign(it, null) }
                        assigning = -1
                    }
                    panel?.pages?.forEachIndexed { p, page ->
                        val dials = page.sections.flatMap { it.fields }.distinct().mapNotNull { shown(it) }
                        if (dials.isEmpty()) return@forEachIndexed
                        Row(Modifier.padding(start = 8.dp, top = 12.dp, bottom = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                            Image(painterResource(stageIcon(page.icon)), null, Modifier.size(16.dp), colorFilter = ColorFilter.tint(c.legend))
                            BasicText(
                                page.title,
                                Modifier.padding(start = 8.dp).semantics { heading() },
                                style = type.labelMedium.copy(color = c.legend),
                            )
                        }
                        dials.forEach { f ->
                            FieldRow(f.label, chosen = current == f.field, tag = "play-field-${f.field}") {
                                slot?.let { onAssign(it, f.field) }
                                assigning = -1
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun slotName(slot: PlaySlot): String = when (slot) {
    PlaySlot.X -> stringResource(Res.string.play_across)
    PlaySlot.Y -> stringResource(Res.string.play_up)
    else -> stringResource(Res.string.play_knob, slot.ordinal - 1)
}

/** A field's value as the pad's 0..1. */
private fun FieldState.fraction(): Float {
    val d = control as ControlKind.Dial
    val span = (d.max - d.min).toFloat().coerceAtLeast(1f)
    return (raw.toFloat() - d.min.toFloat()) / span
}

private fun FieldState.savedFraction(): Float {
    val d = control as ControlKind.Dial
    val span = (d.max - d.min).toFloat().coerceAtLeast(1f)
    return (savedRaw.toFloat() - d.min.toFloat()) / span
}

private fun FieldState.rawAt(fraction: Float): Int {
    val d = control as ControlKind.Dial
    return (d.min.toFloat() + fraction.coerceIn(0f, 1f) * (d.max - d.min).toFloat()).roundToInt()
}

/** The pad and its two axes' legends (each opens the sheet), or the offer to choose them. */
@Composable
private fun Pad(x: FieldState?, y: FieldState?, enabled: Boolean, actions: EditorActions, modifier: Modifier, onAssign: (PlaySlot) -> Unit) {
    val c = OscylTheme.colors
    Column(modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        if (x != null && y != null) {
            XyPad(
                x.fraction(), y.fraction(),
                x.savedFraction(), y.savedFraction(),
                x.label, y.label, x.text, y.text,
                onMove = { fx, fy -> actions.setAll(listOf(x.field to x.rawAt(fx), y.field to y.rawAt(fy))) },
                onRelease = actions::release,
                enabled = enabled,
                onRestore = {
                    actions.setAll(listOf(x.field to x.savedRaw.toInt(), y.field to y.savedRaw.toInt()))
                    actions.release()
                },
            )
        } else {
            // Unassigned: the pad's place, ruled and dashed, offering to choose.
            Box(Modifier.fillMaxWidth().aspectRatio(1f), contentAlignment = Alignment.Center) {
                Canvas(Modifier.fillMaxSize()) {
                    drawRoundRect(
                        c.ruleStrong,
                        cornerRadius = androidx.compose.ui.geometry.CornerRadius(OscylTokens.radiusPanel.toPx()),
                        style = Stroke(OscylTokens.ruleWidth.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(4.dp.toPx(), 4.dp.toPx()))),
                    )
                }
                OutlinedAction(stringResource(Res.string.play_choose_pad), { onAssign(if (x == null) PlaySlot.X else PlaySlot.Y) }, Modifier.testTag("play-choose-pad"))
            }
        }
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedAction("${stringResource(Res.string.play_across)}: ${x?.label ?: stringResource(Res.string.play_none)}", { onAssign(PlaySlot.X) }, Modifier.testTag("play-assign-X"), trailing = { PickerChevron() }, role = Role.DropdownList)
            OutlinedAction("${stringResource(Res.string.play_up)}: ${y?.label ?: stringResource(Res.string.play_none)}", { onAssign(PlaySlot.Y) }, Modifier.testTag("play-assign-Y"), trailing = { PickerChevron() }, role = Role.DropdownList)
        }
    }
}

/** The four macro knobs ([columns] to a row), each with its legend as a 48 dp target that opens the sheet. */
@Composable
private fun Macros(fields: List<FieldState?>, names: List<String?>, comparing: Boolean, enabled: Boolean, actions: EditorActions, columns: Int, onAssign: (PlaySlot) -> Unit) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        fields.chunked(columns).forEachIndexed { r, row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEachIndexed { k, f ->
                    val n = r * columns + k
                    val slot = PlaySlot.entries[n + 2]
                    Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
                        if (f != null) {
                            val d = f.control as ControlKind.Dial
                            Dial(
                                f.raw.toInt(), f.savedRaw.toInt(), d.min.toInt()..d.max.toInt(), names.getOrNull(n) ?: f.label,
                                format = { actions.format(f.field, it) },
                                onChange = { actions.set(f.field, it) },
                                size = DialSize.Small,
                                comparing = comparing,
                                onRelease = actions::release,
                                modifier = Modifier.testTag("play-macro-$n"),
                                // Off on a program that cannot change: shown and said, not turned.
                                enabled = enabled,
                            )
                        } else {
                            Canvas(Modifier.size(64.dp)) {
                                drawCircle(c.ruleStrong, style = Stroke(OscylTokens.ruleWidth.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(3.dp.toPx(), 3.dp.toPx()))))
                            }
                        }
                        // Framed, so it reads as the button it is: it chooses what the knob turns.
                        OutlinedAction(
                            if (f == null) stringResource(Res.string.play_choose) else stringResource(Res.string.play_knob, n + 1),
                            { onAssign(slot) },
                            Modifier.testTag("play-assign-${slot.name}"),
                            // It opens a choice: the chevron says so, as a select field's does.
                            trailing = { PickerChevron() },
                            role = Role.DropdownList,
                        )
                    }
                }
            }
        }
    }
}

/** One choice in the sheet: the field's name, on the lifted plate when it is the one assigned. */
@Composable
private fun FieldRow(label: String, chosen: Boolean, tag: String, onClick: () -> Unit) {
    val c = OscylTheme.colors
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    Box(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .then(if (chosen) Modifier.liftedPlate(c, shape) else Modifier)
            .clickable(interaction, indication = null, role = Role.Button, onClick = onClick)
            .semantics { selected = chosen }
            .touchStates(interaction, shape, c.press)
            .padding(horizontal = 16.dp, vertical = 12.dp)
            .testTag(tag),
        contentAlignment = Alignment.CenterStart,
    ) {
        BasicText(
            label,
            style = MaterialTheme.typography.bodyLarge.copy(
                color = if (chosen) c.onPlate else c.ink,
                fontWeight = if (chosen) FontWeight.SemiBold else FontWeight.Medium,
                textDirection = TextDirection.Content,
                textAlign = startAlign(),
            ),
        )
    }
}

// Previews: the sample panel's Filter (cutoff and resonance on the pad) and four dials.

private fun samplePlay(panel: PanelState): PlayLayout {
    val dials = panel.pages.indices.flatMap { p -> panel.pages[p].sections.flatMap { it.fields } }
        .mapNotNull { panel.field(it) }.filter { it.control is ControlKind.Dial }
    val cutoff = dials.first { it.label == "Cutoff" }.field
    val resonance = dials.first { it.label == "Resonance" }.field
    return PlayLayout(cutoff, resonance, dials.map { it.field }.filter { it != cutoff && it != resonance }.take(4))
}

@Composable
private fun PlayPreview(mode: OscylThemeMode = OscylThemeMode.Dark, unassigned: Boolean = false) = OscylTheme(mode) {
    val panel = remember { samplePanel(start = 3) }
    val layout = if (unassigned) PlayLayout(null, null, List(4) { null }) else samplePlay(panel)
    Box(Modifier.fillMaxSize().background(OscylTheme.colors.ground)) {
        PlayScreen(SampleHead, panel, layout, NoEditorActions, onAssign = { _, _ -> }, onBack = {})
    }
}

@OscylPreviews
@Composable
private fun play(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PlayPreview(mode)

@OscylPreviews
@Composable
private fun play_unassigned(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PlayPreview(mode, unassigned = true)

@OscylA11yPreviews
@Composable
private fun play_a11y() = PlayPreview()

@OscylSizePreviews
@Composable
private fun play_sizes() = PlayPreview()
