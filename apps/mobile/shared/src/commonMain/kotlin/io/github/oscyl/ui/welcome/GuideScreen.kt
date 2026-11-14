package io.github.oscyl.ui.welcome

import io.github.oscyl.ui.parts.LineArt
import io.github.oscyl.ui.parts.EmptyArt
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import io.github.oscyl.ui.parts.BackHeader
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.action_back
import io.github.oscyl.shared.generated.resources.guide_auditioning
import io.github.oscyl.shared.generated.resources.guide_auditioning_body
import io.github.oscyl.shared.generated.resources.guide_connecting
import io.github.oscyl.shared.generated.resources.guide_connecting_body
import io.github.oscyl.shared.generated.resources.guide_editing
import io.github.oscyl.shared.generated.resources.guide_editing_body
import io.github.oscyl.shared.generated.resources.guide_generating
import io.github.oscyl.shared.generated.resources.guide_generating_body
import io.github.oscyl.shared.generated.resources.guide_offline
import io.github.oscyl.shared.generated.resources.guide_offline_body
import io.github.oscyl.shared.generated.resources.guide_syncing
import io.github.oscyl.shared.generated.resources.guide_syncing_body
import io.github.oscyl.shared.generated.resources.guide_title
import io.github.oscyl.shared.generated.resources.guide_units
import io.github.oscyl.shared.generated.resources.guide_units_body
import io.github.oscyl.shared.generated.resources.ic_back
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.shell.LocalIslandClearance
import org.jetbrains.compose.resources.stringResource

/** The desktop guide's chapters, worded for the phone, each under its own line drawing. */
private val Chapters = listOf(
    Triple(Res.string.guide_connecting, Res.string.guide_connecting_body, EmptyArt.Cable),
    Triple(Res.string.guide_offline, Res.string.guide_offline_body, EmptyArt.Shelf),
    Triple(Res.string.guide_editing, Res.string.guide_editing_body, EmptyArt.Knob),
    Triple(Res.string.guide_auditioning, Res.string.guide_auditioning_body, EmptyArt.Trace),
    Triple(Res.string.guide_generating, Res.string.guide_generating_body, EmptyArt.Tiles),
    Triple(Res.string.guide_syncing, Res.string.guide_syncing_body, EmptyArt.Sync),
    Triple(Res.string.guide_units, Res.string.guide_units_body, EmptyArt.Chip),
)

/** The guide: one page of chapters, each a heading screen readers can jump between. */
@Composable
fun GuideScreen(onBack: () -> Unit, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    Column(
        modifier
            .fillMaxSize()
            .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))
            .verticalScroll(rememberScrollState())
            .padding(bottom = 24.dp + LocalIslandClearance.current),
    ) {
        BackHeader(stringResource(Res.string.guide_title), onBack)
        Column(
            Modifier.padding(start = 16.dp, top = 8.dp, end = 16.dp).widthIn(max = 640.dp).fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(24.dp),
        ) {
            Chapters.forEach { (title, body, art) ->
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    LineArt(art, Modifier.padding(bottom = 8.dp))
                    BasicText(
                        stringResource(title),
                        Modifier.semantics { heading() },
                        style = type.titleMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold),
                    )
                    // One step per paragraph, a breath between them.
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        stringResource(body).split('\n').forEach { step ->
                            BasicText(step, style = type.bodyLarge.copy(color = c.ink, textDirection = TextDirection.Content))
                        }
                    }
                }
            }
        }
    }
}

@OscylPreviews
@Composable
private fun guide(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) { GuideScreen(onBack = {}) }

@OscylA11yPreviews
@Composable
private fun guide_a11y() = PreviewTheme { GuideScreen(onBack = {}) }
