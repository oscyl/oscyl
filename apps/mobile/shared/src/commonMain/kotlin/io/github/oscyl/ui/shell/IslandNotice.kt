package io.github.oscyl.ui.shell

import io.github.oscyl.shared.generated.resources.notice_open
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.motionScale
import kotlin.math.abs
import kotlinx.coroutines.launch
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.ic_close
import io.github.oscyl.shared.generated.resources.notice_compare_hint
import io.github.oscyl.shared.generated.resources.notice_dismiss
import io.github.oscyl.shared.generated.resources.notice_removed
import io.github.oscyl.shared.generated.resources.notice_undo
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.floating
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import org.jetbrains.compose.resources.stringResource

/**
 * A notice floating just above the island (a lifted part, like the island): what just
 * happened, Undo (or Open, Retry) when there is something to do, and Dismiss. Its host
 * times it out ([io.github.oscyl.ui.library.noticeTimeout]: never while a screen reader
 * runs and it offers an action), holds the time while a finger rests on it ([onHeld]) and
 * puts it away on a tap elsewhere; a swipe to either side puts it away too. A screen
 * reader announces it as it appears.
 */
@Composable
fun IslandNotice(
    text: String,
    onUndo: (() -> Unit)?,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
    actionLabel: String? = null,
    onOpen: (() -> Unit)? = null,
    onHeld: (Boolean) -> Unit = {},
) {
    val c = OscylTheme.colors
    val motion = LocalOscylMotion.current
    val scope = rememberCoroutineScope()
    val held by rememberUpdatedState(onHeld)
    val away = remember { Animatable(0f) }
    var width by remember { mutableIntStateOf(0) }
    val flick = with(LocalDensity.current) { 400.dp.toPx() }
    Layout(
        contents = listOf(
            {
                BasicText(
                    text,
                    Modifier.padding(vertical = 12.dp).padding(end = 8.dp).semantics { liveRegion = LiveRegionMode.Polite },
                    style = MaterialTheme.typography.bodyMedium.copy(color = c.ink, textDirection = TextDirection.Content),
                )
            },
            {
                if (onOpen != null || onUndo != null) {
                    Row(Modifier.padding(vertical = 6.dp).padding(end = 4.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        // Open first: the program it names, in the editor.
                        if (onOpen != null) OutlinedAction(stringResource(Res.string.notice_open), onOpen)
                        if (onUndo != null) OutlinedAction(actionLabel ?: stringResource(Res.string.notice_undo), onUndo)
                    }
                }
            },
            { IconAction(Res.drawable.ic_close, stringResource(Res.string.notice_dismiss), onDismiss) },
        ),
        modifier = modifier
            .widthIn(max = 560.dp)
            .fillMaxWidth()
            .onSizeChanged { width = it.width }
            .graphicsLayer {
                translationX = away.value
                alpha = 1f - (abs(away.value) / size.width.coerceAtLeast(1f)).coerceIn(0f, 1f)
            }
            // A finger resting on it holds its time; it lets go when the last finger lifts.
            .pointerInput(Unit) {
                awaitEachGesture {
                    awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
                    held(true)
                    do {
                        val event = awaitPointerEvent(PointerEventPass.Initial)
                    } while (event.changes.any { it.pressed })
                    held(false)
                }
            }
            // Swiped past 40% of its width (or flicked), it goes that way over `fast`.
            .draggable(
                rememberDraggableState { d -> scope.launch { away.snapTo(away.value + d) } },
                Orientation.Horizontal,
                onDragStopped = { v ->
                    val w = width.toFloat()
                    if (abs(away.value) > w * 0.4f || abs(v) > flick) {
                        val to = if ((if (abs(v) > flick) v else away.value) > 0f) w else -w
                        val ms = (OscylTokens.fastMs * motionScale(motion)).toInt()
                        if (ms > 0) away.animateTo(to, tween(ms, easing = OscylTokens.easing)) else away.snapTo(to)
                        onDismiss()
                    } else {
                        away.animateTo(0f, tween(OscylTokens.baseMs, easing = OscylTokens.easing))
                    }
                },
            )
            .floating(c)
            .padding(start = 16.dp, top = 4.dp, end = 8.dp, bottom = 4.dp),
    ) { (textM, actionsM, closeM), constraints ->
        // One line of words with its actions beside it when they fit (Material's snackbar);
        // otherwise the words, then the actions under them at the end. Dismiss keeps its corner.
        val close = closeM.first().measure(Constraints())
        val room = (constraints.maxWidth - close.width).coerceAtLeast(0)
        val actions = actionsM.firstOrNull()?.measure(Constraints(maxWidth = room))
        val actionsWidth = actions?.width ?: 0
        val oneLine = textM.first().maxIntrinsicWidth(Constraints.Infinity) <= room - actionsWidth
        val words = textM.first().measure(Constraints(maxWidth = if (oneLine) room - actionsWidth else room))
        val width = constraints.maxWidth
        if (oneLine) {
            val height = maxOf(words.height, actions?.height ?: 0, close.height)
            layout(width, height) {
                words.placeRelative(0, (height - words.height) / 2)
                actions?.placeRelative(room - actionsWidth, (height - actions.height) / 2)
                close.placeRelative(room, (height - close.height) / 2)
            }
        } else {
            val height = maxOf(words.height + (actions?.height ?: 0), close.height)
            layout(width, height) {
                words.placeRelative(0, 0)
                actions?.placeRelative(room - actionsWidth, words.height)
                close.placeRelative(room, 0)
            }
        }
    }
}

@OscylPreviews
@Composable
private fun islandNotice(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    IslandNotice(stringResource(Res.string.notice_removed, "Velvet Pad"), onUndo = {}, onDismiss = {})
}

@OscylA11yPreviews
@Composable
private fun islandNotice_a11y() = PreviewTheme {
    IslandNotice(stringResource(Res.string.notice_removed, "Velvet Pad"), onUndo = {}, onDismiss = {})
}

/** A long notice: Dismiss stays in its corner, the words wrap beside it. */
@OscylPreviews
@Composable
private fun islandNotice_long(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    IslandNotice(stringResource(Res.string.notice_compare_hint), onUndo = null, onDismiss = {})
}
