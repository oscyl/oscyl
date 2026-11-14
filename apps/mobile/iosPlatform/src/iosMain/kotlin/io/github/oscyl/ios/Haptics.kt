package io.github.oscyl.ios

import platform.UIKit.UIImpactFeedbackGenerator
import platform.UIKit.UIImpactFeedbackStyle
import platform.UIKit.UINotificationFeedbackGenerator
import platform.UIKit.UINotificationFeedbackType
import platform.UIKit.UISelectionFeedbackGenerator

/**
 * Oscyl's three touch feedbacks with iOS's generators (the system's own Sounds &
 * Haptics setting applies too): a selection tick, a rigid impact for a detent, and a
 * success notification for a confirmation. Main thread only, as UIKit asks.
 */
class Haptics {
    private val selection = UISelectionFeedbackGenerator()
    private val impact = UIImpactFeedbackGenerator(style = UIImpactFeedbackStyle.UIImpactFeedbackStyleRigid)
    private val notification = UINotificationFeedbackGenerator()

    init {
        selection.prepare()
        impact.prepare()
    }

    // Each feedback readies its generator for the next, so it comes without a lag.
    fun tick() {
        selection.selectionChanged()
        selection.prepare()
    }

    fun detent() {
        impact.impactOccurred()
        impact.prepare()
    }

    fun confirm() {
        notification.notificationOccurred(UINotificationFeedbackType.UINotificationFeedbackTypeSuccess)
        notification.prepare()
    }
}
