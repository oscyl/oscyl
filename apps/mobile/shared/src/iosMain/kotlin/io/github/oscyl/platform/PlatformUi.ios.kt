package io.github.oscyl.platform

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.window.DialogProperties
import platform.Foundation.NSNotificationCenter
import platform.Foundation.NSOperationQueue
import platform.UIKit.UIAccessibilityIsVoiceOverRunning
import platform.UIKit.UIAccessibilityVoiceOverStatusDidChangeNotification

@Composable
actual fun screenReaderOn(): Boolean {
    var on by remember { mutableStateOf(UIAccessibilityIsVoiceOverRunning()) }
    DisposableEffect(Unit) {
        val center = NSNotificationCenter.defaultCenter
        val observer = center.addObserverForName(UIAccessibilityVoiceOverStatusDidChangeNotification, null, NSOperationQueue.mainQueue) { _ ->
            on = UIAccessibilityIsVoiceOverRunning()
        }
        onDispose { center.removeObserver(observer) }
    }
    return on
}

// ponytail: common Compose has no target inset; iOS reads the inset itself, so what stands on
// the keyboard leaves once it has gone. Use the keyboard's end frame if that shows.
@Composable
actual fun imeHeading(): Boolean = WindowInsets.ime.getBottom(LocalDensity.current) > 0

actual fun Modifier.testTagsAsIds(): Modifier = this

actual fun panelDialogProperties(): DialogProperties = DialogProperties(usePlatformDefaultWidth = false)
