package io.github.oscyl.ui.library

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertSame
import kotlin.test.assertTrue

class KeepFirstTest {
    private class Doc : AutoCloseable {
        var closed = false

        override fun close() {
            closed = true
        }
    }

    @Test
    fun aLaterReadNeverReplacesTheOneInUse() {
        val docs = mutableMapOf<String, Doc>()
        val edited = Doc()
        val stale = Doc()
        docs.keepFirst("a", edited)
        docs.keepFirst("a", stale)
        assertSame(edited, docs["a"])
        assertFalse(edited.closed)
        assertTrue(stale.closed)
        assertEquals(1, docs.size)
    }
}
