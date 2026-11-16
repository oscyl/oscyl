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
import io.github.oscyl.ui.synth.Reopen
import io.github.oscyl.ui.synth.SynthCard
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
import kotlin.time.Duration.Companion.seconds
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

    private fun show(midi: MidiPlatform, reopen: Reopen = Reopen()): SynthModel {
        val model = ViewModelProvider.create(store, viewModelFactory { initializer { SynthModel(midi, reopen = reopen) } })[SynthModel::class]
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

    /** Waits until synth [card]'s every program has been read and it is connected. */
    private fun awaitRead(model: SynthModel, timeout: Long = TIMEOUT_MS) = compose.waitUntil(timeout) {
        compose.runOnIdle {
            model.cards.value.isNotEmpty() && model.cards.value.all { c -> c.connected && c.read == null && model.document(c.id)?.allRead() == true }
        }
    }

    private fun awaitCards(model: SynthModel, test: (List<SynthCard>) -> Boolean) =
        compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle { test(model.cards.value) } }

    /** Its cable out, the synth stays with its programs, waiting to be plugged in again. */
    @Test
    fun unplugKeepsTheCardAndItsPrograms() {
        val midi = FakeMidi(XD)
        val model = show(midi)
        awaitRead(model)
        midi.unplug(XD)
        awaitCards(model) { it.singleOrNull()?.connected == false }
        val card = model.cards.value.single()
        assertTrue(model.document(card.id)!!.allRead())
        awaitText("Plug it in to reconnect")
        assertTrue(compose.onAllNodesWithContentDescription("Disconnect", substring = true).fetchSemanticsNodes().isEmpty())
    }

    /** Plugged in again (Android gives it a new id), it comes back to its own card. */
    @Test
    fun replugReattachesToTheSameCard() {
        val midi = FakeMidi(XD)
        val model = show(midi)
        awaitRead(model)
        val before = model.cards.value.single()
        midi.unplug(XD)
        awaitCards(model) { it.singleOrNull()?.connected == false }
        midi.replug(XD.copy(id = "9"))
        awaitRead(model)
        val after = model.cards.value.single()
        assertEquals(before.id, after.id)
        awaitText("Connected, firmware 2.10")
    }

    /** An edit made while the synth was away is still there when it is back. */
    @Test
    fun editsSurviveUnplugAndReplug() {
        val midi = FakeMidi(XD)
        val model = show(midi)
        awaitRead(model)
        val id = model.cards.value.single().id
        midi.unplug(XD)
        awaitCards(model) { it.singleOrNull()?.connected == false }
        val (field, to) = compose.runOnIdle {
            model.document(id)!!.editor(3u).use { e ->
                val f = e.fields(e.firstPage()).first { it.control is io.github.oscyl.core.ControlKind.Dial }
                val to = if (f.raw > 500u) f.raw - 100u else f.raw + 100u
                e.set(f.field, to)
                f.field to to
            }
        }
        midi.replug(XD.copy(id = "9"))
        awaitRead(model)
        compose.runOnIdle {
            model.document(id)!!.editor(3u).use { e ->
                assertTrue(e.field(field).edited)
                assertEquals(to, e.field(field).raw)
            }
            assertTrue(model.kept.tryReceive().isFailure)
        }
    }

    /** A synth that stops answering (asked too soon after it booted) is let go and opened again. */
    @Test
    fun aSynthThatStopsAnsweringIsReopened() {
        val midi = FakeMidi(XD).apply { muteOpens = 1 }
        val model = show(midi)
        awaitRead(model, timeout = 30_000L)
        assertEquals(listOf(XD), midi.released)
        assertEquals(2, midi.opens)
    }

    /**
     * A synth that answers the identity after every reopen but never a read is not
     * reopened forever: the person hears it is reconnecting, then to power-cycle it.
     */
    @Test
    fun aSynthThatNeverAnswersReadsIsGivenUp() {
        val midi = FakeMidi(XD).apply { muteOpens = 99 }
        val model = show(midi, Reopen(gap = 1.seconds, attempts = 2))
        compose.waitUntil(20_000L) { compose.runOnIdle { model.notice.value?.contains("Reconnecting") == true } }
        compose.waitUntil(45_000L) { compose.runOnIdle { model.notice.value?.contains("Turn it off and on again") == true } }
        assertEquals(3, midi.opens)
        assertTrue(model.cards.value.single().let { !it.connected && it.plugged })
    }

    /** A synth the person disconnected (cable in) keeps its card when another of its model is replugged. */
    @Test
    fun aDisconnectedSynthKeepsItsCardWhenAnotherIsReplugged() {
        val other = MidiPortInfo(name = "minilogue xd _ SOUND", id = "8")
        val midi = FakeMidi(XD, other)
        val model = show(midi)
        awaitCards(model) { it.size == 2 }
        awaitRead(model, timeout = 20_000L)
        val mine = model.cards.value.single { it.port.startsWith("7:") }
        val theirs = model.cards.value.single { it.port.startsWith("8:") }
        compose.runOnIdle { model.disconnect(mine) }
        awaitCards(model) { cards -> cards.single { it.id == mine.id }.connected.not() }
        midi.unplug(other)
        awaitCards(model) { cards -> cards.single { it.id == theirs.id }.connected.not() }
        midi.replug(other.copy(id = "11"))
        awaitCards(model) { cards -> cards.single { it.id == theirs.id }.connected }
        assertTrue(model.cards.value.single { it.id == mine.id }.let { !it.connected && it.port.startsWith("7:") })
    }

    /** Two synths of one model: only the one plugged out and in again goes away and comes back. */
    @Test
    fun twoSynthsOnlyTheReplugggedOneReattaches() {
        val other = MidiPortInfo(name = "minilogue xd _ SOUND", id = "8")
        val midi = FakeMidi(XD, other)
        val model = show(midi)
        awaitCards(model) { it.size == 2 }
        awaitRead(model, timeout = 20_000L)
        val ids = model.cards.value.map { it.id }.toSet()
        midi.unplug(other)
        awaitCards(model) { cards -> cards.count { !it.connected } == 1 }
        midi.replug(other.copy(id = "11"))
        awaitRead(model, timeout = 20_000L)
        assertEquals(ids, model.cards.value.map { it.id }.toSet())
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

    /** How many opens give a synth that answers the identity, then nothing (a wedged xd). */
    var muteOpens = 0

    /** How many ports were opened. */
    var opens = 0

    /** Synths opened from now on hold every reply but the identity until [answer]. */
    var holdReplies = false

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
        opens++
        val xd = FakeXd(holdReplies || muteOpens-- > 0)
        open[port.id] = xd
        return OpenResult.Opened(xd, xd::attach)
    }

    /** The synths' panels send [bytes] (a program picked, say). */
    fun panel(bytes: ByteArray) = synchronized(open) { open.values.forEach { it.panel(bytes) } }

    /** Deliver every reply the open synths held. */
    fun answer() = synchronized(open) { open.values.forEach(FakeXd::answer) }

    /** The cable goes in: the platform lists the port (Android gives it a new id). */
    fun replug(port: MidiPortInfo) = ports.update { it + port }

    /** The cable comes out: the sink goes first, then the port. */
    fun unplug(port: MidiPortInfo) {
        open.remove(port.id)?.detach()
        ports.update { it - port }
    }
}

/** A minilogue xd behind a fake USB port: the core's simulator answers what the app sends. */
private class FakeXd(private var holding: Boolean = false) : MidiOut {
    private var sink: MidiInputSink? = null
    private val synth = io.github.oscyl.core.Simulator("korg-minilogue-xd")

    /** Replies given before the app attached its input, or held. */
    private val waiting = mutableListOf<ByteArray>()
    val sent = java.util.Collections.synchronizedList(mutableListOf<ByteArray>())

    @Synchronized
    override fun send(bytes: ByteArray) {
        sent += bytes
        for (reply in synth.handle(bytes)) {
            val identity = reply.size > 1 && reply[1] == 0x7E.toByte()
            val now = sink.takeIf { !holding || identity }
            now?.push(reply) ?: waiting.add(reply)
        }
    }

    @Synchronized
    fun attach(sink: MidiInputSink) {
        this.sink = sink
        val (now, held) = waiting.partition { !holding || (it.size > 1 && it[1] == 0x7E.toByte()) }
        now.forEach(sink::push)
        waiting.retainAll(held)
    }

    /** Its panel sends [bytes]. */
    @Synchronized
    fun panel(bytes: ByteArray) {
        sink?.push(bytes)
    }

    /** Stop holding: what was held goes out now. */
    @Synchronized
    fun answer() {
        holding = false
        sink?.let { s -> waiting.forEach(s::push) }
        if (sink != null) waiting.clear()
    }

    @Synchronized
    fun detach() {
        sink?.disconnect()
        sink?.close()
        sink = null
    }
}
