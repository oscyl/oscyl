package io.github.oscyl.ui.library

import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import io.github.oscyl.platform.screenReaderOn
import io.github.oscyl.ui.parts.Transient
import io.github.oscyl.ui.shell.FootSide
import io.github.oscyl.ui.shell.LocalShellFrame
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlin.time.TimeSource
import androidx.compose.ui.platform.LocalAccessibilityManager
import io.github.oscyl.platform.LocalOscylHaptics
import io.github.oscyl.shared.generated.resources.notice_roll_hint
import io.github.oscyl.shared.generated.resources.notice_saved
import io.github.oscyl.shared.generated.resources.notice_kept
import io.github.oscyl.shared.generated.resources.notice_compare_hint
import io.github.oscyl.shared.generated.resources.notice_snapshot_replaced
import io.github.oscyl.shared.generated.resources.notice_rolled
import io.github.oscyl.shared.generated.resources.notice_added
import kotlinx.coroutines.withContext
import kotlinx.coroutines.NonCancellable
import io.github.oscyl.ui.shell.LocalDestinationShown
import androidx.compose.ui.backhandler.PredictiveBackHandler
import androidx.compose.material3.adaptive.layout.ListDetailPaneScaffold
import io.github.oscyl.ui.editor.samplePanel
import androidx.compose.runtime.CompositionLocalProvider
import io.github.oscyl.ui.parts.LocalSynthStatus
import io.github.oscyl.ui.parts.SynthStatus
import androidx.compose.ui.tooling.preview.Preview
import io.github.oscyl.ui.preview.CutoutFrame
import io.github.oscyl.ui.preview.FakeCutout
import io.github.oscyl.ui.preview.LandscapeCutouts
import io.github.oscyl.ui.preview.PHONE
import io.github.oscyl.ui.preview.PHONE_LANDSCAPE
import io.github.oscyl.ui.preview.PortraitCutouts
import io.github.oscyl.ui.parts.safeInsets
import androidx.compose.foundation.layout.BoxWithConstraints
import io.github.oscyl.shared.generated.resources.editor_empty_body
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.material3.MaterialTheme
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.layout.widthIn
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.background
import androidx.compose.material3.adaptive.ExperimentalMaterial3AdaptiveApi
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfoV2
import androidx.compose.material3.adaptive.layout.AnimatedPane
import androidx.compose.material3.adaptive.layout.ListDetailPaneScaffoldRole
import androidx.compose.material3.adaptive.layout.PaneAdaptedValue
import androidx.compose.material3.adaptive.layout.ThreePaneScaffoldDestinationItem
import androidx.compose.material3.adaptive.layout.calculatePaneScaffoldDirectiveWithTwoPanesOnMediumWidth
import androidx.compose.material3.adaptive.navigation.rememberListDetailPaneScaffoldNavigator
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.oscyl.core.NodeKind
import io.github.oscyl.core.OutlineLine
import io.github.oscyl.core.ProgramRow
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.notice_rail_hint
import io.github.oscyl.shared.generated.resources.notice_saved_review
import io.github.oscyl.shared.generated.resources.notice_cannot_read
import io.github.oscyl.shared.generated.resources.notice_cannot_write
import io.github.oscyl.shared.generated.resources.notice_cleared
import io.github.oscyl.shared.generated.resources.notice_copied
import io.github.oscyl.shared.generated.resources.notice_copied_to
import io.github.oscyl.shared.generated.resources.notice_deleted
import io.github.oscyl.shared.generated.resources.notice_export_lost
import io.github.oscyl.shared.generated.resources.notice_exported
import io.github.oscyl.shared.generated.resources.notice_full
import io.github.oscyl.shared.generated.resources.notice_pasted
import io.github.oscyl.shared.generated.resources.notice_removed
import io.github.oscyl.shared.generated.resources.notice_retry
import io.github.oscyl.shared.generated.resources.notice_save_failed
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.OscylSizePreviews
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.shell.Destination
import io.github.oscyl.ui.shell.IslandNotice
import io.github.oscyl.ui.shell.LocalIslandClearance
import io.github.oscyl.ui.shell.Shell
import io.github.oscyl.ui.synth.ReadProgress
import io.github.oscyl.ui.synth.SynthCard
import kotlinx.coroutines.launch
import org.jetbrains.compose.resources.pluralStringResource
import org.jetbrains.compose.resources.stringResource

/** The Library destination over the app's [LibraryModel]. */
@Composable
fun LibraryScreen(
    model: LibraryModel,
    onOpenSynth: () -> Unit,
    modifier: Modifier = Modifier,
    onOpenProgram: () -> Unit = {},
    roleHint: (@Composable () -> Unit)? = null,
) {
    val ui by model.ui.collectAsStateWithLifecycle()
    LibraryScreen(ui, model, onOpenSynth, modifier, onOpenProgram, roleHint)
}

/**
 * Sources beside their programs (Material 3 adaptive list-detail, in Oscyl's tones):
 * one pane at a time on a phone, where Back (and the predictive gesture) returns to
 * the sources; side by side from medium width. The chosen source is the scaffold's
 * own saved state, so it survives rotation and process death. Notices float above
 * the island.
 */
@OptIn(ExperimentalMaterial3AdaptiveApi::class, androidx.compose.ui.ExperimentalComposeUiApi::class)
@Composable
fun LibraryScreen(
    ui: LibraryUi,
    actions: LibraryActions,
    onOpenSynth: () -> Unit,
    modifier: Modifier = Modifier,
    onOpenProgram: () -> Unit = {},
    roleHint: (@Composable () -> Unit)? = null,
) {
    val directive = calculatePaneScaffoldDirectiveWithTwoPanesOnMediumWidth(currentWindowAdaptiveInfoV2())
        .copy(horizontalPartitionSpacerSize = 0.dp)
    val navigator = rememberListDetailPaneScaffoldNavigator(
        scaffoldDirective = directive,
        initialDestinationHistory = listOfNotNull(
            ThreePaneScaffoldDestinationItem<String>(ListDetailPaneScaffoldRole.List),
            ui.programs?.let { ThreePaneScaffoldDestinationItem(ListDetailPaneScaffoldRole.Detail, it.source) },
        ),
    )
    val scope = rememberCoroutineScope()
    val shown = navigator.currentDestination?.contentKey
    LaunchedEffect(shown) { actions.select(shown) }
    // A library just imported comes into view, as on desktop.
    LaunchedEffect(ui.reveal) {
        val key = ui.reveal ?: return@LaunchedEffect
        navigator.navigateTo(ListDetailPaneScaffoldRole.Detail, key)
        actions.revealed()
    }
    val single = navigator.scaffoldValue[ListDetailPaneScaffoldRole.List] == PaneAdaptedValue.Hidden ||
        navigator.scaffoldValue[ListDetailPaneScaffoldRole.Detail] == PaneAdaptedValue.Hidden
    val motion = LocalOscylMotion.current
    val enter = if (motion) fadeIn(tween(OscylTokens.slowMs, easing = OscylTokens.easing)) else EnterTransition.None
    val exit = if (motion) fadeOut(tween(OscylTokens.slowMs, easing = OscylTokens.easing)) else ExitTransition.None
    val bounds = if (motion) tween<IntRect>(OscylTokens.slowMs, easing = OscylTokens.easing) else snap()
    // Back (and its predictive gesture) returns from the programs to the sources, only while
    // the Library is the destination in sight: hidden under another destination, or under a
    // page or a sheet, it must never take a Back meant for them (it closed the open program).
    val inSight = LocalDestinationShown.current
    PredictiveBackHandler(enabled = inSight && navigator.canNavigateBack()) { progress ->
        try {
            progress.collect { navigator.seekBack(fraction = it.progress) }
            navigator.navigateBack()
        } catch (e: kotlin.coroutines.cancellation.CancellationException) {
            withContext(NonCancellable) { navigator.seekBack(fraction = 0f) }
            throw e
        }
    }
    Box(modifier.fillMaxSize()) {
        ListDetailPaneScaffold(
            directive = navigator.scaffoldDirective,
            scaffoldState = navigator.scaffoldState,
            listPane = {
                AnimatedPane(Modifier.preferredWidth(300.dp), enterTransition = enter, exitTransition = exit, boundsAnimationSpec = bounds) {
                    Row {
                        Sources(
                            ui,
                            selected = shown,
                            actions = actions,
                            onSelect = { key -> scope.launch { navigator.navigateTo(ListDetailPaneScaffoldRole.Detail, key) } },
                            onOpenSynth = onOpenSynth,
                            modifier = Modifier.weight(1f),
                            // Beside the programs, their bar carries the chip and the menu.
                            topBarActions = single,
                            roleHint = roleHint,
                        )
                        // The column's 1 dp rule, beside the programs.
                        if (!single) Box(Modifier.width(OscylTokens.ruleWidth).fillMaxHeight().background(OscylTheme.colors.rule))
                    }
                }
            },
            detailPane = {
                AnimatedPane(enterTransition = enter, exitTransition = exit, boundsAnimationSpec = bounds) {
                    val list = @Composable { listModifier: Modifier ->
                        Box(listModifier) {
                            ProgramList(
                                ui.programs?.takeIf { it.source == shown },
                                actions,
                                onBack = if (single) ({ scope.launch { navigator.navigateBack() } }) else null,
                                onOpen = onOpenProgram,
                                loading = shown != null && ui.programs?.source != shown,
                            )
                        }
                    }
                    BoxWithConstraints {
                    // Room for the editor's place too (a tablet): otherwise the list takes the pane.
                    if (single || maxWidth < 480.dp + 280.dp) {
                        list(Modifier.fillMaxSize())
                    } else {
                        // Beside the sources, the list keeps a reading width, as the desktop's
                        // three panes: the editor's place beside it says what goes there.
                        Row(Modifier.fillMaxSize()) {
                            list(Modifier.widthIn(max = 480.dp).fillMaxHeight())
                            Box(Modifier.width(OscylTokens.ruleWidth).fillMaxHeight().background(OscylTheme.colors.rule))
                            Box(Modifier.weight(1f).fillMaxHeight().padding(24.dp), contentAlignment = Alignment.Center) {
                                BasicText(
                                    stringResource(Res.string.editor_empty_body),
                                    style = MaterialTheme.typography.bodyMedium.copy(
                                        color = OscylTheme.colors.legend,
                                        textAlign = TextAlign.Center,
                                        textDirection = TextDirection.Content,
                                    ),
                                )
                            }
                        }
                    }
                    }
                }
            },
        )
    }
}

/** The Library's notice above the island (the shell shows it on every destination). */
@Composable
fun NoticeHost(ui: LibraryUi, actions: LibraryActions, modifier: Modifier = Modifier, onOpened: () -> Unit = {}) {
    // An action that took (added, kept, saved, pasted, removed…) is felt as well as read.
    val haptics = LocalOscylHaptics.current
    LaunchedEffect(ui.notice) {
        when (ui.notice) {
            is Notice.Added, is Notice.Kept, is Notice.Saved, is Notice.Pasted, is Notice.Removed,
            is Notice.Cleared, is Notice.Deleted, is Notice.Copied, is Notice.CopiedTo, is Notice.Exported,
            -> haptics.confirm()
            else -> Unit
        }
    }
    val current = ui.notice
    // It goes by itself, like a snackbar, after a time that suits it (see [noticeTimeout]),
    // stretched by the platform's accessibility timeout. The time counts only while the app
    // is in front and no finger rests on the notice.
    val text = current?.let { noticeText(it) }
    val a11y = LocalAccessibilityManager.current
    val reader = screenReaderOn()
    val dismiss by rememberUpdatedState(actions::dismissNotice)
    var held by remember { mutableStateOf(false) }
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    LaunchedEffect(current) {
        val notice = current ?: return@LaunchedEffect
        val withAction = notice.undoable || notice.opens != null
        val base = noticeTimeout(notice, text?.length ?: 0, reader) ?: return@LaunchedEffect
        val wait = a11y?.calculateRecommendedTimeoutMillis(base, containsIcons = true, containsText = true, containsControls = withAction) ?: base
        if (wait == Long.MAX_VALUE) return@LaunchedEffect
        var left = wait
        lifecycle.repeatOnLifecycle(Lifecycle.State.RESUMED) {
            snapshotFlow { held }.collectLatest { down ->
                if (down) return@collectLatest
                val start = TimeSource.Monotonic.markNow()
                try {
                    delay(left)
                    dismiss()
                } finally {
                    left = (left - start.elapsedNow().inWholeMilliseconds).coerceAtLeast(0)
                }
            }
        }
    }
    // Clear of the island (its clearance holds the bars under it, or the keyboard), above the
    // page's foot keys, and between the parts at the edges (the editor's rail and Compare).
    val frame = LocalShellFrame.current
    val keys = frame?.reserve(FootSide.Keys) ?: 0.dp
    val start = frame?.reserve(FootSide.Start) ?: 0.dp
    val end = frame?.reserve(FootSide.End) ?: 0.dp
    Transient(
        current,
        modifier
            .windowInsetsPadding(safeInsets.only(WindowInsetsSides.Horizontal))
            .padding(
                start = start.takeIf { it > 0.dp } ?: 12.dp,
                end = end.takeIf { it > 0.dp } ?: 12.dp,
                bottom = 12.dp + LocalIslandClearance.current + if (keys > 0.dp) keys + 8.dp else 0.dp,
            ),
        scaleFrom = 1f,
        rise = 16.dp,
    ) { notice ->
        IslandNotice(
            noticeText(notice),
            onUndo = when {
                notice is Notice.SaveFailed -> actions::retrySave
                notice.undoable -> actions::undoNotice
                else -> null
            },
            onDismiss = actions::dismissNotice,
            onOpen = if (notice.opens != null) ({ actions.openNotice(); onOpened() }) else null,
            actionLabel = if (notice is Notice.SaveFailed) stringResource(Res.string.notice_retry) else null,
            onHeld = { held = it },
        )
    }
}

/**
 * How long [notice] stays, before the platform's accessibility timeout stretches it; null
 * for never. One that only tells: 4 s. A hint: as long as its [length] takes to read
 * (60 ms a character), 4 to 10 s. One with an action (Undo, Open): 10 s. A failed save
 * keeps its Retry until it is taken or put away. With a [screenReader] running, one with
 * an action stays, and one that only tells gets at least 10 s.
 */
internal fun noticeTimeout(notice: Notice, length: Int, screenReader: Boolean): Long? {
    if (notice is Notice.SaveFailed) return null
    val withAction = notice.undoable || notice.opens != null
    if (withAction && screenReader) return null
    val base = when {
        withAction -> NoticeLongMs
        // A how-to line stays as long as it takes to read.
        notice is Notice.CompareHint || notice is Notice.RollHint || notice is Notice.RailHint || (notice is Notice.Saved && notice.review) ->
            (length * 60L).coerceIn(NoticeShortMs, NoticeLongMs)
        else -> NoticeShortMs
    }
    return if (screenReader) maxOf(base, NoticeLongMs) else base
}

/** Material's short and long snackbar durations. */
private const val NoticeShortMs = 4_000L
private const val NoticeLongMs = 10_000L

@Composable
internal fun noticeText(notice: Notice): String = when (notice) {
    is Notice.Removed -> stringResource(Res.string.notice_removed, notice.name)
    is Notice.Cleared -> stringResource(Res.string.notice_cleared, notice.slot)
    is Notice.Pasted -> stringResource(Res.string.notice_pasted, notice.slot)
    is Notice.Deleted -> stringResource(Res.string.notice_deleted, notice.name)
    is Notice.Copied -> stringResource(Res.string.notice_copied, notice.name)
    is Notice.CopiedTo -> stringResource(Res.string.notice_copied_to, notice.library)
    is Notice.Full -> pluralStringResource(Res.plurals.notice_full, notice.max, notice.library, notice.max)
    is Notice.Exported -> stringResource(Res.string.notice_exported, notice.library)
    is Notice.ExportLost -> stringResource(Res.string.notice_export_lost, notice.file)
    is Notice.CannotRead -> stringResource(Res.string.notice_cannot_read, notice.name)
    is Notice.CannotWrite -> stringResource(Res.string.notice_cannot_write, notice.name)
    is Notice.Failed -> notice.reason
    is Notice.SaveFailed -> stringResource(Res.string.notice_save_failed, notice.library, notice.reason.trimEnd('.'))
    is Notice.Added -> stringResource(Res.string.notice_added, notice.slot)
    is Notice.Kept -> stringResource(Res.string.notice_kept, notice.library)
    is Notice.RollHint -> stringResource(Res.string.notice_roll_hint, notice.stage)
    is Notice.Saved -> pluralStringResource(Res.plurals.notice_saved, notice.count, notice.count).let {
        if (notice.review) stringResource(Res.string.notice_saved_review, it) else it
    }
    Notice.RailHint -> stringResource(Res.string.notice_rail_hint)
    is Notice.Rolled -> stringResource(Res.string.notice_rolled, notice.stage)
    is Notice.SnapshotReplaced -> stringResource(Res.string.notice_snapshot_replaced, notice.letter)
    Notice.CompareHint -> stringResource(Res.string.notice_compare_hint)
}

// Sample state for the previews and the device checks: plain values, no model, no core calls.

private fun line(id: String, name: String, depth: Int = 0, folder: Boolean = false, programs: Int? = null, open: Boolean = true, first: Boolean = false, last: Boolean = false) =
    OutlineLine(
        depth.toUInt(), if (folder) NodeKind.FOLDER else NodeKind.LIBRARY, id, name, open, shown = true, first = first, last = last,
        driverId = if (folder) null else "korg-minilogue-xd", programs = programs?.toUInt(), broken = null,
    )

val SampleLines = listOf(
    line("live", "Live", folder = true, first = true),
    line("gig", "Gig set", depth = 1, programs = 14, first = true),
    line("encore", "Encore", depth = 1, folder = true, last = true),
    line("pads", "Pads and drones", depth = 2, programs = 12, first = true, last = true),
    line("sketch", "Sketches", programs = 0),
    line("studio", "Studio", folder = true, open = false, last = true),
)

private val Names = listOf("Velvet Pad", "Glass Bells", "Mono Bass", "Brass Stab", "Night Drive", "Dust Lead", "Wide Strings", "Pluck Echo")

val SampleRows = Names.mapIndexed { i, name ->
    ProgramRow(slot = (i + 1).toString().padStart(3, '0'), name = name, edited = i == 1 || i == 4, readOnly = false, loaded = true)
}

val SampleLibrary = ProgramsUi(librarySource("gig"), "Gig set", SampleRows, selected = 1, inLibrary = true, canUndo = true, canRedo = false)

/** A Prophet Rev2 being read: user banks, then factory slots (read-only). */
val SampleSynth = ProgramsUi(
    synthSource(1u),
    "Prophet Rev2",
    Names.take(4).mapIndexed { i, n -> ProgramRow("U1 ${(i + 1).toString().padStart(3, '0')}", n, edited = i == 2, readOnly = false, loaded = true) } +
        listOf(ProgramRow("U1 005", "", edited = false, readOnly = false, loaded = false)) +
        Names.drop(4).mapIndexed { i, n -> ProgramRow("F1 ${(i + 1).toString().padStart(3, '0')}", n, edited = false, readOnly = true, loaded = true) },
    selected = null,
    inLibrary = false,
    canUndo = false,
    canRedo = false,
    read = ReadProgress(412, 1024),
)

private val SampleSynthCard = SynthCard(1u, "Prophet Rev2", firmware = null, simulated = true, port = "", read = ReadProgress(412, 1024))

val SampleUi = LibraryUi(synths = listOf(SampleSynthCard), lines = SampleLines, unsaved = setOf("pads"), programs = SampleLibrary)

/** The previews' rows carry the sample programs' pictures. */
private val PreviewLibraryActions = object : LibraryActions {
    val pictures = samplePanel().let { p -> listOf(1, 3, 4).mapNotNull { p.graph(it)?.now } }
    override fun thumbnail(index: Int) = pictures[index % pictures.size]
}

@Composable
private fun LibraryPreview(mode: OscylThemeMode = OscylThemeMode.Dark, ui: LibraryUi = SampleUi) = OscylTheme(mode) { LibraryPreviewBody(ui) }

@Composable
private fun LibraryPreviewBody(ui: LibraryUi) {
    val status = SynthStatus(ui.synths.firstOrNull()?.model, reading = ui.synths.firstOrNull()?.read?.let { it.done.toFloat() / it.total })
    CompositionLocalProvider(LocalSynthStatus provides status) {
        // What the app hosts: the notice over the destination, the key counting the list's edits.
        Shell(Destination.Library, onSelect = {}, saveCount = ui.programs?.rows?.count { it.edited } ?: 0, onSave = {}, sync = ui.programs?.inLibrary == false) { modifier ->
            Box(modifier) {
                LibraryScreen(ui, PreviewLibraryActions, onOpenSynth = {})
                NoticeHost(ui, PreviewLibraryActions, Modifier.align(Alignment.BottomCenter))
            }
        }
    }
}

@OscylPreviews
@Composable
private fun library_sources(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = LibraryPreview(mode, SampleUi.copy(programs = null))

@OscylPreviews
@Composable
private fun library_programs(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = LibraryPreview(mode)

@OscylPreviews
@Composable
private fun library_synth(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = LibraryPreview(mode, SampleUi.copy(programs = SampleSynth))

@OscylPreviews
@Composable
private fun library_empty(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    LibraryPreview(mode, LibraryUi(programs = null))

@OscylPreviews
@Composable
private fun library_emptyLibrary(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    LibraryPreview(mode, SampleUi.copy(programs = SampleLibrary.copy(source = librarySource("sketch"), title = "Sketches", rows = emptyList(), selected = null, canUndo = false)))

@OscylPreviews
@Composable
private fun library_notice(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) =
    LibraryPreview(mode, SampleUi.copy(notice = Notice.Removed("Brass Stab")))

@OscylSizePreviews
@Composable
private fun library() = LibraryPreview()

@OscylA11yPreviews
@Composable
private fun library_a11y() = LibraryPreview()

@OscylA11yPreviews
@Composable
private fun library_sources_a11y() = LibraryPreview(ui = SampleUi.copy(programs = null))

@Preview(name = "cutout", device = PHONE)
@Composable
private fun library_cutout(@PreviewParameter(PortraitCutouts::class) cutout: FakeCutout) = OscylTheme(OscylThemeMode.Dark) {
    CutoutFrame(cutout) { LibraryPreviewBody(SampleUi.copy(programs = null)) }
}

@Preview(name = "cutout", device = PHONE_LANDSCAPE)
@Composable
private fun library_cutout_landscape(@PreviewParameter(LandscapeCutouts::class) cutout: FakeCutout) = OscylTheme(OscylThemeMode.Dark) {
    CutoutFrame(cutout) { LibraryPreviewBody(SampleUi.copy(programs = null)) }
}

/** No libraries yet and no answer: the role hint under the empty state. */
@OscylPreviews
@Composable
private fun library_roleHint(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = OscylTheme(mode) {
    Shell(Destination.Library, onSelect = {}, saveCount = 0, onSave = {}) { modifier ->
        LibraryScreen(LibraryUi(programs = null), NoLibraryActions, onOpenSynth = {}, modifier, roleHint = { RoleHint({}) })
    }
}

