package io.github.oscyl.shell

import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.core.setLocale
import io.github.oscyl.platform.MidiPlatform
import io.github.oscyl.platform.NoHaptics
import io.github.oscyl.platform.NoMidi
import io.github.oscyl.platform.OscylHaptics
import io.github.oscyl.platform.Platform
import io.github.oscyl.ui.shell.App
import org.junit.After
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/**
 * The screens the app keeps across a configuration change ask the core for their words
 * again when the language changed, and only then (a rotation keeps them as they are).
 */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class LanguageChangeTest {
    @get:Rule val compose = createComposeRule()

    @After
    fun english() = setLocale("en")

    private class Device(var lang: String) : Platform {
        override val haptics: OscylHaptics = NoHaptics
        override fun midi(): MidiPlatform = NoMidi
        override val language: String get() = lang
    }

    private fun shows(text: String) = compose.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty()

    @Test
    fun retainedScreensFollowALanguageChangeNotARotation() {
        val device = Device("en")
        val restoration = StateRestorationTester(compose)
        restoration.setContent { App(device) }
        compose.onNode(SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.Tab) and hasText("Generate")).performClick()
        compose.waitUntil(10_000) { shows("Fuzz Bass") }
        // A rotation: the same language, the same words.
        restoration.emulateSavedInstanceStateRestore()
        compose.waitUntil(10_000) { shows("Fuzz Bass") }
        // The language changes while the models are kept: Generate's characters follow.
        device.lang = "it"
        restoration.emulateSavedInstanceStateRestore()
        compose.waitUntil(10_000) { shows("Basso fuzz") }
    }
}
