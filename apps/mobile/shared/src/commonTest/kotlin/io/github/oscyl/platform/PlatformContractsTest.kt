package io.github.oscyl.platform

import io.github.oscyl.core.MidiInputSinkInterface
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class PlatformContractsTest {
    @Test
    fun portsAreNamedAsOnTheDesktop() {
        assertEquals("minilogue xd _ SOUND", midiPortName("minilogue xd", "SOUND"))
        assertEquals("prologue", midiPortName("prologue", ""))
        assertEquals("prologue", midiPortName("prologue", "prologue"))
        // Android names no port: the xd's synth port is its second.
        assertEquals("minilogue xd _ MIDI 2", midiPortName("minilogue xd", "", number = 1))
        assertEquals("minilogue xd _ SOUND", midiPortName("minilogue xd", "SOUND", number = 1))
    }

    @Test
    fun everyPlatformKeepsTheSameLayout() {
        val s = AppStorage("/data/app/files/")
        assertEquals("/data/app/files/libraries", s.libraries)
        assertEquals("/data/app/files/backups", s.backups)
        assertEquals("/data/app/files/unsynced", s.unsynced)
        assertEquals("/data/app/files/settings.json", s.settings)
    }

    private class Sink : MidiInputSinkInterface {
        val got = mutableListOf<List<Byte>>()
        var disconnected = false
        override fun push(bytes: ByteArray) {
            got += bytes.toList()
        }
        override fun disconnect() {
            disconnected = true
        }
    }

    @Test
    fun earlyBytesGoFirstAndAClosedLinkFeedsNothing() {
        val order = mutableListOf<String>()
        val link = MidiLink(write = {}, stop = { order += "stop" })
        link.push(byteArrayOf(1))
        val sink = Sink()
        link.attach(sink) { order += "release" }
        link.push(byteArrayOf(2))
        assertEquals(listOf(listOf<Byte>(1), listOf<Byte>(2)), sink.got)
        link.close()
        link.push(byteArrayOf(3))
        assertEquals(2, sink.got.size)
        assertTrue(sink.disconnected)
        // Disconnected, then the ports stop, then the sink is let go of; once.
        assertEquals(listOf("stop", "release"), order)
        link.close()
        assertEquals(listOf("stop", "release"), order)
        // A sink attached after the close is disconnected and released at once.
        val late = Sink()
        link.attach(late) { order += "late" }
        assertTrue(late.disconnected)
        assertEquals("late", order.last())
    }
}
