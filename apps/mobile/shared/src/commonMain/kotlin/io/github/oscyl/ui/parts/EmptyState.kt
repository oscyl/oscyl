package io.github.oscyl.ui.parts

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PreviewTheme
import io.github.oscyl.ui.preview.ThemeModes

/**
 * The app's line drawings, in the front panel's hand: a panel at rest, a plug before its
 * socket, a shelf with a gap (the empty states), and a knob on its scale, a sounding
 * trace, a hand of tiles, a sync and a unit in its slot (the guide's chapters).
 */
enum class EmptyArt { Panel, Cable, Shelf, Knob, Trace, Tiles, Sync, Chip }

/**
 * A designed empty state (DESIGN.md "Empty states"): a line drawing in the panel's
 * hand ([art]; `rule-strong` outlines, `legend` parts, a dotted `rule` for what is
 * missing, no colour), the [title], a line of [body] and the one next step ([action]),
 * centred at a reading width. Silent drawing, spoken words.
 */
@Composable
fun EmptyState(title: String, body: String, modifier: Modifier = Modifier, art: EmptyArt = EmptyArt.Shelf, action: (@Composable () -> Unit)? = null) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    Column(
        modifier.widthIn(max = 400.dp).fillMaxWidth().padding(horizontal = 24.dp, vertical = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        LineArt(art, Modifier.padding(bottom = 8.dp))
        BasicText(
            title,
            Modifier.semantics { heading() },
            style = type.titleMedium.copy(color = c.ink, fontWeight = FontWeight.SemiBold, textAlign = TextAlign.Center, textDirection = TextDirection.Content),
        )
        BasicText(body, style = type.bodyLarge.copy(color = c.legend, textAlign = TextAlign.Center, textDirection = TextDirection.Content))
        if (action != null) Box(Modifier.padding(top = 4.dp)) { action() }
    }
}

/**
 * One line drawing, 120 by 64 dp: `rule-strong` outlines, `legend` parts, a dotted
 * `rule` for what is missing or on its way; no colour. Silent.
 */
@Composable
fun LineArt(art: EmptyArt, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    Canvas(modifier.width(120.dp).height(64.dp)) {
            val u = 1.dp.toPx()
            val line = Stroke(1.5f * u, cap = StrokeCap.Round, join = StrokeJoin.Round)
            val dots = Stroke(1.5f * u, cap = StrokeCap.Round, pathEffect = PathEffect.dashPathEffect(floatArrayOf(0.1f * u, 4f * u)))
            fun box(color: Color, x: Float, y: Float, w: Float, h: Float, r: Float, style: Stroke = line) =
                drawRoundRect(color, Offset(x * u, y * u), Size(w * u, h * u), CornerRadius(r * u), style)
            fun stroke(color: Color, x1: Float, y1: Float, x2: Float, y2: Float, style: Stroke = line) =
                drawLine(color, Offset(x1 * u, y1 * u), Offset(x2 * u, y2 * u), style.width, style.cap, style.pathEffect)
            fun knob(x: Float, y: Float, r: Float) {
                drawCircle(c.legend, r * u, Offset(x * u, y * u), style = line)
                stroke(c.legend, x, y, x, y - r * 0.55f)
            }
            when (art) {
                // The panel at rest: its display's flat trace on a dotted rule, its knobs at noon.
                EmptyArt.Panel -> {
                    box(c.ruleStrong, 1f, 1f, 118f, 62f, 8f)
                    box(c.legend, 12f, 14f, 52f, 36f, 3f)
                    stroke(c.rule, 19f, 32f, 57f, 32f, dots)
                    stroke(c.legend, 24f, 32f, 44f, 32f)
                    knob(82f, 24f, 9f)
                    knob(105f, 24f, 9f)
                    knob(82f, 46f, 5f)
                    knob(105f, 46f, 5f)
                }
                // The plug on its cable, a dotted step from the synth's socket; the lamp unlit.
                EmptyArt.Cable -> {
                    box(c.ruleStrong, 78f, 8f, 41f, 48f, 8f)
                    box(c.legend, 88f, 24f, 11f, 16f, 2f)
                    drawCircle(c.legend, 3f * u, Offset(108f * u, 19f * u), style = line)
                    box(c.legend, 40f, 24f, 16f, 16f, 3f)
                    stroke(c.legend, 56f, 29f, 62f, 29f)
                    stroke(c.legend, 56f, 35f, 62f, 35f)
                    val cable = Path().apply {
                        moveTo(40f * u, 32f * u)
                        cubicTo(22f * u, 32f * u, 24f * u, 52f * u, 2f * u, 52f * u)
                    }
                    drawPath(cable, c.legend, style = line)
                    stroke(c.rule, 68f, 32f, 84f, 32f, dots)
                }
                // A shelf of spines, one place dotted: what is still to come.
                EmptyArt.Shelf -> {
                    stroke(c.ruleStrong, 8f, 57f, 112f, 57f)
                    box(c.legend, 26f, 22f, 11f, 35f, 2f)
                    box(c.legend, 40f, 14f, 11f, 43f, 2f)
                    box(c.rule, 54f, 20f, 11f, 37f, 2f, dots)
                    box(c.legend, 68f, 28f, 11f, 29f, 2f)
                    box(c.legend, 82f, 18f, 11f, 39f, 2f)
                }
                // A knob on its printed scale, the index at two o'clock.
                EmptyArt.Knob -> {
                    val (cx, cy) = 60f to 34f
                    for (i in 0..10) {
                        val ang = (135.0 + 27.0 * i) * kotlin.math.PI / 180.0
                        val (ca, sa) = kotlin.math.cos(ang).toFloat() to kotlin.math.sin(ang).toFloat()
                        stroke(c.ruleStrong, cx + 25f * ca, cy + 25f * sa, cx + (if (i % 5 == 0) 31f else 29f) * ca, cy + (if (i % 5 == 0) 31f else 29f) * sa)
                    }
                    drawCircle(c.legend, 20f * u, Offset(cx * u, cy * u), style = line)
                    stroke(c.legend, cx + 5f, cy - 6f, cx + 13f, cy - 14f)
                }
                // A sounding trace across the display.
                EmptyArt.Trace -> {
                    box(c.ruleStrong, 6f, 8f, 108f, 48f, 6f)
                    stroke(c.rule, 14f, 32f, 106f, 32f, dots)
                    val wave = Path().apply {
                        moveTo(14f * u, 32f * u)
                        for (k in 0 until 4) {
                            val x = 14f + 23f * k
                            cubicTo((x + 5.75f) * u, (if (k % 2 == 0) 12f else 52f) * u, (x + 17.25f) * u, (if (k % 2 == 0) 12f else 52f) * u, (x + 23f) * u, 32f * u)
                        }
                    }
                    drawPath(wave, c.legend, style = line)
                }
                // A hand of tiles, each with its own shape.
                EmptyArt.Tiles -> {
                    for (k in 0 until 3) {
                        val x = 8f + 37f * k
                        box(if (k == 1) c.legend else c.ruleStrong, x, 12f, 30f, 40f, 4f)
                        val w = Path().apply {
                            moveTo((x + 5f) * u, 30f * u)
                            val n = k + 1
                            for (j in 0 until n) {
                                val seg = 20f / n
                                val x0 = x + 5f + seg * j
                                quadraticTo((x0 + seg / 4) * u, 20f * u, (x0 + seg / 2) * u, 30f * u)
                                quadraticTo((x0 + seg * 3 / 4) * u, 40f * u, (x0 + seg) * u, 30f * u)
                            }
                        }
                        drawPath(w, c.legend, style = line)
                    }
                }
                // The phone's library and the synth, and the way between them.
                EmptyArt.Sync -> {
                    box(c.ruleStrong, 4f, 14f, 36f, 36f, 6f)
                    stroke(c.legend, 12f, 24f, 32f, 24f)
                    stroke(c.legend, 12f, 32f, 32f, 32f)
                    stroke(c.legend, 12f, 40f, 24f, 40f)
                    box(c.ruleStrong, 80f, 14f, 36f, 36f, 6f)
                    drawCircle(c.legend, 6f * u, Offset(98f * u, 28f * u), style = line)
                    stroke(c.legend, 88f, 42f, 108f, 42f)
                    stroke(c.legend, 48f, 26f, 72f, 26f)
                    stroke(c.legend, 67f, 21f, 72f, 26f)
                    stroke(c.legend, 67f, 31f, 72f, 26f)
                    stroke(c.rule, 48f, 40f, 72f, 40f, dots)
                }
                // A unit over its slot.
                EmptyArt.Chip -> {
                    box(c.legend, 46f, 10f, 28f, 28f, 3f)
                    for (k in 0 until 3) {
                        stroke(c.legend, 53f + 7f * k, 5f, 53f + 7f * k, 10f)
                        stroke(c.legend, 41f, 17f + 7f * k, 46f, 17f + 7f * k)
                        stroke(c.legend, 74f, 17f + 7f * k, 79f, 17f + 7f * k)
                    }
                    box(c.rule, 40f, 46f, 40f, 12f, 3f, dots)
                    stroke(c.ruleStrong, 24f, 58f, 96f, 58f)
                }
            }
        }
}

/**
 * Rows still loading: [rows] quiet bars where the names will be, in `rule`, at the
 * rows' own height, so nothing moves when the real rows arrive.
 */
@Composable
fun SkeletonRows(rows: Int, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    Column(modifier.fillMaxWidth().padding(horizontal = 8.dp)) {
        repeat(rows) { i ->
            Row(Modifier.fillMaxWidth().height(48.dp).padding(horizontal = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Box(Modifier.width(28.dp).height(10.dp).background(c.rule, RoundedCornerShape(OscylTokens.radiusInner)))
                Box(Modifier.width((90 + (i * 37) % 80).dp).height(12.dp).background(c.rule, RoundedCornerShape(OscylTokens.radiusInner)))
            }
        }
    }
}

@OscylPreviews
@Composable
private fun emptyState(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = PreviewTheme(mode) {
    Column {
        EmptyState("No programs yet", "Make one, or paste one copied from another library.") { OutlinedAction("New program", {}) }
        EmptyState("No program open", "Pick a program in Library to shape it here.", art = EmptyArt.Panel)
        EmptyState("Plug in your synth over USB", "Oscyl finds it by itself.", art = EmptyArt.Cable)
        SkeletonRows(4)
    }
}

