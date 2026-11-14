package io.github.oscyl.parts

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertWidthIsAtLeast
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.junit4.ComposeContentTestRule
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.click
import androidx.compose.ui.test.doubleClick
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.platform.LocalOscylHaptics
import io.github.oscyl.platform.OscylHaptics
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.Dial
import io.github.oscyl.ui.parts.Segments
import io.github.oscyl.ui.parts.stageIcon
import io.github.oscyl.ui.parts.StageItem
import io.github.oscyl.ui.parts.StageRail
import kotlin.math.abs
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** What the dial, segments and stage rail do; how they look is in the preview goldens. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class PartsBehaviourTest {
    @get:Rule val compose = createComposeRule()

    private class CountingHaptics : OscylHaptics {
        var ticks = 0
        var detents = 0
        var bumps = 0
        var confirms = 0
        override fun tick() { ticks++ }
        override fun detent() { detents++ }
        override fun bump() { bumps++ }
        override fun confirm() { confirms++ }
    }

    private val dial get() = compose.onNode(SemanticsMatcher.keyIsDefined(SemanticsProperties.ProgressBarRangeInfo))

    private fun SemanticsNodeInteraction.customAction(label: String) =
        fetchSemanticsNode().config[SemanticsActions.CustomActions].single { it.label == label }

    @Test
    fun dialTalkBackActions() {
        var value by mutableIntStateOf(618)
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                Dial(value, saved = 431, range = 0..1023, label = "Cutoff", format = Int::toString, onChange = { value = it })
            }
        }
        dial.assert(SemanticsMatcher.expectValue(SemanticsProperties.ProgressBarRangeInfo, ProgressBarRangeInfo(618f, 0f..1023f)))
        dial.assert(SemanticsMatcher.expectValue(SemanticsProperties.StateDescription, "618, edited, was 431"))
        dial.assert(SemanticsMatcher.expectValue(SemanticsProperties.ContentDescription, listOf("Cutoff")))

        // Increase and decrease move one printed scale step (a twentieth of the range).
        compose.runOnIdle { dial.customAction("Increase").action() }
        compose.runOnIdle { assertEquals(669, value) }
        compose.runOnIdle { dial.customAction("Decrease").action() }
        compose.runOnIdle { assertEquals(618, value) }
        compose.runOnIdle { dial.customAction("Restore saved value").action() }
        compose.runOnIdle { assertEquals(431, value) }
        // Unedited: the state is the value alone, nothing to restore; the stepper is always offered.
        dial.assert(SemanticsMatcher.expectValue(SemanticsProperties.StateDescription, "431"))
        val labels = dial.fetchSemanticsNode().config[SemanticsActions.CustomActions].map { it.label }
        assertEquals(listOf("Increase", "Decrease", "Set exactly"), labels)
        // Ends clamp.
        value = 1020
        compose.runOnIdle { dial.customAction("Increase").action() }
        compose.runOnIdle { assertEquals(1023, value) }
    }

    @Test
    fun dialAndSegmentTargetsAre48dp() {
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                androidx.compose.foundation.layout.Column {
                    Dial(618, saved = 431, range = 0..1023, label = "Cutoff", format = Int::toString, onChange = {})
                    Segments(listOf("Off", "On"), selected = 1, saved = 0, onSelect = {})
                }
            }
        }
        // A phone's knob: a 96 dp target, "was" inline after the value.
        dial.assertWidthIsAtLeast(96.dp).assertHeightIsAtLeast(96.dp)
        compose.onNodeWithText("On").assertHeightIsAtLeast(48.dp)
        // The saved option, not chosen, says both.
        compose.onNodeWithText("Off").assert(SemanticsMatcher.expectValue(SemanticsProperties.StateDescription, "Not selected, saved choice"))
    }

    /** The knob's centre, in the dial's merged node: the face is its top 96 dp. */
    private fun androidx.compose.ui.test.TouchInjectionScope.disc() = Offset(width / 2f, 48.dp.toPx())

    @Test
    fun aDoubleTapRestoresSaved() {
        var value by mutableIntStateOf(618)
        val haptics = CountingHaptics()
        compose.setContent {
            CompositionLocalProvider(LocalOscylHaptics provides haptics) {
                OscylTheme(OscylThemeMode.Dark) {
                    Dial(value, saved = 431, range = 0..1023, label = "Cutoff", format = Int::toString, onChange = { value = it })
                }
            }
        }
        dial.performTouchInput { doubleClick(disc()) }
        compose.runOnIdle { assertEquals(431, value) }
        assertEquals(1, haptics.confirms)
    }

    @Test
    fun aTapOnWasInTheScalesFootRestoresSaved() {
        var value by mutableIntStateOf(618)
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                Dial(value, saved = 431, range = 0..1023, label = "Cutoff", format = Int::toString, onChange = { value = it })
            }
        }
        // Under the disc, in the face's foot, where "↶ was 431" is printed.
        dial.performTouchInput { click(Offset(width / 2f, 90.dp.toPx())) }
        compose.runOnIdle { assertEquals(431, value) }
    }

    @Test
    fun knobGainAndFineControl() {
        var value by mutableIntStateOf(0)
        val haptics = CountingHaptics()
        compose.setContent {
            CompositionLocalProvider(LocalOscylHaptics provides haptics) {
                OscylTheme(OscylThemeMode.Dark) {
                    Dial(value, saved = 0, range = 0..1000, label = "Cutoff", format = Int::toString, onChange = { value = it })
                }
            }
        }
        // 110 dp up, fast (about 3 dp/ms): half the range.
        dial.performTouchInput {
            down(disc())
            repeat(10) { advanceEventTime(4); moveBy(Offset(0f, -11.dp.toPx())) }
            up()
        }
        compose.runOnIdle { assertEquals(500, value) }
        assertTrue("a tick at each printed step crossed", haptics.ticks > 0)
        assertEquals("a detent at the centre", 1, haptics.detents)

        // 110 dp down, slowly (0.0625 dp/ms): about 0.35 of that.
        dial.performTouchInput {
            down(disc())
            repeat(110) { advanceEventTime(16); moveBy(Offset(0f, 1.dp.toPx())) }
            up()
        }
        compose.runOnIdle { assertEquals((500 - 175).toFloat(), value.toFloat(), 3f) }

        // An end stop bumps.
        dial.performTouchInput {
            down(disc())
            repeat(10) { advanceEventTime(4); moveBy(Offset(0f, 30.dp.toPx())) }
            up()
        }
        compose.runOnIdle { assertEquals(0, value) }
        assertEquals(1, haptics.bumps)
    }

    @Test
    fun aKnobHoldsTheListStillAndTheLabelScrollsIt() {
        var value by mutableIntStateOf(500)
        lateinit var list: androidx.compose.foundation.lazy.LazyListState
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                list = androidx.compose.foundation.lazy.rememberLazyListState()
                androidx.compose.foundation.lazy.LazyColumn(androidx.compose.ui.Modifier.fillMaxSize(), state = list) {
                    item { androidx.compose.foundation.layout.Spacer(androidx.compose.ui.Modifier.height(200.dp)) }
                    item { Dial(value, saved = 500, range = 0..1000, label = "Cutoff", format = Int::toString, onChange = { value = it }) }
                    items(40) { androidx.compose.foundation.layout.Spacer(androidx.compose.ui.Modifier.height(80.dp)) }
                }
            }
        }
        // From the disc: the knob turns, the list stays.
        dial.performTouchInput {
            down(disc())
            repeat(10) { advanceEventTime(8); moveBy(Offset(0f, -8.dp.toPx())) }
            up()
        }
        compose.runOnIdle {
            assertTrue("the knob turned", value > 500)
            assertEquals(0, list.firstVisibleItemIndex)
            assertEquals(0, list.firstVisibleItemScrollOffset)
        }
        // From the label: the list scrolls, the knob stays.
        val turned = value
        compose.onNodeWithText("Cutoff", useUnmergedTree = true).performTouchInput {
            down(center)
            repeat(10) { advanceEventTime(8); moveBy(Offset(0f, -12.dp.toPx())) }
            up()
        }
        compose.runOnIdle {
            assertEquals(turned, value)
            assertTrue("the list scrolled", list.firstVisibleItemIndex > 0 || list.firstVisibleItemScrollOffset > 0)
        }
    }

    @Test
    fun plateGlidesWithMotion() {
        // The control for ReducedMotionTest: at the normal animator scale, one frame is not enough.
        assertFalse("the plate is still on its way", plateArrivedAfterOneFrame(compose))
    }

    @Test
    fun stageRailAnswersATouchAndGlidesAtOnce() {
        val stages = listOf("Voice", "Oscillators", "Filter", "Envelopes", "Modulation", "Effects").map { StageItem(it, "summary", 0, stageIcon(null)) }
        val jumps = mutableStateListOf<Int>()
        val scrubs = mutableStateListOf<Int>()
        var taps = 0
        val haptics = CountingHaptics()
        compose.setContent {
            CompositionLocalProvider(LocalOscylHaptics provides haptics) {
                OscylTheme(OscylThemeMode.Dark) {
                    StageRail(stages, current = 0, onJump = { jumps += it }, onScrub = { scrubs += it }, onTap = { taps++ })
                }
            }
        }
        val tabs = compose.onAllNodes(SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.Tab))
        assertEquals(6, tabs.fetchSemanticsNodes().size)
        tabs[0].assert(SemanticsMatcher.expectValue(SemanticsProperties.Selected, true))
        // At rest every stage is a 48 dp target, and the targets do not overlap.
        val bounds = (0 until 6).map { tabs[it].fetchSemanticsNode().boundsInRoot }
        (0 until 6).forEach { tabs[it].assertWidthIsAtLeast(48.dp).assertHeightIsAtLeast(48.dp) }
        bounds.zipWithNext().forEach { (a, b) -> assertTrue("rows overlap: $a, $b", a.bottom <= b.top + 0.5f) }
        val rail = compose.onNode(SemanticsMatcher.keyIsDefined(SemanticsProperties.SelectableGroup))
        val railAt = rail.fetchSemanticsNode().boundsInRoot.topLeft
        fun centre(i: Int) = bounds[i].center - railAt

        // A tap: the tick is on the touch, the jump on the release.
        rail.performTouchInput { down(centre(3)) }
        compose.runOnIdle {
            assertEquals("the touch itself is answered", 1, haptics.ticks)
            assertTrue(jumps.isEmpty())
        }
        rail.performTouchInput { up() }
        compose.runOnIdle {
            assertEquals(listOf(3), jumps.toList())
            assertEquals(1, taps)
            assertTrue(scrubs.isEmpty())
        }

        // A glide, with nothing to wait for: down on Oscillators (not the current stage, so the
        // page follows as soon as it is a glide), straight down to the fourth stage. A tick per
        // stage, release lands there, and it was not a tap.
        rail.performTouchInput {
            down(centre(1))
            moveTo(centre(2))
        }
        // Open, the rows stand where they stood: nothing moved under the finger.
        compose.runOnIdle { assertEquals(bounds, (0 until 6).map { tabs[it].fetchSemanticsNode().boundsInRoot }) }
        rail.performTouchInput {
            moveTo(centre(3))
            up()
        }
        compose.runOnIdle {
            assertEquals(listOf(1, 2, 3), scrubs.toList())
            assertEquals(listOf(3, 3), jumps.toList())
            assertEquals(1, taps)
            assertEquals(1 + 1 + 2, haptics.ticks) // the tap; then the touch and two stages passed
        }
    }
}

/**
 * Whether, one frame after an Off/On pair flips to On, the plate (light grey in dark)
 * covers On's far edge.
 */
internal fun plateArrivedAfterOneFrame(rule: ComposeContentTestRule): Boolean {
    var selected by mutableIntStateOf(0)
    rule.setContent {
        OscylTheme(OscylThemeMode.Dark) {
            Segments(listOf("Off", "On"), selected, saved = 0, onSelect = { selected = it })
        }
    }
    rule.waitForIdle()
    rule.mainClock.autoAdvance = false
    rule.onNodeWithText("On").performClick()
    rule.mainClock.advanceTimeByFrame()
    val on = rule.onNodeWithText("On").fetchSemanticsNode().boundsInRoot
    // A few px inside On's far edge, at mid height: has the plate arrived?
    val px = rule.onRoot().captureToImage().toPixelMap()[(on.right - 6).toInt(), on.center.y.toInt()]
    val plate = OscylTokens.Dark.plate
    return abs(px.red - plate.red) < 0.06f && abs(px.green - plate.green) < 0.06f && abs(px.blue - plate.blue) < 0.06f
}
