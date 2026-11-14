package io.github.oscyl.android

import android.os.Build
import android.view.HapticFeedbackConstants
import android.view.View
import io.github.oscyl.platform.OscylHaptics

/**
 * Oscyl's haptics through [view]: a clock tick, a segment tick (a firmer detent), a
 * threshold bump (an end stop) and a confirmation, each the closest older constant before the API that brought it. The
 * system's own touch-feedback switch applies as for any view; the app's Preferences
 * switch gates it in the shared UI (`GatedHaptics`).
 */
class AndroidHaptics(private val view: View) : OscylHaptics {
    override fun tick() = perform(HapticFeedbackConstants.CLOCK_TICK)

    override fun detent() = perform(
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            HapticFeedbackConstants.SEGMENT_TICK
        } else {
            HapticFeedbackConstants.CONTEXT_CLICK
        },
    )

    override fun bump() = perform(
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            HapticFeedbackConstants.GESTURE_THRESHOLD_ACTIVATE
        } else {
            HapticFeedbackConstants.LONG_PRESS
        },
    )

    override fun confirm() = perform(
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) HapticFeedbackConstants.CONFIRM else HapticFeedbackConstants.VIRTUAL_KEY,
    )

    // The system's touch-feedback setting is honoured by performHapticFeedback itself.
    private fun perform(feedback: Int) {
        view.performHapticFeedback(feedback)
    }
}
