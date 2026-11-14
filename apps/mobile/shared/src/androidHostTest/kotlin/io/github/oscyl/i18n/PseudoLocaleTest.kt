package io.github.oscyl.i18n

import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class PseudoLocaleTest {
    private fun strings(file: File): Map<String, String> {
        val nodes = DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(file)
            .getElementsByTagName("string")
        return (0 until nodes.length).associate { i ->
            val node = nodes.item(i)
            node.attributes.getNamedItem("name").nodeValue to node.textContent
        }
    }

    /** Each plural's items, by the plural's name and the item's quantity. */
    private fun plurals(file: File): Map<String, String> {
        val nodes = DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(file)
            .getElementsByTagName("plurals")
        return (0 until nodes.length).flatMap { i ->
            val plural = nodes.item(i) as org.w3c.dom.Element
            val items = plural.getElementsByTagName("item")
            (0 until items.length).map { j ->
                val item = items.item(j) as org.w3c.dom.Element
                "${plural.getAttribute("name")}/${item.getAttribute("quantity")}" to item.textContent
            }
        }.toMap()
    }

    private val englishFile = File(checkNotNull(System.getProperty("oscyl.strings")))
    private val pseudoFile = File(checkNotNull(System.getProperty("oscyl.pseudoLocale")), "values-en-rXA/strings.xml")
    private val english = strings(englishFile)
    private val pseudo = strings(pseudoFile)

    @Test
    fun everyKeyIsPseudoLocalised() {
        assertEquals(english.keys, pseudo.keys)
    }

    @Test
    fun pseudoStringsAreBracketedAccentedAndLonger() {
        for ((key, text) in pseudo) {
            assertTrue(text.startsWith("[") && text.endsWith("]"), "$key is bracketed: $text")
            assertTrue(text.length >= english.getValue(key).length * 1.4, "$key is ~40% longer: $text")
        }
        assertEquals("[Öšçýļ on]", pseudo["app_name"])
    }

    @Test
    fun pluralsArePseudoLocalisedItemByItem() {
        val english = plurals(englishFile)
        val pseudo = plurals(pseudoFile)
        assertTrue(english.isNotEmpty(), "the app has plurals")
        assertEquals(english.keys, pseudo.keys)
        for ((key, text) in pseudo) {
            assertTrue(text.startsWith("[") && text.endsWith("]"), "$key is bracketed: $text")
            // The count's argument survives untouched.
            Regex("%\\d+\\$[sd]").findAll(english.getValue(key)).forEach { assertTrue(it.value in text, "$key keeps ${it.value}: $text") }
        }
    }
}
