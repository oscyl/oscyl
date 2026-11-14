package io.github.oscyl.ui.parts

import androidx.compose.ui.semantics.disabled
import io.github.oscyl.platform.LocalOscylHaptics
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.ic_redo
import io.github.oscyl.shared.generated.resources.ic_undo
import io.github.oscyl.shared.generated.resources.programs_redo
import io.github.oscyl.shared.generated.resources.programs_undo
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import org.jetbrains.compose.resources.DrawableResource
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.stringResource

/**
 * A flat header icon button (DESIGN.md: undo, redo, menu have no outline): a 20 dp
 * glyph in `ink` in a 48 dp target, `press` while held, `legend` and inert when not
 * [enabled]. [label] is what a screen reader says.
 */
@Composable
fun IconAction(
    icon: DrawableResource,
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    /** A long press and what a screen reader calls it; it answers even when the tap is not [enabled]. */
    onLongClick: (() -> Unit)? = null,
    onLongClickLabel: String? = null,
    /** Its glyph's ink when not the usual (a quieter `legend` on rows that are not chosen). */
    tint: androidx.compose.ui.graphics.Color? = null,
) {
    val haptics = LocalOscylHaptics.current
    val c = OscylTheme.colors
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    Box(
        modifier
            .size(48.dp)
            .then(
                if (onLongClick == null) {
                    Modifier.clickable(interaction, indication = null, enabled = enabled, role = Role.Button, onClick = onClick)
                } else {
                    Modifier.combinedClickable(
                        interactionSource = interaction,
                        indication = null,
                        role = Role.Button,
                        onLongClickLabel = onLongClickLabel,
                        onLongClick = { haptics.confirm(); onLongClick() },
                        onClick = { if (enabled) onClick() },
                    )
                },
            )
            .semantics {
                contentDescription = label
                if (!enabled && onLongClick != null) disabled()
            }
            .padding(4.dp)
            .touchStates(interaction, shape, c.press),
        contentAlignment = Alignment.Center,
    ) {
        Image(
            painterResource(icon),
            null,
            Modifier.size(20.dp),
            colorFilter = ColorFilter.tint(if (enabled) tint ?: c.ink else c.disabled),
        )
    }
}

@OscylA11yPreviews
@Composable
private fun iconAction_a11y() = iconAction(OscylThemeMode.Dark)

@OscylPreviews
@Composable
private fun iconAction(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    Row {
        IconAction(Res.drawable.ic_undo, stringResource(Res.string.programs_undo), {})
        IconAction(Res.drawable.ic_redo, stringResource(Res.string.programs_redo), {}, enabled = false)
    }
}
