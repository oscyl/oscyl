package io.github.oscyl.android.files

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.ComponentActivity
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.FileProvider
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import io.github.oscyl.platform.FileUnreadable
import io.github.oscyl.platform.FileUnwritable
import io.github.oscyl.platform.Files
import io.github.oscyl.platform.PickFor
import io.github.oscyl.platform.PickedFile
import io.github.oscyl.platform.Sink
import io.github.oscyl.platform.safeFileName
import java.io.IOException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Files through the Storage Access Framework (open and create document) and the share
 * sheet. One per process: each activity [attach]es its result launchers as it starts,
 * and a pick answered after a rotation completes the same wait, so the library model
 * (which outlives the activity) gets its file.
 */
class AndroidFiles private constructor(private val context: Context) : Files {
    private var opener: ActivityResultLauncher<Array<String>>? = null
    private var creator: ActivityResultLauncher<String>? = null
    private var opening: CompletableDeferred<Uri?>? = null
    private var creating: CompletableDeferred<Uri?>? = null

    /** Register [activity]'s pickers (in its onCreate); they go when it is destroyed. */
    fun attach(activity: ComponentActivity) {
        val open = activity.registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
            // No one waits: the process went while the picker was up. Kept for whoever asked for it.
            val wait = opening ?: return@registerForActivityResult run {
                orphaned = uri
                orphanedFor = asked.getString(OPEN, null)
            }
            wait.complete(uri)
            opening = null
        }
        val create = activity.registerForActivityResult(ActivityResultContracts.CreateDocument(MIME)) { uri ->
            val wait = creating
            // No one waits: the bytes went with the process. Said, not dropped.
            if (wait == null) {
                if (uri != null) lost = asked.getString(CREATE, null)
            } else {
                wait.complete(uri)
            }
            creating = null
        }
        opener = open
        creator = create
        activity.lifecycle.addObserver(object : DefaultLifecycleObserver {
            override fun onDestroy(owner: LifecycleOwner) {
                if (opener === open) opener = null
                if (creator === create) creator = null
            }
        })
    }

    /** What each picker was last asked for, kept on disk: the process may go before it answers. */
    private val asked = context.getSharedPreferences("oscyl-picks", Context.MODE_PRIVATE)

    /** A file picked to open after the process went, not taken yet, and what it was for. */
    private var orphaned: Uri? = null
    private var orphanedFor: String? = null

    /** A file to export whose place was chosen after the process went. */
    private var lost: String? = null

    override suspend fun orphan(purpose: PickFor): PickedFile? {
        val uri = withContext(Dispatchers.Main) {
            orphaned?.takeIf { (orphanedFor ?: PickFor.Library.name) == purpose.name }.also { if (it != null) orphaned = null }
        } ?: return null
        return read(uri)
    }

    override suspend fun lostExport(): String? = withContext(Dispatchers.Main) { lost.also { lost = null } }

    override suspend fun open(purpose: PickFor): PickedFile? {
        val uri = pick { wait ->
            asked.edit().putString(OPEN, purpose.name).commit()
            opening = wait
            opener?.launch(arrayOf("*/*")) ?: wait.complete(null)
        } ?: return null
        return read(uri)
    }

    private suspend fun read(uri: Uri): PickedFile =
        withContext(Dispatchers.IO) {
            val resolver = context.contentResolver
            val name = resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
                if (c.moveToFirst()) c.getString(0) else null
            } ?: uri.lastPathSegment.orEmpty()
            val bytes = try {
                resolver.openInputStream(uri)?.use { it.readBytes() }
            } catch (e: IOException) {
                throw FileUnreadable(name, e)
            } catch (e: SecurityException) {
                throw FileUnreadable(name, e)
            } ?: throw FileUnreadable(name)
            PickedFile(name, bytes)
        }

    override suspend fun create(name: String, mime: String): Sink? {
        val uri = pick { wait ->
            asked.edit().putString(CREATE, name).commit()
            creating = wait
            creator?.launch(name) ?: wait.complete(null)
        } ?: return null
        return Sink { bytes ->
            withContext(Dispatchers.IO) {
                try {
                    context.contentResolver.openOutputStream(uri, "wt")?.use { it.write(bytes) }
                } catch (e: IOException) {
                    throw FileUnwritable(name, e)
                } catch (e: SecurityException) {
                    throw FileUnwritable(name, e)
                } ?: throw FileUnwritable(name)
            }
        }
    }

    /** Launch a picker on the main thread and wait for its answer (null: backed out, or no activity). */
    private suspend fun pick(launch: (CompletableDeferred<Uri?>) -> Unit): Uri? {
        val wait = CompletableDeferred<Uri?>()
        withContext(Dispatchers.Main) {
            try {
                launch(wait)
            } catch (e: ActivityNotFoundException) {
                wait.complete(null)
            }
        }
        return wait.await()
    }

    override fun share(name: String, bytes: ByteArray, mime: String) {
        // Only the latest shared file is kept, under the app's cache.
        val dir = context.cacheDir.resolve(SHARED).apply { deleteRecursively(); mkdirs() }
        // One path segment, whatever the name holds ("AC/DC", "../../x"): it stays in the cache.
        val file = dir.resolve(safeFileName(name)).apply { writeBytes(bytes) }
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.files", file)
        val send = Intent(Intent.ACTION_SEND)
            .setType(mime)
            .putExtra(Intent.EXTRA_STREAM, uri)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        context.startActivity(Intent.createChooser(send, null).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    companion object {
        /** Library files have no registered type of their own. */
        private const val MIME = "application/octet-stream"
        private const val SHARED = "shared"
        private const val OPEN = "open"
        private const val CREATE = "create"

        @Volatile private var instance: AndroidFiles? = null

        /** The process's one instance (a new application, as each test has, gets its own). */
        fun of(context: Context): AndroidFiles {
            val app = context.applicationContext
            return instance?.takeIf { it.context === app }
                ?: synchronized(this) { instance?.takeIf { it.context === app } ?: AndroidFiles(app).also { instance = it } }
        }
    }
}
