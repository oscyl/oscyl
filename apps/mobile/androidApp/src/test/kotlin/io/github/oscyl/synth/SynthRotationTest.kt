package io.github.oscyl.synth

import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onAllNodesWithContentDescription
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

/** The synth worker lives in the retained scope: a rotation keeps it and its synths. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class SynthRotationTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test
    fun workerSurvivesRotation() {
        // The first run: start with no synth.
        compose.onNodeWithText("Continue without a synth").performScrollTo().performClick()
        compose.onNodeWithContentDescription("Synths:", substring = true).performClick()
        compose.onNodeWithText("Add a synth").performClick()
        compose.onNodeWithText("minilogue xd").performClick()
        compose.waitUntil(5_000) { compose.onAllNodesWithContentDescription("Disconnect", substring = true).fetchSemanticsNodes().isNotEmpty() }
        compose.activityRule.scenario.recreate()
        // Still the one demo synth: the model, and its worker, were not made again.
        compose.waitForIdle()
        assertEquals(1, compose.onAllNodesWithContentDescription("Disconnect", substring = true).fetchSemanticsNodes().size)
    }
}
