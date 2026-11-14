package io.github.oscyl.preferences

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.core.Theme
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.preferences.PreferencesModel
import io.github.oscyl.ui.preferences.PreferencesScreen
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** Preferences over the core's settings file, in a temporary folder. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class PreferencesBehaviourTest {
    @get:Rule val compose = createComposeRule()

    @get:Rule val temp = TemporaryFolder()

    @Test
    fun switchesPersistAcrossANewProcess() {
        val path = temp.root.resolve("settings.json").path
        val model = PreferencesModel(path)
        compose.setContent { OscylTheme(OscylThemeMode.Dark) { PreferencesScreen(model, emptyList(), onBack = {}) } }
        compose.onNodeWithText("AMOLED black").performClick()
        compose.runOnIdle {
            model.setMotion(false)
            model.setHaptics(false)
            model.setDemo("korg-prologue")
        }
        compose.runOnIdle { assertEquals(Theme.AMOLED, model.prefs.value.theme) }
        // What a new process reads (the writes are queued in order; wait for the last).
        compose.waitUntil(10_000) {
            compose.runOnIdle { PreferencesModel(path).prefs.value.demo == "korg-prologue" }
        }
        val again = PreferencesModel(path).prefs.value
        assertEquals(Theme.AMOLED, again.theme)
        assertEquals(false, again.motion)
        assertEquals(false, again.haptics)
    }
}
