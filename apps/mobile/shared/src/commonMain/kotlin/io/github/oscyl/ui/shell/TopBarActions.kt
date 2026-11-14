package io.github.oscyl.ui.shell

import io.github.oscyl.ui.parts.rememberTransient
import io.github.oscyl.ui.parts.transientLayer
import io.github.oscyl.ui.parts.disabled
import io.github.oscyl.shared.generated.resources.programs_new
import io.github.oscyl.shared.generated.resources.ic_add
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.background
import io.github.oscyl.shared.generated.resources.menu_redo_step
import io.github.oscyl.shared.generated.resources.menu_undo_step
import io.github.oscyl.shared.generated.resources.programs_redo
import io.github.oscyl.shared.generated.resources.programs_undo
import io.github.oscyl.shared.generated.resources.ic_redo
import io.github.oscyl.shared.generated.resources.ic_undo
import io.github.oscyl.shared.generated.resources.play_open
import io.github.oscyl.shared.generated.resources.ic_pad
import io.github.oscyl.shared.generated.resources.history_title
import io.github.oscyl.shared.generated.resources.ic_history
import androidx.compose.ui.platform.testTag
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.LayoutDirection
import io.github.oscyl.shared.generated.resources.ic_about
import io.github.oscyl.shared.generated.resources.ic_guide
import io.github.oscyl.shared.generated.resources.ic_preferences
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.motionScale
import org.jetbrains.compose.resources.DrawableResource
import org.jetbrains.compose.resources.painterResource
import kotlin.math.roundToInt
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupProperties
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.about_action
import io.github.oscyl.shared.generated.resources.app_menu
import io.github.oscyl.shared.generated.resources.guide_action
import io.github.oscyl.shared.generated.resources.ic_more
import io.github.oscyl.shared.generated.resources.prefs_title
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.SynthChip
import io.github.oscyl.ui.parts.floating
import io.github.oscyl.ui.parts.touchStates
import org.jetbrains.compose.resources.stringResource

/** Where the top bar's trailing actions lead: the synths, and the menu's three pages. */
@Immutable
class ShellNavigation(
    val synths: () -> Unit = {},
    val preferences: () -> Unit = {},
    val guide: () -> Unit = {},
    val about: () -> Unit = {},
    /** Add a synth (the picker): the chip's long press. */
    val addSynth: (() -> Unit)? = null,
    /** The edit history (a sheet): a long press on undo, and the editor's menu. */
    val history: () -> Unit = {},
    /** The play page, above the editor. */
    val play: () -> Unit = {},
)

/** The app's [ShellNavigation], provided around the destinations; previews go nowhere. */
val LocalShellNavigation = staticCompositionLocalOf { ShellNavigation() }

/**
 * The top bar's trailing edge on every destination: the synth chip, then the menu.
 * [compact] keeps the chip to its small drawn form, where the bar carries more actions.
 * [history] adds the edit history to the menu (the editor).
 */
@Composable
fun RowScope.TopBarActions(compact: Boolean = false, history: Boolean = false, edits: MenuEdits? = null, onNew: (() -> Unit)? = null) {
    val nav = LocalShellNavigation.current
    // The first run's listening card lands in this chip: one shared element.
    SynthChip(nav.synths, compact = compact, onLongClick = nav.addSynth, modifier = Modifier.testTag("synth-chip").sharedPart(LocalChipKey.current, visible = true))
    AppMenu(nav, history, edits, onNew)
}

/**
 * Undo and redo at the head of the menu, with the step's own words ("Undo: Dust Lead removed").
 * The words are asked for when the menu opens, not whenever the document changes.
 */
class MenuEdits(val undo: () -> String?, val redo: () -> String?, val onUndo: () -> Unit, val onRedo: () -> Unit)

/**
 * The menu (⋮): Preferences, Guide and About Oscyl, each a glyph and its word, on a
 * floating panel under it at the trailing edge. It grows out of that corner (scale and
 * alpha on its layer over `base`, back over `fast`, on the one easing; at once with
 * motion off). Back or a tap outside closes it.
 */
@Composable
private fun AppMenu(nav: ShellNavigation, history: Boolean, edits: MenuEdits? = null, onNew: (() -> Unit)? = null) {
    var open by rememberSaveable { mutableStateOf(false) }
    val label = stringResource(Res.string.app_menu)
    val rtl = LocalLayoutDirection.current == LayoutDirection.Rtl
    // How far it has come out (0..1); its popup stays up for the way back.
    val shown = rememberTransient(open)
    Box {
        IconAction(Res.drawable.ic_more, label, { open = true })
        if (shown.up) {
            val close = { open = false }
            Popup(
                alignment = Alignment.TopEnd,
                onDismissRequest = close,
                properties = PopupProperties(focusable = true),
            ) {
                Column(
                    Modifier
                        .padding(8.dp)
                        // Grown out of its corner.
                        .transientLayer({ shown.value }, scaleFrom = 0.9f, origin = TransformOrigin(if (rtl) 0f else 1f, 0f))
                        .width(IntrinsicSize.Max)
                        // A long step's words wrap (two lines) rather than widen it toward the window.
                        .widthIn(min = 200.dp, max = 320.dp)
                        .floating(OscylTheme.colors)
                        .semantics { paneTitle = label }
                        .padding(vertical = 8.dp),
                ) {
                    if (onNew != null) {
                        MenuRow(Res.drawable.ic_add, stringResource(Res.string.programs_new), tag = "program-new") { close(); onNew() }
                        if (edits == null) Box(Modifier.fillMaxWidth().padding(vertical = 4.dp).height(OscylTokens.ruleWidth).background(OscylTheme.colors.rule))
                    }
                    if (edits != null) {
                        val undo = remember(edits) { edits.undo() }
                        val redo = remember(edits) { edits.redo() }
                        MenuRow(Res.drawable.ic_undo, undo?.let { stringResource(Res.string.menu_undo_step, it) } ?: stringResource(Res.string.programs_undo), enabled = undo != null) { close(); edits.onUndo() }
                        MenuRow(Res.drawable.ic_redo, redo?.let { stringResource(Res.string.menu_redo_step, it) } ?: stringResource(Res.string.programs_redo), enabled = redo != null) { close(); edits.onRedo() }
                        Box(Modifier.fillMaxWidth().padding(vertical = 4.dp).height(OscylTokens.ruleWidth).background(OscylTheme.colors.rule))
                    }
                    if (history) MenuRow(Res.drawable.ic_pad, stringResource(Res.string.play_open)) { close(); nav.play() }
                    if (history) MenuRow(Res.drawable.ic_history, stringResource(Res.string.history_title)) { close(); nav.history() }
                    // The page's own items above, the app's three under a rule.
                    if (history) Box(Modifier.fillMaxWidth().padding(vertical = 4.dp).height(OscylTokens.ruleWidth).background(OscylTheme.colors.rule))
                    MenuRow(Res.drawable.ic_preferences, stringResource(Res.string.prefs_title)) { close(); nav.preferences() }
                    MenuRow(Res.drawable.ic_guide, stringResource(Res.string.guide_action)) { close(); nav.guide() }
                    MenuRow(Res.drawable.ic_about, stringResource(Res.string.about_action)) { close(); nav.about() }
                }
            }
        }
    }
}

@Composable
private fun MenuRow(icon: DrawableResource, text: String, enabled: Boolean = true, tag: String? = null, onClick: () -> Unit) {
    val c = OscylTheme.colors
    val interaction = remember { MutableInteractionSource() }
    val shape = RoundedCornerShape(0.dp)
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .then(if (tag != null) Modifier.testTag(tag) else Modifier)
            .clickable(interaction, indication = null, role = Role.Button, enabled = enabled, onClick = onClick)
            .touchStates(interaction, shape, c.press)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        val ink = if (enabled) c.ink else c.disabled
        Image(painterResource(icon), null, Modifier.size(20.dp), colorFilter = ColorFilter.tint(ink))
        BasicText(text, style = MaterialTheme.typography.bodyLarge.copy(color = ink, fontWeight = FontWeight.Medium, textDirection = TextDirection.Content), maxLines = 2)
    }
}
