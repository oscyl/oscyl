package io.github.oscyl.baselineprofile

import androidx.benchmark.macro.MacrobenchmarkScope
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Direction
import androidx.test.uiautomator.UiObject2
import androidx.test.uiautomator.Until

/** The benchmark variant's package: its own install, its own data. */
const val Package = "io.github.oscyl.bench"

private const val Wait = 10_000L

private fun MacrobenchmarkScope.tag(id: String): UiObject2? = device.wait(Until.findObject(By.res(id)), Wait)

private fun MacrobenchmarkScope.must(id: String): UiObject2 = tag(id) ?: error("no '$id' on screen")

/**
 * From a cold or warm start to the editor on a demo program: the first run tries the
 * minilogue xd demo (its demo library comes with it) the first time, then the
 * Library's demo library, its eleventh program.
 */
fun MacrobenchmarkScope.openEditor() {
    startActivityAndWait()
    // The first run, or the app already past it.
    device.wait(Until.hasObject(By.res(java.util.regex.Pattern.compile("first-try|tab-Library"))), Wait)
    device.findObject(By.res("first-try"))?.let { first ->
        first.click()
        must("plate-korg-minilogue-xd").click()
    }
    must("tab-Library").click()
    tag("program-row") ?: must("library-row").click()
    device.wait(Until.hasObject(By.res("program-row")), Wait)
    device.findObjects(By.res("program-row")).getOrNull(10)?.click() ?: must("program-row").click()
    must("editor-list")
    device.waitForIdle()
}

fun MacrobenchmarkScope.scrollEditor() {
    val list = must("editor-list")
    list.setGestureMargin(device.displayWidth / 8)
    repeat(3) {
        list.fling(Direction.DOWN)
        list.fling(Direction.UP)
    }
}

fun MacrobenchmarkScope.turnKnob() {
    val knob = must("knob")
    val c = knob.visibleCenter
    val travel = device.displayHeight / 8
    repeat(3) {
        // Slow, then fast, up and down: the fine and the coarse gain.
        device.drag(c.x, c.y, c.x, c.y - travel, 120)
        device.drag(c.x, c.y, c.x, c.y + travel, 20)
    }
}

fun MacrobenchmarkScope.switchDestinations() {
    listOf("tab-Library", "tab-Generate", "tab-Editor", "tab-Library", "tab-Editor").forEach {
        must(it).click()
        device.waitForIdle()
    }
}

fun MacrobenchmarkScope.growGraph() {
    repeat(3) {
        must("graph-expand").click()
        must("graph-close").click()
        device.waitForIdle()
    }
}

fun MacrobenchmarkScope.openSheet() {
    repeat(3) {
        must("synth-chip").click()
        must("sheet-grip").click()
        device.waitForIdle()
    }
}

/** The demo library's program list: flings down and back up. */
fun MacrobenchmarkScope.scrollPrograms() {
    must("tab-Library").click()
    tag("program-row") ?: must("library-row").click()
    device.wait(Until.hasObject(By.res("program-row")), Wait)
    val list = must("program-list")
    list.setGestureMargin(device.displayWidth / 8)
    repeat(3) {
        list.fling(Direction.DOWN)
        list.fling(Direction.UP)
    }
}

