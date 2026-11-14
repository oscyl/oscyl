//! Oscyl's translations embedded in the binary, for apps without gettext
//! (the mobile apps): the `po/*.po` catalogs, parsed at first use, behind
//! the [`crate::i18n`] hook. [`set_locale`] picks the language and can be
//! called again when it changes. The GTK app keeps gettext.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Every catalog, by language code.
const CATALOGS: &[(&str, &str)] = &[("it", include_str!("../../../po/it.po"))];

/// One language's messages: singular forms, and plural forms by msgid.
#[derive(Debug, Default, PartialEq)]
pub struct Catalog {
    singular: HashMap<String, String>,
    plural: HashMap<String, Vec<String>>,
}

static CURRENT: RwLock<Option<&'static Catalog>> = RwLock::new(None);
static PARSED: OnceLock<Vec<(&'static str, Catalog)>> = OnceLock::new();

/// Show core and driver strings in the language of BCP 47 `tag` ("it",
/// "it-IT"); a language without a catalog (English among them) shows the
/// msgids.
pub fn set_locale(tag: &str) {
    let lang = tag.split(['-', '_']).next().unwrap_or("").to_lowercase();
    let all = PARSED.get_or_init(|| CATALOGS.iter().map(|(l, po)| (*l, parse(po))).collect());
    let found = all.iter().find(|(l, _)| *l == lang).map(|(_, c)| c);
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = found;
    crate::i18n::set_translator(translate);
    crate::i18n::set_plural_translator(translate_plural);
}

fn current() -> Option<&'static Catalog> {
    *CURRENT.read().unwrap_or_else(|e| e.into_inner())
}

fn translate(msgid: &str) -> String {
    current()
        .and_then(|c| c.singular.get(msgid))
        .cloned()
        .unwrap_or_else(|| msgid.to_string())
}

fn translate_plural(singular: &str, plural: &str, n: u32) -> String {
    // Every catalog Oscyl ships has two forms, the first for one
    // (`nplurals=2; plural=(n != 1)`), as English has.
    let i = usize::from(n != 1);
    current()
        .and_then(|c| c.plural.get(singular))
        .and_then(|forms| forms.get(i))
        .cloned()
        .unwrap_or_else(|| if n == 1 { singular } else { plural }.to_string())
}

/// The C-style string literals of one entry line, joined and unescaped.
fn unquote(s: &str) -> String {
    let s = s.trim();
    let inner = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or("");
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// A gettext `.po` file's translated entries (fuzzy and untranslated ones
/// left out, so they show in English).
pub fn parse(po: &str) -> Catalog {
    #[derive(Default)]
    struct Entry {
        fuzzy: bool,
        id: String,
        plural: Option<String>,
        strs: Vec<(usize, String)>,
    }
    let mut cat = Catalog::default();
    let mut entry = Entry::default();
    // Where continuation lines go: 0 id, 1 plural, 2 a msgstr.
    let mut field = None;
    let flush = |e: Entry, cat: &mut Catalog| {
        if e.fuzzy || e.id.is_empty() {
            return;
        }
        if e.plural.is_some() {
            let mut forms: Vec<(usize, String)> = e.strs;
            forms.sort_by_key(|(i, _)| *i);
            if forms.iter().all(|(_, s)| !s.is_empty()) && !forms.is_empty() {
                cat.plural
                    .insert(e.id, forms.into_iter().map(|(_, s)| s).collect());
            }
        } else if let Some((_, s)) = e.strs.into_iter().next().filter(|(_, s)| !s.is_empty()) {
            cat.singular.insert(e.id, s);
        }
    };
    for line in po.lines().map(str::trim) {
        if line.is_empty() {
            flush(std::mem::take(&mut entry), &mut cat);
            field = None;
        } else if let Some(flags) = line.strip_prefix("#,") {
            entry.fuzzy |= flags.contains("fuzzy");
        } else if line.starts_with('#') {
        } else if let Some(rest) = line.strip_prefix("msgid_plural ") {
            entry.plural = Some(unquote(rest));
            field = Some(1);
        } else if let Some(rest) = line.strip_prefix("msgid ") {
            if !entry.id.is_empty() || !entry.strs.is_empty() {
                flush(std::mem::take(&mut entry), &mut cat);
            }
            entry.id = unquote(rest);
            field = Some(0);
        } else if let Some(rest) = line.strip_prefix("msgstr[") {
            let (n, text) = rest.split_once(']').unwrap_or(("0", ""));
            entry.strs.push((n.parse().unwrap_or(0), unquote(text)));
            field = Some(2);
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            entry.strs.push((0, unquote(rest)));
            field = Some(2);
        } else if line.starts_with('"') {
            let more = unquote(line);
            match field {
                Some(0) => entry.id.push_str(&more),
                Some(1) => {
                    if let Some(p) = entry.plural.as_mut() {
                        p.push_str(&more)
                    }
                }
                Some(2) => {
                    if let Some((_, s)) = entry.strs.last_mut() {
                        s.push_str(&more)
                    }
                }
                _ => {}
            }
        }
    }
    flush(entry, &mut cat);
    cat
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_continuations_plurals_and_fuzzy() {
        let po = r#"
msgid ""
msgstr "Header: x\n"

msgid "Light"
msgstr "Chiaro"

msgid ""
"Two "
"lines"
msgstr ""
"Due "
"righe"

#, fuzzy
msgid "Dark"
msgstr "Scuro?"

msgid "Not yet"
msgstr ""

msgid "%d program"
msgid_plural "%d programs"
msgstr[0] "%d programma"
msgstr[1] "%d programmi"

msgid "Say \"hi\"\n"
msgstr "Di' \"ciao\"\n"
"#;
        let c = parse(po);
        assert_eq!(c.singular["Light"], "Chiaro");
        assert_eq!(c.singular["Two lines"], "Due righe");
        assert!(!c.singular.contains_key("Dark"), "fuzzy stays English");
        assert!(
            !c.singular.contains_key("Not yet"),
            "untranslated stays English"
        );
        assert!(!c.singular.contains_key(""), "the header is no message");
        assert_eq!(c.plural["%d program"], ["%d programma", "%d programmi"]);
        assert_eq!(c.singular["Say \"hi\"\n"], "Di' \"ciao\"\n");
    }

    #[test]
    fn the_shipped_catalog_parses() {
        assert!(parse(CATALOGS[0].1).singular.len() > 500);
    }
}
