package io.github.oscyl.ui.review

import io.github.oscyl.ui.parts.ProgressScale
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.draw.drawBehind
import io.github.oscyl.ui.parts.safeInsets
import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfoV2
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.backhandler.PredictiveBackHandler
import kotlinx.coroutines.CancellationException
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.window.core.layout.WindowSizeClass
import io.github.oscyl.core.ReviewChange
import io.github.oscyl.core.ReviewProgram
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.action_save
import io.github.oscyl.shared.generated.resources.action_save_edited
import io.github.oscyl.shared.generated.resources.action_sync
import io.github.oscyl.shared.generated.resources.action_sync_edited
import io.github.oscyl.shared.generated.resources.ic_close
import io.github.oscyl.shared.generated.resources.review_backing_up
import io.github.oscyl.shared.generated.resources.review_backup_failed
import io.github.oscyl.shared.generated.resources.review_backup_nowhere
import io.github.oscyl.shared.generated.resources.review_change
import io.github.oscyl.shared.generated.resources.review_not_yet
import io.github.oscyl.shared.generated.resources.review_revert
import io.github.oscyl.shared.generated.resources.review_save_failed
import io.github.oscyl.shared.generated.resources.review_sync_failed
import io.github.oscyl.shared.generated.resources.review_sync_lost
import io.github.oscyl.shared.generated.resources.review_sync_note
import io.github.oscyl.shared.generated.resources.review_title
import io.github.oscyl.shared.generated.resources.review_writing
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.CountBox
import io.github.oscyl.ui.parts.DockedSheet
import io.github.oscyl.ui.parts.SheetTitle
import io.github.oscyl.ui.parts.startAlign
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.Legend
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.floating
import io.github.oscyl.ui.parts.liftedPlate
import io.github.oscyl.ui.parts.motionScale
import io.github.oscyl.ui.parts.plateSpring
import io.github.oscyl.ui.parts.touchStates
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.OscylSizePreviews
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.shell.Destination
import io.github.oscyl.ui.shell.LocalIslandClearance
import io.github.oscyl.ui.shell.Shell
import org.jetbrains.compose.resources.pluralStringResource
import org.jetbrains.compose.resources.stringResource

/**
 * The review sheet (DESIGN.md "Review bar"), in the one docked sheet's frame: every
 * edited program and its changes, each program with its own revert, and the action at
 * its foot (Save for a library, Sync for a synth's slots, progress in words while it
 * runs, a failure in words outlined in ink).
 */
@Composable
fun ReviewSheet(ui: ReviewUi, actions: ReviewActions, modifier: Modifier = Modifier) {
    val title = pluralStringResource(Res.plurals.review_title, ui.programs.size, ui.programs.size)
    DockedSheet(ui.open, actions::close, title, modifier) { Sheet(ui, actions, title) }
}

@Composable
private fun Sheet(ui: ReviewUi, actions: ReviewActions, title: String) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    Column(Modifier.fillMaxWidth()) {
        SheetTitle(title)
        if (ui.sync) {
            BasicText(
                stringResource(Res.string.review_sync_note),
                Modifier.fillMaxWidth().padding(horizontal = 16.dp),
                style = type.bodySmall.copy(color = c.legend, textDirection = TextDirection.Content, textAlign = startAlign()),
            )
        }
        LazyColumn(Modifier.weight(1f, fill = false), contentPadding = androidx.compose.foundation.layout.PaddingValues(vertical = 8.dp)) {
            items(ui.programs, key = { it.index.toLong() }) { ProgramChanges(it, enabled = !ui.busy, onRevert = { actions.revert(it.index) }) }
        }
        Footer(ui, actions)
    }
}

/** One edited program: its slot and name, its own revert, then each change in words. */
@Composable
private fun ProgramChanges(p: ReviewProgram, enabled: Boolean, onRevert: () -> Unit) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp)) {
        FlowRow(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            itemVerticalAlignment = Alignment.CenterVertically,
        ) {
            Row(Modifier.semantics(mergeDescendants = true) {}, verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Canvas(Modifier.size(6.dp)) { drawCircle(c.act) }
                BasicText(p.slot, style = type.bodyMedium.copy(color = c.legend, fontFeatureSettings = "tnum"))
                BasicText(p.name, style = type.bodyLarge.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content))
            }
            Spacer(Modifier.weight(1f))
            // Kept in place while a save or sync runs: nothing in the row moves when it starts or ends.
            OutlinedAction(stringResource(Res.string.review_revert), onRevert, enabled = enabled)
        }
        p.changes.forEach { ChangeRow(it) }
    }
}

/** "Filter · Cutoff" over "431 to 618", ruled. */
@Composable
private fun ChangeRow(change: ReviewChange) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    Column(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 44.dp)
            .padding(start = 14.dp, top = 4.dp, bottom = 4.dp)
            .semantics(mergeDescendants = true) {},
        verticalArrangement = Arrangement.Center,
    ) {
        BasicText(change.title, style = type.bodyMedium.copy(color = c.ink, textDirection = TextDirection.Content))
        BasicText(
            stringResource(Res.string.review_change, change.before, change.after),
            style = type.bodySmall.copy(color = c.legend, fontFeatureSettings = "tnum", textDirection = TextDirection.Content),
        )
    }
}

/** The state in words, then the action: the plate with its verb and count. */
@Composable
private fun Footer(ui: ReviewUi, actions: ReviewActions) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    Column(Modifier.fillMaxWidth().padding(12.dp).padding(bottom = io.github.oscyl.ui.parts.LocalSheetFoot.current), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        // A reason ends its own sentence: its full stop gives way to the text's.
        val words = when (val s = ui.state) {
            ReviewState.Idle -> null
            ReviewState.BackingUp -> stringResource(Res.string.review_backing_up)
            is ReviewState.Writing -> stringResource(Res.string.review_writing, s.done, s.total)
            is ReviewState.BackupFailed -> if (s.reason.isEmpty()) {
                stringResource(Res.string.review_backup_nowhere)
            } else {
                stringResource(Res.string.review_backup_failed, s.reason.trimEnd('.'))
            }
            is ReviewState.SyncFailed -> if (s.reason.isEmpty()) stringResource(Res.string.review_sync_lost) else stringResource(Res.string.review_sync_failed, s.reason.trimEnd('.'))
            is ReviewState.SaveFailed -> stringResource(Res.string.review_save_failed, s.reason.trimEnd('.'))
            is ReviewState.NotYet -> stringResource(Res.string.review_not_yet, s.reason.trimEnd('.'))
        }
        if (words != null) {
            val failed = ui.state is ReviewState.Failed
            BasicText(
                words,
                Modifier
                    .fillMaxWidth()
                    .semantics { liveRegion = LiveRegionMode.Polite }
                    .then(if (failed) Modifier.border(OscylTokens.ruleWidth, c.ink, RoundedCornerShape(OscylTokens.radius)).padding(12.dp) else Modifier.padding(horizontal = 4.dp)),
                style = type.bodyMedium.copy(color = c.ink),
            )
        }
        // The write on the same tuning scale as a read.
        (ui.state as? ReviewState.Writing)?.let { ProgressScale(if (it.total > 0) it.done.toFloat() / it.total else 0f, Modifier.padding(horizontal = 4.dp)) }
        if (!ui.busy && ui.programs.isNotEmpty()) {
            val n = ui.programs.size
            val label = stringResource(if (ui.sync) Res.string.action_sync else Res.string.action_save)
            val description = pluralStringResource(if (ui.sync) Res.plurals.action_sync_edited else Res.plurals.action_save_edited, n, n)
            val shape = RoundedCornerShape(OscylTokens.radiusPanel)
            val interaction = remember { MutableInteractionSource() }
            Row(
                Modifier
                    .fillMaxWidth()
                    .heightIn(min = 48.dp)
                    .liftedPlate(c, shape)
                    .clickable(interaction, indication = null, role = Role.Button, onClick = actions::commit)
                    .clearAndSetSemantics {
                        contentDescription = description
                        role = Role.Button
                        onClick { actions.commit(); true }
                    }
                    .touchStates(interaction, shape, c.ink.copy(alpha = 0.22f)),
                horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                BasicText(label, style = type.bodyMedium.copy(color = c.onPlate, fontWeight = FontWeight.SemiBold))
                CountBox(n)
            }
        }
    }
}

// Sample state for the previews and the device checks: plain values, no model, no core calls.

private fun change(field: Int?, stage: String?, title: String, before: String, after: String) =
    ReviewChange(field?.toUInt(), stage, null, title, before, after)

val SampleReview = ReviewUi(
    open = true,
    programs = listOf(
        ReviewProgram(3u, "004", "Glass Choir", listOf(change(10, "Filter", "Filter · Cutoff", "431", "618"), change(13, "Filter", "Filter · Drive", "0%", "50%"))),
        ReviewProgram(11u, "012", "Night Drive", listOf(change(null, null, "Name", "Night Driv", "Night Drive"), change(4, "Oscillators", "VCO 1 · Shape", "120", "300"))),
    ),
)

@Composable
private fun ReviewPreview(mode: OscylThemeMode = OscylThemeMode.Dark, ui: ReviewUi = SampleReview) = OscylTheme(mode) {
    // One frame for the shell and the sheet, as in the app: the sheet keeps clear of a side island.
    androidx.compose.runtime.CompositionLocalProvider(io.github.oscyl.ui.shell.LocalShellFrame provides androidx.compose.runtime.remember { io.github.oscyl.ui.shell.ShellFrame() }) {
        Shell(Destination.Editor, onSelect = {}, saveCount = ui.programs.size, onSave = {}, sync = ui.sync) { modifier ->
            Box(modifier.fillMaxSize())
        }
        ReviewSheet(ui, NoReviewActions)
    }
}

@OscylPreviews
@Composable
private fun review_save(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = ReviewPreview(mode)

@OscylPreviews
@Composable
private fun review_syncing(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    ReviewPreview(mode, SampleReview.copy(sync = true, state = ReviewState.Writing(1, 2)))

@OscylPreviews
@Composable
private fun review_failed(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    ReviewPreview(mode, SampleReview.copy(sync = true, state = ReviewState.SyncFailed("The synth did not confirm slot 12.")))

@OscylA11yPreviews
@Composable
private fun review_a11y() = ReviewPreview(ui = SampleReview.copy(sync = true))

@OscylSizePreviews
@Composable
private fun review_sizes() = ReviewPreview()
