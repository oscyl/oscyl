package io.github.oscyl.ui.shell

import androidx.compose.foundation.layout.heightIn
import io.github.oscyl.ui.preview.OscylSizePreviews
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.parts.floating
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes

/**
 * The island for medium and expanded windows: the same parts stood on end, floating
 * at the start edge. Destinations on their sliding plate, the Save key under it; as wide as
 * its widest label, so large text widens it rather than clipping.
 */
@Composable
fun SideIsland(
    current: Destination,
    onSelect: (Destination) -> Unit,
    saveCount: Int,
    onSave: () -> Unit,
    modifier: Modifier = Modifier,
    sync: Boolean = false,
    onReview: () -> Unit = onSave,
    progress: Float? = null,
    confirmed: Boolean = false,
) = ChromeScale {
    Column(
        // Shell gives it medium height or more; scrolling is the floor, never clipping.
        modifier.width(IntrinsicSize.Max).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(Modifier.fillMaxWidth().floating(OscylTheme.colors).padding(6.dp)) {
            DestinationTabs(current, onSelect, vertical = true, Modifier.fillMaxWidth())
        }
        // As wide as the destinations above it and built as they are (a drawing over its word), a tab's height.
        SavePlate(saveCount, sync, onSave, Modifier.fillMaxWidth().heightIn(min = 64.dp), onReview = onReview, progress = progress, confirmed = confirmed)
    }
}

@OscylA11yPreviews
@Composable
private fun sideIsland_a11y() = PreviewTheme {
    SideIsland(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {})
}

@OscylPreviews
@Composable
private fun sideIsland(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    SideIsland(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {})
}

/** Where the side island stands: beside the content of a foldable and a tablet in landscape, centred on the start edge. */
@OscylSizePreviews
@Composable
private fun sideIsland_sizes() = PreviewTheme {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.CenterStart) {
        SideIsland(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {}, modifier = Modifier.padding(12.dp))
    }
}
