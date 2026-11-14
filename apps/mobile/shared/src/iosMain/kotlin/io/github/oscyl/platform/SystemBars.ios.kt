package io.github.oscyl.platform

import androidx.compose.runtime.Composable

// ponytail: no-op on iOS; set the hosting controller's preferredStatusBarStyle when the iOS app ships themes.
@Composable
actual fun SystemBarsAppearance(light: Boolean) = Unit
