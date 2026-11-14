package io.github.oscyl.ios

import kotlinx.cinterop.UByteVar
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.alloc
import kotlinx.cinterop.allocArray
import kotlinx.cinterop.convert
import kotlinx.cinterop.get
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.nativeHeap
import kotlinx.cinterop.free
import kotlinx.cinterop.pointed
import kotlinx.cinterop.ptr
import kotlinx.cinterop.readBytes
import kotlinx.cinterop.reinterpret
import kotlinx.cinterop.usePinned
import kotlinx.cinterop.value
import platform.CoreFoundation.CFStringRef
import platform.CoreMIDI.MIDIClientCreateWithBlock
import platform.CoreMIDI.MIDIClientRef
import platform.CoreMIDI.MIDIClientRefVar
import platform.CoreMIDI.MIDIEndpointGetEntity
import platform.CoreMIDI.MIDIEndpointRef
import platform.CoreMIDI.MIDIEntityGetDestination
import platform.CoreMIDI.MIDIEntityGetDevice
import platform.CoreMIDI.MIDIEntityGetNumberOfDestinations
import platform.CoreMIDI.MIDIEntityRefVar
import platform.CoreMIDI.MIDIDeviceRefVar
import platform.CoreMIDI.MIDIGetNumberOfSources
import platform.CoreMIDI.MIDIGetSource
import platform.CoreMIDI.MIDIInputPortCreateWithBlock
import platform.CoreMIDI.MIDIObjectGetIntegerProperty
import platform.CoreMIDI.MIDIObjectGetStringProperty
import platform.CoreMIDI.MIDIObjectRef
import platform.CoreMIDI.MIDIOutputPortCreate
import platform.CoreMIDI.MIDIPacket
import platform.CoreMIDI.MIDIPacketList
import platform.CoreMIDI.MIDIPacketListAdd
import platform.CoreMIDI.MIDIPacketListInit
import platform.CoreMIDI.MIDIPacketNext
import platform.CoreMIDI.MIDIPortConnectSource
import platform.CoreMIDI.MIDIPortDispose
import platform.CoreMIDI.MIDIPortRef
import platform.CoreMIDI.MIDIPortRefVar
import platform.CoreMIDI.MIDISend
import platform.CoreMIDI.kMIDIMsgSetupChanged
import platform.CoreMIDI.kMIDIPropertyName
import platform.CoreMIDI.kMIDIPropertyUniqueID
import platform.CoreFoundation.CFStringRefVar
import platform.Foundation.CFBridgingRelease
import platform.Foundation.CFBridgingRetain
import platform.Foundation.NSLog
import platform.Foundation.NSString
import platform.Foundation.create
import platform.darwin.dispatch_async
import platform.darwin.dispatch_get_main_queue
import platform.posix.int32_tVar

/**
 * A synth's port pair as CoreMIDI shows it: the [product] (its device's name), the
 * [port] (the source's own name, "SOUND"), and an [id] unique among the plugged ones
 * (the source's and destination's unique IDs).
 */
class CoreMidiPort(val product: String, val port: String, val id: String)

/** An opened port pair: [send] whole messages; [close] stops the bytes and lets go of the ports. */
interface CoreMidiConnection {
    /** False when the port went (CoreMIDI refused the bytes). */
    fun send(bytes: ByteArray): Boolean

    fun close()
}

/**
 * The process's one CoreMIDI client and output port, made on first use and never
 * disposed (disposing an app's last client can stop the MIDI server, after which no
 * client can be made). Null when CoreMIDI refused them. Its setup-changed notifications
 * (a cable in or out) go to the [CoreMidi] listening now, on the main thread.
 */
private object Client {
    var listener: CoreMidi? = null

    val refs: Pair<MIDIClientRef, MIDIPortRef>? by lazy {
        memScoped {
            val c = alloc<MIDIClientRefVar>()
            val status = MIDIClientCreateWithBlock(cf("Oscyl"), c.ptr) { message ->
                if (message?.pointed?.messageID == kMIDIMsgSetupChanged) {
                    dispatch_async(dispatch_get_main_queue()) { listener?.changed() }
                }
            }
            if (status != 0) {
                NSLog("OscylMidi: no client (%d)", status)
                return@memScoped null
            }
            val out = alloc<MIDIPortRefVar>()
            if (MIDIOutputPortCreate(c.value, cf("Oscyl out"), out.ptr) != 0) return@memScoped null
            c.value to out.value
        }
    }
}

/**
 * USB MIDI through CoreMIDI, the iOS counterpart of Android's `MidiManager`: the
 * process's client, whose setup-changed notifications list the ports again and call
 * [onChange] on the main thread. Each source is paired with a destination of the same
 * entity, as a USB synth's port pairs are. Main thread only. Not [available] when
 * CoreMIDI could not be reached: no ports, nothing opens. Unverified until the first
 * run with an iPhone and a synth: the raw names go to the log.
 */
class CoreMidi(private val onChange: (List<CoreMidiPort>) -> Unit) {
    private val client: MIDIClientRef?
    private val output: MIDIPortRef?
    private val pairs = mutableMapOf<String, Pair<MIDIEndpointRef, MIDIEndpointRef>>()
    private val inputs = mutableSetOf<MIDIPortRef>()
    private var closed = false

    /** CoreMIDI answered: ports can be listed and opened. */
    val available: Boolean

    init {
        val refs = Client.refs
        client = refs?.first
        output = refs?.second
        available = refs != null
        Client.listener = this
    }

    internal fun changed() {
        if (!closed) onChange(ports())
    }

    /** The plugged port pairs, as of now. */
    fun ports(): List<CoreMidiPort> {
        pairs.clear()
        if (!available || closed) return emptyList()
        val list = mutableListOf<CoreMidiPort>()
        for (i in 0uL until MIDIGetNumberOfSources()) {
            val source = MIDIGetSource(i)
            val entity = memScoped {
                val e = alloc<MIDIEntityRefVar>()
                if (MIDIEndpointGetEntity(source, e.ptr) != 0) null else e.value
            } ?: continue
            if (MIDIEntityGetNumberOfDestinations(entity) == 0uL) continue
            val destination = MIDIEntityGetDestination(entity, 0u)
            val device = memScoped {
                val d = alloc<MIDIDeviceRefVar>()
                if (MIDIEntityGetDevice(entity, d.ptr) != 0) null else d.value
            }
            val product = device?.let { stringOf(it, kMIDIPropertyName) } ?: stringOf(entity, kMIDIPropertyName) ?: "MIDI"
            val name = stringOf(source, kMIDIPropertyName).orEmpty()
            // The unique IDs survive a replug; the endpoint refs say whether it is the same plug.
            val id = "${intOf(source, kMIDIPropertyUniqueID)}/${intOf(destination, kMIDIPropertyUniqueID)}/$source/$destination"
            NSLog("OscylMidi: product='%@' source='%@' id=%@", product, name, id)
            pairs[id] = source to destination
            list += CoreMidiPort(product, name, id)
        }
        return list
    }

    /**
     * Open port pair [id]: [onBytes] gets what the synth sends, on CoreMIDI's own thread.
     * Null when it is no longer plugged.
     */
    fun open(id: String, onBytes: (ByteArray) -> Unit): CoreMidiConnection? {
        val (source, destination) = pairs[id] ?: return null
        val client = client ?: return null
        val input = memScoped {
            val p = alloc<MIDIPortRefVar>()
            val status = MIDIInputPortCreateWithBlock(client, inputName, p.ptr) { list, _ ->
                list?.let { forEachPacket(it.pointed, onBytes) }
            }
            if (status != 0) null else p.value
        } ?: return null
        if (MIDIPortConnectSource(input, source, null) != 0) {
            MIDIPortDispose(input)
            return null
        }
        inputs += input
        return object : CoreMidiConnection {
            override fun send(bytes: ByteArray): Boolean = sendTo(destination, bytes)

            override fun close() {
                if (inputs.remove(input)) MIDIPortDispose(input)
            }
        }
    }

    private fun sendTo(destination: MIDIEndpointRef, bytes: ByteArray): Boolean {
        val output = output ?: return false
        // A packet's length is 16 bits: longer messages go in several (CoreMIDI joins sysex).
        return bytes.asList().chunked(MAX_PACKET).all { chunk -> sendPacket(output, destination, chunk.toByteArray()) }
    }

    private fun sendPacket(output: MIDIPortRef, destination: MIDIEndpointRef, bytes: ByteArray): Boolean {
        // A packet list holding one packet of every byte, with room for its headers.
        val size = bytes.size + 1024
        val buffer = nativeHeap.allocArray<UByteVar>(size)
        try {
            val list = buffer.reinterpret<MIDIPacketList>()
            var packet = MIDIPacketListInit(list)
            bytes.asUByteArray().usePinned { pinned ->
                packet = MIDIPacketListAdd(list, size.convert(), packet, 0u, bytes.size.convert(), pinned.addressOf(0))
            }
            return packet != null && MIDISend(output, destination, list) == 0
        } finally {
            nativeHeap.free(buffer)
        }
    }

    /** Let go of this instance's input ports and stop listening; the process's client stays. */
    fun close() {
        closed = true
        inputs.forEach { MIDIPortDispose(it) }
        inputs.clear()
        if (Client.listener === this) Client.listener = null
    }
}

/** Each packet's bytes of [list], in order. */
private fun forEachPacket(list: MIDIPacketList, onBytes: (ByteArray) -> Unit) {
    var packet: MIDIPacket = list.packet.pointed
    repeat(list.numPackets.toInt()) {
        val p: MIDIPacket = packet
        onBytes(p.data.readBytes(p.length.toInt()))
        packet = MIDIPacketNext(p.ptr)?.pointed ?: return
    }
}

private fun stringOf(obj: MIDIObjectRef, property: CFStringRef?): String? = memScoped {
    val s = alloc<CFStringRefVar>()
    if (MIDIObjectGetStringProperty(obj, property, s.ptr) != 0) return null
    CFBridgingRelease(s.value) as? String
}

private fun intOf(obj: MIDIObjectRef, property: CFStringRef?): Int = memScoped {
    val v = alloc<int32_tVar>()
    MIDIObjectGetIntegerProperty(obj, property, v.ptr)
    v.value
}

/** The name every input port gets (made once, kept for the process). */
private val inputName = cf("Oscyl in")

/** At most this many bytes per MIDIPacket (its length is a UInt16). */
private const val MAX_PACKET = 65535

/** A CFString for [s], kept for the process's life (three, made once each). */
private fun cf(s: String): CFStringRef? = CFBridgingRetain(NSString.create(string = s))?.reinterpret()
