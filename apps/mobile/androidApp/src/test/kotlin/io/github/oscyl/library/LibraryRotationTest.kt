package io.github.oscyl.library

import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.android.MainActivity
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** A rotation keeps the library shown and its libraries (one model, one folder). */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class LibraryRotationTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun awaitText(text: String) = compose.waitUntil(10_000) {
        compose.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty()
    }

    @Test
    fun shownLibrarySurvivesRotation() {
        // The first run: start with no synth.
        compose.onNodeWithText("Continue without a synth").performScrollTo().performClick()
        compose.onNodeWithText("New library").performClick()
        // It asks for a name at once: the one it was given is kept.
        compose.waitUntil(10_000) { compose.onAllNodesWithText("Name the new library").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Done").performClick()
        compose.waitUntil(10_000) { compose.onAllNodesWithText("Name the new library").fetchSemanticsNodes().isEmpty() }
        compose.waitUntil(10_000) { compose.onAllNodesWithText("Library 1").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Library 1").performClick()
        awaitText("No programs yet")
        compose.activityRule.scenario.recreate()
        awaitText("No programs yet")
        // Still its programs pane (a phone shows one pane), titled with the library.
        assertEquals(1, compose.onAllNodesWithText("Library 1").fetchSemanticsNodes().size)
        assertEquals(0, compose.onAllNodesWithText("New folder").fetchSemanticsNodes().size)
    }
}
