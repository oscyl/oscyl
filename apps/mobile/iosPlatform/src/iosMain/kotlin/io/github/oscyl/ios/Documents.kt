package io.github.oscyl.ios

import platform.darwin.dispatch_get_main_queue
import platform.darwin.dispatch_async
import platform.Foundation.NSFileCoordinatorWritingForReplacing
import platform.Foundation.NSFileCoordinator
import platform.Foundation.NSError
import platform.CoreGraphics.CGRectMake
import kotlinx.cinterop.useContents
import kotlinx.cinterop.value
import kotlinx.cinterop.ptr
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.alloc
import kotlinx.cinterop.ObjCObjectVar
import kotlin.coroutines.resume
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.usePinned
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import platform.Foundation.NSData
import platform.Foundation.NSFileManager
import platform.Foundation.NSTemporaryDirectory
import platform.Foundation.NSURL
import platform.Foundation.create
import platform.Foundation.dataWithContentsOfURL
import platform.Foundation.lastPathComponent
import platform.Foundation.writeToURL
import platform.UIKit.UIActivityViewController
import platform.UIKit.UIApplication
import platform.UIKit.UIDocumentPickerDelegateProtocol
import platform.UIKit.UIDocumentPickerViewController
import platform.UIKit.UIViewController
import platform.UIKit.UIWindowScene
import platform.UIKit.popoverPresentationController
import platform.UniformTypeIdentifiers.UTTypeData
import platform.darwin.NSObject
import platform.posix.memcpy

/** A file the person picked: its name and contents. */
class PickedDocument(val name: String, val bytes: ByteArray)

/** The picked file called [name] could not be read (not downloaded, a provider error). */
class UnreadableDocument(val name: String) : Exception(name)

/**
 * The document picker and the share sheet, presented over the app's topmost view
 * controller. Every call is on the main thread; reading and writing go to the IO
 * dispatcher.
 */
class Documents {
    /** Keeps the picker's delegate alive while it is up (UIKit holds it weakly). */
    private var delegate: PickerDelegate? = null

    /**
     * Ask for a file to open (any type: the core reads the format); null when the person
     * backs out. Throws [UnreadableDocument] when the picked file can't be read.
     */
    suspend fun open(): PickedDocument? {
        val url = pick(UIDocumentPickerViewController(forOpeningContentTypes = listOf(UTTypeData), asCopy = true)) ?: return null
        return withContext(Dispatchers.Default) {
            try {
                read(url) ?: throw UnreadableDocument(url.lastPathComponent ?: "file")
            } finally {
                // The picker's copy (in tmp/…-Inbox) is read now: let go of it.
                NSFileManager.defaultManager.removeItemAtURL(url, error = null)
            }
        }
    }

    /**
     * Ask where to save a new file called [name]: the picker moves an empty file
     * there and gives back its place, which [write] fills. Null when the person backs out.
     */
    suspend fun create(name: String): NSURL? {
        val empty = NSURL.fileURLWithPath(NSTemporaryDirectory() + name)
        if (!NSData().writeToURL(empty, atomically = true)) return null
        val place = pick(UIDocumentPickerViewController(forExportingURLs = listOf(empty), asCopy = false))
        // Moved there when chosen; left in tmp when the person backed out.
        if (place == null) NSFileManager.defaultManager.removeItemAtURL(empty, error = null)
        return place
    }

    /** Write [bytes] at [url] (a place [create] returned), replacing what it held; false when it can't. */
    suspend fun write(url: NSURL, bytes: ByteArray): Boolean = withContext(Dispatchers.Default) {
        val scoped = url.startAccessingSecurityScopedResource()
        try {
            // Coordinated, for file providers (iCloud Drive and others), and in place: the
            // scope covers the file, not its folder, so no temporary sibling and rename.
            var written = false
            val error = memScoped {
                val e = alloc<ObjCObjectVar<NSError?>>()
                NSFileCoordinator(filePresenter = null).coordinateWritingItemAtURL(url, NSFileCoordinatorWritingForReplacing, e.ptr) { at ->
                    written = at != null && dataOf(bytes).writeToURL(at, atomically = false)
                }
                e.value
            }
            error == null && written
        } finally {
            if (scoped) url.stopAccessingSecurityScopedResource()
        }
    }

    /**
     * A file another app opened in Oscyl ([url], copied into the app's Inbox): read
     * it, then let go of the copy. Throws [UnreadableDocument] when it can't be read.
     */
    suspend fun opened(url: NSURL): PickedDocument = withContext(Dispatchers.Default) {
        try {
            read(url) ?: throw UnreadableDocument(url.lastPathComponent ?: "file")
        } finally {
            NSFileManager.defaultManager.removeItemAtURL(url, error = null)
        }
    }

    /** Offer [bytes] as a file called [name] (one safe file name) to another app (only the latest is kept, in the temporary folder). */
    fun share(name: String, bytes: ByteArray) {
        val dir = NSTemporaryDirectory() + "shared/"
        NSFileManager.defaultManager.removeItemAtPath(dir, error = null)
        NSFileManager.defaultManager.createDirectoryAtPath(dir, withIntermediateDirectories = true, attributes = null, error = null)
        val url = NSURL.fileURLWithPath(dir + name)
        if (!dataOf(bytes).writeToURL(url, atomically = true)) return
        offer(url)
    }

    /** Offer a line of text to another app. */
    fun shareText(text: String) = offer(text)

    /** The share sheet for [item] (a file's URL, a text), over the page shown. */
    private fun offer(item: Any) {
        val top = topController() ?: return
        val sheet = UIActivityViewController(activityItems = listOf(item), applicationActivities = null)
        // On iPad the sheet is a popover: centred over the page, with no arrow.
        sheet.popoverPresentationController?.let { popover ->
            popover.sourceView = top.view
            popover.sourceRect = top.view.bounds.useContents { CGRectMake(origin.x + size.width / 2, origin.y + size.height / 2, 0.0, 0.0) }
            popover.permittedArrowDirections = 0u
        }
        top.presentViewController(sheet, animated = true, completion = null)
    }

    private suspend fun pick(picker: UIDocumentPickerViewController): NSURL? {
        val top = topController() ?: return null
        return suspendCancellableCoroutine { done ->
            val d = PickerDelegate { url ->
                delegate = null
                if (done.isActive) done.resume(url)
            }
            delegate = d
            picker.delegate = d
            picker.allowsMultipleSelection = false
            done.invokeOnCancellation {
                // UIKit, on the main thread only (cancellation may come from another).
                dispatch_async(dispatch_get_main_queue()) { picker.dismissViewControllerAnimated(true, completion = null) }
            }
            top.presentViewController(picker, animated = true, completion = null)
        }
    }
}

private class PickerDelegate(private val done: (NSURL?) -> Unit) : NSObject(), UIDocumentPickerDelegateProtocol {
    override fun documentPicker(controller: UIDocumentPickerViewController, didPickDocumentsAtURLs: List<*>) =
        done(didPickDocumentsAtURLs.firstOrNull() as? NSURL)

    override fun documentPickerWasCancelled(controller: UIDocumentPickerViewController) = done(null)
}

/** The app's topmost view controller, to present over. */
private fun topController(): UIViewController? {
    val scene = UIApplication.sharedApplication.connectedScenes.firstOrNull { it is UIWindowScene } as? UIWindowScene
    var top = scene?.keyWindow?.rootViewController ?: return null
    while (true) top = top.presentedViewController ?: return top
}

private fun read(url: NSURL): PickedDocument? {
    val scoped = url.startAccessingSecurityScopedResource()
    try {
        val data = NSData.dataWithContentsOfURL(url) ?: return null
        val bytes = ByteArray(data.length.toInt())
        if (bytes.isNotEmpty()) bytes.usePinned { memcpy(it.addressOf(0), data.bytes, data.length) }
        return PickedDocument(url.lastPathComponent ?: "file", bytes)
    } finally {
        if (scoped) url.stopAccessingSecurityScopedResource()
    }
}

private fun dataOf(bytes: ByteArray): NSData =
    if (bytes.isEmpty()) NSData() else bytes.usePinned { NSData.create(bytes = it.addressOf(0), length = bytes.size.toULong()) }
