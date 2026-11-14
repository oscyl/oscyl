package io.github.oscyl.shell

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsNode
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.library.LibraryScreen
import io.github.oscyl.ui.library.NoLibraryActions
import io.github.oscyl.ui.library.SampleUi
import io.github.oscyl.ui.parts.CollapsingBarState
import io.github.oscyl.ui.parts.CollapsingTopBar
import io.github.oscyl.ui.parts.LargeTitle
import io.github.oscyl.ui.parts.rememberCollapsingBarState
import io.github.oscyl.ui.preview.CutoutFrame
import io.github.oscyl.ui.preview.FakeCutout
import io.github.oscyl.ui.shell.Destination
import io.github.oscyl.ui.shell.DestinationHost
import io.github.oscyl.ui.shell.Shell
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The collapsing bar, kept destinations and the insets every part keeps clear. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class TopBarBehaviourTest {
    @get:Rule val compose = createComposeRule()

    @Test
    fun barFractionFollowsTheScroll() {
        lateinit var list: LazyListState
        lateinit var bar: CollapsingBarState
        var titlePx = 0
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                list = rememberLazyListState()
                bar = rememberCollapsingBarState(list)
                titlePx = with(LocalDensity.current) { 40.dp.roundToPx() }
                Column(Modifier.fillMaxSize()) {
                    CollapsingTopBar(bar, "Title", "Subtitle")
                    LazyColumn(Modifier.fillMaxSize(), state = list) {
                        item { LargeTitle("Title", bar, Modifier.height(40.dp)) }
                        items(60) { BasicText("Row $it", Modifier.height(48.dp)) }
                    }
                }
            }
        }
        compose.runOnIdle { assertEquals(0f, bar.fraction(), 0f) }
        compose.runOnIdle { runBlocking { list.scrollToItem(0, titlePx / 2) } }
        compose.runOnIdle { assertEquals(0.5f, bar.fraction(), 0.05f) }
        compose.runOnIdle { runBlocking { list.scrollToItem(0, titlePx) } }
        compose.runOnIdle { assertEquals(1f, bar.fraction(), 0f) }
        compose.runOnIdle { runBlocking { list.scrollToItem(20) } }
        compose.runOnIdle { assertEquals(1f, bar.fraction(), 0f) }
        // The title is one heading, in the bar, wherever it shows.
        assertEquals(1, compose.onAllNodesWithHeading().size)
    }

    @Test
    fun aSwitchKeepsEachDestinationsScroll() {
        var current by mutableStateOf(Destination.Library)
        val lists = mutableMapOf<Destination, LazyListState>()
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                DestinationHost(current, Modifier.fillMaxSize()) { d ->
                    val list = rememberLazyListState()
                    lists[d] = list
                    LazyColumn(Modifier.fillMaxSize(), state = list) { items(100) { BasicText("${d.name} $it", Modifier.height(48.dp)) } }
                }
            }
        }
        compose.runOnIdle { runBlocking { lists.getValue(Destination.Library).scrollToItem(40) } }
        compose.runOnIdle { current = Destination.Generate }
        compose.waitForIdle()
        // Only the destination shown is in the tree (the other is silent and untouchable).
        assertTrue(compose.onAllNodesWithTextPrefix("Library").isEmpty())
        compose.runOnIdle { current = Destination.Library }
        compose.waitForIdle()
        compose.runOnIdle { assertEquals(40, lists.getValue(Destination.Library).firstVisibleItemIndex) }
        assertTrue(compose.onAllNodesWithTextPrefix("Generate").isEmpty())
    }

    @Test
    fun noPartSitsUnderAPortraitCutout() = listOf(FakeCutout.PunchHole, FakeCutout.Corner, FakeCutout.TallNotch, FakeCutout.Double).forEach(::checkClear)

    @Test
    @Config(qualifiers = "w800dp-h360dp-xxhdpi")
    fun noPartSitsUnderALandscapeCutout() = listOf(FakeCutout.Landscape90, FakeCutout.Landscape270).forEach(::checkClear)

    private var cutout by mutableStateOf(FakeCutout.PunchHole)
    private var started = false

    /** Every control, and every line of text, lies inside the window less [c]'s insets. */
    private fun checkClear(c: FakeCutout) {
        cutout = c
        if (!started) {
            started = true
            compose.setContent {
                OscylTheme(OscylThemeMode.Dark) {
                    CutoutFrame(cutout) {
                        Shell(Destination.Library, onSelect = {}, saveCount = 4, onSave = {}) { modifier ->
                            LibraryScreen(SampleUi.copy(programs = null), NoLibraryActions, onOpenSynth = {}, modifier)
                        }
                    }
                }
            }
        }
        compose.waitForIdle()
        val root = compose.onRoot().getBoundsInRoot()
        val density = compose.density
        val safe = with(density) {
            Rect(c.left.dp.toPx(), c.top.dp.toPx(), root.right.toPx() - c.right.dp.toPx(), root.bottom.toPx() - c.bottom.dp.toPx())
        }
        val parts = compose.onAllNodes(
            SemanticsMatcher("a control or a line of text") {
                SemanticsActions.OnClick in it.config || SemanticsProperties.Text in it.config
            },
        ).fetchSemanticsNodes()
        assertTrue("$c: parts found", parts.isNotEmpty())
        parts.forEach { node ->
            val b = node.boundsInRoot
            // Off-screen rows (scrolled out) are not on the glass.
            if (b.bottom <= 0f || b.top >= with(density) { root.bottom.toPx() }) return@forEach
            assertTrue("$c: ${node.label()} at $b lies inside $safe", b.left >= safe.left - 0.5f && b.top >= safe.top - 0.5f && b.right <= safe.right + 0.5f && b.bottom <= safe.bottom + 0.5f)
        }
    }

    private fun SemanticsNode.label() =
        (config.getOrElse(SemanticsProperties.Text) { emptyList() }.joinToString() + config.getOrElse(SemanticsProperties.ContentDescription) { emptyList() }.joinToString()).ifEmpty { "node $id" }

    private fun androidx.compose.ui.test.SemanticsNodeInteractionsProvider.onAllNodesWithHeading() =
        onAllNodes(SemanticsMatcher.keyIsDefined(SemanticsProperties.Heading)).fetchSemanticsNodes()

    private fun androidx.compose.ui.test.SemanticsNodeInteractionsProvider.onAllNodesWithTextPrefix(prefix: String) =
        onAllNodes(SemanticsMatcher("text starts with $prefix") { n -> n.config.getOrElse(SemanticsProperties.Text) { emptyList() }.any { it.text.startsWith("$prefix ") } }).fetchSemanticsNodes()
}
