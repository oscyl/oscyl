package io.github.oscyl.ios

import platform.Foundation.NSApplicationSupportDirectory
import platform.Foundation.NSFileManager
import platform.Foundation.NSLocale
import platform.Foundation.NSUserDomainMask
import platform.Foundation.preferredLanguages

/**
 * The app's private folder for its own files: Application Support, made on first use.
 * Libraries, settings, unsynced edits and backups are the person's data, not caches,
 * so none is excluded from the device backup.
 */
fun appSupportDir(): String {
    val url = NSFileManager.defaultManager.URLForDirectory(
        NSApplicationSupportDirectory,
        NSUserDomainMask,
        appropriateForURL = null,
        create = true,
        error = null,
    )
    return requireNotNull(url?.path) { "no Application Support folder" }
}

/** The person's first preferred language (`it-IT`), as iOS ranks them for this app; null when none. */
fun preferredLanguage(): String? = NSLocale.preferredLanguages.firstOrNull() as? String
