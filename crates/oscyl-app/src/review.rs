//! Reviewing edits before they are kept: every change of a program in
//! words, grouped by stage; which programs a sync writes to which slots;
//! and the backup taken before it. Shared by every app.

use std::path::{Path, PathBuf};

use oscyl_core::native;
use oscyl_driver::{Curation, Driver, LibraryData};

use crate::editor::{before_after, section_of};
use crate::i18n::tr;
use crate::slots::{slot_label, slots};
use crate::state::Doc;

/// One change to a program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// The field, or `None` for the program's name.
    pub field: Option<usize>,
    /// The page the field is on, if the editor shows it.
    pub stage: Option<usize>,
    /// "Filter · Cutoff", or "Name", translated.
    pub title: String,
    pub before: String,
    pub after: String,
}

/// One step of a document's history in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryStep {
    /// What it did: "Filter · Cutoff: 431 to 618", "Glass Choir removed".
    pub title: String,
    /// The program it touched (its position as the step left it).
    pub index: usize,
    /// Done (the undo side) or undone (the redo side).
    pub done: bool,
}

/// The document's history, oldest step first, each in words (translated).
pub fn history(doc: &Doc) -> Vec<HistoryStep> {
    use crate::i18n::trn;
    use crate::state::StepInfo;
    let c = doc.driver.curation();
    let n = |k: usize| u32::try_from(k).unwrap_or(u32::MAX);
    let done = doc.done();
    doc.steps()
        .into_iter()
        .enumerate()
        .map(|(at, step)| {
            let (title, index) = match step {
                StepInfo::Edit {
                    index,
                    label: Some(label),
                    ..
                } => (label.to_string(), index),
                StepInfo::Edit {
                    index,
                    before,
                    after,
                    label: None,
                } => {
                    let all = changes(doc.driver, &c, after, before);
                    let title = match all.as_slice() {
                        [] => tr("Program replaced"),
                        [one] => tr("{title}: {before} to {after}")
                            .replace("{title}", &one.title)
                            .replace("{before}", &one.before)
                            .replace("{after}", &one.after),
                        // A graph handle moves two fields; a pasted program, many.
                        [first, rest @ ..] if rest.len() < 3 => rest
                            .iter()
                            .fold(first.title.clone(), |t, ch| format!("{t}, {}", ch.title)),
                        many => trn("%d change", "%d changes", n(many.len())),
                    };
                    (title, index)
                }
                StepInfo::Replaced(k) => {
                    (trn("%d program replaced", "%d programs replaced", n(k)), 0)
                }
                StepInfo::Added(at, 1, name) if !name.is_empty() => {
                    (tr("{name} added").replace("{name}", &name), at)
                }
                StepInfo::Added(at, k, _) => {
                    (trn("%d program added", "%d programs added", n(k)), at)
                }
                StepInfo::Removed(at, 1, name) if !name.is_empty() => {
                    (tr("{name} removed").replace("{name}", &name), at)
                }
                StepInfo::Removed(at, k, _) => {
                    (trn("%d program removed", "%d programs removed", n(k)), at)
                }
                StepInfo::Moved(_, to) => (tr("Program moved"), to),
                StepInfo::Swapped(_, b) => (tr("Programs swapped"), b),
            };
            HistoryStep {
                title,
                index,
                done: at < done,
            }
        })
        .collect()
}

/// The history's words for snapshot `letter` put back.
pub fn recalled_label(letter: &str) -> String {
    tr("Recalled snapshot {letter}").replace("{letter}", letter)
}

/// Every change from `base` to `p`: the name first, then the fields in
/// schema order.
pub fn changes(d: &dyn Driver, c: &Curation, p: &[u8], base: &[u8]) -> Vec<Change> {
    let (name, saved_name) = (d.name(p).ok(), d.name(base).ok());
    let renamed = (name != saved_name).then(|| Change {
        field: None,
        stage: None,
        title: tr("Name"),
        before: saved_name.unwrap_or_default(),
        after: name.unwrap_or_default(),
    });
    let fields = (0..d.fields().len())
        .filter(|&f| d.get(p, f) != d.get(base, f))
        .map(|field| {
            let (stage, title) = match section_of(c, field) {
                Some((stage, section)) => (
                    Some(stage),
                    format!("{} · {}", tr(section), tr(d.short_label(field))),
                ),
                None => (None, tr(d.short_label(field))),
            };
            let (before, after) = before_after(d, field, base, p);
            Change {
                field: Some(field),
                stage,
                title,
                before,
                after,
            }
        });
    renamed.into_iter().chain(fields).collect()
}

/// A sync: the edited programs and what goes to which slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPlan {
    /// Positions in the synth's document.
    pub indices: Vec<usize>,
    /// (slot, program) to write.
    pub batch: Vec<(u16, Vec<u8>)>,
}

/// What syncing `doc` (a synth's slots) writes, or why it can't yet:
/// every edited program must have been read (`loaded`).
pub fn sync_plan(doc: &Doc, loaded: impl Fn(usize) -> bool) -> Result<SyncPlan, String> {
    let indices = doc.edited_indices();
    if !indices.iter().all(|&i| loaded(i)) {
        return Err(tr("Wait until all programs have been read from the synth."));
    }
    let at = slots(doc.driver);
    let batch = indices
        .iter()
        .filter_map(|&i| Some((*at.get(i)?, doc.work(i)?.to_vec())))
        .collect();
    Ok(SyncPlan { indices, batch })
}

/// "001 Velvet Pad": an edited program as a sync confirmation lists it.
pub fn sync_names(doc: &Doc, indices: &[usize]) -> Vec<String> {
    indices
        .iter()
        .map(|&i| format!("{} {}", slot_label(doc.driver, i), doc.name(i)))
        .collect()
}

/// The format synth backups of `d` are kept in: the synth's own when it
/// keeps each program's notes (the slot names), else Oscyl's.
pub fn backup_ext(d: &dyn Driver) -> &'static str {
    match d.file_formats().first() {
        Some(f) if f.keeps_metadata => f.ext,
        _ => native::EXT,
    }
}

/// Save `lib` of synth `d` as a new backup in folder `dir`, named
/// "Synth backup `stamp`", atomically; returns its path.
pub fn write_backup(
    dir: &Path,
    stamp: &str,
    d: &dyn Driver,
    lib: &LibraryData,
) -> Result<PathBuf, String> {
    let ext = backup_ext(d);
    let name = tr("Synth backup");
    // Two backups in one second never overwrite each other.
    let path = (1..)
        .map(|n| match n {
            1 => dir.join(format!("{name} {stamp}.{ext}")),
            n => dir.join(format!("{name} {stamp} ({n}).{ext}")),
        })
        .find(|p| !p.exists())
        .expect("a free name");
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let bytes = crate::files::write(ext, d, lib)?;
    crate::libraries::write_atomic(&path, |w| {
        std::io::Write::write_all(w, &bytes).map_err(|e| e.to_string())
    })?;
    Ok(path)
}

/// Now as "YYYY-MM-DD HH.MM.SS" in UTC, for file names.
pub fn utc_stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    stamp_of(secs)
}

fn stamp_of(secs: u64) -> String {
    // Days to a civil date (Howard Hinnant's algorithm).
    let days = (secs / 86_400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let t = secs % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02} {:02}.{:02}.{:02}",
        t / 3600,
        t / 60 % 60,
        t % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_named_step_is_one_undo_step_with_its_words() {
        let d = oscyl_midi::driver("korg-minilogue-xd").unwrap();
        let mut doc = crate::state::Doc::from_data(d, d.demo_library(), None, "Demo").unwrap();
        let rolled = crate::generator::roll_page(d, doc.work(0).unwrap(), 3, 9).unwrap();
        doc.set_program_named(0, rolled, "Rolled Filter");
        let h = history(&doc);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].title, "Rolled Filter");
        assert!(doc.undo().is_some() && !doc.can_undo());
    }

    fn xd() -> &'static dyn Driver {
        oscyl_midi::driver("korg-minilogue-xd").unwrap()
    }

    #[test]
    fn changes_name_the_stage_and_both_values() {
        let d = xd();
        let c = d.curation();
        let base = d.init_program();
        let mut p = base.clone();
        let cutoff = d.fields().iter().position(|f| f.name == "Cutoff").unwrap();
        d.set(&mut p, cutoff, 700).unwrap();
        d.set_name(&mut p, "Lead").unwrap();
        let all = changes(d, &c, &p, &base);
        assert_eq!(all.len(), 2);
        assert_eq!((all[0].field, all[0].after.as_str()), (None, "Lead"));
        assert_eq!(all[1].field, Some(cutoff));
        assert!(all[1].stage.is_some() && all[1].title.contains(" · "));
        assert!(changes(d, &c, &base, &base).is_empty());
    }

    #[test]
    fn the_history_says_each_step_in_words() {
        let d = xd();
        let mut doc = Doc::synth(d, vec![d.init_program(); 4], "xd");
        let cutoff = d.fields().iter().position(|f| f.name == "Cutoff").unwrap();
        doc.set_field(0, cutoff, 700).unwrap();
        doc.seal();
        doc.set_name(2, "Lead").unwrap();
        doc.undo();
        let h = history(&doc);
        assert_eq!(h.len(), 2);
        assert!(
            h[0].done
                && h[0].index == 0
                && h[0].title.contains("Cutoff")
                && h[0].title.contains(" to "),
            "{:?}",
            h[0]
        );
        assert!(
            !h[1].done && h[1].index == 2 && h[1].title.starts_with("Name: "),
            "{:?}",
            h[1]
        );
    }

    #[test]
    fn a_sync_waits_for_the_read_and_writes_only_edits() {
        let d = xd();
        let mut doc = Doc::synth(d, vec![d.init_program(); slots(d).len()], "xd");
        doc.set_name(3, "Mine").unwrap();
        assert!(sync_plan(&doc, |_| false).is_err());
        let plan = sync_plan(&doc, |_| true).unwrap();
        assert_eq!(plan.indices, [3]);
        assert_eq!(plan.batch[0].0, slots(d)[3]);
        assert_eq!(sync_names(&doc, &plan.indices), ["004 Mine"]);
    }

    #[test]
    fn backups_are_written_and_stamped() {
        assert_eq!(stamp_of(0), "1970-01-01 00.00.00");
        assert_eq!(stamp_of(1_700_000_000), "2023-11-14 22.13.20");
        let dir = std::env::temp_dir().join(format!("oscyl-backup-test-{}", std::process::id()));
        let d = xd();
        let path = write_backup(&dir, "now", d, &d.demo_library()).unwrap();
        let again = write_backup(&dir, "now", d, &d.demo_library()).unwrap();
        assert_ne!(
            path, again,
            "a second backup in the same second keeps the first"
        );
        let bytes = std::fs::read(&path).unwrap();
        let (_, back) = crate::files::read(&path.to_string_lossy(), &bytes).unwrap();
        assert_eq!(back.programs.len(), d.demo_library().programs.len());
        assert_eq!(backup_ext(d), "mnlgxdlib");
        let _ = std::fs::remove_dir_all(dir);
    }
}
