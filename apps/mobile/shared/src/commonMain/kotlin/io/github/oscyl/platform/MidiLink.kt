package io.github.oscyl.platform

import io.github.oscyl.core.MidiInputSink
import io.github.oscyl.core.MidiInputSinkInterface
import io.github.oscyl.core.MidiOut
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.midi_in_use
import io.github.oscyl.shared.generated.resources.midi_open_failed
import io.github.oscyl.shared.generated.resources.midi_unavailable
import io.github.oscyl.shared.generated.resources.midi_unplugged
import kotlinx.atomicfu.locks.SynchronizedObject
import kotlinx.atomicfu.locks.synchronized
import org.jetbrains.compose.resources.getString

/**
 * A synth's port as the platforms name it, as ALSA does on the desktop (what the
 * drivers' port hints were written against): `<product> _ <port>` ("minilogue xd _
 * SOUND"), or the product alone when the port has no name of its own.
 */
fun midiPortName(product: String, port: String): String =
    if (port.isBlank() || port == product) product else "$product _ $port"

/**
 * One opened port pair as Rust's [MidiOut], on any platform: [write] sends bytes
 * (throwing `LinkException` when the port went), [push] hands received bytes on, and
 * [stop] lets go of the platform's ports. Bytes that arrive before the sink is
 * attached (the inquiry's answer can beat it) wait and go first.
 */
class MidiLink(private val write: (ByteArray) -> Unit, private val stop: () -> Unit) : MidiOut {
    private val lock = SynchronizedObject()
    private var sink: MidiInputSinkInterface? = null
    private var release: (() -> Unit)? = null
    private var early: MutableList<ByteArray>? = mutableListOf()
    private var closed = false

    override fun send(bytes: ByteArray) = write(bytes)

    /** Bytes from the device, on whatever thread the platform delivers them. */
    fun push(bytes: ByteArray) {
        synchronized(lock) {
            if (closed) return
            sink?.push(bytes) ?: early?.add(bytes)
        }
    }

    fun attach(sink: MidiInputSink) = attach(sink) { sink.close() }

    /** Feed [sink] from now on ([release] lets go of it); a link already closed disconnects and releases it at once. */
    fun attach(sink: MidiInputSinkInterface, release: () -> Unit) {
        synchronized(lock) {
            if (!closed) {
                early?.forEach(sink::push)
                early = null
                this.sink = sink
                this.release = release
                return
            }
        }
        sink.disconnect()
        release()
    }

    /**
     * Disconnect the sink (later pushes are ignored), stop the ports, then release the
     * sink: once `closed` is set under the lock, nothing can push into it.
     */
    fun close() {
        val release = synchronized(lock) {
            if (closed) return
            closed = true
            sink?.disconnect()
            sink = null
            release.also { release = null }
        }
        stop()
        release?.invoke()
    }
}

/** Why a port could not be opened, said the same on every platform. */
enum class MidiTrouble { Unavailable, Unplugged, OpenFailed, InUse }

/** [trouble] in words, naming the port ([port]'s name). */
suspend fun midiTrouble(trouble: MidiTrouble, port: String = ""): String = when (trouble) {
    MidiTrouble.Unavailable -> getString(Res.string.midi_unavailable)
    MidiTrouble.Unplugged -> getString(Res.string.midi_unplugged, port)
    MidiTrouble.OpenFailed -> getString(Res.string.midi_open_failed, port)
    MidiTrouble.InUse -> getString(Res.string.midi_in_use, port)
}
