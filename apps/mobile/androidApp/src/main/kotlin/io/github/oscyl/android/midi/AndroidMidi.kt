package io.github.oscyl.android.midi

import android.content.Context
import android.content.pm.PackageManager
import android.hardware.usb.UsbDevice
import android.media.midi.MidiDevice
import android.media.midi.MidiDeviceInfo
import android.media.midi.MidiInputPort
import android.media.midi.MidiManager
import android.media.midi.MidiOutputPort
import android.media.midi.MidiReceiver
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import androidx.core.os.BundleCompat
import io.github.oscyl.core.LinkException
import io.github.oscyl.platform.MidiLink
import io.github.oscyl.platform.MidiTrouble
import io.github.oscyl.platform.midiTrouble
import io.github.oscyl.platform.midiPortName
import io.github.oscyl.platform.MidiPlatform
import io.github.oscyl.platform.MidiPortInfo
import io.github.oscyl.platform.OpenResult
import java.io.IOException
import kotlin.coroutines.resume
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.suspendCancellableCoroutine

/**
 * USB MIDI through Android's [MidiManager]. Each device's ports are listed as pairs
 * (what the device sends, what it takes), named as ALSA names them, which is what the
 * drivers' port hints were written against: `<product> _ <port name>` ("minilogue xd
 * _ SOUND"), or the product alone when the port has no name. Unverified until the first
 * run with a phone and an xd; the raw values go to the debug log. Hotplug arrives on the main thread; so does everything here
 * except each receiver's bytes, which come on the port's own thread.
 */
class AndroidMidi(context: Context) : MidiPlatform {
    private val app = context.applicationContext
    private val main = Handler(Looper.getMainLooper())

    /** Absent on devices without MIDI (the feature is optional in the manifest). */
    private val manager: MidiManager? =
        if (app.packageManager.hasSystemFeature(PackageManager.FEATURE_MIDI)) app.getSystemService(MidiManager::class.java) else null

    /** Plugged devices by their MIDI id, and what each offers. */
    private val devices = mutableMapOf<Int, Pair<MidiDeviceInfo, List<Pair<MidiPortInfo, Ends>>>>()
    private val links = mutableMapOf<String, MidiLink>()

    private val _ports = MutableStateFlow<List<MidiPortInfo>>(emptyList())
    override val ports: StateFlow<List<MidiPortInfo>> = _ports.asStateFlow()

    private val callback = object : MidiManager.DeviceCallback() {
        override fun onDeviceAdded(device: MidiDeviceInfo) = add(device)

        override fun onDeviceRemoved(device: MidiDeviceInfo) = remove(device)
    }

    init {
        manager?.let { m ->
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                m.getDevicesForTransport(MidiManager.TRANSPORT_MIDI_BYTE_STREAM).forEach(::add)
                m.registerDeviceCallback(MidiManager.TRANSPORT_MIDI_BYTE_STREAM, app.mainExecutor, callback)
            } else {
                @Suppress("DEPRECATION")
                m.devices.forEach(::add)
                @Suppress("DEPRECATION")
                m.registerDeviceCallback(callback, main)
            }
        }
    }

    private fun add(info: MidiDeviceInfo) {
        if (info.id in devices) return
        val product = info.properties.getString(MidiDeviceInfo.PROPERTY_PRODUCT)
            ?: info.properties.getString(MidiDeviceInfo.PROPERTY_NAME)
            ?: info.id.toString()
        val takes = info.ports.filter { it.type == MidiDeviceInfo.PortInfo.TYPE_INPUT }
        val pairs = info.ports.filter { it.type == MidiDeviceInfo.PortInfo.TYPE_OUTPUT }.mapNotNull { sends ->
            val input = takes.firstOrNull { sends.name.isNotBlank() && it.name == sends.name }
                ?: takes.firstOrNull { it.portNumber == sends.portNumber }
                ?: return@mapNotNull null
            val port = MidiPortInfo(
                name = midiPortName(product, sends.name),
                id = "${info.id}/${input.portNumber}/${sends.portNumber}",
            )
            Log.d(TAG, "device ${info.id}: product='$product' port ${sends.portNumber} '${sends.name}' -> '${port.name}'")
            port to Ends(input.portNumber, sends.portNumber)
        }
        devices[info.id] = info to pairs
        publish()
    }

    /** The device went: its links let go of their sinks first, then the ports disappear. */
    private fun remove(info: MidiDeviceInfo) {
        val (_, pairs) = devices.remove(info.id) ?: return
        pairs.forEach { (port, _) -> links.remove(port.id)?.close() }
        publish()
    }

    private fun publish() {
        _ports.value = devices.values.flatMap { (_, pairs) -> pairs.map { it.first } }
    }

    override suspend fun open(port: MidiPortInfo): OpenResult {
        val m = manager ?: return OpenResult.Failed(midiTrouble(MidiTrouble.Unavailable))
        val entry = devices.values.firstNotNullOfOrNull { (info, pairs) ->
            pairs.firstOrNull { it.first == port }?.let { info to it.second }
        } ?: return OpenResult.Failed(midiTrouble(MidiTrouble.Unplugged, port.name))
        val (info, ends) = entry
        links.remove(port.id)?.close()
        val device = openDevice(m, info) ?: run {
            // The MIDI service opens most USB devices itself; ask for access only when it could not.
            val usb = BundleCompat.getParcelable(info.properties, MidiDeviceInfo.PROPERTY_USB_DEVICE, UsbDevice::class.java)
                ?: return OpenResult.Failed(midiTrouble(MidiTrouble.OpenFailed, port.name))
            if (!UsbAccess.request(app, usb)) return OpenResult.PermissionDenied
            openDevice(m, info) ?: return OpenResult.Failed(midiTrouble(MidiTrouble.OpenFailed, port.name))
        }
        // Unplugged while it opened.
        if (devices[info.id]?.first !== info) {
            device.close()
            return OpenResult.Failed(midiTrouble(MidiTrouble.Unplugged, port.name))
        }
        val input = device.openInputPort(ends.input)
        val output = device.openOutputPort(ends.output)
        if (input == null || output == null) {
            input?.close()
            output?.close()
            device.close()
            return OpenResult.Failed(midiTrouble(MidiTrouble.InUse, port.name))
        }
        val link = linkOf(device, input, output)
        links[port.id] = link
        return OpenResult.Opened(link) { link.attach(it) }
    }

    private suspend fun openDevice(m: MidiManager, info: MidiDeviceInfo): MidiDevice? =
        awaitOpened { done -> m.openDevice(info, done, main) }

    override fun release(port: MidiPortInfo) {
        links.remove(port.id)?.close()
    }

    override fun close() {
        manager?.unregisterDeviceCallback(callback)
        links.values.forEach(MidiLink::close)
        links.clear()
        devices.clear()
        publish()
    }
}

private const val TAG = "OscylMidi"

/**
 * Wait for what [open] hands its callback. One that arrives after the wait was
 * cancelled (the model went) is closed, never leaked.
 */
internal suspend fun <T : AutoCloseable> awaitOpened(open: (callback: (T?) -> Unit) -> Unit): T? =
    suspendCancellableCoroutine { done -> open { opened -> done.resume(opened) { _, _, _ -> opened?.close() } } }

/** A device's port numbers: the one it takes bytes on and the one it sends from. */
private data class Ends(val input: Int, val output: Int)

/** One opened port pair: bytes go out through [input], [output]'s arrive in the link. */
private fun linkOf(device: MidiDevice, input: MidiInputPort, output: MidiOutputPort): MidiLink {
    lateinit var receiver: MidiReceiver
    val link = MidiLink(
        write = { bytes ->
            try {
                input.send(bytes, 0, bytes.size)
            } catch (e: IOException) {
                throw LinkException.Closed(e.message ?: e.toString())
            }
        },
        stop = {
            output.disconnect(receiver)
            output.close()
            input.close()
            device.close()
        },
    )
    receiver = object : MidiReceiver() {
        override fun onSend(msg: ByteArray, offset: Int, count: Int, timestamp: Long) = link.push(msg.copyOfRange(offset, offset + count))
    }
    output.connect(receiver)
    return link
}
