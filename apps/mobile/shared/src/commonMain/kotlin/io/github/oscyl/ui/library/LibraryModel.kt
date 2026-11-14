package io.github.oscyl.ui.library

import io.github.oscyl.core.auditionTarget
import io.github.oscyl.core.SynthRef
import io.github.oscyl.core.GraphShape
import io.github.oscyl.core.HistoryStep
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.oscyl.shared.generated.resources.library_demo
import io.github.oscyl.core.Clip
import io.github.oscyl.core.Document
import io.github.oscyl.core.FfiException
import io.github.oscyl.core.Libraries
import io.github.oscyl.core.NodeKind
import io.github.oscyl.core.OutlineLine
import io.github.oscyl.core.ProgramAction
import io.github.oscyl.core.ProgramMenu
import io.github.oscyl.core.ProgramRow
import io.github.oscyl.core.ReorderHow
import io.github.oscyl.core.Trashed
import io.github.oscyl.core.FileFormatInfo
import io.github.oscyl.core.drivers
import io.github.oscyl.core.BackupFile
import io.github.oscyl.core.forgetUnsynced
import io.github.oscyl.core.keepUnsynced
import io.github.oscyl.core.newLibraryDriver
import io.github.oscyl.core.readFile as readBackup
import io.github.oscyl.core.unsynced
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import io.github.oscyl.core.exportFile
import io.github.oscyl.core.exportFormats as formatsOf
import io.github.oscyl.core.importFile as readFile
import io.github.oscyl.core.programMenu
import io.github.oscyl.platform.FileUnreadable
import io.github.oscyl.platform.FileUnwritable
import io.github.oscyl.platform.Files
import io.github.oscyl.platform.PickFor
import io.github.oscyl.platform.PickedFile
import io.github.oscyl.platform.NoFiles
import io.github.oscyl.platform.safeFileName
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.file_library
import io.github.oscyl.shared.generated.resources.library_disconnected
import io.github.oscyl.ui.synth.KeptEdits
import io.github.oscyl.ui.synth.unsyncedKey
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.NonCancellable
import org.jetbrains.compose.resources.getString
import io.github.oscyl.ui.synth.ReadProgress
import io.github.oscyl.ui.synth.SynthCard
import io.github.oscyl.ui.synth.SynthModel
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** The program list of one source. */
data class ProgramsUi(
    /** Its key ([librarySource], [synthSource]). */
    val source: String,
    val title: String,
    val rows: List<ProgramRow>,
    val selected: Int?,
    /** One of Oscyl's libraries (programs can be added and removed), not the synth. */
    val inLibrary: Boolean,
    val canUndo: Boolean,
    val canRedo: Boolean,
    /** A synth still being read. */
    val read: ReadProgress? = null,
    /** The document's revision: a program's picture is asked again when it changes. */
    val revision: ULong = 0u,
    /** A connected synth plays this document's programs: a swipe on a row auditions it. */
    val canAudition: Boolean = false,
)

/** Everything the Library destination shows. */
data class LibraryUi(
    /** Connected synths. */
    val synths: List<SynthCard> = emptyList(),
    /** Oscyl's libraries as a tree, depth first. */
    val lines: List<OutlineLine> = emptyList(),
    /** Libraries whose last save failed. */
    val unsaved: Set<String> = emptySet(),
    val programs: ProgramsUi? = null,
    val notice: Notice? = null,
    /** A source to bring into view (a library just imported); the screen calls [LibraryActions.revealed]. */
    val reveal: String? = null,
)

/** What a notice on the island says; [undoable] ones offer Undo. */
sealed interface Notice {
    val undoable: Boolean get() = false

    /** The program it is about, which its Open brings into the editor: its source and index. */
    val opens: Pair<String, Int>? get() = null

    data class Removed(val name: String) : Notice { override val undoable get() = true }
    data class Cleared(val slot: String) : Notice { override val undoable get() = true }
    data class Pasted(val slot: String) : Notice { override val undoable get() = true }
    data class Deleted(val name: String) : Notice { override val undoable get() = true }
    data class Copied(val name: String) : Notice
    data class CopiedTo(val library: String) : Notice
    data class Full(val library: String, val max: Int) : Notice
    data class Exported(val library: String) : Notice
    data class ExportLost(val file: String) : Notice

    data class CannotRead(val name: String) : Notice
    data class CannotWrite(val name: String) : Notice

    /** The core's words (a file, the folder, a refused edit). */
    data class Failed(val reason: String) : Notice

    /** A library's edits could not be written: they stay in memory, marked, and Retry writes them again. */
    data class SaveFailed(val library: String, val reason: String) : Notice

    /** A new program, at [slot] ([index] of [source]). */
    data class Added(val slot: String, val source: String, val index: Int) : Notice {
        override val undoable get() = true
        override val opens get() = source to index
    }

    /** A generated sound kept at the end of [library] ([index] of [source]). */
    data class Kept(val library: String, val source: String, val index: Int) : Notice {
        override val opens get() = source to index
    }

    /** A stage rolled by the dice. */
    data class Rolled(val stage: String) : Notice { override val undoable get() = true }

    /** A snapshot stored over another. */
    data class SnapshotReplaced(val letter: String) : Notice { override val undoable get() = true }

    /** A tap on a stage's dice: it rolls on a hold. */
    data class RollHint(val stage: String) : Notice

    /** A library's edits written from the Save key: [count] programs. The first time, it adds that a hold [review]s first. */
    data class Saved(val count: Int, val review: Boolean = false) : Notice

    /** A plain tap on the stage rail: it can be glided along. */
    data object RailHint : Notice

    /** A tap on Compare that was too short to hear anything: how it works. */
    data object CompareHint : Notice
}

/**
 * The program the editor shows: the selected program of the source shown, with its
 * document, and the document's [revision] when it was last shown (so a change made
 * elsewhere, such as an undo or a revert from the program menu, reaches the editor).
 */
data class OpenProgram(
    val source: String,
    val document: Document,
    val index: Int,
    /** One of Oscyl's libraries (saved after every change), not a synth's slots. */
    val inLibrary: Boolean,
    /** The connected synth whose slots these are; null for a library. */
    val synthId: ULong?,
    val revision: ULong,
    /** Edited programs: keeping the edits changes this, not the revision. */
    val edited: UInt = 0u,
    /** Changes when programs move, come or go: the program at [index] may be another one. */
    val arranged: Int = 0,
)

/** The source shown, whose edits the island's Save keeps. */
data class ShownSource(
    val source: String,
    val document: Document,
    val inLibrary: Boolean,
    val synthId: ULong?,
    val revision: ULong,
    val edited: UInt,
)

/** The key of library [id] as a source. */
fun librarySource(id: String) = "library:$id"

/** The key of connected synth [id] as a source. */
fun synthSource(id: ULong) = "synth:$id"

/** What the Library destination can ask for; every default does nothing (previews). */
interface LibraryActions {
    fun select(source: String?) {}
    fun selectProgram(index: Int) {}
    fun newLibrary(parent: String? = null) {}
    fun newFolder(parent: String? = null) {}
    fun setOpen(id: String, open: Boolean) {}
    fun renameNode(id: String, name: String) {}
    fun duplicateLibrary(id: String, name: String) {}
    fun deleteNode(id: String) {}
    fun stepNode(id: String, by: Int) {}
    fun moveNode(id: String, parent: String?) {}
    fun undo() {}
    fun redo() {}

    /** The shown document's history, oldest step first, and a jump to where [done] of its steps are done. */
    fun history(): List<HistoryStep> = emptyList()
    fun jumpHistory(done: Int) {}
    fun newProgram() {}

    /** Stop the read of the synth shown. */
    fun stopRead() {}

    /** Where each new program (or duplicate) went: the list scrolls to it. */
    val placed: kotlinx.coroutines.flow.Flow<Int> get() = kotlinx.coroutines.flow.emptyFlow()

    /** Play program [index] on the synth that plays this document, without opening it. */
    fun audition(index: Int) {}

    /** Program [index]'s first picture, for its row; null before it is read or when it has none. */
    fun thumbnail(index: Int): GraphShape? = null

    /** Program [index]'s menu, as the core decides it. */
    suspend fun menu(index: Int): ProgramMenu? = null
    fun act(index: Int, action: ProgramAction) {}

    /** Program [from] dragged to [to] (the ones between shift by one): one undo step. */
    fun move(from: Int, to: Int) {}
    fun renameProgram(index: Int, name: String) {}

    /** Copy program [index] to library [id] (a new one when null). */
    fun copyTo(index: Int, id: String?) {}
    /** Pick a file and keep it as a new library. */
    fun importFile() {}

    /** Library [id] as a file in format [ext], saved where the person picks. */
    fun export(id: String, ext: String) {}

    /** Library [id] in its synth's usual format, to another app. */
    fun share(id: String) {}

    /** The formats library [id] exports to (the usual one first). */
    fun exportFormats(id: String): List<FileFormatInfo> = emptyList()

    /** A backup file as a new library. */
    fun openBackup(file: BackupFile) {}

    /** A backup file, to another app. */
    fun shareBackup(file: BackupFile) {}

    /** The screen has shown [ProgramsUi]'s new source [LibraryUi.reveal]. */
    fun revealed() {}
    fun undoNotice() {}

    /** The program the notice is about, selected and open in the editor. */
    fun openNotice() {}

    /** Write again the libraries whose save failed. */
    fun retrySave() {}
    fun dismissNotice() {}
}

/** Does nothing: previews. */
object NoLibraryActions : LibraryActions

/**
 * Oscyl's libraries in [dir] (app-private storage) and the synths' slots, for the
 * Library destination. Every rule (what a menu offers, what an edit may do, how files
 * are kept) is the core's; this model only calls it, keeps the folder's work on one
 * background thread, and saves a library after each change, as the desktop does.
 */
class LibraryModel(
    private val dir: String?,
    private val synth: SynthModel? = null,
    private val files: Files = NoFiles,
    /** Where connected synths' unsynced edits are kept between runs; null: not kept (previews). */
    private val unsyncedDir: String? = null,
    /** The folder's one background thread: every core call that reads or writes it (tests pass their own). */
    private val io: CoroutineDispatcher = Dispatchers.IO.limitedParallelism(1),
) : ViewModel(), LibraryActions {
    private val _ui = MutableStateFlow(LibraryUi())
    val ui: StateFlow<LibraryUi> = _ui.asStateFlow()

    private val _open = MutableStateFlow<OpenProgram?>(null)

    /** The program open in the editor, if any. */
    val open: StateFlow<OpenProgram?> = _open.asStateFlow()

    private val _shown = MutableStateFlow<ShownSource?>(null)

    /** The source shown, if any. */
    val shown: StateFlow<ShownSource?> = _shown.asStateFlow()

    /** By synth: the library this run made for kept generated sounds. */
    private val keptInto = mutableMapOf<String, String>()

    /** The unsynced-edits files this model writes (its run's), which recovery leaves alone. */
    private val written = mutableSetOf<String>()
    private var libraries: Libraries? = null

    /** Completes once the folder has opened (with null when it could not). */
    private val ready = CompletableDeferred<Libraries?>()
    private val docs = mutableMapOf<String, Document>()
    private var source: String? = null
    private var selected: Int? = null

    // The program open in the editor: its source and place, apart from the source the list
    // shows. Looking around the Library (the sources, another library) never closes it.
    private var openSource: String? = null
    private var openIndex: Int? = null
    private var clipboard: Clip? = null

    /** What the notice's Undo takes back: a deletion, or the last step in a source (its key). */
    private var trashed: Trashed? = null
    private var undoIn: String? = null

    init {
        viewModelScope.launch {
            try {
                libraries = dir?.let { withContext(io) { Libraries.open(it) } }
                ready.complete(libraries)
                refreshOutline()
                // A source restored after the process went away waits for the folder.
                source?.let { select(it) }
                recoverUnsynced()
                // A file picked to import while the system ended the process: imported now.
                importPicked { files.orphan(PickFor.Library) }
                // An export whose place was chosen after the process went: nothing was written.
                files.lostExport()?.let { notify(Notice.ExportLost(it)) }
                // A file another app opens in Oscyl becomes a library, shown.
                launch { files.opened.collect { read -> importPicked { read() } } }
            } catch (e: FfiException) {
                ready.complete(null)
                notify(Notice.Failed(e.reason()))
            }
        }
        synth?.let { s ->
            viewModelScope.launch {
                // Read progress moves the synth's card and the list's progress line only.
                s.cards.map { cards -> cards.filter { it.connected } }.distinctUntilChanged().collect { cards ->
                    val ids = cards.map { it.id }
                    _ui.update { ui ->
                        ui.copy(
                            synths = cards,
                            programs = ui.programs?.let { p -> p.copy(read = cards.firstOrNull { synthSource(it.id) == p.source }?.read) },
                        )
                    }
                    if (ids != connected) {
                        connected = ids
                        // A synth's list follows its read; any list follows whether a swipe can play it.
                        refreshPrograms()
                    }
                }
            }
            // A whole read, a write or a synth gone: the list again, and what is kept of its edits.
            viewModelScope.launch {
                s.programs.collect {
                    if (source?.startsWith(SYNTH) == true) refreshPrograms()
                    saveUnsynced()
                }
            }
            // One slot read: its row alone.
            viewModelScope.launch {
                s.received.collect { (id, i) ->
                    if (source == synthSource(id)) patchRow(i)
                }
            }
            viewModelScope.launch { for (edits in s.kept) keep(edits) }
        }
    }

    /** The connected synths the list last saw. */
    private var connected = emptyList<ULong>()

    /** Row [i] of the source shown, from the core, without rebuilding the others. */
    private fun patchRow(i: UInt) {
        val (_, doc) = current() ?: return
        val row = doc.row(i) ?: return
        _ui.update { ui ->
            val p = ui.programs ?: return@update ui
            val index = i.toInt()
            if (index >= p.rows.size) ui else ui.copy(programs = p.copy(rows = p.rows.toMutableList().also { it[index] = row }))
        }
    }

    /** Every connected synth's unsynced edits, kept on disk so they outlive the process. */
    private fun saveUnsynced() {
        val dir = unsyncedDir ?: return
        val s = synth ?: return
        // What is kept is taken now, on this thread: a document closed later (the app going) can't lose it.
        val kept = s.cards.value.filter { it.connected }.mapNotNull { card -> s.document(card.id)?.let { unsyncedKey(card) to it.offline("") } }
        written += kept.map { it.first }
        viewModelScope.launch {
            withContext(io + NonCancellable) {
                for ((key, doc) in kept) {
                    try {
                        keepUnsynced(dir, key, doc)
                    } catch (_: FfiException) {
                        // ponytail: the next change tries again; the edits are still in memory.
                    } finally {
                        doc?.close()
                    }
                }
            }
        }
    }

    /** A previous run's unsynced edits: kept as libraries, then forgotten. */
    private suspend fun recoverUnsynced() {
        val dir = unsyncedDir ?: return
        val s = ready.await() ?: return
        val models = drivers().associate { it.id to it.model }
        // Only earlier runs' files: this run writes its own (unsyncedKey), never theirs.
        for (file in withContext(io) { unsynced(dir) }.filterNot { it.name.substringBeforeLast('.') in written }) {
            attempt {
                withContext(io + NonCancellable) {
                    val imported = readFile(file.name, readBackup(file.path))
                    val title = getString(Res.string.library_disconnected, models[imported.driverId] ?: imported.driverId)
                    imported.document.use { s.add(title, it, null) }
                    forgetUnsynced(dir, file.name.substringBeforeLast('.'))
                }
            }
        }
        refreshOutline()
    }

    /** A gone synth's edits become a new library, saved even if the app is closing. */
    private suspend fun keep(edits: KeptEdits) {
        val s = ready.await()
        if (s == null) {
            edits.document.close()
            return
        }
        val id = attempt { withContext(io + NonCancellable) { edits.document.use { s.add(edits.title, it, null) } } } ?: return
        // Kept as a library now: the copy kept for the next run goes.
        edits.key?.let { key -> unsyncedDir?.let { d -> withContext(io + NonCancellable) { forgetUnsynced(d, key) } } }
        refreshOutline()
        if (edits.reveal) _ui.update { it.copy(reveal = librarySource(id)) }
    }

    /** A backup file as a new library, shown at once. */
    override fun openBackup(file: BackupFile) {
        viewModelScope.launch {
            val s = ready.await() ?: return@launch
            val id = attempt {
                withContext(io) {
                    val imported = readFile(file.name, readBackup(file.path))
                    imported.document.use { s.add(imported.name, it, null) }
                }
            } ?: return@launch
            refreshOutline()
            _ui.update { it.copy(reveal = librarySource(id)) }
        }
    }

    /** A backup file, to another app. */
    override fun shareBackup(file: BackupFile) {
        viewModelScope.launch {
            attempt { withContext(io) { files.share(file.name, readBackup(file.path), MIME) } }
        }
    }

    // Sources

    override fun select(source: String?) {
        // Back on the open program's source, the list shows it selected again.
        if (source != this.source) selected = if (source != null && source == openSource) openIndex else null
        this.source = source
        val id = source?.removePrefix(LIBRARY)?.takeIf { source.startsWith(LIBRARY) }
        if (id == null || id in docs) return refreshPrograms()
        refreshPrograms()
        val store = libraries ?: return
        viewModelScope.launch {
            try {
                val doc = withContext(io) { store.read(id) }
                // A second read of the same library never replaces the one being edited.
                docs.keepFirst(id, doc)
                refreshPrograms()
            } catch (e: FfiException) {
                notify(Notice.Failed(e.reason()))
            }
        }
    }

    override fun newLibrary(parent: String?) {
        // The core picks the synth, as on desktop.
        val driver = newLibraryDriver(_ui.value.synths.map { it.driverId }, current()?.second?.driverId())
        store { s: Libraries -> s.create(s.newLibraryName(), driver, parent) }
    }

    override fun newFolder(parent: String?) = store { s -> s.newFolder(s.newFolderName(), parent) }

    override fun setOpen(id: String, open: Boolean) = store { s -> s.setOpen(id, open) }

    override fun renameNode(id: String, name: String) = store { s -> s.rename(id, name) }

    override fun duplicateLibrary(id: String, name: String) = store { s ->
        // Its last edits are on disk already (saved after every change).
        s.duplicate(id, name)
    }

    override fun stepNode(id: String, by: Int) = store { s -> s.step(id, by) }

    override fun moveNode(id: String, parent: String?) = store { s -> s.place(id, parent, UInt.MAX_VALUE) }

    override fun deleteNode(id: String) {
        val name = _ui.value.lines.firstOrNull { it.id == id }?.name ?: return
        store({ s -> s.delete(id) }) { t ->
            notify(Notice.Deleted(name))
            trashed = t
        }
    }

    /** Run [change] on the folder off the main thread, show the tree again, then [then] with its result. */
    private fun <T> store(change: (Libraries) -> T, then: (T) -> Unit) {
        viewModelScope.launch {
            // A tap before the folder has opened waits for it.
            val s = ready.await() ?: return@launch
            try {
                val result = withContext(io) { change(s) }
                refreshOutline()
                then(result)
            } catch (e: FfiException) {
                notify(Notice.Failed(e.reason()))
                refreshOutline()
            }
        }
    }

    private fun store(change: (Libraries) -> Any?) = store(change) {}

    private suspend fun refreshOutline() {
        val s = libraries ?: return
        val lines = withContext(io) { s.outline() }
        _ui.update { it.copy(lines = lines) }
        // A library that went (deleted, or inside a deleted folder) leaves the list.
        val gone = source?.takeIf { it.startsWith(LIBRARY) }?.removePrefix(LIBRARY)?.takeIf { id -> lines.none { it.id == id } }
        if (gone != null) {
            source = null
            selected = null
            if (openSource == LIBRARY + gone) {
                openSource = null
                openIndex = null
            }
        }
        refreshPrograms()
    }

    // Programs

    /** The source shown: its key in the store (null for a synth) and its document. */
    private fun current(): Pair<String?, Document>? = docOf(source)

    /** Source [key]'s key in the store (null for a synth) and its document. */
    private fun docOf(key: String?): Pair<String?, Document>? {
        key ?: return null
        return if (key.startsWith(LIBRARY)) {
            val id = key.removePrefix(LIBRARY)
            docs[id]?.let { id to it }
        } else {
            key.removePrefix(SYNTH).toULongOrNull()?.let { synth?.document(it) }?.let { null to it }
        }
    }

    /** The program open in the editor, from its own source (whatever the list shows). */
    private fun refreshOpen() {
        val key = openSource
        val (id, doc) = docOf(key) ?: run {
            // Its source went (a synth unplugged, a library deleted): nothing is open.
            openSource = null
            openIndex = null
            _open.value = null
            return
        }
        val synthId = key!!.removePrefix(SYNTH).toULongOrNull()?.takeIf { id == null }
        _open.value = openIndex?.takeIf { it < doc.len().toInt() }?.let { i ->
            OpenProgram(key, doc, i, id != null, synthId, doc.revision(), doc.editedCount(), arranged)
        }
    }

    private fun refreshPrograms() {
        val key = source
        // The list and the open program are one where the list shows the open program's source.
        if (key != null && key == openSource) openIndex = selected
        refreshOpen()
        val (id, doc) = current() ?: run {
            _shown.value = null
            return _ui.update { it.copy(programs = null) }
        }
        val rows = doc.rows()
        val synthId = key!!.removePrefix(SYNTH).toULongOrNull()?.takeIf { id == null }
        val revision = doc.revision()
        val edited = doc.editedCount()
        _shown.value = ShownSource(key, doc, id != null, synthId, revision, edited)
        val card = if (id == null) _ui.value.synths.firstOrNull { synthSource(it.id) == key } else null
        _ui.update {
            it.copy(
                programs = ProgramsUi(
                    source = key!!,
                    title = id?.let { lib -> it.lines.firstOrNull { l -> l.id == lib }?.name } ?: card?.model ?: doc.title(),
                    rows = rows,
                    selected = selected?.takeIf { s -> s < rows.size },
                    inLibrary = id != null,
                    canUndo = doc.canUndo(),
                    canRedo = doc.canRedo(),
                    read = card?.read,
                    revision = revision,
                    canAudition = auditionTarget(_ui.value.synths.map { s -> SynthRef(s.id, s.driverId) }, synthId, doc.driverId()) != null,
                ),
            )
        }
    }

    // Pictures by (revision, index): an edit asks again, a scroll does not. The core draws one in about a microsecond.
    // ponytail: first in, first out (common Kotlin has no access-ordered map); an LRU if rows thrash.
    private val thumbnails = LinkedHashMap<Pair<ULong, Int>, GraphShape?>()

    private fun trimThumbnails() {
        while (thumbnails.size > 600) thumbnails.remove(thumbnails.keys.first())
    }

    override fun audition(index: Int) {
        val shown = _shown.value ?: return
        val bytes = shown.document.program(index.toUInt()) ?: return
        val target = auditionTarget(_ui.value.synths.map { SynthRef(it.id, it.driverId) }, shown.synthId, shown.document.driverId()) ?: return
        synth?.audition(target, bytes)
    }

    override fun stopRead() {
        val id = source?.takeIf { it.startsWith(SYNTH) }?.removePrefix(SYNTH)?.toULongOrNull() ?: return
        _ui.value.synths.firstOrNull { it.id == id }?.let { synth?.cancelRead(it) }
    }

    // The document the cached pictures are of: revisions are counted per document.
    private var thumbnailsOf: Document? = null

    override fun thumbnail(index: Int): GraphShape? {
        val shown = _shown.value ?: return null
        if (thumbnailsOf !== shown.document) {
            thumbnails.clear()
            thumbnailsOf = shown.document
        }
        return thumbnails.getOrPut(shown.revision to index) { shown.document.thumbnail(index.toUInt(), 48u) }.also { trimThumbnails() }
    }

    override fun selectProgram(index: Int) {
        selected = index
        openSource = source
        openIndex = index
        refreshPrograms()
    }

    /**
     * Change the open program's document with [change] (the editor's edits, its undo and
     * redo, its history, a kept generated sound): whatever the list shows. [change] gets
     * the document and returns where the open program now stands (null: where it was).
     */
    private fun editOpen(change: (Document) -> Int?) {
        val key = openSource ?: return
        val (id, doc) = docOf(key) ?: return
        val count = doc.len()
        if (undoIn != null) {
            undoIn = null
            _ui.update { it.copy(notice = null) }
        }
        change(doc)?.let { at ->
            openIndex = at
            if (source == key) selected = at
        }
        refreshPrograms()
        if (id != null) save(id, doc, outline = doc.len() != count) else saveUnsynced()
    }

    /** Change the document shown with [change], show it, and save it if it is a library. */
    private fun edit(change: (Document) -> Unit) {
        val (id, doc) = current() ?: return
        val count = doc.len()
        // A later change makes an Undo notice stale (its Undo would take back this one), and
        // an Open one too (its program may stand elsewhere now).
        if (undoIn != null || _ui.value.notice?.opens != null) {
            undoIn = null
            _ui.update { it.copy(notice = null) }
        }
        change(doc)
        refreshPrograms()
        // Every change to a connected synth's slots is kept on disk at once (the one place all go through).
        if (id != null) save(id, doc, outline = doc.len() != count) else saveUnsynced()
    }

    /** Write library [id]; a failure is said once, or every time when [retry]ing. */
    private fun save(id: String, doc: Document, outline: Boolean = false, retry: Boolean = false) {
        val s = libraries ?: return
        viewModelScope.launch {
            try {
                // Queued saves finish even when the model goes.
                withContext(io + NonCancellable) { s.save(id, doc) }
                _ui.update { it.copy(unsaved = it.unsaved - id) }
                if (outline) refreshOutline()
            } catch (e: FfiException) {
                if (retry || id !in _ui.value.unsaved) notify(Notice.SaveFailed(nameOf(id), e.reason()))
                _ui.update { it.copy(unsaved = it.unsaved + id) }
            }
        }
    }

    /**
     * Put generated [program] over the program open (one undo step, saved like any
     * edit); returns its slot, or null when it can't go there (said in a notice).
     */
    fun keepOverOpen(program: ByteArray): String? {
        val open = _open.value ?: return null
        var slot: String? = null
        editOpen { doc ->
            try {
                doc.put(open.index.toUInt(), program)
                slot = doc.slotLabel(open.index.toUInt())
            } catch (e: FfiException) {
                notify(Notice.Failed(e.reason()))
            }
            null
        }
        return slot
    }

    /**
     * Keep generated [program] of [driver]'s synth at the end of the library shown when
     * it is one of that synth's, else of a new library; returns that library's name.
     */
    suspend fun keepInLibrary(driver: String, program: ByteArray): String? {
        val s = ready.await() ?: return null
        val shown = source?.takeIf { it.startsWith(LIBRARY) }?.removePrefix(LIBRARY)
            ?.takeIf { id -> _ui.value.lines.firstOrNull { it.id == id }?.driverId == driver }
        // Else the library this run made for kept sounds of that synth (one gone since: a new one).
        val made = keptInto[driver]?.takeIf { id -> docs[id] != null || withContext(io) { runCatching { s.read(id).close() }.isSuccess } }
        var name: String? = null
        val id = shown ?: made ?: attempt {
            withContext(io) { s.newLibraryName().also { name = it }.let { n -> s.create(n, driver, null) } }
        }?.also { keptInto[driver] = it } ?: return null
        val doc = docs[id] ?: attempt { withContext(io) { s.read(id) } }?.let { docs.keepFirst(id, it); docs[id] } ?: return null
        attempt { doc.put(null, program) } ?: return null
        // Saved like any edit: a failure is marked, and Retry covers it.
        save(id, doc, outline = true)
        if (source == LIBRARY + id) refreshPrograms()
        return (name ?: nameOf(id)).also { notify(Notice.Kept(it, LIBRARY + id, doc.len().toInt() - 1)) }
    }

    /** Put program [index] of the source shown back as saved (one undo step). */
    fun revert(index: Int) = edit { it.revertProgram(index.toUInt()) }

    /**
     * Keep the edits of the library shown: write it, then mark every program saved. A
     * write that fails keeps the edits marked; returns why, or null when kept.
     */
    suspend fun keepEdits(): String? {
        val (id, doc) = current() ?: return null
        id ?: return null
        val s = ready.await() ?: return null
        return try {
            withContext(io + NonCancellable) { s.save(id, doc) }
            doc.markSaved()
            _ui.update { it.copy(unsaved = it.unsaved - id) }
            refreshPrograms()
            null
        } catch (e: FfiException) {
            _ui.update { it.copy(unsaved = it.unsaved + id) }
            e.reason()
        }
    }

    /** The editor changed the open program (a gesture ended): show it, save it if it is a library's, keep a synth's. */
    fun editorChanged() {
        val before = _shown.value
        editOpen { null }
        // One program changed: the pictures of the others stand at the new revision too. Asked
        // again, every row of the list (composed under the editor) called the core at each
        // knob's release.
        val now = _shown.value ?: return
        if (before != null && before.document === now.document && thumbnailsOf === now.document && before.revision != now.revision && now.source == openSource) {
            thumbnails.entries.filter { it.key.first == before.revision && it.key.second != openIndex }
                .forEach { thumbnails[now.revision to it.key.second] = it.value }
            trimThumbnails()
        }
    }

    /** The editor's undo and redo: the open program's document, whatever the list shows. */
    fun undoOpen() = editOpen { doc -> var at: Int? = null; watchArrangement(doc) { at = doc.undo()?.toInt() }; at }

    fun redoOpen() = editOpen { doc -> var at: Int? = null; watchArrangement(doc) { at = doc.redo()?.toInt() }; at }

    /** The open program's document's history (the editor's history sheet). */
    fun openHistory(): List<HistoryStep> = docOf(openSource)?.second?.history().orEmpty()

    fun jumpOpenHistory(done: Int) = editOpen { doc -> var at: Int? = null; watchArrangement(doc) { at = doc.jumpHistory(done.toUInt())?.toInt() }; at }

    override fun undo() = edit { doc -> watchArrangement(doc) { doc.undo()?.let { selected = it.toInt() } } }

    override fun redo() = edit { doc -> watchArrangement(doc) { doc.redo()?.let { selected = it.toInt() } } }

    /**
     * Runs [step] (an undo or a redo) and counts a new arrangement when it brought programs
     * back, took them away or moved them: the open position may hold another program.
     * A rename taken back changes one name, not the order, and counts nothing.
     */
    private fun watchArrangement(doc: Document, step: () -> Unit) {
        val before = doc.rows().map { it.name }
        step()
        val after = doc.rows().map { it.name }
        if (before.size != after.size || (before != after && before.sorted() == after.sorted())) arranged++
    }

    override fun history(): List<HistoryStep> = current()?.second?.history().orEmpty()

    // Many undos or redos as one move: one refresh, one save.
    override fun jumpHistory(done: Int) = edit { doc -> watchArrangement(doc) { doc.jumpHistory(done.toUInt())?.let { selected = it.toInt() } } }

    override fun newProgram() {
        val at = selected
        edit { doc -> place(doc, doc.newProgram(at?.toUInt())) }
    }

    /** A new program in the library shown, opened in the editor. */
    fun newAndOpen() {
        val before = selected
        newProgram()
        selected?.takeIf { it != before }?.let(::selectProgram)
    }

    private val _placed = kotlinx.coroutines.flow.MutableSharedFlow<Int>(extraBufferCapacity = 1)
    override val placed: kotlinx.coroutines.flow.Flow<Int> = _placed

    /** Show [at] selected, the list taken to it and a word with Undo; or say the library is full. */
    private fun place(doc: Document, at: UInt?) {
        if (at == null) return notify(Notice.Full(_ui.value.programs?.title ?: doc.title(), doc.maxPrograms().toInt()))
        selected = at.toInt()
        _placed.tryEmit(at.toInt())
        notify(Notice.Added(doc.slotLabel(at), source ?: "", at.toInt()), undo = source)
    }

    override suspend fun menu(index: Int): ProgramMenu? {
        val (id, doc) = current() ?: return null
        val clip = clipboard
        // It reads the libraries' outline: off the main thread.
        return withContext(io) { programMenu(doc, index.toUInt(), libraries, id, clip) }
    }

    // Counts each time the open position comes to hold another program (a move into it, a
    // removal or an insertion before it, an undo or redo of one): the editor opens it afresh.
    private var arranged = 0

    /** Where the selected program stands after the program at [from] went to [to] (a neighbour's swap). */
    private fun follow(selected: Int?, from: Int, to: Int): Int? = when (selected) {
        from -> to
        to -> from
        else -> selected
    }

    override fun act(index: Int, action: ProgramAction) {
        // Only the list of the open program's source can move it.
        val open = if (source == openSource) selected else null
        val reaches = when (action) {
            // The moved program lands on the neighbour's place, the neighbour on its own.
            ProgramAction.MOVE_UP -> open == index - 1
            ProgramAction.MOVE_DOWN -> open == index + 1
            ProgramAction.REMOVE -> open != null && open >= index
            ProgramAction.DUPLICATE, ProgramAction.NEW_PROGRAM -> open != null && open > index
            else -> false
        }
        if (reaches) arranged++
        val (id, doc) = current() ?: return
        val i = index.toUInt()
        val slot = doc.slotLabel(i)
        val name = doc.name(i)
        val key = source ?: return
        when (action) {
            ProgramAction.COPY -> {
                clipboard?.close()
                clipboard = doc.copy(listOf(i))
                notify(Notice.Copied(name))
            }
            ProgramAction.PASTE_OVER -> clipboard?.let { clip ->
                edit { if (it.pasteOver(i, clip)) notify(Notice.Pasted(slot), undo = key) }
            }
            ProgramAction.DUPLICATE -> edit { doc ->
                val pasted = doc.copy(listOf(i)).use { clip -> doc.paste(i + 1u, clip) }
                place(doc, if (pasted) i + 1u else null)
            }
            ProgramAction.NEW_PROGRAM -> edit { place(it, it.newProgram(i)) }
            // The selection follows its own program: the moved one, or the neighbour it passed.
            ProgramAction.MOVE_UP -> edit { if (it.reorder(i, i - 1u, ReorderHow.MOVE) > 0u) selected = follow(selected, index, index - 1) }
            ProgramAction.MOVE_DOWN -> edit { if (it.reorder(i, i + 1u, ReorderHow.MOVE) > 0u) selected = follow(selected, index, index + 1) }
            ProgramAction.REVERT -> edit { it.revertProgram(i) }
            ProgramAction.REMOVE -> edit {
                it.remove(i)
                notify(Notice.Removed(name), undo = key)
            }
            ProgramAction.CLEAR -> edit {
                it.clearProgram(i)
                notify(Notice.Cleared(slot), undo = key)
            }
            // The screen asks for a name or a library first.
            ProgramAction.RENAME, ProgramAction.COPY_TO_LIBRARY -> Unit
        }
    }

    override fun move(from: Int, to: Int) {
        if (from == to) return
        // Only the list of the open program's source can move it; anything between the ends shifts.
        val open = if (source == openSource) selected else null
        edit { doc ->
            if (doc.reorder(from.toUInt(), to.toUInt(), ReorderHow.MOVE) > 0u) {
                if (open != null && open in minOf(from, to)..maxOf(from, to)) arranged++
                selected = selected?.let { sel ->
                    when {
                        sel == from -> to
                        from < to && sel in from + 1..to -> sel - 1
                        from > to && sel in to until from -> sel + 1
                        else -> sel
                    }
                }
            }
        }
    }

    override fun renameProgram(index: Int, name: String) = edit { doc ->
        try {
            doc.setName(index.toUInt(), name)
        } catch (e: FfiException) {
            notify(Notice.Failed(e.reason()))
        }
    }

    override fun copyTo(index: Int, id: String?) {
        val (_, doc) = current() ?: return
        copyFrom(doc, index, id)
    }

    /** The program open in the editor, copied into a new library (a factory slot's way to change it). */
    fun copyOpenToNewLibrary() {
        val open = _open.value ?: return
        copyFrom(open.document, open.index, null)
    }

    private fun copyFrom(doc: Document, index: Int, id: String?) {
        val s = libraries ?: return
        val clip = doc.copy(listOf(index.toUInt()))
        viewModelScope.launch {
            try {
                val target = id ?: withContext(io) { s.create(s.newLibraryName(), doc.driverId(), null) }
                val into = docs[target] ?: withContext(io) { s.read(target) }.also { docs[target] = it }
                if (!into.paste(into.len(), clip)) {
                    notify(Notice.Full(nameOf(target), into.maxPrograms().toInt()))
                } else {
                    withContext(io + NonCancellable) { s.save(target, into) }
                    refreshOutline()
                    notify(Notice.CopiedTo(nameOf(target)))
                }
            } catch (e: FfiException) {
                notify(Notice.Failed(e.reason()))
            } finally {
                clip.close()
            }
        }
    }

    private fun nameOf(id: String) = _ui.value.lines.firstOrNull { it.id == id }?.name ?: ""

    // Files

    override fun importFile() = importPicked { files.open() }

    /** Keep a picked file as a new library and show it; [pick] asks for it. */
    private fun importPicked(pick: suspend () -> PickedFile?) {
        viewModelScope.launch {
            val s = ready.await() ?: return@launch
            val picked = attempt { pick() } ?: return@launch
            val id = attempt {
                withContext(io) {
                    // Rust decides the format and refuses what a library can't hold.
                    val imported = readFile(picked.name, picked.bytes)
                    imported.document.use { s.add(imported.name, it, null) }
                }
            } ?: return@launch
            refreshOutline()
            _ui.update { it.copy(reveal = librarySource(id)) }
        }
    }

    /** Synth [driverId]'s demo library (the example sounds its driver makes), kept as a new library and shown. */
    fun addDemo(driverId: String) {
        viewModelScope.launch {
            val s = ready.await() ?: return@launch
            val title = getString(Res.string.library_demo)
            val id = attempt { withContext(io) { Document.demo(driverId, title).use { s.add(title, it, null) } } } ?: return@launch
            refreshOutline()
            _ui.update { it.copy(reveal = librarySource(id)) }
        }
    }

    override fun revealed() = _ui.update { it.copy(reveal = null) }

    override fun exportFormats(id: String): List<FileFormatInfo> {
        val driver = _ui.value.lines.firstOrNull { it.id == id }?.driverId ?: return emptyList()
        return try {
            formatsOf(driver)
        } catch (e: FfiException) {
            emptyList()
        }
    }

    override fun export(id: String, ext: String) {
        val shown = nameOf(id)
        viewModelScope.launch {
            // A file needs a name: a library with none takes the translated word.
            val name = shown.ifBlank { getString(Res.string.file_library) }
            val bytes = fileOf(id, ext) ?: return@launch
            val sink = attempt { files.create(safeFileName("$name.$ext"), MIME) } ?: return@launch
            attempt { withContext(io) { sink.write(bytes) } } ?: return@launch
            notify(Notice.Exported(name))
        }
    }

    override fun share(id: String) {
        val ext = exportFormats(id).firstOrNull()?.ext ?: return
        val shown = nameOf(id)
        viewModelScope.launch {
            val name = shown.ifBlank { getString(Res.string.file_library) }
            val bytes = fileOf(id, ext) ?: return@launch
            attempt { withContext(io) { files.share(safeFileName("$name.$ext"), bytes, MIME) } }
        }
    }

    /** Library [id] as a file in format [ext], built off the main thread; null (said) when it can't be. */
    private suspend fun fileOf(id: String, ext: String): ByteArray? {
        val s = libraries ?: return null
        // Its last edits are on disk (saved after every change), but an open one is current.
        val doc = docs[id]
        return attempt {
            withContext(io) {
                if (doc != null) exportFile(doc, ext) else s.read(id).use { exportFile(it, ext) }
            }
        }
    }

    /** [block]'s result, or null with a notice of why: a bad file or a failing picker never crashes the app. */
    private suspend fun <T> attempt(block: suspend () -> T): T? = try {
        block()
    } catch (e: CancellationException) {
        throw e
    } catch (e: FfiException) {
        notify(Notice.Failed(e.reason()))
        null
    } catch (e: FileUnreadable) {
        notify(Notice.CannotRead(e.name))
        null
    } catch (e: FileUnwritable) {
        notify(Notice.CannotWrite(e.name))
        null
    }

    // Notices

    private fun notify(notice: Notice, undo: String? = null) {
        forgetTrash()
        undoIn = undo
        undoBy = null
        _ui.update { it.copy(notice = notice) }
    }

    // A notice's own Undo, when it is not a step of the source shown (the editor's notices).
    private var undoBy: (() -> Unit)? = null

    /** Show [notice] above the island on every destination; its Undo runs [undo]. */
    fun say(notice: Notice, undo: (() -> Unit)? = null) {
        notify(notice)
        undoBy = undo
    }

    /**
     * A how-to line ([Notice.CompareHint], [Notice.RollHint], [Notice.RailHint]): shown only when no notice with
     * an action (Undo, Open, Retry) stands, and never ending a deletion's chance of Undo.
     */
    fun hint(notice: Notice) {
        val now = _ui.value.notice
        if (now != null && (now.undoable || now.opens != null || now is Notice.SaveFailed || trashed != null)) return
        _ui.update { it.copy(notice = notice) }
    }

    /** The editor's own steps, taken back from a notice: the open program's document. */
    fun undoOpenFromNotice() = undoOpen()

    /** The deletion can no longer be undone: its files go. */
    private fun forgetTrash() {
        val t = trashed ?: return
        trashed = null
        val s = libraries
        viewModelScope.launch {
            withContext(io) { s?.purge(t) }
            t.close()
        }
    }

    override fun undoNotice() {
        val t = trashed
        val target = undoIn
        val by = undoBy
        trashed = null
        undoIn = null
        undoBy = null
        _ui.update { it.copy(notice = null) }
        when {
            by != null -> by()
            t != null -> store({ s -> s.restore(t) }) { t.close() }
            // Only while that source is still the one shown (a synth may have gone).
            target != null && target == source -> undo()
        }
    }

    /** Write again every library whose last save failed. */
    override fun retrySave() {
        _ui.update { it.copy(notice = null) }
        for (id in _ui.value.unsaved) docs[id]?.let { save(id, it, retry = true) }
    }

    override fun openNotice() {
        val (key, index) = _ui.value.notice?.opens ?: return
        dismissNotice()
        select(key)
        selectProgram(index)
    }

    override fun dismissNotice() {
        forgetTrash()
        undoIn = null
        undoBy = null
        _ui.update { it.copy(notice = null) }
    }

    override fun onCleared() {
        // After every save already queued on the folder's thread, not beside them.
        val t = trashed
        val open = docs.values.toList()
        val clip = clipboard
        val s = libraries
        CoroutineScope(io).launch {
            t?.let { s?.purge(it); it.close() }
            open.forEach { it.close() }
            clip?.close()
            s?.close()
        }
    }

    private companion object {
        /** Library files have no registered type of their own. */
        const val MIME = "application/octet-stream"
        const val LIBRARY = "library:"
        const val SYNTH = "synth:"
    }
}

/** Why the core refused or failed, in its words. */
internal fun FfiException.reason(): String = when (this) {
    is FfiException.Failed -> reason
    is FfiException.Refused -> reason
    is FfiException.UnknownDriver -> message ?: id
}

/** Put [value] under [key] unless one is there already; then the newcomer is closed. */
internal fun <K, V : AutoCloseable> MutableMap<K, V>.keepFirst(key: K, value: V) {
    if (containsKey(key)) value.close() else put(key, value)
}

/** Whether a line is a folder. */
val OutlineLine.isFolder: Boolean get() = kind == NodeKind.FOLDER
