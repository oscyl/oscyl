package io.github.oscyl.ui.editor

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.ui.zIndex
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.key
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.editor_slot
import io.github.oscyl.shared.generated.resources.graph_description
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.ui.parts.safeInsets
import io.github.oscyl.ui.shell.LocalIslandClearance
import io.github.oscyl.ui.shell.FootSide
import io.github.oscyl.ui.shell.ReserveFoot
import io.github.oscyl.ui.shell.TopBarActions
import org.jetbrains.compose.resources.stringResource

/**
 * A phone on its side (compact height): one stage at a time (DESIGN.md "Stage view").
 * At the start edge, the stage strip with Hold to compare at its foot (the left
 * thumb); in the middle, the stage's title, the program and the chip above its picture,
 * large; beside it, the stage's controls in a scrolling column. The island stands on
 * end at the end edge (the shell's). Every edge part pads for a cutout on its side.
 */
@Composable
internal fun StageView(head: EditorHead, panel: PanelState, actions: EditorActions, stage: Int, onStage: (Int) -> Unit, modifier: Modifier) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val page = stage.coerceIn(panel.pages.indices)
    LaunchedEffect(panel, page) { actions.showing(setOf(page)) }
    androidx.compose.foundation.layout.BoxWithConstraints(modifier.testTag(StageViewTag)) {
    // Hold to compare stands at the strip's foot where six stages and it fit; on a
    // shorter screen it moves up beside the chip rather than squeeze.
    val footRoom = maxHeight >= 400.dp
    // A notice starts past the strip and Compare at its foot.
    ReserveFoot(FootSide.Start, 8.dp + RailSlot + 8.dp, shown = footRoom)
    Row(Modifier.fillMaxSize()) {
        Column(
            Modifier
                .zIndex(1f)
                .fillMaxHeight()
                .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Start + WindowInsetsSides.Vertical))
                .padding(start = 8.dp, top = 8.dp, bottom = 8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
        ) {
            // A fixed slot: held, the rail grows over the graph, which never reflows for it.
            Box(Modifier.width(RailSlot).zIndex(1f)) {
                Rail(panel, { page }, onJump = onStage, onScrub = onStage, modifier = Modifier, open = false, atStart = true)
            }
            // At the start edge the slide goes toward the end.
            if (footRoom) RailCompare(head.comparing, actions::compare, actions::morph, { panel.blend }, towardEnd = true, onShortTap = actions::compareTapped)
        }
        Column(
            Modifier
                .weight(1.25f)
                .fillMaxHeight()
                .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Top + WindowInsetsSides.Bottom))
                .padding(start = 12.dp, end = 8.dp, bottom = 8.dp),
        ) {
            // Chrome: its words grow with the system font to 130%.
            io.github.oscyl.ui.shell.ChromeScale {
                Row(Modifier.fillMaxWidth().heightIn(min = 48.dp), verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        BasicText(
                            panel.pages[page].title,
                            Modifier.semantics { heading() },
                            style = type.titleMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content),
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                        BasicText(
                            head.name + " · " + stringResource(Res.string.editor_slot, head.slot, head.model),
                            style = type.bodySmall.copy(color = c.legend, textDirection = TextDirection.Content),
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    if (!footRoom) CompareHold(head.comparing, compact = true, onCompare = actions::compare, onMorph = actions::morph, blend = { panel.blend })
                    TopBarActions(compact = true)
                }
            }
            val graph = panel.graph(page)
            Box(Modifier.weight(1f).fillMaxWidth()) {
                if (graph?.now != null) {
                    StageGraphView(
                        graph,
                        panel.handles(page),
                        stringResource(Res.string.graph_description, panel.pages[page].title, panel.stage(page)?.summary.orEmpty()),
                        onDrag = { h, x, y -> actions.drag(page, h, x, y) },
                        onRelease = actions::release,
                        height = null,
                        large = true,
                        following = { panel.following },
                    )
                } else {
                    StageHeading(panel, page, onRoll = { actions.roll(page) }.takeIf { !head.readOnly && head.loaded }, onRollTapped = { actions.rollTapped(page) })
                }
            }
        }
        Column(
            Modifier
                .weight(1f)
                .fillMaxHeight()
                // The island stands on end past this column: its clearance is the shell's.
                .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Top))
                .verticalScroll(rememberScrollState())
                .padding(start = 4.dp, top = 8.dp, end = 8.dp, bottom = 16.dp + LocalIslandClearance.current),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            key(page) { panel.pages[page].sections.indices.forEach { SectionPanel(panel, page, it, actions) } }
        }
    }
    }
}

/** The stage strip's resting width (the 48 dp rail). */
private val RailSlot = 48.dp

/** The stage view's test tag. */
const val StageViewTag = "stage view"
