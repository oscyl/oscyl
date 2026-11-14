package io.github.oscyl.ui.shell

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.Velocity
import androidx.compose.ui.unit.dp
import kotlin.math.abs
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.LocalOscylMotion

/**
 * Whether the page under the island is being scrolled a long way by hand: while it is, the
 * floating parts over it (the island, the Save key, the foot's keys) dim a little so what
 * passes under them can be made out, and come back when it stops (as Claude's app does).
 * "A long way" is the scroll's own travel since it started ([travel], about a long page's
 * worth), so any long page dims it (a long list, a licence, the editor flung far) and a
 * short page, which cannot travel that far, never does. Its own signals only: the travel
 * starts it, the fling's end or the next touch ends it.
 */
@Stable
class ScrollPeek(private val travel: Float) {
    private var moved = 0f
    // Only a finger's scroll and the fling it lets go count: a scroll the app makes (a new
    // set brought into view) has no finger and no fling, and would never end the peek.
    private var finger = false
    private var flinging = false

    var scrolling by mutableStateOf(false)
        private set

    private val connection = object : NestedScrollConnection {
        override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset {
            if (consumed.y != 0f && (finger || flinging)) {
                moved += abs(consumed.y)
                if (moved >= travel) scrolling = true
            }
            return Offset.Zero
        }

        override suspend fun onPreFling(available: Velocity): Velocity {
            flinging = true
            return Velocity.Zero
        }

        override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
            flinging = false
            stop()
            return Velocity.Zero
        }
    }

    private fun stop() {
        moved = 0f
        scrolling = false
    }

    /** On the page's root: listens to its scrolls, and a touch (which stops a fling) ends the peek. */
    fun Modifier.peekSource(): Modifier = nestedScroll(connection).pointerInput(this@ScrollPeek) {
        awaitEachGesture {
            awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
            // A touch stops a fling (which then may not report its end).
            flinging = false
            stop()
            finger = true
            do {
                val event = awaitPointerEvent(PointerEventPass.Initial)
            } while (event.changes.any { it.pressed })
            finger = false
        }
    }
}

/** How far a scroll travels before the floating parts dim: about a long page's worth. */
val PeekTravel = 900.dp

/** How opaque a floating part over a long list stands while it scrolls: its words stay the clearer ones. */
private const val PeekAlpha = 0.65f

val LocalScrollPeek = staticCompositionLocalOf<ScrollPeek?> { null }

/** The floating parts' opacity now, read it in a graphicsLayer (draw only, nothing moves). */
@Composable
fun peekAlpha(peek: ScrollPeek? = LocalScrollPeek.current): State<Float> {
    val motion = LocalOscylMotion.current
    return animateFloatAsState(if (peek?.scrolling == true) PeekAlpha else 1f, tween(if (motion) OscylTokens.slowMs else 0, easing = OscylTokens.easing), label = "peek")
}
