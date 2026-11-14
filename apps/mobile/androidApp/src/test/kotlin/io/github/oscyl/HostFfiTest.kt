package io.github.oscyl

import io.github.oscyl.core.Audition
import io.github.oscyl.core.ControlKind
import io.github.oscyl.core.Libraries
import io.github.oscyl.core.ReorderHow
import io.github.oscyl.core.drivers
import io.github.oscyl.core.exportFile
import io.github.oscyl.core.exportFormats
import io.github.oscyl.core.importFile
import io.github.oscyl.core.setLocale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

/** The Rust core loads on the JVM, so unit tests can use the real bindings. */
class HostFfiTest {
    @get:Rule val temp = TemporaryFolder()

    @Test
    fun rustCoreListsDrivers() {
        val all = drivers()
        assertTrue(all.isNotEmpty())
        // What the synth picker shows, from the driver registry.
        val xd = all.single { it.id == XD }
        assertEquals("4 voices", xd.voices)
        assertEquals(2.toUByte(), xd.tier)
        assertTrue(all.all { it.panel.knobs.isNotEmpty() })
        assertEquals(xd.panel, drivers().single { it.id == XD }.panel)
    }

    @Test
    fun librariesAndDocumentsRoundTrip() {
        val dir = temp.newFolder("libraries").path
        val libraries = Libraries.open(dir)
        val id = libraries.create(libraries.newLibraryName(), XD, null)
        libraries.rename(id, "Gig set")

        val doc = libraries.read(id)
        assertEquals(0u, doc.newProgram(null))
        assertEquals(1u, doc.newProgram(0u))
        doc.setName(0u, "Lead")
        doc.setName(1u, "Pad")
        assertEquals(2u, doc.reorder(0u, 1u, ReorderHow.MOVE))
        assertEquals(listOf("Pad", "Lead"), doc.rows().map { it.name })
        doc.undo()
        assertEquals(listOf("Lead", "Pad"), doc.rows().map { it.name })
        doc.redo()
        assertFalse(doc.canRedo())
        libraries.save(id, doc)

        val again = Libraries.open(dir)
        assertEquals(listOf("Gig set"), again.outline().map { it.name })
        val reread = again.read(id)
        assertEquals(listOf("Pad", "Lead"), reread.rows().map { it.name })

        val formats = exportFormats(XD).map { it.ext }
        assertTrue("oscyllib" in formats && formats.size > 1)
        for (ext in listOf("oscyllib", formats.first())) {
            val imported = importFile("Gig set.$ext", exportFile(reread, ext))
            assertEquals(XD, imported.driverId)
            assertEquals("Gig set", imported.name)
            assertEquals(reread.rows(), imported.document.rows())
        }
    }

    /** Every synth's program edits through the editor model: a dial, a handle, undo and the review. */
    @Test
    fun editorEditsEverySynth() {
        val libraries = Libraries.open(temp.newFolder("editor").path)
        for (driver in listOf(XD, "korg-prologue", "sequential-prophet-rev2")) {
            val doc = libraries.read(libraries.create(driver, driver, null))
            assertEquals(0u, doc.newProgram(null))
            val editor = doc.editor(0u)
            val pages = editor.pages()
            assertTrue(driver, pages.isNotEmpty())
            val dial = pages.indices.flatMap { editor.fields(it.toUInt()) }
                .first { it.relevant && (it.control as? ControlKind.Dial)?.let { d -> d.max > d.min + 2u } == true }
            val range = dial.control as ControlKind.Dial
            val to = if (dial.raw == range.max) range.min else range.max
            assertTrue(driver, editor.set(dial.field, to) is Audition)
            assertTrue(driver, editor.field(dial.field).edited)
            assertEquals(listOf(dial.field), doc.review().single().changes.mapNotNull { it.field })
            editor.seal()
            doc.undo()
            assertFalse(driver, editor.field(dial.field).edited)
            // A handle's drag names fields of its own page.
            val page = pages.indices.firstOrNull { editor.handles(it.toUInt()).isNotEmpty() } ?: continue
            val handle = editor.handles(page.toUInt()).first()
            val moved = editor.drag(page.toUInt(), 0u, 1f - handle.x, 1f - handle.y)
            val own = pages[page].sections.flatMap { it.fields }
            assertTrue(driver, moved.isNotEmpty() && moved.all { it.field in own })
        }
    }

    /** With Italian chosen, driver words and core refusals come in Italian; English again after. */
    @Test
    fun coreStringsFollowTheLocale() {
        try {
            setLocale("it-IT")
            val libraries = Libraries.open(temp.newFolder("it").path)
            val doc = libraries.read(libraries.create("Gig", XD, null))
            doc.newProgram(null)
            val titles = doc.editor(0u).pages().map { it.title }
            assertTrue(titles.toString(), "Filtro" in titles)
            val rev2 = io.github.oscyl.core.Document.synth("sequential-prophet-rev2", "Rev2")
            val factory = rev2.rows().indexOfFirst { it.readOnly }.toUInt()
            val refused = runCatching { rev2.setName(factory, "Mine") }.exceptionOrNull() as io.github.oscyl.core.FfiException.Refused
            assertEquals("Questo slot è di sola lettura (un banco di fabbrica).", refused.reason)
        } finally {
            setLocale("en")
        }
    }

    private companion object {
        const val XD = "korg-minilogue-xd"
    }
}
