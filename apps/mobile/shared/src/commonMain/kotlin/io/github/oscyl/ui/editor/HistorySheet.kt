package io.github.oscyl.ui.editor

import io.github.oscyl.shared.generated.resources.history_steps
import io.github.oscyl.shared.generated.resources.history_snapshots
import io.github.oscyl.ui.parts.Legend
import androidx.compose.runtime.rememberUpdatedState
import io.github.oscyl.ui.parts.liftedPlate
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.shared.generated.resources.snapshots_hint
import io.github.oscyl.shared.generated.resources.snapshot_stored
import io.github.oscyl.shared.generated.resources.snapshot_store
import io.github.oscyl.shared.generated.resources.snapshot_recall
import io.github.oscyl.shared.generated.resources.snapshot_empty
import io.github.oscyl.shared.generated.resources.snapshot
import io.github.oscyl.platform.LocalOscylHaptics
import androidx.compose.ui.semantics.testTag
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.runtime.getValue
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.border
import androidx.compose.animation.core.tween
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import io.github.oscyl.core.HistoryStep
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.history_empty_body
import io.github.oscyl.shared.generated.resources.history_empty_title
import io.github.oscyl.shared.generated.resources.history_now
import io.github.oscyl.shared.generated.resources.history_start
import io.github.oscyl.shared.generated.resources.history_title
import io.github.oscyl.shared.generated.resources.history_undone
import io.github.oscyl.shared.generated.resources.ic_close
import io.github.oscyl.shared.generated.resources.sheet_close
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.EmptyArt
import io.github.oscyl.ui.parts.EmptyState
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.startAlign
import io.github.oscyl.ui.parts.touchStates
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.parts.DockedSheet
import io.github.oscyl.ui.parts.SheetTitle
import org.jetbrains.compose.resources.stringResource

/**
 * The edit history, in the bottom sheet's frame (DESIGN.md "Edit history"): every step
 * of the document shown, the newest first, down to "As saved" (no step done). The step
 * the document stands at carries the index mark (an ink line at its start, as a dial's)
 * and is said as "Now"; the steps undone wait above it, dimmer, to be done again. A tap
 * on a row goes there: so many undos or redos as one move ([onJump], with how many
 * steps are done after it). [steps] come oldest first, as the core lists them.
 */
@Composable
fun HistoryList(
    steps: List<HistoryStep>,
    onJump: (done: Int) -> Unit,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
    /** A, B, C of the open program (stored or not); null with no program open. */
    snapshots: List<Boolean>? = null,
    onStore: (Int) -> Unit = {},
    onRecall: (Int) -> Unit = {},
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val done = steps.count { it.done }
    Column(modifier.fillMaxWidth()) {
        SheetTitle(stringResource(Res.string.history_title))
        // Two runs, each with its legend: the program's snapshots, then the document's steps.
        if (snapshots != null) {
            Legend(stringResource(Res.string.history_snapshots), Modifier.padding(start = 16.dp, top = 4.dp, end = 16.dp))
            SnapshotPlates(snapshots, onStore, onRecall)
            if (steps.isNotEmpty()) Legend(stringResource(Res.string.history_steps), Modifier.padding(start = 16.dp, top = 8.dp, end = 16.dp, bottom = 4.dp))
        }
        Column(
            Modifier.weight(1f, fill = false).fillMaxWidth().verticalScroll(rememberScrollState()).padding(start = 8.dp, end = 8.dp, bottom = 16.dp + io.github.oscyl.ui.parts.LocalSheetFoot.current),
        ) {
            if (steps.isEmpty()) {
                EmptyState(
                    stringResource(Res.string.history_empty_title),
                    stringResource(Res.string.history_empty_body),
                    Modifier.align(Alignment.CenterHorizontally),
                    art = EmptyArt.Panel,
                )
                return@Column
            }
            steps.indices.reversed().forEach { i ->
                val step = steps[i]
                StepRow(step.slot, step.title, now = i == done - 1, undone = !step.done) { onJump(i + 1) }
            }
            StepRow("", stringResource(Res.string.history_start), now = done == 0, undone = false) { onJump(0) }
        }
    }
}

/**
 * Snapshots A, B, C of the program being shaped (DESIGN.md "Edit history"): 40 x 32 dp
 * plates in 48 dp targets. Empty, a `rule-strong` outline (a key, still, on a sheet's raised ground) with a `legend` letter; stored, a
 * lifted plate. A hold stores (a confirm, and the plate lifts on `base`), a tap recalls;
 * an empty plate's tap does nothing. Screen readers: "Snapshot A, empty", Store, Recall.
 */
@Composable
internal fun SnapshotPlates(stored: List<Boolean>, onStore: (Int) -> Unit, onRecall: (Int) -> Unit) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val haptics = LocalOscylHaptics.current
    val motion = LocalOscylMotion.current
    val shape = RoundedCornerShape(OscylTokens.radius)
    // Read in the gesture as they are now, not as they were when it began listening.
    val storedNow by rememberUpdatedState(stored)
    val store by rememberUpdatedState(onStore)
    val recall by rememberUpdatedState(onRecall)
    val storeLabel = stringResource(Res.string.snapshot_store)
    val recallLabel = stringResource(Res.string.snapshot_recall)
    Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp), verticalAlignment = Alignment.CenterVertically) {
        stored.forEachIndexed { i, full ->
            val letter = "ABC"[i].toString()
            val name = stringResource(Res.string.snapshot, letter)
            val state = stringResource(if (full) Res.string.snapshot_stored else Res.string.snapshot_empty)
            val lift by animateFloatAsState(if (full) 1f else 0f, tween(if (motion) OscylTokens.baseMs else 0, easing = OscylTokens.easing))
            Box(
                Modifier
                    .size(48.dp)
                    .clearAndSetSemantics {
                        contentDescription = name
                        stateDescription = state
                        testTag = "snapshot-$letter"
                        customActions = listOfNotNull(
                            CustomAccessibilityAction(storeLabel) { onStore(i); true },
                            if (full) CustomAccessibilityAction(recallLabel) { onRecall(i); true } else null,
                        )
                    }
                    .pointerInput(i) {
                        detectTapGestures(
                            onTap = { if (storedNow.getOrElse(i) { false }) recall(i) },
                            onLongPress = {
                                haptics.confirm()
                                store(i)
                            },
                        )
                    },
                contentAlignment = Alignment.Center,
            ) {
                // Both faces drawn, the stored one lifting in over the outline.
                Box(Modifier.size(40.dp, 32.dp).border(OscylTokens.ruleWidth, c.ruleStrong, shape).graphicsLayer { alpha = 1f - lift })
                Box(Modifier.size(40.dp, 32.dp).graphicsLayer { alpha = lift; translationY = (1f - lift) * 2.dp.toPx() }.liftedPlate(c, shape))
                BasicText(
                    letter,
                    style = type.titleSmall.copy(color = if (full) c.onPlate else c.legend, fontWeight = FontWeight.SemiBold),
                )
            }
        }
        BasicText(
            stringResource(Res.string.snapshots_hint),
            Modifier.weight(1f).padding(start = 8.dp),
            style = type.bodySmall.copy(color = c.legend, textAlign = startAlign()),
        )
    }
}

/** One step: its program's slot, what it did; the index mark and full ink where the document stands, `legend` when undone. */
@Composable
private fun StepRow(slot: String, title: String, now: Boolean, undone: Boolean, onClick: () -> Unit) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    val state = when {
        now -> stringResource(Res.string.history_now)
        undone -> stringResource(Res.string.history_undone)
        else -> null
    }
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .clickable(interaction, indication = null, role = Role.Button, onClick = onClick)
            .semantics(mergeDescendants = true) {
                selected = now
                if (state != null) stateDescription = state
            }
            .touchStates(interaction, shape, c.press)
            .drawBehind {
                // The index: where the document stands, as on a dial.
                if (now) {
                    val x = if (layoutDirection == LayoutDirection.Rtl) size.width - 1.dp.toPx() else 1.dp.toPx()
                    drawLine(c.ink, Offset(x, 12.dp.toPx()), Offset(x, size.height - 12.dp.toPx()), 2.dp.toPx(), StrokeCap.Round)
                }
            }
            .padding(horizontal = 8.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // The slots' column keeps its width, so every step's words start on one line.
        Box(Modifier.widthIn(min = 40.dp)) {
            BasicText(slot, style = type.bodyMedium.copy(color = c.legend, fontFeatureSettings = "tnum"), maxLines = 1)
        }
        BasicText(
            title,
            Modifier.weight(1f),
            style = type.bodyLarge.copy(
                color = if (undone) c.legend else c.ink,
                fontWeight = FontWeight.Medium,
                fontFeatureSettings = "tnum",
                textDirection = TextDirection.Content,
                textAlign = startAlign(),
            ),
        )
    }
}

private val SampleSteps = listOf(
    HistoryStep("Filter · Cutoff: 431 to 618", 3u, "004", true),
    HistoryStep("Name: Night Driv to Night Drive", 11u, "012", true),
    HistoryStep("Amp EG · Attack, Amp EG · Decay", 3u, "004", true),
    HistoryStep("Dust Lead removed", 5u, "006", false),
    HistoryStep("Filter · Drive: 0% to 50%", 3u, "004", false),
)

@Composable
private fun HistoryPreview(mode: OscylThemeMode = OscylThemeMode.Dark, steps: List<HistoryStep> = SampleSteps) = OscylTheme(mode) {
    Box { DockedSheet(open = true, onClose = {}, title = stringResource(Res.string.history_title)) { HistoryList(steps, onJump = {}, onClose = {}, snapshots = listOf(true, false, true)) } }
}

@OscylPreviews
@Composable
private fun history(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = HistoryPreview(mode)

@OscylPreviews
@Composable
private fun history_empty(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = HistoryPreview(mode, emptyList())

@OscylA11yPreviews
@Composable
private fun history_a11y() = HistoryPreview()
