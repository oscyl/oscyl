package io.github.oscyl.ui.welcome

import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.BoxWithConstraints
import io.github.oscyl.shared.generated.resources.first_not_now
import androidx.compose.ui.platform.testTag
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.app_name
import io.github.oscyl.shared.generated.resources.first_found
import io.github.oscyl.shared.generated.resources.first_privacy
import io.github.oscyl.shared.generated.resources.first_purpose
import io.github.oscyl.shared.generated.resources.first_read
import io.github.oscyl.shared.generated.resources.first_read_body
import io.github.oscyl.shared.generated.resources.first_skip
import io.github.oscyl.shared.generated.resources.first_try
import io.github.oscyl.shared.generated.resources.synth_empty_body
import io.github.oscyl.shared.generated.resources.synth_empty_title
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.generate.Primary
import io.github.oscyl.ui.parts.Lamp
import io.github.oscyl.ui.parts.LocalOscylMotion
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.floating
import io.github.oscyl.ui.parts.motionScale
import io.github.oscyl.ui.parts.safeInsets
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.OscylSizePreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.synth.SynthCard
import kotlin.math.PI
import kotlin.math.roundToInt
import kotlin.math.sin
import org.jetbrains.compose.resources.stringResource

/**
 * The first run, one screen (DESIGN.md "First run"): the oscillator line draws itself,
 * then "Oscyl", one line of purpose, and a listening card, "Plug in your synth", its
 * lamp's ring pulsing on the slow curve. When a real synth answers, the card lifts,
 * the lamp lights and it offers to read its programs ([onRead]). "No synth here? Try
 * one" opens the picker ([onTry]); [onSkip] starts with nothing. It shows once.
 */
@Composable
fun WelcomeScreen(
    cards: List<SynthCard>,
    onRead: (SynthCard) -> Unit,
    onTry: () -> Unit,
    onSkip: () -> Unit,
    modifier: Modifier = Modifier,
    /** The listening card's shared end: as the app takes over, it flies into the chip. */
    card: Modifier = Modifier,
) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val found = cards.firstOrNull { it.connected && !it.simulated }
    val heading = @Composable {
        BasicText(
            stringResource(Res.string.app_name),
            Modifier.semantics { heading() },
            style = OscylTheme.type.programTitle.copy(color = c.ink),
        )
        BasicText(stringResource(Res.string.first_purpose), style = type.bodyLarge.copy(color = c.legend, textDirection = TextDirection.Content))
    }
    val rest = @Composable {
        ListeningCard(found, onRead, card)
        if (found == null) {
            OutlinedAction(stringResource(Res.string.first_try), onTry, Modifier.testTag("first-try"))
            // Without a synth the way on is the page's one plate, as Read its programs is with one.
            Box(Modifier.testTag("first-skip")) { Primary(stringResource(Res.string.first_skip), enabled = true, onClick = onSkip) }
        } else {
            // Found, there is still a way on without reading (plugged in only to charge, or to look first).
            OutlinedAction(stringResource(Res.string.first_not_now), onSkip, Modifier.testTag("first-skip"))
        }
        BasicText(stringResource(Res.string.first_privacy), style = type.bodySmall.copy(color = c.legend, textDirection = TextDirection.Content))
    }
    BoxWithConstraints(modifier.fillMaxSize().windowInsetsPadding(safeInsets)) {
        if (maxHeight < 480.dp && maxWidth > maxHeight) {
            // A phone on its side: the name and purpose at the start, the card and the ways on
            // beside them, so an action is in sight without scrolling.
            Row(
                Modifier.fillMaxSize().padding(horizontal = 24.dp, vertical = 16.dp),
                horizontalArrangement = Arrangement.spacedBy(32.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OscillatorLine(Modifier.fillMaxWidth().height(48.dp))
                    heading()
                }
                Column(
                    Modifier.weight(1f).fillMaxHeight().verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
                ) { rest() }
            }
        } else {
            Column(
                Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 16.dp, vertical = 32.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(20.dp, Alignment.CenterVertically),
            ) {
                Column(Modifier.widthIn(max = 480.dp).fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(20.dp)) {
                    OscillatorLine(Modifier.fillMaxWidth().height(72.dp))
                    heading()
                    rest()
                }
            }
        }
    }
}

/**
 * Plug in your synth, listening: a lamp with a ring that pulses outward on the slow
 * curve. Found ([found]), the card lifts onto the floating shadow, the lamp lights and
 * Read its programs takes the place of the pulse.
 */
@Composable
private fun ListeningCard(found: SynthCard?, onRead: (SynthCard) -> Unit, modifier: Modifier) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val motion = LocalOscylMotion.current
    val shape = RoundedCornerShape(OscylTokens.radiusPanel)
    val pulse = rememberInfiniteTransition(label = "listening")
    val ring by pulse.animateFloat(0f, 1f, infiniteRepeatable(tween(1600, easing = OscylTokens.easing), RepeatMode.Restart), label = "ring")
    Column(
        modifier
            .fillMaxWidth()
            .then(if (found != null) Modifier.floating(c, shape) else Modifier.border(OscylTokens.ruleWidth, c.ruleStrong, shape))
            .padding(16.dp)
            .semantics { liveRegion = LiveRegionMode.Polite },
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(14.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(24.dp), contentAlignment = Alignment.Center) {
                if (found == null && motion) {
                    Canvas(Modifier.size(24.dp)) {
                        drawCircle(c.legend.copy(alpha = (1f - ring) * 0.6f), radius = 4.dp.toPx() + ring * 8.dp.toPx(), style = Stroke(1.dp.toPx()))
                    }
                }
                Lamp(found != null)
            }
            Column {
                BasicText(
                    found?.let { stringResource(Res.string.first_found) + " · " + it.model } ?: stringResource(Res.string.synth_empty_title),
                    style = type.titleMedium.copy(color = c.ink, textDirection = TextDirection.Content),
                )
                BasicText(
                    stringResource(if (found != null) Res.string.first_read_body else Res.string.synth_empty_body),
                    style = type.bodyMedium.copy(color = c.legend, textDirection = TextDirection.Content),
                )
            }
        }
        if (found != null) Primary(stringResource(Res.string.first_read), enabled = true) { onRead(found) }
    }
}

/** The oscillator line drawing itself left to right over `slow` × 4, once; whole at once with motion off. */
@Composable
private fun OscillatorLine(modifier: Modifier) {
    val c = OscylTheme.colors
    val motion = LocalOscylMotion.current
    val drawn = remember { Animatable(if (motion) 0f else 1f) }
    LaunchedEffect(motion) {
        val scale = motionScale(motion)
        if (scale == 0f) drawn.snapTo(1f) else drawn.animateTo(1f, tween((OscylTokens.slowMs * 4 * scale).roundToInt(), easing = OscylTokens.easing))
    }
    val path = remember { Path() }
    Canvas(modifier.graphicsLayer {}) {
        path.rewind()
        val mid = size.height / 2
        val amp = size.height * 0.38f
        val steps = 96
        for (i in 0..steps) {
            val x = size.width * i / steps
            // A saw softening into a sine: the app's mark, an oscillator.
            val t = i / steps.toFloat()
            val y = mid - amp * sin(2 * PI.toFloat() * 2 * t) * (0.6f + 0.4f * t)
            if (i == 0) path.moveTo(x, y) else path.lineTo(x, y)
        }
        clipRect(right = size.width * drawn.value) {
            drawPath(path, c.ink, style = Stroke(2.dp.toPx(), cap = StrokeCap.Round))
        }
        drawLine(c.rule, Offset(0f, mid), Offset(size.width, mid), 1.dp.toPx())
    }
}

private val Found = SynthCard(1u, "prologue", firmware = "2.10", simulated = false, port = "9:prologue _ SOUND")

@OscylPreviews
@Composable
private fun welcome(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) { WelcomeScreen(emptyList(), {}, {}, {}) }

@OscylPreviews
@Composable
private fun welcome_found(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) { WelcomeScreen(listOf(Found), {}, {}, {}) }

@OscylSizePreviews
@Composable
private fun welcome_sizes() = PreviewTheme { WelcomeScreen(emptyList(), {}, {}, {}) }

@OscylA11yPreviews
@Composable
private fun welcome_a11y() = PreviewTheme { WelcomeScreen(listOf(Found), {}, {}, {}) }
