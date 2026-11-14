package io.github.oscyl.ui.parts

import io.github.oscyl.ui.shell.ChromeScale
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.action_back
import io.github.oscyl.shared.generated.resources.ic_back
import io.github.oscyl.theme.OscylTheme
import org.jetbrains.compose.resources.stringResource

/**
 * A page above a destination: Back, then its [title] as the page's heading; blank, no
 * heading. Every such page alike, and in step with the top bars: a 56 dp row, the arrow
 * 4 dp from the edge, the title in the page titles' own size. Put it at the page's
 * edge: it brings its own inset.
 */
@Composable
fun BackHeader(title: String, onBack: () -> Unit, modifier: Modifier = Modifier) = ChromeScale {
    Row(modifier.heightIn(min = 56.dp).padding(horizontal = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        IconAction(Res.drawable.ic_back, stringResource(Res.string.action_back), onBack)
        // No heading at all until there is a title (a licence still loading): never an empty one.
        if (title.isBlank()) return@Row
        BasicText(
            title,
            Modifier.padding(start = 4.dp, end = 12.dp).semantics { heading() },
            style = MaterialTheme.typography.headlineSmall.copy(
                color = OscylTheme.colors.ink,
                fontWeight = FontWeight.SemiBold,
                textDirection = TextDirection.Content,
            ),
            // One line: a long name ends in an ellipsis rather than breaking the header.
            maxLines = 1,
            overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
        )
    }
}
