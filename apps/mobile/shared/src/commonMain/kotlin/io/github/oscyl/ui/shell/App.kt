package io.github.oscyl.ui.shell

import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.graphics.CompositingStrategy
import io.github.oscyl.ui.preferences.LocalTeaching
import io.github.oscyl.ui.review.ReviewState
import io.github.oscyl.ui.preferences.Teaching
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.WindowInsets
import io.github.oscyl.shared.generated.resources.notice_retry
import io.github.oscyl.ui.library.NoticeHost
import io.github.oscyl.ui.library.Notice
import io.github.oscyl.ui.play.PlayScreen
import io.github.oscyl.ui.play.PlayModel
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.history_title
import io.github.oscyl.shared.generated.resources.sheet_title
import io.github.oscyl.ui.editor.HistoryList
import kotlinx.coroutines.flow.first
import io.github.oscyl.ui.synth.Live
import io.github.oscyl.ui.synth.SynthPicker
import io.github.oscyl.ui.parts.DockedSheet
import io.github.oscyl.ui.parts.safeInsets
import androidx.compose.ui.unit.Dp
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.animation.ContentTransform
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.IntOffset
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.snap
import io.github.oscyl.platform.imeHeading
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.graphicsLayer
import io.github.oscyl.ui.parts.LocalSheetLayer
import io.github.oscyl.ui.parts.SheetLayer
import io.github.oscyl.ui.parts.SheetLayerHost
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfoV2
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.listSaver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshots.SnapshotStateList
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.intl.Locale
import androidx.compose.ui.text.toUpperCase
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation3.runtime.NavEntry
import androidx.navigation3.runtime.NavKey
import androidx.navigation3.ui.NavDisplay
import androidx.window.core.layout.WindowSizeClass
import io.github.oscyl.platform.LocalOscylHaptics
import io.github.oscyl.platform.Platform
import io.github.oscyl.platform.testTagsAsIds
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.about.AboutScreen
import io.github.oscyl.ui.about.preloadLicences
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch
import io.github.oscyl.ui.about.LicenceScreen
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.OscylSizePreviews
import io.github.oscyl.ui.preview.ThemeModes
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.oscyl.ui.editor.EditorDestination
import io.github.oscyl.ui.editor.EditorModel
import io.github.oscyl.ui.library.LibraryModel
import io.github.oscyl.core.setLocale
import io.github.oscyl.platform.GatedHaptics
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.LocalSynthStatus
import io.github.oscyl.ui.parts.SynthStatus
import androidx.compose.animation.SharedTransitionLayout
import io.github.oscyl.ui.generate.GenerateModel
import io.github.oscyl.ui.generate.GenerateScreen
import io.github.oscyl.ui.preferences.PreferencesModel
import io.github.oscyl.ui.preferences.PreferencesScreen
import io.github.oscyl.ui.preferences.mode
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import io.github.oscyl.ui.review.ReviewModel
import io.github.oscyl.ui.welcome.GuideScreen
import io.github.oscyl.ui.welcome.WelcomeScreen
import io.github.oscyl.core.Role
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.backhandler.PredictiveBackHandler
import io.github.oscyl.core.BackupFile
import io.github.oscyl.ui.library.synthSource
import io.github.oscyl.ui.synth.SynthActions
import io.github.oscyl.ui.synth.SynthCard
import io.github.oscyl.ui.synth.UnitSlot
import io.github.oscyl.ui.review.ReviewSheet
import io.github.oscyl.ui.library.LibraryScreen
import io.github.oscyl.ui.library.RoleHint
import io.github.oscyl.ui.synth.SynthModel
import io.github.oscyl.ui.synth.SynthScreen
import org.jetbrains.compose.resources.stringResource

/**
 * The app: Oscyl's theme, the destination back stack (Library at its root, so Back
 * from any other destination returns there, with the predictive gesture) and the
 * adaptive shell around it.
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun App(platform: Platform) {
    // Core and driver words follow the app's language (before any model asks for them).
    val language = platform.language ?: Locale.current.toLanguageTag()
    remember(language) { setLocale(language) }
    val preferences = viewModel { PreferencesModel(platform.settingsPath) }
    val prefs by preferences.prefs.collectAsStateWithLifecycle()
    val haptics = remember(platform) { GatedHaptics(platform.haptics) { preferences.prefs.value.haptics } }
    OscylTheme(prefs.theme.mode) {
      // The status and navigation bars' icons in the app's theme, whatever the system's.
      io.github.oscyl.platform.SystemBarsAppearance(light = OscylTheme.colors.ground.luminance() > 0.5f)
      // Test tags double as resource ids, so the benchmarks' UI Automator finds the parts by name.
      Box(Modifier.testTagsAsIds()) {
        val teaching = remember(preferences) { Teaching({ preferences.prefs.value.taught(it) }, preferences::teach) }
        CompositionLocalProvider(LocalOscylHaptics provides haptics, LocalOscylMotion provides prefs.motion, LocalTeaching provides teaching) {
            // The destinations stand side by side in the island's order: a switch moves 30 dp along it (mirrored right to left).
            val rtl = LocalLayoutDirection.current == LayoutDirection.Rtl
            val axis = with(LocalDensity.current) { 30.dp.roundToPx() } * if (rtl) -1 else 1
            // The one synth worker, in the activity's retained scope: it outlives rotation.
            val synth = viewModel { SynthModel(platform.midi(), files = platform.files, backupsDir = platform.backupsDir) }
            // Demo mode: its synth connects once per run.
            LaunchedEffect(synth) {
                val demo = preferences.prefs.value.demo
                if (demo != null && !preferences.demoOpened) {
                    preferences.demoOpened = true
                    synth.openDemo(demo)
                }
            }
            val library = viewModel { LibraryModel(platform.librariesDir, synth, platform.files, platform.unsyncedDir) }
            val editor = viewModel { EditorModel(library, synth, { preferences.prefs.value.role }, teaching) }
            val play = viewModel { PlayModel(editor, preferences) }
            val review = viewModel { ReviewModel(library, synth, platform.backupsDir) }
            val generate = viewModel { GenerateModel(library, synth) }
            // A language change (the activity is made again; the models are kept): the core
            // words they hold are asked again. Rotation keeps the language and asks nothing.
            var wordsIn by rememberSaveable { mutableStateOf(language) }
            LaunchedEffect(language) {
                if (wordsIn != language) {
                    wordsIn = language
                    editor.relocalise()
                    generate.relocalise()
                    synth.relocalise()
                    review.refresh()
                }
            }
            val reviewUi by review.ui.collectAsStateWithLifecycle()
            val cards by synth.cards.collectAsStateWithLifecycle()
            val editorHead by editor.head.collectAsStateWithLifecycle()
            val libraryUi by library.ui.collectAsStateWithLifecycle()
            val programsRevision by synth.programs.collectAsStateWithLifecycle()
            val writes by synth.writes.collectAsStateWithLifecycle()
            // Each synth's edits waiting, asked again whenever an edit (through the
            // library), a read or a write may have changed them.
            val editedCounts = remember(cards, libraryUi, editorHead, programsRevision, writes) {
                cards.associate { it.id to (synth.document(it.id)?.editedCount()?.toInt() ?: 0) }
            }
            // The destination shown (later launches start where the welcome's answer said,
            // the desktop's start stage), and the pages over it: About, a licence, ...
            var current by rememberSaveable { mutableStateOf(startDestination(prefs.role)) }
            val backStack = rememberSaveable(saver = BackStackSaver) { mutableStateListOf<NavKey>(HomePage) }
            // The synth sheet, open over whatever is shown.
            var sheet by rememberSaveable { mutableStateOf(false) }
            // The edit history's sheet, over the editor.
            var history by rememberSaveable { mutableStateOf(false) }
            // The library program open in the editor (its source and place), kept past the
            // process: the models die with it, the destination and its pages do not, and the
            // editor would come back empty. A synth's program is not kept (its link is gone).
            var kept by rememberSaveable { mutableStateOf<Pair<String, Int>?>(null) }
            LaunchedEffect(Unit) {
                val (source, index) = kept ?: Pair(null, 0)
                if (source != null && library.open.value == null) {
                    library.select(source)
                    // Its programs arrive when the library has been read; anything else chosen meanwhile wins.
                    val shown = library.ui.first { it.programs != null }.programs
                    if (shown?.source == source && index < shown.rows.size && library.open.value == null) library.selectProgram(index)
                }
                library.open.collect { open -> kept = open?.takeIf { it.inLibrary }?.let { it.source to it.index } }
            }
            val go = { destination: Destination ->
                backStack.retainAll { it == HomePage }
                current = destination
            }
            val connected = cards.filter { it.connected }
            val reads = cards.mapNotNull { it.read }
            val status = SynthStatus(
                model = connected.firstOrNull()?.model,
                drawing = connected.firstOrNull()?.let { card -> synth.drivers.firstOrNull { it.id == card.driverId }?.panel },
                more = (connected.size - 1).coerceAtLeast(0),
                live = editorHead?.live == true,
                sends = { editor.panel?.sends ?: 0 },
                reading = reads.takeIf { it.isNotEmpty() }?.let { r -> r.sumOf { it.done }.toFloat() / r.sumOf { it.total }.coerceAtLeast(1) },
            )
            val scope = rememberCoroutineScope()
            // The licences are read once, early and off the main thread.
            LaunchedEffect(Unit) { preloadLicences() }
            val navigation = remember(backStack) {
                ShellNavigation(
                    synths = { sheet = true },
                    addSynth = { backStack.push(PickerPage) },
                    history = { history = true },
                    play = { backStack.push(PlayPage) },
                    preferences = { backStack.push(PreferencesPage) },
                    guide = { backStack.push(GuidePage) },
                    // The page enters whole: its notices are read before it opens (at once when preloaded).
                    about = { scope.launch { preloadLicences(); backStack.push(AboutPage) } },
                )
            }
            SharedTransitionLayout {
            // The first run gives way to the app: its listening card flies into the chip.
            AnimatedContent(
                prefs.onboarded,
                transitionSpec = {
                    if (prefs.motion) fadeIn(tween(OscylTokens.slowMs, easing = OscylTokens.easing)) togetherWith fadeOut(tween(OscylTokens.baseMs, easing = OscylTokens.easing))
                    else EnterTransition.None togetherWith ExitTransition.None
                },
                label = "first run",
            ) { onboarded ->
            if (!onboarded) {
            CompositionLocalProvider(LocalSharedTransitionScope provides this@SharedTransitionLayout) {
                // The first run, once; going back skips it. The role question left it:
                // a new install starts in the Library (the setting stays in Preferences).
                var picking by rememberSaveable { mutableStateOf(false) }
                val done = { demo: String? ->
                    preferences.finishWelcome(null, demo)
                    if (demo != null) {
                        library.addDemo(demo)
                        preferences.demoOpened = true
                        synth.openDemo(demo)
                    }
                    backStack.retainAll { it == HomePage }
                    current = Destination.Library
                }
                PredictiveBackHandler { progress ->
                    progress.collect {}
                    if (picking) picking = false else done(null)
                }
                Box(Modifier.fillMaxSize().background(OscylTheme.colors.ground)) {
                    if (picking) {
                        val access by synth.access.collectAsStateWithLifecycle()
                        SynthPicker(
                            synth.drivers,
                            access,
                            away = emptyList(),
                            onDemo = { done(it) },
                            onAllow = synth::allow,
                            onConnect = synth::connect,
                            onBack = { picking = false },
                        )
                    } else {
                        WelcomeScreen(
                            cards,
                            onRead = { card ->
                                done(null)
                                if (card.read == null) synth.readAll(card)
                            },
                            onTry = { picking = true },
                            onSkip = { done(null) },
                            card = Modifier.sharedPart(chipKey(Destination.Library), visible = !prefs.onboarded),
                        )
                    }
                }
            }
            } else {
            // One frame for the shell and the sheets over it: they keep clear of the side island.
            val frame = remember { ShellFrame() }
            CompositionLocalProvider(
                LocalSynthStatus provides status,
                LocalShellNavigation provides navigation,
                LocalSharedTransitionScope provides this@SharedTransitionLayout,
                LocalShellFrame provides frame,
            ) {
            // The sheets rise over the shell, island and all: a page's own (Play's knob
            // picker) through the sheet layer, drawn last.
            val sheetLayer = remember { SheetLayer() }
            Box(Modifier.fillMaxSize()) {
            CompositionLocalProvider(LocalSheetLayer provides sheetLayer) {
            Shell(
                // Pages above a destination (About, a licence) keep its plate on the island.
                current = current,
                onSelect = go,
                // The Save key counts and keeps one source: in the Library the list's, elsewhere
                // the editor's program's (the list may show another while it is open).
                saveCount = (if (current == Destination.Library) null else editorHead?.saveCount) ?: libraryUi.programs?.rows?.count { it.edited } ?: 0,
                // A tap keeps a library's edits at once; a hold (and a synth's sync) shows the review first.
                onSave = {
                    saveTarget(current, library)
                    // The first Save says, in the line it shows anyway, that a hold reviews first.
                    var hint = false
                    teaching.once(io.github.oscyl.ui.preferences.Hints.SAVE_REVIEW) { hint = true }
                    review.quickSave(hint)
                },
                onReview = {
                    teaching.teach(io.github.oscyl.ui.preferences.Hints.SAVE_REVIEW)
                    saveTarget(current, library)
                    review.open()
                },
                // The key shows a save or a sync on its way, and says "Saved" with the notice.
                saving = when (val state = reviewUi.state) {
                    ReviewState.BackingUp -> 0f
                    is ReviewState.Writing -> if (state.total > 0) state.done.toFloat() / state.total else 0f
                    else -> null
                },
                saved = libraryUi.notice is io.github.oscyl.ui.library.Notice.Saved,
                sync = (if (current == Destination.Library) null else editorHead?.sync) ?: (libraryUi.programs?.inLibrary == false),
            ) { modifier ->
                Box(modifier) {
                    // Back from another destination returns to the Library, with the predictive gesture.
                    val destinations = remember { DestinationMotion(current) }
                    PredictiveBackHandler(enabled = backStack.size == 1 && current != Destination.Library) { progress ->
                        // The Library comes in under the finger, as it will when Back is let go.
                        try {
                            progress.collect { destinations.seek(current, Destination.Library, it.progress) }
                            current = Destination.Library
                        } catch (e: kotlin.coroutines.cancellation.CancellationException) {
                            kotlinx.coroutines.withContext(kotlinx.coroutines.NonCancellable) { destinations.release(if (prefs.motion) 1f else 0f) }
                            throw e
                        }
                    }
                    // The destinations stay composed under the pages above them: Back from a page
                    // uncovers what was there, with nothing to compose again.
                    DestinationHost(
                        current,
                        // Under a page or the open synth sheet they are silent: that is the pane.
                        // A touch on the page (not on the notice, which stands over it) puts a notice away.
                        Modifier.fillMaxSize().pointerInput(Unit) {
                            // A tap: a scroll or a drag (a deletion's Undo still wanted) leaves it.
                            awaitEachGesture {
                                val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
                                val up = waitForUpOrCancellation(PointerEventPass.Initial) ?: return@awaitEachGesture
                                val tapped = (up.position - down.position).getDistance() <= viewConfiguration.touchSlop
                                if (tapped && library.ui.value.notice != null) library.dismissNotice()
                            }
                        }.then(if (sheet || history || reviewUi.open || backStack.size > 1) Modifier.clearAndSetSemantics {} else Modifier),
                        destinations,
                        axis,
                        covered = sheet || history || reviewUi.open || backStack.size > 1,
                    ) { destination ->
                        when (destination) {
                            Destination.Library -> LibraryScreen(
                                library,
                                onOpenSynth = navigation.synths,
                                onOpenProgram = { go(Destination.Editor) },
                                roleHint = if (prefs.role == null) ({ RoleHint(preferences::setRole) }) else null,
                            )
                            Destination.Editor -> EditorDestination(
                                editor,
                                library,
                                onOpenLibrary = { go(Destination.Library) },
                                onOpenSynth = navigation.synths,
                            )
                            Destination.Generate -> GenerateScreen(generate)
                        }
                    }
                    // Notices stand above the island on every destination (pages cover them).
                    NoticeHost(libraryUi, library, Modifier.align(Alignment.BottomCenter), onOpened = { go(Destination.Editor) })
                    NavDisplay(
                        backStack = backStack,
                        modifier = Modifier.fillMaxSize().then(if (sheet || history || reviewUi.open) Modifier.clearAndSetSemantics {} else Modifier),
                        // Never the destinations' own entry: a second Back (or a second tap on a page's
                        // arrow) landing while a page is still on its way out would empty the stack,
                        // and an empty stack is a crash.
                        onBack = { if (backStack.size > 1) backStack.removeAt(backStack.lastIndex) },
                        transitionSpec = { pageChange(forward = true, prefs.motion, rtl) },
                        popTransitionSpec = { pageChange(forward = false, prefs.motion, rtl) },
                        // Predictive Back drives this one's progress with the finger: the page
                        // follows it toward the edge it came from, over the page it returns to.
                        predictivePopTransitionSpec = { pageChange(forward = false, prefs.motion, rtl) },
                        entryProvider = { key ->
                            NavEntry(key) {
                                val back = { if (backStack.size > 1) backStack.removeAt(backStack.lastIndex); Unit }
                                // A page above the destinations slides over them: it has its own ground.
                                // (its touches end there); under them all, an empty entry lets the destinations show.
                                Box(if (key == HomePage) Modifier else Modifier.fillMaxSize().background(OscylTheme.colors.ground).pointerInput(Unit) {}) {
                                when (key) {
                                    HomePage -> Unit
                                    PickerPage -> {
                                        val access by synth.access.collectAsStateWithLifecycle()
                                        SynthPicker(
                                            synth.drivers,
                                            access,
                                            away = cards.filter { !it.connected },
                                            onDemo = { id ->
                                                synth.openDemo(id)
                                                back()
                                                sheet = true
                                            },
                                            onAllow = synth::allow,
                                            onConnect = synth::connect,
                                            onBack = back,
                                        )
                                    }
                                    PreferencesPage -> PreferencesScreen(preferences, synth.drivers, onBack = back)
                                    PlayPage -> PlayScreen(play, editor, onBack = back)
                                    GuidePage -> GuideScreen(onBack = back)
                                    AboutPage -> AboutScreen(onBack = back, onOpen = { backStack.push(LicencePage(it.id, it.version)) })
                                    is LicencePage -> LicenceScreen(key.id, key.version, onBack = back)
                                    else -> Unit
                                }
                                }
                            }
                        },
                    )
                }
            }
            DockedSheet(open = history, onClose = { history = false }, title = stringResource(Res.string.history_title)) { visible ->
                // Asked again whenever it shows and after every change to the document.
                val steps = remember(visible, libraryUi, editorHead) { if (visible) library.openHistory() else emptyList() }
                val snapshots by editor.snapshots.collectAsStateWithLifecycle()
                HistoryList(
                    steps,
                    onJump = library::jumpOpenHistory,
                    onClose = { history = false },
                    // Only for a program open in the editor that can change.
                    snapshots = snapshots.takeIf { editorHead?.let { it.loaded && !it.readOnly } == true },
                    onStore = editor::storeSnapshot,
                    onRecall = editor::recallSnapshot,
                )
            }
            DockedSheet(open = sheet, onClose = { sheet = false }, title = stringResource(Res.string.sheet_title)) { visible ->
                SynthScreen(
                    synth,
                    onAbout = navigation.about,
                    // The review is the island's sheet: this one gives way to it.
                    actions = remember(synth, library, review) { AppSynthActions(synth, library, review) { sheet = false } },
                    edited = editedCounts,
                    live = editorHead?.takeIf { it.canLive }?.let { Live(it.live, it.liveSynth, editor::toggleLive) },
                    onAdd = {
                        sheet = false
                        backStack.push(PickerPage)
                    },
                    onClose = { sheet = false },
                    visible = visible,
                )
            }
            ReviewSheet(reviewUi, review)
            }
            SheetLayerHost(sheetLayer)
            }
            }
            }
            }
            }
        }
      }
    }
}

/**
 * A page above the destinations comes in whole from the end edge and covers what it
 * opened from, which gives way a quarter of the width and dims; Back takes it out the
 * same way, over the page it returns to (DESIGN.md Motion: `slow`, the one easing).
 * Only placement and alpha change. None with motion off.
 */
private fun pageChange(forward: Boolean, motion: Boolean, rtl: Boolean): ContentTransform {
    if (!motion) return EnterTransition.None togetherWith ExitTransition.None
    val end = if (rtl) -1 else 1
    val move = tween<IntOffset>(OscylTokens.slowMs, easing = OscylTokens.easing)
    val dim = tween<Float>(OscylTokens.slowMs, easing = OscylTokens.easing)
    return if (forward) {
        slideInHorizontally(move) { it * end } togetherWith (slideOutHorizontally(move) { -it / 4 * end } + fadeOut(dim, targetAlpha = 0.5f))
    } else {
        // The page leaving stays on top of the one coming back.
        ContentTransform(
            slideInHorizontally(move) { -it / 4 * end } + fadeIn(dim, initialAlpha = 0.5f),
            slideOutHorizontally(move) { it * end },
            targetContentZIndex = -1f,
        )
    }
}

/** A synth card's work, through the models that own it. */
private class AppSynthActions(val synth: SynthModel, val library: LibraryModel, val review: ReviewModel, val onReview: () -> Unit) : SynthActions {
    override fun readAll(card: SynthCard) = synth.readAll(card)
    override fun cancelRead(card: SynthCard) = synth.cancelRead(card)
    override fun readCurrent(card: SynthCard) = synth.readCurrent(card)
    override fun review(card: SynthCard) {
        onReview()
        library.select(synthSource(card.id))
        review.openFor(synthSource(card.id))
    }
    override fun loadUnit(card: SynthCard, slot: UnitSlot) = synth.loadUnit(card, slot.module, slot.slot)
    override fun clearUnit(card: SynthCard, slot: UnitSlot) = synth.clearUnit(card, slot.module, slot.slot, slot.name ?: "")
    override fun openBackup(file: BackupFile) = library.openBackup(file)
    override fun shareBackup(file: BackupFile) = library.shareBackup(file)
    override fun putBack(file: BackupFile) = synth.putBack(file)
}

/** Where a run starts for [role]: the Library, or the editor for sounds and performances. */
private fun startDestination(role: Role?): Destination = when (role) {
    Role.SOUND_DESIGN, Role.PERFORM -> Destination.Editor
    Role.ORGANISE, null -> Destination.Library
}

/**
 * Open [page] above what is shown, once: a second tap landing before the first has
 * drawn (a doubled tap on a menu row) would stack the page on itself, and a page twice
 * in the stack shares one saved-state key.
 */
private fun SnapshotStateList<NavKey>.push(page: NavKey) {
    if (page !in this) add(page)
}

/** The destinations, under every page. */
data object HomePage : NavKey

/** Add a synth: the picker, over the destination. */
data object PickerPage : NavKey

/** The guide, from the top bar's menu. */
data object GuidePage : NavKey

/** About Oscyl, from the top bar's menu. */
data object AboutPage : NavKey

/** Preferences, from the top bar's menu. */
data object PreferencesPage : NavKey

/** The play page, from the editor's bar. */
data object PlayPage : NavKey

/** One notice's licence text, above About. */
data class LicencePage(val id: String, val version: String?) : NavKey

/** Destinations by name, About as `about`, a licence as `licence`, its id and version (NUL-separated). */
private val BackStackSaver = listSaver<SnapshotStateList<NavKey>, String>(
    save = { stack ->
        stack.map { key ->
            when (key) {
                HomePage -> "home"
                PickerPage -> "picker"
                AboutPage -> "about"
                PreferencesPage -> "preferences"
                PlayPage -> "play"
                GuidePage -> "guide"
                is LicencePage -> listOfNotNull("licence", key.id, key.version).joinToString("\u0000")
                else -> error("unsaved page $key")
            }
        }
    },
    restore = { saved ->
        saved.mapTo(mutableStateListOf()) { entry ->
            val parts = entry.split('\u0000')
            when (parts[0]) {
                "home" -> HomePage
                "picker" -> PickerPage
                "about" -> AboutPage
                "preferences" -> PreferencesPage
                "play" -> PlayPage
                "guide" -> GuidePage
                "licence" -> LicencePage(parts[1], parts.getOrNull(2))
                else -> error("unknown page $entry")
            }
        }
    },
)

/**
 * The island around the [content]: standing at the start edge, beside the content, in
 * a window at least medium wide and medium tall (foldables, tablets); on end at the
 * end edge in a wide but short window (a phone on its side); otherwise floating at
 * the foot (phones).
 */
@Composable
fun Shell(
    current: Destination,
    onSelect: (Destination) -> Unit,
    saveCount: Int,
    onSave: () -> Unit,
    sync: Boolean = false,
    onReview: () -> Unit = onSave,
    /** A save or a sync under way, how far (0..1), and the "Saved" notice standing: the Save key shows both. */
    saving: Float? = null,
    saved: Boolean = false,
    /** The system bars at the foot (navigation bar, taskbar): what content keeps clear beside a side island. */
    systemBottom: Dp = safeInsets.only(WindowInsetsSides.Bottom).asPaddingValues().calculateBottomPadding(),
    content: @Composable (Modifier) -> Unit,
) {
    val sizeClass = currentWindowAdaptiveInfoV2().windowSizeClass
    val wideEnough = sizeClass.isWidthAtLeastBreakpoint(WindowSizeClass.WIDTH_DP_MEDIUM_LOWER_BOUND)
    val tallEnough = sizeClass.isHeightAtLeastBreakpoint(WindowSizeClass.HEIGHT_DP_MEDIUM_LOWER_BOUND)
    val wide = wideEnough && tallEnough
    // Search in the program list belongs to the shell: the floating island turns into its field.
    val search = LocalProgramSearch.current ?: rememberSaveable(saver = ProgramSearch.Saver) { ProgramSearch() }
    val frame = LocalShellFrame.current ?: remember { ShellFrame() }
    val density = LocalDensity.current
    CompositionLocalProvider(LocalProgramSearch provides search, LocalShellFrame provides frame) {
    Box(Modifier.fillMaxSize().background(OscylTheme.colors.ground)) {
        if (wideEnough && !tallEnough) {
            // A phone on its side: the island stands on end at the end edge, in the right thumb's reach.
            Row(Modifier.fillMaxSize()) {
                CompositionLocalProvider(LocalIslandClearance provides systemBottom) {
                    // The island keeps the end edge's cutout and bar: the content does not pad for them again.
                    content(Modifier.weight(1f).consumeWindowInsets(safeInsets.only(WindowInsetsSides.End)))
                }
                SideIsland(
                    current, onSelect, saveCount, onSave,
                    sync = sync,
                    onReview = onReview,
                    progress = saving,
                    confirmed = saved,
                    modifier = Modifier
                        .align(Alignment.CenterVertically)
                        .onSizeChanged { frame.sideStart = 0.dp; frame.sideEnd = with(density) { it.width.toDp() } }
                        .windowInsetsPadding(safeInsets.only(WindowInsetsSides.End + WindowInsetsSides.Vertical))
                        .padding(8.dp),
                )
            }
        } else if (wide) {
            Row(Modifier.fillMaxSize()) {
                SideIsland(
                    current, onSelect, saveCount, onSave,
                    sync = sync,
                    onReview = onReview,
                    progress = saving,
                    confirmed = saved,
                    modifier = Modifier
                        .align(Alignment.CenterVertically)
                        .onSizeChanged { frame.sideEnd = 0.dp; frame.sideStart = with(density) { it.width.toDp() } }
                        .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Start + WindowInsetsSides.Vertical))
                        .padding(12.dp),
                )
                // No island at the foot: the system bars are what the content keeps clear there.
                CompositionLocalProvider(LocalIslandClearance provides systemBottom) {
                    content(Modifier.weight(1f).consumeWindowInsets(safeInsets.only(WindowInsetsSides.Start)))
                }
            }
        } else {
            DisposableEffect(frame) {
                frame.sideStart = 0.dp
                frame.sideEnd = 0.dp
                onDispose {}
            }
            var clearance by remember { mutableStateOf(0.dp) }
            DisposableEffect(search) {
                search.hosted = true
                onDispose { search.hosted = false }
            }
            // Searching, the island is the field and stands on the keyboard (safeInsets holds
            // the IME). Otherwise, with the keyboard up the island steps away, and what stood
            // on it stands on the keyboard.
            val searching = search.active && search.available
            val ime = WindowInsets.ime.getBottom(density)
            // While the page scrolls by hand the island dims, so the rows under it read.
            val peek = remember(density) { ScrollPeek(with(density) { PeekTravel.toPx() }) }
            val peekAlpha by peekAlpha(peek)
            CompositionLocalProvider(
                LocalIslandClearance provides if (ime > 0 && !searching) with(density) { ime.toDp() } else clearance,
                LocalScrollPeek provides peek,
            ) {
                content(with(peek) { Modifier.fillMaxSize().peekSource() })
            }
            val place = Modifier
                .align(Alignment.BottomCenter)
                // The island with its margin and the inset under it: what scrolling content keeps clear.
                .onSizeChanged { clearance = with(density) { it.height.toDp() } }
                .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Horizontal + WindowInsetsSides.Bottom))
                .padding(12.dp)
                // A phone in landscape: keep the island a hand's width, not a full-width bar.
                .widthIn(max = 560.dp)
                // Dimmed by fading each stroke, not through an offscreen copy: making that copy
                // as a scroll began cost its first frames (30 ms and more of the render thread).
                .graphicsLayer { alpha = if (searching) 1f else peekAlpha; compositingStrategy = CompositingStrategy.ModulateAlpha }
            if (ime == 0 && !searching && clearance > 0.dp) {
                // The page fades into the ground under the island (as Claude's and Telegram's
                // composers do): rows never read through beside it, and the island stands
                // clearly above them. Drawn only, never a target.
                val ground = OscylTheme.colors.ground
                Box(
                    Modifier
                        .align(Alignment.BottomCenter)
                        .fillMaxWidth()
                        .height(clearance + IslandFade)
                        .drawWithCache {
                            // One gradient per size; the dim is its alpha (no layer, nothing made per frame).
                            val fade = Brush.verticalGradient(0f to ground.copy(alpha = 0f), 0.4f to ground.copy(alpha = 0.85f), 1f to ground.copy(alpha = 0.95f))
                            onDrawBehind { drawRect(fade, alpha = peekAlpha) }
                        },
                )
            }
            // The island, the search field, or nothing while the keyboard is up (or on its
            // way): each change a crossfade, in `base`, out `fast`; nothing resizes per frame.
            val motion = LocalOscylMotion.current
            AnimatedContent(
                when {
                    searching -> FootPart.Search
                    imeHeading() -> FootPart.None
                    else -> FootPart.Island
                },
                place,
                transitionSpec = {
                    if (!motion) EnterTransition.None togetherWith ExitTransition.None
                    else (fadeIn(tween(OscylTokens.baseMs, easing = OscylTokens.easing)) togetherWith fadeOut(tween(OscylTokens.fastMs, easing = OscylTokens.easing)))
                        .using(SizeTransform(clip = false) { _, _ -> snap() })
                },
                contentAlignment = Alignment.BottomCenter,
                label = "foot",
            ) { part ->
                when (part) {
                    FootPart.Search -> SearchIsland(search)
                    FootPart.Island -> Island(current, onSelect, saveCount, onSave, sync = sync, onReview = onReview, search = search.takeIf { it.available }, progress = saving, confirmed = saved)
                    FootPart.None -> Box(Modifier.fillMaxWidth())
                }
            }
        }
    }
    }
}

/** What stands at the foot of a compact window. */
private enum class FootPart { Island, Search, None }

/** How far above the island the page starts fading under it. */
private val IslandFade = OscylTokens.railAnchor

/** Outside the Library, the Save key keeps the editor's program's source: the list is taken there first. */
private fun saveTarget(current: Destination, library: LibraryModel) {
    if (current == Destination.Library) return
    val open = library.open.value?.source ?: return
    if (open != library.ui.value.programs?.source) library.select(open)
}

/**
 * How much of the foot is covered: by the floating island (with the system bars under
 * it), or beside a side island by the system bars alone. A scrolling destination pads
 * its end by this, so its last line scrolls clear; a docked part stands on it.
 */
val LocalIslandClearance = compositionLocalOf { 0.dp }

/** Where a part at the foot stands besides the island: a notice keeps clear of it. */
enum class FootSide { Keys, Start, End }

/**
 * The foot as the shell knows it (one owner): the page's foot keys (their height) and the
 * parts standing at its edges (the editor's rail and Compare, their width with their
 * margins), each said by the part while its destination is in sight; and the side
 * island's width, which floating panels keep clear of. The notice stands above the keys
 * and between the edge parts.
 */
@androidx.compose.runtime.Stable
class ShellFrame {
    internal val parts = androidx.compose.runtime.mutableStateMapOf<Any, Pair<FootSide, Dp>>()

    /** The tallest (or widest) part on [side] now in sight. */
    fun reserve(side: FootSide): Dp = parts.values.filter { it.first == side }.maxOfOrNull { it.second } ?: 0.dp

    /** The side island and its margins, at the start or the end edge (0 with the island at the foot). */
    var sideStart by mutableStateOf(0.dp)
    var sideEnd by mutableStateOf(0.dp)
}

/** The shell's [ShellFrame]; null outside a shell (a part alone in a test). */
val LocalShellFrame = compositionLocalOf<ShellFrame?> { null }

/** This part stands at the foot on [side], [amount] tall (keys) or wide (edges), while its destination is in sight. */
@Composable
fun ReserveFoot(side: FootSide, amount: Dp, shown: Boolean = true) {
    val frame = LocalShellFrame.current ?: return
    val inSight = LocalDestinationShown.current && shown && amount > 0.dp
    val key = remember { Any() }
    DisposableEffect(frame, inSight, side, amount) {
        if (inSight) frame.parts[key] = side to amount else frame.parts.remove(key)
        onDispose { frame.parts.remove(key) }
    }
}

/** A destination's title legend only: the shell's previews. */
@Composable
private fun Placeholder(destination: Destination) {
    val c = OscylTheme.colors
    Box(
        Modifier
            .fillMaxSize()
            .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))
            .padding(16.dp),
    ) {
        BasicText(
            stringResource(destination.label).toUpperCase(Locale.current),
            Modifier.semantics { heading() },
            style = MaterialTheme.typography.labelMedium.copy(color = c.legend),
        )
    }
}

@Composable
private fun ShellPreview(mode: OscylThemeMode = OscylThemeMode.Dark) = OscylTheme(mode) {
    Shell(Destination.Editor, onSelect = {}, saveCount = 4, onSave = {}) { modifier ->
        Box(modifier) { Placeholder(Destination.Editor) }
    }
}

@OscylSizePreviews
@Composable
private fun shell() = ShellPreview()

@OscylPreviews
@Composable
private fun shell_compact(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = ShellPreview(mode)

@OscylA11yPreviews
@Composable
private fun shell_a11y() = ShellPreview()
