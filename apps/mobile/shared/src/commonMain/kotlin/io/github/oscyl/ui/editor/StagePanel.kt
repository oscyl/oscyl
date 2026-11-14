package io.github.oscyl.ui.editor

import io.github.oscyl.shared.generated.resources.stage_roll
import io.github.oscyl.shared.generated.resources.ic_roll
import androidx.compose.foundation.layout.Spacer
import io.github.oscyl.platform.LocalOscylHaptics
import androidx.compose.ui.platform.testTag
import androidx.compose.foundation.layout.Box
import io.github.oscyl.shared.generated.resources.graph_expand
import io.github.oscyl.shared.generated.resources.ic_expand
import io.github.oscyl.ui.parts.IconAction
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.foundation.layout.Row
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.graph_description
import io.github.oscyl.theme.OscylTheme
import org.jetbrains.compose.resources.stringResource

/**
 * A stage's heading (DESIGN.md Title: 600, ink) with its one-line summary in
 * `legend` beside it, on one line.
 */
@Composable
fun StageHeading(panel: PanelState, page: Int, modifier: Modifier = Modifier, onRoll: (() -> Unit)? = null, onRollTapped: () -> Unit = {}) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    // One line, always: the summary changes as knobs turn, and a summary that wrapped
    // would push the whole page down a line under the finger. It gives way with an ellipsis.
    Row(
        modifier.fillMaxWidth().padding(top = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        BasicText(
            panel.pages[page].title,
            Modifier.semantics { heading() },
            style = type.titleMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
        panel.stage(page)?.summary?.takeIf { it.isNotEmpty() }?.let {
            BasicText(
                it,
                Modifier.weight(1f),
                style = type.bodySmall.copy(color = c.legend, fontFeatureSettings = "tnum", textDirection = TextDirection.Content),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        // The summary's weight gives way to the dice: the heading keeps one line.
        if (onRoll != null) {
            val haptics = LocalOscylHaptics.current
            if (panel.stage(page)?.summary.isNullOrEmpty()) Spacer(Modifier.weight(1f))
            // A hold rolls (a stray tap in the scrolling path would rewrite a stage); a tap says so.
            val rollWord = stringResource(Res.string.stage_roll, panel.pages[page].title)
            IconAction(
                Res.drawable.ic_roll, rollWord, onRollTapped, Modifier.testTag("roll-$page"),
                onLongClick = onRoll, onLongClickLabel = rollWord,
            )
        }
    }
}

/**
 * A stage's picture with its handles, when it has one; it reads its own state. With
 * [onExpand], a tap on the picture or its ⤢ corner opens it full screen; [frame]
 * marks the frame for the container transform.
 */
@Composable
fun StageGraphSlot(
    panel: PanelState,
    page: Int,
    actions: EditorActions,
    modifier: Modifier = Modifier,
    onExpand: (() -> Unit)? = null,
    frame: Modifier = Modifier,
) {
    val graph = panel.graph(page) ?: return
    if (graph.now == null) return
    val title = panel.pages[page].title
    val description = stringResource(Res.string.graph_description, title, panel.stage(page)?.summary.orEmpty())
    Box(modifier) {
        StageGraphView(
            graph,
            panel.handles(page),
            description,
            onDrag = { h, x, y -> actions.drag(page, h, x, y) },
            onRelease = actions::release,
            modifier = frame,
            onTap = onExpand,
            following = { panel.following },
        )
        if (onExpand != null) {
            IconAction(
                Res.drawable.ic_expand,
                stringResource(Res.string.graph_expand, title),
                onExpand,
                Modifier.align(Alignment.TopEnd).testTag("graph-expand"),
            )
        }
    }
}
