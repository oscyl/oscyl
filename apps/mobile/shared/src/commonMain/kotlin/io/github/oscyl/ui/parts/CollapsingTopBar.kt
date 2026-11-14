package io.github.oscyl.ui.parts

import io.github.oscyl.ui.shell.ChromeScale
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.tooling.preview.PreviewParameter
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.app_menu
import io.github.oscyl.shared.generated.resources.ic_more
import io.github.oscyl.shared.generated.resources.ic_undo
import io.github.oscyl.shared.generated.resources.programs_undo
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.ThemeModes
import org.jetbrains.compose.resources.stringResource
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.gestures.ScrollableState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.layout
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.text
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens

/**
 * A collapsing top bar's progress, from its page's [scroll]: 0 at rest, with the page's
 * large title in the content, rising to 1 once that title has scrolled out under the
 * bar. Read [fraction] in the layout or draw phase only, so a scroll never recomposes.
 */
@Stable
class CollapsingBarState internal constructor(private val scroll: ScrollableState) {
    /** The large title's height (px), measured by [LargeTitle]. */
    internal var titleHeight by mutableIntStateOf(0)

    val fraction: () -> Float = {
        val h = titleHeight.toFloat()
        when (scroll) {
            is LazyListState -> when {
                scroll.firstVisibleItemIndex > 0 -> 1f
                h <= 0f -> 0f
                else -> (scroll.firstVisibleItemScrollOffset / h).coerceIn(0f, 1f)
            }
            is ScrollState -> if (h <= 0f) 0f else (scroll.value / h).coerceIn(0f, 1f)
            else -> 0f
        }
    }
}

/** The bar state of a page scrolled by [scroll] (a lazy list's state, or a column's). */
@Composable
fun rememberCollapsingBarState(scroll: ScrollableState): CollapsingBarState = remember(scroll) { CollapsingBarState(scroll) }

/** Marks the page's own title block (first in its scrolling content) for [state]: the bar lifts as it leaves. */
fun Modifier.collapsingTitle(state: CollapsingBarState): Modifier = layout { measurable, constraints ->
    val placeable = measurable.measure(constraints)
    state.titleHeight = placeable.height
    layout(placeable.width, placeable.height) { placeable.place(0, 0) }
}

/**
 * The page's large title, first in its scrolling content (DESIGN.md "Collapsing top
 * bar"). It is measured for the bar, and silent to screen readers: the bar's title is
 * the page's one heading, wherever it shows.
 */
@Composable
fun LargeTitle(text: String, state: CollapsingBarState, modifier: Modifier = Modifier, style: TextStyle = MaterialTheme.typography.headlineSmall) {
    BasicText(
        text,
        modifier
            .clearAndSetSemantics {}
            .collapsingTitle(state),
        style = style.copy(color = OscylTheme.colors.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content),
        maxLines = 3,
        overflow = TextOverflow.Ellipsis,
    )
}

/**
 * The top bar of a destination, one rule everywhere: [leading] is navigation only (Back
 * where the page has a parent, else nothing), the [title] and [subtitle], then
 * [trailing]: the page's own actions (undo, redo), the synth chip and the menu. At rest the bar is the
 * page itself and its title waits in the content ([LargeTitle]); as that title leaves,
 * it rises into the bar and the bar lifts with the floating shadow, fully scroll-linked
 * with no snap. Every per-frame value is read in draw and layout lambdas.
 *
 * Put it above the scrolling content; it pads for the top and side insets (the status
 * bar and a cutout on any of those edges) and draws over the content it lifts from.
 */
@Composable
fun CollapsingTopBar(
    state: CollapsingBarState,
    title: String,
    subtitle: String?,
    modifier: Modifier = Modifier,
    leading: @Composable RowScope.() -> Unit = {},
    trailing: @Composable RowScope.() -> Unit = {},
    /** In the title's place, at full strength (the program list's search field). */
    titleContent: (@Composable () -> Unit)? = null,
    /** The bar's least height: 0 for a bar with no actions (a pane beside another), which then keeps only its title's room. */
    minHeight: Dp = 56.dp,
) = ChromeScale {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val lift = state.fraction
    Row(
        modifier
            .fillMaxWidth()
            .barLift(c, lift)
            .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))
            .heightIn(min = minHeight)
            .padding(horizontal = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        leading()
        if (titleContent != null) {
            Box(Modifier.weight(1f).padding(horizontal = 8.dp)) { titleContent() }
        } else Box(
            Modifier
                .weight(1f)
                .padding(horizontal = 8.dp)
                // Said whole even while it waits unseen (alpha 0) for the large title to leave.
                .clearAndSetSemantics {
                    if (title.isNotEmpty()) {
                        heading()
                        text = AnnotatedString(title)
                    }
                },
        ) {
            Column(
                Modifier.graphicsLayer {
                    val a = lift()
                    alpha = a
                    // It rises the last few dp into place as the large title leaves.
                    translationY = (1f - a) * 8.dp.toPx()
                },
            ) {
                BasicText(
                    title,
                    style = type.titleMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textDirection = TextDirection.Content),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                if (subtitle != null) {
                    BasicText(
                        subtitle,
                        style = type.bodySmall.copy(color = c.legend, fontFeatureSettings = "tnum", textDirection = TextDirection.Content),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
        trailing()
    }
}

/** The bar lifted, its title risen: as it is once the page has scrolled. */
@Composable
private fun LiftedBar() {
    val state = remember { CollapsingBarState(LazyListState(firstVisibleItemIndex = 3)) }
    CompositionLocalProvider(LocalSynthStatus provides SynthStatus("minilogue xd", live = true)) {
        Box(Modifier.fillMaxWidth().height(160.dp).background(OscylTheme.colors.ground)) {
            CollapsingTopBar(
                state,
                title = "Velvet Pad",
                subtitle = "Filter",
                trailing = {
                    IconAction(Res.drawable.ic_undo, stringResource(Res.string.programs_undo), {})
                    SynthChip({}, compact = true)
                    IconAction(Res.drawable.ic_more, stringResource(Res.string.app_menu), {})
                },
            )
        }
    }
}

@OscylPreviews
@Composable
private fun collapsingTopBar(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = OscylTheme(mode) { LiftedBar() }

@OscylA11yPreviews
@Composable
private fun collapsingTopBar_a11y() = OscylTheme(OscylThemeMode.Dark) { LiftedBar() }

/**
 * A bar's lift as the page scrolls under it, [lift] 0..1 read in the draw phase only: the
 * raised fill, the floating shadow and a 1 dp rule at its foot (`rule-strong` on AMOLED,
 * where it is the whole lift). Drawn over what follows it.
 */
fun Modifier.barLift(c: io.github.oscyl.theme.OscylColors, lift: () -> Float): Modifier {
    val f = OscylTokens.Floating
    val amoled = c.shadow.alpha == 0f
    return zIndex(1f)
        .softShadow(RectangleShape, f.farRadius, DpOffset(0.dp, f.farOffset), c.shadow.copy(alpha = c.shadow.alpha * f.farAlpha)) { lift() }
        .softShadow(RectangleShape, f.nearRadius, DpOffset(0.dp, f.nearOffset), c.shadow.copy(alpha = c.shadow.alpha * f.nearAlpha)) { lift() }
        .drawBehind {
            val a = lift()
            if (a <= 0f) return@drawBehind
            drawRect(c.raised, alpha = a)
            val w = OscylTokens.ruleWidth.toPx()
            drawRect(if (amoled) c.ruleStrong else c.rule, Offset(0f, size.height - w), size.copy(height = w), alpha = a)
        }
}
