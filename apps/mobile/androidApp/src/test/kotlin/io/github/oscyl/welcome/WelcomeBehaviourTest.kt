package io.github.oscyl.welcome

import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onAllNodesWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.core.Role as Answer
import io.github.oscyl.platform.MidiPlatform
import io.github.oscyl.platform.NoHaptics
import io.github.oscyl.platform.NoMidi
import io.github.oscyl.platform.OscylHaptics
import io.github.oscyl.platform.Platform
import io.github.oscyl.ui.preferences.PreferencesModel
import io.github.oscyl.ui.shell.App
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The first run's welcome, over a settings file in a temporary folder. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class WelcomeBehaviourTest {
    @get:Rule val compose = createComposeRule()

    @get:Rule val temp = TemporaryFolder()

    private val tabs get() = compose.onAllNodes(SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.Tab))

    private fun platform(path: String) = object : Platform {
        override val haptics: OscylHaptics = NoHaptics
        override fun midi(): MidiPlatform = NoMidi
        override val settingsPath = path
        override val librariesDir = File(path).resolveSibling("libraries").path
    }

    @Test
    fun aDemoFromThePickerStartsInTheLibraryAndTheFirstRunShowsOnce() {
        val path = temp.root.resolve("settings.json").path
        compose.setContent { App(platform(path)) }
        compose.onNodeWithText("No synth here? Try one").performScrollTo().performClick()
        compose.onNodeWithText("prologue").performClick()
        // The Library first, the simulated synth in the chip.
        compose.waitUntil(TIMEOUT_MS) { tabs.fetchSemanticsNodes().size == 3 }
        tabs[0].assertIsSelected()
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodesWithContentDescription("Synths: prologue").fetchSemanticsNodes().isNotEmpty() }
        compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle { PreferencesModel(path).prefs.value.onboarded } }
        // The demo library is kept among the libraries.
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodesWithText("Demo library").fetchSemanticsNodes().isNotEmpty() }
        val kept = PreferencesModel(path).prefs.value
        assertEquals(null, kept.role)
        assertEquals("korg-prologue", kept.demo)
        assertTrue(kept.onboarded)
    }

    @Test
    fun skippingStartsWithNothing() {
        val path = temp.root.resolve("settings.json").path
        compose.setContent { App(platform(path)) }
        compose.onNodeWithText("Continue without a synth").performScrollTo().performClick()
        compose.waitUntil(TIMEOUT_MS) { tabs.fetchSemanticsNodes().size == 3 }
        compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle { PreferencesModel(path).prefs.value.onboarded } }
        compose.onNodeWithContentDescription("Synths: none connected").assertExists()
    }

    @Test
    fun aLaterRunStartsWhereTheAnswerSaid() {
        val path = temp.root.resolve("settings.json").path
        val first = PreferencesModel(path)
        first.finishWelcome(Answer.ORGANISE, null)
        compose.waitUntil(TIMEOUT_MS) { PreferencesModel(path).prefs.value.onboarded }
        compose.setContent { App(platform(path)) }
        tabs[0].assertIsSelected()
        compose.onNodeWithText("Continue without a synth").assertDoesNotExist()
    }

    private companion object {
        const val TIMEOUT_MS = 30_000L
    }
}
