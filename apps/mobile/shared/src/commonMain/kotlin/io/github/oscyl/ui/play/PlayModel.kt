package io.github.oscyl.ui.play

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.oscyl.core.PlayLayout
import io.github.oscyl.ui.editor.EditorModel
import io.github.oscyl.ui.preferences.PreferencesModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/** The play page's controls: the pad's two axes and the four macro knobs. */
enum class PlaySlot { X, Y, M1, M2, M3, M4 }

/** No control assigned. */
val EmptyPlayLayout = PlayLayout(null, null, List(4) { null })

/** What a slot of [this] is assigned to. */
fun PlayLayout.of(slot: PlaySlot): UInt? = when (slot) {
    PlaySlot.X -> x
    PlaySlot.Y -> y
    else -> macros.getOrNull(slot.ordinal - 2)
}

/**
 * The play page's layout for the open program's synth model: the one chosen for that
 * model (kept in the settings), else the core's defaults for it. Changing a control
 * keeps the layout for the model, so every program of it opens the same.
 */
class PlayModel(private val editor: EditorModel, private val prefs: PreferencesModel) : ViewModel() {
    private val _layout = MutableStateFlow(EmptyPlayLayout)
    val layout: StateFlow<PlayLayout> = _layout.asStateFlow()

    init {
        viewModelScope.launch {
            editor.driverId.collect { driver ->
                _layout.value = driver?.let { prefs.playLayout(it) ?: editor.playDefaults() } ?: EmptyPlayLayout
            }
        }
    }

    /** Put [field] (null: none) on [slot] and keep the layout for the model. */
    fun assign(slot: PlaySlot, field: UInt?) {
        val driver = editor.driverId.value ?: return
        val l = _layout.value
        val next = when (slot) {
            PlaySlot.X -> l.copy(x = field)
            PlaySlot.Y -> l.copy(y = field)
            else -> l.copy(macros = l.macros.toMutableList().also { it[slot.ordinal - 2] = field })
        }
        _layout.value = next
        prefs.setPlayLayout(driver, next)
    }
}
