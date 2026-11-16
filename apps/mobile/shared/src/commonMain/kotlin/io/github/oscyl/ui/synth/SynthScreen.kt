package io.github.oscyl.ui.synth

import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.foundation.layout.wrapContentSize
import io.github.oscyl.ui.parts.LineArt
import io.github.oscyl.shared.generated.resources.backup_all
import io.github.oscyl.shared.generated.resources.backup_unit
import androidx.compose.ui.semantics.clearAndSetSemantics
import io.github.oscyl.shared.generated.resources.sheet_live_caption
import io.github.oscyl.ui.parts.ProgressScale
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import io.github.oscyl.shared.generated.resources.ic_add
import io.github.oscyl.shared.generated.resources.ic_backups
import io.github.oscyl.shared.generated.resources.ic_review
import io.github.oscyl.shared.generated.resources.ic_check
import io.github.oscyl.shared.generated.resources.ic_connect
import io.github.oscyl.shared.generated.resources.ic_disconnect
import io.github.oscyl.shared.generated.resources.ic_keep
import io.github.oscyl.shared.generated.resources.ic_read
import io.github.oscyl.shared.generated.resources.ic_units
import io.github.oscyl.shared.generated.resources.sheet_job_keep
import io.github.oscyl.shared.generated.resources.sheet_job_review
import io.github.oscyl.ui.parts.CountBox
import io.github.oscyl.ui.parts.EmptyArt
import org.jetbrains.compose.resources.DrawableResource
import org.jetbrains.compose.resources.painterResource
import io.github.oscyl.shared.generated.resources.empty_backups
import io.github.oscyl.ui.parts.EmptyState
import androidx.compose.ui.platform.testTag
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.unit.IntOffset
import io.github.oscyl.shared.generated.resources.action_back
import io.github.oscyl.shared.generated.resources.ic_back
import io.github.oscyl.shared.generated.resources.ic_close
import io.github.oscyl.shared.generated.resources.option_off
import io.github.oscyl.shared.generated.resources.sheet_add
import io.github.oscyl.shared.generated.resources.sheet_close
import io.github.oscyl.shared.generated.resources.sheet_live
import io.github.oscyl.shared.generated.resources.sheet_title
import io.github.oscyl.shared.generated.resources.sheet_units
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.DockedSheet
import io.github.oscyl.ui.parts.SheetTitle
import io.github.oscyl.ui.parts.touchStates
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateOf
import io.github.oscyl.shared.generated.resources.backups_put_back
import io.github.oscyl.shared.generated.resources.units_clear_confirm
import io.github.oscyl.shared.generated.resources.units_clear_body
import io.github.oscyl.shared.generated.resources.units_clear_title
import io.github.oscyl.ui.library.ConfirmDialog
import io.github.oscyl.core.BackupFile
import io.github.oscyl.core.ShelfUnitInfo
import io.github.oscyl.core.UnitParamInfo
import androidx.compose.foundation.selection.toggleable
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.stateDescription
import io.github.oscyl.shared.generated.resources.ic_remove
import io.github.oscyl.shared.generated.resources.units_params_hidden
import io.github.oscyl.shared.generated.resources.units_params_shown
import io.github.oscyl.shared.generated.resources.units_shelf
import io.github.oscyl.shared.generated.resources.units_shelf_add
import io.github.oscyl.shared.generated.resources.units_shelf_empty
import io.github.oscyl.shared.generated.resources.units_shelf_load
import io.github.oscyl.shared.generated.resources.units_shelf_load_named
import io.github.oscyl.shared.generated.resources.units_shelf_remove_named
import androidx.compose.runtime.LaunchedEffect
import org.jetbrains.compose.resources.pluralStringResource
import io.github.oscyl.shared.generated.resources.synth_read_progress
import io.github.oscyl.shared.generated.resources.synth_stop_reading
import io.github.oscyl.shared.generated.resources.synth_read_again
import io.github.oscyl.shared.generated.resources.synth_read_current
import io.github.oscyl.shared.generated.resources.synth_review
import io.github.oscyl.shared.generated.resources.units_empty
import io.github.oscyl.shared.generated.resources.units_load
import io.github.oscyl.shared.generated.resources.units_load_named
import io.github.oscyl.shared.generated.resources.units_clear
import io.github.oscyl.shared.generated.resources.units_clear_named
import io.github.oscyl.shared.generated.resources.backups_title
import io.github.oscyl.shared.generated.resources.backups_empty
import io.github.oscyl.shared.generated.resources.backups_open
import io.github.oscyl.shared.generated.resources.backups_share

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.intl.Locale
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.toUpperCase
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.oscyl.core.DriverInfo
import io.github.oscyl.core.KnobMark
import io.github.oscyl.core.PanelDrawing
import io.github.oscyl.core.UnitRect
import io.github.oscyl.platform.MidiPortInfo
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.synth_no_midi
import io.github.oscyl.shared.generated.resources.synth_action_allow
import io.github.oscyl.shared.generated.resources.synth_action_allow_named
import io.github.oscyl.shared.generated.resources.synth_action_connect
import io.github.oscyl.shared.generated.resources.synth_action_connect_named
import io.github.oscyl.shared.generated.resources.synth_action_disconnect
import io.github.oscyl.shared.generated.resources.synth_action_disconnect_named
import io.github.oscyl.shared.generated.resources.synth_demo_legend
import io.github.oscyl.shared.generated.resources.synth_empty_body
import io.github.oscyl.shared.generated.resources.synth_empty_title
import io.github.oscyl.shared.generated.resources.synth_state_access
import io.github.oscyl.shared.generated.resources.synth_state_connected
import io.github.oscyl.shared.generated.resources.synth_state_disconnected
import io.github.oscyl.shared.generated.resources.synth_state_unplugged
import io.github.oscyl.shared.generated.resources.synth_state_firmware
import io.github.oscyl.shared.generated.resources.synth_state_simulated
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.Lamp
import io.github.oscyl.ui.parts.Legend
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.Segments
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.OscylSizePreviews
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.shell.Destination
import io.github.oscyl.ui.shell.Shell
import org.jetbrains.compose.resources.stringResource

/** What a connected synth's card can do beyond connecting; every default does nothing (previews). */
interface SynthActions {
    fun readAll(card: SynthCard) {}
    fun cancelRead(card: SynthCard) {}
    fun readCurrent(card: SynthCard) {}

    /** Review the synth's edits and sync them (the island's review sheet, on its slots). */
    fun review(card: SynthCard) {}

    fun loadUnit(card: SynthCard, slot: UnitSlot) {}
    fun clearUnit(card: SynthCard, slot: UnitSlot) {}
    fun openBackup(file: BackupFile) {}
    fun shareBackup(file: BackupFile) {}

    /** Put a cleared unit back from its backup. */
    fun putBack(file: BackupFile) {}

    /** Pick a unit file for the synth's shelf. */
    fun addToShelf(card: SynthCard) {}

    /** Load a unit of the shelf into the first free slot of its section. */
    fun loadFromShelf(card: SynthCard, unit: ShelfUnitInfo) {}
    fun removeFromShelf(card: SynthCard, unit: ShelfUnitInfo) {}
}

/** Does nothing: previews. */
object NoSynthActions : SynthActions

/** The synth sheet's content over the app's [SynthModel]; [visible] while the sheet is in sight. */
@Composable
fun SynthScreen(
    model: SynthModel,
    onAbout: () -> Unit,
    modifier: Modifier = Modifier,
    onPreferences: () -> Unit = {},
    actions: SynthActions = NoSynthActions,
    onGuide: () -> Unit = {},
    edited: Map<ULong, Int> = emptyMap(),
    live: Live? = null,
    onAdd: () -> Unit = {},
    onClose: (() -> Unit)? = null,
    visible: Boolean = true,
) {
    val cards by model.cards.collectAsStateWithLifecycle()
    val access by model.access.collectAsStateWithLifecycle()
    val notice by model.notice.collectAsStateWithLifecycle()
    val units by model.units.collectAsStateWithLifecycle()
    val shelved by model.shelved.collectAsStateWithLifecycle()
    val backups by model.backups.collectAsStateWithLifecycle()
    // Kept composed under the foot, it asks nothing until it is in sight.
    LaunchedEffect(visible) { if (visible) model.refreshBackups() }
    // A logue synth's unit slots, listed once its card is in view.
    LaunchedEffect(cards, visible) { if (visible) cards.filter { it.connected }.forEach(model::listUnits) }
    SynthScreen(
        cards = cards,
        access = access,
        notice = notice,
        drivers = model.drivers,
        onDemo = model::openDemo,
        onDisconnect = model::disconnect,
        onConnect = model::connect,
        onAllow = model::allow,
        onAbout = onAbout,
        modifier = modifier,
        onPreferences = onPreferences,
        onGuide = onGuide,
        units = units,
        shelved = shelved,
        backups = backups,
        actions = actions,
        edited = edited,
        live = live,
        onAdd = onAdd,
        onClose = onClose,
        visible = visible,
    )
}

/**
 * The program's Live switch, on the card of the synth that plays it ([id]; null: the
 * first connected one).
 */
class Live(val on: Boolean, val id: ULong? = null, val toggle: () -> Unit)

/**
 * The synth sheet (DESIGN.md "Synth sheet"): a rack of nameplate cards, one per synth,
 * real or demo, in the order they connected. A card carries the synth's drawn panel,
 * its model, its lamp and connection line, a read's progress, Live (while the open
 * program can play on it), its jobs as tiles (the read or Stop, keeping the current
 * program, its user units, the review of its edits, the backups) and Disconnect as a
 * quiet glyph at its end; a synth that went is a card with Connect. Ports waiting for
 * USB access are rows with Allow. Units and Backups open as sub-pages, slid in on the
 * shared axis. A dashed plate at the end adds a synth (the picker).
 */
@Composable
fun SynthScreen(
    cards: List<SynthCard>,
    access: List<MidiPortInfo>,
    notice: String?,
    drivers: List<DriverInfo>,
    onDemo: (driverId: String) -> Unit,
    onDisconnect: (SynthCard) -> Unit,
    onConnect: (port: String) -> Unit,
    onAllow: (MidiPortInfo) -> Unit,
    onAbout: () -> Unit,
    modifier: Modifier = Modifier,
    onPreferences: () -> Unit = {},
    onGuide: () -> Unit = {},
    units: Map<ULong, List<UnitSlot>> = emptyMap(),
    /** Each synth model's shelf of unit files, by driver id. */
    shelved: Map<String, List<ShelfUnitInfo>> = emptyMap(),
    backups: List<BackupFile> = emptyList(),
    actions: SynthActions = NoSynthActions,
    edited: Map<ULong, Int> = emptyMap(),
    live: Live? = null,
    onAdd: () -> Unit = {},
    onClose: (() -> Unit)? = null,
    visible: Boolean = true,
    /** The sub-page it opens on (previews): "backups", or a synth's id for its units. */
    startPage: String? = null,
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    // The sub-page shown: units (a synth's id) or backups; null, the rack.
    var page by rememberSaveable { mutableStateOf(startPage) }
    // Put away, it opens on the rack again.
    LaunchedEffect(visible) { if (!visible) page = null }
    val motion = LocalOscylMotion.current
    val panels = remember(drivers) { drivers.associate { it.id to it.panel } }
    val firstConnected = cards.firstOrNull { it.connected }
    Column(modifier.fillMaxWidth()) {
        SheetTitle(
            if (page == "backups") stringResource(Res.string.backups_title) else if (page != null) stringResource(Res.string.sheet_units) else stringResource(Res.string.sheet_title),
            leading = if (page != null) ({ IconAction(Res.drawable.ic_back, stringResource(Res.string.action_back), { page = null }) }) else null,
        )
        AnimatedContent(
            page,
            Modifier.weight(1f, fill = false),
            transitionSpec = {
                if (!motion) {
                    EnterTransition.None togetherWith ExitTransition.None
                } else {
                    // Shared axis X: a sub-page comes in from the end, Back returns it there.
                    val forward = targetState != null
                    val spec = tween<IntOffset>(OscylTokens.slowMs, easing = OscylTokens.easing)
                    val fade = tween<Float>(OscylTokens.baseMs, easing = OscylTokens.easing)
                    (slideInHorizontally(spec) { w -> if (forward) w / 4 else -w / 4 } + fadeIn(fade)) togetherWith
                        (slideOutHorizontally(spec) { w -> if (forward) -w / 4 else w / 4 } + fadeOut(fade))
                }
            },
            label = "sheet page",
        ) { shown ->
            Column(
                Modifier
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
                    .padding(start = 12.dp, top = 4.dp, end = 12.dp, bottom = 16.dp + io.github.oscyl.ui.parts.LocalSheetFoot.current),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                when {
                    shown == "backups" -> Backups(backups, actions)
                    shown != null -> {
                        val card = cards.firstOrNull { it.id.toString() == shown }
                        val slots = card?.let { units[it.id] }
                        if (card != null && slots != null) UnitSlots(card, slots, shelved[card.driverId].orEmpty(), actions) else page = null
                    }
                    else -> {
                        cards.forEach { card ->
                            SynthPlate(
                                card,
                                panels[card.driverId],
                                live = live?.takeIf { card.connected && (it.id ?: firstConnected?.id) == card.id },
                                onConnect = { onConnect(card.port) },
                            ) {
                                val jobs = buildList<@Composable (Modifier) -> Unit> {
                                    if (card.read != null) {
                                        add { JobTile(Res.drawable.ic_close, stringResource(Res.string.synth_stop_reading), { actions.cancelRead(card) }, it) }
                                    } else {
                                        add { JobTile(Res.drawable.ic_read, stringResource(Res.string.synth_read_again), { actions.readAll(card) }, it) }
                                    }
                                    add {
                                        JobTile(Res.drawable.ic_keep, stringResource(Res.string.sheet_job_keep), { actions.readCurrent(card) }, it, described = stringResource(Res.string.synth_read_current))
                                    }
                                    units[card.id]?.takeIf { it.isNotEmpty() }?.let { slots ->
                                        add { JobTile(Res.drawable.ic_units, stringResource(Res.string.sheet_units) + "\n" + slots.count { s -> s.name != null }, { page = card.id.toString() }, it) }
                                    }
                                    val waiting = edited[card.id] ?: 0
                                    if (waiting > 0 && card.read == null) {
                                        add {
                                            JobTile(
                                                Res.drawable.ic_review, stringResource(Res.string.sheet_job_review), { actions.review(card) }, it,
                                                described = pluralStringResource(Res.plurals.synth_review, waiting, waiting), count = waiting,
                                            )
                                        }
                                    }
                                    if (backups.isNotEmpty() && card == firstConnected) {
                                        add { JobTile(Res.drawable.ic_backups, stringResource(Res.string.backups_title) + "\n" + backups.size, { page = "backups" }, it) }
                                    }
                                    // Last, with its word: a plug alone at the card's edge was a guess.
                                    if (card.connected) {
                                        add {
                                            JobTile(
                                                Res.drawable.ic_disconnect, stringResource(Res.string.synth_action_disconnect), { onDisconnect(card) }, it,
                                                described = stringResource(Res.string.synth_action_disconnect_named, card.model),
                                            )
                                        }
                                    }
                                }
                                Tiles(jobs)
                            }
                        }
                        access.forEach { port ->
                            StatusPanel(port.name, stringResource(Res.string.synth_state_access), lit = false,
                                stringResource(Res.string.synth_action_allow),
                                stringResource(Res.string.synth_action_allow_named, port.name)) { onAllow(port) }
                        }
                        if (notice != null) {
                            BasicText(
                                // An empty reason: the platform has no MIDI at all.
                                notice.ifEmpty { stringResource(Res.string.synth_no_midi) },
                                Modifier.padding(horizontal = 4.dp).semantics { liveRegion = LiveRegionMode.Polite },
                                style = type.bodyMedium.copy(color = c.ink, textDirection = TextDirection.Content),
                            )
                        }
                        // The backups outlive their synths: with none connected they stand alone.
                        if (backups.isNotEmpty() && firstConnected == null) {
                            Tiles(listOf { JobTile(Res.drawable.ic_backups, stringResource(Res.string.backups_title) + "\n" + backups.size, { page = "backups" }, it) })
                        }
                        if (cards.isEmpty() && access.isEmpty()) {
                            EmptyState(
                                stringResource(Res.string.synth_empty_title),
                                stringResource(Res.string.synth_empty_body),
                                Modifier.align(Alignment.CenterHorizontally),
                                art = EmptyArt.Cable,
                            ) { OutlinedAction(stringResource(Res.string.sheet_add), onAdd, leading = { Glyph(Res.drawable.ic_add) }) }
                        } else {
                            AddPlate(onAdd)
                        }
                    }
                }
            }
        }
    }
}

/** A 20 dp glyph in `ink`, silent. */
@Composable
private fun Glyph(icon: DrawableResource, modifier: Modifier = Modifier) {
    Image(painterResource(icon), null, modifier.size(20.dp), colorFilter = ColorFilter.tint(OscylTheme.colors.ink))
}

/**
 * One synth's nameplate card, on the ground's tone inside a 1 dp rule: its panel in
 * small, the model (and "Demo"), the lamp beside its connection line (a read's words
 * while it runs), and Disconnect as a quiet glyph at the end; under them the read's
 * tuning scale, Live, and its [jobs]. Not connected, the card's one action is Connect.
 */
@Composable
private fun SynthPlate(
    card: SynthCard,
    panel: PanelDrawing?,
    live: Live?,
    onConnect: () -> Unit,
    jobs: @Composable () -> Unit,
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val shape = RoundedCornerShape(OscylTokens.radiusPanel)
    val line = when {
        card.simulated -> stringResource(Res.string.synth_state_simulated)
        !card.plugged -> stringResource(Res.string.synth_state_unplugged)
        !card.connected -> stringResource(Res.string.synth_state_disconnected)
        card.firmware != null -> stringResource(Res.string.synth_state_firmware, card.firmware)
        else -> stringResource(Res.string.synth_state_connected)
    }
    Column(
        Modifier
            .fillMaxWidth()
            .background(c.ground, shape)
            .border(OscylTokens.ruleWidth, c.rule, shape)
            .padding(start = 12.dp, top = 8.dp, end = 12.dp, bottom = 12.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth().heightIn(min = 48.dp), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
            if (panel != null) PanelThumb(panel, Modifier.width(80.dp).aspectRatio(2.2f))
            Column(Modifier.weight(1f).semantics(mergeDescendants = true) {}, verticalArrangement = Arrangement.spacedBy(2.dp)) {
                BasicText(card.model, style = type.bodyLarge.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    Lamp(card.connected)
                    // While it is read, the line says how far (in the connection line's own place).
                    val read = card.read
                    // A screen reader hears the read at its start, each quarter and its end, not per program.
                    val quarter = read?.let { r -> if (r.total > 0) r.total * (r.done * 4 / r.total) / 4 else 0 }
                    val heard = read?.let { stringResource(Res.string.synth_read_progress, quarter!!, it.total) }
                    BasicText(
                        if (read != null) stringResource(Res.string.synth_read_progress, read.done, read.total) else line,
                        Modifier.then(if (heard != null) Modifier.clearAndSetSemantics { contentDescription = heard; liveRegion = LiveRegionMode.Polite } else Modifier),
                        style = type.bodyMedium.copy(color = c.legend, fontFeatureSettings = "tnum", textDirection = TextDirection.Content),
                    )
                }
                if (card.simulated) BasicText(stringResource(Res.string.synth_demo_legend).toUpperCase(Locale.current), style = type.labelMedium.copy(color = c.legend))
            }
        }
        // The read's tuning scale: its strip is always there, so a read moves nothing on the card.
        if (card.connected) ProgressScale(card.read?.let { r -> if (r.total > 0) r.done.toFloat() / r.total else 0f })
        if (live != null) {
            Segments(
                listOf(stringResource(Res.string.option_off), stringResource(Res.string.sheet_live)),
                selected = if (live.on) 1 else 0,
                saved = if (live.on) 1 else 0,
                onSelect = { if ((it == 1) != live.on) live.toggle() },
                modifier = Modifier.fillMaxWidth(),
                index = false,
            )
            // What Live means, under it: the one switch that decides whether edits are heard.
            BasicText(stringResource(Res.string.sheet_live_caption), style = type.bodySmall.copy(color = c.legend))
        }
        if (card.connected) {
            jobs()
        } else if (card.plugged) {
            val named = stringResource(Res.string.synth_action_connect_named, card.model)
            OutlinedAction(stringResource(Res.string.synth_action_connect), onConnect, Modifier.semantics { contentDescription = named }, leading = { Glyph(Res.drawable.ic_connect) })
        }
    }
}

/** The [tiles] in rows of three (two at large text), each row's tiles as tall as its tallest; a short row keeps its cells. */
@Composable
private fun Tiles(tiles: List<@Composable (Modifier) -> Unit>) {
    val columns = if (androidx.compose.ui.platform.LocalDensity.current.fontScale > 1.3f) 2 else 3
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        tiles.chunked(columns).forEach { row ->
            Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { it(Modifier.weight(1f).fillMaxHeight()) }
                repeat(columns - row.size) { Spacer(Modifier.weight(1f)) }
            }
        }
    }
}

/**
 * One job as a tile: its glyph over its [label], a whole tap target on `raised` in a
 * 1 dp rule; [count] (edits waiting) in the orange box beside the glyph. [described]
 * is what a screen reader says where the label is cut short.
 */
@Composable
private fun JobTile(icon: DrawableResource, label: String, onClick: () -> Unit, modifier: Modifier = Modifier, described: String? = null, count: Int = 0) {
    val c = OscylTheme.colors
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    Column(
        modifier
            .heightIn(min = 64.dp)
            .background(c.raised, shape)
            .border(OscylTokens.ruleWidth, c.rule, shape)
            .clickable(interaction, indication = null, role = Role.Button, onClick = onClick)
            .then(if (described != null) Modifier.semantics { contentDescription = described } else Modifier)
            .touchStates(interaction, shape, c.press)
            .padding(horizontal = 6.dp, vertical = 10.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(6.dp, Alignment.CenterVertically),
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
            Glyph(icon)
            if (count > 0) CountBox(count)
        }
        BasicText(
            label,
            style = MaterialTheme.typography.bodyMedium.copy(color = c.ink, fontWeight = FontWeight.Medium, textAlign = TextAlign.Center, textDirection = TextDirection.Content),
        )
    }
}

/** Add a synth: a dashed plate (a place still empty in the rack) with its + and its words. */
@Composable
private fun AddPlate(onAdd: () -> Unit) = DashedPlate(stringResource(Res.string.sheet_add), onAdd)

/**
 * A place still empty in the rack: a dashed `rule-strong` plate with a + and [text], in a
 * row, or [stacked] (the + over the words, centred: a tile in the picker's grid).
 */
@Composable
internal fun DashedPlate(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, stacked: Boolean = false) {
    val c = OscylTheme.colors
    val shape = RoundedCornerShape(OscylTokens.radiusPanel)
    val interaction = remember { MutableInteractionSource() }
    val plate = modifier
        .fillMaxWidth()
        .heightIn(min = 56.dp)
        .clickable(interaction, indication = null, role = Role.Button, onClick = onClick)
        .touchStates(interaction, shape, c.press)
        .drawBehind {
            val w = OscylTokens.ruleWidth.toPx()
            val r = OscylTokens.radiusPanel.toPx()
            drawRoundRect(
                c.ruleStrong, Offset(w / 2, w / 2), Size(size.width - w, size.height - w), CornerRadius(r),
                Stroke(w, pathEffect = PathEffect.dashPathEffect(floatArrayOf(4.dp.toPx(), 4.dp.toPx()))),
            )
        }
        .padding(horizontal = 16.dp, vertical = 12.dp)
    val words = @Composable {
        BasicText(
            text,
            style = (if (stacked) MaterialTheme.typography.bodyMedium else MaterialTheme.typography.bodyLarge)
                .copy(color = c.ink, fontWeight = FontWeight.Medium, textAlign = TextAlign.Center, textDirection = TextDirection.Content),
        )
    }
    if (stacked) {
        Column(plate, horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically)) {
            Glyph(Res.drawable.ic_add)
            words()
        }
    } else {
        Row(plate, horizontalArrangement = Arrangement.spacedBy(10.dp, Alignment.CenterHorizontally), verticalAlignment = Alignment.CenterVertically) {
            Glyph(Res.drawable.ic_add)
            words()
        }
    }
}

/**
 * A port waiting for USB access, in a ruled frame: the lamp beside its name and state
 * (read together), then one outlined action, which moves under them when they need the room.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun StatusPanel(
    model: String,
    caption: String,
    lit: Boolean,
    action: String,
    actionDescription: String,
    onAction: () -> Unit,
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val shape = RoundedCornerShape(OscylTokens.radiusPanel)
    FlowRow(
        Modifier
            .fillMaxWidth()
            .background(c.ground, shape)
            .border(OscylTokens.ruleWidth, c.rule, shape)
            .padding(start = 16.dp, top = 8.dp, end = 8.dp, bottom = 8.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        itemVerticalAlignment = Alignment.CenterVertically,
    ) {
        Row(
            Modifier.padding(vertical = 4.dp).padding(end = 8.dp).semantics(mergeDescendants = true) {},
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // A nameplate like every card's: which synth waits is not known yet (the port
            // says only its name), so the panel at rest in the line hand stands for it.
            Box(Modifier.width(56.dp).aspectRatio(2f), contentAlignment = Alignment.Center) {
                LineArt(EmptyArt.Panel, Modifier.wrapContentSize(unbounded = true).graphicsLayer { scaleX = 0.45f; scaleY = 0.45f })
            }
            Lamp(lit)
            Column {
                BasicText(model, style = type.bodyLarge.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content))
                BasicText(caption, style = type.bodyMedium.copy(color = c.legend, textDirection = TextDirection.Content))
            }
        }
        OutlinedAction(action, onAction, Modifier.semantics { contentDescription = actionDescription })
    }
}

/**
 * A unit's name on its row. One that declares parameters is a toggle: a tap shows them
 * under it, each with its range (said as shown or hidden), and another hides them.
 */
@Composable
private fun UnitName(name: String, caption: String?, params: List<UnitParamInfo>, lead: String? = null, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    var open by rememberSaveable(name, lead) { mutableStateOf(false) }
    val shown = stringResource(if (open) Res.string.units_params_shown else Res.string.units_params_hidden)
    Column(
        modifier
            .then(
                if (params.isEmpty()) Modifier.semantics(mergeDescendants = true) {}
                else Modifier
                    .clip(RoundedCornerShape(OscylTokens.radius))
                    .toggleable(open, role = Role.Button) { open = it }
                    .semantics(mergeDescendants = true) { stateDescription = shown },
            )
            .heightIn(min = 48.dp)
            .padding(vertical = 4.dp),
        verticalArrangement = Arrangement.Center,
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            lead?.let { BasicText(it, style = type.bodyMedium.copy(color = c.legend, fontFeatureSettings = "tnum")) }
            Column {
                BasicText(name, style = type.bodyMedium.copy(color = c.ink, fontWeight = FontWeight.Medium, textDirection = TextDirection.Content))
                caption?.let { BasicText(it, style = type.bodySmall.copy(color = c.legend)) }
                if (open) {
                    params.forEach { p ->
                        BasicText("${p.name} · ${p.range}", style = type.bodySmall.copy(color = c.legend, textDirection = TextDirection.Content))
                    }
                }
            }
        }
    }
}

/**
 * A logue synth's shelf of unit files (Add, then each unit with Load, into the first free
 * slot of its section, and a way off the shelf), then its user-unit slots by section:
 * each slot's unit or Empty, with Load or Clear.
 */
@Composable
private fun UnitSlots(card: SynthCard, slots: List<UnitSlot>, shelf: List<ShelfUnitInfo>, actions: SynthActions) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    // Clearing asks first, as on the desktop: it wipes the slot on the synth.
    var clearing by remember { mutableStateOf<UnitSlot?>(null) }
    clearing?.let { slot ->
        ConfirmDialog(
            stringResource(Res.string.units_clear_title, slot.name ?: ""),
            stringResource(Res.string.units_clear_body),
            stringResource(Res.string.units_clear_confirm),
            onDismiss = { clearing = null },
        ) {
            clearing = null
            actions.clearUnit(card, slot)
        }
    }
    Column(Modifier.padding(horizontal = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        FlowRow(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, itemVerticalAlignment = Alignment.CenterVertically) {
            Legend(stringResource(Res.string.units_shelf))
            OutlinedAction(stringResource(Res.string.units_shelf_add), { actions.addToShelf(card) }, Modifier.testTag("shelf-add"))
        }
        if (shelf.isEmpty()) {
            BasicText(stringResource(Res.string.units_shelf_empty), style = type.bodyMedium.copy(color = c.legend))
        }
        shelf.forEach { unit ->
            FlowRow(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                itemVerticalAlignment = Alignment.CenterVertically,
            ) {
                UnitName(unit.name, unit.section, unit.params, modifier = Modifier.weight(1f))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    val load = stringResource(Res.string.units_shelf_load_named, unit.name, unit.section)
                    OutlinedAction(stringResource(Res.string.units_shelf_load), { actions.loadFromShelf(card, unit) }, Modifier.semantics { contentDescription = load })
                    IconAction(Res.drawable.ic_remove, stringResource(Res.string.units_shelf_remove_named, unit.name), { actions.removeFromShelf(card, unit) })
                }
            }
        }
        // Previews and tests draw sample slots: no call to the core here.
        slots.groupBy { it.module }.forEach { (_, group) ->
            val section = group.first().section
            Legend(section, Modifier.padding(top = 8.dp))
            group.forEach { slot ->
                val number = (slot.slot.toInt() + 1).toString().padStart(2, '0')
                FlowRow(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    itemVerticalAlignment = Alignment.CenterVertically,
                ) {
                    if (slot.name != null && slot.params.isNotEmpty()) {
                        UnitName(slot.name, null, slot.params, lead = number, modifier = Modifier.weight(1f))
                    } else {
                        Row(Modifier.semantics(mergeDescendants = true) {}, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            BasicText(number, style = type.bodyMedium.copy(color = c.legend, fontFeatureSettings = "tnum"))
                            BasicText(
                                slot.name ?: stringResource(Res.string.units_empty),
                                style = type.bodyMedium.copy(color = if (slot.name == null) c.legend else c.ink, textDirection = TextDirection.Content),
                            )
                        }
                    }
                    if (slot.name == null) {
                        val described = stringResource(Res.string.units_load_named, section, number)
                        OutlinedAction(stringResource(Res.string.units_load), { actions.loadUnit(card, slot) }, Modifier.semantics { contentDescription = described })
                    } else {
                        val described = stringResource(Res.string.units_clear_named, slot.name)
                        OutlinedAction(stringResource(Res.string.units_clear), { clearing = slot }, Modifier.semantics { contentDescription = described })
                    }
                }
            }
        }
    }
}

/** The backups Oscyl took (before syncs and cleared unit slots), newest first: open one as a library, or share it. */
@Composable
private fun Backups(backups: List<BackupFile>, actions: SynthActions) {
    val c = OscylTheme.colors
    Column(Modifier.fillMaxWidth().padding(horizontal = 4.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        if (backups.isEmpty()) {
            EmptyState(stringResource(Res.string.empty_backups), stringResource(Res.string.backups_empty), Modifier.align(Alignment.CenterHorizontally))
        }
        backups.forEach { file ->
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                // In words: what it holds and when, not the file's name.
                val named = backupWords(file.name)
                BasicText(
                    named?.let { (what, unit) -> if (unit) stringResource(Res.string.backup_unit, what) else stringResource(Res.string.backup_all) } ?: file.name,
                    style = MaterialTheme.typography.bodyMedium.copy(color = c.ink, fontWeight = FontWeight.Medium, textDirection = TextDirection.Content),
                )
                named?.let { BasicText(backupWhen(file.name).orEmpty(), style = MaterialTheme.typography.bodySmall.copy(color = c.legend, fontFeatureSettings = "tnum")) }
                FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    // A unit's backup is its raw slot data: it shares, it does not open as a library.
                    if (file.name.endsWith(".slot.bin")) {
                        OutlinedAction(stringResource(Res.string.backups_put_back), { actions.putBack(file) })
                    } else {
                        OutlinedAction(stringResource(Res.string.backups_open), { actions.openBackup(file) })
                    }
                    OutlinedAction(stringResource(Res.string.backups_share), { actions.shareBackup(file) })
                }
            }
        }
    }
}

/** A backup file's name ("Synth backup 2026-11-13 10.04.11.mnlgxdlib", "MorphTbl 2026-…slot.bin") as what it holds: (name, a unit's). Null: not one Oscyl wrote. */
fun backupWords(file: String): Pair<String, Boolean>? {
    val m = BackupName.matchEntire(file) ?: return null
    val unit = file.endsWith(".slot.bin")
    return m.groupValues[1] to unit
}

/** When a backup was taken, from its name: "2026-11-13 · 10:04". */
fun backupWhen(file: String): String? = BackupName.matchEntire(file)?.groupValues?.let { g -> "${g[2]} · ${g[3]}:${g[4]}" }

private val BackupName = Regex("""^(.*?) (\d{4}-\d{2}-\d{2}) (\d{2})\.(\d{2})\.\d{2}\..+$""")

/** Previews never call the core: sample nameplates, drawn as the core would. */
internal fun plate(cols: Int) = PanelDrawing(
    (0 until cols).flatMap { c -> (0 until 2).map { r -> KnobMark(0.45f + c * 0.09f, 0.3f + r * 0.35f, 0.05f) } },
    UnitRect(0.06f, 0.25f, 0.24f, 0.2f),
)

private val SampleDrivers = listOf(
    DriverInfo("korg-minilogue-xd", "KORG", "minilogue xd", "4 voices", 2u, plate(6)),
    DriverInfo("korg-prologue", "KORG", "prologue", "8 or 16 voices", 2u, plate(5)),
    DriverInfo("sequential-prophet-rev2", "Sequential", "Prophet Rev2", "8 or 16 voices", 2u, plate(6)),
)

/** The screen in the app's shell, as it stands beside or above the island. */
@Composable
private fun SynthPreview(
    mode: OscylThemeMode = OscylThemeMode.Dark,
    cards: List<SynthCard> = emptyList(),
    access: List<MidiPortInfo> = emptyList(),
    units: Map<ULong, List<UnitSlot>> = emptyMap(),
    backups: List<BackupFile> = emptyList(),
    actions: SynthActions = NoSynthActions,
    edited: Map<ULong, Int> = emptyMap(),
    startPage: String? = null,
    shelved: Map<String, List<ShelfUnitInfo>> = emptyMap(),
) = OscylTheme(mode) {
    // One frame for the shell and the sheet, as in the app: the sheet keeps clear of a side island.
    Box { androidx.compose.runtime.CompositionLocalProvider(io.github.oscyl.ui.shell.LocalShellFrame provides remember { io.github.oscyl.ui.shell.ShellFrame() }) {
        Shell(Destination.Library, onSelect = {}, saveCount = 0, onSave = {}) { modifier -> Box(modifier) }
        // Over the shell, island and all, as in the app.
        DockedSheet(open = true, onClose = {}, title = stringResource(Res.string.sheet_title)) {
            SynthScreen(
                cards, access, notice = null, SampleDrivers, onDemo = {}, onDisconnect = {}, onConnect = {}, onAllow = {}, onAbout = {},
                units = units, shelved = shelved, backups = backups, actions = actions, edited = edited,
                live = Live(true, toggle = {}).takeIf { cards.any { it.connected } }, onClose = {}, startPage = startPage,
            )
        }
    } }
}

private val SampleParams = listOf(UnitParamInfo("Wave A", "1 to 46"), UnitParamInfo("Sub Mix", "0 % to 100 %"))

private val SampleShelf = listOf(
    ShelfUnitInfo("anthem.mnlgxdunit", "anthem", "osc", "Oscillators", SampleParams),
    ShelfUnitInfo("shimmer.mnlgxdunit", "shimmer", "revfx", "Reverbs", emptyList()),
)

private val SampleUnits = listOf(
    UnitSlot("osc", 0u, "MorphTbl", "Oscillators"), UnitSlot("osc", 1u, "waves", "Oscillators", SampleParams), UnitSlot("osc", 2u, null, "Oscillators"),
    UnitSlot("modfx", 0u, "Wobbler", "Modulation effects"), UnitSlot("modfx", 1u, null, "Modulation effects"),
)

private val SampleBackups = listOf(
    BackupFile("/b/1", "Synth backup 2026-11-13 10.04.11.mnlgxdlib", 2u),
    BackupFile("/b/2", "MorphTbl 2026-11-12 18.20.03.slot.bin", 1u),
)

/** The demo card with three edits waiting. */
private val EditedOne = mapOf(1uL to 3)

private val DemoCard = SynthCard(1u, "minilogue xd", firmware = null, simulated = true, port = "", driverId = "korg-minilogue-xd")
private val UsbCard = SynthCard(2u, "prologue", firmware = "2.10", simulated = false, port = "9:prologue _ SOUND", driverId = "korg-prologue")

@OscylPreviews
@Composable
private fun synth_empty(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = SynthPreview(mode)

@OscylPreviews
@Composable
private fun synth_demo_connected(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    SynthPreview(mode, cards = listOf(DemoCard, UsbCard))

@OscylPreviews
@Composable
private fun synth_permission(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    SynthPreview(mode, access = listOf(MidiPortInfo("minilogue xd", "7")))

@OscylPreviews
@Composable
private fun synth_disconnected(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    SynthPreview(mode, cards = listOf(UsbCard.copy(connected = false)))

@OscylSizePreviews
@Composable
private fun synth() = SynthPreview(cards = listOf(DemoCard, UsbCard))

@OscylPreviews
@Composable
private fun synth_reading(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    SynthPreview(mode, cards = listOf(DemoCard.copy(read = ReadProgress(212, 500))), backups = SampleBackups)

@OscylPreviews
@Composable
private fun synth_units(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    SynthPreview(mode, cards = listOf(DemoCard), units = mapOf(1uL to SampleUnits), backups = SampleBackups, edited = EditedOne)

@OscylA11yPreviews
@Composable
private fun synth_units_a11y() =
    SynthPreview(cards = listOf(DemoCard), units = mapOf(1uL to SampleUnits), backups = SampleBackups, edited = EditedOne)

@OscylPreviews
@Composable
private fun synth_unitsPage(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    SynthPreview(mode, cards = listOf(DemoCard), units = mapOf(1uL to SampleUnits), startPage = "1", shelved = mapOf(DemoCard.driverId to SampleShelf))

@OscylPreviews
@Composable
private fun synth_backupsPage(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    SynthPreview(mode, cards = listOf(DemoCard), backups = SampleBackups, startPage = "backups")

@OscylA11yPreviews
@Composable
private fun synth_a11y() = SynthPreview(access = listOf(MidiPortInfo("minilogue xd", "7")))

@OscylPreviews
@Composable
private fun synth_picker(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = OscylTheme(mode) {
    Shell(Destination.Library, onSelect = {}, saveCount = 0, onSave = {}) { modifier ->
        SynthPicker(SampleDrivers, emptyList(), emptyList(), onDemo = {}, onAllow = {}, onConnect = {}, onBack = {}, modifier = modifier)
    }
}

@OscylA11yPreviews
@Composable
private fun synth_picker_a11y() = OscylTheme(OscylThemeMode.Dark) {
    Shell(Destination.Library, onSelect = {}, saveCount = 0, onSave = {}) { modifier ->
        SynthPicker(SampleDrivers, emptyList(), emptyList(), onDemo = {}, onAllow = {}, onConnect = {}, onBack = {}, modifier = modifier)
    }
}
