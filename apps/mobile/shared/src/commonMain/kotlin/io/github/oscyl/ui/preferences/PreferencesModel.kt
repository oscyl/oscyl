package io.github.oscyl.ui.preferences

import io.github.oscyl.core.PlayLayout
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.oscyl.core.FfiException
import io.github.oscyl.core.Role
import io.github.oscyl.core.Settings
import io.github.oscyl.core.Theme
import io.github.oscyl.theme.OscylThemeMode
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** The preferences as the app shows them. */
data class Prefs(
    val theme: Theme = Theme.SYSTEM,
    val motion: Boolean = true,
    val haptics: Boolean = true,
    /** Demo mode: the driver whose simulated synth connects at start. */
    val demo: String? = null,
    /** The welcome has been answered (or there is no settings file: previews, tests). */
    val onboarded: Boolean = true,
    /** What the person mainly does, as they said at the welcome. */
    val role: Role? = null,
    /** The one-time hints already shown; null (no settings file: previews, tests) counts every one as shown. */
    val taught: Set<String>? = null,
) {
    /** Hint [name] has been shown. */
    fun taught(name: String): Boolean = taught == null || name in taught
}

/**
 * One-time hints: whether hint `name` was shown ([taught]), and saying it has been
 * ([teach]). Without a provider every hint counts as shown (previews, tests).
 */
class Teaching(val taught: (String) -> Boolean, val teach: (String) -> Unit) {
    /** [show] the hint [name] if it has not been shown, and keep it as shown. */
    inline fun once(name: String, show: () -> Unit) {
        if (taught(name)) return
        teach(name)
        show()
    }
}

/**
 * The one-time hints' names (DESIGN.md "First-run hints"). A hint about a gesture is
 * retired by doing the gesture, shown or not: [Teaching.teach] it where the gesture lands.
 */
object Hints {
    /** Gliding along the stage rail. */
    const val RAIL_GLIDE = "rail-glide"

    /** Holding a program's row, then moving, reorders. */
    const val ROW_DRAG = "row-drag"

    /** The search key of a long list. */
    const val SEARCH = "search"

    /** Holding Save shows the changes first. */
    const val SAVE_REVIEW = "save-review"
}

val LocalTeaching = androidx.compose.runtime.staticCompositionLocalOf { Teaching({ true }, {}) }

/** The theme mode a preference asks for. */
val Theme.mode: OscylThemeMode
    get() = when (this) {
        Theme.SYSTEM -> OscylThemeMode.System
        Theme.LIGHT -> OscylThemeMode.Light
        Theme.DARK -> OscylThemeMode.Dark
        Theme.AMOLED -> OscylThemeMode.Amoled
    }

/** What Preferences can change; every default does nothing (previews). */
interface PreferencesActions {
    fun setTheme(theme: Theme) {}
    fun setMotion(on: Boolean) {}
    fun setHaptics(on: Boolean) {}
    fun setDemo(driverId: String?) {}
}

/** Does nothing: previews. */
object NoPreferencesActions : PreferencesActions

/**
 * The person's preferences, kept by the core (`oscyl-app` settings) in the file at
 * [path] (null: previews, nothing kept). Each change is written at once, in order, off
 * the main thread, so it survives the process going away.
 */
class PreferencesModel(private val path: String?) : ViewModel(), PreferencesActions {
    // ponytail: the file is a few bytes and the first frame needs the theme, so it is read here, not on IO.
    private val settings: Settings? = path?.let { Settings.load(it) }
    private val _prefs = MutableStateFlow(settings?.let(::read) ?: Prefs())
    val prefs: StateFlow<Prefs> = _prefs.asStateFlow()

    /** Writes, one at a time, in order. */
    private val io = Dispatchers.IO.limitedParallelism(1)

    /** The demo of [Prefs.demo] has been opened in this process. */
    var demoOpened = false

    private fun read(s: Settings) = Prefs(s.theme(), s.motion(), s.haptics(), s.demo(), s.onboarded(), s.role(), s.taught().toSet())

    private fun change(apply: (Settings) -> Unit) {
        val s = settings ?: return
        apply(s)
        _prefs.value = read(s)
        viewModelScope.launch {
            try {
                withContext(io + NonCancellable) { s.save() }
            } catch (_: FfiException) {
                // ponytail: a preference that could not be written applies for this run; no notice yet.
            }
        }
    }

    override fun setTheme(theme: Theme) = change { it.setTheme(theme) }

    /** The play page's layout chosen for model [driverId], if one was. Kept in the one settings file. */
    fun playLayout(driverId: String): PlayLayout? = settings?.playLayout(driverId)

    fun setPlayLayout(driverId: String, layout: PlayLayout) = change { it.setPlayLayout(driverId, layout) }

    override fun setMotion(on: Boolean) = change { it.setMotion(on) }

    override fun setHaptics(on: Boolean) = change { it.setHaptics(on) }

    override fun setDemo(driverId: String?) = change { it.setDemo(driverId) }

    /**
     * The welcome answered: [role] (null when skipped) and the demo synth to connect
     * ([demo], null for none). It never shows again.
     */
    fun finishWelcome(role: Role?, demo: String?) = change {
        it.setOnboarded(true)
        it.setRole(role)
        if (demo != null) it.setDemo(demo)
    }

    /** Hint [name] has been shown: it is not shown again. */
    fun teach(name: String) {
        if (!_prefs.value.taught(name)) change { it.setTaught(name) }
    }

    /** What the person mainly does, answered later (the Library's hint): where a program opens. */
    fun setRole(role: Role?) = change { it.setRole(role) }

    override fun onCleared() {
        settings?.close()
    }
}
