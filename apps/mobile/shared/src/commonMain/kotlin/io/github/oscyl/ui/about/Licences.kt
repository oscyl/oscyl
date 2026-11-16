package io.github.oscyl.ui.about

import androidx.compose.runtime.Immutable
import io.github.oscyl.shared.generated.resources.Res
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.contentOrNull

/**
 * One piece of third-party software in the app, with what its licence says.
 * Names, licences and texts are the authors' own and are never translated.
 */
@Immutable
data class Notice(
    /** Unique with [version]: a crate's name, a Maven coordinate, or the notice's name. */
    val id: String,
    val name: String,
    val version: String?,
    /** An SPDX expression ("MIT OR Apache-2.0"). */
    val licence: String,
    val url: String?,
    val texts: List<String>,
)

/** By name (ignoring case), then id and version: one stable list. */
val NoticeOrder: Comparator<Notice> =
    compareBy<Notice>({ it.name.lowercase() }, { it.id }, { it.version.orEmpty() })

/** UniFFI's source, as its MPL-2.0 asks a recipient to be told. */
const val UniffiSource = "https://github.com/mozilla/uniffi-rs"

/**
 * Every notice, sorted and unique: the Rust crates ([rustJson], from
 * `scripts/licences.sh`), the Kotlin and Android libraries ([kotlinJson], from the
 * AboutLibraries Gradle plugin's export), UniFFI's MPL-2.0 notice, the Albert Sans font and
 * KORG's logue SDK (in the oscillator engine unit Oscyl loads into synths).
 */
fun notices(rustJson: String, kotlinJson: String, albertSansLicence: String, logueSdkLicence: String): List<Notice> {
    val rust = Json.parseToJsonElement(rustJson).jsonObject
    val texts = rust.getValue("licences").jsonArray.map { it.jsonObject }
    val crates = rust.getValue("crates").jsonArray.map { element ->
        val crate = element.jsonObject
        val name = crate.getValue("name").jsonPrimitive.content
        Notice(
            id = name,
            name = name,
            version = crate.getValue("version").jsonPrimitive.content,
            licence = crate.getValue("licence").jsonPrimitive.content,
            url = crate["url"]?.jsonPrimitive?.contentOrNull,
            texts = crate.getValue("texts").jsonArray.map { texts[it.jsonPrimitive.int].getValue("text").jsonPrimitive.content },
        )
    }
    val kotlin = Json.parseToJsonElement(kotlinJson).jsonObject
    val licences = kotlin.getValue("licenses").jsonObject
    val libraries = kotlin.getValue("libraries").jsonArray.map { element ->
        val library = element.jsonObject
        val keys = library["licenses"]?.jsonArray?.map { it.jsonPrimitive.content }.orEmpty()
        val used = keys.mapNotNull { licences[it]?.jsonObject }
        Notice(
            id = library.getValue("uniqueId").jsonPrimitive.content,
            name = library.getValue("name").jsonPrimitive.content,
            version = library["artifactVersion"]?.jsonPrimitive?.contentOrNull,
            licence = used.zip(keys) { l, key -> l["spdxId"]?.jsonPrimitive?.contentOrNull ?: key }.joinToString(" OR "),
            url = library["website"]?.jsonPrimitive?.contentOrNull,
            texts = used.mapNotNull { it["content"]?.jsonPrimitive?.contentOrNull },
        )
    }
    val mpl = texts.first { it.getValue("id").jsonPrimitive.content == "MPL-2.0" }.getValue("text").jsonPrimitive.content
    val own = listOf(
        Notice("UniFFI", "UniFFI", version = null, licence = "MPL-2.0", url = UniffiSource, texts = listOf(mpl)),
        Notice("Albert Sans", "Albert Sans", version = null, licence = "OFL-1.1", url = "https://github.com/usted/Albert-Sans", texts = listOf(albertSansLicence)),
        Notice("logue SDK", "logue SDK", version = null, licence = "BSD-3-Clause", url = "https://github.com/korginc/logue-sdk", texts = listOf(logueSdkLicence)),
    )
    return (own + crates + libraries).distinctBy { it.id to it.version }.sortedWith(NoticeOrder)
}

private val noticesLock = Mutex()
private var noticesCache: List<Notice>? = null

/** The notices from the files the app ships (Compose resources), parsed once, off the main thread. */
suspend fun loadNotices(): List<Notice> = noticesLock.withLock {
    noticesCache ?: withContext(Dispatchers.Default) {
        notices(
            rustJson = Res.readBytes("files/licences/rust.json").decodeToString(),
            kotlinJson = Res.readBytes("files/aboutlibraries.json").decodeToString(),
            albertSansLicence = Res.readBytes("files/licences/OFL-AlbertSans.txt").decodeToString(),
            logueSdkLicence = Res.readBytes("files/licences/BSD-3-Clause-logue-sdk.txt").decodeToString(),
        )
    }.also { noticesCache = it }
}

private var ownCache: List<String>? = null

/** Oscyl's own licence texts: MIT and Apache 2.0, either at the user's option; read once. */
suspend fun loadOwnLicence(): List<String> = noticesLock.withLock {
    ownCache ?: withContext(Dispatchers.Default) {
        listOf(
            Res.readBytes("files/licences/LICENSE-MIT").decodeToString(),
            Res.readBytes("files/licences/LICENSE-APACHE").decodeToString(),
        )
    }.also { ownCache = it }
}

/**
 * Everything the About and licence pages show, read and parsed off the main thread:
 * the app asks for it before it opens either page, so each page is whole on its first frame.
 */
suspend fun preloadLicences() {
    loadNotices()
    loadOwnLicence()
}

/** The notices, if they are loaded: what the pages start from (complete once preloaded). */
fun loadedNotices(): List<Notice>? = noticesCache

/** Oscyl's own licence texts, if they are loaded. */
fun loadedOwnLicence(): List<String>? = ownCache
