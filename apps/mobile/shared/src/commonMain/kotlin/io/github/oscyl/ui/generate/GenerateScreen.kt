package io.github.oscyl.ui.generate

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.input.TextFieldLineLimits
import androidx.compose.foundation.text.input.rememberTextFieldState
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import io.github.oscyl.shared.generated.resources.action_cancel
import io.github.oscyl.shared.generated.resources.gen_code
import io.github.oscyl.shared.generated.resources.gen_code_deal
import io.github.oscyl.shared.generated.resources.gen_code_enter
import io.github.oscyl.shared.generated.resources.gen_code_share
import io.github.oscyl.shared.generated.resources.gen_code_title
import io.github.oscyl.shared.generated.resources.gen_family
import io.github.oscyl.shared.generated.resources.gen_family_all
import io.github.oscyl.shared.generated.resources.gen_family_bass
import io.github.oscyl.shared.generated.resources.gen_family_effect
import io.github.oscyl.shared.generated.resources.gen_family_keys
import io.github.oscyl.shared.generated.resources.gen_family_lead
import io.github.oscyl.shared.generated.resources.gen_family_pad
import io.github.oscyl.shared.generated.resources.gen_family_rhythm
import io.github.oscyl.shared.generated.resources.gen_morph
import io.github.oscyl.shared.generated.resources.gen_surprise
import io.github.oscyl.shared.generated.resources.ic_dice
import io.github.oscyl.shared.generated.resources.ic_share
import io.github.oscyl.ui.library.LocalPanelLeave

import io.github.oscyl.shared.generated.resources.ic_check
import org.jetbrains.compose.resources.painterResource
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import io.github.oscyl.platform.LocalOscylHaptics
import androidx.compose.ui.platform.testTag
import io.github.oscyl.ui.parts.LocalSynthStatus
import io.github.oscyl.ui.library.MenuItem
import io.github.oscyl.ui.library.MenuPanel
import io.github.oscyl.shared.generated.resources.gen_intro_no_synth
import io.github.oscyl.shared.generated.resources.gen_keep_in_library
import io.github.oscyl.shared.generated.resources.gen_keep_in_slot
import io.github.oscyl.shared.generated.resources.gen_keep
import io.github.oscyl.shared.generated.resources.empty_hand
import io.github.oscyl.ui.parts.EmptyState
import io.github.oscyl.shared.generated.resources.destination_generate
import io.github.oscyl.ui.parts.CollapsingTopBar
import io.github.oscyl.ui.parts.LargeTitle
import io.github.oscyl.ui.parts.rememberCollapsingBarState
import io.github.oscyl.ui.shell.TopBarActions
import io.github.oscyl.ui.parts.safeInsets
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.oscyl.core.AxisWord
import io.github.oscyl.core.CharacterInfo
import io.github.oscyl.core.GraphMark
import io.github.oscyl.core.GraphShape
import io.github.oscyl.core.HandState
import io.github.oscyl.core.MacroInfo
import io.github.oscyl.core.MarkStyle
import io.github.oscyl.core.SoundTile
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.gen_breed
import io.github.oscyl.shared.generated.resources.gen_character
import io.github.oscyl.shared.generated.resources.gen_deal
import io.github.oscyl.shared.generated.resources.gen_dealing
import io.github.oscyl.shared.generated.resources.gen_empty
import io.github.oscyl.shared.generated.resources.gen_hand_of
import io.github.oscyl.shared.generated.resources.gen_intro
import io.github.oscyl.shared.generated.resources.gen_keep_stages
import io.github.oscyl.shared.generated.resources.gen_kept
import io.github.oscyl.shared.generated.resources.gen_next
import io.github.oscyl.shared.generated.resources.gen_previous
import io.github.oscyl.shared.generated.resources.gen_shape
import io.github.oscyl.shared.generated.resources.gen_vary
import io.github.oscyl.shared.generated.resources.gen_vary_clear
import io.github.oscyl.shared.generated.resources.gen_vary_subtle
import io.github.oscyl.shared.generated.resources.gen_vary_wild
import io.github.oscyl.shared.generated.resources.ic_back
import io.github.oscyl.shared.generated.resources.ic_chevron
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.editor.MiniGraph
import io.github.oscyl.ui.parts.Dial
import io.github.oscyl.ui.parts.DialSize
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.Legend
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.Segments
import io.github.oscyl.ui.parts.liftedPlate
import io.github.oscyl.ui.parts.touchStates
import io.github.oscyl.ui.parts.FootKey
import io.github.oscyl.shared.generated.resources.ic_keep
import io.github.oscyl.shared.generated.resources.ic_generate
import androidx.compose.foundation.layout.fillMaxHeight
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.OscylSizePreviews
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.shell.Destination
import io.github.oscyl.ui.shell.LocalIslandClearance
import io.github.oscyl.ui.shell.FootSide
import io.github.oscyl.ui.shell.ReserveFoot
import io.github.oscyl.ui.parts.rememberTransient
import io.github.oscyl.ui.parts.transientLayer
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.clearAndSetSemantics
import kotlin.math.roundToInt
import io.github.oscyl.ui.shell.Shell
import kotlin.math.PI
import kotlin.math.sin
import org.jetbrains.compose.resources.stringResource

/** The Generate destination over the app's [GenerateModel]. */
@Composable
fun GenerateScreen(model: GenerateModel, modifier: Modifier = Modifier) {
    val ui by model.ui.collectAsStateWithLifecycle()
    GenerateScreen(ui, model, modifier)
}

/**
 * Generate (UX brief): the synth's characters under their family row, four macro dials
 * with Surprise me and Enter a code under them, and the stages to keep from the program
 * open; then the hand of six as lifted tiles (two columns on a phone, three when wider),
 * Back and Forward through the last hands, Vary, Breed, Morph and Keep for the sound
 * picked, and its recipe code to share. Picking a sound plays it on the synth.
 */
@Composable
fun GenerateScreen(ui: GenerateUi, actions: GenerateActions, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val clearance = LocalIslandClearance.current
    val scroll = rememberScrollState()
    val bar = rememberCollapsingBarState(scroll)
    val title = stringResource(Res.string.destination_generate)
    // Keep asks where (a synth's open slot or a library).
    var keeping by rememberSaveable { mutableStateOf(false) }
    Column(modifier.fillMaxSize()) {
    CollapsingTopBar(bar, title, ui.model, trailing = { TopBarActions() })
    Box(Modifier.weight(1f).fillMaxWidth()) {
    Column(
        Modifier
            .fillMaxSize()
            .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Horizontal))
            .verticalScroll(scroll)
            // The page scrolls under the foot's keys and the island, and its end clears both.
            .padding(start = 16.dp, end = 16.dp, bottom = clearance + 56.dp + 12.dp + 16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Column(Modifier.widthIn(max = 720.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                LargeTitle(title, bar)
                BasicText(ui.model, style = type.bodyMedium.copy(color = c.legend, textDirection = TextDirection.Content))
            }
            // With no synth, no promise of sound: how to get it.
            val heard = LocalSynthStatus.current.model != null
            BasicText(stringResource(if (heard) Res.string.gen_intro else Res.string.gen_intro_no_synth), style = type.bodyMedium.copy(color = c.legend))
            Legend(stringResource(Res.string.gen_character))
            Characters(ui, actions)
            Legend(stringResource(Res.string.gen_shape))
            // Four in a row where they fit, else two by two (never three and one).
            BoxWithConstraints(Modifier.fillMaxWidth()) {
                FlowRow(Modifier.fillMaxWidth(), maxItemsInEachRow = if (maxWidth >= 400.dp) 4 else 2) {
                    ui.macros.forEachIndexed { i, m ->
                        val v = ui.values.getOrElse(i) { 50 }
                        // A macro is a setting, not an edit: no "was" mark.
                        Box(Modifier.weight(1f), contentAlignment = Alignment.TopCenter) {
                            Dial(v, saved = v, range = 0..100, label = m.title, format = { it.toString() }, onChange = { actions.value(i, it) }, size = DialSize.Small, revertible = false)
                        }
                    }
                }
            }
            // The core picks for you, or deals the sound a code names.
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                val can = !ui.busy && ui.characters.isNotEmpty()
                OutlinedAction(
                    stringResource(Res.string.gen_surprise), actions::surprise, Modifier.testTag("generate-surprise"),
                    leading = { Image(painterResource(Res.drawable.ic_dice), null, Modifier.size(18.dp), colorFilter = ColorFilter.tint(if (can) c.ink else c.legend)) },
                    enabled = can,
                )
                OutlinedAction(stringResource(Res.string.gen_code_enter), { actions.enterCode(true) }, Modifier.testTag("generate-enter-code"), enabled = can)
            }
            if (ui.slot != null && ui.pages.isNotEmpty()) {
                Legend(stringResource(Res.string.gen_keep_stages, ui.slot))
                FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    ui.pages.forEachIndexed { i, page -> Chip(page, i in ui.keep) { actions.keepPage(i, it) } }
                }
            }
            val hand = ui.hand
            if (hand == null) {
                EmptyState(stringResource(Res.string.empty_hand), stringResource(Res.string.gen_empty))
            } else {
                // A new hand comes into view (it is below the dials on a phone).
                val seen = remember { BringIntoViewRequester() }
                LaunchedEffect(hand.at, hand.count, hand.origin) { seen.bringIntoView() }
                Hand(hand, actions, Modifier.bringIntoViewRequester(seen))
                HandActions(ui, hand, actions)
            }
        }
    }
    // The page's actions float at the foot's end above the island, the page scrolling
    // under them (the same keys as the Library's New). With sounds dealt, Keep (the
    // page's point) stands on the plate beside Generate.
    val dealWord = stringResource(if (ui.busy) Res.string.gen_dealing else Res.string.gen_deal)
    val canDeal = !ui.busy && ui.characters.isNotEmpty()
    // Keep arrives as every transient part does; Generate gives it room by placement only,
    // stepping aside as it comes (and back as it goes), and leaves the plate to it.
    val keep = rememberTransient(ui.hand != null)
    val density = LocalDensity.current
    ReserveFoot(FootSide.Keys, 56.dp)
    Layout(
        content = {
            FootKey(dealWord, Res.drawable.ic_generate, actions::deal, Modifier.fillMaxHeight().testTag("generate-deal"), primary = ui.hand == null, enabled = canDeal)
            if (keep.up) {
                // A synth's open slot can take it as well as a library: ask which.
                FootKey(
                    stringResource(Res.string.gen_keep), Res.drawable.ic_keep,
                    { if (ui.slot != null) keeping = true else actions.keepInLibrary() },
                    Modifier
                        .fillMaxHeight()
                        .testTag("generate-keep")
                        .transientLayer({ keep.value })
                        .then(if (ui.hand == null) Modifier.clearAndSetSemantics {} else Modifier),
                    primary = true,
                    enabled = ui.hand != null,
                )
            }
        },
        modifier = Modifier
            .align(Alignment.BottomEnd)
            .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Horizontal))
            .padding(start = 12.dp, end = 12.dp, bottom = 12.dp + clearance)
            .height(56.dp),
    ) { measurables, constraints ->
        val keys = measurables.map { it.measure(constraints.copy(minWidth = 0)) }
        val gap = with(density) { 8.dp.roundToPx() }
        val deal = keys[0]
        val kept = keys.getOrNull(1)
        layout(keys.sumOf { it.width } + gap * (keys.size - 1), constraints.maxHeight) {
            // Read here only: Keep's arrival moves Generate by placement, frame by frame.
            val aside = kept?.let { ((it.width + gap) * (1f - keep.value)).roundToInt() } ?: 0
            deal.placeRelative(aside, 0)
            kept?.placeRelative(deal.width + gap, 0)
        }
    }
    }
    if (ui.entering) CodeDialog(ui.codeError, ui.busy, onDismiss = { actions.enterCode(false) }, onDeal = actions::dealCode)
    val hand = ui.hand
    if (keeping && hand != null) {
        MenuPanel(hand.sounds.getOrNull(hand.selected.toInt())?.name ?: stringResource(Res.string.gen_keep), onDismiss = { keeping = false }) {
            ui.slot?.let { slot -> MenuItem(stringResource(Res.string.gen_keep_in_slot, slot)) { keeping = false; actions.keepInSlot() } }
            MenuItem(stringResource(Res.string.gen_keep_in_library)) { keeping = false; actions.keepInLibrary() }
        }
    }
    }
}

/** A family's name, from its id ([CharacterInfo.family]). */
@Composable
private fun familyTitle(id: String): String = stringResource(
    when (id) {
        "keys" -> Res.string.gen_family_keys
        "bass" -> Res.string.gen_family_bass
        "lead" -> Res.string.gen_family_lead
        "pad" -> Res.string.gen_family_pad
        "rhythm" -> Res.string.gen_family_rhythm
        else -> Res.string.gen_family_effect
    },
)

/**
 * The characters: a row of families (All first; it scrolls sideways when it must), then
 * the chosen family's characters as a wrapped group of choices, the chosen one on the plate.
 */
@Composable
private fun Characters(ui: GenerateUi, actions: GenerateActions) {
    val all = families(ui.characters)
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        // A synth with one family needs no row.
        if (all.size > 1) {
            val label = stringResource(Res.string.gen_family)
            Row(
                Modifier
                    .semantics { contentDescription = label }
                    .selectableGroup()
                    .horizontalScroll(rememberScrollState())
                    .testTag("generate-families"),
            ) {
                FamilyTab(stringResource(Res.string.gen_family_all), ui.family == null) { actions.family(null) }
                all.forEach { id -> FamilyTab(familyTitle(id), ui.family == id) { actions.family(id) } }
            }
        }
        FlowRow(
            Modifier.selectableGroup(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            ui.characters.forEachIndexed { i, ch ->
                if (ui.family == null || ch.family == ui.family) {
                    Choice(ch.title, i == ui.character, description = ch.hint) { actions.character(i) }
                }
            }
        }
    }
}

/**
 * A family in its row: its word alone, in `legend`; the chosen one in `ink` at 600 over an
 * ink index line (weight and line, not colour alone). Quieter than the characters under
 * it, which are the choice that counts.
 */
@Composable
private fun FamilyTab(text: String, on: Boolean, onClick: () -> Unit) {
    val c = OscylTheme.colors
    val haptics = LocalOscylHaptics.current
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    val motion = LocalOscylMotion.current
    val ink by animateColorAsState(if (on) c.ink else c.legend, tween(if (motion) OscylTokens.fastMs else 0, easing = OscylTokens.easing))
    Box(
        Modifier
            .heightIn(min = 48.dp)
            .selectable(on, interaction, indication = null, role = Role.RadioButton) { if (!on) haptics.tick(); onClick() }
            .touchStates(interaction, shape, c.press)
            .padding(horizontal = 12.dp)
            .drawBehind {
                // The index line under the chosen family's word, drawn in place: nothing is laid out for it.
                if (on) {
                    val h = 2.dp.toPx()
                    drawRect(c.ink, Offset(0f, size.height - 10.dp.toPx()), Size(size.width, h))
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        // Both weights are laid out, so choosing a family moves none of them.
        val style = MaterialTheme.typography.bodyMedium.copy(textDirection = TextDirection.Content)
        BasicText(text, Modifier.clearAndSetSemantics {}, style = style.copy(color = androidx.compose.ui.graphics.Color.Transparent, fontWeight = FontWeight.SemiBold))
        BasicText(text, style = style.copy(color = ink, fontWeight = if (on) FontWeight.SemiBold else FontWeight.Medium))
    }
}

/**
 * Enter a code: a field for a recipe code typed or pasted, Cancel and Deal. What is wrong
 * with a bad code stands under the field, in the core's words, in a line that is always
 * kept (nothing moves when it appears) and is said at once.
 */
@Composable
private fun CodeDialog(error: String?, busy: Boolean, onDismiss: () -> Unit, onDeal: (String) -> Unit) {
    val c = OscylTheme.colors
    val state = rememberTextFieldState()
    val focus = remember { FocusRequester() }
    val fieldLabel = stringResource(Res.string.gen_code)
    val deal = { if (!busy) onDeal(state.text.toString().trim()) }
    MenuPanel(stringResource(Res.string.gen_code_title), onDismiss) {
        // Inside the dialog's window, once the field is there.
        LaunchedEffect(Unit) { focus.requestFocus() }
        BasicTextField(
            state,
            Modifier
                .padding(horizontal = 16.dp, vertical = 8.dp)
                .fillMaxWidth()
                .heightIn(min = 48.dp)
                .border(if (error != null) 2.dp else OscylTokens.ruleWidth, if (error != null) c.ink else c.ruleStrong, RoundedCornerShape(OscylTokens.radius))
                .padding(horizontal = 12.dp, vertical = 12.dp)
                .focusRequester(focus)
                .testTag("generate-code-field")
                .semantics { contentDescription = fieldLabel },
            textStyle = MaterialTheme.typography.bodyLarge.copy(color = c.ink, fontFamily = FontFamily.Monospace, textDirection = TextDirection.Ltr),
            cursorBrush = SolidColor(c.ink),
            lineLimits = TextFieldLineLimits.SingleLine,
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.None, autoCorrectEnabled = false, imeAction = ImeAction.Done),
            onKeyboardAction = { deal() },
        )
        BasicText(
            error.orEmpty(),
            Modifier
                .padding(horizontal = 16.dp)
                .fillMaxWidth()
                .testTag("generate-code-error")
                .semantics { liveRegion = LiveRegionMode.Assertive },
            style = MaterialTheme.typography.bodySmall.copy(color = c.ink, fontWeight = FontWeight.SemiBold),
            minLines = 1,
        )
        FlowRow(
            Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, bottom = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
        ) {
            val leave = LocalPanelLeave.current
            OutlinedAction(stringResource(Res.string.action_cancel), { leave(onDismiss) })
            OutlinedAction(stringResource(Res.string.gen_code_deal), deal, enabled = !busy)
        }
    }
}

/** One choice: outlined, the plate when chosen. */
@Composable
private fun Choice(text: String, on: Boolean, description: String? = null, onClick: () -> Unit) {
    val c = OscylTheme.colors
    val haptics = LocalOscylHaptics.current
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    Box(
        Modifier
            .heightIn(min = 48.dp)
            .then(if (on) Modifier.liftedPlate(c, shape) else Modifier.border(OscylTokens.ruleWidth, c.ruleStrong, shape))
            .selectable(on, interaction, indication = null, role = Role.RadioButton) { if (!on) haptics.tick(); onClick() }
            .then(if (description != null) Modifier.semantics { stateDescription = description } else Modifier)
            .touchStates(interaction, shape, c.press)
            .padding(horizontal = 12.dp),
        contentAlignment = Alignment.Center,
    ) {
        BasicText(
            text,
            style = MaterialTheme.typography.bodyMedium.copy(
                color = if (on) c.onPlate else c.ink,
                fontWeight = if (on) FontWeight.SemiBold else FontWeight.Medium,
                textDirection = TextDirection.Content,
            ),
        )
    }
}

/**
 * A page to keep: a toggle chip, the plate when on. Several can be held, so it reads as
 * a checkbox, not as the single choices above it: an empty box before its word, a check
 * on the plate when held (the same width either way: nothing moves as it toggles).
 */
@Composable
private fun Chip(text: String, on: Boolean, onChange: (Boolean) -> Unit) {
    val c = OscylTheme.colors
    val haptics = LocalOscylHaptics.current
    val shape = RoundedCornerShape(OscylTokens.radius)
    val interaction = remember { MutableInteractionSource() }
    Box(
        Modifier
            .heightIn(min = 48.dp)
            .then(if (on) Modifier.liftedPlate(c, shape) else Modifier.border(OscylTokens.ruleWidth, c.ruleStrong, shape))
            .toggleable(on, interaction, indication = null, role = Role.Checkbox) { haptics.tick(); onChange(it) }
            .touchStates(interaction, shape, c.press)
            .padding(start = 10.dp, end = 12.dp),
        contentAlignment = Alignment.Center,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            if (on) {
                Image(painterResource(Res.drawable.ic_check), null, Modifier.size(16.dp), colorFilter = ColorFilter.tint(c.onPlate))
            } else {
                Canvas(Modifier.size(16.dp)) {
                    val w = OscylTokens.ruleWidth.toPx()
                    val inset = 2.dp.toPx()
                    drawRoundRect(
                        c.ruleStrong, Offset(inset + w / 2, inset + w / 2), Size(size.width - 2 * inset - w, size.height - 2 * inset - w),
                        CornerRadius(2.dp.toPx()), style = Stroke(w),
                    )
                }
            }
            BasicText(text, style = MaterialTheme.typography.bodyMedium.copy(color = if (on) c.onPlate else c.ink, fontWeight = FontWeight.Medium))
        }
    }
}

/** The primary action: the plate. */
@Composable
internal fun Primary(text: String, enabled: Boolean, onClick: () -> Unit) {
    val c = OscylTheme.colors
    val shape = RoundedCornerShape(OscylTokens.radiusPanel)
    val interaction = remember { MutableInteractionSource() }
    Box(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .liftedPlate(c, shape)
            .clickable(interaction, indication = null, enabled = enabled, role = Role.Button, onClick = onClick)
            .touchStates(interaction, shape, c.ink.copy(alpha = 0.22f)),
        contentAlignment = Alignment.Center,
    ) {
        BasicText(text, style = MaterialTheme.typography.bodyLarge.copy(color = c.onPlate, fontWeight = FontWeight.SemiBold))
    }
}

/** The six sounds as lifted tiles; two columns, three from 600 dp, the picked one on the plate. */
@Composable
private fun Hand(hand: HandState, actions: GenerateActions, modifier: Modifier = Modifier) {
    BoxWithConstraints(modifier.fillMaxWidth()) {
        val columns = if (maxWidth >= 600.dp) 3 else 2
        val gap = 8.dp
        // Equal shares of the row (a computed width rounds past it and wraps every tile alone).
        FlowRow(
            Modifier.selectableGroup().fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(gap),
            verticalArrangement = Arrangement.spacedBy(gap),
            maxItemsInEachRow = columns,
        ) {
            hand.sounds.forEachIndexed { i, s -> Tile(s, i == hand.selected.toInt(), Modifier.weight(1f)) { actions.select(i) } }
        }
    }
}

@Composable
private fun Tile(sound: SoundTile, picked: Boolean, modifier: Modifier, onPick: () -> Unit) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val shape = RoundedCornerShape(OscylTokens.radiusPanel)
    val interaction = remember { MutableInteractionSource() }
    val motion = LocalOscylMotion.current
    val fill by animateColorAsState(if (picked) c.plate else c.raised, tween(if (motion) OscylTokens.baseMs else 0, easing = OscylTokens.easing))
    val kept = sound.kept?.let { stringResource(Res.string.gen_kept, it) }
    Column(
        modifier
            .liftedPlate(c, shape, fill)
            .selectable(picked, interaction, indication = null, role = Role.RadioButton, onClick = onPick)
            .then(if (kept != null) Modifier.semantics { stateDescription = kept } else Modifier)
            .touchStates(interaction, shape, c.press)
            .padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        val ink = if (picked) c.onPlate else c.ink
        sound.graph?.let { MiniGraph(it, ink, Modifier.fillMaxWidth().height(40.dp)) }
        BasicText(sound.name, style = type.bodyMedium.copy(color = ink, fontWeight = FontWeight.SemiBold), maxLines = 2, overflow = TextOverflow.Ellipsis)
        val caption = listOfNotNull(sound.category, kept).joinToString(" · ")
        if (caption.isNotEmpty()) BasicText(caption, style = type.bodySmall.copy(color = if (picked) c.plateLegend else c.legend))
    }
}

/**
 * Where the hand came from, Back and Forward, then Vary (with its amount), Breed with and
 * Morph to (each asks which other sound of the hand, in the same row of names), and the
 * picked sound's recipe code with its Share.
 */
@Composable
private fun HandActions(ui: GenerateUi, hand: HandState, actions: GenerateActions) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    // Which action waits for another sound of the hand: 0 none, 1 Breed with, 2 Morph to.
    var picking by rememberSaveable { mutableStateOf(0) }
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                BasicText(hand.origin, style = type.bodyMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content))
                if (hand.count > 1u) {
                    BasicText(stringResource(Res.string.gen_hand_of, hand.at.toInt() + 1, hand.count.toInt()), style = type.bodySmall.copy(color = c.legend))
                }
            }
            IconAction(Res.drawable.ic_back, stringResource(Res.string.gen_previous), { actions.step(-1) }, enabled = hand.at > 0u)
            IconAction(Res.drawable.ic_chevron, stringResource(Res.string.gen_next), { actions.step(1) }, enabled = hand.at + 1u < hand.count)
        }
        Segments(
            listOf(stringResource(Res.string.gen_vary_subtle), stringResource(Res.string.gen_vary_clear), stringResource(Res.string.gen_vary_wild)),
            selected = ui.amount,
            saved = ui.amount,
            onSelect = actions::amount,
            modifier = Modifier.fillMaxWidth(),
            index = false,
        )
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedAction(stringResource(Res.string.gen_vary), actions::vary)
            OutlinedAction(stringResource(Res.string.gen_breed), { picking = if (picking == 1) 0 else 1 }, role = Role.DropdownList)
            OutlinedAction(stringResource(Res.string.gen_morph), { picking = if (picking == 2) 0 else 2 }, Modifier.testTag("generate-morph"), role = Role.DropdownList)
        }
        if (picking != 0) {
            // The others of the hand, to breed the picked one with or morph it into.
            val what = stringResource(if (picking == 1) Res.string.gen_breed else Res.string.gen_morph)
            FlowRow(
                Modifier.semantics { contentDescription = what },
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                hand.sounds.forEachIndexed { i, s ->
                    if (i != hand.selected.toInt()) {
                        OutlinedAction(s.name, {
                            val morph = picking == 2
                            picking = 0
                            if (morph) actions.morph(i) else actions.breed(i)
                        })
                    }
                }
            }
        }
        // The picked sound's recipe code, small, with its Share; a sound without one shows none.
        ui.code?.let { code ->
            val label = stringResource(Res.string.gen_code)
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f).semantics(mergeDescendants = true) {}) {
                    BasicText(label, style = type.bodySmall.copy(color = c.legend))
                    BasicText(
                        code,
                        Modifier.testTag("generate-code"),
                        style = type.bodySmall.copy(color = c.ink, fontFamily = FontFamily.Monospace, textDirection = TextDirection.Ltr),
                        maxLines = 1,
                        overflow = TextOverflow.MiddleEllipsis,
                    )
                }
                IconAction(Res.drawable.ic_share, stringResource(Res.string.gen_code_share), actions::shareCode)
            }
        }
    }
}

// Sample state for the previews and the device checks: plain values, no model, no core calls.

private fun wave(k: Float) = GraphShape(
    0u, listOf(GraphMark(MarkStyle.LINE, (0..48).flatMap { i -> val x = i / 48f; listOf(x, 0.5f + 0.4f * sin(2 * PI.toFloat() * k * x)) })),
    listOf(0.5f), null, false, emptyList(), emptyList(), time = false, pitch = false,
)

val SampleGenerate = GenerateUi(
    model = "minilogue xd",
    characters = listOf("Pad" to "pad", "Bass" to "bass", "Lead" to "lead", "Bells" to "keys", "Pluck" to "keys", "Strings" to "pad", "Brass" to "lead", "Space" to "effect")
        .map { (title, family) -> CharacterInfo(title, "$title sounds", family) },
    character = 2,
    family = "lead",
    code = "lead-nn6agh34s69jr",
    macros = listOf("Bright", "Movement", "Grit", "Ambience").map { MacroInfo(it, "") },
    values = listOf(60, 35, 20, 70),
    pages = listOf("Voice", "Oscillators", "Filter", "Envelopes", "Effects"),
    keep = setOf(4),
    slot = "004",
    hand = HandState(
        listOf("Glass Lead 12", "Neon Lead 4", "Hollow Lead 87", "Sharp Lead 3", "Soft Lead 41", "Wide Lead 9").mapIndexed { i, n ->
            SoundTile(n, "Lead", wave(1f + i), if (i == 1) "004" else null)
        },
        selected = 1u,
        origin = "Lead",
        at = 1u,
        count = 2u,
    ),
)

@Composable
private fun GeneratePreview(mode: OscylThemeMode = OscylThemeMode.Dark, ui: GenerateUi = SampleGenerate) = OscylTheme(mode) {
    Shell(Destination.Generate, onSelect = {}, saveCount = 1, onSave = {}) { modifier -> GenerateScreen(ui, NoGenerateActions, modifier) }
}

@OscylPreviews
@Composable
private fun generate(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = GeneratePreview(mode)

@OscylPreviews
@Composable
private fun generate_empty(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = GeneratePreview(mode, SampleGenerate.copy(hand = null, slot = null))

@OscylA11yPreviews
@Composable
private fun generate_a11y() = GeneratePreview()

@OscylSizePreviews
@Composable
private fun generate_sizes() = GeneratePreview()
