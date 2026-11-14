package io.github.oscyl.ui.about

import io.github.oscyl.ui.parts.safeInsets
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.intl.Locale
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.toUpperCase
import androidx.compose.ui.text.withLink
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.unit.dp
import io.github.oscyl.ui.parts.BackHeader
import io.github.oscyl.shared.generated.resources.Res
import io.github.oscyl.shared.generated.resources.about_notice_caption
import io.github.oscyl.shared.generated.resources.about_own
import io.github.oscyl.shared.generated.resources.about_third_party
import io.github.oscyl.shared.generated.resources.about_third_party_body
import io.github.oscyl.shared.generated.resources.about_title
import io.github.oscyl.shared.generated.resources.about_uniffi
import io.github.oscyl.theme.OscylTheme
import io.github.oscyl.theme.OscylThemeMode
import io.github.oscyl.theme.OscylTokens
import io.github.oscyl.ui.parts.OutlinedAction
import io.github.oscyl.ui.parts.touchStates
import io.github.oscyl.ui.preview.OscylA11yPreviews
import io.github.oscyl.ui.preview.OscylPreviews
import io.github.oscyl.ui.preview.PHONE
import io.github.oscyl.ui.preview.ThemeModes
import io.github.oscyl.ui.shell.Destination
import io.github.oscyl.ui.shell.LocalIslandClearance
import io.github.oscyl.ui.shell.Shell
import org.jetbrains.compose.resources.stringResource

/** Oscyl's own licence, as a notice whose page shows both texts. */
const val OscylNoticeId = "Oscyl"

/** The About page over the shipped notices: whole on its first frame once [preloadLicences] has run. */
@Composable
fun AboutScreen(onBack: () -> Unit, onOpen: (Notice) -> Unit, modifier: Modifier = Modifier) {
    val notices by produceState(loadedNotices().orEmpty()) { value = loadNotices() }
    AboutScreen(notices, onBack, onOpen, modifier)
}

/**
 * About Oscyl: its own licence (MIT or Apache 2.0, either), UniFFI's MPL-2.0 notice
 * with the source's address, then every piece of third-party software, one row each
 * (name; version and licence), each opening its licence text.
 */
@Composable
fun AboutScreen(notices: List<Notice>, onBack: () -> Unit, onOpen: (Notice) -> Unit, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val own = remember { Notice(OscylNoticeId, "Oscyl", null, "MIT OR Apache-2.0", OscylSource, emptyList()) }
    LazyColumn(
        modifier.fillMaxSize().windowInsetsPadding(barInsets),
        contentPadding = pagePadding(),
    ) {
        item {
            Column(Modifier.widthIn(max = 560.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                // The back arrow's 48 dp target sits in the gutter, its glyph on the text's edge.
                BackHeader(stringResource(Res.string.about_title), onBack, Modifier.offset(x = (-16).dp))
                Paragraph(stringResource(Res.string.about_own))
                NoticeRow(own, onOpen)
                Paragraph(linked(stringResource(Res.string.about_uniffi, UniffiSource), UniffiSource))
                Spacer(Modifier.height(12.dp))
                BasicText(
                    stringResource(Res.string.about_third_party).toUpperCase(Locale.current),
                    Modifier.semantics { heading() },
                    style = type.labelMedium.copy(color = c.legend),
                )
                Paragraph(stringResource(Res.string.about_third_party_body))
            }
        }
        items(notices, key = { "${it.id} ${it.version}" }) { notice ->
            Box(Modifier.widthIn(max = 560.dp)) { NoticeRow(notice, onOpen) }
        }
    }
}

/** One notice's licence: its name, version and licence, the source's address, then the texts as written. */
@Composable
fun LicenceScreen(notice: Notice?, onBack: () -> Unit, modifier: Modifier = Modifier) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    LazyColumn(modifier.fillMaxSize().windowInsetsPadding(barInsets), contentPadding = pagePadding()) {
        item {
            Column(Modifier.widthIn(max = 640.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                BackHeader(notice?.name.orEmpty(), onBack, Modifier.offset(x = (-16).dp))
                if (notice != null) {
                    BasicText(caption(notice), style = type.bodyMedium.copy(color = c.legend, textDirection = TextDirection.Content))
                    notice.url?.let { Paragraph(linked(it, it)) }
                }
            }
        }
        items(notice?.texts.orEmpty()) { text ->
            // The licence as its authors wrote it: left to right, untranslated, selectable.
            SelectionContainer(Modifier.widthIn(max = 640.dp).padding(top = 16.dp)) {
                BasicText(text, style = type.bodySmall.copy(color = c.ink, textDirection = TextDirection.Ltr))
            }
        }
    }
}

/** A notice's page loaded by id: Oscyl's own licence, or a shipped notice. */
@Composable
fun LicenceScreen(id: String, version: String?, onBack: () -> Unit, modifier: Modifier = Modifier) {
    fun loaded(): Notice? = if (id == OscylNoticeId) {
        loadedOwnLicence()?.let { Notice(OscylNoticeId, "Oscyl", null, "MIT OR Apache-2.0", OscylSource, it) }
    } else {
        loadedNotices()?.firstOrNull { it.id == id && it.version == version }
    }
    val notice by produceState(loaded(), id, version) {
        value = if (id == OscylNoticeId) {
            Notice(OscylNoticeId, "Oscyl", null, "MIT OR Apache-2.0", OscylSource, loadOwnLicence())
        } else {
            loadNotices().firstOrNull { it.id == id && it.version == version }
        }
    }
    LicenceScreen(notice, onBack, modifier)
}

/** Oscyl's own repository. */
private const val OscylSource = "https://github.com/oscyl/oscyl"

/** The system bars, kept off the page's viewport: nothing scrolls under the status bar. */
private val barInsets @Composable get() = safeInsets.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal)

/** Clear of the island; 16 dp gutters. */
@Composable
private fun pagePadding(): PaddingValues =
    PaddingValues(start = 16.dp, end = 16.dp, bottom = 24.dp + LocalIslandClearance.current)

@Composable
private fun caption(notice: Notice): String =
    notice.version?.let { stringResource(Res.string.about_notice_caption, it, notice.licence) } ?: notice.licence

@Composable
private fun Paragraph(text: String) = Paragraph(buildAnnotatedString { append(text) })

@Composable
private fun Paragraph(text: AnnotatedString) {
    BasicText(
        text,
        style = MaterialTheme.typography.bodyMedium.copy(color = OscylTheme.colors.ink, textDirection = TextDirection.Content),
    )
}

/** [text] with [url] inside it as a link (underlined ink, opened by the platform). */
@Composable
private fun linked(text: String, url: String): AnnotatedString {
    val style = TextLinkStyles(SpanStyle(color = OscylTheme.colors.ink, textDecoration = TextDecoration.Underline))
    return buildAnnotatedString {
        val at = text.indexOf(url)
        if (at < 0) {
            append(text)
            return@buildAnnotatedString
        }
        append(text.substring(0, at))
        withLink(LinkAnnotation.Url(url, style)) { append(url) }
        append(text.substring(at + url.length))
    }
}

/** A notice as a row over a rule: name, then version and licence; opens its licence. */
@Composable
private fun NoticeRow(notice: Notice, onOpen: (Notice) -> Unit) {
    val c = OscylTheme.colors
    val type = MaterialTheme.typography
    val interaction = remember { MutableInteractionSource() }
    val shape = RoundedCornerShape(OscylTokens.radius)
    Column(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 56.dp)
            .drawBehind {
                val y = size.height - OscylTokens.ruleWidth.toPx() / 2
                drawLine(c.rule, Offset(0f, y), Offset(size.width, y), OscylTokens.ruleWidth.toPx())
            }
            .clickable(interaction, indication = null, role = Role.Button) { onOpen(notice) }
            .touchStates(interaction, shape, c.press)
            .padding(vertical = 8.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp, Alignment.CenterVertically),
    ) {
        BasicText(notice.name, style = type.bodyLarge.copy(color = c.ink, fontWeight = FontWeight.Medium, textDirection = TextDirection.Content))
        BasicText(caption(notice), style = type.bodySmall.copy(color = c.legend, textDirection = TextDirection.Content))
    }
}

private val SampleNotices = listOf(
    Notice("Albert Sans", "Albert Sans", null, "OFL-1.1", "https://github.com/usted/Albert-Sans", listOf("Copyright 2021 The Albert Sans Project Authors")),
    Notice("androidx.activity:activity", "Activity", "1.12.0", "Apache-2.0", null, listOf("Apache License")),
    Notice("anyhow", "anyhow", "1.0.104", "MIT OR Apache-2.0", "https://github.com/dtolnay/anyhow", listOf("MIT License")),
    Notice("UniFFI", "UniFFI", null, "MPL-2.0", UniffiSource, listOf("Mozilla Public License Version 2.0")),
    Notice("uniffi_core", "uniffi_core", "0.32.0", "MPL-2.0", UniffiSource, listOf("Mozilla Public License Version 2.0")),
)

private const val SampleLicence = """MIT License

Copyright (c) The anyhow authors

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software."""

/** The pages in the app's shell, Synth on the island's plate. */
@Composable
private fun AboutPreview(mode: OscylThemeMode = OscylThemeMode.Dark, content: @Composable (Modifier) -> Unit) = OscylTheme(mode) {
    Shell(Destination.Library, onSelect = {}, saveCount = 0, onSave = {}) { modifier -> content(modifier) }
}

@OscylPreviews
@Composable
private fun about_compact(@PreviewParameter(ThemeModes::class) mode: OscylThemeMode) = AboutPreview(mode) {
    AboutScreen(SampleNotices, onBack = {}, onOpen = {}, it)
}

@OscylA11yPreviews
@Composable
private fun about_a11y() = AboutPreview { AboutScreen(SampleNotices, onBack = {}, onOpen = {}, it) }

@Preview(name = "licence", device = PHONE)
@Composable
private fun about_licence() = AboutPreview {
    LicenceScreen(SampleNotices[2].copy(texts = listOf(SampleLicence)), onBack = {}, it)
}
