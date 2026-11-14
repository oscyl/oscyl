package io.github.oscyl.ui.editor

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.ClipOp
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.intl.Locale
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.toUpperCase
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.Legend

/**
 * One section of a stage (DESIGN.md "section panel"): its legend, then a framed panel
 * (1 dp rule, 10 dp corners, a faint enamel sheen at the top) where the dials lead and
 * the selectors stand in a column beside them, wrapping under when the panel is too
 * narrow; dial rows drop columns as text grows. A section with nothing that applies
 * (layer B of a one-layer program) is left out. Each control reads its own field, so
 * an edit redraws that control only.
 */
@Composable
fun SectionPanel(panel: PanelState, page: Int, section: Int, actions: EditorActions, modifier: Modifier = Modifier) {
    val info = panel.pages[page].sections[section]
    // What is shown, and as what: this changes only when a control comes, goes or
    // changes kind, never while a value moves.
    val shown by remember(panel, page, section) {
        derivedStateOf {
            if (panel.stage(page)?.sectionsShown?.getOrNull(section) == false) {
                emptyList()
            } else {
                info.fields.mapNotNull { panel.field(it) }.filter { it.relevant }.map { it.field to it.isDial }
            }
        }
    }
    if (shown.isEmpty()) return
    val c = OscylTheme.colors
    val title = info.title
    // Where the legend broke the top rule (px), written in layout and read in draw.
    val engraved = remember { FloatArray(2) } // [frame top, legend width]
    Layout(
        content = {
            if (title != null) {
                BasicText(
                    title.toUpperCase(Locale.current),
                    Modifier.semantics { heading() },
                    style = MaterialTheme.typography.labelMedium.copy(color = c.legend),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            FlowRow(
                Modifier.fillMaxWidth().padding(PaddingValues(start = 12.dp, top = 16.dp, end = 12.dp, bottom = 12.dp)),
                // Each row of knobs spread evenly across the frame (not packed at its start).
                horizontalArrangement = Arrangement.SpaceEvenly,
                verticalArrangement = Arrangement.spacedBy(12.dp),
                itemVerticalAlignment = Alignment.Top,
            ) {
                shown.filter { it.second }.forEach { FieldSlot(panel, it.first, actions) }
                val selectors = shown.filterNot { it.second }
                if (selectors.isNotEmpty()) {
                    SelectorColumn { selectors.forEach { FieldSlot(panel, it.first, actions) } }
                }
            }
        },
        modifier = modifier.fillMaxWidth().drawBehind {
            // The frame, its top rule broken where the legend is engraved into it.
            val top = engraved[0]
            val legendWidth = engraved[1]
            val rule = OscylTokens.ruleWidth.toPx()
            val radius = CornerRadius(OscylTokens.radiusPanel.toPx())
            val frame = Size(size.width, size.height - top)
            drawRoundRect(
                Brush.verticalGradient(0f to c.ink.copy(alpha = 0.03f), 0.45f to Color.Transparent, startY = top, endY = size.height),
                Offset(0f, top), frame, radius,
            )
            val gap = if (legendWidth > 0f) 12.dp.toPx() + legendWidth + 12.dp.toPx() else 0f
            val ltr = layoutDirection == LayoutDirection.Ltr
            val from = if (ltr) 12.dp.toPx() else size.width - gap
            val to = if (ltr) gap else size.width - 12.dp.toPx()
            clipRect(from, top - rule * 2, to, top + rule * 2, clipOp = if (gap > 0f) ClipOp.Difference else ClipOp.Intersect) {
                drawRoundRect(c.rule, Offset(rule / 2, top + rule / 2), Size(frame.width - rule, frame.height - rule), radius, style = Stroke(rule))
            }
            if (gap == 0f) drawRoundRect(c.rule, Offset(rule / 2, top + rule / 2), Size(frame.width - rule, frame.height - rule), radius, style = Stroke(rule))
        },
    ) { measurables, constraints ->
        val width = constraints.maxWidth
        val inset = 18.dp.roundToPx()
        val legend = if (title != null) measurables[0].measure(Constraints(maxWidth = (width - 2 * inset).coerceAtLeast(0))) else null
        val flow = measurables.last().measure(constraints.copy(minHeight = 0))
        val top = (legend?.height ?: 0) / 2
        engraved[0] = top.toFloat()
        engraved[1] = (legend?.width ?: 0).toFloat()
        layout(width, top + flow.height) {
            flow.placeRelative(0, top)
            legend?.placeRelative(inset, 0)
        }
    }
}

/** One field's control, reading its own state, so its edits redraw it alone. */
@Composable
private fun FieldSlot(panel: PanelState, field: UInt, actions: EditorActions) {
    val state = panel.field(field) ?: return
    FieldControl(state, panel.comparing, actions, enabled = panel.writable)
}

/** Sections with their names engraved into the frame's top rule; a long one ends in an ellipsis inside the frame. */
@Composable
private fun SectionSamples(longNames: Boolean) {
    val panel = remember { samplePanel(longNames) }
    Column(verticalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(16.dp)) {
        SectionPanel(panel, 1, 0, NoEditorActions)
        SectionPanel(panel, 4, 0, NoEditorActions)
    }
}

@io.github.oscyl.ui.preview.OscylPreviews
@Composable
private fun section_engraved(@androidx.compose.ui.tooling.preview.PreviewParameter(io.github.oscyl.ui.preview.ThemeModes::class) mode: io.github.oscyl.theme.OscylThemeMode) =
    io.github.oscyl.ui.preview.PreviewTheme(mode) { SectionSamples(longNames = false) }

@io.github.oscyl.ui.preview.OscylA11yPreviews
@Composable
private fun section_engraved_a11y() = io.github.oscyl.ui.preview.PreviewTheme { SectionSamples(longNames = true) }
