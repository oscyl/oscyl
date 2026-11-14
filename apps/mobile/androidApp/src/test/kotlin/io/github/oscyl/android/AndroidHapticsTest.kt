package io.github.oscyl.android

import android.view.HapticFeedbackConstants
import android.view.View
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.platform.GatedHaptics
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(AndroidJUnit4::class)
@Config(sdk = [36])
class AndroidHapticsTest {
    private val context = ApplicationProvider.getApplicationContext<android.app.Application>()
    private val view = View(context)
    private var enabled = true
    private val haptics = GatedHaptics(AndroidHaptics(view)) { enabled }
    private val last get() = shadowOf(view).lastHapticFeedbackPerformed()

    @Test
    fun hapticsOffSendsNothing() {
        enabled = false
        haptics.tick(); haptics.detent(); haptics.confirm()
        assertEquals(-1, last)

        enabled = true
        haptics.tick()
        assertEquals(HapticFeedbackConstants.CLOCK_TICK, last)
        haptics.detent()
        assertEquals(HapticFeedbackConstants.SEGMENT_TICK, last)
        haptics.confirm()
        assertEquals(HapticFeedbackConstants.CONFIRM, last)
    }
}
