package io.github.oscyl.editor

import androidx.compose.ui.unit.dp
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.swipeLeft
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performScrollToIndex
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTouchInput
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.core.ControlKind
import io.github.oscyl.core.Libraries
import io.github.oscyl.synth.FakeMidi
import io.github.oscyl.platform.MidiPortInfo
import io.github.oscyl.platform.NoMidi
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.editor.EditorModel
import io.github.oscyl.ui.editor.EditorScreen
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.library.librarySource
import io.github.oscyl.ui.library.synthSource
import io.github.oscyl.ui.synth.SynthModel
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The editor over the real core (host bindings): a library's program, a fake USB minilogue xd, the demo Rev2. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class EditorBehaviourTest {
    @get:Rule val compose = createComposeRule()

    @get:Rule val temp = TemporaryFolder()

    private val store = ViewModelStore()

    @After
    fun clear() = store.clear()

    private lateinit var library: LibraryModel
    private lateinit var synth: SynthModel
    private lateinit var editor: EditorModel
    private var dir = ""

    /** An xd library with one program, open in the editor, with [midi] plugged in. */
    private fun show(midi: io.github.oscyl.platform.MidiPlatform = NoMidi, open: Boolean = true): EditorModel {
        dir = temp.newFolder("libraries").path
        val id = Libraries.open(dir).use { l ->
            l.create("Gig", XD, null).also { id -> l.read(id).use { doc -> doc.newProgram(null); doc.setName(0u, "Glass"); l.save(id, doc) } }
        }
        fun <T : androidx.lifecycle.ViewModel> make(kind: kotlin.reflect.KClass<T>, new: () -> T): T =
            ViewModelProvider.create(store, object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <V : androidx.lifecycle.ViewModel> create(modelClass: kotlin.reflect.KClass<V>, extras: androidx.lifecycle.viewmodel.CreationExtras): V = new() as V
            })[kind]
        synth = make(SynthModel::class) { SynthModel(midi) }
        library = make(LibraryModel::class) { LibraryModel(dir, synth) }
        editor = make(EditorModel::class) { EditorModel(library, synth) }
        compose.setContent {
            // As App does: the synths' panels are followed frame by frame.
            androidx.compose.runtime.LaunchedEffect(editor) { editor.followPanels() }
            OscylTheme(OscylThemeMode.Dark) {
                val head by editor.head.collectAsStateWithLifecycle()
                EditorScreen(head, editor.panel, editor, onOpenLibrary = {})
            }
        }
        if (open) {
            compose.runOnIdle { library.select(librarySource(id)) }
            await { library.ui.value.programs?.rows?.isNotEmpty() == true }
            compose.runOnIdle { library.selectProgram(0) }
            await { editor.head.value != null }
        }
        return editor
    }

    /** The stage rail's tab for stage [title]. */
    private fun tab(title: String) = compose.onNode(
        SemanticsMatcher("rail tab $title") { node ->
            node.config.getOrElseNullable(androidx.compose.ui.semantics.SemanticsProperties.ContentDescription) { null }?.any { d -> d.startsWith("$title, ") } == true
        },
    )

    private fun await(condition: () -> Boolean) = compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle(condition) }

    private fun cutoff(): UInt = editor.panel!!.pages.flatMap { p -> p.sections.flatMap { it.fields } }
        .first { editor.panel!!.field(it)?.label == "Cutoff" }

    @Test
    fun dialEditMarksTheFieldAndPlaysLive() {
        val midi = FakeMidi(USB_XD)
        show(midi)
        await { editor.head.value?.canLive == true }
        compose.runOnIdle { editor.toggleLive() }
        await { editor.head.value?.live == true }
        val f = cutoff()
        val before = editor.panel!!.field(f)!!.raw
        val to = if (before > 500u) before - 100u else before + 100u
        val sent = midi.sent.size
        tab("Filter").performClick()
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodes(hasContentDescription("Cutoff")).fetchSemanticsNodes().isNotEmpty() }
        compose.onNode(hasContentDescription("Cutoff"))
            .performSemanticsAction(SemanticsActions.SetProgress) { it(to.toFloat()) }
        await { editor.panel!!.field(f)!!.edited }
        assertEquals(to, editor.panel!!.field(f)!!.raw)
        assertEquals(1, editor.head.value!!.saveCount)
        // The cutoff goes to the synth as a Control Change, so held notes keep sounding.
        await { midi.sent.drop(sent).any { (it[0].toInt() and 0xF0) == 0xB0 } }
        assertTrue(editor.panel!!.sends > 0)
        // The gesture ended: the library was saved with the edit.
        await { Libraries.open(dir).use { l -> l.read(l.outline().first().id).use { it.editor(0u).use { e -> e.field(f).raw } } } == to }
    }

    @Test
    fun compareShowsSavedAndReleaseReturns() {
        show()
        // Pictures and fields keep current on screen: show the filter.
        tab("Filter").performClick()
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodes(hasContentDescription("Cutoff")).fetchSemanticsNodes().isNotEmpty() }
        val f = cutoff()
        val saved = editor.panel!!.field(f)!!.raw
        val to = if (saved > 500u) saved - 50u else saved + 50u
        compose.runOnIdle {
            editor.set(f, to.toInt())
            editor.release()
        }
        compose.onNodeWithContentDescription("Hold to compare").performTouchInput { down(center) }
        await { editor.head.value?.comparing == true }
        assertEquals(saved, editor.panel!!.field(f)!!.raw)
        assertTrue(editor.panel!!.comparing)
        compose.onNodeWithContentDescription("Hold to compare").performTouchInput { up() }
        await { editor.head.value?.comparing == false }
        assertEquals(to, editor.panel!!.field(f)!!.raw)
    }

    /** An edit of the cutoff by 200, sealed; returns the field, its saved and its edited value. */
    private fun editedCutoff(): Triple<UInt, UInt, UInt> {
        tab("Filter").performClick()
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodes(hasContentDescription("Cutoff")).fetchSemanticsNodes().isNotEmpty() }
        val f = cutoff()
        val saved = editor.panel!!.field(f)!!.raw
        val to = if (saved > 500u) saved - 200u else saved + 200u
        compose.runOnIdle {
            editor.set(f, to.toInt())
            editor.release()
        }
        return Triple(f, saved, to)
    }

    @Test
    fun slidingCompareBlendsValues() {
        show()
        val (f, saved, to) = editedCutoff()
        compose.onNodeWithContentDescription("Hold to compare").performTouchInput {
            down(center)
            // 60 dp toward the start (left here) of the 160 that run from saved to the edit.
            moveBy(Offset(-60.dp.toPx(), 0f))
        }
        await { editor.panel!!.blend in 0.2f..0.5f }
        val shown = editor.panel!!.field(f)!!.raw
        assertTrue("$shown lies between $saved and $to", shown > minOf(saved, to) && shown < maxOf(saved, to))
        compose.onNodeWithTag("morph-scale", useUnmergedTree = true).assertExists()
        compose.onNodeWithContentDescription("Hold to compare").performTouchInput { up() }
        await { editor.head.value?.comparing == false }
        assertEquals(to, editor.panel!!.field(f)!!.raw)
    }

    @Test
    fun releaseReturnsToEdit() {
        show()
        val (f, _, to) = editedCutoff()
        compose.onNodeWithContentDescription("Hold to compare").performTouchInput {
            down(center)
            moveBy(Offset(-120.dp.toPx(), 0f))
            up()
        }
        await { editor.panel!!.blend == 1f && editor.head.value?.comparing == false }
        assertEquals(to, editor.panel!!.field(f)!!.raw)
        compose.onNodeWithTag("morph-scale", useUnmergedTree = true).assertDoesNotExist()
    }

    @Test
    fun holdWithoutSlideStillCompares() {
        show()
        val (f, saved, _) = editedCutoff()
        compose.onNodeWithContentDescription("Hold to compare").performTouchInput { down(center) }
        await { editor.head.value?.comparing == true }
        assertEquals(saved, editor.panel!!.field(f)!!.raw)
        assertEquals(0f, editor.panel!!.blend)
        // Held, the fader is out beside the part ("Slide to blend"), so the slide can be found.
        compose.onNodeWithTag("morph-scale", useUnmergedTree = true).assertExists()
        compose.onNodeWithContentDescription("Hold to compare").performTouchInput { up() }
        await { editor.head.value?.comparing == false }
    }

    @Test
    fun undoTakesBackAnEdit() {
        show()
        // Pictures and fields keep current on screen: show the filter.
        tab("Filter").performClick()
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodes(hasContentDescription("Cutoff")).fetchSemanticsNodes().isNotEmpty() }
        val f = cutoff()
        compose.runOnIdle {
            editor.set(f, 10)
            editor.release()
        }
        await { editor.head.value?.canUndo == true }
        compose.onNodeWithContentDescription("Undo").performClick()
        await { editor.panel?.field(f)?.edited == false }
        assertEquals(0, editor.head.value!!.saveCount)
    }

    @Test
    fun stageRailJumpScrolls() {
        show()
        val effects = editor.panel!!.pages.indexOfFirst { it.title == "Effects" }
        assertTrue(effects > 0)
        tab("Effects").performClick()
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodesWithText("Effects").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Effects").assertIsDisplayed()
    }

    @Test
    fun aFactorySlotRefusesEdits() {
        show(open = false)
        compose.runOnIdle { synth.openDemo(REV2) }
        await { synth.cards.value.any { it.connected } }
        val card = synth.cards.value.first()
        compose.runOnIdle { library.select(synthSource(card.id)) }
        await { library.ui.value.programs?.rows?.isNotEmpty() == true }
        val factory = library.ui.value.programs!!.rows.indexOfFirst { it.readOnly }
        compose.runOnIdle { library.selectProgram(factory) }
        await { editor.head.value?.readOnly == true }
        compose.onNodeWithText("Factory slot: read-only. Copy it to a library to change it.").assertIsDisplayed()
        val dial = editor.panel!!.pages.indices.flatMap { p -> editor.panel!!.pages[p].sections.flatMap { it.fields } }
            .mapNotNull { editor.panel!!.field(it) }.first { it.control is ControlKind.Dial }
        compose.runOnIdle {
            editor.set(dial.field, dial.raw.toInt() + 1)
            editor.release()
        }
        compose.runOnIdle {
            assertFalse(editor.panel!!.field(dial.field)!!.edited)
            assertEquals(0, editor.head.value!!.saveCount)
        }
        // Its controls are off: said, not turned.
        compose.onAllNodes(SemanticsMatcher.keyIsDefined(androidx.compose.ui.semantics.SemanticsProperties.Disabled) and hasContentDescription(dial.label))
            .fetchSemanticsNodes().let { assertTrue("${dial.label} is off", it.isNotEmpty()) }
    }

    /** Every field of the open program as the core has it now. */
    private fun raws(): Map<UInt, UInt> = library.shown.value!!.document.editor(0u).use { e ->
        e.pages().flatMap { p -> p.sections.flatMap { it.fields } }.associateWith { e.field(it).raw }
    }

    private fun filterPage() = editor.panel!!.pages.indexOfFirst { it.title == "Filter" }

    @Test
    fun diceRollsOnlyItsStage() {
        show()
        tab("Filter").performClick()
        val page = filterPage()
        val stage = editor.panel!!.pages[page].sections.flatMap { it.fields }.toSet()
        val before = compose.runOnIdle { raws() }
        // The dice roll on a hold (a tap only says so).
        compose.onNodeWithTag("roll-$page").performSemanticsAction(SemanticsActions.OnLongClick)
        await { raws() != before }
        val after = compose.runOnIdle { raws() }
        val changed = after.filter { (f, v) -> before[f] != v }.keys
        assertTrue("only the filter's fields moved: $changed", changed.isNotEmpty() && changed.all { it in stage })
        // Undo is under the thumb: the notice says what was rolled.
        compose.runOnIdle { assertEquals(io.github.oscyl.ui.library.Notice.Rolled("Filter"), library.ui.value.notice) }
        assertEquals(1, editor.head.value!!.saveCount)
    }

    @Test
    fun undoAfterRollRestoresStage() {
        show()
        tab("Filter").performClick()
        val page = filterPage()
        val before = compose.runOnIdle { raws() }
        // The dice roll on a hold (a tap only says so).
        compose.onNodeWithTag("roll-$page").performSemanticsAction(SemanticsActions.OnLongClick)
        await { raws() != before }
        compose.runOnIdle { editor.undo() }
        await { raws() == before }
    }

    @Test
    fun noDiceOnReadOnlyOrUnread() {
        show(open = false)
        compose.runOnIdle { synth.openDemo(REV2) }
        await { synth.cards.value.any { it.connected } }
        compose.runOnIdle { library.select(synthSource(synth.cards.value.first().id)) }
        await { library.ui.value.programs?.rows?.isNotEmpty() == true }
        val factory = library.ui.value.programs!!.rows.indexOfFirst { it.readOnly }
        compose.runOnIdle { library.selectProgram(factory) }
        await { editor.head.value?.readOnly == true }
        assertEquals(0, compose.onAllNodes(hasTestTagStartingWith("roll-")).fetchSemanticsNodes().size)
    }

    private fun hasTestTagStartingWith(prefix: String) = SemanticsMatcher("tag starts with $prefix") { node ->
        node.config.getOrElseNullable(androidx.compose.ui.semantics.SemanticsProperties.TestTag) { null }?.startsWith(prefix) == true
    }

    @Test
    fun recallIsOneUndoStep() {
        show()
        val stored = compose.runOnIdle { raws() }
        compose.runOnIdle { editor.storeSnapshot(0) }
        await { editor.snapshots.value == listOf(true, false, false) }
        val (f, _, to) = editedCutoff()
        compose.runOnIdle { editor.recallSnapshot(0) }
        await { raws() == stored }
        assertEquals("Recalled snapshot A", library.history().last().title)
        compose.runOnIdle { editor.undo() }
        await { editor.panel!!.field(f)!!.raw == to }
    }

    @Test
    fun openingAnotherProgramClearsSnapshots() {
        show()
        compose.runOnIdle {
            editor.storeSnapshot(1)
            library.newProgram()
        }
        await { library.ui.value.programs!!.rows.size == 2 }
        compose.runOnIdle { library.selectProgram(1) }
        await { editor.snapshots.value == listOf(false, false, false) }
    }

    @Test
    fun removingTheOpenProgramClearsSnapshots() {
        show()
        compose.runOnIdle { library.newProgram() }
        await { library.ui.value.programs!!.rows.size == 2 }
        compose.runOnIdle { library.selectProgram(0) }
        await { editor.head.value != null }
        compose.runOnIdle { editor.storeSnapshot(0) }
        await { editor.snapshots.value[0] }
        // Another program lands on the open index: the snapshot was of the one removed.
        compose.runOnIdle { library.act(0, io.github.oscyl.core.ProgramAction.REMOVE) }
        await { library.ui.value.programs!!.rows.size == 1 && editor.snapshots.value == listOf(false, false, false) }
    }

    /** Three programs, the middle one open with snapshot A stored. */
    private fun threeWithSnapshot() {
        show()
        compose.runOnIdle { library.newProgram() }
        await { library.ui.value.programs!!.rows.size == 2 }
        compose.runOnIdle { library.newProgram() }
        await { library.ui.value.programs!!.rows.size == 3 }
        compose.runOnIdle {
            library.renameProgram(0, "One")
            library.renameProgram(1, "Two")
            library.renameProgram(2, "Three")
            library.selectProgram(1)
        }
        await { editor.head.value?.name == "Two" }
        compose.runOnIdle { editor.storeSnapshot(0) }
        await { editor.snapshots.value[0] }
    }

    @Test
    fun undoOfARemovalBeforeTheOpenProgramClearsSnapshots() {
        threeWithSnapshot()
        compose.runOnIdle { library.act(0, io.github.oscyl.core.ProgramAction.REMOVE) }
        await { library.ui.value.programs!!.rows.size == 2 }
        // Now "Three" is at the open place; the snapshot (of "Two") is gone.
        await { editor.snapshots.value == listOf(false, false, false) }
        compose.runOnIdle { library.undo() }
        await { library.ui.value.programs!!.rows.size == 3 }
        await { editor.snapshots.value == listOf(false, false, false) }
    }

    @Test
    fun aPressDuringTheMorphsGlideKeepsComparing() {
        show()
        editedCutoff()
        val compare = compose.onNodeWithContentDescription("Hold to compare")
        compare.performTouchInput {
            down(center)
            moveBy(Offset(-120.dp.toPx(), 0f))
            up()
            // At once, before the glide to the edit has landed.
            down(center)
        }
        compose.waitForIdle()
        compose.mainClock.advanceTimeBy(1000)
        compose.waitForIdle()
        compose.runOnIdle {
            assertTrue("held: still comparing", editor.head.value!!.comparing)
            assertEquals(0f, editor.panel!!.blend)
        }
        compare.performTouchInput { up() }
        await { editor.head.value?.comparing == false }
    }

    /** Looking around the Library (its sources, another library) never closes the program on the bench. */
    @Test
    fun browsingTheLibraryKeepsTheOpenProgram() {
        show()
        val (f, _, to) = editedCutoff()
        compose.runOnIdle { library.select(null) }
        compose.waitForIdle()
        compose.runOnIdle {
            assertTrue("still open", library.open.value != null && editor.head.value != null)
            assertEquals(to, editor.panel!!.field(f)!!.raw)
        }
        // The editor's undo still reaches its own program while the list shows nothing.
        compose.runOnIdle { editor.undo() }
        await { editor.panel!!.field(f)!!.raw != to }
        // Back on its library, the list shows it selected.
        compose.runOnIdle { library.select(library.ui.value.lines.first().let { librarySource(it.id) }) }
        await { library.ui.value.programs?.selected == 0 }
    }

    @Test
    fun recallRefusedOnReadOnly() {
        show(open = false)
        compose.runOnIdle { synth.openDemo(REV2) }
        await { synth.cards.value.any { it.connected } }
        compose.runOnIdle { library.select(synthSource(synth.cards.value.first().id)) }
        await { library.ui.value.programs?.rows?.isNotEmpty() == true }
        val factory = library.ui.value.programs!!.rows.indexOfFirst { it.readOnly }
        compose.runOnIdle { library.selectProgram(factory) }
        await { editor.head.value?.readOnly == true }
        compose.runOnIdle {
            editor.storeSnapshot(0)
            editor.recallSnapshot(0)
        }
        compose.runOnIdle { assertEquals(0, editor.head.value!!.saveCount) }
    }

    @Test
    fun aSlowHandleDragFollowsTheFingerAndIsOneUndoStep() {
        show()
        tab("Filter").performClick()
        val handle = SemanticsMatcher("cutoff handle") { node ->
            node.config.getOrElseNullable(androidx.compose.ui.semantics.SemanticsProperties.ContentDescription) { null }
                ?.any { it.startsWith("Cutoff and resonance") } == true
        }
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodes(handle).fetchSemanticsNodes().isNotEmpty() }
        val f = cutoff()
        val before = editor.panel!!.field(f)!!.raw
        // Many moves of one pixel, each smaller than a raw step: together they still move it.
        compose.onNode(handle).performTouchInput {
            down(center)
            moveBy(androidx.compose.ui.geometry.Offset(-viewConfiguration.touchSlop - 1f, 0f))
            repeat(120) { moveBy(androidx.compose.ui.geometry.Offset(-1f, 0f)) }
            up()
        }
        await { editor.panel!!.field(f)!!.raw != before }
        compose.onNodeWithContentDescription("Undo").performClick()
        await { editor.panel!!.field(f)!!.raw == before }
        assertFalse("the whole drag was one step", editor.head.value!!.canUndo)
    }

    @Test
    fun theHistoryListsEachEditAndAJumpGoesThereInOneMove() {
        show()
        // The Filter in view: only the stages on screen keep their fields current.
        tab("Filter").performClick()
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodes(hasContentDescription("Cutoff")).fetchSemanticsNodes().isNotEmpty() }
        val f = cutoff()
        val before = editor.panel!!.field(f)!!.raw
        compose.runOnIdle {
            editor.set(f, 100)
            editor.release()
            editor.set(f, 200)
            editor.release()
        }
        await { editor.head.value?.canUndo == true }
        val steps = compose.runOnIdle { library.history() }
        assertEquals(2, steps.size)
        assertTrue(steps.all { it.done && it.title.contains("Cutoff") })
        // Back to as saved: both undone as one move, and still listed, waiting.
        compose.runOnIdle { library.jumpHistory(0) }
        await { editor.panel!!.field(f)!!.raw == before && editor.head.value?.canUndo == false }
        assertTrue(compose.runOnIdle { library.history() }.none { it.done })
        // Forward to the first edit only.
        compose.runOnIdle { library.jumpHistory(1) }
        await { editor.panel!!.field(f)!!.raw == 100u }
        assertEquals(listOf(true, false), compose.runOnIdle { library.history() }.map { it.done })
    }

    @Test
    fun aHandleDragOnAPagerStageTheListDoesNotShowMovesItsPictureEveryStep() {
        show()
        val panel = editor.panel!!
        val filter = panel.pages.indexOfFirst { it.title == "Filter" }
        // The first stage with a picture, full screen; the list under it stays at its top.
        compose.onAllNodes(hasContentDescription("full screen", substring = true))[0].performClick()
        compose.waitForIdle()
        val handle = SemanticsMatcher("cutoff handle") { node ->
            node.config.getOrElseNullable(androidx.compose.ui.semantics.SemanticsProperties.ContentDescription) { null }
                ?.any { it.startsWith("Cutoff and resonance") } == true
        }
        // Swipe the pager to the Filter: a stage the list's own view never reported.
        // (Its neighbour's handle is composed one page early: wait until it is the page in view.)
        fun inView() = compose.onAllNodes(handle).fetchSemanticsNodes().any { n -> n.boundsInRoot.width > 0f && n.boundsInRoot.left >= 0f && n.boundsInRoot.right <= compose.onRoot().fetchSemanticsNode().boundsInRoot.width }
        repeat(6) {
            if (!inView()) {
                compose.onNodeWithTag("graph pager").performTouchInput { swipeLeft() }
                compose.waitForIdle()
            }
        }
        assertTrue("the Filter's page is in view", inView())
        val (dot, picture) = compose.runOnIdle { panel.handles(filter).first().x to panel.graph(filter)!!.now }
        compose.onNode(handle).performTouchInput {
            down(center)
            moveBy(androidx.compose.ui.geometry.Offset(-viewConfiguration.touchSlop - 1f, 0f))
            repeat(60) { moveBy(androidx.compose.ui.geometry.Offset(-2f, 0f)) }
        }
        // Still held: the handle and the curve have already followed, step by step.
        compose.runOnIdle {
            assertTrue("the handle stood still under the drag", panel.handles(filter).first().x < dot)
            assertTrue("the curve stood still under the drag", panel.graph(filter)!!.now != picture)
        }
        compose.onNode(handle).performTouchInput { up() }
    }

    @Test
    fun aSlotNotReadYetSaysSoRefusesEditsAndNeverPlays() {
        // A USB xd whose first read is held while the read is stopped: no slot is read.
        val midi = FakeMidi(USB_XD).apply { holdReplies = true }
        show(midi, open = false)
        await { synth.cards.value.any { it.connected } }
        val card = synth.cards.value.first()
        compose.runOnIdle { synth.cancelRead(card) }
        midi.answer()
        await { synth.cards.value.first().read == null }
        compose.runOnIdle { library.select(synthSource(card.id)) }
        await { library.ui.value.programs?.rows?.isNotEmpty() == true }
        compose.runOnIdle { library.selectProgram(5) }
        await { editor.head.value?.loaded == false }
        compose.onNode(androidx.compose.ui.test.hasScrollAction()).performScrollToIndex(0)
        compose.onNodeWithText("Not read yet", substring = true).assertExists()
        compose.runOnIdle { editor.toggleLive() }
        await { editor.head.value?.live == true }
        val sent = midi.sent.size
        val f = cutoff()
        compose.runOnIdle {
            editor.set(f, 7)
            editor.release()
        }
        compose.runOnIdle {
            assertFalse(editor.panel!!.field(f)!!.edited)
            // The placeholder never plays: nothing but the read requests went out.
            assertTrue(midi.sent.drop(sent).none { it[0] != 0xF0.toByte() || it.size > 16 })
        }
    }

    /** A whole program sent to the synth's edit buffer (current-program dump). */
    private fun isDump(m: ByteArray) = m.size > 6 && m[0] == 0xF0.toByte() && m[6] == 0x40.toByte()

    /** The panel picks program [n] (of the first hundred). */
    private fun pick(midi: FakeMidi, n: Int) =
        midi.panel(byteArrayOf(0xB0.toByte(), 0, 0, 0xB0.toByte(), 0x20, 0, 0xC0.toByte(), n.toByte()))

    /** A USB xd read in full, its slot [at] open, with Live on when [live]; the mark of what was sent so far. */
    private fun openOnUsbXd(midi: FakeMidi, at: Int, live: Boolean): Pair<io.github.oscyl.ui.synth.SynthCard, Int> {
        show(midi, open = false)
        await { synth.cards.value.any { it.connected } }
        val card = synth.cards.value.first()
        await { synth.document(card.id)?.allRead() == true && synth.cards.value.first().read == null }
        compose.runOnIdle { library.select(synthSource(card.id)) }
        await { library.ui.value.programs?.rows?.isNotEmpty() == true }
        compose.runOnIdle { library.selectProgram(at) }
        await { editor.head.value?.canLive == true }
        if (live) {
            compose.runOnIdle { editor.toggleLive() }
            await { editor.head.value?.live == true }
            // Live on plays the open program: past that, nothing else should go.
            await { midi.sent.any(::isDump) }
        }
        return card to midi.sent.size
    }

    /** Live on: the synth's panel picking a program opens it here, and nothing is sent back. */
    @Test
    fun liveFollowsTheSynthPanel() {
        val midi = FakeMidi(USB_XD)
        val (_, mark) = openOnUsbXd(midi, at = 3, live = true)
        pick(midi, 7)
        await { library.open.value?.index == 7 }
        pick(midi, 8)
        pick(midi, 9)
        await { library.open.value?.index == 9 }
        compose.runOnIdle { assertTrue(midi.sent.drop(mark).none(::isDump)) }
    }

    /** The slot picked holds unsynced edits here: they are sent, so the synth plays what is shown. */
    @Test
    fun followingASlotWithEditsSendsThem() {
        val midi = FakeMidi(USB_XD)
        val (card, _) = openOnUsbXd(midi, at = 3, live = true)
        compose.runOnIdle {
            synth.document(card.id)!!.editor(7u).use { e ->
                val f = e.fields(e.firstPage()).first { it.control is io.github.oscyl.core.ControlKind.Dial }
                e.set(f.field, if (f.raw > 500u) f.raw - 100u else f.raw + 100u)
            }
        }
        val mark = midi.sent.size
        pick(midi, 7)
        await { library.open.value?.index == 7 }
        await { midi.sent.drop(mark).any(::isDump) }
    }

    /** Live off: the app stays where it is. */
    @Test
    fun withoutLiveTheAppStays() {
        val midi = FakeMidi(USB_XD)
        val (card, _) = openOnUsbXd(midi, at = 3, live = false)
        pick(midi, 7)
        // Asked after the pick, the slots' listing comes back after it has been handled.
        compose.runOnIdle { synth.listUnits(card) }
        await { card.id in synth.units.value }
        compose.runOnIdle { assertEquals(3, library.open.value?.index) }
    }

    /** Live on: a knob turned on the synth moves the open program's value here, sending nothing back. */
    @Test
    fun liveFollowsTheSynthsKnobs() {
        val midi = FakeMidi(USB_XD)
        val (_, mark) = openOnUsbXd(midi, at = 3, live = true)
        // The filter on screen: its dials are kept current.
        tab("Filter").performClick()
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodes(hasContentDescription("Cutoff")).fetchSemanticsNodes().isNotEmpty() }
        val f = cutoff()
        midi.panel(byteArrayOf(0xB0.toByte(), 63, 5, 0xB0.toByte(), 43, 85))
        await { editor.panel!!.field(f)!!.raw == 685u }
        assertTrue(editor.panel!!.field(f)!!.edited)
        // Its lower bits and its value apart still pair up.
        midi.panel(byteArrayOf(0xB0.toByte(), 63, 1))
        midi.panel(byteArrayOf(0xB0.toByte(), 43, 10))
        await { editor.panel!!.field(f)!!.raw == 81u }
        compose.runOnIdle { assertTrue(midi.sent.drop(mark).none { (it[0].toInt() and 0xF0) == 0xB0 || isDump(it) }) }
    }

    /** Live off: the synth's knobs leave the app's values alone. */
    @Test
    fun withoutLiveTheKnobsLeaveTheAppAlone() {
        val midi = FakeMidi(USB_XD)
        val (card, _) = openOnUsbXd(midi, at = 3, live = false)
        val f = cutoff()
        val before = editor.panel!!.field(f)!!.raw
        midi.panel(byteArrayOf(0xB0.toByte(), 63, 5, 0xB0.toByte(), 43, 85))
        compose.runOnIdle { synth.listUnits(card) }
        await { card.id in synth.units.value }
        compose.runOnIdle { assertEquals(before, editor.panel!!.field(f)!!.raw) }
    }

    private companion object {
        const val TIMEOUT_MS = 10_000L
        const val XD = "korg-minilogue-xd"
        const val REV2 = "sequential-prophet-rev2"
        val USB_XD = MidiPortInfo(name = "minilogue xd _ SOUND", id = "7")
    }
}
