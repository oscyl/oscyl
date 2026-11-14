package io.github.oscyl.ui.shell

import io.github.oscyl.platform.imeHeading
import io.github.oscyl.ui.parts.FloatingRadius
import io.github.oscyl.ui.parts.Transient
import io.github.oscyl.ui.parts.transientAlpha
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.ic_close
import io.github.oscyl.shared.generated.resources.ic_search
import io.github.oscyl.shared.generated.resources.search_all
import io.github.oscyl.shared.generated.resources.search_clear_text
import io.github.oscyl.shared.generated.resources.search_close
import io.github.oscyl.shared.generated.resources.search_done
import io.github.oscyl.shared.generated.resources.search_edited
import io.github.oscyl.shared.generated.resources.search_factory
import io.github.oscyl.shared.generated.resources.search_programs
import io.github.oscyl.shared.generated.resources.search_user
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.IconAction
import io.github.oscyl.ui.parts.Segments
import io.github.oscyl.ui.parts.floating
import io.github.oscyl.ui.parts.touchStates
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.stringResource

/** Which programs a search keeps, besides the words. */
enum class SearchFilter { All, Edited, User, Factory }

/**
 * Search in the program list, owned by the shell: the island turns into its field
 * (DESIGN.md "Search"). The program list says when it is in sight ([available]) and
 * whether it has factory slots; it reads [query] and [filter] to filter its rows.
 */
@Stable
class ProgramSearch {
    /** A program list is in sight: the island carries the search key. */
    var available by mutableStateOf(false)

    /** That list is a long one (a synth's slots): the key nods once, the first time. */
    var long by mutableStateOf(false)

    /** The island is the search field. */
    var active by mutableStateOf(false)
    var query by mutableStateOf("")
    var filter by mutableStateOf(SearchFilter.All)

    /** The list holds factory slots (a synth's): User and Factory are offered. */
    var factory by mutableStateOf(false)

    /** The floating island carries the key and the field; beside a side island the list does. */
    var hosted by mutableStateOf(false)

    /** The source searched: another one starts a fresh search. */
    var source: Any? = null

    /** The field takes focus (and the keyboard) once, as it opens; not on every return. */
    var focusPending by mutableStateOf(false)

    fun open() {
        active = true
        focusPending = true
    }

    fun close() {
        active = false
        query = ""
        filter = SearchFilter.All
    }

    companion object {
        val Saver = Saver<ProgramSearch, List<Any>>(
            save = { listOf(it.active, it.query, it.filter.ordinal) },
            restore = { s -> ProgramSearch().apply { active = s[0] as Boolean; query = s[1] as String; filter = SearchFilter.entries[s[2] as Int] } },
        )
    }
}

/** The shell's search; without a shell (previews, tests) the list keeps one of its own. */
val LocalProgramSearch = staticCompositionLocalOf<ProgramSearch?> { null }

/** The search key at the island's start: a floating part of the island's make, 48 dp wide. */
@Composable
internal fun SearchKey(search: ProgramSearch, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    val interaction = remember { MutableInteractionSource() }
    val label = stringResource(Res.string.search_programs)
    val haptics = io.github.oscyl.platform.LocalOscylHaptics.current
    // In sight of a long list for the first time, the glass nods once (a layer turning:
    // nothing is measured), and that is all that is ever said about it.
    val teaching = io.github.oscyl.ui.preferences.LocalTeaching.current
    val motion = io.github.oscyl.ui.parts.LocalOscylMotion.current
    val nod = remember { androidx.compose.animation.core.Animatable(0f) }
    androidx.compose.runtime.LaunchedEffect(search.long) {
        if (!search.long) return@LaunchedEffect
        teaching.once(io.github.oscyl.ui.preferences.Hints.SEARCH) {
            if (io.github.oscyl.ui.parts.motionScale(motion) != 0f) {
                nod.animateTo(-18f, io.github.oscyl.ui.parts.plateSpring(0.01f))
                nod.animateTo(0f, io.github.oscyl.ui.parts.plateSpring(0.01f))
            }
        }
    }
    Box(
        modifier
            .width(48.dp)
            .floating(c)
            .clickable(interaction, indication = null, role = Role.Button) { haptics.tick(); teaching.teach(io.github.oscyl.ui.preferences.Hints.SEARCH); search.open() }
            .semantics { contentDescription = label }
            .touchStates(interaction, androidx.compose.foundation.shape.RoundedCornerShape(14.dp), c.press)
            .testTag("search-open"),
        contentAlignment = Alignment.Center,
    ) {
        Image(painterResource(Res.drawable.ic_search), null, Modifier.size(22.dp).graphicsLayer { rotationZ = nod.value }, colorFilter = ColorFilter.tint(c.ink))
    }
}

/**
 * The island as the search field: the whole width, the glyph, the field, while the
 * keyboard is up "Done" (it puts the keyboard away and keeps the filter; its place is kept
 * when hidden), and a trailing ✕ (clears the text; with none, closes the search). Above
 * it while typing, the filters (All · Edited, and User · Factory for a synth's slots) on a
 * plate as tall as the field. Focused as it opens.
 */
@Composable
internal fun SearchIsland(search: ProgramSearch, modifier: Modifier = Modifier) = ChromeScale {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val keyboard = LocalSoftwareKeyboardController.current
    // Where the keyboard is going, not where it is: the filters and Done fade as it starts
    // to close, not once it has gone.
    val typing = imeHeading()
    val done by transientAlpha(typing)
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) {
        if (search.focusPending) {
            search.focusPending = false
            focus.requestFocus()
        }
    }
    val hint = stringResource(Res.string.search_programs)
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        // Arriving and leaving as every transient part at the foot does; the field below
        // stays where it stands.
        Transient(Unit.takeIf { typing }, scaleFrom = 1f, rise = 16.dp) {
            val options = buildList {
                add(SearchFilter.All to stringResource(Res.string.search_all))
                add(SearchFilter.Edited to stringResource(Res.string.search_edited))
                if (search.factory) {
                    add(SearchFilter.User to stringResource(Res.string.search_user))
                    add(SearchFilter.Factory to stringResource(Res.string.search_factory))
                }
            }
            val at = options.indexOfFirst { it.first == search.filter }.coerceAtLeast(0)
            // As tall as the field (the island's height), the chosen plate's corners concentric with the part's.
            Segments(
                options.map { it.second }, at, at, onSelect = { search.filter = options[it].first },
                modifier = Modifier.fillMaxWidth().floating(c).padding(PlateGap),
                index = false, framed = false,
                innerRadius = FloatingRadius - PlateGap,
                optionHeight = 64.dp - PlateGap * 2,
            )
        }
        Row(
            Modifier.fillMaxWidth().heightIn(min = 64.dp).floating(c).padding(start = 16.dp, end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Image(painterResource(Res.drawable.ic_search), null, Modifier.size(20.dp), colorFilter = ColorFilter.tint(c.ink))
            BasicTextField(
                search.query,
                { search.query = it },
                Modifier.weight(1f).focusRequester(focus).testTag("search-field").semantics { contentDescription = hint },
                textStyle = type.bodyLarge.copy(color = c.ink, textDirection = TextDirection.Content),
                singleLine = true,
                cursorBrush = SolidColor(c.ink),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { keyboard?.hide() }),
                decorationBox = { inner ->
                    Box {
                        if (search.query.isEmpty()) BasicText(hint, style = type.bodyLarge.copy(color = c.legend), maxLines = 1, overflow = TextOverflow.Ellipsis)
                        inner()
                    }
                },
            )
            // "Done": the keyboard goes, the filter stays. Always laid out (the field never
            // widens or narrows as the keyboard comes and goes), drawn and said only while typing; the ✕ keeps the end.
            Box(
                Modifier
                    .heightIn(min = 48.dp)
                    .graphicsLayer { alpha = done }
                    .then(
                        if (typing) Modifier.clickable(role = Role.Button) { keyboard?.hide() } else Modifier.clearAndSetSemantics {},
                    )
                    .drawBehind { drawLine(c.rule, Offset(0f, size.height * 0.2f), Offset(0f, size.height * 0.8f), OscylTokens.ruleWidth.toPx()) }
                    .padding(horizontal = 14.dp),
                contentAlignment = Alignment.Center,
            ) {
                BasicText(stringResource(Res.string.search_done), style = type.labelLarge.copy(color = c.ink, fontWeight = FontWeight.SemiBold))
            }
            if (search.query.isNotEmpty()) {
                IconAction(Res.drawable.ic_close, stringResource(Res.string.search_clear_text), { search.query = "" }, Modifier.testTag("search-clear"))
            } else {
                IconAction(Res.drawable.ic_close, stringResource(Res.string.search_close), search::close, Modifier.testTag("search-close"))
            }
        }
    }
}

/** Between a floating part's rim and the plates inside it. */
private val PlateGap = 6.dp
