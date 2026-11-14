package io.github.oscyl.i18n

import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import org.w3c.dom.Element

/** Every English string has an Italian and a pseudo-locale twin; plurals cover one and other. */
class StringsCompleteTest {
    private val english = File(checkNotNull(System.getProperty("oscyl.strings")))
    private val italian = checkNotNull(english.parentFile).resolveSibling("values-it/strings.xml")
    private val pseudo = File(checkNotNull(System.getProperty("oscyl.pseudoLocale")), "values-en-rXA/strings.xml")

    /** Key -> plural quantities (empty for plain strings and arrays). */
    private fun keys(file: File): Map<String, Set<String>> {
        val root = DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(file).documentElement
        val children = root.childNodes
        return (0 until children.length).map { children.item(it) }.filterIsInstance<Element>().associate { node ->
            val items = node.getElementsByTagName("item")
            val quantities = (0 until items.length).mapNotNull { (items.item(it) as Element).getAttribute("quantity").ifEmpty { null } }
            node.getAttribute("name") to quantities.toSet()
        }
    }

    @Test
    fun stringsComplete() {
        val base = keys(english)
        assertEquals(base.keys, keys(italian).keys, "values-it has every key")
        assertEquals(base.keys, keys(pseudo).keys, "en-rXA has every key")
        for (file in listOf(english, italian, pseudo)) {
            for ((key, quantities) in keys(file)) {
                if (quantities.isNotEmpty()) {
                    assertTrue(quantities.containsAll(setOf("one", "other")), "$key in ${file.parentFile?.name} has one and other: $quantities")
                }
            }
        }
    }
}
