package io.github.oscyl.ui.parts

import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.runtime.getValue
import androidx.compose.ui.graphics.graphicsLayer
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.shell.ChromeScale
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import io.github.oscyl.platform.LocalOscylHaptics
import io.github.oscyl.theme.OscylTheme
import org.jetbrains.compose.resources.DrawableResource
import org.jetbrains.compose.resources.painterResource

/**
 * A page's own action, floating at the foot's end above the island (DESIGN.md "Foot keys"):
 * 56 dp tall, the island's height and corner, an icon and a word. The page scrolls under
 * it. [primary] is the page's point and stands on the plate, like the Save key; otherwise
 * it floats like the island. A change between the two crossfades the plate on `base`, as
 * the Save key does: nothing is laid out again. [said] is what a screen reader hears when
 * the word is short. Its words follow the chrome's scale ([ChromeScale]).
 */
@Composable
fun FootKey(
    label: String,
    icon: DrawableResource?,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    primary: Boolean = false,
    enabled: Boolean = true,
    said: String = label,
) = ChromeScale {
    val c = OscylTheme.colors
    val haptics = LocalOscylHaptics.current
    val motion = LocalOscylMotion.current
    val shape = RoundedCornerShape(FloatingRadius)
    val interaction = remember { MutableInteractionSource() }
    val plate by animateFloatAsState(if (primary) 1f else 0f, tween(if (motion) OscylTokens.baseMs else 0, easing = OscylTokens.easing))
    val ink by animateColorAsState(
        when {
            !enabled -> c.legend
            primary -> c.onPlate
            else -> c.ink
        },
        tween(if (motion) OscylTokens.baseMs else 0, easing = OscylTokens.easing),
    )
    val act = { haptics.tick(); onClick() }
    Box(
        modifier
            .heightIn(min = 56.dp)
            .floating(c, shape)
            .clickable(interaction, indication = null, enabled = enabled, role = Role.Button, onClick = act)
            // Its word is its name (a shorter word than the action's says it in full).
            .semantics(mergeDescendants = true) { if (said != label) contentDescription = said }
            .touchStates(interaction, shape, if (primary) c.ink.copy(alpha = 0.22f) else c.press),
        contentAlignment = Alignment.Center,
    ) {
        Box(Modifier.matchParentSize().graphicsLayer { alpha = plate }.liftedPlate(c, shape))
        Row(
            Modifier.padding(start = if (icon != null) 16.dp else 20.dp, end = 20.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            if (icon != null) Image(painterResource(icon), null, Modifier.size(20.dp), colorFilter = ColorFilter.tint(ink))
            BasicText(label, style = MaterialTheme.typography.labelLarge.copy(color = ink, fontWeight = FontWeight.SemiBold), maxLines = 1)
        }
    }
}
