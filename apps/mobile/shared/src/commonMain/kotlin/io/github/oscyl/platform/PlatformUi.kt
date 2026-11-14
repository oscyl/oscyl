package io.github.oscyl.platform

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.window.DialogProperties

/**
 * A screen reader is running (TalkBack, VoiceOver): what is said must wait for it, so a
 * notice with an action never goes by itself.
 */
@Composable
expect fun screenReaderOn(): Boolean

/**
 * The keyboard is up or on its way up: where it is going, not where it is, so what stands
 * on it leaves as it starts to close and arrives as it starts to open.
 */
@Composable
expect fun imeHeading(): Boolean

/** Test tags double as resource ids (Android's UI Automator finds the parts by name). */
expect fun Modifier.testTagsAsIds(): Modifier

/** A panel's dialog window: the whole window, its insets left to the panel. */
expect fun panelDialogProperties(): DialogProperties
