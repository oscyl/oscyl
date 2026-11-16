package io.github.oscyl.ui.editor

import io.github.oscyl.core.PlayLayout
import io.github.oscyl.core.FieldChange
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.oscyl.core.Role
import io.github.oscyl.core.Audition
import io.github.oscyl.core.FfiException
import io.github.oscyl.core.FieldState
import io.github.oscyl.core.HandleInfo
import io.github.oscyl.core.PageInfo
import io.github.oscyl.core.ProgramEditor
import io.github.oscyl.core.StageGraph
import io.github.oscyl.core.StageState
import io.github.oscyl.core.SynthRef
import io.github.oscyl.core.auditionTarget
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.library.OpenProgram
import io.github.oscyl.ui.synth.SynthCard
import io.github.oscyl.ui.synth.SynthModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.channels.Channel
import androidx.compose.runtime.withFrameNanos

/** The editor's top row and title: what changes only now and then. */
data class EditorHead(
    val name: String,
    val slot: String,
    /** The synth model ("minilogue xd"). */
    val model: String,
    /** A factory slot: shown, never edited. */
    val readOnly: Boolean = false,
    /** Read from the synth (always in a library). */
    val loaded: Boolean = true,
    val canUndo: Boolean = false,
    val canRedo: Boolean = false,
    /** A connected synth can play this program. */
    val canLive: Boolean = false,
    /** The synth that plays it while Live (the sheet's switch sits on its card). */
    val liveSynth: ULong? = null,
    val live: Boolean = false,
    val comparing: Boolean = false,
    /** Edited programs in the document, for the island's Save. */
    val saveCount: Int = 0,
    /** The document is a synth's slots: Save syncs them. */
    val sync: Boolean = false,
)

/**
 * Everything the panel shows, as snapshot state the controls read one by one: a dial
 * drag changes its own field's state (and the stages and pictures that follow it),
 * so only those redraw. Values that come out equal leave their readers alone.
 * Previews build one from plain sample values.
 */
@Stable
class PanelState(
    val pages: List<PageInfo>,
    val firstPage: Int,
    fields: List<FieldState>,
    stages: List<StageState>,
    graphs: List<StageGraph>,
    handles: List<List<HandleInfo>>,
) {
    private val fields: Map<UInt, MutableState<FieldState>> = fields.associate { it.field to mutableStateOf(it) }
    private val stages = stages.map { mutableStateOf(it) }
    private val graphs = graphs.map { mutableStateOf(it) }
    private val handles = handles.map { mutableStateOf(it) }

    /** Tells this program's panel from the next one's (item keys). */
    val id: Int = next++

    /** Hold to compare: everything shows the saved program. */
    var comparing by mutableStateOf(false)

    /** The program can change (read, not a factory slot): its controls answer. */
    var writable by mutableStateOf(true)

    /** Where the program shown stands between saved (0) and the edit (1): a morph moves it. */
    var blend by mutableFloatStateOf(1f)

    /** Changes sent to the synth so far: the Live lamp blinks on each. */
    var sends by mutableIntStateOf(0)

    fun field(f: UInt): FieldState? = fields[f]?.value

    fun stage(page: Int): StageState? = stages.getOrNull(page)?.value

    fun graph(page: Int): StageGraph? = graphs.getOrNull(page)?.value

    fun handles(page: Int): List<HandleInfo> = handles.getOrNull(page)?.value.orEmpty()

    /**
     * The pictures shown now came from a control being held (a knob turning, a handle
     * dragged, the Compare fader sliding): a graph draws them at once, in the frame they
     * land, where any other change glides. Read in the draw phase and by the glide's
     * effect; plain, because the picture it speaks of is the state that redraws.
     */
    var following = false
        private set

    /** The stages on screen (all until the page says), and those whose pictures are out of date. */
    private var onScreen: Set<Int>? = null
    private val stale = mutableSetOf<Int>()

    /**
     * Take the program's state: every stage's summary and count, and the fields and
     * pictures of the stages on screen; the others wait until they come into view
     * ([show]), so a drag computes only what can be seen.
     */
    internal fun update(editor: ProgramEditor, following: Boolean = false) {
        this.following = following
        val shown = pages.indices.filter { onScreen?.contains(it) != false }
        stale += pages.indices - shown.toSet()
        take(editor, shown)
    }

    /** Stages [shown] came into view: bring their pictures up to date. */
    internal fun show(editor: ProgramEditor, shown: Set<Int>) {
        onScreen = shown
        val fresh = shown.filter { stale.remove(it) }
        if (fresh.isNotEmpty()) take(editor, fresh)
    }

    /** Every stage at a glance, and [pages]' fields, pictures and handles: one call to the core. */
    private fun take(editor: ProgramEditor, pages: List<Int>) {
        val u = editor.update(pages.map { it.toUInt() }, GRAPH_POINTS)
        u.stages.forEachIndexed { i, s -> stages.getOrNull(i)?.value = s }
        u.fields.forEach { f -> fields[f.field]?.value = f }
        pages.forEachIndexed { k, p ->
            graphs[p].value = u.graphs[k]
            handles[p].value = u.handles[k]
        }
    }

    companion object {
        private var next = 0

        /** The stage pictures' curves: a point every few pixels across a phone, cheap to redraw per frame. */
        const val GRAPH_POINTS = 256u

        /** The panel of [editor], opening at page [at], else where [role] starts. */
        fun of(editor: ProgramEditor, role: Role? = null, at: Int? = null): PanelState {
            val pages = editor.pages()
            val range = pages.indices.map { it.toUInt() }
            return PanelState(
                pages,
                at ?: editor.startPage(role).toInt(),
                range.flatMap { editor.fields(it) },
                editor.stages(),
                range.map { editor.graphs(it, GRAPH_POINTS) },
                range.map { editor.handles(it) },
            )
        }
    }
}

/** What the editor can ask for; every default does nothing (previews). */
interface EditorActions {
    fun set(field: UInt, raw: Int) {}
    fun choose(field: UInt, option: Int) {}
    fun revert(field: UInt) {}

    /** A gesture ended: one undo step, and the library is saved. */
    fun release() {}
    fun drag(page: Int, handle: Int, x: Float, y: Float) {}
    fun format(field: UInt, raw: Int): String = raw.toString()

    /** The value typed [text] names for [field] (as the core reads it), or null when it names none. */
    fun parse(field: UInt, text: String): Int? = text.trim().toIntOrNull()

    /** Stages [pages] are on screen now. */
    fun showing(pages: Set<Int>) {}
    fun undo() {}
    fun redo() {}
    fun toggleLive() {}
    fun compare(on: Boolean) {}

    /** Show the program a part [t] of the way from saved to the edit; null ends the morph (back at the edit). */
    fun morph(t: Float?) {}

    /** Roll stage [page]'s values: one undo step. */
    fun roll(page: Int) {}

    /** Compare was tapped too briefly to hear anything: say how it works. */
    fun compareTapped() {}

    /** A plain tap on a stage of the rail: the rail can be glided along, said once. */
    fun railTapped() {}

    /** A tap on a stage's dice, which rolls on a hold: how it works. */
    fun rollTapped(page: Int) {}

    /** Copy the open program into a new library (a factory slot's way to change it). */
    fun copyToLibrary() {}

    /** Set several fields as one gesture (the play page's pad): one undo step with the rest of it until [release]. */
    fun setAll(changes: List<Pair<UInt, Int>>) {}

    /** The play page shows [pages]' fields too (empty when it closes): they stay current as they change. */
    fun playing(pages: Set<Int>) {}

    /** Keep the open program as snapshot [slot] (0, 1, 2: A, B, C), in memory, until another program opens. */
    fun storeSnapshot(slot: Int) {}

    /** Put snapshot [slot] back: one undo step. An empty one does nothing. */
    fun recallSnapshot(slot: Int) {}
}

/** Does nothing: previews. */
object NoEditorActions : EditorActions

/**
 * The program open in the editor: the library model's selection, edited through the
 * core's [ProgramEditor] (every rule is the core's), with live audition on the synth
 * that plays it. The library is saved when a gesture ends, as on the desktop.
 */
class EditorModel(
    private val library: LibraryModel,
    private val synth: SynthModel,
    /** What the person mainly does (the welcome's answer), which picks the page a program opens at. */
    private val role: () -> Role? = { null },
    /** One-time hints (Compare's, on the first edit); by default all shown (tests). */
    private val teaching: io.github.oscyl.ui.preferences.Teaching = io.github.oscyl.ui.preferences.Teaching({ true }, {}),
) : ViewModel(), EditorActions {
    private val _head = MutableStateFlow<EditorHead?>(null)
    val head: StateFlow<EditorHead?> = _head.asStateFlow()

    // The stages the play page reads, besides those the editor's list shows.
    private var playPages = emptySet<Int>()

    private val _driverId = MutableStateFlow<String?>(null)

    /** The open program's synth model (driver id); null with none open. */
    val driverId: StateFlow<String?> = _driverId.asStateFlow()

    // Snapshots A, B, C of the open program: bytes in memory only.
    private val snapshotBytes = arrayOfNulls<ByteArray>(3)
    private val _snapshots = MutableStateFlow(List(3) { false })

    /** Which of A, B, C hold a snapshot. */
    val snapshots: StateFlow<List<Boolean>> = _snapshots.asStateFlow()

    /** The panel of the program open; null when none is. */
    var panel: PanelState? by mutableStateOf(null)
        private set

    private var editor: ProgramEditor? = null
    private var opened: OpenProgram? = null
    private var target: ULong? = null
    private var simulated = false
    private var live = false
    private val models = synth.drivers.associate { it.id to it.model }

    /** An edit has run since the last gesture ended. */
    private var midGesture = false

    /** The knob being turned, and its values as the core wrote them during this turn. */
    private var turned: UInt? = null
    private val formats = HashMap<UInt, String>()

    init {
        viewModelScope.launch {
            // Only what matters here: a read's progress is no reason to look again.
            val synths = synth.cards.map { cards -> cards.filter { it.connected }.map { it.copy(read = null) } }.distinctUntilChanged()
            combine(library.open, synths) { open, cards -> open to cards }
                .collect { (open, cards) -> show(open, cards) }
        }
        viewModelScope.launch { synth.selected.collect { (card, slot) -> follow(card, slot) } }
        viewModelScope.launch {
            synth.knobs.collect { (card, controls) ->
                turnedOnPanel += card to controls
                panelTurned.trySend(Unit)
            }
        }
    }

    /** What the synths' panels sent since the last frame, in order. */
    private val turnedOnPanel = mutableListOf<Pair<ULong, List<io.github.oscyl.core.ControlChange>>>()
    private val panelTurned = Channel<Unit>(Channel.CONFLATED)

    /**
     * Follow the synths' panels, once per frame: a knob turned fast sends a step every
     * few milliseconds, faster than a frame, and taking them one by one (each redrawing
     * the stages) fell seconds behind. Each frame takes everything sent since the last
     * one and redraws the stages once, so the value shown is the knob's, in the next
     * frame. As with a finger, the library hears of it once the knob rests: a frame
     * with nothing new from the panel. Run by the composition, which has the frame clock.
     */
    suspend fun followPanels() {
        for (turned in panelTurned) {
            while (withFrameNanos { takePanels() }) Unit
            panelRested()
        }
    }

    /** Everything the panels sent since the last frame, applied; false when nothing came. */
    private fun takePanels(): Boolean {
        if (turnedOnPanel.isEmpty()) return false
        val batch = turnedOnPanel.toList()
        turnedOnPanel.clear()
        batch.groupBy({ it.first }, { it.second }).forEach { (card, sent) -> followKnobs(card, sent.flatten()) }
        return true
    }

    /** The panel's knob rests: the library shows and saves the change, as at a finger's release. */
    private fun panelRested() {
        if (!panelMoved) return
        panelMoved = false
        midGesture = false
        library.editorChanged()
        refreshHead()
    }

    /** The panel changed the program since it last rested. */
    private var panelMoved = false

    /** Each synth's decoder of what its panel's knobs send (null: its driver has none). */
    private val decoders = mutableMapOf<ULong, io.github.oscyl.core.PanelDecoder?>()

    /** The field the synth's panel turned last. */
    private var panelField: UInt? = null

    /**
     * Live on and a program of synth [card] open: the knobs turned on its panel move the
     * open program's values, as edits not sent back (the synth has them). One undo step
     * per control turned. Otherwise its decoder starts afresh next time.
     */
    private fun followKnobs(card: ULong, controls: List<io.github.oscyl.core.ControlChange>) {
        val open = opened
        val e = editor
        if (!live || open == null || e == null || open.synthId != card || !e.loaded()) {
            decoders.remove(card)?.close()
            return
        }
        val decoder = decoders.getOrPut(card) { io.github.oscyl.core.panelDecoder(open.document.driverId()) } ?: return
        var changed = false
        for (c in controls) {
            val change = decoder.control(c.cc, c.value, e) ?: continue
            if (panelField != null && panelField != change.field) e.seal()
            panelField = change.field
            changed = edit(turning = change.field, send = false, show = false) { it.set(change.field, change.raw) } || changed
        }
        if (changed) {
            panel?.update(e, following = true)
            panelMoved = true
        }
    }

    /** The row the synth's panel just picked, opened by following it. */
    private var following: Int? = null

    /**
     * Live on and a program of synth [card] open: its panel picked [slot], so it opens
     * here too (in the list when it shows that synth). The synth already plays the
     * stored version: nothing is sent unless the app holds unsynced edits to it.
     */
    private fun follow(card: ULong, slot: UShort) {
        val open = opened ?: return
        if (!live || open.synthId != card || library.ui.value.programs?.source != open.source) return
        val index = open.document.slotIndex(slot)?.toInt() ?: return
        if (index == open.index) return
        following = index
        library.selectProgram(index)
    }

    /** The pages in view, as the screen last said. */
    private var shownPages = emptySet<Int>()

    private fun show(open: OpenProgram?, cards: List<SynthCard>) {
        val same = open != null && opened?.document === open.document && opened?.index == open.index && opened?.arranged == open.arranged
        opened = open
        if (open == null) {
            close()
            return
        }
        val driver = open.document.driverId()
        model = models[driver] ?: driver
        target = auditionTarget(cards.map { SynthRef(it.id, it.driverId) }, open.synthId, driver)
        simulated = cards.firstOrNull { it.id == target }?.simulated == true
        if (same) {
            turned = null
            formats.clear()
            midGesture = false
            editor?.let { e -> panel?.update(e) }
        } else {
            close()
            val e = open.document.editor(open.index.toUInt())
            editor = e
            rollSeed = kotlin.random.Random.nextLong().toULong()
            panel = PanelState.of(e, role())
            // A program opened while Live is on plays at once (never a slot not read yet);
            // one the synth's panel picked plays there already, unless edited here.
            val followed = following == open.index
            following = null
            if (live && e.loaded() && (!followed || e.edited())) play(Audition.Program(e.program()))
        }
        refreshHead()
        // Once the program's editor is there: the play page asks it for the model's defaults.
        _driverId.value = driver
    }

    /** The app's language changed: the core's words (pages, labels, values) are asked again. */
    fun relocalise() {
        val e = editor ?: return
        turned = null
        formats.clear()
        // The same program, at the page being read, and nothing sent to the synth again.
        panel = PanelState.of(e, role(), at = shownPages.minOrNull())
    }

    private fun close() {
        turned = null
        formats.clear()
        midGesture = false
        editor?.close()
        editor = null
        panel = null
        _head.value = null
        // Snapshots belong to the program they were taken of.
        clearSnapshots()
        _driverId.value = null
    }

    private fun refreshHead() {
        val e = editor ?: return
        val open = opened ?: return
        val f = e.facts()
        _head.value = EditorHead(
            name = f.name,
            slot = f.slot,
            model = model,
            readOnly = f.readOnly,
            loaded = f.loaded,
            canUndo = f.canUndo,
            canRedo = f.canRedo,
            canLive = target != null,
            liveSynth = target,
            live = live,
            comparing = f.comparing,
            saveCount = f.editedCount.toInt(),
            sync = !open.inLibrary,
        )
        panel?.writable = f.loaded && !f.readOnly
    }

    /** The synth model's name, for the slot legend. */
    private var model = ""

    /**
     * Run an edit; true when it took. The core refuses a factory slot and a slot not
     * read yet: nothing changes, and the note under the title says why.
     */
    private fun edit(
        turning: UInt? = null,
        following: Boolean = turning != null,
        send: Boolean = true,
        /** False when the caller redraws once after several edits. */
        show: Boolean = true,
        change: (ProgramEditor) -> Audition?,
    ): Boolean {
        val e = editor ?: return false
        // Another control, or another kind of edit: what was written for the last knob is void.
        if (turning == null || turning != turned) formats.clear()
        turned = turning
        val played = try {
            change(e)
        } catch (_: FfiException) {
            return false
        }
        if (show) panel?.update(e, following)
        if (send) played?.let(::play)
        // Within one gesture the head (undo, the Save count) changes with its first step only.
        if (!midGesture) refreshHead()
        midGesture = true
        return true
    }


    /** Send what an edit plays to the synth that plays it, while Live is on. */
    private fun play(audition: Audition) {
        val id = target?.takeIf { live } ?: return
        when (audition) {
            is Audition.Live -> synth.live(id, audition.messages)
            is Audition.Program -> synth.audition(id, audition.data)
        }
        // As on the desktop, the demo's lamp stays still: nothing goes down a cable.
        if (!simulated) panel?.let { it.sends++ }
    }

    override fun set(field: UInt, raw: Int) {
        edit(turning = field) { it.set(field, raw.toUInt()) }
    }

    override fun choose(field: UInt, option: Int) {
        if (edit { it.choose(field, option.toUInt()) }) release()
    }

    override fun revert(field: UInt) {
        if (edit { it.revertField(field) }) release()
    }

    override fun release() {
        midGesture = false
        editor?.seal() ?: return
        // The library model shows and saves the change, then hands the program back.
        library.editorChanged()
        refreshHead()
        // The first edit ever teaches Compare once, before anyone has to guess it.
        // (A real edit: a gesture can end with nothing changed.)
        val edited = panel?.let { p -> p.pages.indices.any { (p.stage(it)?.edits ?: 0u) > 0u } } == true
        if (edited && !teaching.taught(COMPARE_HINT)) {
            teaching.teach(COMPARE_HINT)
            library.hint(io.github.oscyl.ui.library.Notice.CompareHint)
        }
    }

    override fun drag(page: Int, handle: Int, x: Float, y: Float) {
        // Every field the handle moves, as one step of the drag's one undo step.
        edit(following = true) { it.dragTo(page.toUInt(), handle.toUInt(), x, y) }
    }

    override fun format(field: UInt, raw: Int): String {
        val e = editor ?: return raw.toString()
        // A turning knob asks for its ends, its saved value and its value on every step:
        // each is written by the core once per turn. Other fields always ask the core.
        return if (field == turned) formats.getOrPut(raw.toUInt()) { e.format(field, raw.toUInt()) } else e.format(field, raw.toUInt())
    }

    override fun parse(field: UInt, text: String): Int? = editor?.parse(field, text)?.toInt()

    override fun showing(pages: Set<Int>) {
        shownPages = pages
        val e = editor ?: return
        panel?.show(e, pages + playPages)
    }

    override fun playing(pages: Set<Int>) {
        playPages = pages
        val e = editor ?: return
        panel?.show(e, shownPages + pages)
    }

    override fun setAll(changes: List<Pair<UInt, Int>>) {
        edit { it.setAll(changes.map { (f, raw) -> FieldChange(f, raw.toUInt()) }) }
    }

    /** The play page's layout the open program's model starts with. */
    fun playDefaults(): PlayLayout? = editor?.playDefaults()

    override fun undo() = library.undoOpen()

    override fun redo() = library.redoOpen()

    override fun toggleLive() {
        live = !live
        refreshHead()
        editor?.let { if (live && it.loaded()) play(Audition.Program(it.program())) }
    }

    override fun compare(on: Boolean) {
        // Held: there is nothing left to say about holding it.
        if (on) teaching.teach(COMPARE_HINT)
        blendTo(if (on) 0f else 1f, following = false)
    }

    override fun compareTapped() = library.hint(io.github.oscyl.ui.library.Notice.CompareHint)

    override fun railTapped() = teaching.once(io.github.oscyl.ui.preferences.Hints.RAIL_GLIDE) {
        library.hint(io.github.oscyl.ui.library.Notice.RailHint)
    }

    override fun rollTapped(page: Int) {
        val title = panel?.pages?.getOrNull(page)?.title ?: return
        library.hint(io.github.oscyl.ui.library.Notice.RollHint(title))
    }

    override fun copyToLibrary() = library.copyOpenToNewLibrary()

    private fun clearSnapshots() {
        snapshotBytes.fill(null)
        _snapshots.value = List(3) { false }
    }

    override fun storeSnapshot(slot: Int) {
        val e = editor ?: return
        if (slot !in 0..2 || !e.loaded() || e.comparing()) return
        snapshotBytes[slot] = e.program()
        _snapshots.value = snapshotBytes.map { it != null }
    }

    override fun recallSnapshot(slot: Int) {
        val bytes = snapshotBytes.getOrNull(slot) ?: return
        if (edit { it.recall(bytes, "ABC"[slot].toString()) }) release()
    }

    // The dice's seeds: a fresh start per program opened, then one more per roll.
    private var rollSeed = 0uL

    override fun roll(page: Int) {
        if (edit { it.roll(page.toUInt(), rollSeed++) }) {
            release()
            // Undo is under the thumb: the notice above the island takes the roll back.
            val stage = panel?.pages?.getOrNull(page)?.title ?: return
            library.say(io.github.oscyl.ui.library.Notice.Rolled(stage)) { library.undoOpen() }
        }
    }

    override fun morph(t: Float?) = blendTo(t ?: 1f, following = t != null)

    /** Stand at [t] between saved and the edit; a fader sliding there is [following] (its pictures follow the finger). */
    private fun blendTo(t: Float, following: Boolean) {
        val e = editor ?: return
        val to = t.coerceIn(0f, 1f)
        if (e.blend() == to) return
        turned = null
        formats.clear()
        // The core blends every value and picture; the synth plays the blend (its worker sends the newest only).
        e.setBlend(to)
        panel?.let {
            it.blend = to
            it.comparing = to < 1f
            it.update(e, following)
        }
        _head.update { it?.copy(comparing = to < 1f) }
        if (e.loaded()) play(Audition.Program(e.program()))
    }

    override fun onCleared() {
        // A gesture still held (Compare) may end after this: it finds no editor.
        editor?.close()
        editor = null
    }
}

/** The one-time hint of Compare, said on the first edit. */
private const val COMPARE_HINT = "compare"
