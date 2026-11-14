package io.github.oscyl.baselineprofile

import androidx.benchmark.macro.junit4.BaselineProfileRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** The app's Baseline Profile: start, open a program, then every journey the frame benchmarks measure. */
@RunWith(AndroidJUnit4::class)
class BaselineProfileGenerator {
    @get:Rule val rule = BaselineProfileRule()

    @Test
    fun generate() = rule.collect(Package, includeInStartupProfile = true) {
        openEditor()
        scrollEditor()
        turnKnob()
        growGraph()
        openSheet()
        switchDestinations()
    }
}
