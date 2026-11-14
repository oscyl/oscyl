package io.github.oscyl.library

import androidx.compose.ui.unit.dp
import androidx.compose.ui.test.swipeUp
import androidx.compose.ui.test.swipeLeft
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.hasTestTag
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollToIndex
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.core.ProgramRow
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.library.LibraryActions
import io.github.oscyl.ui.library.ProgramList
import io.github.oscyl.ui.library.ProgramsUi
import io.github.oscyl.ui.library.librarySource
import io.github.oscyl.ui.editor.samplePanel
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The program list over plain rows: search (what it keeps, hides, turns off) and the rows' pictures. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class ProgramListTest {
    @get:Rule val compose = createComposeRule()

    private val names = listOf("String Room", "Old Strings", "Mono Bass", "Café Keys") + (5..60).map { "Row $it" }
    private val rows = names.mapIndexed { i, n -> ProgramRow((i + 1).toString().padStart(3, '0'), n, edited = false, readOnly = false, loaded = true) }
    private var programs by mutableStateOf(ProgramsUi(librarySource("gig"), "Gig", rows, selected = null, inLibrary = true, canUndo = false, canRedo = false))
    private val selected = mutableListOf<Int>()
    private val picture = samplePanel(start = 3).graph(3)!!.now!!
    private val auditioned = mutableListOf<Int>()
    private val actions = object : LibraryActions {
        override fun selectProgram(index: Int) { selected += index }
        override fun thumbnail(index: Int) = picture
        override fun audition(index: Int) { auditioned += index }
        val placedNow = kotlinx.coroutines.flow.MutableSharedFlow<Int>(extraBufferCapacity = 1)
        override val placed = placedNow
        override fun newProgram() { placedNow.tryEmit(50) }
        override fun move(from: Int, to: Int) { moved += from to to }
    }
    private val moved = mutableListOf<Pair<Int, Int>>()

    private fun show() = compose.setContent { OscylTheme(OscylThemeMode.Dark) { ProgramList(programs, actions, onBack = null, topBarActions = false) } }

    private fun search(text: String) {
        // The search key turns into the field (here the list carries both: no shell).
        if (compose.onAllNodesWithTag("search-field").fetchSemanticsNodes().isEmpty()) compose.onNodeWithTag("search-open").performClick()
        compose.onNodeWithTag("search-field").performTextInput(text)
        compose.waitForIdle()
    }

    private fun rowsShown() = compose.onAllNodesWithTag("program-row").fetchSemanticsNodes().size

    @Test
    fun searchFiltersByNameAndSlot() {
        show()
        search("str")
        assertEquals(2, rowsShown())
        compose.onNodeWithText("String Room").performClick()
        // The document's index, not the place in the filtered list.
        assertEquals(listOf(0), selected)
        compose.onNodeWithTag("search-clear").performClick()
        search("011")
        assertEquals(1, rowsShown())
        compose.onNodeWithText("Row 11").assertExists()
        compose.onNodeWithTag("search-clear").performClick()
        search("cafe")
        compose.onNodeWithText("Café Keys").assertExists()
    }

    @Test
    fun newGoesToWhatItMade() {
        show()
        compose.onNodeWithText("String Room").assertExists()
        compose.runOnIdle { actions.newProgram() }
        compose.waitForIdle()
        // The list taken to the new program (here, row 51).
        compose.onNodeWithText("Row 51").assertExists()
        compose.onNodeWithText("String Room").assertDoesNotExist()
    }

    @Test
    fun filterKeepsOnlyEditedAndCloseBringsAllBack() {
        programs = programs.copy(rows = rows.mapIndexed { i, r -> if (i == 2) r.copy(edited = true) else r })
        val search = io.github.oscyl.ui.shell.ProgramSearch()
        compose.setContent {
            androidx.compose.runtime.CompositionLocalProvider(io.github.oscyl.ui.shell.LocalProgramSearch provides search) {
                OscylTheme(OscylThemeMode.Dark) { ProgramList(programs, actions, onBack = null, topBarActions = false) }
            }
        }
        compose.onNodeWithTag("search-open").performClick()
        compose.runOnIdle { search.filter = io.github.oscyl.ui.shell.SearchFilter.Edited }
        compose.waitForIdle()
        assertEquals(1, rowsShown())
        compose.onNodeWithText("Mono Bass").assertExists()
        // The ✕ of an empty field closes the search: filter and all.
        compose.onNodeWithTag("search-close").performClick()
        compose.waitForIdle()
        assertEquals(true, rowsShown() > 2)
    }

    @Test
    fun aHeldRowDraggedDownMovesThere() {
        show()
        val height = row("Mono Bass").fetchSemanticsNode().size.height.toFloat()
        row("Mono Bass").performTouchInput {
            down(center)
            advanceEventTime(viewConfiguration.longPressTimeoutMillis + 100)
            // Two rows and a quarter down: it lands on the second row under it.
            for (k in 1..9) moveBy(androidx.compose.ui.geometry.Offset(0f, height * 0.25f))
            up()
        }
        compose.waitForIdle()
        assertEquals(listOf(2 to 4), moved)
    }

    @Test
    fun swipingToHearBouncesUntilTheFirstTouch() {
        programs = programs.copy(canAudition = true)
        val taught = mutableListOf<String>()
        compose.mainClock.autoAdvance = false
        compose.setContent {
            androidx.compose.runtime.CompositionLocalProvider(
                io.github.oscyl.ui.preferences.LocalTeaching provides io.github.oscyl.ui.preferences.Teaching({ it in taught }, { taught += it }),
            ) { OscylTheme(OscylThemeMode.Dark) { ProgramList(programs, actions, onBack = null, topBarActions = false) } }
        }
        // The first row bounces open on "Play" and comes home: never left standing open.
        compose.mainClock.advanceTimeBy(400)
        val home = row("Mono Bass").fetchSemanticsNode().boundsInRoot.left
        val peek = row("String Room").fetchSemanticsNode().boundsInRoot.left
        assertTrue(peek != home)
        compose.mainClock.autoAdvance = true
        compose.waitForIdle()
        assertEquals(home, row("String Room").fetchSemanticsNode().boundsInRoot.left)
        assertTrue("taught before any touch: $taught", "swipe-play" !in taught)
        row("Mono Bass").performTouchInput { down(center); up() }
        compose.waitForIdle()
        assertEquals("taught: $taught", 1, taught.count { it == "swipe-play" })
    }

    @Test
    fun matchesAreUnderlined() {
        val marked = io.github.oscyl.ui.library.underlined("Old Strings", "str")
        assertEquals(listOf(4 to 7), marked.spanStyles.map { it.start to it.end })
        // Nothing to mark without words.
        assertEquals(0, io.github.oscyl.ui.library.underlined("Old Strings", "").spanStyles.size)
    }

    @Test
    fun scrubberTakesTheListToTheSlotUnderTheThumb() {
        show()
        compose.mainClock.autoAdvance = false
        // A fling: the scale shows while the list moves.
        compose.onNodeWithTag("program-list").performTouchInput { swipeUp(startY = bottom - 10f, endY = bottom - 400f, durationMillis = 100) }
        compose.mainClock.advanceTimeBy(64)
        compose.onNodeWithTag("slot-scrubber", useUnmergedTree = true).performTouchInput {
            down(bottomCenter.copy(y = bottom - 4f))
            up()
        }
        compose.mainClock.autoAdvance = true
        compose.waitForIdle()
        compose.onNodeWithText("Row 60").assertExists()
        compose.onNodeWithText("String Room").assertDoesNotExist()
    }

    @Test
    fun searchSaysHowManyMatch() {
        show()
        search("str")
        compose.onNodeWithText("2 of 60 programs").assertExists()
    }

    @Test
    fun searchWithNoMatchShowsEmptyState() {
        show()
        search("zzz")
        assertEquals(0, rowsShown())
        compose.onNodeWithText("Clear search").performClick()
        compose.waitForIdle()
        assertEquals(true, rowsShown() > 2)
    }

    @Test
    fun clearingSearchRestoresListAndScroll() {
        show()
        compose.onNodeWithTag("program-list").performScrollToIndex(40)
        compose.waitForIdle()
        compose.onNodeWithText("Row 40").assertExists()
        search("bass")
        assertEquals(1, rowsShown())
        compose.onNodeWithTag("search-clear").performClick()
        compose.waitForIdle()
        compose.onNodeWithText("Row 40").assertExists()
        compose.onNodeWithText("String Room").assertDoesNotExist()
    }

    @Test
    fun clearingALongSearchRestoresTheScroll() {
        show()
        compose.onNodeWithTag("program-list").performScrollToIndex(4)
        compose.waitForIdle()
        compose.onNodeWithText("Café Keys").assertExists()
        compose.onNodeWithText("String Room").assertDoesNotExist()
        // 56 rows match: the filtered list is longer than where the list stood.
        search("row")
        compose.onNodeWithTag("search-clear").performClick()
        compose.waitForIdle()
        compose.onNodeWithText("Café Keys").assertExists()
        compose.onNodeWithText("String Room").assertDoesNotExist()
    }

    @Test
    fun reorderIsOffWhileSearching() {
        show()
        search("row")
        val row = compose.onAllNodesWithTag("program-row").fetchSemanticsNodes().first()
        assertFalse(SemanticsActions.CustomActions in row.config)
    }

    @Test
    fun rowWithoutProgramShowsNoThumbnail() {
        programs = programs.copy(rows = rows.take(3) + ProgramRow("004", "", edited = false, readOnly = false, loaded = false))
        show()
        // Three read programs have their picture; the slot not read yet has none.
        assertEquals(3, compose.onAllNodesWithTag("program-thumbnail", useUnmergedTree = true).fetchSemanticsNodes().size)
    }

    /** A synth's slots as a read reaches them: the first [read] read, the rest still to come. */
    private fun reading(read: Int) = programs.copy(
        rows = rows.mapIndexed { i, r -> if (i < read) r else r.copy(name = "", loaded = false) },
        inLibrary = false,
        read = io.github.oscyl.ui.synth.ReadProgress(read, rows.size),
    )

    @Test
    fun rowsFillInAsTheyAreRead() {
        programs = reading(2)
        show()
        compose.onNodeWithText("String Room").assertExists()
        compose.onNodeWithText("Mono Bass").assertDoesNotExist()
        // A slot to come is a placeholder said as reading, with no picture and no menu.
        assertEquals(2, compose.onAllNodesWithTag("program-thumbnail", useUnmergedTree = true).fetchSemanticsNodes().size)
        compose.onNodeWithTag("read-stop").assertExists()
        programs = reading(3)
        compose.waitForIdle()
        compose.onNodeWithText("Mono Bass").assertExists()
        compose.onNodeWithText("Café Keys").assertDoesNotExist()
    }

    @Test
    fun listScrollsWhileReading() {
        programs = reading(1)
        show()
        compose.onNodeWithTag("program-list").performScrollToIndex(40)
        compose.waitForIdle()
        (2..rows.size).forEach { programs = reading(it); compose.waitForIdle() }
        // The read went on underneath; the list stayed where it was taken.
        compose.onNodeWithText("Row 40").assertExists()
        compose.onNodeWithText("String Room").assertDoesNotExist()
    }

    private fun row(name: String) = compose.onNode(hasText(name) and hasTestTag("program-row"), useUnmergedTree = false)

    @Test
    fun swipePastThresholdAuditionsOnce() {
        programs = programs.copy(canAudition = true)
        show()
        row("Mono Bass").performTouchInput { swipeLeft(startX = right - 8f, endX = right - 8f - 90.dp.toPx(), durationMillis = 300) }
        compose.waitForIdle()
        assertEquals(listOf(2), auditioned)
        assertEquals(emptyList<Int>(), selected)
    }

    @Test
    fun shortSwipeDoesNothing() {
        programs = programs.copy(canAudition = true)
        show()
        row("Mono Bass").performTouchInput { swipeLeft(startX = right - 8f, endX = right - 8f - 40.dp.toPx(), durationMillis = 300) }
        compose.waitForIdle()
        assertEquals(emptyList<Int>(), auditioned)
    }

    @Test
    fun noSwipeWithoutSynthOrOnUnreadRow() {
        show()
        row("Mono Bass").performTouchInput { swipeLeft(startX = right - 8f, endX = right - 8f - 90.dp.toPx(), durationMillis = 300) }
        compose.waitForIdle()
        assertEquals(emptyList<Int>(), auditioned)
        programs = reading(2).copy(canAudition = true)
        compose.waitForIdle()
        // A slot to come: no swipe, no Audition for a screen reader.
        val unread = compose.onAllNodesWithTag("program-row").fetchSemanticsNodes()[2]
        val labels = unread.config.getOrElse(SemanticsActions.CustomActions) { emptyList() }.map { it.label }
        assertFalse("Audition" in labels)
    }

    @Test
    fun auditionIsAScreenReaderAction() {
        programs = programs.copy(canAudition = true)
        show()
        val node = row("Mono Bass").fetchSemanticsNode()
        compose.runOnIdle { node.config[SemanticsActions.CustomActions].single { it.label == "Audition" }.action() }
        assertEquals(listOf(2), auditioned)
    }

    @Test
    fun verticalScrollStillWorksOverRows() {
        programs = programs.copy(canAudition = true)
        show()
        row("Mono Bass").performTouchInput { swipeUp(startY = bottom, endY = top - 600f) }
        compose.waitForIdle()
        compose.onNodeWithText("String Room").assertDoesNotExist()
        assertEquals(emptyList<Int>(), auditioned)
    }

    @Test
    fun searchSurvivesRotation() {
        val restoration = StateRestorationTester(compose)
        restoration.setContent { OscylTheme(OscylThemeMode.Dark) { ProgramList(programs, actions, onBack = null, topBarActions = false) } }
        search("str")
        restoration.emulateSavedInstanceStateRestore()
        compose.waitForIdle()
        assertEquals(2, rowsShown())
    }
}
