package io.github.oscyl.theme

import androidx.compose.ui.graphics.Color
import kotlin.math.max
import kotlin.math.min
import kotlin.test.Test
import kotlin.test.assertEquals

class TokensTest {
    private val themes = mapOf(
        "light" to OscylTokens.Light,
        "dark" to OscylTokens.Dark,
        "amoled" to OscylTokens.Amoled,
    )

    @Test
    fun darkGroundMatchesTokens() {
        assertEquals(Color(0xFF202122), OscylTokens.Dark.ground)
    }

    @Test
    fun amoledShadowIsTransparent() {
        assertEquals(0f, OscylTokens.Amoled.shadow.alpha)
    }

    @Test
    fun orangeIsTheOnlyAccent() {
        for ((name, colors) in themes) {
            val hued = colors.roles.filterValues(::isChromatic).keys
            assertEquals(setOf("on", "act"), hued, "chromatic roles in $name")
        }
    }

    @Test
    fun materialSchemeInventsNoHue() {
        for (mode in listOf(OscylThemeMode.Light, OscylThemeMode.Dark, OscylThemeMode.Amoled)) {
            val scheme = oscylColorScheme(mode)
            val hued = scheme.roles().filterValues(::isChromatic).keys
            assertEquals(emptySet(), hued, "chromatic Material roles in $mode")
        }
    }

    private fun isChromatic(c: Color): Boolean =
        max(c.red, max(c.green, c.blue)) - min(c.red, min(c.green, c.blue)) > CHROMA_LIMIT

    private companion object {
        // The greys are warm by a few levels (#62615c); orange and green are far above.
        const val CHROMA_LIMIT = 0.05f
    }
}

private fun androidx.compose.material3.ColorScheme.roles(): Map<String, Color> = mapOf(
    "primary" to primary, "onPrimary" to onPrimary,
    "primaryContainer" to primaryContainer, "onPrimaryContainer" to onPrimaryContainer,
    "inversePrimary" to inversePrimary,
    "secondary" to secondary, "onSecondary" to onSecondary,
    "secondaryContainer" to secondaryContainer, "onSecondaryContainer" to onSecondaryContainer,
    "tertiary" to tertiary, "onTertiary" to onTertiary,
    "tertiaryContainer" to tertiaryContainer, "onTertiaryContainer" to onTertiaryContainer,
    "background" to background, "onBackground" to onBackground,
    "surface" to surface, "onSurface" to onSurface,
    "surfaceVariant" to surfaceVariant, "onSurfaceVariant" to onSurfaceVariant,
    "surfaceTint" to surfaceTint,
    "inverseSurface" to inverseSurface, "inverseOnSurface" to inverseOnSurface,
    "error" to error, "onError" to onError,
    "errorContainer" to errorContainer, "onErrorContainer" to onErrorContainer,
    "outline" to outline, "outlineVariant" to outlineVariant, "scrim" to scrim,
    "surfaceBright" to surfaceBright, "surfaceDim" to surfaceDim,
    "surfaceContainer" to surfaceContainer, "surfaceContainerHigh" to surfaceContainerHigh,
    "surfaceContainerHighest" to surfaceContainerHighest, "surfaceContainerLow" to surfaceContainerLow,
    "surfaceContainerLowest" to surfaceContainerLowest,
    "primaryFixed" to primaryFixed, "primaryFixedDim" to primaryFixedDim,
    "onPrimaryFixed" to onPrimaryFixed, "onPrimaryFixedVariant" to onPrimaryFixedVariant,
    "secondaryFixed" to secondaryFixed, "secondaryFixedDim" to secondaryFixedDim,
    "onSecondaryFixed" to onSecondaryFixed, "onSecondaryFixedVariant" to onSecondaryFixedVariant,
    "tertiaryFixed" to tertiaryFixed, "tertiaryFixedDim" to tertiaryFixedDim,
    "onTertiaryFixed" to onTertiaryFixed, "onTertiaryFixedVariant" to onTertiaryFixedVariant,
)
