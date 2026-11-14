package io.github.oscyl.library

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.oscyl.core.Libraries
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.library.librarySource
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicInteger
import kotlin.coroutines.CoroutineContext
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The program menu reads the libraries' outline: it is asked on the folder's thread, never the main one. */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w360dp-h800dp-xxhdpi")
class ProgramMenuThreadTest {
    @get:Rule val compose = createComposeRule()

    @get:Rule val temp = TemporaryFolder()

    private val store = ViewModelStore()
    private val thread = Executors.newSingleThreadExecutor { Thread(it, "library-io") }

    @After
    fun clear() {
        store.clear()
        thread.shutdown()
    }

    /** The folder's thread, counting the work it is given and checking it is never the main thread. */
    private class Folder(private val inner: CoroutineDispatcher, private val main: Thread) : CoroutineDispatcher() {
        val dispatched = AtomicInteger()
        val offMain = AtomicInteger()

        override fun dispatch(context: CoroutineContext, block: Runnable) {
            dispatched.incrementAndGet()
            inner.dispatch(context) {
                if (Thread.currentThread() !== main) offMain.incrementAndGet()
                block.run()
            }
        }
    }

    @Test
    fun theMenuIsAskedOffTheMainThread() {
        val dir = temp.newFolder("libraries").path
        val id = Libraries.open(dir).use { l -> l.create("Gig", "korg-minilogue-xd", null).also { l.read(it).use { d -> d.newProgram(null); l.save(it, d) } } }
        val folder = Folder(thread.asCoroutineDispatcher(), Thread.currentThread())
        val model = ViewModelProvider.create(store, viewModelFactory { initializer { LibraryModel(dir, io = folder) } })[LibraryModel::class]
        compose.setContent {}
        compose.waitUntil(10_000) { compose.runOnIdle { model.ui.value.lines.any { it.id == id } } }
        compose.runOnIdle { model.select(librarySource(id)) }
        compose.waitUntil(10_000) { compose.runOnIdle { model.ui.value.programs?.rows?.isNotEmpty() == true } }
        val before = folder.dispatched.get()
        val menu = runBlocking { model.menu(0) }
        assertNotNull(menu)
        assertTrue("asked on the folder's thread", folder.dispatched.get() > before)
        assertTrue("never on the main thread", folder.offMain.get() == folder.dispatched.get())
    }
}
