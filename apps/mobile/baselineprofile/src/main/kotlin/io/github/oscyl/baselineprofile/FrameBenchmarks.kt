package io.github.oscyl.baselineprofile

import androidx.benchmark.macro.CompilationMode
import androidx.benchmark.macro.FrameTimingMetric
import androidx.benchmark.macro.MacrobenchmarkScope
import androidx.benchmark.macro.StartupMode
import androidx.benchmark.macro.junit4.MacrobenchmarkRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Frame times of the redesign's journeys (FrameTimingMetric), with the Baseline
 * Profile as an install would have it. Targets: p95 frame duration at most 8.3 ms on
 * the Pixel 7 Pro (120 Hz) and 16.7 ms on the Pixel 2 XL (60 Hz).
 */
@RunWith(AndroidJUnit4::class)
class FrameBenchmarks {
    @get:Rule val rule = MacrobenchmarkRule()

    private fun measure(journey: MacrobenchmarkScope.() -> Unit) = rule.measureRepeated(
        packageName = Package,
        metrics = listOf(FrameTimingMetric()),
        compilationMode = CompilationMode.Partial(),
        startupMode = StartupMode.WARM,
        // -e iterations 2 while tuning one journey; 5 for the numbers in the docs.
        iterations = androidx.test.platform.app.InstrumentationRegistry.getArguments().getString("iterations")?.toIntOrNull() ?: 5,
        setupBlock = { openEditor() },
    ) { journey() }

    @Test fun editorScroll() = measure { scrollEditor() }

    @Test fun programListScroll() = measure { scrollPrograms() }

    @Test fun knobDrag() = measure { turnKnob() }

    @Test fun graphGrowAndBack() = measure { growGraph() }

    @Test fun destinationSwitch() = measure { switchDestinations() }

    @Test fun synthSheet() = measure { openSheet() }
}
