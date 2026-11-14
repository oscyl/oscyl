package io.github.oscyl.ui.parts

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes

/**
 * The orange count box (DESIGN.md `edit-count`): tabular figures at 700 on `act`,
 * 6 dp corners, never a pill. Decorative: the action it sits in says the count in words.
 * Above [max] it reads "[max]+".
 */
@Composable
fun CountBox(count: Int, modifier: Modifier = Modifier, max: Int = Int.MAX_VALUE) {
    val c = OscylTheme.colors
    Box(
        modifier
            .heightIn(min = 18.dp)
            .widthIn(min = 18.dp)
            .background(c.act, RoundedCornerShape(OscylTokens.radius))
            .padding(horizontal = 4.dp),
        contentAlignment = Alignment.Center,
    ) {
        BasicText(
            // Latin digits, as the slots and values around it (one digit system on a page).
            if (count > max) "$max+" else count.toString(),
            style = MaterialTheme.typography.labelMedium.copy(
                color = c.onAct,
                fontWeight = FontWeight.Bold,
                fontFeatureSettings = "tnum",
                letterSpacing = 0.sp,
                textAlign = TextAlign.Center,
            ),
        )
    }
}

@OscylA11yPreviews
@Composable
private fun countBox_a11y() = countBox(OscylThemeMode.Dark)

@OscylPreviews
@Composable
private fun countBox(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        CountBox(1)
        CountBox(4)
        CountBox(128)
    }
}
