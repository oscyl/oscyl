package io.github.oscyl.ui.parts

import io.github.oscyl.shared.generated.resources.ic_chevron
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.destination_synth
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import org.jetbrains.compose.resources.stringResource

/**
 * The outlined button (DESIGN.md `button-outlined`): a 1 dp `rule-strong` ring with
 * 6 dp corners and no fill, `ink` text at 500, `press` while held. The outline is
 * 36 dp tall inside a 48 dp touch target. Destructive actions use this too, never orange.
 * Pressed, `press` is there at once; focused, the 2 dp ink ring; disabled, `legend` on `rule`.
 */
@Composable
fun OutlinedAction(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    leading: (@Composable () -> Unit)? = null,
    /** Not enabled, it keeps its place in `legend` on a `rule` ring and does nothing (said as disabled). */
    enabled: Boolean = true,
    /** After the words: a picker's chevron. */
    trailing: (@Composable () -> Unit)? = null,
    /** What a screen reader calls it ([Role.DropdownList] for one that opens a choice). */
    role: Role = Role.Button,
) {
    val c = OscylTheme.colors
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    Box(
        modifier
            .defaultMinSize(minWidth = 48.dp, minHeight = 48.dp)
            .clickable(interactionSource = interaction, indication = null, enabled = enabled, role = role, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Row(
            Modifier
                .heightIn(min = 36.dp)
                .border(OscylTokens.ruleWidth, if (enabled) c.ruleStrong else c.rule, shape)
                .touchStates(interaction, shape, c.press)
                .padding(horizontal = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            leading?.invoke()
            BasicText(text, style = MaterialTheme.typography.bodyMedium.copy(color = if (enabled) c.ink else c.legend, fontWeight = FontWeight.Medium))
            trailing?.invoke()
        }
    }
}

@OscylA11yPreviews
@Composable
private fun outlinedAction_a11y() = outlinedAction(OscylThemeMode.Dark)

@OscylPreviews
@Composable
private fun outlinedAction(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    androidx.compose.foundation.layout.Column {
        OutlinedAction(stringResource(Res.string.destination_synth), onClick = {}, leading = { Lamp(lit = true) })
        OutlinedAction(stringResource(Res.string.destination_synth), onClick = {}, enabled = false)
    }
}

/** A picker's trailing chevron, in `legend` (as a select field's). */
@Composable
fun PickerChevron() = androidx.compose.foundation.Image(
    org.jetbrains.compose.resources.painterResource(Res.drawable.ic_chevron),
    null,
    Modifier.size(16.dp),
    colorFilter = androidx.compose.ui.graphics.ColorFilter.tint(OscylTheme.colors.legend),
)
