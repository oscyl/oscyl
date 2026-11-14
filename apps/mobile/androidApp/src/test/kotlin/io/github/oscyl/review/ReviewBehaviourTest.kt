package io.github.oscyl.review

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import io.github.oscyl.ui.parts.DockedSheet
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.CreationExtras
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.core.ControlKind
import io.github.oscyl.core.Libraries
import io.github.oscyl.platform.NoMidi
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.editor.EditorModel
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.library.librarySource
import io.github.oscyl.ui.library.synthSource
import io.github.oscyl.ui.review.ReviewModel
import io.github.oscyl.ui.review.ReviewSheet
import io.github.oscyl.ui.review.ReviewState
import io.github.oscyl.ui.synth.SynthModel
import java.io.File
import kotlin.reflect.KClass
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The review sheet over the real core (host bindings): Save a library, Sync the demo synth, failures keep edits. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class ReviewBehaviourTest {
    @get:Rule val compose = createComposeRule()

    @get:Rule val temp = TemporaryFolder()

    private val store = ViewModelStore()

    @After
    fun clear() = store.clear()

    private var synthsOpen by mutableStateOf(false)

    private lateinit var library: LibraryModel
    private lateinit var synth: SynthModel
    private lateinit var editor: EditorModel
    private lateinit var review: ReviewModel
    private lateinit var dir: String
    private lateinit var gig: String

    private fun <T : ViewModel> make(kind: KClass<T>, new: () -> T): T =
        ViewModelProvider.create(
            store,
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <V : ViewModel> create(modelClass: KClass<V>, extras: CreationExtras): V = new() as V
            },
        )[kind]

    private fun show(backups: String) {
        dir = temp.newFolder("libraries").path
        gig = Libraries.open(dir).use { l -> l.create("Gig", XD, null).also { id -> l.read(id).use { d -> d.newProgram(null); l.save(id, d) } } }
        synth = make(SynthModel::class) { SynthModel(NoMidi) }
        library = make(LibraryModel::class) { LibraryModel(dir, synth) }
        editor = make(EditorModel::class) { EditorModel(library, synth) }
        review = make(ReviewModel::class) { ReviewModel(library, synth, backups) }
        compose.setContent {
            OscylTheme(OscylThemeMode.Dark) {
                val ui by review.ui.collectAsStateWithLifecycle()
                Box(Modifier.fillMaxSize()) {
                    DockedSheet(synthsOpen, onClose = { synthsOpen = false }, title = "Synths") { BasicText("Rack") }
                    ReviewSheet(ui, review)
                }
            }
        }
    }

    private fun await(condition: () -> Boolean) = compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle(condition) }

    /** Turn the first dial of the open program, as a finished gesture. */
    private fun editADial(): UInt {
        await { editor.panel != null }
        val f = compose.runOnIdle {
            val panel = editor.panel!!
            val dial = panel.pages.indices.flatMap { p -> panel.pages[p].sections.flatMap { it.fields } }
                .mapNotNull { panel.field(it) }.first { it.relevant && it.control is ControlKind.Dial }
            val max = (dial.control as ControlKind.Dial).max
            editor.set(dial.field, if (dial.raw == max) 0 else max.toInt())
            editor.release()
            dial.field
        }
        await { editor.head.value?.saveCount == 1 }
        return f
    }

    private fun openLibraryProgram() {
        await { library.ui.value.lines.any { it.id == gig } }
        compose.runOnIdle { library.select(librarySource(gig)) }
        await { library.ui.value.programs?.rows?.isNotEmpty() == true }
        compose.runOnIdle { library.selectProgram(0) }
    }

    @Test
    fun saveClearsTheCountAndPersists() {
        show(temp.newFolder("backups").path)
        openLibraryProgram()
        val f = editADial()
        compose.runOnIdle { review.open() }
        compose.onNodeWithContentDescription("Save 1 edited program").performClick()
        await { editor.head.value?.saveCount == 0 && !review.ui.value.open }
        val kept = editor.panel!!.field(f)!!.raw
        // On disk, and nothing shows as edited.
        val saved = Libraries.open(dir).use { l -> l.read(l.outline().first().id).use { d -> d.editor(0u).use { it.field(f).raw } } }
        assertEquals(kept, saved)
        assertEquals(false, editor.panel!!.field(f)!!.edited)
    }

    @Test
    fun theSaveKeyKeepsALibraryInOneTap() {
        show(temp.newFolder("backups").path)
        openLibraryProgram()
        val f = editADial()
        compose.runOnIdle { review.quickSave() }
        await { editor.head.value?.saveCount == 0 }
        // No sheet: the notice says it.
        assertEquals(false, review.ui.value.open)
        assertEquals(io.github.oscyl.ui.library.Notice.Saved(1), library.ui.value.notice)
        val saved = Libraries.open(dir).use { l -> l.read(l.outline().first().id).use { d -> d.editor(0u).use { it.field(f).raw } } }
        assertEquals(editor.panel!!.field(f)!!.raw, saved)
    }

    @Test
    fun syncWritesTheDemoAndMarksItSynced() {
        val backups = temp.newFolder("backups")
        show(backups.path)
        compose.runOnIdle { synth.openDemo(XD) }
        await { synth.cards.value.any { it.connected } }
        val id = synth.cards.value.first().id
        compose.runOnIdle { library.select(synthSource(id)) }
        // Every slot read, then the first opened.
        await { library.ui.value.programs?.let { p -> p.read == null && p.rows.all { it.loaded } } == true }
        compose.runOnIdle { library.selectProgram(0) }
        editADial()
        compose.runOnIdle { review.open() }
        compose.onNodeWithText("Only these slots change. Oscyl backs up what they hold now first.").assertExists()
        compose.onNodeWithContentDescription("Sync 1 edited program").performClick()
        await { review.ui.value.state == ReviewState.Idle && editor.head.value?.saveCount == 0 }
        assertTrue("a backup was taken first", backups.listFiles().orEmpty().any { it.name.startsWith("Synth backup") })
    }

    @Test
    fun aFailedBackupWritesNothingAndKeepsTheEdits() {
        // The backups folder cannot be made: a file stands in its way.
        val blocked = File(temp.root, "blocked").apply { writeText("") }
        show(File(blocked, "backups").path)
        compose.runOnIdle { synth.openDemo(XD) }
        await { synth.cards.value.any { it.connected } }
        val id = synth.cards.value.first().id
        compose.runOnIdle { library.select(synthSource(id)) }
        await { library.ui.value.programs?.let { p -> p.read == null && p.rows.all { it.loaded } } == true }
        compose.runOnIdle { library.selectProgram(0) }
        editADial()
        compose.runOnIdle {
            review.open()
            review.commit()
        }
        await { review.ui.value.state is ReviewState.BackupFailed }
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodesWithText("Couldn’t back up the synth", substring = true).fetchSemanticsNodes().isNotEmpty() }
        assertEquals(1, editor.head.value!!.saveCount)
        assertTrue(review.ui.value.open)
    }

    /** Review asked for from the synth sheet: that one lands before this one rises, never two panes. */
    @Test
    fun reviewFromSynthsShowsOneSheet() {
        show(temp.newFolder("backups").path)
        openLibraryProgram()
        editADial()
        compose.runOnIdle { synthsOpen = true }
        compose.waitForIdle()
        val panes = SemanticsMatcher.keyIsDefined(SemanticsProperties.PaneTitle)
        assertEquals(1, compose.onAllNodes(panes, useUnmergedTree = true).fetchSemanticsNodes().size)
        compose.mainClock.autoAdvance = false
        compose.runOnIdle {
            synthsOpen = false
            review.open()
        }
        repeat(40) {
            compose.mainClock.advanceTimeByFrame()
            assertTrue(compose.onAllNodes(panes, useUnmergedTree = true).fetchSemanticsNodes().size <= 1)
        }
        compose.mainClock.autoAdvance = true
        compose.waitForIdle()
        val titles = compose.onAllNodes(panes, useUnmergedTree = true).fetchSemanticsNodes().map { it.config[SemanticsProperties.PaneTitle] }
        assertEquals(listOf("1 edited program"), titles)
    }

    private companion object {
        const val TIMEOUT_MS = 30_000L
        const val XD = "korg-minilogue-xd"
    }
}
