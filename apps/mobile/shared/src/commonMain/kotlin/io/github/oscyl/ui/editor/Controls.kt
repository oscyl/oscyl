package io.github.oscyl.ui.editor

import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.ui.platform.LocalDensity
import io.github.oscyl.ui.parts.rememberTransient
import io.github.oscyl.ui.parts.transientLayer
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupPositionProvider
import androidx.compose.ui.window.PopupProperties
import io.github.oscyl.core.ControlKind
import io.github.oscyl.core.FieldState
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.choice_description
import io.github.oscyl.shared.generated.resources.dial_state_edited
import io.github.oscyl.shared.generated.resources.ic_chevron
import io.github.oscyl.shared.generated.resources.option_off
import io.github.oscyl.shared.generated.resources.option_on
import io.github.oscyl.shared.generated.resources.segment_saved
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.Dial
import io.github.oscyl.ui.parts.DialSize
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.Segments
import io.github.oscyl.ui.parts.WasLine
import io.github.oscyl.ui.parts.floating
import io.github.oscyl.ui.parts.liftedPlate
import io.github.oscyl.ui.parts.touchStates
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.stringResource

/** Whether [field] is drawn as a dial (the dials lead a section). */
val FieldState.isDial: Boolean get() = control is ControlKind.Dial

/** One field's control, as the core says to draw it. */
@Composable
fun FieldControl(field: FieldState, comparing: Boolean, actions: EditorActions, modifier: Modifier = Modifier, enabled: Boolean = true) {
    val f = field.field
    when (val kind = field.control) {
        is ControlKind.Dial -> Dial(
            value = field.raw.toInt(),
            saved = field.savedRaw.toInt(),
            range = kind.min.toInt()..kind.max.toInt(),
            label = field.label,
            format = { actions.format(f, it) },
            onChange = { actions.set(f, it) },
            size = if (kind.large) DialSize.Large else DialSize.Small,
            modifier = modifier,
            comparing = comparing,
            onRelease = actions::release,
            typed = { actions.parse(f, it) },
            enabled = enabled,
        )
        is ControlKind.Segments -> Selector(field, actions, modifier) {
            Segments(kind.options, field.option?.toInt() ?: 0, field.savedOption?.toInt() ?: 0, onSelect = { actions.choose(f, it) }, enabled = enabled)
        }
        ControlKind.Toggle -> Selector(field, actions, modifier) {
            Segments(
                listOf(stringResource(Res.string.option_off), stringResource(Res.string.option_on)),
                field.option?.toInt() ?: 0,
                field.savedOption?.toInt() ?: 0,
                onSelect = { actions.choose(f, it) },
                enabled = enabled,
            )
        }
        is ControlKind.Menu -> Selector(field, actions, modifier) {
            WrappedChoice(field.label, kind.options, field.option?.toInt() ?: 0, field.savedOption?.toInt() ?: 0, enabled = enabled) { actions.choose(f, it) }
        }
    }
}

/**
 * A selector with its legend above (and the orange edit dot after it) and the
 * "was" line under it, as DESIGN.md's "Selector revert": its space always kept.
 */
@Composable
private fun Selector(field: FieldState, actions: EditorActions, modifier: Modifier, control: @Composable () -> Unit) {
    val c = OscylTheme.colors
    val motion = LocalOscylMotion.current
    val edit by animateFloatAsState(if (field.edited) 1f else 0f, tween(if (motion) OscylTokens.baseMs else 0, easing = OscylTokens.easing))
    val state = if (field.edited) stringResource(Res.string.dial_state_edited, field.text, field.savedText) else field.text
    Column(modifier.widthIn(max = 360.dp)) {
        // The legend's row keeps one height, a whole touch target's: "was X" stands at its
        // end while edited, so nothing below moves and no line waits empty under the control.
        // At large text it has a line of its own under the legend (always kept): beside it, its
        // space left the name so little room that a word broke inside.
        val legend = @Composable { m: Modifier ->
            Row(
                m.padding(bottom = 8.dp).clearAndSetSemantics {},
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                BasicText(
                    field.label,
                    Modifier.weight(1f, fill = false),
                    style = MaterialTheme.typography.bodyMedium.copy(color = c.legend, textDirection = TextDirection.Content),
                    maxLines = 2,
                    overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                )
                Canvas(Modifier.size(6.dp).graphicsLayer { alpha = edit }) { drawCircle(c.act) }
            }
        }
        if (LocalDensity.current.fontScale > 1.3f) {
            legend(Modifier.fillMaxWidth().heightIn(min = 48.dp).wrapContentHeight(Alignment.Bottom))
            WasLine(field.savedText, field.edited, { edit }, onRestore = { actions.revert(field.field) })
        } else {
            Row(Modifier.fillMaxWidth().heightIn(min = 48.dp), verticalAlignment = Alignment.Bottom) {
                legend(Modifier.weight(1f))
                WasLine(field.savedText, field.edited, { edit }, onRestore = { actions.revert(field.field) })
            }
        }
        Box(Modifier.semantics { contentDescription = field.label; stateDescription = state }) { control() }
    }
}

/**
 * Many named options (DESIGN.md has no menu of its own): an outlined field showing the
 * option chosen; a tap opens a floating plate of every option, wrapped in rows, the
 * chosen one on the plate and the saved one notched in orange.
 */
@Composable
internal fun WrappedChoice(label: String, options: List<String>, selected: Int, saved: Int, enabled: Boolean = true, onSelect: (Int) -> Unit) {
    val c = OscylTheme.colors
    var open by remember { mutableStateOf(false) }
    val interaction = remember { MutableInteractionSource() }
    val shape = RoundedCornerShape(OscylTokens.radius)
    val chosen = options.getOrElse(selected) { "" }
    val description = stringResource(Res.string.choice_description, label, chosen)
    Row(
        Modifier
            .heightIn(min = 48.dp)
            .clickable(interaction, indication = null, role = Role.DropdownList, enabled = enabled) { open = true }
            .semantics { contentDescription = description }
            .border(OscylTokens.ruleWidth, c.ruleStrong, shape)
            .touchStates(interaction, shape, c.press)
            .padding(start = 12.dp, end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        BasicText(
            chosen,
            Modifier.weight(1f, fill = false),
            style = MaterialTheme.typography.bodyMedium.copy(color = c.ink, fontWeight = FontWeight.Medium, textDirection = TextDirection.Content),
        )
        Image(painterResource(Res.drawable.ic_chevron), null, Modifier.size(16.dp), colorFilter = ColorFilter.tint(c.legend))
    }
    // It arrives and leaves as every transient part does.
    val plate = rememberTransient(open)
    if (plate.up) {
        Popup(
            popupPositionProvider = BelowAnchor,
            onDismissRequest = { open = false },
            properties = PopupProperties(focusable = true),
        ) {
            val savedChoice = stringResource(Res.string.segment_saved)
            FlowRow(
                Modifier
                    // Grown from under the field it sets.
                    .transientLayer({ plate.value }, origin = TransformOrigin(0f, 0f))
                    .widthIn(max = 360.dp)
                    .floating(c)
                    .verticalScroll(rememberScrollState())
                    .padding(6.dp)
                    .selectableGroup(),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                options.forEachIndexed { i, option ->
                    val on = i == selected
                    val inner = RoundedCornerShape(OscylTokens.radiusInner)
                    val press = remember { MutableInteractionSource() }
                    val fill by animateColorAsState(if (on) c.plate else c.plate.copy(alpha = 0f), tween(OscylTokens.baseMs, easing = OscylTokens.easing))
                    Box(
                        Modifier
                            .heightIn(min = 48.dp)
                            .then(if (on) Modifier.liftedPlate(c, inner, fill) else Modifier.background(fill, inner))
                            .selectable(on, press, indication = null, role = Role.RadioButton) {
                                onSelect(i)
                                open = false
                            }
                            .then(if (i == saved && !on) Modifier.semantics { stateDescription = savedChoice } else Modifier)
                            .touchStates(press, inner, c.press)
                            .drawBehind {
                                // The saved option, while another is chosen: the orange notch.
                                if (i == saved && !on) {
                                    val long = minOf(12.dp.toPx(), size.width)
                                    val thick = 3.dp.toPx()
                                    drawRoundRect(c.act, Offset(size.width / 2 - long / 2, size.height - thick - 3.dp.toPx()), Size(long, thick), CornerRadius(thick / 2))
                                }
                            }
                            .padding(horizontal = 11.dp, vertical = 6.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        BasicText(
                            option,
                            style = MaterialTheme.typography.bodyMedium.copy(
                                color = if (on) c.onPlate else c.ink,
                                fontWeight = if (on) FontWeight.SemiBold else FontWeight.Medium,
                                textDirection = TextDirection.Content,
                            ),
                        )
                    }
                }
            }
        }
    }
}

/** Under its anchor, start-aligned and kept inside the window. */
private object BelowAnchor : PopupPositionProvider {
    override fun calculatePosition(
        anchorBounds: androidx.compose.ui.unit.IntRect,
        windowSize: androidx.compose.ui.unit.IntSize,
        layoutDirection: androidx.compose.ui.unit.LayoutDirection,
        popupContentSize: androidx.compose.ui.unit.IntSize,
    ): IntOffset {
        val x = if (layoutDirection == androidx.compose.ui.unit.LayoutDirection.Ltr) anchorBounds.left else anchorBounds.right - popupContentSize.width
        val below = anchorBounds.bottom + 4
        val y = if (below + popupContentSize.height <= windowSize.height) below else maxOf(0, anchorBounds.top - popupContentSize.height - 4)
        return IntOffset(x.coerceIn(0, maxOf(0, windowSize.width - popupContentSize.width)), y)
    }
}

/** Keeps a column of selectors from growing past a hand's width beside the dials. */
@Composable
internal fun SelectorColumn(content: @Composable () -> Unit) {
    Column(Modifier.widthIn(max = 360.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) { content() }
}
