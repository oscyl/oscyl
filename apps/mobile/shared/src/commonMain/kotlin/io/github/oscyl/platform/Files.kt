package io.github.oscyl.platform

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow

/** What a file is picked for: a picked file whose wait went with the process goes back to it. */
enum class PickFor { Library, Unit }

/** A file the person picked: its display name (the core reads the format from it) and contents. */
class PickedFile(val name: String, val bytes: ByteArray)

/** Where an exported file goes, once the person has chosen. */
fun interface Sink {
    /** Write [bytes] there, replacing what it held. Blocks: call it off the main thread. */
    suspend fun write(bytes: ByteArray)
}

/**
 * The platform's files: its own pickers and share sheet (Android's Storage Access
 * Framework, iOS's document picker). Only bytes cross into the core.
 */
interface Files {
    /** Ask for a file to open, for [purpose]; null when the person backs out. */
    suspend fun open(purpose: PickFor = PickFor.Library): PickedFile?

    /** Ask where to save a new file called [name]; null when the person backs out. */
    suspend fun create(name: String, mime: String): Sink?

    /** Offer [bytes] as a file called [name] to another app. */
    fun share(name: String, bytes: ByteArray, mime: String)

    /** Offer a line of [text] to another app (a message, a note, the clipboard). */
    fun shareText(text: String) = Unit

    /**
     * A file picked to open for [purpose] whose wait went with the app's process (the
     * system ended it while the picker was up), read now; null when there is none, or
     * it was picked for something else. Taken once.
     */
    suspend fun orphan(purpose: PickFor): PickedFile? = null

    /**
     * The name of a file to export whose place was chosen after the app's process went:
     * nothing was written there. Taken once.
     */
    suspend fun lostExport(): String? = null

    /**
     * Files another app opened in Oscyl (iOS's "Open in", a tapped attachment), to keep
     * as libraries: each one read when taken, throwing [FileUnreadable] when it can't be.
     */
    val opened: Flow<suspend () -> PickedFile> get() = emptyFlow()
}

/** The platform could not read the picked file called [name]. */
class FileUnreadable(val name: String, cause: Throwable? = null) : Exception(name, cause)

/** The platform could not write the file called [name]. */
class FileUnwritable(val name: String, cause: Throwable? = null) : Exception(name, cause)

/**
 * [name] as one safe file name: no folder separators (they become `_`), so it can
 * never name a path outside where it is written; [fallback] when nothing usable is left.
 */
fun safeFileName(name: String, fallback: String = "library"): String {
    val single = name.map { if (it == '/' || it == '\\' || it.code < 32) '_' else it }.joinToString("").trim()
    return if (single.isEmpty() || single.all { it == '.' }) fallback else single
}

/** No files: previews, tests that don't need them, and iOS until its pickers land (M7). */
object NoFiles : Files {
    override suspend fun open(purpose: PickFor): PickedFile? = null

    override suspend fun create(name: String, mime: String): Sink? = null

    override fun share(name: String, bytes: ByteArray, mime: String) = Unit
}
