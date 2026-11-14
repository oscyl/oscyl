//! `oscyl-gtk --write-pot <file>`: write the translation template.
//!
//! Collects every `tr("…")`/`trn("…", "…")` literal from the app's sources
//! (embedded at compile time), the action labels, and the user-visible
//! strings of the drivers that have translations, through the driver
//! contract: page titles and introductions, section titles, parameter
//! labels, value names, help, characters and macros. Other drivers'
//! strings stay English.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use oscyl_driver::{Control, Driver};

/// Drivers whose strings are in the translations (by driver id).
const TRANSLATED: &[&str] = &["korg-minilogue-xd"];

const SOURCES: &[(&str, &str)] = &[
    ("actions.rs", include_str!("actions.rs")),
    ("app.rs", include_str!("app.rs")),
    ("dialogs.rs", include_str!("dialogs.rs")),
    ("editor.rs", include_str!("editor.rs")),
    (
        "oscyl-app/editor.rs",
        include_str!("../../../crates/oscyl-app/src/editor.rs"),
    ),
    (
        "oscyl-app/generator.rs",
        include_str!("../../../crates/oscyl-app/src/generator.rs"),
    ),
    (
        "oscyl-app/units.rs",
        include_str!("../../../crates/oscyl-app/src/units.rs"),
    ),
    (
        "oscyl-app/review.rs",
        include_str!("../../../crates/oscyl-app/src/review.rs"),
    ),
    (
        "oscyl-app/panel.rs",
        include_str!("../../../crates/oscyl-app/src/panel.rs"),
    ),
    ("files.rs", include_str!("files.rs")),
    (
        "oscyl-app/files.rs",
        include_str!("../../../crates/oscyl-app/src/files.rs"),
    ),
    ("generator.rs", include_str!("generator.rs")),
    ("guide.rs", include_str!("guide.rs")),
    ("history.rs", include_str!("history.rs")),
    (
        "libraries.rs",
        include_str!("../../../crates/oscyl-app/src/libraries.rs"),
    ),
    ("library_menu.rs", include_str!("library_menu.rs")),
    ("onboarding.rs", include_str!("onboarding.rs")),
    ("perform.rs", include_str!("perform.rs")),
    ("program_menu.rs", include_str!("program_menu.rs")),
    ("review.rs", include_str!("review.rs")),
    ("widgets/dock.rs", include_str!("widgets/dock.rs")),
    ("widgets/fold.rs", include_str!("widgets/fold.rs")),
    ("programs.rs", include_str!("programs.rs")),
    (
        "settings.rs",
        include_str!("../../../crates/oscyl-app/src/settings.rs"),
    ),
    ("sidebar.rs", include_str!("sidebar.rs")),
    (
        "state.rs",
        include_str!("../../../crates/oscyl-app/src/state.rs"),
    ),
    (
        "synth.rs",
        include_str!("../../../crates/oscyl-app/src/synth.rs"),
    ),
    ("synth_events.rs", include_str!("synth_events.rs")),
    ("units.rs", include_str!("units.rs")),
    ("window.rs", include_str!("window.rs")),
    ("widgets/dial.rs", include_str!("widgets/dial.rs")),
    (
        "widgets/stage_strip.rs",
        include_str!("widgets/stage_strip.rs"),
    ),
];

/// String literals passed to `tr(`/`trn(` and to the guide's chapter
/// fields, each with its plural form (`trn` only). Handles escaped quotes
/// and line-continuation escapes.
fn literals(src: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    for marker in ["tr(", "trn(", "title: ", "body: ", "label("] {
        let mut rest = src;
        while let Some(pos) = rest.find(marker) {
            // A whole word (not `str(` or `set_label(`), then the literal,
            // possibly on the next line.
            let whole = !rest[..pos].ends_with(|c: char| c.is_alphanumeric() || c == '_');
            rest = rest[pos + marker.len()..].trim_start();
            let Some(after) = rest.strip_prefix('"').filter(|_| whole) else {
                continue;
            };
            rest = after;
            let mut s = String::new();
            let mut chars = rest.char_indices();
            while let Some((i, c)) = chars.next() {
                match c {
                    '\\' => match chars.next() {
                        Some((_, 'n')) => s.push('\n'),
                        Some((_, '\n')) => {
                            // Line continuation: skip leading whitespace.
                            while let Some((_, c)) = chars.clone().next() {
                                if c.is_whitespace() {
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                        }
                        Some((_, c)) => s.push(c),
                        None => break,
                    },
                    '"' => {
                        rest = &rest[i + 1..];
                        break;
                    }
                    c => s.push(c),
                }
            }
            // trn: also take the plural literal.
            let plural = if marker == "trn("
                && let Some(p) = rest.find('"')
                && let Some(end) = rest[p + 1..].find('"')
            {
                Some(rest[p + 1..p + 1 + end].to_string())
            } else {
                None
            };
            if !s.is_empty() {
                out.push((s, plural));
            }
        }
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Every message, with its plural form when it has one.
pub fn messages() -> BTreeMap<String, Option<String>> {
    let plurals: BTreeMap<String, Option<String>> =
        SOURCES.iter().flat_map(|(_, src)| literals(src)).collect();
    let mut msgs: BTreeSet<String> = plurals.keys().cloned().collect();
    for d in TRANSLATED.iter().filter_map(|id| oscyl_midi::driver(id)) {
        driver_strings(d, &mut msgs);
    }
    msgs.extend(crate::actions::ACTIONS.iter().map(|a| a.label.to_string()));
    msgs.retain(|m| !m.trim().is_empty());
    msgs.into_iter()
        .map(|m| {
            let plural = plurals.get(&m).cloned().flatten();
            (m, plural)
        })
        .collect()
}

/// What `d` shows through the app: its pages, sections, labels, help,
/// value names, characters and macros.
fn driver_strings(d: &dyn Driver, msgs: &mut BTreeSet<String>) {
    let init = d.init_program();
    for page in d.curation().pages {
        msgs.insert(page.title.into());
        msgs.insert(page.help.into());
        for section in page.sections {
            msgs.insert(section.title.into());
            for f in section.fields {
                msgs.insert(d.short_label(f).into());
                let h = d.help(f);
                msgs.insert(h.summary.into());
                msgs.insert(h.detail.into());
                if let Control::Choice(names) = d.control(&init, f) {
                    msgs.extend(names.iter().map(|n| n.to_string()));
                }
            }
        }
    }
    msgs.extend(d.fields().iter().map(|f| f.name.to_string()));
    for c in d.characters() {
        msgs.insert(c.title.into());
        msgs.insert(c.hint.into());
    }
    for m in d.macro_info() {
        msgs.insert(m.title.into());
        msgs.insert(m.help.into());
    }
    msgs.extend(d.categories().iter().map(|c| c.to_string()));
    // Labels and handles that change with the list facet (a depth reads
    // as a chord in one voice mode, a detune in another).
    let mut programs = vec![init.clone()];
    if let Some(f) = d.list_facet() {
        let spec = d.fields()[f];
        for v in spec.min..=spec.max {
            let mut p = init.clone();
            if d.set(&mut p, f, v).is_ok() {
                programs.push(p);
            }
        }
    }
    for p in &programs {
        for f in 0..d.fields().len() {
            msgs.insert(d.label_in(p, f).into_owned());
        }
        for page in 0..d.curation().pages.len() {
            msgs.extend(d.handles(p, page).iter().map(|h| h.name.to_string()));
        }
    }
    // The phrases summaries pass to the translator, over the demo sounds;
    // values (also passed through) are not phrases.
    programs.extend(d.demo_library().programs.into_iter().map(|p| p.data));
    let asked = std::cell::RefCell::new(BTreeSet::new());
    let record = |s: &str| {
        asked.borrow_mut().insert(s.to_string());
        s.to_string()
    };
    let mut values = BTreeSet::new();
    for p in &programs {
        for page in 0..d.curation().pages.len() {
            for compact in [false, true] {
                d.page_summary_in(p, page, compact, &record);
            }
        }
        values.extend((0..d.fields().len()).map(|f| d.format_value(p, f)));
    }
    msgs.extend(
        asked
            .into_inner()
            .into_iter()
            .filter(|s| !values.contains(s)),
    );
}

pub fn write(path: &std::path::Path) -> std::io::Result<()> {
    let mut pot = String::from(
        "# Oscyl translation template.\n\
         # This file is distributed under the same licence as Oscyl (MIT OR Apache-2.0).\n\
         msgid \"\"\nmsgstr \"\"\n\
         \"Project-Id-Version: oscyl\\n\"\n\
         \"Content-Type: text/plain; charset=UTF-8\\n\"\n\
         \"Content-Transfer-Encoding: 8bit\\n\"\n\
         \"Plural-Forms: nplurals=INTEGER; plural=EXPRESSION;\\n\"\n\n",
    );
    for (m, plural) in messages() {
        let _ = match plural {
            Some(p) => writeln!(
                pot,
                "msgid \"{}\"\nmsgid_plural \"{}\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n",
                escape(&m),
                escape(&p)
            ),
            None => writeln!(pot, "msgid \"{}\"\nmsgstr \"\"\n", escape(&m)),
        };
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, pot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_plain_escaped_and_continued_literals() {
        let src = r#"tr("Open") tr("Say \"hi\"") trn("%d file", "%d files", n) tr("a \
                    b") tr(
                        "Next line") from_str("Not this") set_label("Nor this")"#;
        let l = literals(src);
        let has = |m: &str, p: Option<&str>| l.contains(&(m.to_string(), p.map(String::from)));
        assert!(has("Open", None));
        assert!(has("Say \"hi\"", None));
        assert!(has("%d file", Some("%d files")));
        assert!(has("a b", None));
        assert!(has("Next line", None));
        assert!(!l.iter().any(|(m, _)| m.starts_with("No")));
    }

    #[test]
    fn template_covers_core_and_app_strings() {
        let m = messages();
        assert!(m.contains_key("Welcome to Oscyl"));
        assert!(m.contains_key("Cutoff"));
        let xd = oscyl_midi::driver("korg-minilogue-xd").unwrap();
        let cutoff = xd.fields().iter().position(|f| f.name == "Cutoff").unwrap();
        assert!(m.contains_key(xd.help(cutoff).summary));
        assert_eq!(m["%d program"].as_deref(), Some("%d programs"));
        assert!(m.len() > 300, "only {} messages", m.len());
    }

    /// msgids of a .po/.pot file (header excluded), with continuation lines joined.
    fn msgids(po: &str) -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        let mut cur: Option<String> = None;
        for line in po.lines() {
            let line = line.trim();
            let quoted = |l: &str| l.trim_matches('"').to_string();
            if let Some(rest) = line.strip_prefix("msgid ") {
                cur = Some(quoted(rest));
            } else if line.starts_with('"') {
                if let Some(c) = cur.as_mut() {
                    c.push_str(&quoted(line));
                }
            } else if let Some(c) = cur.take()
                && !c.is_empty()
            {
                ids.insert(c);
            }
        }
        ids.extend(cur.filter(|c| !c.is_empty()));
        ids
    }

    #[test]
    fn italian_translates_every_message() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../po");
        let read = |f: &str| std::fs::read_to_string(dir.join(f)).expect("po file");
        let (pot, it) = (msgids(&read("oscyl.pot")), msgids(&read("it.po")));
        let missing: Vec<_> = pot.difference(&it).collect();
        assert!(missing.is_empty(), "missing from it.po: {missing:?}");
    }

    #[test]
    fn template_is_current() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../po");
        let pot = std::fs::read_to_string(dir.join("oscyl.pot")).expect("pot");
        let fresh: BTreeSet<String> = messages().keys().map(|m| escape(m)).collect();
        assert_eq!(msgids(&pot), fresh, "regenerate po/oscyl.pot");
    }
}
