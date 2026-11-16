package io.github.oscyl

import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.window.ComposeUIViewController
import io.github.oscyl.core.LinkException
import io.github.oscyl.ios.CoreMidi
import io.github.oscyl.ios.CoreMidiConnection
import io.github.oscyl.ios.CoreMidiPort
import io.github.oscyl.ios.Documents
import io.github.oscyl.ios.Haptics
import io.github.oscyl.ios.UnreadableDocument
import io.github.oscyl.ios.appSupportDir
import io.github.oscyl.ios.preferredLanguage
import io.github.oscyl.platform.AppStorage
import io.github.oscyl.platform.FileUnreadable
import io.github.oscyl.platform.FileUnwritable
import io.github.oscyl.platform.Files
import io.github.oscyl.platform.MidiLink
import io.github.oscyl.platform.MidiPlatform
import io.github.oscyl.platform.MidiPortInfo
import io.github.oscyl.platform.MidiTrouble
import io.github.oscyl.platform.OpenResult
import io.github.oscyl.platform.OscylHaptics
import io.github.oscyl.platform.PickFor
import io.github.oscyl.platform.PickedFile
import io.github.oscyl.platform.Platform
import io.github.oscyl.platform.Sink
import io.github.oscyl.platform.midiPortName
import io.github.oscyl.platform.midiTrouble
import io.github.oscyl.ui.shell.App
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.receiveAsFlow
import platform.Foundation.NSURL
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * The iOS device: CoreMIDI, the document picker and share sheet, UIKit's haptic
 * generators, Application Support for Oscyl's files and the person's preferred
 * language. The platform work itself lives in `:iosPlatform` (whose klibs build on
 * Linux too); this only adapts it to the shared contracts.
 */
internal class IosPlatform : Platform {
    private val storage = AppStorage(appSupportDir())
    override val librariesDir: String = storage.libraries
    override val backupsDir: String = storage.backups
    override val unsyncedDir: String = storage.unsynced
    override val settingsPath: String = storage.settings
    override val language: String? = preferredLanguage()

    override val haptics: OscylHaptics = object : OscylHaptics {
        private val h = Haptics()
        override fun tick() = h.tick()
        override fun detent() = h.detent()
        override fun confirm() = h.confirm()
    }

    override val files: Files = IosFiles(Documents())

    override fun midi(): MidiPlatform = IosMidi()
}

/** The document picker as [Files]; iOS keeps the app alive while it is up, so no pick is orphaned. */
private class IosFiles(private val documents: Documents) : Files {
    override suspend fun open(purpose: PickFor): PickedFile? = try {
        documents.open()?.let { PickedFile(it.name, it.bytes) }
    } catch (e: UnreadableDocument) {
        throw FileUnreadable(e.name, e)
    }

    override suspend fun create(name: String, mime: String): Sink? {
        val url = documents.create(name) ?: return null
        return Sink { bytes -> if (!documents.write(url, bytes)) throw FileUnwritable(name) }
    }

    override fun share(name: String, bytes: ByteArray, mime: String) = documents.share(name, bytes)

    override fun shareText(text: String) = documents.shareText(text)

    override val opened: Flow<suspend () -> PickedFile> = Incoming.files.receiveAsFlow()
}

/** Files other apps open in Oscyl, waiting for the library to take them (one taker). */
private object Incoming {
    val files = Channel<suspend () -> PickedFile>(Channel.UNLIMITED)
    val documents = Documents()
}

/**
 * A file another app opened in Oscyl (SwiftUI's `onOpenURL`), handed to the library,
 * which reads it off the main thread and keeps it as a new library, or says it
 * couldn't read it.
 */
fun openFile(url: NSURL) {
    Incoming.files.trySend {
        try {
            Incoming.documents.opened(url).let { PickedFile(it.name, it.bytes) }
        } catch (e: UnreadableDocument) {
            throw FileUnreadable(e.name, e)
        }
    }
}

/** CoreMIDI as [MidiPlatform]: a port that goes has its link closed (sink disconnected, released) before it leaves the list. */
private class IosMidi : MidiPlatform {
    private val _ports = MutableStateFlow<List<MidiPortInfo>>(emptyList())
    override val ports: StateFlow<List<MidiPortInfo>> = _ports.asStateFlow()
    private val links = mutableMapOf<String, MidiLink>()
    private val midi = CoreMidi(::publish)

    init {
        publish(midi.ports())
    }

    private fun publish(found: List<CoreMidiPort>) {
        val ids = found.map { it.id }.toSet()
        links.keys.filter { it !in ids }.forEach { links.remove(it)?.close() }
        _ports.value = found.map { MidiPortInfo(midiPortName(it.product, it.port), it.id) }
    }

    override suspend fun open(port: MidiPortInfo): OpenResult {
        if (!midi.available) return OpenResult.Failed(midiTrouble(MidiTrouble.Unavailable))
        links.remove(port.id)?.close()
        val gone = midiTrouble(MidiTrouble.Unplugged, port.name)
        var connection: CoreMidiConnection? = null
        val link = MidiLink(
            write = { bytes -> if (connection?.send(bytes) != true) throw LinkException.Closed(gone) },
            stop = { connection?.close() },
        )
        // Bytes may arrive (on CoreMIDI's thread) before the sink: the link keeps them.
        connection = midi.open(port.id, link::push) ?: return OpenResult.Failed(gone)
        links[port.id] = link
        return OpenResult.Opened(link) { link.attach(it) }
    }

    override fun release(port: MidiPortInfo) {
        links.remove(port.id)?.close()
    }

    override fun close() {
        // First: a notification queued now finds the client closed and publishes nothing.
        midi.close()
        links.values.forEach(MidiLink::close)
        links.clear()
        _ports.value = emptyList()
    }
}

/** The root view controller the iOS app's SwiftUI shell hosts. */
fun MainViewController() = ComposeUIViewController {
    App(remember { IosPlatform() })
    // The CI smoke test's signal that the app is up (it then takes the screenshot).
    LaunchedEffect(Unit) {
        withFrameNanos {}
        println("Oscyl: first frame")
    }
}
