package io.github.oscyl.ui.synth

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.oscyl.core.Document
import io.github.oscyl.core.DriverInfo
import io.github.oscyl.core.SlotProgram
import io.github.oscyl.core.Synth
import io.github.oscyl.core.SynthEvent
import io.github.oscyl.core.SynthListener
import io.github.oscyl.core.candidates
import io.github.oscyl.core.drivers
import io.github.oscyl.platform.MidiPlatform
import io.github.oscyl.platform.MidiPortInfo
import io.github.oscyl.platform.OpenResult
import io.github.oscyl.core.FfiException
import io.github.oscyl.core.BackupFile
import io.github.oscyl.core.backups
import io.github.oscyl.core.readFile
import io.github.oscyl.core.readUnit
import io.github.oscyl.core.unitBackupSection
import io.github.oscyl.core.unitBackupPath
import io.github.oscyl.core.unitExt
import io.github.oscyl.core.unitSection
import io.github.oscyl.platform.FileUnreadable
import io.github.oscyl.platform.Files
import io.github.oscyl.platform.NoFiles
import io.github.oscyl.platform.PickFor
import io.github.oscyl.platform.PickedFile
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.library_current
import io.github.oscyl.shared.generated.resources.library_disconnected
import io.github.oscyl.shared.generated.resources.notice_cannot_read
import io.github.oscyl.shared.generated.resources.notice_unit_lost
import io.github.oscyl.shared.generated.resources.units_no_empty
import io.github.oscyl.shared.generated.resources.units_no_synth
import io.github.oscyl.ui.library.reason
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.withContext
import org.jetbrains.compose.resources.getString
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.ReceiveChannel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.yield
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.TimeSource

/**
 * One synth on the Synth screen. A USB synth the person disconnected stays, not
 * [connected], while its cable is in, so it can be connected again.
 */
data class SynthCard(
    /** The worker's id while connected. */
    val id: ULong,
    val model: String,
    val firmware: String?,
    val simulated: Boolean,
    /** The port it answered on (empty for a demo). */
    val port: String,
    val connected: Boolean = true,
    /** While its programs are being read: how far. */
    val read: ReadProgress? = null,
    /** The worker's driver id. */
    val driverId: String = "",
)

/**
 * Programs to keep as a new library: a gone synth's unsynced edits, or the program
 * a synth is playing. [title] is the library's name; [reveal] shows it at once.
 */
class KeptEdits(val title: String, val document: Document, val reveal: Boolean = false, val key: String? = null)

/** This process's mark: worker ids restart each run, so each run keeps its own files. */
private val Run = kotlin.random.Random.nextLong().toULong().toString(36)

/**
 * The name a connected synth's unsynced edits are kept under: unique to this run, so
 * this run never deletes or overwrites a file a previous run left (which the next
 * start imports as a library).
 */
fun unsyncedKey(card: SynthCard) = "${card.driverId}-${card.id}-$Run"

/**
 * One user-unit slot of a logue synth: its section (`osc`, `modfx`, …) and the
 * section's name for people, its number and its unit, if any.
 */
data class UnitSlot(val module: String, val slot: UByte, val name: String?, val section: String = module)

/** A write to a synth: under way (how far), stored, or failed with the reason. */
sealed interface WriteState {
    data class Writing(val done: Int, val total: Int) : WriteState
    data class Written(val count: Int) : WriteState
    data class Failed(val reason: String) : WriteState
}

/** How many of a synth's programs have been read. */
data class ReadProgress(val done: Int, val total: Int)

/**
 * The app's synth worker and its USB ports: one [Synth] for the process, made in the
 * retained (ViewModel) scope so it outlives rotation. Every plugged port a driver
 * recognises by name is opened and linked; one that goes is reported `gone`. Events
 * arrive on a Rust thread and are handled, in order, on the main dispatcher.
 */
class SynthModel(
    private val midi: MidiPlatform,
    newSynth: (SynthListener) -> Synth = ::Synth,
    private val files: Files = NoFiles,
    private val backupsDir: String? = null,
) : ViewModel() {
    /** The app's language changed: the unit sections' names are asked again. */
    fun relocalise() {
        _units.update { all -> all.mapValues { (_, slots) -> slots.map { it.copy(section = unitSection(it.module)) } } }
    }

    /** Every driver, for the demo picker. */
    val drivers: List<DriverInfo> = drivers()
    private val models = drivers.associate { it.id to it.model }

    private val _cards = MutableStateFlow<List<SynthCard>>(emptyList())
    val cards: StateFlow<List<SynthCard>> = _cards.asStateFlow()

    private val _access = MutableStateFlow<List<MidiPortInfo>>(emptyList())

    /** Plugged synths waiting for the person to allow USB access. */
    val access: StateFlow<List<MidiPortInfo>> = _access.asStateFlow()

    private val _notice = MutableStateFlow<String?>(null)

    /** Why the last connection failed, in the core's or the platform's words; cleared by the next success. */
    val notice: StateFlow<String?> = _notice.asStateFlow()

    /** Each connected synth's slots, by worker id, filled in as they are read. */
    private val documents = mutableMapOf<ULong, Document>()

    private val _programs = MutableStateFlow(0L)

    /** Changes whenever a synth's programs change as a whole (a read finished, a write stored, a synth gone). */
    val programs: StateFlow<Long> = _programs.asStateFlow()

    private val _received = MutableSharedFlow<Pair<ULong, UInt>>(extraBufferCapacity = 64, onBufferOverflow = BufferOverflow.DROP_OLDEST)

    /**
     * One slot read (synth id, slot index): the list updates that row alone, so a
     * long read never redraws the whole list per program. A dropped one is made good
     * by [programs] when the read ends.
     */
    val received: SharedFlow<Pair<ULong, UInt>> = _received.asSharedFlow()

    private val _backups = MutableStateFlow<List<BackupFile>>(emptyList())

    /** The backups Oscyl took, newest first. */
    val backups: StateFlow<List<BackupFile>> = _backups.asStateFlow()

    /** List the backups folder again (off the main thread). */
    fun refreshBackups() {
        val dir = backupsDir ?: return
        viewModelScope.launch { _backups.value = withContext(Dispatchers.IO) { backups(dir) } }
    }

    private val _units = MutableStateFlow<Map<ULong, List<UnitSlot>>>(emptyMap())

    /** Each logue synth's user-unit slots, by worker id, once listed. */
    val units: StateFlow<Map<ULong, List<UnitSlot>>> = _units.asStateFlow()

    /** Synth [id]'s slots, while it is connected. */
    fun document(id: ULong): Document? = documents[id]

    private val _kept = Channel<KeptEdits>(Channel.UNLIMITED)

    /**
     * Edits left on a synth that went away (unplugged, disconnected), never dropped:
     * the library model keeps each as a new library. Held until someone receives them.
     */
    val kept: ReceiveChannel<KeptEdits> = _kept

    /** Close synth [id]'s slots, handing over its unsynced edits first. */
    private suspend fun release(id: ULong) {
        val doc = documents.remove(id) ?: return
        val card = _cards.value.firstOrNull { it.id == id }
        val model = card?.model ?: doc.title()
        val title = getString(Res.string.library_disconnected, model)
        doc.offline(title)?.let { _kept.trySend(KeptEdits(title, it, key = card?.let(::unsyncedKey))) }
        doc.close()
    }

    private val events = Channel<SynthEvent>(Channel.UNLIMITED)
    private val synth = newSynth(object : SynthListener {
        override fun onEvent(event: SynthEvent) {
            events.trySend(event)
        }
    })

    /** Plugged ports by their link name; the driver for those a driver recognised. */
    private val plugged = mutableMapOf<String, MidiPortInfo>()
    private val driverOf = mutableMapOf<String, String>()

    /** Ports with an open in flight, so a second tap does not open them twice. */
    private val opening = mutableSetOf<String>()

    init {
        viewModelScope.launch {
            // A whole read can arrive at once (the demo synth answers 500 slots in a blink), and
            // handled in one go it held the main thread for frames on end. Events are taken in
            // slices of a few milliseconds, the main loop turning between them: frames and
            // touches pass, and the read still ends quickly.
            var slice = TimeSource.Monotonic.markNow()
            for (event in events) {
                on(event)
                if (slice.elapsedNow() >= EventSlice) {
                    yield()
                    slice = TimeSource.Monotonic.markNow()
                }
            }
        }
        viewModelScope.launch { midi.ports.collect(::replug) }
        // A unit file picked while the system ended the process: its slot is not known now.
        viewModelScope.launch {
            val lost = try {
                files.orphan(PickFor.Unit)
            } catch (e: FileUnreadable) {
                PickedFile(e.name, ByteArray(0))
            } ?: return@launch
            _notice.value = getString(Res.string.notice_unit_lost, lost.name)
        }
    }

    fun openDemo(driverId: String) {
        synth.openDemo(driverId)
    }

    fun disconnect(card: SynthCard) = synth.disconnect(card.id)

    /** List [card]'s user-unit slots, once, if it has any (asked when its card shows them, as the desktop's units page does). */
    fun listUnits(card: SynthCard) {
        if (card.id in _units.value || card.id in listing || unitExt(card.driverId) == null) return
        listing += card.id
        synth.readSlots(card.id)
    }

    /** Synths whose slots were asked for. */
    private val listing = mutableSetOf<ULong>()

    /** Read every program of [card] again. */
    fun readAll(card: SynthCard) = synth.readAll(card.id)

    /** Stop reading [card]; the programs not read yet show as such. */
    fun cancelRead(card: SynthCard) = synth.cancelRead(card.id)

    /** Read the program [card] is playing; it is kept as a new library and shown. */
    fun readCurrent(card: SynthCard) = synth.readCurrent(card.id)

    /** Pick a unit file and load it into [slot] of [module] on [card]; why not, in words, when it can't be. */
    fun loadUnit(card: SynthCard, module: String, slot: UByte) {
        viewModelScope.launch {
            val picked = try {
                files.open(PickFor.Unit)
            } catch (e: FileUnreadable) {
                _notice.value = getString(Res.string.notice_cannot_read, e.name)
                null
            } ?: return@launch
            try {
                val unit = withContext(Dispatchers.IO) { readUnit(card.driverId, module, picked.bytes) }
                synth.uploadUnit(card.id, module, slot, unit.data)
            } catch (e: FfiException) {
                _notice.value = e.reason()
            }
        }
    }

    /**
     * Put a cleared unit back from its backup ([file], a `.slot.bin`): into the first
     * empty slot of its section on a connected synth it belongs to; said in a notice
     * when there is none.
     */
    fun putBack(file: BackupFile) {
        viewModelScope.launch {
            val bytes = try {
                withContext(Dispatchers.IO) { readFile(file.path) }
            } catch (_: FfiException) {
                _notice.value = getString(Res.string.notice_cannot_read, file.name)
                return@launch
            }
            for (card in _cards.value.filter { it.connected }) {
                val module = unitBackupSection(card.driverId, bytes) ?: continue
                val slot = _units.value[card.id]?.firstOrNull { it.module == module && it.name == null }
                if (slot == null) {
                    _notice.value = getString(Res.string.units_no_empty, unitSection(module), card.model)
                    return@launch
                }
                try {
                    val unit = readUnit(card.driverId, module, bytes)
                    synth.uploadUnit(card.id, module, slot.slot, unit.data)
                } catch (e: FfiException) {
                    _notice.value = e.reason()
                }
                return@launch
            }
            _notice.value = getString(Res.string.units_no_synth)
        }
    }

    /** Clear [slot] of [module] on [card], its unit ([name]) backed up first; nothing is cleared without the backup. */
    fun clearUnit(card: SynthCard, module: String, slot: UByte, name: String) {
        val dir = backupsDir ?: return
        try {
            synth.clearSlot(card.id, module, slot, unitBackupPath(dir, name))
        } catch (e: FfiException) {
            _notice.value = e.reason()
        }
    }

    /** Play [program] on synth [id] from its edit buffer, without storing it. */
    fun audition(id: ULong, program: ByteArray) = synth.audition(id, program)

    /** The programs of each write under way, to hand back as saved once the synth stores them. */
    private val writing = mutableMapOf<ULong, List<SlotProgram>>()

    private val _writes = MutableStateFlow<Map<ULong, WriteState>>(emptyMap())

    /** Each synth's last write, by worker id. */
    val writes: StateFlow<Map<ULong, WriteState>> = _writes.asStateFlow()

    /**
     * Write [programs] to synth [id] (the driver plans the safe commit). When the synth
     * has stored them, its slots' document marks them synced; on a failure nothing is.
     */
    fun write(id: ULong, programs: List<SlotProgram>) {
        // The synth went (during the backup, say): the write ends at once, as failed.
        if (id !in documents) {
            _writes.update { it + (id to WriteState.Failed("")) }
            return
        }
        writing[id] = programs
        _writes.update { it + (id to WriteState.Writing(0, programs.size)) }
        synth.write(id, programs)
    }

    /** Send live parameter changes to synth [id], one MIDI message each, in order. */
    fun live(id: ULong, messages: List<ByteArray>) = synth.live(id, messages)

    /** Ask for USB access to [port] again. */
    fun allow(port: MidiPortInfo) = connect(linkName(port))

    /** Connect a USB synth the person disconnected (its card's port), or retry one waiting for access. */
    fun connect(port: String) {
        val info = plugged[port] ?: return
        val driver = driverOf[port] ?: return
        _access.update { it - info }
        viewModelScope.launch { open(info, driver) }
    }

    private fun replug(ports: List<MidiPortInfo>) {
        val now = ports.associateBy(::linkName)
        for (port in plugged.keys - now.keys) {
            plugged.remove(port)
            driverOf.remove(port)
            synth.gone(port)
            _access.update { list -> list.filterNot { linkName(it) == port } }
            _cards.update { list -> list.filterNot { it.port == port } }
        }
        val fresh = now.filterKeys { it !in plugged }
        plugged += fresh
        for (candidate in candidates(fresh.keys.toList()).distinctBy { it.port }) {
            driverOf[candidate.port] = candidate.driverId
            viewModelScope.launch { open(now.getValue(candidate.port), candidate.driverId) }
        }
    }

    private suspend fun open(port: MidiPortInfo, driverId: String) {
        val name = linkName(port)
        if (!opening.add(name)) return
        try {
            when (val result = midi.open(port)) {
                is OpenResult.Opened -> result.attachInput(synth.openLink(driverId, name, result.out))
                OpenResult.PermissionDenied -> _access.update { it - port + port }
                is OpenResult.Failed -> _notice.value = result.reason
            }
        } finally {
            opening.remove(name)
        }
    }

    /** The synth whose error the notice shows. */
    private var errorFor: ULong? = null

    private suspend fun on(event: SynthEvent) {
        when (event) {
            is SynthEvent.Connected -> {
                _notice.value = null
                val card = SynthCard(
                    event.id,
                    models[event.driverId] ?: event.driverId,
                    event.firmware,
                    event.simulated,
                    if (event.simulated) "" else event.port,
                    driverId = event.driverId,
                )
                _cards.update { list -> list.filterNot { it.id == event.id || (!event.simulated && it.port == card.port) } + card }
                release(event.id)
                documents[event.id] = Document.synth(event.driverId, card.model)
                synth.readAll(event.id)
            }
            is SynthEvent.ReadProgress -> updateCard(event.id) {
                it.copy(read = ReadProgress(event.done.toInt(), event.total.toInt()))
            }
            is SynthEvent.Program -> documents[event.id]?.receive(event.slot, event.data)?.let { _received.tryEmit(event.id to it) }
            is SynthEvent.ReadDone -> {
                updateCard(event.id) { it.copy(read = null) }
                _programs.value++
            }
            is SynthEvent.Disconnected -> {
                // A write cut short stored nothing it can vouch for (no reason: the synth went).
                if (writing.remove(event.id) != null) _writes.update { it + (event.id to WriteState.Failed("")) }
                release(event.id)
                _units.update { it - event.id }
                _programs.value++
                disconnected(event.id)
                // A cable still in: let go of its ports until the person connects again.
                _cards.value.firstOrNull { it.id == event.id && !it.simulated }?.let { card -> plugged[card.port]?.let(midi::release) }
                // The cable came out: its error was this, not news.
                if (errorFor == event.id) {
                    _notice.value = null
                    errorFor = null
                }
            }
            is SynthEvent.ConnectFailed -> _notice.value = event.message
            is SynthEvent.WriteProgress -> _writes.update { it + (event.id to WriteState.Writing(event.done.toInt(), event.total.toInt())) }
            is SynthEvent.Written -> {
                // What was written becomes each slot's saved version; an edit made
                // while it ran stays an edit.
                val batch = writing.remove(event.id).orEmpty()
                for (slot in event.indices) {
                    batch.firstOrNull { it.slot == slot }?.let { documents[event.id]?.receive(slot, it.data) }
                }
                _programs.value++
                _writes.update { it + (event.id to WriteState.Written(event.indices.size)) }
                refreshBackups()
            }
            is SynthEvent.WriteFailed -> {
                writing.remove(event.id)
                _writes.update { it + (event.id to WriteState.Failed(event.message)) }
            }
            is SynthEvent.Slots -> {
                _units.update { it + (event.id to event.slots.map { s -> UnitSlot(s.module, s.slot, s.name, unitSection(s.module)) }) }
                // A cleared slot left a backup.
                refreshBackups()
            }
            is SynthEvent.Current -> {
                val model = _cards.value.firstOrNull { it.id == event.id }?.let { c -> c.model to c.driverId } ?: return
                val title = getString(Res.string.library_current, model.first)
                try {
                    _kept.trySend(KeptEdits(title, Document.ofPrograms(model.second, title, listOf(event.data)), reveal = true))
                } catch (e: FfiException) {
                    _notice.value = e.reason()
                }
            }
            // An Error before its Disconnected is status (the cable came out): shown,
            // and taken back if the Disconnected follows.
            is SynthEvent.Error -> {
                _notice.value = event.message
                errorFor = event.id
            }
            else -> Unit
        }
    }

    private fun updateCard(id: ULong, change: (SynthCard) -> SynthCard) =
        _cards.update { list -> list.map { if (it.id == id) change(it) else it } }

    private fun disconnected(id: ULong) = _cards.update { list ->
        list.mapNotNull { card ->
            when {
                card.id != id || !card.connected -> card
                // Still plugged in: keep it, to connect again.
                card.port in plugged -> card.copy(connected = false)
                else -> null
            }
        }
    }

    override fun onCleared() {
        documents.values.forEach { it.close() }
        documents.clear()
        midi.close()
        synth.quit()
        synth.close()
        events.close()
    }

    private companion object {
        /** Unique among plugged ports, as `Synth.openLink` needs: `"42:minilogue xd"`. */
        fun linkName(port: MidiPortInfo) = "${port.id}:${port.name}"
    }
}

/** How long the synth's events are handled before the main loop turns: a quarter of a 60 Hz frame. */
private val EventSlice = 4.milliseconds
