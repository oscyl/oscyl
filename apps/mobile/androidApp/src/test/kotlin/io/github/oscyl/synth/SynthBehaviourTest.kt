package io.github.oscyl.synth

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onAllNodesWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.core.MidiInputSink
import io.github.oscyl.core.MidiOut
import io.github.oscyl.platform.MidiPlatform
import io.github.oscyl.platform.MidiPortInfo
import io.github.oscyl.platform.OpenResult
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.synth.SynthModel
import io.github.oscyl.ui.synth.SynthScreen
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.update
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/**
 * The Synth screen over the real Rust worker (host bindings): the demo, a USB port
 * that answers like a minilogue xd, unplugging it, and USB access refused.
 */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class SynthBehaviourTest {
    @get:Rule val compose = createComposeRule()

    private val store = ViewModelStore()

    @After
    fun clear() = store.clear()

    private fun show(midi: MidiPlatform): SynthModel {
        val model = ViewModelProvider.create(store, viewModelFactory { initializer { SynthModel(midi) } })[SynthModel::class]
        compose.setContent { OscylTheme(OscylThemeMode.Dark) { SynthScreen(model, onAbout = {}) } }
        return model
    }

    private fun awaitText(text: String) = compose.waitUntil(TIMEOUT_MS) {
        compose.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty()
    }

    @Test
    fun demoConnectsAndShowsCard() {
        val model = show(FakeMidi())
        compose.onNodeWithText("Plug in your synth over USB").assertIsDisplayed()
        compose.runOnIdle { model.openDemo("korg-minilogue-xd") }
        awaitText("Simulated, no sound")
        val card = model.cards.value.single()
        assertEquals("minilogue xd", card.model)
        assertTrue(card.simulated)
        compose.onNodeWithText("minilogue xd").assertIsDisplayed()
        compose.onNodeWithContentDescription("Disconnect minilogue xd").assertIsDisplayed()
        // Connected, the empty state gives way to its row; adding another stays offered.
        assertTrue(compose.onAllNodesWithText("Plug in your synth over USB").fetchSemanticsNodes().isEmpty())
        assertTrue(compose.onAllNodesWithText("Add a synth").fetchSemanticsNodes().isNotEmpty())
    }

    @Test
    fun unplugShowsDisconnected() {
        val midi = FakeMidi(XD)
        val model = show(midi)
        awaitText("Connected, firmware 2.10")
        assertEquals(false, model.cards.value.single().simulated)
        midi.unplug(XD)
        awaitText("Plug in your synth over USB")
        assertTrue(model.cards.value.isEmpty())
        assertTrue(compose.onAllNodesWithContentDescription("Disconnect", substring = true).fetchSemanticsNodes().isEmpty())
    }

    @Test
    fun permissionDeniedShowsRetry() {
        val midi = FakeMidi(XD).apply { denials = 1 }
        show(midi)
        awaitText("Allow USB access")
        compose.onNodeWithText("Allow USB access").performClick()
        awaitText("Connected, firmware 2.10")
        assertTrue(compose.onAllNodesWithText("Allow USB access").fetchSemanticsNodes().isEmpty())
    }

    @Test
    fun disconnectThenConnectAgain() {
        show(FakeMidi(XD))
        awaitText("Connected, firmware 2.10")
        compose.onNodeWithContentDescription("Disconnect minilogue xd").performClick()
        awaitText("Disconnected")
        compose.onNodeWithText("Connect").performClick()
        awaitText("Connected, firmware 2.10")
    }

    @Test
    fun aWriteToASynthThatWentFailsAtOnce() {
        val model = show(FakeMidi())
        compose.runOnIdle { model.openDemo("korg-minilogue-xd") }
        compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle { model.cards.value.any { it.connected } } }
        val card = model.cards.value.first()
        compose.runOnIdle { model.disconnect(card) }
        compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle { model.cards.value.none { it.connected } } }
        compose.runOnIdle { model.write(card.id, listOf(io.github.oscyl.core.SlotProgram(0u, ByteArray(4)))) }
        compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle { model.writes.value[card.id] is io.github.oscyl.ui.synth.WriteState.Failed } }
    }

    private companion object {
        const val TIMEOUT_MS = 10_000L
        val XD = MidiPortInfo(name = "minilogue xd _ SOUND", id = "7")
    }

    @Test
    fun backupsReadAsWhatTheyHold() {
        assertEquals("Synth backup" to false, io.github.oscyl.ui.synth.backupWords("Synth backup 2026-11-13 10.04.11.mnlgxdlib"))
        assertEquals("MorphTbl" to true, io.github.oscyl.ui.synth.backupWords("MorphTbl 2026-11-12 18.20.03.slot.bin"))
        assertEquals("2026-11-13 · 10:04", io.github.oscyl.ui.synth.backupWhen("Synth backup 2026-11-13 10.04.11.mnlgxdlib"))
        assertEquals(null, io.github.oscyl.ui.synth.backupWords("someone else's.syx"))
    }
}

/** USB ports in a test: each answers the device inquiry like a minilogue xd 2.10. */
class FakeMidi(vararg plugged: MidiPortInfo) : MidiPlatform {
    override val ports = MutableStateFlow(plugged.toList())

    /** How many opens to refuse USB access before allowing it. */
    var denials = 0

    private val open = mutableMapOf<String, FakeXd>()

    /** The ports the app let go of. */
    val released = mutableListOf<MidiPortInfo>()

    override fun release(port: MidiPortInfo) {
        released += port
        open.remove(port.id)?.detach()
    }

    /** Everything the app sent to the synths, message by message. */
    val sent: List<ByteArray> get() = synchronized(open) { open.values.flatMap { it.sent } }

    override suspend fun open(port: MidiPortInfo): OpenResult {
        if (denials > 0) {
            denials--
            return OpenResult.PermissionDenied
        }
        open.remove(port.id)?.detach()
        val xd = FakeXd()
        open[port.id] = xd
        return OpenResult.Opened(xd, xd::attach)
    }

    /** The cable comes out: the sink goes first, then the port. */
    fun unplug(port: MidiPortInfo) {
        open.remove(port.id)?.detach()
        ports.update { it - port }
    }
}

private class FakeXd : MidiOut {
    private var sink: MidiInputSink? = null
    private var asked = false
    val sent = java.util.Collections.synchronizedList(mutableListOf<ByteArray>())

    @Synchronized
    override fun send(bytes: ByteArray) {
        sent += bytes
        // The universal device inquiry: F0 7E <channel> 06 01 F7.
        if (bytes.size >= 5 && bytes[1] == 0x7E.toByte() && bytes[3] == 0x06.toByte() && bytes[4] == 0x01.toByte()) {
            asked = true
            sink?.push(IDENTITY)
        }
    }

    @Synchronized
    fun attach(sink: MidiInputSink) {
        this.sink = sink
        if (asked) sink.push(IDENTITY)
    }

    @Synchronized
    fun detach() {
        sink?.disconnect()
        sink?.close()
        sink = null
    }

    private companion object {
        /** The minilogue xd's identity reply, firmware 2.10. */
        val IDENTITY = byteArrayOf(
            0xF0.toByte(), 0x7E, 0x00, 0x06, 0x02, 0x42, 0x51, 0x01, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00, 0xF7.toByte(),
        )
    }
}
