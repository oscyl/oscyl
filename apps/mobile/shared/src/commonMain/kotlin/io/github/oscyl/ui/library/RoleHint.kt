package io.github.oscyl.ui.library

import io.github.oscyl.shared.generated.resources.onb_perform_body
import io.github.oscyl.shared.generated.resources.onb_organise_body
import io.github.oscyl.shared.generated.resources.onb_design_body
import androidx.compose.ui.text.style.TextDirection
import io.github.oscyl.theme.OscylTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import io.github.oscyl.core.Role
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.onb_design
import io.github.oscyl.shared.generated.resources.onb_organise
import io.github.oscyl.shared.generated.resources.onb_perform
import io.github.oscyl.shared.generated.resources.onb_question
import io.github.oscyl.ui.parts.Legend
import io.github.oscyl.ui.parts.OutlinedAction
import androidx.compose.foundation.layout.FlowRow
import org.jetbrains.compose.resources.stringResource

/** The roles in the desktop's order. */
private val Roles = listOf(Role.SOUND_DESIGN, Role.ORGANISE, Role.PERFORM)

/**
 * The first run's old question, now an optional hint in the Library's empty state:
 * what the person mainly does, which picks the page a program opens at. Nothing is
 * chosen until they choose ([onPick]); once answered it is gone.
 */
@Composable
fun RoleHint(onPick: (Role) -> Unit, modifier: Modifier = Modifier) {
    Column(modifier.fillMaxWidth().padding(top = 16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Legend(stringResource(Res.string.onb_question))
        // Each answer with what it does: where a program will open.
        listOf(
            Res.string.onb_design to Res.string.onb_design_body,
            Res.string.onb_organise to Res.string.onb_organise_body,
            Res.string.onb_perform to Res.string.onb_perform_body,
        ).forEachIndexed { i, (label, body) ->
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                OutlinedAction(stringResource(label), { onPick(Roles[i]) })
                BasicText(
                    stringResource(body),
                    Modifier.padding(start = 4.dp),
                    style = MaterialTheme.typography.bodySmall.copy(color = OscylTheme.colors.legend, textDirection = TextDirection.Content),
                )
            }
        }
    }
}
