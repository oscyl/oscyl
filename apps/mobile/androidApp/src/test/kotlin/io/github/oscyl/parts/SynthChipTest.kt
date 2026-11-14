package io.github.oscyl.parts

import io.github.oscyl.ui.parts.shortModel
import org.junit.Assert.assertEquals
import org.junit.Test

/** The full chip names the synth by its short name beside the nameplate. */
class SynthChipTest {
    @Test
    fun shortModelKeepsShortLastWord() {
        assertEquals("xd", shortModel("minilogue xd"))
        assertEquals("Rev2", shortModel("Prophet Rev2"))
        assertEquals("prologue", shortModel("prologue"))
        assertEquals("", shortModel(""))
    }
}
