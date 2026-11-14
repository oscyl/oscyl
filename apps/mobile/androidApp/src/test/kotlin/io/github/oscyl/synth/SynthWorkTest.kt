package io.github.oscyl.synth

import androidx.compose.runtime.collectAsState
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.onNodeWithContentDescription
import io.github.oscyl.ui.synth.SynthScreen
import io.github.oscyl.ui.synth.UnitSlot
import io.github.oscyl.ui.synth.SynthCard
import io.github.oscyl.ui.synth.SynthActions
import io.github.oscyl.core.BackupFile
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performSemanticsAction
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.viewmodel.CreationExtras
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.core.ControlKind
import io.github.oscyl.core.Document
import io.github.oscyl.core.ProgramAction
import io.github.oscyl.core.keepUnsynced
import io.github.oscyl.core.importFile
import io.github.oscyl.core.Libraries
import io.github.oscyl.core.exportFile
import io.github.oscyl.platform.Files
import io.github.oscyl.platform.MidiPortInfo
import io.github.oscyl.platform.NoMidi
import io.github.oscyl.platform.PickFor
import io.github.oscyl.platform.PickedFile
import io.github.oscyl.platform.Sink
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.editor.EditorModel
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.library.LibraryScreen
import io.github.oscyl.ui.library.Notice
import io.github.oscyl.ui.library.librarySource
import io.github.oscyl.ui.library.synthSource
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

/** The synth's work (reads, current program, units, backups) and the library's safety nets, over the real core, in temporary folders. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class SynthWorkTest {
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

    private fun demo(driver: String, synth: SynthModel): ULong {
        compose.runOnIdle { synth.openDemo(driver) }
        await { synth.cards.value.any { it.connected } }
        return synth.cards.value.first().id
    }

    @Test
    fun aReadCanBeStoppedAndStartedAgain() {
        val synth = make(SynthModel::class) { SynthModel(NoMidi) }
        compose.setContent {}
        val id = demo(REV2, synth)
        await { synth.cards.value.first().read != null }
        compose.runOnIdle { synth.cancelRead(synth.cards.value.first()) }
        await { synth.cards.value.first().read == null }
        val doc = synth.document(id)!!
        assertTrue("stopped before the end", doc.rows().any { !it.loaded })
        compose.runOnIdle { synth.readAll(synth.cards.value.first()) }
        await { synth.cards.value.first().read != null }
    }

    @Test
    fun disconnectingLetsGoOfThePorts() {
        val midi = FakeMidi(USB_XD)
        val synth = make(SynthModel::class) { SynthModel(midi) }
        compose.setContent {}
        await { synth.cards.value.any { it.connected } }
        compose.runOnIdle { synth.disconnect(synth.cards.value.first()) }
        await { midi.released.isNotEmpty() }
        assertEquals(USB_XD, midi.released.single())
    }

    @Test
    fun theCurrentProgramIsKeptAsALibrary() {
        val synth = make(SynthModel::class) { SynthModel(NoMidi) }
        val library = make(LibraryModel::class) { LibraryModel(temp.newFolder("libraries").path, synth) }
        compose.setContent {}
        demo(XD, synth)
        compose.runOnIdle { synth.readCurrent(synth.cards.value.first()) }
        await { library.ui.value.lines.any { it.name == "Current program (minilogue xd)" } }
    }

    @Test
    fun unitSlotsAreListedAndClearedWithABackup() {
        val backups = temp.newFolder("backups")
        val synth = make(SynthModel::class) { SynthModel(NoMidi, backupsDir = backups.path) }
        val actions = object : SynthActions {
            override fun clearUnit(card: SynthCard, slot: UnitSlot) = synth.clearUnit(card, slot.module, slot.slot, slot.name ?: "")
            override fun putBack(file: BackupFile) = synth.putBack(file)
        }
        compose.setContent { OscylTheme(OscylThemeMode.Dark) { SynthScreen(synth, onAbout = {}, actions = actions) } }
        val id = demo(XD, synth)
        await { synth.units.value[id]?.isNotEmpty() == true }
        val used = synth.units.value[id]!!.first { it.name != null }
        val slotNow = { synth.units.value[id]!!.first { it.module == used.module && it.slot == used.slot }.name }
        // The units are a sub-page of the synth's row.
        compose.onNodeWithText("User units", substring = true).performClick()
        // Clear asks first; Cancel leaves the slot alone.
        compose.onNodeWithContentDescription("Clear ${used.name} (a backup is taken first)").performScrollTo().performClick()
        compose.onNodeWithText("Cancel").performClick()
        compose.onNodeWithContentDescription("Clear ${used.name} (a backup is taken first)").performScrollTo().performClick()
        compose.onNodeWithText("Clear slot").performClick()
        await { slotNow() == null }
        assertTrue(backups.listFiles().orEmpty().any { it.name.endsWith(".slot.bin") })
        await { synth.backups.value.any { it.name.endsWith(".slot.bin") } }
        // Its backup (a sub-page too) puts it back, into the first empty slot of its section: the one it left.
        compose.onNodeWithContentDescription("Back").performClick()
        compose.onNodeWithText("Backups", substring = true).performClick()
        compose.onNodeWithText("Put back").performScrollTo().performClick()
        await { slotNow() == used.name }
    }

    /** A tiny oscillator unit file for the minilogue xd, as the logue SDK packs one. */
    private fun unitFile(name: String): ByteArray {
        val out = java.io.ByteArrayOutputStream()
        java.util.zip.ZipOutputStream(out).use { zip ->
            zip.putNextEntry(java.util.zip.ZipEntry("unit/manifest.json"))
            zip.write(
                """{"header":{"platform":"minilogue-xd","module":"osc","api":"1.1-0","dev_id":0,"prg_id":0,"version":"1.0-0","name":"$name","params":[]}}""".toByteArray(),
            )
            zip.putNextEntry(java.util.zip.ZipEntry("unit/payload.bin"))
            zip.write("UOSC".toByteArray() + ByteArray(60))
        }
        return out.toByteArray()
    }

    @Test
    fun aUnitFileIsLoadedIntoAnEmptySlot() {
        val files = object : Files {
            override suspend fun open(purpose: PickFor): PickedFile? = PickedFile("Tiny.mnlgxdunit", unitFile("Tiny"))
            override suspend fun create(name: String, mime: String): Sink? = null
            override fun share(name: String, bytes: ByteArray, mime: String) = Unit
        }
        val synth = make(SynthModel::class) { SynthModel(NoMidi, files = files) }
        compose.setContent {}
        val id = demo(XD, synth)
        compose.runOnIdle { synth.listUnits(synth.cards.value.first()) }
        await { synth.units.value[id]?.isNotEmpty() == true }
        val empty = synth.units.value[id]!!.first { it.module == "osc" && it.name == null }
        compose.runOnIdle { synth.loadUnit(synth.cards.value.first(), empty.module, empty.slot) }
        await { synth.units.value[id]!!.first { it.module == "osc" && it.slot == empty.slot }.name == "Tiny" }
    }

    @Test
    fun aUnitForAnotherSectionIsRefusedInWords() {
        val files = object : Files {
            override suspend fun open(purpose: PickFor): PickedFile? = PickedFile("Tiny.mnlgxdunit", unitFile("Tiny"))
            override suspend fun create(name: String, mime: String): Sink? = null
            override fun share(name: String, bytes: ByteArray, mime: String) = Unit
        }
        val synth = make(SynthModel::class) { SynthModel(NoMidi, files = files) }
        compose.setContent {}
        demo(XD, synth)
        compose.runOnIdle { synth.loadUnit(synth.cards.value.first(), "delfx", 3u) }
        await { synth.notice.value?.contains("Tiny") == true }
    }

    @Test
    fun editsMadeDuringAReadAreKept() {
        val synth = make(SynthModel::class) { SynthModel(NoMidi) }
        val library = make(LibraryModel::class) { LibraryModel(temp.newFolder("libraries").path, synth) }
        val editor = make(EditorModel::class) { EditorModel(library, synth) }
        compose.setContent {}
        val id = demo(REV2, synth)
        compose.runOnIdle { library.select(synthSource(id)) }
        await { library.ui.value.programs?.rows?.firstOrNull()?.loaded == true }
        compose.runOnIdle { library.selectProgram(0) }
        await { editor.head.value?.loaded == true }
        await { synth.cards.value.first().read == null }
        // Read everything again, and edit the first slot while its program comes back.
        val before = synth.programs.value
        val field = compose.runOnIdle {
            synth.readAll(synth.cards.value.first())
            val panel = editor.panel!!
            val dial = panel.pages.indices.flatMap { p -> panel.pages[p].sections.flatMap { it.fields } }
                .mapNotNull { panel.field(it) }.first { it.relevant && it.control is ControlKind.Dial }
            val max = (dial.control as ControlKind.Dial).max
            editor.set(dial.field, if (dial.raw == max) 0 else max.toInt())
            editor.release()
            dial.field
        }
        // The read ends (its end changes the programs as a whole).
        await { synth.programs.value > before && synth.cards.value.first().read == null }
        assertTrue(synth.document(id)!!.isEdited(0u))
        compose.runOnIdle { assertTrue(editor.panel!!.field(field)!!.edited) }
    }

    @Test
    fun unsyncedSynthEditsOutliveTheProcess() {
        val unsynced = temp.newFolder("unsynced")
        val dir = temp.newFolder("libraries").path
        val synth = make(SynthModel::class) { SynthModel(NoMidi) }
        val library = make(LibraryModel::class) { LibraryModel(dir, synth, unsyncedDir = unsynced.path) }
        compose.setContent {}
        val id = demo(XD, synth)
        compose.runOnIdle { library.select(synthSource(id)) }
        await { library.ui.value.programs?.rows?.take(3)?.all { it.loaded } == true }
        val names = library.ui.value.programs!!.rows.map { it.name }
        // Every change is on disk by itself: no hand-made save after any of them.
        compose.runOnIdle { library.renameProgram(1, "Mine") }
        await { kept(unsynced)?.get(1) == "Mine" }
        compose.runOnIdle {
            library.act(0, ProgramAction.COPY)
            library.act(2, ProgramAction.PASTE_OVER)
        }
        await { kept(unsynced)?.get(2) == names[0] }
        compose.runOnIdle { library.undo() }
        await { kept(unsynced)?.get(2) == names[2] && kept(unsynced)?.get(1) == "Mine" }
        compose.runOnIdle { library.act(0, ProgramAction.CLEAR) }
        await { kept(unsynced)?.get(0) != names[0] }
        compose.runOnIdle { library.revert(0) }
        await { kept(unsynced)?.get(0) == names[0] && kept(unsynced)?.get(1) == "Mine" }
        // The process goes; the next one finds the edits and keeps them as a library.
        store.clear()
        val next = make(LibraryModel::class) { LibraryModel(dir, null, unsyncedDir = unsynced.path) }
        await { next.ui.value.lines.any { it.name == "minilogue xd (disconnected)" } }
        await { unsynced.listFiles().orEmpty().isEmpty() }
    }

    @Test
    fun aPreviousRunsEditsSurviveASynthConnectingWithTheSameId() {
        val unsynced = temp.newFolder("unsynced")
        val dir = temp.newFolder("libraries").path
        // A previous run's minilogue xd, worker id 1, left an edit.
        Document.synth(XD, "xd").use { d ->
            d.setName(3u, "Left behind")
            d.offline("").use { keepUnsynced(unsynced.path, "$XD-1-earlier", it) }
        }
        val synth = make(SynthModel::class) { SynthModel(NoMidi) }
        val library = make(LibraryModel::class) { LibraryModel(dir, synth, unsyncedDir = unsynced.path) }
        compose.setContent {}
        // This run's first synth is id 1 again, and its read ends with nothing edited.
        val id = demo(XD, synth)
        assertEquals(1uL, id)
        await { synth.cards.value.first().read == null }
        await { library.ui.value.lines.any { it.name == "minilogue xd (disconnected)" } }
        val line = library.ui.value.lines.first { it.name == "minilogue xd (disconnected)" }
        Libraries.open(dir).use { l -> l.read(line.id).use { assertEquals("Left behind", it.name(3u)) } }
    }

    /** The program names in the one unsynced-edits file in [dir], or null when there is none. */
    private fun kept(dir: File): List<String>? = compose.runOnIdle {
        val file = dir.listFiles().orEmpty().singleOrNull { it.name.endsWith(".oscyllib") } ?: return@runOnIdle null
        importFile(file.name, file.readBytes()).document.use { d -> (0u until d.len()).map { d.name(it) } }
    }

    @Test
    fun aScreenReaderMovesAProgram() {
        val dir = temp.newFolder("libraries").path
        val id = Libraries.open(dir).use { l ->
            l.create("Gig", XD, null).also { id ->
                l.read(id).use { d ->
                    d.newProgram(null); d.newProgram(null)
                    d.setName(0u, "First"); d.setName(1u, "Second"); l.save(id, d)
                }
            }
        }
        val library = make(LibraryModel::class) { LibraryModel(dir) }
        compose.setContent { OscylTheme(OscylThemeMode.Dark) { androidx.compose.foundation.layout.Box {
                LibraryScreen(library, onOpenSynth = {})
                io.github.oscyl.ui.library.NoticeHost(library.ui.collectAsState().value, library)
            } } }
        await { library.ui.value.lines.any { it.id == id } }
        compose.onNodeWithText("Gig").performClick()
        compose.waitUntil(TIMEOUT_MS) { compose.onAllNodesWithText("First").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("First", useUnmergedTree = true).performSemanticsActionOnRow("Move down")
        await { library.ui.value.programs!!.rows.map { it.name } == listOf("Second", "First") }
    }

    @Test
    fun aFailedSaveIsRetried() {
        val dir = temp.newFolder("libraries")
        val id = Libraries.open(dir.path).use { l -> l.create("Gig", XD, null).also { id -> l.read(id).use { d -> d.newProgram(null); l.save(id, d) } } }
        val library = make(LibraryModel::class) { LibraryModel(dir.path) }
        compose.setContent { OscylTheme(OscylThemeMode.Dark) { androidx.compose.foundation.layout.Box {
                LibraryScreen(library, onOpenSynth = {})
                io.github.oscyl.ui.library.NoticeHost(library.ui.collectAsState().value, library)
            } } }
        await { library.ui.value.lines.any { it.id == id } }
        compose.onNodeWithText("Gig").performClick()
        await { library.ui.value.programs?.rows?.isNotEmpty() == true }
        // The folder refuses writes for a while.
        dir.walk().forEach { it.setWritable(false) }
        try {
            compose.runOnIdle { library.renameProgram(0, "Kept") }
            await { library.ui.value.notice is Notice.SaveFailed }
            assertTrue(id in library.ui.value.unsaved)
        } finally {
            dir.walk().forEach { it.setWritable(true) }
        }
        compose.onNodeWithText("Retry").performClick()
        await { id !in library.ui.value.unsaved }
        val saved = Libraries.open(dir.path).use { l -> l.read(id).use { it.name(0u) } }
        assertEquals("Kept", saved)
    }

    @Test
    fun aFilePickedAsTheProcessWentGoesWhereItWasAskedFor() {
        val fixture = Libraries.open(temp.newFolder().path).use { l ->
            val id = l.create("Fixture", XD, null)
            l.read(id).use { d -> d.newProgram(null); exportFile(d, "oscyllib") }
        }
        // As the platform keeps them: what was asked, and an export whose place came too late.
        class Orphans(var picked: PickedFile?, val asked: PickFor) : Files {
            var lost: String? = "Gig.mnlgxdlib"
            override suspend fun open(purpose: PickFor): PickedFile? = null
            override suspend fun create(name: String, mime: String): Sink? = null
            override fun share(name: String, bytes: ByteArray, mime: String) = Unit
            override suspend fun orphan(purpose: PickFor): PickedFile? = picked.takeIf { purpose == asked }.also { if (it != null) picked = null }
            override suspend fun lostExport(): String? = lost.also { lost = null }
        }
        val forLibrary = Orphans(PickedFile("Picked.oscyllib", fixture), PickFor.Library)
        val library = make(LibraryModel::class) { LibraryModel(temp.newFolder("libraries").path, files = forLibrary) }
        compose.setContent {}
        await { library.ui.value.lines.any { it.name == "Picked" } }
        await { library.ui.value.notice == Notice.ExportLost("Gig.mnlgxdlib") }
        // A unit picked for a slot is not imported as a library: the synth model says so.
        val forUnit = Orphans(PickedFile("Tiny.mnlgxdunit", unitFile("Tiny")), PickFor.Unit)
        val synth = make(SynthModel::class) { SynthModel(NoMidi, files = forUnit) }
        await { synth.notice.value?.contains("Tiny.mnlgxdunit") == true }
        assertEquals(null, forUnit.picked)
    }

    private companion object {
        const val TIMEOUT_MS = 30_000L
        const val XD = "korg-minilogue-xd"
        const val REV2 = "sequential-prophet-rev2"
        val USB_XD = MidiPortInfo(name = "minilogue xd _ SOUND", id = "7")
    }
}

/** Run the custom accessibility action called [label] on the row this text sits in. */
private fun androidx.compose.ui.test.SemanticsNodeInteraction.performSemanticsActionOnRow(label: String) {
    val node = fetchSemanticsNode()
    var n: androidx.compose.ui.semantics.SemanticsNode? = node
    while (n != null) {
        val actions = n.config.getOrElseNullable(SemanticsActions.CustomActions) { null }
        val action = actions?.firstOrNull { it.label == label }
        if (action != null) {
            action.action()
            return
        }
        n = n.parent
    }
    error("no action $label")
}
