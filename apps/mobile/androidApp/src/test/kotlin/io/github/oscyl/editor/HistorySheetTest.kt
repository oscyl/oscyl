package io.github.oscyl.editor

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.click
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.longClick
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.editor.HistoryList
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The snapshot plates on the history sheet: hold stores, tap recalls. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class HistorySheetTest {
    @get:Rule val compose = createComposeRule()

    private var stored by mutableStateOf(listOf(false, false, false))
    private val recalled = mutableListOf<Int>()

    private fun show() = compose.setContent {
        OscylTheme(OscylThemeMode.Dark) {
            HistoryList(
                emptyList(), onJump = {}, onClose = {},
                snapshots = stored,
                onStore = { i -> stored = stored.toMutableList().also { it[i] = true } },
                onRecall = { recalled += it },
            )
        }
    }

    private fun plate(letter: String) = compose.onNodeWithTag("snapshot-$letter", useUnmergedTree = true)

    @Test
    fun holdStoresTapRecalls() {
        show()
        plate("B").performTouchInput { longClick() }
        compose.waitForIdle()
        assertEquals(listOf(false, true, false), stored)
        assertEquals("Stored", plate("B").fetchSemanticsNode().config[SemanticsProperties.StateDescription])
        plate("B").performTouchInput { click() }
        compose.waitForIdle()
        assertEquals(listOf(1), recalled)
    }

    @Test
    fun emptyPlateTapDoesNothing() {
        show()
        plate("A").performTouchInput { click() }
        compose.waitForIdle()
        assertEquals(emptyList<Int>(), recalled)
        assertEquals(listOf(false, false, false), stored)
    }
}
