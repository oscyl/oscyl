package io.github.oscyl.platform

import androidx.compose.runtime.staticCompositionLocalOf

/** Oscyl's touch feedbacks; every haptic in the app goes through one of these. */
interface OscylHaptics {
    /** A light tick: a printed scale step, a stage passing under the finger. */
    fun tick()

    /** A firmer detent: the saved value, a knob's centre. */
    fun detent()

    /** The firmest: a knob reaching an end stop. Platforms without one give a detent. */
    fun bump() = detent()

    /** A confirmation: an action has taken effect. */
    fun confirm()
}

/** Feels nothing: previews, tests and platforms without haptics. */
object NoHaptics : OscylHaptics {
    override fun tick() = Unit
    override fun detent() = Unit
    override fun confirm() = Unit
}

/** [inner], silent while [enabled] says no (the Preferences switch). */
class GatedHaptics(private val inner: OscylHaptics, private val enabled: () -> Boolean) : OscylHaptics {
    override fun tick() { if (enabled()) inner.tick() }
    override fun detent() { if (enabled()) inner.detent() }
    override fun bump() { if (enabled()) inner.bump() }
    override fun confirm() { if (enabled()) inner.confirm() }
}

/** The platform's haptics, provided by [io.github.oscyl.ui.shell.App]. */
val LocalOscylHaptics = staticCompositionLocalOf<OscylHaptics> { NoHaptics }
