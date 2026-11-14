package io.github.oscyl.platform

import androidx.compose.ui.Modifier

/** Keeps the system's own edge gesture (Android's Back swipe) off this part: its slide is the app's. */
expect fun Modifier.excludeSystemGestures(): Modifier
