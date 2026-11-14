package io.github.oscyl.android.midi

import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** The MIDI device open's wait: what opens after the wait is gone is closed. */
class AwaitOpenedTest {
    private class Device : AutoCloseable {
        var closed = false
        override fun close() {
            closed = true
        }
    }

    @Test
    fun aDeviceThatOpensAfterTheWaitWentIsClosed() = runBlocking {
        var callback: ((Device?) -> Unit)? = null
        val wait = async(Dispatchers.Unconfined, start = CoroutineStart.UNDISPATCHED) { awaitOpened<Device> { callback = it } }
        // The model goes before the system answers.
        wait.cancel()
        yield()
        val late = Device()
        callback!!(late)
        assertTrue("closed, not leaked", late.closed)
    }

    @Test
    fun aDeviceThatOpensInTimeIsHandedOver() = runBlocking {
        val device = Device()
        val opened = awaitOpened<Device> { it(device) }
        assertEquals(device, opened)
        assertFalse(device.closed)
    }
}
