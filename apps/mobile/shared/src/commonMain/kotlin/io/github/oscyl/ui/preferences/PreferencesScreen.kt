package io.github.oscyl.ui.preferences

import io.github.oscyl.ui.parts.safeInsets
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.oscyl.ui.parts.BackHeader
import io.github.oscyl.core.DriverInfo
import io.github.oscyl.core.Theme
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.action_back
import io.github.oscyl.shared.generated.resources.ic_back
import io.github.oscyl.shared.generated.resources.option_off
import io.github.oscyl.shared.generated.resources.option_on
import io.github.oscyl.shared.generated.resources.prefs_demo
import io.github.oscyl.shared.generated.resources.prefs_demo_body
import io.github.oscyl.shared.generated.resources.prefs_haptics
import io.github.oscyl.shared.generated.resources.prefs_haptics_body
import io.github.oscyl.shared.generated.resources.prefs_motion
import io.github.oscyl.shared.generated.resources.prefs_motion_body
import io.github.oscyl.shared.generated.resources.prefs_theme
import io.github.oscyl.shared.generated.resources.prefs_title
import io.github.oscyl.shared.generated.resources.theme_amoled
import io.github.oscyl.shared.generated.resources.theme_dark
import io.github.oscyl.shared.generated.resources.theme_light
import io.github.oscyl.shared.generated.resources.theme_system
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.Legend
import io.github.oscyl.ui.parts.Segments
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.shell.LocalIslandClearance
import org.jetbrains.compose.resources.stringResource

/** Preferences over the app's [PreferencesModel]. */
@Composable
fun PreferencesScreen(model: PreferencesModel, drivers: List<DriverInfo>, onBack: () -> Unit, modifier: Modifier = Modifier) {
    val prefs by model.prefs.collectAsStateWithLifecycle()
    PreferencesScreen(prefs, drivers, model, onBack, modifier)
}

/**
 * Preferences (the desktop's Appearance and the mobile switches): the theme, the app's
 * own animation switch, haptics, and demo mode with its synth. Booleans are Off/On
 * segments, as every two-way choice in Oscyl.
 */
@Composable
fun PreferencesScreen(prefs: Prefs, drivers: List<DriverInfo>, actions: PreferencesActions, onBack: () -> Unit, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val title = stringResource(Res.string.prefs_title)
    Column(
        modifier
            .fillMaxSize()
            .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))
            .verticalScroll(rememberScrollState())
            .padding(bottom = 24.dp + LocalIslandClearance.current),
    ) {
        BackHeader(title, onBack)
        Column(
            Modifier.padding(start = 16.dp, top = 8.dp, end = 16.dp).widthIn(max = 560.dp).fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(24.dp),
        ) {
            val themes = listOf(Theme.SYSTEM, Theme.LIGHT, Theme.DARK, Theme.AMOLED)
            ChoiceRow(
                stringResource(Res.string.prefs_theme),
                null,
                listOf(
                    stringResource(Res.string.theme_system),
                    stringResource(Res.string.theme_light),
                    stringResource(Res.string.theme_dark),
                    stringResource(Res.string.theme_amoled),
                ),
                themes.indexOf(prefs.theme),
            ) { actions.setTheme(themes[it]) }
            OffOn(stringResource(Res.string.prefs_motion), stringResource(Res.string.prefs_motion_body), prefs.motion, actions::setMotion)
            OffOn(stringResource(Res.string.prefs_haptics), stringResource(Res.string.prefs_haptics_body), prefs.haptics, actions::setHaptics)
            ChoiceRow(
                stringResource(Res.string.prefs_demo),
                stringResource(Res.string.prefs_demo_body),
                listOf(stringResource(Res.string.option_off)) + drivers.map { it.model },
                prefs.demo?.let { id -> drivers.indexOfFirst { it.id == id } + 1 }?.takeIf { it > 0 } ?: 0,
            ) { actions.setDemo(if (it == 0) null else drivers[it - 1].id) }
        }
    }
}

@Composable
private fun OffOn(label: String, body: String, on: Boolean, onChange: (Boolean) -> Unit) =
    ChoiceRow(label, body, listOf(stringResource(Res.string.option_off), stringResource(Res.string.option_on)), if (on) 1 else 0) { onChange(it == 1) }

/** A legend, a line on what it does, and its segments (the chosen one is the one kept: no saved notch). */
@Composable
internal fun ChoiceRow(label: String, body: String?, options: List<String>, selected: Int, onSelect: (Int) -> Unit) {
    val c = OscylTheme.colors
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Legend(label)
        if (body != null) {
            BasicText(body, style = MaterialTheme.typography.bodyMedium.copy(color = c.legend, textDirection = TextDirection.Content))
        }
        Segments(options, selected, saved = selected, onSelect = onSelect, modifier = Modifier.semantics { contentDescription = label }, index = false)
    }
}

private val SampleDrivers = listOf(
    DriverInfo("korg-minilogue-xd", "KORG", "minilogue xd"),
    DriverInfo("korg-prologue", "KORG", "prologue"),
    DriverInfo("sequential-prophet-rev2", "Sequential", "Prophet Rev2"),
)

@Composable
private fun PreferencesPreview(mode: OscylThemeMode = OscylThemeMode.Dark, prefs: Prefs = Prefs(theme = Theme.AMOLED, haptics = false)) =
    io.github.oscyl.ui.preview.PreviewTheme(mode) { PreferencesScreen(prefs, SampleDrivers, NoPreferencesActions, onBack = {}) }

@OscylPreviews
@Composable
private fun preferences(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreferencesPreview(mode)

@OscylPreviews
@Composable
private fun preferences_demo(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    PreferencesPreview(mode, Prefs(motion = false, demo = "korg-prologue"))

@OscylA11yPreviews
@Composable
private fun preferences_a11y() = PreferencesPreview()
