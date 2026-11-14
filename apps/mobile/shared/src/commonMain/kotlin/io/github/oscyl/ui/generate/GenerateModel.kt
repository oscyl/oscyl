package io.github.oscyl.ui.generate

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.oscyl.core.CharacterInfo
import io.github.oscyl.core.Generator
import io.github.oscyl.core.HandState
import io.github.oscyl.core.MacroInfo
import io.github.oscyl.core.SynthRef
import io.github.oscyl.core.auditionTarget
import io.github.oscyl.core.drivers
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.synth.SynthModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** What the Generate destination shows. */
data class GenerateUi(
    /** The synth model the sounds are for. */
    val model: String = "",
    val characters: List<CharacterInfo> = emptyList(),
    val character: Int = 0,
    val macros: List<MacroInfo> = emptyList(),
    /** Bright, movement, grit, space: 0..100. */
    val values: List<Int> = listOf(50, 50, 50, 50),
    /** The editor's pages, whose sound can be kept from the program open. */
    val pages: List<String> = emptyList(),
    val keep: Set<Int> = emptySet(),
    /** The program open in the editor, as Keep in Slot names it; null when none is. */
    val slot: String? = null,
    val hand: HandState? = null,
    /** Vary's amount: 0 subtle, 1 clear, 2 wild. */
    val amount: Int = 1,
    /** Dealing (a moment). */
    val busy: Boolean = false,
)

/** What Generate can ask for; every default does nothing (previews). */
interface GenerateActions {
    fun character(i: Int) {}
    fun value(i: Int, v: Int) {}
    fun keepPage(page: Int, on: Boolean) {}
    fun deal() {}
    fun select(i: Int) {}
    fun amount(a: Int) {}
    fun vary() {}
    fun breed(other: Int) {}
    fun step(delta: Int) {}
    fun keepInSlot() {}
    fun keepInLibrary() {}
}

/** Does nothing: previews. */
object NoGenerateActions : GenerateActions

/**
 * The Generate destination: the core's generator (`oscyl-app` hands) for the synth of
 * the program open, else the first synth connected, else the first Oscyl knows. A
 * sound picked plays on the synth that plays its programs (the page's promise, as on
 * the desktop); Keep puts it over the program open or at the end of a library.
 */
class GenerateModel(
    private val library: LibraryModel,
    private val synth: SynthModel,
    private val seed: Long = kotlin.random.Random.nextLong(),
) : ViewModel(), GenerateActions {
    private val _ui = MutableStateFlow(GenerateUi())
    val ui: StateFlow<GenerateUi> = _ui.asStateFlow()
    private var generator: Generator? = null
    private var driver: String? = null

    /** Each synth's generator, kept: going back to a synth finds its hands as they were. */
    private val generators = mutableMapOf<String, Generator>()
    private val models = drivers().associate { it.id to it.model }

    init {
        viewModelScope.launch {
            combine(library.open, synth.cards) { open, cards -> open to cards.filter { it.connected } }.collect { (open, cards) ->
                val id = open?.document?.driverId() ?: cards.firstOrNull()?.driverId ?: drivers().first().id
                if (id != driver) use(id)
                _ui.update { it.copy(slot = open?.let { o -> o.document.slotLabel(o.index.toUInt()) }) }
            }
        }
    }

    /** Generate for [id]'s synth from now on, with its own hands (each synth its own seed). */
    private fun use(id: String) {
        val g = generators.getOrPut(id) { Generator(id, seed.toULong() + generators.size.toULong()) }
        generator = g
        driver = id
        _ui.value = GenerateUi(
            model = models[id] ?: id,
            characters = g.characters(),
            macros = g.macros(),
            pages = g.pages(),
            slot = _ui.value.slot,
        )
        show(play = false)
    }

    /** The app's language changed: the core's words (characters, dials, pages) are asked again; the hands stay. */
    fun relocalise() {
        val g = generator ?: return
        _ui.update { it.copy(characters = g.characters(), macros = g.macros(), pages = g.pages()) }
        show(play = false)
    }

    /** Run [work] on the generator off the main thread, then show the hand. */
    private fun dealt(play: Boolean = true, work: (Generator) -> Boolean) {
        val g = generator ?: return
        if (_ui.value.busy) return
        _ui.update { it.copy(busy = true) }
        viewModelScope.launch {
            val ok = withContext(Dispatchers.IO) { work(g) }
            _ui.update { it.copy(busy = false) }
            if (ok) show(play)
        }
    }

    /** The hand as it is now, its pictures made off the main thread. */
    private fun show(play: Boolean) {
        val g = generator ?: return
        viewModelScope.launch {
            val hand = withContext(Dispatchers.IO) { g.hand(POINTS) }
            if (g === generator) _ui.update { it.copy(hand = hand) }
            if (play) play()
        }
    }

    /** The selected sound on the synth that plays this synth's programs, if one is connected. */
    private fun play() {
        val g = generator ?: return
        val p = g.selectedProgram() ?: return
        val cards = synth.cards.value.filter { it.connected }
        val target = auditionTarget(cards.map { SynthRef(it.id, it.driverId) }, library.open.value?.synthId, driver ?: return) ?: return
        synth.audition(target, p)
    }

    override fun character(i: Int) = _ui.update { it.copy(character = i) }

    override fun value(i: Int, v: Int) = _ui.update { it.copy(values = it.values.toMutableList().also { l -> l[i] = v }) }

    override fun keepPage(page: Int, on: Boolean) = _ui.update { it.copy(keep = if (on) it.keep + page else it.keep - page) }

    override fun deal() {
        val ui = _ui.value
        // Kept pages come from the program open, as it is now.
        val open = library.open.value
        val base = open?.let { o -> o.document.editor(o.index.toUInt()).use { it.program() } }
        dealt { g -> g.deal(ui.character.toUInt(), ui.values.map { it / 100f }, ui.keep.map { it.toUInt() }, base) }
    }

    // While a deal runs it holds the hands: a tap waits for it rather than block the main thread.
    override fun select(i: Int) {
        if (_ui.value.busy) return
        generator?.select(i.toUInt())
        show(play = true)
    }

    override fun amount(a: Int) = _ui.update { it.copy(amount = a) }

    override fun vary() = dealt { g -> g.vary(_ui.value.amount.toUInt()) }

    override fun breed(other: Int) = dealt { g -> g.breed(other.toUInt()) }

    override fun step(delta: Int) {
        if (_ui.value.busy) return
        if (generator?.step(delta) == true) show(play = true)
    }

    override fun keepInSlot() {
        if (_ui.value.busy) return
        val g = generator ?: return
        val p = g.selectedProgram() ?: return
        val slot = library.keepOverOpen(p) ?: return
        g.markKept(slot)
        show(play = false)
    }

    override fun keepInLibrary() {
        if (_ui.value.busy) return
        val g = generator ?: return
        val p = g.selectedProgram() ?: return
        val id = driver ?: return
        viewModelScope.launch {
            val name = library.keepInLibrary(id, p) ?: return@launch
            g.markKept(name)
            show(play = false)
        }
    }

    override fun onCleared() {
        generators.values.forEach { it.close() }
    }

    private companion object {
        /** The tiles' pictures are small. */
        const val POINTS = 96u
    }
}
