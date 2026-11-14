package io.github.oscyl.ui.about

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/** The About page's notices, built from the files the app ships. */
class LicencesTest {
    private val files = File(checkNotNull(System.getProperty("oscyl.composeResources")), "files")
    private val notices = notices(
        rustJson = files.resolve("licences/rust.json").readText(),
        kotlinJson = files.resolve("aboutlibraries.json").readText(),
        albertSansLicence = files.resolve("licences/OFL-AlbertSans.txt").readText(),
    )

    @Test
    fun licencesIncludeUniffiAndAlbertSans() {
        val uniffi = notices.single { it.name == "UniFFI" }
        assertEquals("MPL-2.0", uniffi.licence)
        assertEquals("https://github.com/mozilla/uniffi-rs", uniffi.url)
        assertTrue(uniffi.texts.single().contains("Mozilla Public License"))
        val albert = notices.single { it.name == "Albert Sans" }
        assertEquals("OFL-1.1", albert.licence)
        assertTrue(albert.texts.single().contains("SIL Open Font License"))
        // Both sides: a Rust crate and a Kotlin library, each with its text.
        assertTrue(notices.any { it.name == "uniffi_core" && it.texts.isNotEmpty() })
        assertTrue(notices.any { it.id == "androidx.activity:activity" && it.texts.isNotEmpty() })
    }

    @Test
    fun licencesSortedAndUnique() {
        assertEquals(notices.sortedWith(NoticeOrder), notices)
        assertEquals(notices.size, notices.distinctBy { it.id to it.version }.size)
        assertTrue(notices.all { it.texts.isNotEmpty() && it.licence.isNotBlank() }, "every notice has a licence and its text")
    }
}
