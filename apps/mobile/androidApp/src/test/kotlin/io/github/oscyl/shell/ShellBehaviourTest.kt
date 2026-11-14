package io.github.oscyl.shell

import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.onRoot
import io.github.oscyl.ui.generate.SampleGenerate
import io.github.oscyl.ui.generate.NoGenerateActions
import io.github.oscyl.ui.generate.GenerateScreen
import io.github.oscyl.ui.shell.LocalIslandClearance
import io.github.oscyl.ui.shell.Shell
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assertContentDescriptionContains
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsNotSelected
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertWidthIsAtLeast
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.longClick
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.preview.PreviewPlatform
import io.github.oscyl.ui.shell.App
import io.github.oscyl.ui.shell.Destination
import io.github.oscyl.ui.shell.Island
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/**
 * What the island does; how it looks is in the preview goldens
 * (every @Preview becomes a Roborazzi capture).
 */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class ShellBehaviourTest {
    @get:Rule val compose = createComposeRule()

    private val tabs get() = compose.onAllNodes(SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.Tab))

    @Test
    fun islandSelectionSurvivesRecreation() {
        val restoration = StateRestorationTester(compose)
        restoration.setContent { App(PreviewPlatform) }
        tabs[0].assertIsSelected()
        tabs[2].performClick()
        tabs[2].assertIsSelected()
        restoration.emulateSavedInstanceStateRestore()
        tabs[2].assertIsSelected()
        tabs[0].assertIsNotSelected()
    }

    /** A program list in sight puts the search key on the island; tapped, the island is the field. */
    @Test
    fun theIslandTurnsIntoTheSearchField() {
        val search = io.github.oscyl.ui.shell.ProgramSearch()
        compose.setContent {
            androidx.compose.runtime.CompositionLocalProvider(io.github.oscyl.ui.shell.LocalProgramSearch provides search) {
                OscylTheme(OscylThemeMode.Dark) {
                    Shell(Destination.Library, onSelect = {}, saveCount = 0, onSave = {}) { modifier ->
                        androidx.compose.foundation.layout.Box(modifier)
                    }
                }
            }
        }
        // No list in sight: no key.
        assertEquals(0, compose.onAllNodes(androidx.compose.ui.test.hasTestTag("search-open")).fetchSemanticsNodes().size)
        compose.runOnIdle { search.available = true }
        compose.onNode(androidx.compose.ui.test.hasTestTag("search-open")).performClick()
        compose.onNode(androidx.compose.ui.test.hasTestTag("search-field")).assertExists()
        assertEquals(0, tabs.fetchSemanticsNodes().size)
        // The ✕ of an empty field gives the island back.
        compose.onNode(androidx.compose.ui.test.hasTestTag("search-close")).performClick()
        assertTrue(tabs.fetchSemanticsNodes().isNotEmpty())
        compose.onNode(androidx.compose.ui.test.hasTestTag("search-open")).assertExists()
    }

    /** Robolectric has no system bars: the navigation bar's height is handed in, as the platform would. */
    @Test
    @Config(qualifiers = "w900dp-h700dp-xhdpi")
    fun besideASideIslandTheFootKeepsClearOfTheNavigationBar() {
        var clearance = 0.dp
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                Shell(Destination.Generate, onSelect = {}, saveCount = 0, onSave = {}, systemBottom = 48.dp) { modifier ->
                    clearance = LocalIslandClearance.current
                    GenerateScreen(SampleGenerate, NoGenerateActions, modifier)
                }
            }
        }
        compose.runOnIdle { assertEquals(48.dp, clearance) }
        // The docked Generate stands above the bar, not under it.
        val root = compose.onRoot().getBoundsInRoot()
        val generate = compose.onNode(hasText("Generate") and SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.Button)).getBoundsInRoot()
        assertTrue("Generate's foot ${generate.bottom} clears the bar (root ${root.bottom})", generate.bottom <= root.bottom - 48.dp)
    }

    @Test
    fun islandHas48dpTargetsAndLabels() {
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) { Island(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {}) }
        }
        val nodes = tabs.fetchSemanticsNodes()
        assertEquals(3, nodes.size)
        nodes.forEachIndexed { i, node ->
            val label = node.config.getOrElse(SemanticsProperties.Text) { emptyList() }.joinToString()
            assertTrue("tab $i has a label", label.isNotBlank())
            assertTrue("tab $i says whether it is selected", SemanticsProperties.Selected in node.config)
            tabs[i].assertWidthIsAtLeast(48.dp).assertHeightIsAtLeast(48.dp)
        }
        tabs[1].assertIsSelected()
        // The Save plate: a 48 dp button whose label says the count in words.
        compose.onNode(hasClickAction() and SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.Button))
            .assertContentDescriptionContains("Save 4 edited programs")
            .assertWidthIsAtLeast(48.dp)
            .assertHeightIsAtLeast(48.dp)
        // The ATF gate runs on a device: the instrumented tests on the Gradle Managed Device (M3 Task 4).
    }

    /** The Save key is one size whatever it says, saves on a tap and reviews on a hold. */
    @Test
    fun theSaveKeyKeepsItsSizeAndItsTwoActions() {
        var count by androidx.compose.runtime.mutableIntStateOf(0)
        var progress by androidx.compose.runtime.mutableStateOf<Float?>(null)
        var sync by androidx.compose.runtime.mutableStateOf(false)
        val done = mutableListOf<String>()
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                Island(Destination.Editor, onSelect = {}, saveCount = count, onSave = { done += "save" }, onReview = { done += "review" }, progress = progress, sync = sync)
            }
        }
        val key = compose.onNode(SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.Button))
        val tabsAt = { tabs.fetchSemanticsNodes().map { it.boundsInRoot } }
        val clean = key.fetchSemanticsNode().boundsInRoot
        val tabsClean = tabsAt()
        key.assertContentDescriptionContains("Saved")
        // Clean, one edit, more than 99, a save under way: the key and the tabs stay where they are.
        for ((n, p) in listOf(1 to null, 140 to null, 140 to 0.5f)) {
            count = n
            progress = p
            compose.waitForIdle()
            assertEquals("the key at $n", clean, key.fetchSemanticsNode().boundsInRoot)
            assertEquals("the tabs at $n", tabsClean, tabsAt())
        }
        key.assertContentDescriptionContains("Save 140 edited programs")
        // A synth's slots say Sync: the same width, so moving between destinations moves no tab.
        sync = true
        compose.waitForIdle()
        assertEquals("the key on a synth", clean, key.fetchSemanticsNode().boundsInRoot)
        sync = false
        progress = null
        key.performClick()
        key.performTouchInput { longClick() }
        compose.runOnIdle { assertEquals(listOf("save", "review"), done) }
    }
}
