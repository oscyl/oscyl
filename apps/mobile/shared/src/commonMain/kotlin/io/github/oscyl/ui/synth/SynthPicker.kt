package io.github.oscyl.ui.synth

import androidx.compose.ui.platform.LocalDensity
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import io.github.oscyl.ui.parts.barLift
import androidx.compose.ui.Alignment
import io.github.oscyl.shared.generated.resources.ic_search
import org.jetbrains.compose.resources.painterResource
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.Image
import io.github.oscyl.shared.generated.resources.picker_none_body
import io.github.oscyl.shared.generated.resources.empty_plugged
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.EmptyState
import androidx.compose.ui.platform.testTag
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.oscyl.core.DriverInfo
import io.github.oscyl.core.PanelDrawing
import io.github.oscyl.platform.MidiPortInfo
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.picker_ask
import io.github.oscyl.shared.generated.resources.picker_demo
import io.github.oscyl.shared.generated.resources.picker_none
import io.github.oscyl.shared.generated.resources.picker_plugged
import io.github.oscyl.shared.generated.resources.picker_search
import io.github.oscyl.shared.generated.resources.picker_title
import io.github.oscyl.shared.generated.resources.picker_waiting
import io.github.oscyl.shared.generated.resources.synth_action_allow
import io.github.oscyl.shared.generated.resources.synth_action_allow_named
import io.github.oscyl.shared.generated.resources.synth_action_connect
import io.github.oscyl.shared.generated.resources.synth_action_connect_named
import io.github.oscyl.shared.generated.resources.synth_state_access
import io.github.oscyl.shared.generated.resources.synth_state_disconnected
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.BackHeader
import io.github.oscyl.ui.parts.EmptyArt
import io.github.oscyl.ui.parts.Segments
import io.github.oscyl.ui.parts.liftedPlate
import io.github.oscyl.ui.parts.safeInsets
import io.github.oscyl.ui.parts.touchStates
import io.github.oscyl.ui.shell.LocalIslandClearance
import org.jetbrains.compose.resources.stringResource

/** Where a missing synth is asked for: the new-synth request form. */
const val DriverRequestUrl = "https://github.com/oscyl/oscyl/issues/new?template=new_synth_request.yml"

/**
 * Add a synth (DESIGN.md "Synth picker"): a search over models and makers, then
 * Plugged in (the ports seen now: Allow or Connect, or "Waiting for USB") or Try a
 * demo (a nameplate per driver: its panel drawn from the driver's own pages, the
 * model, its voices and tier). The last tile asks for a driver. Every model comes from
 * the core's registry, so a new driver needs no change here.
 */
@Composable
fun SynthPicker(
    drivers: List<DriverInfo>,
    access: List<MidiPortInfo>,
    away: List<SynthCard>,
    onDemo: (String) -> Unit,
    onAllow: (MidiPortInfo) -> Unit,
    onConnect: (String) -> Unit,
    onBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    var query by rememberSaveable { mutableStateOf("") }
    // Plugged in first when something is plugged in; else the demos.
    var tab by rememberSaveable { mutableIntStateOf(if (access.isNotEmpty() || away.isNotEmpty()) 0 else 1) }
    val uri = LocalUriHandler.current
    val q = query.trim()
    fun matches(vararg words: String) = q.isEmpty() || words.any { it.contains(q, ignoreCase = true) }
    val shown = drivers.filter { matches(it.model, it.vendor) }
    Column(modifier.fillMaxSize().windowInsetsPadding(safeInsets.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))) {
        // The header, the search and the tabs stay; the grid scrolls under them and lifts them.
        val grid = rememberLazyGridState()
        val liftAt = with(LocalDensity.current) { 8.dp.toPx() }
        Column(Modifier.barLift(c) { if (grid.firstVisibleItemIndex > 0) 1f else (grid.firstVisibleItemScrollOffset / liftAt).coerceIn(0f, 1f) }) {
            BackHeader(stringResource(Res.string.picker_title), onBack)
            val searchLabel = stringResource(Res.string.picker_search)
            // The search glyph leads, as on the program search.
            Row(
                Modifier
                    .padding(horizontal = 16.dp)
                    .fillMaxWidth()
                    .heightIn(min = 48.dp)
                    .border(OscylTokens.ruleWidth, c.ruleStrong, RoundedCornerShape(OscylTokens.radius))
                    .padding(horizontal = 12.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Image(painterResource(Res.drawable.ic_search), null, Modifier.size(20.dp), colorFilter = ColorFilter.tint(c.legend))
                Box(Modifier.weight(1f)) {
                    if (query.isEmpty()) BasicText(searchLabel, style = type.bodyLarge.copy(color = c.legend))
                    BasicTextField(
                        query,
                        { query = it },
                        Modifier.fillMaxWidth().semantics { contentDescription = searchLabel },
                        singleLine = true,
                        textStyle = type.bodyLarge.copy(color = c.ink, textDirection = TextDirection.Content),
                        cursorBrush = SolidColor(c.ink),
                    )
                }
            }
            Segments(
                listOf(stringResource(Res.string.picker_plugged), stringResource(Res.string.picker_demo)),
                selected = tab,
                saved = tab,
                onSelect = { tab = it },
                modifier = Modifier.padding(16.dp).fillMaxWidth(),
                index = false,
            )
        }
        LazyVerticalGrid(
            GridCells.Adaptive(156.dp),
            Modifier.fillMaxSize(),
            state = grid,
            contentPadding = androidx.compose.foundation.layout.PaddingValues(start = 16.dp, end = 16.dp, bottom = 16.dp + LocalIslandClearance.current),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            if (tab == 0) {
                val ports = access.filter { matches(it.name) }
                val gone = away.filter { matches(it.model) }
                if (ports.isEmpty() && gone.isEmpty()) {
                    item(span = { GridItemSpan(maxLineSpan) }) {
                        EmptyState(stringResource(Res.string.empty_plugged), stringResource(Res.string.picker_waiting), art = EmptyArt.Cable) {
                            OutlinedAction(stringResource(Res.string.picker_demo), { tab = 1 })
                        }
                    }
                }
                items(ports, key = { "port ${it.id}" }, span = { GridItemSpan(maxLineSpan) }) { port ->
                    PortRow(port.name, stringResource(Res.string.synth_state_access), stringResource(Res.string.synth_action_allow), stringResource(Res.string.synth_action_allow_named, port.name)) { onAllow(port) }
                }
                items(gone, key = { "card ${it.id}" }, span = { GridItemSpan(maxLineSpan) }) { card ->
                    PortRow(card.model, stringResource(Res.string.synth_state_disconnected), stringResource(Res.string.synth_action_connect), stringResource(Res.string.synth_action_connect_named, card.model)) { onConnect(card.port) }
                }
            } else {
                if (shown.isEmpty()) {
                    item(span = { GridItemSpan(maxLineSpan) }) {
                        EmptyState(stringResource(Res.string.picker_none), stringResource(Res.string.picker_none_body)) {
                            OutlinedAction(stringResource(Res.string.picker_ask), { uri.openUri(DriverRequestUrl) })
                        }
                    }
                }
                items(shown, key = { it.id }) { driver -> Box(Modifier.testTag("plate-${driver.id}")) { Nameplate(driver) { onDemo(driver.id) } } }
                item(key = "ask") {
                    // A place still empty in the rack, as the sheet's Add a synth: dashed, its + over its words.
                    DashedPlate(stringResource(Res.string.picker_ask), { uri.openUri(DriverRequestUrl) }, stacked = true)
                }
            }
        }
    }
}

/** A port or a gone synth: its name and state, and the one action. */
@Composable
private fun PortRow(name: String, state: String, action: String, described: String, onAction: () -> Unit) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    Tile(onClick = onAction, Modifier.semantics { contentDescription = described }) {
        BasicText(name, style = type.bodyLarge.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content))
        BasicText("$state · $action", style = type.bodyMedium.copy(color = c.legend))
    }
}

/** A raised tile on a 1 dp rule, a whole tap target. */
@Composable
private fun Tile(onClick: () -> Unit, modifier: Modifier = Modifier, content: @Composable () -> Unit) {
    val c = OscylTheme.colors
    val shape = RoundedCornerShape(OscylTokens.radiusPanel)
    val interaction = remember { MutableInteractionSource() }
    Column(
        modifier
            .fillMaxWidth()
            .heightIn(min = 56.dp)
            .border(OscylTokens.ruleWidth, c.rule, shape)
            .clickable(interaction, indication = null, role = Role.Button, onClick = onClick)
            .touchStates(interaction, shape, c.press)
            .padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) { content() }
}

/**
 * A driver's nameplate: an abstract panel drawn from its own pages and fields (a knob
 * per field column, the name display), never the maker's trade dress; then the model
 * and a legend line (voices · tier).
 */
@Composable
private fun Nameplate(driver: DriverInfo, onClick: () -> Unit) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    // The voices only: the driver's tier is the core's business, not the player's.
    val legend = driver.voices
    Tile(onClick = onClick) {
        PanelThumb(driver.panel, Modifier.fillMaxWidth().aspectRatio(2.2f))
        BasicText(driver.model, style = type.bodyLarge.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content))
        BasicText(driver.vendor, style = type.labelMedium.copy(color = c.legend))
        BasicText(legend, style = type.bodySmall.copy(color = c.legend, fontFeatureSettings = "tnum"))
    }
}

/**
 * A synth's panel in small, on a lifted plate: an abstract drawing from the driver's own
 * pages and fields (its name display, a knob per field column), never the maker's trade dress.
 */
@Composable
internal fun PanelThumb(panel: PanelDrawing, modifier: Modifier = Modifier, pad: Dp = 6.dp, minKnob: Dp = 3.5.dp) {
    val c = OscylTheme.colors
    Canvas(modifier.liftedPlate(c, RoundedCornerShape(OscylTokens.radius), fill = c.raised)) {
        val pad = pad.toPx()
        val w = size.width - 2 * pad
        val h = size.height - 2 * pad
        panel.display?.let { d ->
            drawRoundRect(c.ink.copy(alpha = 0.85f), Offset(pad + d.x * w, pad + d.y * h), Size(d.w * w, d.h * h), CornerRadius(2.dp.toPx()))
        }
        val scale = minOf(w, h)
        panel.knobs.forEach { k ->
            val center = Offset(pad + k.x * w, pad + k.y * h)
            val r = (k.r * scale).coerceAtLeast(minKnob.toPx())
            drawCircle(c.ground, r, center)
            drawCircle(c.ruleStrong, r, center, style = Stroke(1.dp.toPx()))
            drawLine(c.ink, center, center + Offset(0f, -r * 0.8f), 1.5.dp.toPx())
        }
    }
}
