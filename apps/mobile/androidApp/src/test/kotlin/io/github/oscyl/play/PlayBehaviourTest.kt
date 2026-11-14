package io.github.oscyl.play

import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTouchInput
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
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
import io.github.oscyl.ui.play.PlayModel
import io.github.oscyl.ui.play.PlayScreen
import io.github.oscyl.ui.play.PlaySlot
import io.github.oscyl.ui.preferences.PreferencesModel
import io.github.oscyl.ui.synth.SynthModel
import java.io.File
import kotlin.reflect.KClass
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The play page over the real core: an xd library program, its pad and knobs, the layout kept per model. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class PlayBehaviourTest {
    @get:Rule val compose = createComposeRule()

    @get:Rule val temp = TemporaryFolder()

    private val store = ViewModelStore()

    @After
    fun clear() = store.clear()

    private lateinit var library: LibraryModel
    private lateinit var synth: SynthModel
    private lateinit var editor: EditorModel
    private lateinit var prefs: PreferencesModel
    private lateinit var play: PlayModel

    private fun <T : ViewModel> make(kind: KClass<T>, new: () -> T): T =
        ViewModelProvider.create(store, object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <V : ViewModel> create(modelClass: KClass<V>, extras: CreationExtras): V = new() as V
        })[kind]

    private fun await(condition: () -> Boolean) = compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle(condition) }

    /** An xd library with one program, open, and the play page over it ([restoring]: through a recreation). */
    private fun show(open: Boolean = true, restoring: StateRestorationTester? = null) {
        val dir = temp.newFolder("libraries").path
        val id = Libraries.open(dir).use { l -> l.create("Gig", XD, null).also { id -> l.read(id).use { d -> d.newProgram(null); l.save(id, d) } } }
        synth = make(SynthModel::class) { SynthModel(NoMidi) }
        library = make(LibraryModel::class) { LibraryModel(dir, synth) }
        editor = make(EditorModel::class) { EditorModel(library, synth) }
        prefs = make(PreferencesModel::class) { PreferencesModel(File(temp.root, "settings.json").path) }
        play = make(PlayModel::class) { PlayModel(editor, prefs) }
        val content = @androidx.compose.runtime.Composable { OscylTheme(OscylThemeMode.Dark) { PlayScreen(play, editor, onBack = {}) } }
        if (restoring != null) restoring.setContent(content) else compose.setContent(content)
        if (open) {
            compose.runOnIdle { library.select(librarySource(id)) }
            await { library.ui.value.programs?.rows?.isNotEmpty() == true }
            compose.runOnIdle { library.selectProgram(0) }
            await { editor.head.value != null && play.layout.value.x != null }
        }
    }

    @Test
    fun opensWithDefaultsForTheModel() {
        show()
        compose.runOnIdle { assertEquals(editor.playDefaults(), play.layout.value) }
        compose.onNodeWithTag("xy-pad").assertExists()
        assertEquals(4, (0..3).count { compose.onAllNodesWithTag("play-macro-$it").fetchSemanticsNodes().isNotEmpty() })
    }

    @Test
    fun padMovesBothFieldsAsOneUndoStep() {
        show()
        val (x, y) = compose.runOnIdle { play.layout.value.x!! to play.layout.value.y!! }
        val before = compose.runOnIdle { editor.panel!!.field(x)!!.raw to editor.panel!!.field(y)!!.raw }
        compose.onNodeWithTag("xy-pad").performTouchInput {
            down(Offset(width * 0.2f, height * 0.8f))
            moveBy(Offset(width * 0.5f, -height * 0.5f))
            up()
        }
        await { editor.panel!!.field(x)!!.raw != before.first && editor.panel!!.field(y)!!.raw != before.second }
        await { library.history().size == 1 }
    }

    @Test
    fun macroTurnsItsField() {
        show()
        val f = compose.runOnIdle { play.layout.value.macros[0]!! }
        val state = compose.runOnIdle { editor.panel!!.field(f)!! }
        val max = (state.control as ControlKind.Dial).max
        val to = if (state.raw == max) 0f else max.toFloat()
        // Two of the xd's dials are both called "Shape" (VCO 1 and 2): the page names which ("VCO 1 Shape").
        compose.onNode(hasContentDescription(state.label, substring = true) and hasAnyAncestor(hasTestTag("play-macro-0"))).performSemanticsAction(SemanticsActions.SetProgress) { it(to) }
        await { editor.panel!!.field(f)!!.raw.toFloat() == to }
    }

    @Test
    fun assigningAFieldIsRememberedForTheModel() {
        show()
        val (old, other) = compose.runOnIdle {
            val panel = editor.panel!!
            val taken = play.layout.value.let { listOfNotNull(it.x, it.y) + it.macros.filterNotNull() }
            play.layout.value.macros[0]!! to panel.pages.indices.flatMap { p -> panel.pages[p].sections.flatMap { it.fields } }
                .mapNotNull { panel.field(it) }.first { it.relevant && it.control is ControlKind.Dial && it.field !in taken }.field
        }
        compose.onNodeWithTag("play-assign-M1").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("play-field-$other").performClick()
        await { play.layout.value.macros[0] == other }
        assertNotEquals(old, other)
        // Kept for the model: a page made again opens with it.
        compose.runOnIdle { assertEquals(other, prefs.playLayout(XD)!!.macros[0]) }
    }

    @Test
    fun fieldNotShownByThisProgramIsUnassignedHere() {
        show()
        // A field this program does not show (or any non-dial, which the pad and knobs never show).
        val hidden = compose.runOnIdle {
            val panel = editor.panel!!
            panel.pages.indices.flatMap { p -> panel.pages[p].sections.flatMap { it.fields } }
                .mapNotNull { panel.field(it) }.first { !it.relevant || it.control !is ControlKind.Dial }.field
        }
        compose.runOnIdle { play.assign(PlaySlot.M2, hidden) }
        compose.waitForIdle()
        compose.onAllNodesWithTag("play-macro-1").fetchSemanticsNodes().let { assertTrue(it.isEmpty()) }
        compose.onNodeWithText("Choose").assertExists()
        compose.runOnIdle { assertEquals(hidden, play.layout.value.macros[1]) }
    }

    @Test
    fun unassignedPadOffersToChoose() {
        show()
        compose.runOnIdle { play.assign(PlaySlot.X, null) }
        compose.onNodeWithTag("play-choose-pad").assertExists()
        compose.onNodeWithTag("xy-pad").assertDoesNotExist()
    }

    @Test
    fun readOnlyProgramDisablesControls() {
        show(open = false)
        compose.runOnIdle { synth.openDemo(REV2) }
        await { synth.cards.value.any { it.connected } }
        compose.runOnIdle { library.select(synthSource(synth.cards.value.first().id)) }
        await { library.ui.value.programs?.rows?.isNotEmpty() == true }
        val factory = library.ui.value.programs!!.rows.indexOfFirst { it.readOnly }
        compose.runOnIdle { library.selectProgram(factory) }
        await { editor.head.value?.readOnly == true }
        // Read or still on its way, it cannot change: the note says which.
        val note = if (compose.runOnIdle { editor.head.value!!.loaded }) "Factory slot: read-only." else "Not read yet:"
        compose.onNodeWithText(note, substring = true).assertExists()
        // Every knob shown is off: said, not turned.
        val disabled = SemanticsMatcher.keyIsDefined(SemanticsProperties.Disabled)
        (0..3).filter { compose.onAllNodesWithTag("play-macro-$it").fetchSemanticsNodes().isNotEmpty() }.forEach {
            compose.onNode(hasAnyAncestor(hasTestTag("play-macro-$it")) and hasContentDescription("", substring = true) and disabled).assertExists()
        }
        if (compose.onAllNodesWithTag("xy-pad").fetchSemanticsNodes().isNotEmpty()) compose.onNodeWithTag("xy-pad").assertIsNotEnabled()
    }

    @Test
    fun rotationKeepsPageAndSheet() {
        val restoration = StateRestorationTester(compose)
        show(restoring = restoration)
        compose.onNodeWithTag("play-assign-X").performClick()
        compose.waitForIdle()
        val pane = SemanticsMatcher.expectValue(SemanticsProperties.PaneTitle, "Choose for Across")
        assertEquals(1, compose.onAllNodes(pane, useUnmergedTree = true).fetchSemanticsNodes().size)
        restoration.emulateSavedInstanceStateRestore()
        compose.waitForIdle()
        assertEquals(1, compose.onAllNodes(pane, useUnmergedTree = true).fetchSemanticsNodes().size)
        compose.onNodeWithTag("xy-pad").assertExists()
    }

    private companion object {
        const val TIMEOUT_MS = 10_000L
        const val XD = "korg-minilogue-xd"
        const val REV2 = "sequential-prophet-rev2"
    }
}
