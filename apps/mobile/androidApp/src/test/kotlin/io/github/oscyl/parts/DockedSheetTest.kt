package io.github.oscyl.parts

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.swipeDown
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.parts.DockedSheet
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The one docked sheet: every way to put it away, and what it keeps. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class DockedSheetTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    private var open by mutableStateOf(true)

    private val pane = SemanticsMatcher.expectValue(SemanticsProperties.PaneTitle, "Sheet")

    private fun show() = compose.setContent {
        OscylTheme(OscylThemeMode.Dark) {
            Box {
                DockedSheet(open, onClose = { open = false }, title = "Sheet") {
                    BasicText("Body", Modifier.fillMaxWidth().height(400.dp).testTag("body"))
                }
            }
        }
    }

    private fun settled() {
        compose.mainClock.advanceTimeBy(1000)
        compose.waitForIdle()
    }

    private fun panes() = compose.onAllNodes(pane, useUnmergedTree = true).fetchSemanticsNodes().size

    @Test
    fun tapOnVeilCloses() {
        show()
        settled()
        compose.onRoot().performTouchInput { click(topCenter.copy(y = 40f)) }
        settled()
        assertFalse(open)
    }

    @Test
    fun backCloses() {
        show()
        settled()
        compose.runOnIdle { compose.activity.onBackPressedDispatcher.onBackPressed() }
        settled()
        assertFalse(open)
    }

    @Test
    fun dragDownPastHalfCloses() {
        show()
        settled()
        compose.onNodeWithTag("body").performTouchInput { swipeDown(top, bottom + height, durationMillis = 600) }
        settled()
        assertFalse(open)
    }

    @Test
    fun shortDragSettlesOpen() {
        show()
        settled()
        compose.onNodeWithTag("body").performTouchInput { swipeDown(top, top + 40f, durationMillis = 600) }
        settled()
        assertTrue(open)
        assertEquals(1, panes())
    }

    @Test
    fun rotationKeepsSheetOpen() {
        val restoration = StateRestorationTester(compose)
        restoration.setContent {
            var shown by rememberSaveable { mutableStateOf(true) }
            OscylTheme(OscylThemeMode.Dark) {
                Box {
                    DockedSheet(shown, onClose = { shown = false }, title = "Sheet") {
                        BasicText("Body", Modifier.fillMaxWidth().height(400.dp).testTag("body"))
                    }
                }
            }
        }
        settled()
        restoration.emulateSavedInstanceStateRestore()
        settled()
        assertEquals(1, panes())
        // Its drag range came back with it: a long drag still puts it away.
        compose.onNodeWithTag("body").performTouchInput { swipeDown(top, bottom + height, durationMillis = 600) }
        settled()
        assertEquals(0, panes())
    }

    @Test
    fun closedSheetIsSilentToScreenReaders() {
        open = false
        show()
        settled()
        assertEquals(0, panes())
        compose.onNodeWithTag("body").assertDoesNotExist()
    }
}
