package io.github.oscyl.a11y

import io.github.oscyl.ui.generate.GenerateScreen
import io.github.oscyl.ui.generate.NoGenerateActions
import io.github.oscyl.ui.generate.SampleGenerate
import io.github.oscyl.ui.welcome.GuideScreen
import io.github.oscyl.ui.welcome.WelcomeScreen
import io.github.oscyl.core.Theme
import io.github.oscyl.ui.preferences.NoPreferencesActions
import io.github.oscyl.ui.preferences.Prefs
import io.github.oscyl.ui.preferences.PreferencesScreen

import androidx.compose.runtime.remember
import io.github.oscyl.ui.editor.EditorHead
import io.github.oscyl.ui.review.NoReviewActions
import io.github.oscyl.ui.review.ReviewSheet
import io.github.oscyl.ui.review.ReviewState
import io.github.oscyl.ui.review.ReviewUi
import io.github.oscyl.ui.review.SampleReview
import io.github.oscyl.ui.editor.EditorScreen
import io.github.oscyl.ui.editor.NoEditorActions
import io.github.oscyl.ui.editor.SampleHead
import io.github.oscyl.ui.editor.samplePanel

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.accessibility.enableAccessibilityChecks
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.tryPerformAccessibilityChecks
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.compose.ui.test.longClick
import androidx.compose.ui.test.performTouchInput
import io.github.oscyl.core.DriverInfo
import io.github.oscyl.core.MenuEntry
import io.github.oscyl.core.ProgramAction
import io.github.oscyl.core.ProgramMenu
import io.github.oscyl.ui.library.LibraryActions
import io.github.oscyl.ui.library.LibraryScreen
import io.github.oscyl.ui.library.LibraryUi
import io.github.oscyl.ui.library.NoLibraryActions
import io.github.oscyl.ui.library.Notice
import io.github.oscyl.ui.library.SampleSynth
import io.github.oscyl.ui.library.SampleUi
import io.github.oscyl.platform.MidiPortInfo
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.preview.PreviewPlatform
import io.github.oscyl.ui.shell.App
import io.github.oscyl.ui.synth.SynthCard
import io.github.oscyl.ui.synth.SynthScreen
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Accessibility Test Framework checks (touch targets, labels, contrast, ...) on a real
 * device, where ATF runs: the island shell, every Synth and Library screen state, in each theme.
 * Any ATF error fails the test.
 */
@RunWith(AndroidJUnit4::class)
class AccessibilityChecksTest {
    @get:Rule val compose = createComposeRule()

    @Before
    fun enableChecks() = compose.enableAccessibilityChecks()

    private var mode by mutableStateOf(OscylThemeMode.Dark)

    /** Checks [content] once per theme. */
    private fun checkEachTheme(content: @Composable () -> Unit) {
        compose.setContent {
            OscylTheme(mode) { Box(Modifier.fillMaxSize().background(OscylTheme.colors.ground)) { content() } }
        }
        for (theme in listOf(OscylThemeMode.Dark, OscylThemeMode.Light, OscylThemeMode.Amoled)) {
            mode = theme
            compose.waitForIdle()
            compose.onRoot().tryPerformAccessibilityChecks()
        }
    }

    private fun synth(cards: List<SynthCard> = emptyList(), access: List<MidiPortInfo> = emptyList()) = checkEachTheme {
        SynthScreen(cards, access, notice = null, DRIVERS, onDemo = {}, onDisconnect = {}, onConnect = {}, onAllow = {}, onAbout = {})
    }

    @Test
    fun islandShell() {
        // The app itself (light, as the device is), then the synths' page from the chip.
        compose.setContent { App(PreviewPlatform) }
        compose.onRoot().tryPerformAccessibilityChecks()
        compose.onNodeWithContentDescription("Synths:", substring = true).performClick()
        compose.waitForIdle()
        compose.onRoot().tryPerformAccessibilityChecks()
    }

    @Test
    fun synthEmpty() = synth()

    @Test
    fun synthDemoConnected() = synth(cards = listOf(DEMO, USB))

    @Test
    fun synthPermission() = synth(access = listOf(MidiPortInfo("minilogue xd", "7")))

    @Test
    fun synthUnitsAndBackups() = checkEachTheme {
        SynthScreen(
            listOf(DEMO.copy(read = io.github.oscyl.ui.synth.ReadProgress(10, 500))), emptyList(), notice = null, DRIVERS,
            onDemo = {}, onDisconnect = {}, onConnect = {}, onAllow = {}, onAbout = {},
            units = mapOf(1uL to listOf(io.github.oscyl.ui.synth.UnitSlot("osc", 0u, "MorphTbl", "Oscillators"), io.github.oscyl.ui.synth.UnitSlot("osc", 1u, null, "Oscillators"))),
            backups = listOf(io.github.oscyl.core.BackupFile("/b", "Synth backup 2026-11-13 10.04.11.mnlgxdlib", 1u)),
        )
    }

    @Test
    fun synthDisconnected() = synth(cards = listOf(USB.copy(connected = false)))

    @Test
    fun aboutPassesAccessibilityChecks() {
        // The real page over the shipped notices, then one licence.
        compose.setContent { App(PreviewPlatform) }
        compose.onNodeWithContentDescription("More options").performClick()
        compose.onNodeWithText("About Oscyl").performClick()
        compose.waitUntil(10_000) { compose.onAllNodesWithText("Albert Sans").fetchSemanticsNodes().isNotEmpty() }
        compose.onRoot().tryPerformAccessibilityChecks()
        compose.onNode(hasScrollAction()).performScrollToNode(hasText("anyhow"))
        compose.onNodeWithText("anyhow").performClick()
        compose.waitUntil(10_000) { compose.onAllNodesWithText("Permission is hereby granted", substring = true).fetchSemanticsNodes().isNotEmpty() }
        compose.onRoot().tryPerformAccessibilityChecks()
    }

    private fun library(ui: LibraryUi, actions: LibraryActions = NoLibraryActions) = checkEachTheme {
        LibraryScreen(ui, actions, onOpenSynth = {})
    }

    @Test
    fun librarySources() = library(SampleUi.copy(programs = null))

    @Test
    fun libraryPrograms() = library(SampleUi)

    @Test
    fun librarySynthReading() = library(SampleUi.copy(programs = SampleSynth))

    @Test
    fun libraryEmpty() = library(LibraryUi())

    @Test
    fun libraryNotice() = library(SampleUi.copy(notice = Notice.Removed("Brass Stab")))

    @Test
    fun programMenu() {
        val menu = ProgramMenu(
            sections = listOf(
                listOf(MenuEntry(ProgramAction.RENAME, true), MenuEntry(ProgramAction.COPY, true), MenuEntry(ProgramAction.PASTE_OVER, false)),
                listOf(MenuEntry(ProgramAction.REVERT, true), MenuEntry(ProgramAction.REMOVE, true)),
            ),
            targets = emptyList(),
        )
        library(SampleUi, object : LibraryActions { override suspend fun menu(index: Int) = menu })
        compose.onNodeWithText("Glass Bells").performTouchInput { longClick() }
        compose.waitUntil(10_000) { compose.onAllNodesWithText("Paste sound here").fetchSemanticsNodes().isNotEmpty() }
        // The dialog is its own window: check it too.
        compose.onNodeWithText("Paste sound here").tryPerformAccessibilityChecks()
    }

    private fun editor(head: EditorHead?, start: Int = 0, comparing: Boolean = false) = checkEachTheme {
        val panel = remember { samplePanel(start = start).also { it.comparing = comparing } }
        EditorScreen(head?.copy(comparing = comparing), panel, NoEditorActions, onOpenLibrary = {})
    }

    @Test
    fun editorTop() = editor(SampleHead)

    @Test
    fun editorFilter() = editor(SampleHead, start = 3)

    @Test
    fun editorComparing() = editor(SampleHead, start = 3, comparing = true)

    @Test
    fun editorEffects() = editor(SampleHead, start = 5)

    @Test
    fun editorReadOnly() = editor(SampleHead.copy(readOnly = true, canLive = false))

    @Test
    fun editorEmpty() = editor(null)

    private fun review(ui: ReviewUi) = checkEachTheme { Box(Modifier.fillMaxSize()) { ReviewSheet(ui, NoReviewActions) } }

    @Test
    fun reviewSave() = review(SampleReview)

    @Test
    fun reviewSyncing() = review(SampleReview.copy(sync = true, state = ReviewState.Writing(1, 2)))

    @Test
    fun reviewFailed() = review(SampleReview.copy(sync = true, state = ReviewState.SyncFailed("The synth did not answer.")))

    @Test
    fun preferences() = checkEachTheme {
        PreferencesScreen(Prefs(theme = Theme.AMOLED, demo = "korg-prologue"), DRIVERS, NoPreferencesActions, onBack = {})
    }

    @Test
    fun generate() = checkEachTheme { GenerateScreen(SampleGenerate, NoGenerateActions) }

    @Test
    fun welcome() = checkEachTheme { WelcomeScreen(emptyList(), onRead = {}, onTry = {}, onSkip = {}) }

    @Test
    fun welcomeFound() = checkEachTheme { WelcomeScreen(listOf(USB), onRead = {}, onTry = {}, onSkip = {}) }

    @Test
    fun picker() = checkEachTheme {
        io.github.oscyl.ui.synth.SynthPicker(DRIVERS, emptyList(), emptyList(), onDemo = {}, onAllow = {}, onConnect = {}, onBack = {})
    }

    @Test
    fun guide() = checkEachTheme { GuideScreen(onBack = {}) }

    private companion object {
        val DRIVERS = listOf(
            DriverInfo("korg-minilogue-xd", "KORG", "minilogue xd"),
            DriverInfo("korg-prologue", "KORG", "prologue"),
            DriverInfo("sequential-prophet-rev2", "Sequential", "Prophet Rev2"),
        )
        val DEMO = SynthCard(1u, "minilogue xd", firmware = null, simulated = true, port = "")
        val USB = SynthCard(2u, "prologue", firmware = "2.10", simulated = false, port = "9:prologue _ SOUND")
    }
}
