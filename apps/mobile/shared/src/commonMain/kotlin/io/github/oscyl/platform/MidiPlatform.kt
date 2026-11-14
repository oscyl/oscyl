package io.github.oscyl.platform

import io.github.oscyl.core.MidiInputSink
import io.github.oscyl.core.MidiOut
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/** A MIDI device the platform sees: its product name and a unique [id] among the plugged ones. */
data class MidiPortInfo(val name: String, val id: String)

/** What opening a port came to. */
sealed interface OpenResult {
    /**
     * Open: Rust sends through [out]; hand the sink `Synth.openLink` returns to
     * [attachInput] and the platform pushes the device's bytes into it (and, when the
     * device goes, disconnects and releases it).
     */
    data class Opened(val out: MidiOut, val attachInput: (MidiInputSink) -> Unit) : OpenResult

    /** The person said no to USB access (or has not answered yet). */
    data object PermissionDenied : OpenResult

    /** Why, in the platform's words; empty when the platform has no MIDI at all. */
    data class Failed(val reason: String) : OpenResult
}

/**
 * The device's USB MIDI: the plugged [ports], kept current as cables come and go, and
 * [open]. A port that disappears has had its sink disconnected and released already.
 * [close] lets go of every port and stops watching.
 */
interface MidiPlatform : AutoCloseable {
    val ports: StateFlow<List<MidiPortInfo>>

    suspend fun open(port: MidiPortInfo): OpenResult

    /** Let go of [port]'s opened ports (its synth was disconnected; the cable may still be in). */
    fun release(port: MidiPortInfo) = Unit

    override fun close() = Unit
}

/** No MIDI: previews, tests, and iOS until CoreMIDI lands (M7). Never sees a port. */
object NoMidi : MidiPlatform {
    override val ports: StateFlow<List<MidiPortInfo>> = MutableStateFlow(emptyList())

    override suspend fun open(port: MidiPortInfo): OpenResult = OpenResult.Failed("")
}
