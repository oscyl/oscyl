package io.github.oscyl.theme

import androidx.compose.foundation.text.selection.TextSelectionColors
import androidx.compose.foundation.text.selection.LocalTextSelectionColors
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.em
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.albert_sans_bold
import org.jetbrains.compose.resources.Font

enum class OscylThemeMode { System, Light, Dark, Amoled }

/** Oscyl's own text styles, beside Material's type scale. */
@Immutable
class OscylType(
    /** The program title: Albert Sans Bold, the only place the face is used. */
    val programTitle: TextStyle,
)

val LocalOscylColors = staticCompositionLocalOf { OscylTokens.Light }
val LocalOscylType = staticCompositionLocalOf { OscylType(programTitle = TextStyle.Default) }

/**
 * Oscyl's Front Panel identity, generated from `design/tokens.json`: every colour role
 * through [LocalOscylColors], and a Material 3 theme mapped per DESIGN.md
 * ("Material 3 / Compose mapping") so anything that reads [MaterialTheme] stays achromatic.
 */
@Composable
fun OscylTheme(mode: OscylThemeMode = OscylThemeMode.System, content: @Composable () -> Unit) {
    val resolved = when (mode) {
        OscylThemeMode.System -> if (isSystemInDarkTheme()) OscylThemeMode.Dark else OscylThemeMode.Light
        else -> mode
    }
    val albertSans = FontFamily(Font(Res.font.albert_sans_bold, FontWeight.Bold))
    val typography = remember { oscylTypography() }
    val type = remember(albertSans, typography) {
        OscylType(
            programTitle = typography.headlineLarge.copy(
                fontFamily = albertSans,
                fontWeight = FontWeight.Bold,
                letterSpacing = (-0.02).em,
            ),
        )
    }
    CompositionLocalProvider(
        LocalOscylColors provides resolved.colors(),
        LocalOscylType provides type,
    ) {
        MaterialTheme(
            colorScheme = remember(resolved) { oscylColorScheme(resolved) },
            shapes = OscylShapes,
            typography = typography,
        ) {
            // Selected text and its handles in the panel's ink, never the platform's tint.
            val ink = LocalOscylColors.current.ink
            CompositionLocalProvider(LocalTextSelectionColors provides TextSelectionColors(handleColor = ink, backgroundColor = ink.copy(alpha = 0.25f)), content = content)
        }
    }
}

object OscylTheme {
    val colors: OscylColors
        @Composable @ReadOnlyComposable get() = LocalOscylColors.current
    val type: OscylType
        @Composable @ReadOnlyComposable get() = LocalOscylType.current
}

private fun OscylThemeMode.colors(): OscylColors = when (this) {
    OscylThemeMode.Light, OscylThemeMode.System -> OscylTokens.Light
    OscylThemeMode.Dark -> OscylTokens.Dark
    OscylThemeMode.Amoled -> OscylTokens.Amoled
}

private val OscylShapes = Shapes(
    extraSmall = RoundedCornerShape(OscylTokens.radiusInner),
    small = RoundedCornerShape(OscylTokens.radius),
    medium = RoundedCornerShape(OscylTokens.radius),
    large = RoundedCornerShape(OscylTokens.radiusPanel),
)

/** Platform face everywhere; legends are medium weight and tracked out. */
private fun oscylTypography(): Typography {
    val base = Typography()
    return base.copy(
        labelMedium = base.labelMedium.copy(fontWeight = FontWeight.Medium, letterSpacing = 0.08.em),
        // Running text untracked (Material's +0.5 sp reads loose); only the uppercase legends are tracked out.
        bodyLarge = base.bodyLarge.copy(letterSpacing = 0.em),
        bodyMedium = base.bodyMedium.copy(letterSpacing = 0.em),
        bodySmall = base.bodySmall.copy(letterSpacing = 0.em),
    )
}

/** The Material 3 colour scheme for [mode], mapped from the tokens per DESIGN.md. */
internal fun oscylColorScheme(mode: OscylThemeMode): ColorScheme {
    val c = mode.colors()
    return ColorScheme(
        primary = c.plate,
        onPrimary = c.onPlate,
        primaryContainer = c.plate,
        onPrimaryContainer = c.onPlate,
        inversePrimary = c.onPlate,
        secondary = c.ink,
        onSecondary = c.ground,
        secondaryContainer = c.raised,
        onSecondaryContainer = c.ink,
        tertiary = c.ink,
        onTertiary = c.ground,
        tertiaryContainer = c.raised,
        onTertiaryContainer = c.ink,
        background = c.ground,
        onBackground = c.ink,
        surface = c.ground,
        onSurface = c.ink,
        surfaceVariant = c.list,
        onSurfaceVariant = c.legend,
        // No tonal elevation: tinting with the surface itself changes nothing.
        surfaceTint = c.ground,
        inverseSurface = c.plate,
        inverseOnSurface = c.onPlate,
        // Errors are said in words and outlined in ink; no hue is borrowed.
        error = c.ink,
        onError = c.ground,
        errorContainer = c.raised,
        onErrorContainer = c.ink,
        outline = c.ruleStrong,
        outlineVariant = c.rule,
        scrim = c.shadow,
        surfaceBright = c.raised,
        surfaceContainer = c.list,
        surfaceContainerHigh = c.raised,
        surfaceContainerHighest = c.raised,
        surfaceContainerLow = c.sidebar,
        surfaceContainerLowest = c.raised,
        surfaceDim = c.sidebar,
        primaryFixed = c.plate,
        primaryFixedDim = c.plate,
        onPrimaryFixed = c.onPlate,
        onPrimaryFixedVariant = c.onPlate,
        secondaryFixed = c.raised,
        secondaryFixedDim = c.raised,
        onSecondaryFixed = c.ink,
        onSecondaryFixedVariant = c.ink,
        tertiaryFixed = c.raised,
        tertiaryFixedDim = c.raised,
        onTertiaryFixed = c.ink,
        onTertiaryFixedVariant = c.ink,
    )
}
