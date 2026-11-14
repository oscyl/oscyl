package io.github.oscyl.library

import androidx.activity.ComponentActivity
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.library.LibraryScreen
import io.github.oscyl.ui.shell.LocalDestinationShown
import org.junit.After
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/**
 * The Library stays composed, hidden, under the other destinations and the pages: Back is
 * then never its to take. (On a phone it took the Back meant for the play page and closed
 * the program open in the editor.)
 */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class LibraryBackTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @get:Rule val temp = TemporaryFolder()

    private val store = ViewModelStore()

    @After
    fun clear() = store.clear()

    private var inSight by mutableStateOf(true)

    @Test
    fun aHiddenLibraryLeavesBackAlone() {
        val dir = temp.newFolder("libraries").path
        val model = ViewModelProvider.create(store, viewModelFactory { initializer { LibraryModel(dir, null) } })[LibraryModel::class]
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                CompositionLocalProvider(LocalDestinationShown provides inSight) { LibraryScreen(model, onOpenSynth = {}) }
            }
        }
        compose.onNodeWithText("New library").performClick()
        // It asks for a name at once: the one it was given is kept.
        compose.waitUntil(10_000) { compose.onAllNodesWithText("Name the new library").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Done").performClick()
        compose.waitUntil(10_000) { compose.onAllNodesWithText("Name the new library").fetchSemanticsNodes().isEmpty() }
        compose.waitUntil(10_000) { compose.onAllNodesWithText("Library 1").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Library 1").performClick()
        compose.waitUntil(10_000) { compose.runOnIdle { model.ui.value.programs != null } }
        // Hidden (another destination, or a page above): Back goes elsewhere, the list stays.
        inSight = false
        compose.waitForIdle()
        compose.runOnIdle { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.waitForIdle()
        compose.runOnIdle { assert(model.ui.value.programs != null) { "a hidden Library took Back" } }
        // In sight, Back returns from the programs to the sources.
        inSight = true
        compose.waitForIdle()
        compose.runOnIdle { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.waitUntil(10_000) { compose.runOnIdle { model.ui.value.programs == null } }
    }
}
