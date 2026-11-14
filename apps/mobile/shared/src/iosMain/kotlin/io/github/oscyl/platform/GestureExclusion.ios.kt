package io.github.oscyl.platform

import androidx.compose.ui.Modifier

// iOS's edge swipe belongs to navigation controllers, which this app does not use.
actual fun Modifier.excludeSystemGestures(): Modifier = this
