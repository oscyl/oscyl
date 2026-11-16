package io.github.oscyl.library

import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.runtime.getValue
import io.github.oscyl.ui.editor.EditorModel
import io.github.oscyl.core.ControlKind
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.longClick
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextReplacement
import androidx.compose.ui.test.performTouchInput
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.lifecycle.viewModelScope
import io.github.oscyl.core.Libraries
import io.github.oscyl.core.ProgramAction
import io.github.oscyl.platform.NoMidi
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.library.LibraryScreen
import io.github.oscyl.ui.library.Notice
import io.github.oscyl.ui.library.librarySource
import io.github.oscyl.ui.library.synthSource
import io.github.oscyl.ui.synth.SynthModel
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The Library destination over the real core (host bindings), in a temporary libraries folder. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class LibraryBehaviourTest {
    @get:Rule val compose = createComposeRule()

    @get:Rule val temp = TemporaryFolder()

    private val store = ViewModelStore()

    @After
    fun clear() = store.clear()

    private fun show(synth: SynthModel? = null): LibraryModel {
        val dir = temp.newFolder("libraries").path
        val model = ViewModelProvider.create(store, viewModelFactory { initializer { LibraryModel(dir, synth) } })[LibraryModel::class]
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                androidx.compose.foundation.layout.Box {
                    LibraryScreen(model, onOpenSynth = {})
                    val ui by model.ui.collectAsStateWithLifecycle()
                    io.github.oscyl.ui.library.NoticeHost(ui, model)
                }
            }
        }
        return model
    }

    private fun await(condition: () -> Boolean) = compose.waitUntil(TIMEOUT_MS, condition)

    private fun awaitText(text: String) = await { compose.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty() }

    /** Waits for [condition] on the model or the folder, letting the app's coroutines run between looks. */
    private fun awaitIdle(condition: () -> Boolean) = compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle(condition) }

    /** Library [id]'s program names as the folder holds them (what was saved). */
    private fun saved(id: String) = Libraries.open(temp.root.resolve("libraries").path).use { l ->
        l.read(id).use { doc -> doc.rows().map { it.name } }
    }

    private fun names(model: LibraryModel) = model.ui.value.programs!!.rows.map { it.name }

    @Test
    fun createRenameReorderAndUndo() {
        val model = show()
        newLibrary()
        compose.onNodeWithText("Library 1").performTouchInput { longClick() }
        compose.onNodeWithText("Rename").performClick()
        compose.onNodeWithContentDescription("Name").performTextReplacement("Gig set")
        compose.onNodeWithText("Rename").performClick()
        awaitText("Gig set")
        compose.onNodeWithText("Gig set").performClick()
        awaitText("No programs yet")
        newProgram()
        newProgram()
        model.renameProgram(0, "Lead")
        model.renameProgram(1, "Pad")
        compose.waitForIdle()
        assertEquals(listOf("Lead", "Pad"), names(model))
        model.act(0, ProgramAction.MOVE_DOWN)
        compose.waitForIdle()
        assertEquals(listOf("Pad", "Lead"), names(model))
        // Undo heads the menu, in the step's own words.
        compose.onNodeWithContentDescription(MENU).performClick()
        compose.onNodeWithText("Undo:", substring = true).performClick()
        compose.waitForIdle()
        assertEquals(listOf("Lead", "Pad"), names(model))
        // Saved after every change: the folder reads back the same.
        val id = model.ui.value.lines.single().id
        awaitIdle { saved(id) == listOf("Lead", "Pad") }
    }

    /** New library: it asks for its name at once; the name it was given is kept. */
    private fun newLibrary() {
        compose.onNodeWithText("New library").performClick()
        awaitText("Name the new library")
        compose.onNodeWithText("Done").performClick()
        await { compose.onAllNodesWithText("Name the new library").fetchSemanticsNodes().isEmpty() }
        awaitText("Library 1")
    }

    /** New program, from the list's menu (where it lives besides the empty state). */
    private fun newProgram() {
        // Robolectric hands the first tap after a focusable popup closed to the window's
        // focus, not the button: tap until the menu is up.
        await {
            compose.onNodeWithContentDescription(MENU).performClick()
            compose.onAllNodesWithTag("program-new").fetchSemanticsNodes().isNotEmpty()
        }
        compose.onNodeWithTag("program-new").performClick()
        // The menu gone (it fades out), so the next tap lands on the list, not on its outside.
        await { compose.onAllNodesWithTag("program-new").fetchSemanticsNodes().isEmpty() }
    }

    /** A new library with one program in it, shown. */
    private fun oneProgram(model: LibraryModel) {
        newLibrary()
        compose.onNodeWithText("Library 1").performClick()
        awaitText("No programs yet")
        newProgram()
        awaitIdle { model.ui.value.programs?.rows?.size == 1 }
    }

    @Test
    fun aNewProgramsNoticeOpensIt() {
        val model = show()
        oneProgram(model)
        await { model.ui.value.notice is Notice.Added }
        assertEquals(null, model.open.value)
        compose.runOnIdle { model.openNotice() }
        await { model.open.value?.index == 0 }
        assertEquals(null, model.ui.value.notice)
    }

    @Test
    fun rowShowsThumbnailWhenLoaded() {
        val model = show()
        oneProgram(model)
        await { compose.onAllNodesWithTag("program-thumbnail", useUnmergedTree = true).fetchSemanticsNodes().size == 1 }
    }

    @Test
    fun editInvalidatesThumbnail() {
        val synth = ViewModelProvider.create(store, viewModelFactory { initializer { SynthModel(NoMidi) } })[SynthModel::class]
        val model = show(synth)
        val editor = ViewModelProvider.create(store, viewModelFactory { initializer { EditorModel(model, synth) } })[EditorModel::class]
        oneProgram(model)
        compose.runOnIdle { model.selectProgram(0) }
        awaitIdle { editor.panel != null }
        val before = compose.runOnIdle { model.thumbnail(0) }
        val revision = compose.runOnIdle { model.ui.value.programs!!.revision }
        compose.runOnIdle {
            // Turn a dial on the program's first picture's page: the picture moves.
            val panel = editor.panel!!
            val page = panel.pages.indices.first { panel.graph(it)?.now != null }
            val dial = panel.pages[page].sections.flatMap { it.fields }.mapNotNull { panel.field(it) }
                .first { it.relevant && it.control is ControlKind.Dial && panel.graph(page)?.now != null }
            val max = (dial.control as ControlKind.Dial).max
            editor.set(dial.field, if (dial.raw == max) 0 else max.toInt())
            editor.release()
        }
        awaitIdle { model.ui.value.programs!!.revision != revision }
        compose.runOnIdle { assertTrue("the picture changed", model.thumbnail(0) != before) }
    }

    /** A library of one program, every dial at its bottom ([top] false) or its top. */
    private fun libraryWith(dir: String, name: String, top: Boolean): String = Libraries.open(dir).use { l ->
        l.create(name, "korg-minilogue-xd", null).also { id ->
            l.read(id).use { d ->
                d.newProgram(null)
                d.editor(0u).use { e ->
                    e.pages().indices.flatMap { p -> e.fields(p.toUInt()) }.forEach { f ->
                        (f.control as? io.github.oscyl.core.ControlKind.Dial)?.let { e.set(f.field, if (top) it.max else it.min) }
                    }
                    e.seal()
                }
                l.save(id, d)
            }
        }
    }

    @Test
    fun picturesNeverCrossFromOneLibraryToAnother() {
        val model = show()
        val dir = temp.root.resolve("libraries").path
        val (a, b) = libraryWith(dir, "A", top = false) to libraryWith(dir, "B", top = true)
        compose.runOnIdle { model.select(librarySource(a)) }
        awaitIdle { model.ui.value.programs?.source == librarySource(a) && model.thumbnail(0) != null }
        val first = compose.runOnIdle { model.thumbnail(0) }
        compose.runOnIdle { model.select(librarySource(b)) }
        awaitIdle { model.ui.value.programs?.source == librarySource(b) }
        // Both unedited, both at revision 0: the cache must still tell them apart.
        compose.runOnIdle { assertTrue("B shows its own picture", model.thumbnail(0) != first) }
    }

    @Test
    fun aSynthConnectingLetsALibrarySwipe() {
        val synth = ViewModelProvider.create(store, viewModelFactory { initializer { SynthModel(NoMidi) } })[SynthModel::class]
        val model = show(synth)
        oneProgram(model)
        compose.runOnIdle { assertTrue(!model.ui.value.programs!!.canAudition) }
        synth.openDemo("korg-minilogue-xd")
        awaitIdle { model.ui.value.programs?.canAudition == true }
    }

    @Test
    fun deleteOffersUndoThatRestores() {
        val model = show()
        newLibrary()
        compose.onNodeWithText("Library 1").performTouchInput { longClick() }
        compose.onNodeWithText("Delete").performClick()
        awaitText("Deleted “Library 1”")
        assertTrue(model.ui.value.lines.isEmpty())
        compose.onNodeWithText("Undo").performClick()
        awaitText("Library 1")
        assertEquals(listOf("Library 1"), model.ui.value.lines.map { it.name })
        assertEquals(null, model.ui.value.notice)
    }

    @Test
    fun removedProgramComesBackWithUndo() {
        val model = show()
        newLibrary()
        compose.onNodeWithText("Library 1").performClick()
        awaitText("No programs yet")
        newProgram()
        model.renameProgram(0, "Keep")
        compose.waitForIdle()
        compose.onNodeWithText("Keep").performTouchInput { longClick() }
        // The menu opens once the core has answered, off the main thread.
        awaitText("Remove from library")
        compose.onNodeWithText("Remove from library").performClick()
        awaitText("Removed “Keep”")
        assertTrue(names(model).isEmpty())
        compose.onNodeWithText("Undo").performClick()
        awaitText("Keep")
        assertEquals(listOf("Keep"), names(model))
    }

    @Test
    fun aSecondReadNeverReplacesTheLibraryBeingEdited() {
        val model = show()
        newLibrary()
        val id = model.ui.value.lines.single().id
        // Edit the moment the first read lands, while the second is still on its way.
        model.viewModelScope.launch {
            model.ui.first { it.programs?.source == librarySource(id) }
            model.newProgram()
            model.renameProgram(0, "Kept")
        }
        model.select(librarySource(id))
        model.select(null)
        model.select(librarySource(id))
        awaitIdle { model.ui.value.programs?.rows?.map { it.name } == listOf("Kept") }
        compose.waitForIdle()
        assertEquals(listOf("Kept"), names(model))
        awaitIdle { saved(id) == listOf("Kept") }
    }

    @Test
    fun anUnpluggedSynthLeavesItsEditsInALibrary() {
        val synth = ViewModelProvider.create(store, viewModelFactory { initializer { SynthModel(NoMidi) } })[SynthModel::class]
        val model = show(synth)
        synth.openDemo("korg-minilogue-xd")
        awaitText("minilogue xd")
        val card = model.ui.value.synths.single()
        model.select(synthSource(card.id))
        awaitIdle { model.ui.value.programs != null }
        model.renameProgram(2, "Kept")
        synth.disconnect(card)
        awaitText("minilogue xd (disconnected)")
        val kept = model.ui.value.lines.single { it.name == "minilogue xd (disconnected)" }
        assertEquals("Kept", saved(kept.id)[2])
    }

    /** A USB synth unplugged stays listed with its programs and edits, and no library is made of them. */
    @Test
    fun anUnpluggedUsbSynthStaysListedWithItsEdits() {
        val port = io.github.oscyl.platform.MidiPortInfo(name = "minilogue xd _ SOUND", id = "7")
        val midi = io.github.oscyl.synth.FakeMidi(port)
        val synth = ViewModelProvider.create(store, viewModelFactory { initializer { SynthModel(midi) } })[SynthModel::class]
        val model = show(synth)
        awaitIdle { model.ui.value.synths.size == 1 }
        val card = model.ui.value.synths.single()
        model.select(synthSource(card.id))
        awaitIdle { model.ui.value.programs?.rows?.getOrNull(2)?.loaded == true }
        model.renameProgram(2, "Kept")
        midi.unplug(port)
        awaitIdle { model.ui.value.synths.singleOrNull()?.connected == false }
        assertEquals(card.id, model.ui.value.synths.single().id)
        assertEquals(synthSource(card.id), model.ui.value.programs?.source)
        assertEquals("Kept", names(model)[2])
        assertTrue(model.ui.value.lines.none { it.name.endsWith("(disconnected)") })
    }

    @Test
    fun factorySlotsRefuseEdits() {
        val synth = ViewModelProvider.create(store, viewModelFactory { initializer { SynthModel(NoMidi) } })[SynthModel::class]
        val model = show(synth)
        synth.openDemo("sequential-prophet-rev2")
        awaitText("Prophet Rev2")
        compose.onNodeWithText("Prophet Rev2").performClick()
        await { model.ui.value.programs?.source == synthSource(model.ui.value.synths.single().id) }
        val factory = model.ui.value.programs!!.rows.indexOfFirst { it.readOnly }
        assertTrue(factory > 0)
        // Read first: a name that arrives later is not an edit.
        // The factory banks come after the user banks: on a slow CI runner that read
        // outlasts the usual bound.
        compose.waitUntil(READ_MS) { model.ui.value.programs!!.rows[factory].loaded }
        val before = model.ui.value.programs!!.rows[factory].name
        model.renameProgram(factory, "Mine")
        compose.waitForIdle()
        assertTrue(model.ui.value.notice is Notice.Failed)
        assertEquals(before, model.ui.value.programs!!.rows[factory].name)
        val enabled = kotlinx.coroutines.runBlocking { model.menu(factory) }!!.sections.flatten().filter { it.enabled }.map { it.action }
        // Copied (and, once read, copied to a library), never changed.
        assertTrue(ProgramAction.COPY in enabled)
        assertTrue((enabled - setOf(ProgramAction.COPY, ProgramAction.COPY_TO_LIBRARY)).isEmpty())
    }

    private companion object {
        const val MENU = "More options"
        const val TIMEOUT_MS = 10_000L
        const val READ_MS = 60_000L
    }
}
