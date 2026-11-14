package io.github.oscyl.android

import android.content.Context
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import io.github.oscyl.android.files.AndroidFiles
import io.github.oscyl.android.midi.AndroidMidi
import io.github.oscyl.platform.AppStorage
import io.github.oscyl.platform.Files
import io.github.oscyl.platform.MidiPlatform
import io.github.oscyl.platform.OscylHaptics
import io.github.oscyl.platform.Platform
import io.github.oscyl.ui.shell.App
import android.graphics.drawable.ColorDrawable
import androidx.compose.ui.graphics.toArgb
import io.github.oscyl.core.Settings
import io.github.oscyl.core.Theme
import io.github.oscyl.theme.OscylTokens

/** A chosen theme's ground; null follows the system (the launch theme has it already). */
private fun groundOf(theme: Theme) = when (theme) {
    Theme.SYSTEM -> null
    Theme.LIGHT -> OscylTokens.Light.ground
    Theme.DARK -> OscylTokens.Dark.ground
    Theme.AMOLED -> OscylTokens.Amoled.ground
}

/** This Android device: haptics through the window, behind the Preferences switch; USB MIDI; app-private libraries; SAF files. */
private class AndroidPlatform(
    override val haptics: OscylHaptics,
    private val context: Context,
    override val files: Files,
) : Platform {
    override fun midi(): MidiPlatform = AndroidMidi(context)

    private val storage = AppStorage(context.filesDir.path)
    override val librariesDir: String = storage.libraries
    override val backupsDir: String = storage.backups
    override val unsyncedDir: String = storage.unsynced
    override val settingsPath: String = storage.settings
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val files = AndroidFiles.of(this).also { it.attach(this) }
        val platform = AndroidPlatform(AndroidHaptics(window.decorView), applicationContext, files)
        // The window behind the first frame wears the chosen theme's ground (AMOLED black too).
        val theme = Settings.load(platform.settingsPath).use { it.theme() }
        groundOf(theme)?.let { window.setBackgroundDrawable(ColorDrawable(it.toArgb())) }
        setContent { App(platform) }
        // Once the app has drawn its own ground, the window's is a second full-screen fill
        // on every frame (a cost a QHD phone feels): drop it after the first frame.
        val decor = window.decorView
        decor.viewTreeObserver.addOnDrawListener(object : android.view.ViewTreeObserver.OnDrawListener {
            override fun onDraw() {
                decor.post {
                    window.setBackgroundDrawable(null)
                    if (decor.viewTreeObserver.isAlive) decor.viewTreeObserver.removeOnDrawListener(this)
                }
            }
        })
    }
}
