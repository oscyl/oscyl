package io.github.oscyl.ui.library

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class NoticeTimeoutTest {
    @Test
    fun eachNoticeStaysAsLongAsItNeeds() {
        assertEquals(4_000L, noticeTimeout(Notice.Copied("Velvet Pad"), 20, screenReader = false))
        assertEquals(10_000L, noticeTimeout(Notice.Removed("Velvet Pad"), 20, screenReader = false))
        // A hint as long as its words take to read, 4 to 10 s.
        assertEquals(7_500L, noticeTimeout(Notice.CompareHint, 125, screenReader = false))
        assertEquals(4_000L, noticeTimeout(Notice.CompareHint, 10, screenReader = false))
        assertEquals(10_000L, noticeTimeout(Notice.CompareHint, 400, screenReader = false))
        // A failed save keeps its Retry; a screen reader keeps every action.
        assertNull(noticeTimeout(Notice.SaveFailed("Gig set", "No room."), 30, screenReader = false))
        assertNull(noticeTimeout(Notice.Removed("Velvet Pad"), 20, screenReader = true))
        assertEquals(10_000L, noticeTimeout(Notice.Copied("Velvet Pad"), 20, screenReader = true))
    }
}
