package io.github.oscyl.shell

import androidx.compose.runtime.setValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.getValue
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.BasicText
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.swipeUp
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.ui.shell.ScrollPeek
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The island's peek: on while the page is scrolled by hand, off once it stops or is touched. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class ScrollPeekTest {
    @get:Rule val compose = createComposeRule()

    @Test
    fun aLongScrollPeeksUntilItStopsAndATouchEndsIt() {
        var travel by mutableStateOf(Float.MAX_VALUE)
        lateinit var peek: ScrollPeek
        compose.setContent {
            peek = remember(travel) { ScrollPeek(travel) }
            LazyColumn(with(peek) { Modifier.fillMaxSize().peekSource() }.testTag("list")) {
                items(400) { BasicText("Row $it", Modifier.height(48.dp)) }
            }
        }
        // A scroll shorter than the travel (a short page cannot go further) dims nothing.
        compose.onNodeWithTag("list").performTouchInput { swipeUp(durationMillis = 200) }
        assertFalse(peek.scrolling)
        compose.waitForIdle()
        travel = 100f
        compose.waitForIdle()
        compose.mainClock.autoAdvance = false
        compose.onNodeWithTag("list").performTouchInput { swipeUp(durationMillis = 200) }
        compose.mainClock.advanceTimeByFrame()
        assertTrue(peek.scrolling)
        // The fling runs on; a touch stops it and the peek with it.
        compose.onNodeWithTag("list").performTouchInput { down(center) }
        compose.mainClock.advanceTimeByFrame()
        assertFalse(peek.scrolling)
        compose.onNodeWithTag("list").performTouchInput { up() }
        compose.mainClock.autoAdvance = true
        compose.waitForIdle()
        assertFalse(peek.scrolling)
        // Left to fling out, it ends with the fling.
        compose.onNodeWithTag("list").performTouchInput { swipeUp(durationMillis = 200) }
        compose.waitForIdle()
        assertFalse(peek.scrolling)
    }
}
