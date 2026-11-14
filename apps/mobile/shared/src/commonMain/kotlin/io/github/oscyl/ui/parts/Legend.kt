package io.github.oscyl.ui.parts

import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.intl.Locale
import androidx.compose.ui.text.toUpperCase
import io.github.oscyl.theme.OscylTheme

/** A small uppercase section legend (DESIGN.md Label), read as a heading. */
@Composable
fun Legend(text: String, modifier: Modifier = Modifier, maxLines: Int = Int.MAX_VALUE) {
    BasicText(
        text.toUpperCase(Locale.current),
        modifier.semantics { heading() },
        style = MaterialTheme.typography.labelMedium.copy(color = OscylTheme.colors.legend),
        maxLines = maxLines,
        overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
    )
}

/**
 * The start side as a fixed alignment. Names keep their own direction
 * (`TextDirection.Content`), which would otherwise also choose their side: a Latin
 * program name in a right-to-left layout would hug the left, away from its slot.
 */
@Composable
fun startAlign(): androidx.compose.ui.text.style.TextAlign =
    if (androidx.compose.ui.platform.LocalLayoutDirection.current == androidx.compose.ui.unit.LayoutDirection.Rtl) {
        androidx.compose.ui.text.style.TextAlign.Right
    } else {
        androidx.compose.ui.text.style.TextAlign.Left
    }
