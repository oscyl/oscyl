package io.github.oscyl.library

import androidx.compose.runtime.collectAsState
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.test.ext.junit.runners.AndroidJUnit4
import android.content.Context
import androidx.test.core.app.ApplicationProvider
import io.github.oscyl.android.files.AndroidFiles
import io.github.oscyl.core.Libraries
import io.github.oscyl.core.exportFile
import io.github.oscyl.core.importFile
import io.github.oscyl.platform.Files
import io.github.oscyl.platform.PickedFile
import io.github.oscyl.platform.Sink
import io.github.oscyl.platform.safeFileName
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.library.LibraryScreen
import io.github.oscyl.ui.library.Notice
import io.github.oscyl.ui.library.librarySource
import java.io.File
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** Import, export and share through a fake platform, and migration of old libraries, over the real core. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class LibraryFilesTest {
    @get:Rule val compose = createComposeRule()

    @get:Rule val temp = TemporaryFolder()

    private val store = ViewModelStore()

    @After
    fun clear() = store.clear()

    /** Picks what [next] holds; keeps what is saved and shared. */
    private class FakeFiles : Files {
        var next: PickedFile? = null
        val saved = mutableMapOf<String, ByteArray>()
        val shared = mutableMapOf<String, ByteArray>()

        override suspend fun open(purpose: io.github.oscyl.platform.PickFor) = next

        override suspend fun create(name: String, mime: String) = Sink { saved[name] = it }

        override fun share(name: String, bytes: ByteArray, mime: String) {
            shared[name] = bytes
        }
    }

    private fun show(files: Files, dir: File = temp.newFolder("libraries")): LibraryModel {
        val model = ViewModelProvider.create(store, viewModelFactory { initializer { LibraryModel(dir.path, files = files) } })[LibraryModel::class]
        compose.setContent { OscylTheme(OscylThemeMode.Dark) { androidx.compose.foundation.layout.Box {
                LibraryScreen(model, onOpenSynth = {})
                io.github.oscyl.ui.library.NoticeHost(model.ui.collectAsState().value, model)
            } } }
        return model
    }

    /** Waits for [condition] on the model, letting the app's coroutines run between looks. */
    private fun await(condition: () -> Boolean) = compose.waitUntil(TIMEOUT_MS) { compose.runOnIdle(condition) }

    private fun awaitText(text: String) = compose.waitUntil(TIMEOUT_MS) {
        compose.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty()
    }

    /** A library of three named programs, in format [ext], made by the core. */
    private fun fixture(ext: String): ByteArray {
        val libraries = Libraries.open(temp.newFolder().path)
        val id = libraries.create("Fixture", XD, null)
        val doc = libraries.read(id)
        listOf("Lead", "Pad", "Bass").forEachIndexed { i, name ->
            doc.newProgram(null)
            doc.setName(i.toUInt(), name)
        }
        return exportFile(doc, ext)
    }

    @Test
    fun importsVendorAndOscylFiles() {
        val files = FakeFiles()
        val model = show(files)
        for ((name, ext) in listOf("Gig" to "mnlgxdlib", "Pads" to "oscyllib")) {
            files.next = PickedFile("$name.$ext", fixture(ext))
            compose.onNodeWithText("Import file").performClick()
            awaitText(name)
            // The new library is shown (on a phone, its programs pane); Back returns to the sources.
            await { model.ui.value.programs?.title == name }
            assertEquals(listOf("Lead", "Pad", "Bass"), model.ui.value.programs!!.rows.map { it.name })
            compose.onNodeWithContentDescription("Back").performClick()
        }
    }

    @Test
    fun exportsRoundTripAndShares() {
        val files = FakeFiles()
        val model = show(files)
        files.next = PickedFile("Gig.mnlgxdlib", fixture("mnlgxdlib"))
        compose.onNodeWithText("Import file").performClick()
        awaitText("Gig")
        val id = model.ui.value.lines.single().id
        val formats = model.exportFormats(id).map { it.ext }
        assertEquals("oscyllib", formats.last())
        for (ext in formats) {
            model.export(id, ext)
            await { "Gig.$ext" in files.saved }
            val back = importFile("Gig.$ext", files.saved.getValue("Gig.$ext"))
            assertEquals(listOf("Lead", "Pad", "Bass"), back.document.rows().map { it.name })
        }
        assertEquals(Notice.Exported("Gig"), model.ui.value.notice)
        model.share(id)
        await { files.shared.isNotEmpty() }
        assertArrayEquals(files.saved.getValue("Gig.${formats.first()}"), files.shared.getValue("Gig.${formats.first()}"))
    }

    @Test
    fun namesWithSlashesStayOneFileName() {
        val files = FakeFiles()
        val model = show(files)
        files.next = PickedFile("Gig.mnlgxdlib", fixture("mnlgxdlib"))
        compose.onNodeWithText("Import file").performClick()
        awaitText("Gig")
        val id = model.ui.value.lines.single().id
        model.renameNode(id, "AC/DC")
        awaitText("AC/DC")
        model.export(id, "oscyllib")
        await { "AC_DC.oscyllib" in files.saved }
        model.renameNode(id, "../../x")
        awaitText("../../x")
        model.share(id)
        await { files.shared.isNotEmpty() }
        assertEquals(setOf(".._.._x.mnlgxdlib"), files.shared.keys)
    }

    @Test
    fun androidShareStaysInTheCache() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        for (name in listOf("AC/DC.oscyllib", "../../x.oscyllib")) {
            AndroidFiles.of(context).share(name, byteArrayOf(1, 2, 3), "application/octet-stream")
            val shared = context.cacheDir.resolve("shared").listFiles()!!.single()
            assertEquals(context.cacheDir.resolve("shared").canonicalPath, shared.parentFile!!.canonicalPath)
            assertEquals(safeFileName(name), shared.name)
        }
    }

    @Test
    fun aBadFileIsANoticeNotACrash() {
        val files = FakeFiles()
        val model = show(files)
        files.next = PickedFile("Broken.mnlgxdlib", "not a library".encodeToByteArray())
        compose.onNodeWithText("Import file").performClick()
        await { model.ui.value.notice != null }
        assertTrue(model.ui.value.notice is Notice.Failed)
        assertTrue(model.ui.value.lines.isEmpty())
    }

    @Test
    fun oldLibrariesMigrateOnFirstOpen() {
        val dir = temp.newFolder("libraries")
        dir.resolve("0123456789abcdef.mnlgxdlib").writeBytes(fixture("mnlgxdlib"))
        dir.resolve("index.json").writeText(
            """{"libraries":[{"id":"0123456789abcdef","name":"Old set","created":1,"changed":1}]}""",
        )
        val model = show(FakeFiles(), dir)
        awaitText("Old set")
        model.select(librarySource("0123456789abcdef"))
        await { model.ui.value.programs != null }
        assertEquals(listOf("Lead", "Pad", "Bass"), model.ui.value.programs!!.rows.map { it.name })
        assertTrue(dir.resolve("before-oscyllib/0123456789abcdef.mnlgxdlib").isFile)
    }

    private companion object {
        const val XD = "korg-minilogue-xd"
        const val TIMEOUT_MS = 10_000L
    }
}
