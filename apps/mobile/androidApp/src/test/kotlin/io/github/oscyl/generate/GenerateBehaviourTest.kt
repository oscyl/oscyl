package io.github.oscyl.generate

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.viewmodel.CreationExtras
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.core.Generator
import io.github.oscyl.core.Libraries
import io.github.oscyl.core.setLocale
import io.github.oscyl.platform.NoMidi
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.generate.GenerateModel
import io.github.oscyl.ui.generate.GenerateScreen
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.synth.SynthModel
import kotlin.reflect.KClass
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** Generate over the real core, in temporary folders. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class GenerateBehaviourTest {
    @get:Rule val compose = createComposeRule()

    @get:Rule val temp = TemporaryFolder()

    private val store = ViewModelStore()

    @After
    fun clear() = store.clear()

    private fun <T : ViewModel> make(kind: KClass<T>, new: () -> T): T =
        ViewModelProvider.create(
            store,
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <V : ViewModel> create(modelClass: KClass<V>, extras: CreationExtras): V = new() as V
            },
        )[kind]

    private fun await(condition: () -> Boolean) = compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle(condition) }

    @Test
    fun theSameSeedDealsTheSameSounds() {
        fun dealt(seed: ULong) = Generator(XD, seed).use { g ->
            g.deal(1u, listOf(0.2f, 0.8f, 0.5f, 0.1f), emptyList(), null)
            (0 until 6).map { g.select(it.toUInt()); g.selectedProgram()!! }
        }
        val a = dealt(7u)
        val b = dealt(7u)
        a.zip(b).forEach { (x, y) -> assertArrayEquals(x, y) }
        assertEquals(6, a.map { it.toList() }.distinct().size)
    }

    @Test
    fun aKeptSoundIsAddedToALibrary() {
        val synth = make(SynthModel::class) { SynthModel(NoMidi) }
        val dir = temp.newFolder("libraries").path
        val library = make(LibraryModel::class) { LibraryModel(dir, synth) }
        val generate = make(GenerateModel::class) { GenerateModel(library, synth, seed = 3) }
        compose.setContent { OscylTheme(OscylThemeMode.Dark) { GenerateScreen(generate) } }
        compose.onNode(hasText("Generate") and hasClickAction()).performClick()
        await { generate.ui.value.hand != null }
        // Keep, docked beside Generate: with no slot open it goes straight to a library.
        compose.onNode(hasText("Keep") and hasClickAction()).performClick()
        await { generate.ui.value.hand!!.sounds.any { it.kept != null } }
        val kept = generate.ui.value.hand!!.sounds.single { it.kept != null }
        await { library.ui.value.lines.size == 1 }
        val line = library.ui.value.lines.single()
        assertEquals(line.name, kept.kept)
        assertNotNull(compose.onNodeWithText("kept in ${line.name}", substring = true).fetchSemanticsNode())
        // A second sound goes into the same library, not a new one.
        val other = generate.ui.value.hand!!.sounds.indexOfFirst { it.kept == null }
        compose.runOnIdle { generate.select(other) }
        await { generate.ui.value.hand!!.selected == other.toUInt() }
        compose.runOnIdle { generate.keepInLibrary() }
        // A hand marks the sound it kept last.
        await { generate.ui.value.hand!!.sounds[other].kept != null }
        assertEquals(1, library.ui.value.lines.size)
        Libraries.open(dir).use { l -> l.read(line.id).use { assertEquals(2u, it.len()) } }
    }

    @Test
    fun aLanguageChangeAsksTheCoreForItsWordsAgain() {
        val synth = make(SynthModel::class) { SynthModel(NoMidi) }
        val library = make(LibraryModel::class) { LibraryModel(temp.newFolder("libraries").path, synth) }
        val generate = make(GenerateModel::class) { GenerateModel(library, synth, seed = 3) }
        compose.setContent {}
        await { generate.ui.value.pages.isNotEmpty() }
        val english = generate.ui.value.pages
        try {
            setLocale("it")
            compose.runOnIdle { generate.relocalise() }
            await { generate.ui.value.pages != english }
        } finally {
            setLocale("en")
        }
    }

    private companion object {
        const val TIMEOUT_MS = 30_000L
        const val XD = "korg-minilogue-xd"
    }
}
