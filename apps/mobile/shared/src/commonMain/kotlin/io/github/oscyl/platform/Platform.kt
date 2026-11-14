package io.github.oscyl.platform

/** What the app needs from the device it runs on. */
interface Platform {
    val haptics: OscylHaptics

    /** The device's MIDI, made once for the app's synth worker (which closes it). */
    fun midi(): MidiPlatform

    /** Pickers and the share sheet. */
    val files: Files get() = NoFiles

    /** Where Oscyl's own libraries live (app-private storage); null: nowhere (previews). */
    val librariesDir: String? get() = null

    /** The preferences file (app-private storage); null: none kept (previews). */
    val settingsPath: String? get() = null

    /** Where connected synths' unsynced edits are kept between runs (app-private storage); null: not kept. */
    val unsyncedDir: String? get() = null

    /** Where the backups taken before each sync go (app-private storage); null: nowhere (previews). */
    val backupsDir: String? get() = null

    /** The person's language as a BCP 47 tag, when the platform says it better than Compose's locale (iOS's preferred languages); null: Compose's. */
    val language: String? get() = null
}
