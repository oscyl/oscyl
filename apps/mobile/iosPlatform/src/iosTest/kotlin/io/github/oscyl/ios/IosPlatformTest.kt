package io.github.oscyl.ios

import kotlin.test.Test
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import platform.Foundation.NSFileManager

/** Runs on the simulator (`:iosPlatform:iosSimulatorArm64Test`, macOS CI). */
class IosPlatformTest {
    @Test
    fun appSupportIsAFolderTheAppCanWrite() {
        val dir = appSupportDir()
        assertTrue(NSFileManager.defaultManager.isWritableFileAtPath(dir), dir)
    }

    @Test
    fun aLanguageIsPreferred() {
        assertNotNull(preferredLanguage())
    }

    @Test
    fun coreMidiListsWhatIsPluggedAndLetsGo() {
        // The simulator has no USB synth: the list is empty. A test binary may get no
        // MIDI server at all; then it says so instead of failing.
        val midi = CoreMidi {}
        assertTrue(midi.ports().all { it.id.isNotEmpty() })
        if (!midi.available) assertTrue(midi.ports().isEmpty())
        midi.close()
        assertTrue(midi.ports().isEmpty(), "closed: no ports")
    }
}
