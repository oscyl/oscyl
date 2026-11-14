package io.github.oscyl.ui.parts

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf

/**
 * Insets standing in for the window's: previews and tests set them to draw a display
 * cutout on any edge (which Robolectric's window never has). Null: the window's own.
 */
val LocalFakeInsets = staticCompositionLocalOf<WindowInsets?> { null }

/**
 * What every part keeps clear: the system bars, the display cutout on any edge and the
 * IME (`WindowInsets.safeDrawing`), or the [LocalFakeInsets] standing in for them.
 */
val safeInsets: WindowInsets
    @Composable get() = LocalFakeInsets.current ?: WindowInsets.safeDrawing
