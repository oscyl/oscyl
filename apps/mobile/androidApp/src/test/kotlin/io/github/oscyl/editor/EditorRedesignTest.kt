package io.github.oscyl.editor

import androidx.compose.runtime.setValue
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.activity.BackEventCompat
import androidx.activity.ComponentActivity
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.swipeLeft
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.editor.EditorActions
import io.github.oscyl.ui.editor.EditorScreen
import io.github.oscyl.ui.editor.SampleHead
import io.github.oscyl.ui.editor.StageViewTag
import io.github.oscyl.ui.editor.samplePanel
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.swipeUp
import androidx.compose.ui.platform.testTag
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.semantics.getOrNull
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The phone editor: the full-screen graph, the rail's Compare and the stage view. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class EditorRedesignTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    private val compares = mutableStateListOf<Boolean>()

    private val actions = object : EditorActions {
        override fun compare(on: Boolean) { compares += on }
    }

    private fun show(restoration: StateRestorationTester? = null) {
        val content = @androidx.compose.runtime.Composable {
            OscylTheme(OscylThemeMode.Dark) {
                // The filter first: its picture is in view.
                val panel = remember { samplePanel(start = 3) }
                EditorScreen(SampleHead, panel, actions, onOpenLibrary = {})
            }
        }
        if (restoration != null) restoration.setContent(content) else compose.setContent(content)
    }

    private val close get() = compose.onNodeWithContentDescription("Close full screen")

    private fun open() {
        compose.onNodeWithContentDescription("Open Filter full screen").performClick()
        compose.waitForIdle()
        close.assertExists()
    }

    private fun back(progress: List<Float>, cancel: Boolean) {
        val back = compose.activity.onBackPressedDispatcher
        compose.runOnIdle { back.dispatchOnBackStarted(BackEventCompat(0f, 400f, 0f, BackEventCompat.EDGE_LEFT)) }
        progress.forEach { p -> compose.runOnIdle { back.dispatchOnBackProgressed(BackEventCompat(p * 300f, 400f, p, BackEventCompat.EDGE_LEFT)) } }
        compose.runOnIdle { if (cancel) back.dispatchOnBackCancelled() else back.onBackPressed() }
        compose.waitForIdle()
    }

    /**
     * The owner's "solid colour in the editor": a graph left full screen, the program closed
     * (another source chosen in the Library), then another program opened. The screen's
     * saved "graph open" came back for the new program and hid its list.
     */
    @Test
    fun aGraphLeftOpenNeverComesBackOverAnotherProgram() {
        var shown by mutableStateOf<io.github.oscyl.ui.editor.EditorHead?>(SampleHead)
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                val head = shown
                val panel = remember(head) { samplePanel(start = 3) }
                EditorScreen(head, if (head == null) null else panel, actions, onOpenLibrary = {})
            }
        }
        open()
        compose.runOnIdle { shown = null }
        compose.onNodeWithText("No program open").assertExists()
        compose.runOnIdle { shown = SampleHead.copy(name = "Another", slot = "005") }
        compose.waitForIdle()
        // The new program's own list, with its pictures to open: no page stands over it.
        close.assertDoesNotExist()
        compose.onNodeWithContentDescription("Open Filter full screen").assertExists()
        // And the first program again: its graph was closed by leaving it, not kept for later.
        compose.runOnIdle { shown = SampleHead }
        compose.waitForIdle()
        close.assertDoesNotExist()
        compose.onNodeWithContentDescription("Open Filter full screen").assertExists()
    }

    @Test
    fun theGraphOpensAndBackClosesItPartwayAndWhole() {
        show()
        open()
        compose.onNodeWithText("Glass Choir · 3 of 4").assertExists()
        // Partway, then let go: it stays open.
        back(listOf(0.2f, 0.4f), cancel = true)
        close.assertExists()
        // All the way: back into its frame.
        back(listOf(0.3f, 0.7f, 1f), cancel = false)
        close.assertDoesNotExist()
        compose.onNodeWithContentDescription("Open Filter full screen").assertExists()
    }

    @Test
    fun theChevronClosesAndThePagerTurnsStages() {
        show()
        open()
        compose.onNodeWithText("Glass Choir · 3 of 4").assertExists()
        compose.onNodeWithTag("graph pager", useUnmergedTree = true).performTouchInput { swipeLeft() }
        compose.waitForIdle()
        compose.onNodeWithText("Glass Choir · 4 of 4").assertExists()
        close.performClick()
        compose.waitForIdle()
        close.assertDoesNotExist()
    }

    @Test
    fun rotatingKeepsTheGraphOpenOnItsStage() {
        val restoration = StateRestorationTester(compose)
        show(restoration)
        open()
        restoration.emulateSavedInstanceStateRestore()
        compose.waitForIdle()
        close.assertExists()
        compose.onNodeWithText("Glass Choir · 3 of 4").assertExists()
    }

    private fun firstLook(taught: MutableList<String>) = compose.setContent {
        androidx.compose.runtime.CompositionLocalProvider(
            io.github.oscyl.ui.preferences.LocalTeaching provides io.github.oscyl.ui.preferences.Teaching({ it in taught }, { taught += it }),
        ) {
            OscylTheme(OscylThemeMode.Dark) { EditorScreen(SampleHead, remember { samplePanel(start = 3) }, actions, onOpenLibrary = {}) }
        }
    }

    @Test
    fun theOpenRailStaysUnderATouchAndClosesOnceItHasLanded() {
        val taught = mutableListOf<String>()
        firstLook(taught)
        val rail = compose.onNodeWithTag("stage-rail", useUnmergedTree = true)
        val open = rail.fetchSemanticsNode().size.width
        // The first touch on it: it stays, names and all (it used to leave under the finger).
        rail.performTouchInput { down(center) }
        compose.waitForIdle()
        assertEquals(open, rail.fetchSemanticsNode().size.width)
        assertTrue(taught.isEmpty())
        // Let go: the page goes there, and then the rail rests small, its names kept as shown.
        rail.performTouchInput { up() }
        compose.mainClock.advanceTimeBy(1000)
        compose.waitForIdle()
        assertTrue(rail.fetchSemanticsNode().size.width < open)
        assertEquals(listOf("stage-rail"), taught)
    }

    @Test
    fun theOpenRailClosesOnTheFirstTouchOnThePage() {
        val taught = mutableListOf<String>()
        firstLook(taught)
        val rail = compose.onNodeWithTag("stage-rail", useUnmergedTree = true)
        val open = rail.fetchSemanticsNode().size.width
        compose.onNodeWithTag("editor-list").performTouchInput { down(androidx.compose.ui.geometry.Offset(20f, centerY)); up() }
        compose.waitForIdle()
        assertTrue(rail.fetchSemanticsNode().size.width < open)
        assertEquals(listOf("stage-rail"), taught)
    }

    /** The zoomed graph's controls sheet rises to its content, never past it. */
    @Test
    fun theControlsSheetRisesOnlyToItsContent() {
        var page by androidx.compose.runtime.mutableIntStateOf(2)
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                androidx.compose.foundation.layout.Box(androidx.compose.ui.Modifier.fillMaxSize()) {
                    io.github.oscyl.ui.editor.ControlsSheet(
                        remember { samplePanel(start = 3) }, page, actions, room = 780.dp, settled = true,
                        modifier = androidx.compose.ui.Modifier.align(androidx.compose.ui.Alignment.BottomCenter),
                    )
                }
            }
        }
        fun settle() { compose.mainClock.advanceTimeBy(1000); compose.waitForIdle() }
        fun top(tag: String) = compose.onNodeWithTag(tag, useUnmergedTree = true).fetchSemanticsNode().boundsInRoot.top
        val root = compose.onRoot().fetchSemanticsNode().boundsInRoot
        settle()
        // The mixer's three knobs all show in the peek: no grip, no action, and a drag moves nothing.
        compose.onNodeWithTag("controls-grip", useUnmergedTree = true).assertDoesNotExist()
        assertTrue(compose.onAllNodes(androidx.compose.ui.test.SemanticsMatcher("more controls") { it.config.getOrNull(androidx.compose.ui.semantics.SemanticsActions.OnClick)?.label == "Show all the controls" }, useUnmergedTree = true).fetchSemanticsNodes().isEmpty())
        val rest = top("controls-sheet")
        compose.onNodeWithTag("controls-sheet", useUnmergedTree = true).performTouchInput { swipeUp() }
        settle()
        assertEquals(rest, top("controls-sheet"), 0.5f)
        // The filter's four controls do not: the grip is there, and a drag raises the sheet
        // until its last row shows, well short of most of the window.
        page = 3
        settle()
        compose.onNodeWithTag("controls-grip", useUnmergedTree = true).assertExists()
        assertEquals("the peek stands where it stood", rest, top("controls-sheet"), 0.5f)
        compose.onNodeWithTag("controls-sheet", useUnmergedTree = true).performTouchInput { swipeUp(startY = 20f, endY = -600f) }
        settle()
        val raised = top("controls-sheet")
        assertTrue("it rose", raised < rest - 1f)
        assertTrue("to its content, not to most of the window: $raised of ${root.height}", raised > root.bottom - 0.82f * 780 * compose.density.density + 1f)
    }

    @Test
    fun compareAtTheRailsFootHolds() {
        show()
        compose.onNodeWithContentDescription("Hold to compare").performTouchInput { down(center) }
        compose.runOnIdle { assertEquals(listOf(true), compares.toList()) }
        compose.onNodeWithContentDescription("Hold to compare").performTouchInput { up() }
        compose.runOnIdle { assertEquals(listOf(true, false), compares.toList()) }
    }

    @Test
    @Config(qualifiers = "w800dp-h360dp-xxhdpi")
    fun aPhoneOnItsSideGetsTheStageView() {
        show()
        compose.onNodeWithTag(StageViewTag).assertExists()
        // One stage at a time: the filter's own controls, not another stage's.
        compose.onNodeWithContentDescription("Cutoff").assertExists()
        assertTrue(compose.onAllNodesWithContentDescriptionPrefix("Attack").isEmpty())
    }

    private fun androidx.compose.ui.test.SemanticsNodeInteractionsProvider.onAllNodesWithContentDescriptionPrefix(prefix: String) =
        onAllNodes(androidx.compose.ui.test.hasContentDescription(prefix, substring = true)).fetchSemanticsNodes()
}
