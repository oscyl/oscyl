package io.github.oscyl.ui.preview

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import io.github.oscyl.ui.parts.LocalFakeInsets
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewParameterProvider
import androidx.compose.ui.unit.dp
import io.github.oscyl.platform.MidiPlatform
import io.github.oscyl.platform.NoHaptics
import io.github.oscyl.platform.NoMidi
import io.github.oscyl.platform.OscylHaptics
import io.github.oscyl.platform.Platform
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode

/*
 * Previews are the single source of the screenshot goldens: Roborazzi turns every
 * @Preview under io.github.oscyl into a Robolectric capture (see androidApp's build).
 * They take plain state only, so they render fast in the IDE.
 */

/** A component on the ground in [mode], 12 dp from the edges as the island floats in the app. */
@Composable
fun PreviewTheme(mode: OscylThemeMode = OscylThemeMode.Dark, content: @Composable () -> Unit) {
    OscylTheme(mode) {
        Box(Modifier.background(OscylTheme.colors.ground).padding(12.dp)) { content() }
    }
}

/** Dark, light and AMOLED, one capture each; take the mode with `@PreviewParameter(ThemeModes::class)`. */
class ThemeModes : PreviewParameterProvider<OscylThemeMode> {
    override val values = sequenceOf(OscylThemeMode.Dark, OscylThemeMode.Light, OscylThemeMode.Amoled)
    override fun getDisplayName(index: Int): String = values.elementAt(index).name.lowercase()
}

/** Every theme (pair with a [ThemeModes] parameter). */
@Preview(name = "themes", device = PHONE)
annotation class OscylPreviews

/** The accessibility and i18n bar: 200% text, mirrored, pseudo-locale, the smallest phone, largest display size. */
@Preview(name = "fontScale2_360dp", fontScale = 2f, device = PHONE)
@Preview(name = "rtl", locale = "ar", device = PHONE)
@Preview(name = "pseudoLocale_xa", locale = "en-rXA", device = PHONE)
@Preview(name = "smallPhone_320dp", device = "spec:width=320dp,height=640dp,dpi=320")
@Preview(name = "displaySizeLargest", device = "spec:width=277dp,height=615dp,dpi=624")
@Preview(name = "fontScale2_landscape", fontScale = 2f, device = PHONE_LANDSCAPE)
annotation class OscylA11yPreviews

/** The other window shapes (the phone in portrait is [OscylPreviews]): phone in landscape, unfolded foldable, tablet in landscape. */
@Preview(name = "compact_landscape", device = PHONE_LANDSCAPE)
@Preview(name = "medium_foldable", device = "spec:width=600dp,height=900dp,dpi=320")
@Preview(name = "expanded_tablet", device = "spec:width=1280dp,height=800dp,dpi=240")
annotation class OscylSizePreviews

/** A 360 dp phone at 3x. */
const val PHONE = "spec:width=360dp,height=800dp,dpi=480"

/** The same phone turned on its side: medium width, compact height. */
const val PHONE_LANDSCAPE = "spec:width=800dp,height=360dp,dpi=480"

/**
 * A display cutout to preview against (Robolectric's window has none): the insets a
 * phone would report, the status and navigation bars included, and where the cutout's
 * black shape sits. [landscape] previews take [PHONE_LANDSCAPE].
 */
enum class FakeCutout(val left: Int, val top: Int, val right: Int, val bottom: Int, val landscape: Boolean = false) {
    /** A centred punch-hole camera in the status bar. */
    PunchHole(0, 36, 0, 24),

    /** A camera in the top corner, taller than the status bar. */
    Corner(0, 44, 0, 24),

    /** A tall notch, centred. */
    TallNotch(0, 56, 0, 24),

    /** A cutout at the top and another at the foot. */
    Double(0, 48, 0, 48),

    /** On its side, rotated 90°: the cutout and the status bar on the left edge. */
    Landscape90(48, 24, 24, 0, landscape = true),

    /** On its side, rotated 270°: the cutout on the right edge, the navigation bar on the left. */
    Landscape270(24, 24, 48, 0, landscape = true),
}

/** One capture per portrait [FakeCutout] (on [PHONE]); take it with `@PreviewParameter(PortraitCutouts::class)`. */
class PortraitCutouts : PreviewParameterProvider<FakeCutout> {
    override val values = FakeCutout.entries.filter { !it.landscape }.asSequence()
    override fun getDisplayName(index: Int): String = values.elementAt(index).name.lowercase()
}

/** One capture per landscape [FakeCutout] (on [PHONE_LANDSCAPE]), both rotations. */
class LandscapeCutouts : PreviewParameterProvider<FakeCutout> {
    override val values = FakeCutout.entries.filter { it.landscape }.asSequence()
    override fun getDisplayName(index: Int): String = values.elementAt(index).name.lowercase()
}

/**
 * [content] laid out against [cutout]'s insets (through [LocalFakeInsets]), with the
 * cutout drawn over it in black, as the glass would show it.
 */
@Composable
fun CutoutFrame(cutout: FakeCutout, content: @Composable () -> Unit) {
    val insets = WindowInsets(left = cutout.left.dp, top = cutout.top.dp, right = cutout.right.dp, bottom = cutout.bottom.dp)
    CompositionLocalProvider(LocalFakeInsets provides insets) {
        Box(Modifier.fillMaxSize()) {
            content()
            Box(
                Modifier.fillMaxSize().drawWithContent {
                    val black = Color.Black
                    val r = 12.dp.toPx()
                    when (cutout) {
                        FakeCutout.PunchHole -> drawCircle(black, r, Offset(size.width / 2, 18.dp.toPx()))
                        FakeCutout.Corner -> drawCircle(black, 14.dp.toPx(), Offset(28.dp.toPx(), 24.dp.toPx()))
                        FakeCutout.TallNotch -> drawRoundRect(black, Offset(size.width / 2 - 80.dp.toPx(), -r), Size(160.dp.toPx(), 56.dp.toPx() + r), CornerRadius(r))
                        FakeCutout.Double -> {
                            drawRoundRect(black, Offset(size.width / 2 - 60.dp.toPx(), -r), Size(120.dp.toPx(), 48.dp.toPx() + r), CornerRadius(r))
                            drawRoundRect(black, Offset(size.width / 2 - 60.dp.toPx(), size.height - 48.dp.toPx()), Size(120.dp.toPx(), 48.dp.toPx() + r), CornerRadius(r))
                        }
                        FakeCutout.Landscape90 -> drawRoundRect(black, Offset(-r, size.height / 2 - 70.dp.toPx()), Size(48.dp.toPx() + r, 140.dp.toPx()), CornerRadius(r))
                        FakeCutout.Landscape270 -> drawRoundRect(black, Offset(size.width - 48.dp.toPx(), size.height / 2 - 70.dp.toPx()), Size(48.dp.toPx() + r, 140.dp.toPx()), CornerRadius(r))
                    }
                },
            )
        }
    }
}

/** A device with nothing attached. */
object PreviewPlatform : Platform {
    override val haptics: OscylHaptics = NoHaptics

    override fun midi(): MidiPlatform = NoMidi
}
