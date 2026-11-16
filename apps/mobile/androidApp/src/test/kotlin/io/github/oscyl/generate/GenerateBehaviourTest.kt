package io.github.oscyl.generate

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onNodeWithTag
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
import io.github.oscyl.platform.Files
import io.github.oscyl.platform.NoFiles
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
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
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

    @Test
    fun familiesSurpriseCodesAndMorph() {
        val shared = mutableListOf<String>()
        val files = object : Files by NoFiles {
            override fun shareText(text: String) {
                shared += text
            }
        }
        val synth = make(SynthModel::class) { SynthModel(NoMidi) }
        val library = make(LibraryModel::class) { LibraryModel(temp.newFolder("libraries").path, synth) }
        val generate = make(GenerateModel::class) { GenerateModel(library, synth, seed = 3, files = files) }
        compose.setContent { OscylTheme(OscylThemeMode.Dark) { GenerateScreen(generate) } }
        await { generate.ui.value.characters.isNotEmpty() }
        // The page opens on the first character's family: a handful of tiles, not all of them.
        assertEquals(generate.ui.value.characters[0].family, generate.ui.value.family)
        compose.onNodeWithText("Wobble").assertDoesNotExist()
        // Another family shows its own characters and takes the choice along.
        compose.onNode(hasText("Bass") and hasClickAction()).performClick()
        await { generate.ui.value.family == "bass" }
        generate.ui.value.let { assertEquals("bass", it.characters[it.character].family) }
        assertNotNull(compose.onNodeWithText("Wobble").fetchSemanticsNode())

        // Surprise deals, and the page shows what was picked: its character (out of a family that hides it) and dials.
        compose.onNodeWithTag("generate-surprise").performScrollTo().performClick()
        await { generate.ui.value.hand != null && generate.ui.value.code != null }
        val ui = generate.ui.value
        assertTrue(ui.family == null || ui.characters[ui.character].family == ui.family)
        assertTrue(ui.values.all { it % 5 == 0 })
        // The picked sound's code is shown, and shared as text.
        val code = ui.code!!
        assertNotNull(compose.onNodeWithText(code).fetchSemanticsNode())
        compose.runOnIdle { generate.shareCode() }
        assertEquals(listOf(code), shared)

        // A bad code stays in its field with the reason under it; nothing is dealt.
        compose.onNodeWithTag("generate-enter-code").performScrollTo().performClick()
        await { generate.ui.value.entering }
        val count = generate.ui.value.hand!!.count
        compose.runOnIdle { generate.dealCode("pad-nonsense") }
        await { generate.ui.value.codeError != null }
        assertNotNull(compose.onNodeWithText(generate.ui.value.codeError!!).fetchSemanticsNode())
        assertEquals(count, generate.ui.value.hand!!.count)
        // The good one deals its sound first and closes the field.
        compose.runOnIdle { generate.dealCode(code.uppercase()) }
        await { !generate.ui.value.entering && generate.ui.value.hand!!.count == count + 1u }
        await { generate.ui.value.code == code }

        // Morph: six steps between two sounds of the hand, named in the legend, with no code.
        compose.runOnIdle { generate.morph(3) }
        await { generate.ui.value.hand!!.origin.startsWith("From") }
        assertEquals(6, generate.ui.value.hand!!.sounds.size)
        assertNull(generate.ui.value.code)
    }

    private companion object {
        const val TIMEOUT_MS = 30_000L
        const val XD = "korg-minilogue-xd"
    }
}
