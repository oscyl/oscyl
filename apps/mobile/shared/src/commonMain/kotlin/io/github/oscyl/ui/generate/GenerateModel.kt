package io.github.oscyl.ui.generate

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.oscyl.core.CharacterInfo
import io.github.oscyl.core.Dealt
import io.github.oscyl.core.FfiException
import io.github.oscyl.core.Generator
import io.github.oscyl.core.HandState
import io.github.oscyl.core.MacroInfo
import io.github.oscyl.core.OscillatorHand
import io.github.oscyl.core.Oscillators
import io.github.oscyl.core.oscillatorFamilies
import io.github.oscyl.core.oscillatorsSupported
import io.github.oscyl.core.SynthRef
import io.github.oscyl.core.UnitChip
import io.github.oscyl.core.auditionTarget
import io.github.oscyl.core.drivers
import io.github.oscyl.platform.Files
import io.github.oscyl.platform.NoFiles
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.library.reason
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
import kotlin.math.roundToInt

/** What the Generate destination shows. */
data class GenerateUi(
    /** The synth model the sounds are for. */
    val model: String = "",
    val characters: List<CharacterInfo> = emptyList(),
    val character: Int = 0,
    /** The family whose characters show (its id, as [CharacterInfo.family]); null shows all. */
    val family: String? = null,
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
    /** The picked sound's recipe code; null when it has none (a variation, a child, a morph step). */
    val code: String? = null,
    /** A code is being typed in. */
    val entering: Boolean = false,
    /** What is wrong with the code typed last, in the core's words. */
    val codeError: String? = null,
    /** The user units the synth holds ("My units"), each on unless the player left it out. */
    val units: List<UnitChip> = emptyList(),
    /** Said once: a kept sound needs its user unit where it goes. */
    val unitWarning: String? = null,
    /** The synth runs Oscyl's oscillators: New oscillators shows. */
    val oscillatorsOn: Boolean = false,
    /** The hand of new oscillators; null before the first. */
    val oscillators: OscillatorHand? = null,
    /** A synth of this model is connected (Deal with a new oscillator loads into it). */
    val connected: Boolean = false,
    /** The new oscillator loading for Deal with a new oscillator: its sounds follow once the synth lists it. */
    val pending: String? = null,
    /** Why the sounds of a new oscillator were not dealt, in the core's words. */
    val oscillatorNote: String? = null,
)

/** The families [characters] come in, in the core's order. */
fun families(characters: List<CharacterInfo>): List<String> =
    listOf("keys", "bass", "lead", "pad", "rhythm", "effect").filter { f -> characters.any { it.family == f } }

/** What Generate can ask for; every default does nothing (previews). */
interface GenerateActions {
    fun character(i: Int) {}
    fun family(id: String?) {}
    fun surprise() {}
    fun morph(other: Int) {}
    fun shareCode() {}
    fun enterCode(open: Boolean) {}
    fun dealCode(code: String) {}
    fun unitOn(i: Int, on: Boolean) {}
    fun dismissUnitWarning() {}
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
    fun dealOscillators() {}
    fun selectOscillator(i: Int) {}
    fun varyOscillator() {}
    fun breedOscillator(other: Int) {}
    fun keepOscillator() {}
    fun dealWithNewOscillator() {}
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
    private val files: Files = NoFiles,
) : ViewModel(), GenerateActions {
    private val _ui = MutableStateFlow(GenerateUi())
    val ui: StateFlow<GenerateUi> = _ui.asStateFlow()
    private var generator: Generator? = null
    private var driver: String? = null

    /** The family picked, kept for the session and across synths; until one is, each synth opens on its first character's. */
    private var family: String? = null
    private var familyPicked = false

    /** Each synth's generator, kept: going back to a synth finds its hands as they were. */
    private val generators = mutableMapOf<String, Generator>()

    /** The new oscillators' hands, the same for every synth that runs them. */
    private val oscillators = Oscillators(seed.toULong())
    private val models = drivers().associate { it.id to it.model }

    init {
        viewModelScope.launch {
            combine(library.open, synth.cards) { open, cards -> open to cards.filter { it.connected } }.collect { (open, cards) ->
                val id = open?.document?.driverId() ?: cards.firstOrNull()?.driverId ?: drivers().first().id
                if (id != driver) use(id)
                _ui.update {
                    it.copy(slot = open?.let { o -> o.document.slotLabel(o.index.toUInt()) }, connected = cards.any { c -> c.driverId == id })
                }
            }
        }
        // A synth listed its slots: "My units" follows (the core keeps each synth's units).
        viewModelScope.launch { synth.units.collect { refreshUnits() } }
        // A synth failed or went: a new oscillator waiting to be listed will not be (the notice says why).
        viewModelScope.launch { synth.failed.collect { _ui.update { it.copy(pending = null) } } }
    }

    /** Generate for [id]'s synth from now on, with its own hands (each synth its own seed). */
    private fun use(id: String) {
        val g = generators.getOrPut(id) { Generator(id, seed.toULong() + generators.size.toULong()) }
        generator = g
        driver = id
        val characters = g.characters()
        _ui.value = GenerateUi(
            model = models[id] ?: id,
            characters = characters,
            // A handful of tiles, not all of them; a family this synth lacks shows all.
            family = (if (familyPicked) family else characters.firstOrNull()?.family)?.takeIf { f -> characters.any { it.family == f } },
            macros = g.macros(),
            pages = g.pages(),
            slot = _ui.value.slot,
            units = g.units(),
            oscillatorsOn = oscillatorsSupported(id),
            oscillators = _ui.value.oscillators,
            connected = synth.cards.value.any { it.connected && it.driverId == id },
        )
        show(play = false)
    }

    /** The app's language changed: the core's words (characters, dials, pages) are asked again; the hands stay. */
    fun relocalise() {
        val g = generator ?: return
        _ui.update { it.copy(characters = g.characters(), macros = g.macros(), pages = g.pages()) }
        show(play = false)
    }

    private fun refreshUnits() {
        val g = generator ?: return
        _ui.update { it.copy(units = g.units()) }
        dealPending()
    }

    /** A listing came while a deal ran: the new oscillator's deal waits for that one to end. */
    private var listedWhileBusy = false

    /** The new oscillator is in a slot with its parameters known: deal six sounds that use it (of a character that can, shown chosen). */
    private fun dealPending() {
        val name = _ui.value.pending ?: return
        if (_ui.value.busy) {
            listedWhileBusy = true
            return
        }
        val ui = _ui.value
        dealt { g ->
            try {
                val d = g.dealWithUnit(ui.character.toUInt(), ui.values.map { it / 100f }, name) ?: return@dealt false
                chosen(d)
                _ui.update { it.copy(pending = null) }
                true
            } catch (e: FfiException) {
                _ui.update { it.copy(pending = null, oscillatorNote = e.reason()) }
                false
            }
        }
    }

    /** A deal ended: one a listing asked for meanwhile runs now. */
    private fun dealEnded() {
        if (listedWhileBusy) {
            listedWhileBusy = false
            dealPending()
        }
    }

    override fun unitOn(i: Int, on: Boolean) {
        val g = generator ?: return
        val unit = _ui.value.units.getOrNull(i) ?: return
        g.setUnitOn(unit.module, unit.slot, on)
        refreshUnits()
    }

    override fun dismissUnitWarning() = _ui.update { it.copy(unitWarning = null) }

    /** Run [work] on the generator off the main thread, then show the hand. */
    private fun dealt(play: Boolean = true, work: (Generator) -> Boolean) {
        val g = generator ?: return
        if (_ui.value.busy) return
        _ui.update { it.copy(busy = true) }
        viewModelScope.launch {
            val ok = withContext(Dispatchers.IO) { work(g) }
            _ui.update { it.copy(busy = false) }
            if (ok) show(play)
            dealEnded()
        }
    }

    /** The hand as it is now, its pictures made off the main thread. */
    private fun show(play: Boolean) {
        val g = generator ?: return
        viewModelScope.launch {
            val (hand, code) = withContext(Dispatchers.IO) { g.hand(POINTS) to g.code() }
            if (g === generator) _ui.update { it.copy(hand = hand, code = code) }
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

    /** Show only [id]'s characters (null: all); the chosen one stays when it is among them, else the family's first is. */
    override fun family(id: String?) {
        family = id
        familyPicked = true
        _ui.update { ui ->
            val shown = { i: Int -> id == null || ui.characters.getOrNull(i)?.family == id }
            ui.copy(family = id, character = if (shown(ui.character)) ui.character else ui.characters.indices.firstOrNull(shown) ?: ui.character)
        }
    }

    /** The page moves to what the core chose (a surprise, a code): its character, out of a family that hides it, and its dials. */
    private fun chosen(dealt: Dealt) = _ui.update { ui ->
        val character = dealt.character.toInt()
        val hidden = ui.family != null && ui.characters.getOrNull(character)?.family != ui.family
        if (hidden) { family = null; familyPicked = true }
        ui.copy(
            character = character,
            family = if (hidden) null else ui.family,
            values = dealt.macros.map { (it * 100).roundToInt() },
        )
    }

    override fun surprise() = dealt { g -> g.surprise()?.also(::chosen) != null }

    override fun morph(other: Int) = dealt { g -> g.morph(other.toUInt()) }

    override fun shareCode() {
        _ui.value.code?.let(files::shareText)
    }

    override fun enterCode(open: Boolean) = _ui.update { it.copy(entering = open, codeError = null) }

    /** Deal the hand [code] names; a bad code stays in its field with the core's word on why. */
    override fun dealCode(code: String) = dealt { g ->
        try {
            chosen(g.dealCode(code))
            _ui.update { it.copy(entering = false, codeError = null) }
            true
        } catch (e: FfiException) {
            _ui.update { it.copy(codeError = e.reason()) }
            false
        }
    }

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
        // Over a slot of a synth, or over a program of a library (which any synth may play).
        val onSynth = library.open.value?.synthId != null
        g.keepWarning(onSynth)?.let { w -> _ui.update { it.copy(unitWarning = w) } }
        val slot = library.keepOverOpen(p) ?: return
        g.markKept(slot)
        show(play = false)
    }

    override fun keepInLibrary() {
        if (_ui.value.busy) return
        val g = generator ?: return
        val p = g.selectedProgram() ?: return
        val id = driver ?: return
        g.keepWarning(false)?.let { w -> _ui.update { it.copy(unitWarning = w) } }
        viewModelScope.launch {
            val name = library.keepInLibrary(id, p) ?: return@launch
            g.markKept(name)
            show(play = false)
        }
    }

    /** Run [work] on the oscillators off the main thread, then show their hand. */
    private fun oscillatorsDealt(work: (Oscillators) -> Boolean) {
        if (_ui.value.busy) return
        _ui.update { it.copy(busy = true) }
        viewModelScope.launch {
            val hand = withContext(Dispatchers.IO) { if (work(oscillators)) oscillators.hand(WAVE_POINTS) else null }
            _ui.update { it.copy(busy = false, oscillators = hand ?: it.oscillators) }
            dealEnded()
        }
    }

    override fun dealOscillators() = oscillatorsDealt { it.deal(null); true }

    override fun selectOscillator(i: Int) {
        if (_ui.value.busy) return
        oscillators.select(i.toUInt())
        _ui.update { it.copy(oscillators = it.oscillators?.copy(selected = i.toUInt())) }
    }

    override fun varyOscillator() = oscillatorsDealt { it.vary(_ui.value.amount.toUInt()) }

    override fun breedOscillator(other: Int) = oscillatorsDealt { it.breed(other.toUInt()) }

    /** Keep the picked oscillator: on the shelf, and into the synth when one is connected. */
    override fun keepOscillator() {
        val hand = _ui.value.oscillators ?: return
        val tile = hand.oscillators.getOrNull(hand.selected.toInt()) ?: return
        val id = driver ?: return
        viewModelScope.launch { synth.keepOscillator(id, tile.patch) }
    }

    /** Make an oscillator, load it, and deal sounds that use it once the synth lists it. */
    override fun dealWithNewOscillator() {
        val id = driver ?: return
        if (_ui.value.busy || _ui.value.pending != null) return
        viewModelScope.launch {
            val tile = withContext(Dispatchers.IO) {
                Oscillators(kotlin.random.Random.nextLong().toULong()).use { o ->
                    o.deal(oscillatorFamilies().random().id)
                    o.hand(WAVE_POINTS)?.oscillators?.firstOrNull()
                }
            } ?: return@launch
            _ui.update { it.copy(oscillatorNote = null) }
            if (synth.keepOscillator(id, tile.patch) == true) {
                _ui.update { it.copy(pending = tile.name) }
                // Listed already (a fast synth): deal now.
                dealPending()
            }
        }
    }

    override fun onCleared() {
        generators.values.forEach { it.close() }
        oscillators.close()
    }

    private companion object {
        /** The tiles' pictures are small. */
        const val POINTS = 96u

        /** One cycle of an oscillator, small. */
        const val WAVE_POINTS = 96u
    }
}
