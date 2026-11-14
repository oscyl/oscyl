package io.github.oscyl.platform

/**
 * Where Oscyl keeps its own files under the app-private folder [root] (Android's
 * files dir, iOS's Application Support): the same layout on every platform.
 */
class AppStorage(root: String) {
    private val base = root.trimEnd('/')
    val libraries = "$base/libraries"
    val backups = "$base/backups"
    val unsynced = "$base/unsynced"
    val settings = "$base/settings.json"
}
