package io.github.oscyl.parts

import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performTextReplacement
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.click
import androidx.compose.ui.test.longClick
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.parts.Dial
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** A knob's exact value: a long press on its value opens the stepper. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class ValueStepperTest {
    @get:Rule val compose = createComposeRule()

    private var value by mutableIntStateOf(500)
    private val changes = mutableListOf<Int>()
    private var done = 0

    private fun show(start: Int = 500, typed: ((String) -> Int?)? = null) {
        value = start
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                Dial(
                    value, saved = 500, range = 0..1023, label = "Cutoff", format = { "$it" },
                    onChange = { changes += it; value = it },
                    onRelease = { done++ },
                    typed = typed,
                )
            }
        }
        compose.onNodeWithTag("dial-value", useUnmergedTree = true).performTouchInput { longClick() }
        compose.waitForIdle()
    }

    @Test
    fun longPressOnValueOpensStepper() {
        show()
        compose.onNodeWithTag("stepper").assertExists()
    }

    @Test
    fun tapOnValueOpensStepperAndRestoresSaved() {
        value = 700
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                Dial(value, saved = 500, range = 0..1023, label = "Cutoff", format = { "$it" }, onChange = { changes += it; value = it }, onRelease = { done++ })
            }
        }
        compose.onNodeWithTag("dial-value", useUnmergedTree = true).performTouchInput { click() }
        compose.waitForIdle()
        compose.onNodeWithTag("stepper-restore").performClick()
        compose.waitForIdle()
        assertEquals(500, value)
        compose.onNodeWithTag("stepper").assertDoesNotExist()
    }

    @Test
    fun plusStepsOnceAndSeals() {
        show()
        compose.onNodeWithTag("stepper-plus").performClick()
        compose.waitForIdle()
        assertEquals(listOf(501), changes)
        assertEquals(1, done)
    }

    @Test
    fun heldPlusRepeats() {
        show()
        compose.mainClock.autoAdvance = false
        compose.onNodeWithTag("stepper-plus").performTouchInput { down(center) }
        compose.mainClock.advanceTimeBy(1500)
        compose.onNodeWithTag("stepper-plus").performTouchInput { up() }
        compose.mainClock.autoAdvance = true
        compose.waitForIdle()
        assertTrue("held for 1.5 s it stepped ${changes.size} times", changes.size > 8)
        assertEquals((501..500 + changes.size).toList(), changes)
        assertEquals(1, done)
    }

    @Test
    fun stepperStopsAtEnds() {
        show(start = 1023)
        compose.onNodeWithTag("stepper-plus").assertIsNotEnabled()
        compose.onNodeWithTag("stepper-plus").performTouchInput { click(center) }
        compose.waitForIdle()
        assertEquals(emptyList<Int>(), changes)
    }

    @Test
    fun stepperSpeaksValue() {
        show()
        val node = compose.onNodeWithTag("stepper-value").fetchSemanticsNode()
        assertEquals("500", node.config[SemanticsProperties.StateDescription])
    }

    @Test
    fun typedValueSetsAndSeals() {
        show(typed = { it.trim().toIntOrNull() })
        compose.onNodeWithTag("stepper-value").performTextReplacement("618")
        compose.onNodeWithTag("stepper-value").performImeAction()
        compose.waitForIdle()
        assertEquals(listOf(618), changes)
        assertEquals(1, done)
    }

    @Test
    fun typedNonsenseShowsNoSuchValue() {
        show(typed = { it.trim().toIntOrNull() })
        compose.onNodeWithTag("stepper-value").performTextReplacement("abc")
        compose.onNodeWithTag("stepper-value").performImeAction()
        compose.waitForIdle()
        compose.onNodeWithText("No such value").assertExists()
        assertEquals(emptyList<Int>(), changes)
        assertEquals(0, done)
    }
}
