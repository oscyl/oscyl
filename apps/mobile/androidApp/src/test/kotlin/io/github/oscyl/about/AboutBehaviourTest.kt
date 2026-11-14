package io.github.oscyl.about

import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollToIndex
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.test.hasScrollAction
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.ui.preview.PreviewPlatform
import io.github.oscyl.ui.shell.App
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The menu → About Oscyl → a notice's licence → Back, over the files the app ships. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class AboutBehaviourTest {
    @get:Rule val compose = createComposeRule()

    private fun await(text: String, substring: Boolean = false) = compose.waitUntil(10_000) {
        compose.onAllNodesWithText(text, substring = substring).fetchSemanticsNodes().isNotEmpty()
    }

    @Test
    fun aboutListsNoticesAndOpensALicence() {
        compose.setContent { App(PreviewPlatform) }
        compose.onNodeWithContentDescription("More options").performClick()
        compose.onNodeWithText("About Oscyl").performClick()
        await("Albert Sans")
        compose.onNode(hasScrollAction()).performScrollToNode(hasText("uniffi_core"))
        compose.onNodeWithText("uniffi_core").performClick()
        await("Mozilla Public License", substring = true)
        compose.onNodeWithContentDescription("Back").performClick()
        // Back on About, where it was left (its scroll kept).
        await("uniffi_core")
        compose.onNode(hasScrollAction()).performScrollToIndex(0)
        compose.onNodeWithContentDescription("Back").performClick()
        await("No synth connected.")
    }

    @Test
    fun preloadedAboutIsWholeOnItsFirstFrame() {
        kotlinx.coroutines.runBlocking { io.github.oscyl.ui.about.preloadLicences() }
        compose.mainClock.autoAdvance = false
        compose.setContent { io.github.oscyl.ui.about.AboutScreen(onBack = {}, onOpen = {}) }
        // No frame has run past the first composition: the notices are there already.
        compose.onNodeWithText("Albert Sans").assertExists()
    }
}
