package io.github.oscyl.platform

import androidx.compose.runtime.Composable

/**
 * The system bars' icons follow the app's own theme, not the system's: dark icons over a
 * [light] ground, light ones over a dark ground (a light theme on a dark system kept white
 * icons on a white bar).
 */
@Composable
expect fun SystemBarsAppearance(light: Boolean)
