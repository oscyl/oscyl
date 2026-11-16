package io.github.oscyl.ui.review

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.oscyl.core.FfiException
import io.github.oscyl.core.ReviewProgram
import io.github.oscyl.core.writeBackup
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.ui.library.ShownSource
import io.github.oscyl.ui.library.reason
import io.github.oscyl.ui.library.synthSource
import io.github.oscyl.ui.synth.SynthModel
import io.github.oscyl.ui.synth.WriteState
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.mapNotNull
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** Where keeping the edits stands. */
sealed interface ReviewState {
    data object Idle : ReviewState

    /** Taking the backup of what the synth's slots hold before a sync. */
    data object BackingUp : ReviewState
    data class Writing(val done: Int, val total: Int) : ReviewState

    /** Nothing was kept: why, in the core's or the system's words. */
    sealed interface Failed : ReviewState { val reason: String }
    data class BackupFailed(override val reason: String) : Failed
    data class SyncFailed(override val reason: String) : Failed
    data class SaveFailed(override val reason: String) : Failed

    /** The sync cannot start yet (the synth is still being read). */
    data class NotYet(override val reason: String) : Failed
}

/** The review sheet: every edited program of the source shown, with its changes. */
data class ReviewUi(
    val open: Boolean = false,
    /** The source is a synth's slots: keeping the edits syncs them. */
    val sync: Boolean = false,
    val programs: List<ReviewProgram> = emptyList(),
    val state: ReviewState = ReviewState.Idle,
) {
    /** A save or a sync is under way: the action waits. */
    val busy: Boolean get() = state is ReviewState.BackingUp || state is ReviewState.Writing
}

/** What the review sheet can ask for; every default does nothing (previews). */
interface ReviewActions {
    fun open() {}
    fun close() {}
    fun revert(index: UInt) {}
    fun commit() {}
}

/** Does nothing: previews. */
object NoReviewActions : ReviewActions

/**
 * The edits of the source shown, under review: Save keeps a library's (written, then
 * marked saved; a failed write keeps them marked), Sync writes a synth's edited slots
 * the desktop way: every edited slot read, a backup of what they hold now, then the
 * write; only what the synth stored is marked synced.
 */
class ReviewModel(
    private val library: LibraryModel,
    private val synth: SynthModel,
    private val backupsDir: String?,
) : ViewModel(), ReviewActions {
    private val _ui = MutableStateFlow(ReviewUi())
    val ui: StateFlow<ReviewUi> = _ui.asStateFlow()
    private var shown: ShownSource? = null
    private var sync: Job? = null

    init {
        viewModelScope.launch {
            library.shown.collect { source ->
                shown = source
                refresh()
            }
        }
    }

    /** Ask the core again for the changes in words (also after a language change). */
    internal fun refresh() {
        val source = shown
        // A synth's slots close when it goes; until the library shows that, nothing to review.
        val programs = try {
            source?.document?.review().orEmpty()
        } catch (_: IllegalStateException) {
            emptyList()
        }
        _ui.update {
            it.copy(
                sync = source?.inLibrary == false,
                programs = programs,
                // Nothing left to keep: the sheet folds away.
                open = it.open && (programs.isNotEmpty() || it.busy),
            )
        }
    }

    override fun open() {
        shown = library.shown.value
        if (shown == null) return
        _ui.update { it.copy(open = true, state = if (it.busy) it.state else ReviewState.Idle) }
        refresh()
    }

    override fun close() = _ui.update { it.copy(open = false) }

    /** Open the review of [source] once the library shows it (a synth's card asks). */
    fun openFor(source: String) {
        waiting?.cancel()
        waiting = viewModelScope.launch {
            // Or not at all: the synth went before its slots showed.
            val gone = { cards: List<io.github.oscyl.ui.synth.SynthCard> -> cards.none { synthSource(it.id) == source && synth.document(it.id) != null } }
            val shown = combine(library.shown, synth.cards) { shown, cards -> (shown?.source == source) to gone(cards) }
                .first { (shown, gone) -> shown || gone }
            if (shown.first) open()
        }
    }

    /** A review waiting for its source to show. */
    private var waiting: kotlinx.coroutines.Job? = null

    override fun revert(index: UInt) = library.revert(index.toInt())

    /**
     * The Save key's tap: a library's edits are written at once and a notice says so (a
     * failure opens the review with its reason); a synth's slots open the review, since
     * a sync overwrites the synth. With [reviewHint] the notice adds that a hold reviews first.
     */
    fun quickSave(reviewHint: Boolean = false) {
        // The source the library shows now (the Save key may just have taken it there).
        shown = library.shown.value
        val source = shown ?: return
        if (!source.inLibrary) return open()
        if (_ui.value.busy) return
        // From the source itself, not the sheet's last list (which may still be another's).
        val count = source.document.review().size
        // Under way: the Save key runs its scale, and a second tap waits.
        _ui.update { it.copy(state = ReviewState.Writing(0, count)) }
        viewModelScope.launch {
            val failure = library.keepEdits()
            if (failure != null) {
                _ui.update { it.copy(open = true, state = ReviewState.SaveFailed(failure)) }
                refresh()
            } else {
                _ui.update { it.copy(state = ReviewState.Idle) }
                library.say(io.github.oscyl.ui.library.Notice.Saved(count, reviewHint))
            }
        }
    }

    override fun commit() {
        val source = shown ?: return
        if (_ui.value.busy) return
        if (source.inLibrary) {
            viewModelScope.launch {
                val failure = library.keepEdits()
                _ui.update { it.copy(state = failure?.let(ReviewState::SaveFailed) ?: ReviewState.Idle) }
            }
            return
        }
        val id = source.synthId ?: return
        val doc = source.document
        val plan = try {
            doc.syncPlan()
        } catch (e: FfiException) {
            return _ui.update { it.copy(state = ReviewState.NotYet(e.reason())) }
        }
        sync = viewModelScope.launch {
            _ui.update { it.copy(state = ReviewState.BackingUp) }
            // No backup, nothing written.
            val dir = backupsDir ?: return@launch _ui.update { it.copy(state = ReviewState.BackupFailed("")) }
            try {
                withContext(Dispatchers.IO + NonCancellable) { writeBackup(dir, doc, plan.indices) }
            } catch (e: FfiException) {
                return@launch _ui.update { it.copy(state = ReviewState.BackupFailed(e.reason())) }
            }
            synth.write(id, plan.programs)
            val end = synth.writes.mapNotNull { it[id] }.first { state ->
                if (state is WriteState.Writing) _ui.update { it.copy(state = ReviewState.Writing(state.done, state.total)) }
                state !is WriteState.Writing
            }
            _ui.update { it.copy(state = if (end is WriteState.Failed) ReviewState.SyncFailed(end.reason) else ReviewState.Idle) }
            refresh()
        }
    }
}
