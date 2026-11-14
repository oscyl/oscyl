package io.github.oscyl.parts

import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.unit.LayoutDirection
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.parts.XyPad
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The XY pad: both axes from one finger, y upward, mirrored right to left. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class XyPadTest {
    @get:Rule val compose = createComposeRule()

    private var x by mutableStateOf(0.5f)
    private var y by mutableStateOf(0.5f)
    private var releases = 0
    private var restores = 0

    private fun show(enabled: Boolean = true, direction: LayoutDirection = LayoutDirection.Ltr) = compose.setContent {
        OscylTheme(OscylThemeMode.Dark) {
            CompositionLocalProvider(LocalLayoutDirection provides direction) {
                XyPad(
                    x, y, savedX = 0.5f, savedY = 0.5f, xLabel = "Cutoff", yLabel = "Resonance", xText = "${(x * 100).toInt()}", yText = "${(y * 100).toInt()}",
                    onMove = { nx, ny -> x = nx; y = ny }, onRelease = { releases++ }, enabled = enabled,
                    onRestore = { x = 0.5f; y = 0.5f; restores++ },
                )
            }
        }
    }

    private val pad get() = compose.onNodeWithTag("xy-pad")

    @Test
    fun dragReportsBothAxes() {
        show()
        pad.performTouchInput {
            down(center)
            moveBy(Offset(width / 5f, -height / 5f))
            up()
        }
        compose.waitForIdle()
        assertTrue("x grew: $x", x > 0.6f)
        assertTrue("y grew upward: $y", y > 0.6f)
    }

    @Test
    fun touchElsewhereMovesThePoint() {
        show()
        pad.performTouchInput {
            down(Offset(width * 0.85f, height * 0.85f))
            up()
        }
        compose.waitForIdle()
        assertTrue("x $x", x > 0.8f)
        assertTrue("y $y", y < 0.2f)
    }

    @Test
    fun releaseCallsOnReleaseOnce() {
        show()
        pad.performTouchInput {
            down(center)
            moveBy(Offset(30f, 0f))
            moveBy(Offset(30f, 0f))
            up()
        }
        compose.waitForIdle()
        assertEquals(1, releases)
    }

    @Test
    fun secondFingerIsIgnored() {
        show()
        pad.performTouchInput {
            down(0, center)
            down(1, Offset(width * 0.9f, height * 0.1f))
            moveBy(1, Offset(-10f, 10f))
            up(1)
            up(0)
        }
        compose.waitForIdle()
        assertEquals(0.5f, x, 0.05f)
        assertEquals(0.5f, y, 0.05f)
        assertEquals(1, releases)
    }

    @Test
    fun disabledPadDoesNotMove() {
        show(enabled = false)
        pad.performTouchInput {
            down(Offset(width * 0.9f, height * 0.9f))
            up()
        }
        compose.waitForIdle()
        assertEquals(0.5f to 0.5f, x to y)
        assertEquals(0, releases)
    }

    @Test
    fun screenReaderAdjustsEachAxis() {
        show()
        val node = pad.fetchSemanticsNode()
        assertEquals("Cutoff 50, Resonance 50", node.config[SemanticsProperties.StateDescription])
        val actions = node.config[SemanticsActions.CustomActions].associateBy { it.label }
        compose.runOnIdle { actions.getValue("More Cutoff").action() }
        compose.runOnIdle { actions.getValue("Less Resonance").action() }
        assertEquals(0.6f, x, 0.001f)
        assertEquals(0.4f, y, 0.001f)
        assertEquals(2, releases)
    }

    @Test
    fun rtlValuePlateStaysBesideThePoint() {
        show(direction = LayoutDirection.Rtl)
        pad.performTouchInput { down(Offset(width * 0.7f, height * 0.5f)) }
        compose.waitForIdle()
        val padBounds = pad.fetchSemanticsNode().boundsInRoot
        val plate = compose.onNodeWithText(" · ", substring = true).fetchSemanticsNode().boundsInRoot
        assertTrue("plate $plate inside pad $padBounds", plate.left >= padBounds.left && plate.right <= padBounds.right)
        pad.performTouchInput { up() }
    }

    @Test
    fun doubleTapRestoresTheSavedPoint() {
        show()
        pad.performTouchInput {
            click(Offset(width * 0.85f, height * 0.15f))
            advanceEventTime(100)
            click(Offset(width * 0.85f, height * 0.15f))
        }
        compose.waitForIdle()
        assertEquals(1, restores)
        assertEquals(0.5f to 0.5f, x to y)
    }

    @Test
    fun rtlMirrorsX() {
        show(direction = LayoutDirection.Rtl)
        pad.performTouchInput {
            down(Offset(width * 0.85f, height * 0.5f))
            up()
        }
        compose.waitForIdle()
        assertTrue("toward the start (right here) is less: $x", x < 0.2f)
    }
}
