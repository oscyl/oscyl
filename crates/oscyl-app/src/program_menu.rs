//! What a program row's menu offers and when: copy, paste over, copy to a
//! library, new program, move, revert, and clear or remove. The synth's
//! slots are fixed, so there "delete" clears a slot to the init program; in
//! one of Oscyl's libraries it removes the program. Each app draws the menu
//! with its own labels.

use crate::state::Doc;

/// One thing a program row's menu does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Copy,
    PasteOver,
    CopyToLibrary,
    NewProgram,
    MoveUp,
    MoveDown,
    SwapUp,
    SwapDown,
    Revert,
    /// Take the program out of the library.
    Remove,
    /// Put the init program in the slot.
    Clear,
}

/// The menu's sections in order: in one of Oscyl's libraries
/// (`in_library`) programs can be added and removed, on the synth slots
/// are cleared. (Swapping is a keyboard action, not in the menu.)
pub fn sections(in_library: bool) -> Vec<Vec<Action>> {
    let mut out = vec![vec![Action::Copy, Action::PasteOver, Action::CopyToLibrary]];
    if in_library {
        out.push(vec![Action::NewProgram]);
    }
    out.push(vec![Action::MoveUp, Action::MoveDown]);
    out.push(vec![
        Action::Revert,
        if in_library {
            Action::Remove
        } else {
            Action::Clear
        },
    ]);
    out
}

/// Where the program is shown.
#[derive(Debug, Clone, Default)]
pub struct Context<'a> {
    /// One of Oscyl's own libraries (not the synth, not a file).
    pub in_library: bool,
    /// Programs can be put in another order here (a readable library of
    /// Oscyl's, or the synth once all of it has been read).
    pub can_reorder: bool,
    /// The program has been read (always, outside the synth).
    pub read: bool,
    /// The driver of the program on the clipboard, if any.
    pub clipboard: Option<&'a str>,
}

/// Whether `action` makes sense for program `i` of `doc` now. A factory
/// slot can be copied, never changed.
pub fn enabled(doc: &Doc, i: usize, cx: &Context, action: Action) -> bool {
    let exists = doc.work(i).is_some();
    let writes = exists && !doc.is_read_only(i);
    let len = doc.len();
    let order = cx.can_reorder && writes;
    match action {
        Action::Copy => exists,
        Action::PasteOver => writes && cx.clipboard == Some(doc.driver.info().id),
        Action::CopyToLibrary => exists && cx.read,
        Action::NewProgram => cx.in_library && cx.can_reorder,
        Action::MoveUp | Action::SwapUp => order && i > 0 && i < len,
        Action::MoveDown | Action::SwapDown => order && i + 1 < len,
        Action::Revert => doc.is_edited(i),
        Action::Remove => cx.in_library && exists,
        Action::Clear => writes,
    }
}

/// A library a program could be copied to.
#[derive(Debug, Clone)]
pub struct Candidate<K> {
    pub key: K,
    pub name: String,
    pub driver: String,
    /// Oscyl's own (not a file opened from elsewhere).
    pub managed: bool,
    /// Could not be read.
    pub broken: bool,
}

/// The libraries a program of synth `driver` can be copied to, in order:
/// Oscyl's own readable ones of that synth, except the one shown.
pub fn copy_targets<K: PartialEq>(
    libraries: impl IntoIterator<Item = Candidate<K>>,
    shown: Option<&K>,
    driver: &str,
) -> Vec<(K, String)> {
    libraries
        .into_iter()
        .filter(|l| l.managed && !l.broken && Some(&l.key) != shown && l.driver == driver)
        .map(|l| (l.key, l.name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oscyl_driver::Driver;

    fn rev2() -> &'static dyn Driver {
        oscyl_midi::driver("sequential-prophet-rev2").unwrap()
    }

    fn xd() -> &'static dyn Driver {
        oscyl_midi::driver("korg-minilogue-xd").unwrap()
    }

    #[test]
    fn library_and_synth_menus() {
        let flat = |in_library| sections(in_library).concat();
        assert!(flat(true).contains(&Action::Remove) && flat(true).contains(&Action::NewProgram));
        assert!(flat(false).contains(&Action::Clear) && !flat(false).contains(&Action::NewProgram));
    }

    #[test]
    fn factory_slots_copy_but_never_change() {
        let d = rev2();
        let n = crate::slots::slots(d).len();
        let doc = Doc::synth(d, vec![d.init_program(); n], "Rev2");
        let factory = (0..n).find(|&i| doc.is_read_only(i)).unwrap();
        let cx = Context {
            can_reorder: true,
            read: true,
            clipboard: Some(d.info().id),
            ..Context::default()
        };
        assert!(enabled(&doc, factory, &cx, Action::Copy));
        for a in [
            Action::PasteOver,
            Action::Clear,
            Action::MoveUp,
            Action::Remove,
        ] {
            assert!(!enabled(&doc, factory, &cx, a), "{a:?}");
        }
        assert!(enabled(&doc, 1, &cx, Action::PasteOver));
        assert!(enabled(&doc, 1, &cx, Action::MoveUp));
        assert!(!enabled(&doc, 0, &cx, Action::MoveUp));
        // Another synth's program can't be pasted.
        let other = Context {
            clipboard: Some(xd().info().id),
            ..cx
        };
        assert!(!enabled(&doc, 1, &other, Action::PasteOver));
    }

    #[test]
    fn unread_programs_stay_where_they_are() {
        let d = xd();
        let doc = Doc::synth(d, vec![d.init_program(); 3], "xd");
        let cx = Context::default();
        assert!(!enabled(&doc, 1, &cx, Action::MoveUp));
        assert!(!enabled(&doc, 1, &cx, Action::CopyToLibrary));
        assert!(!enabled(&doc, 3, &cx, Action::Copy));
    }

    #[test]
    fn targets_are_other_readable_libraries_of_the_synth() {
        let l = |key: u8, driver: &str, managed, broken| Candidate {
            key,
            name: format!("L{key}"),
            driver: driver.into(),
            managed,
            broken,
        };
        let all = vec![
            l(0, "a", true, false),
            l(1, "a", true, false),
            l(2, "b", true, false),
            l(3, "a", false, false),
            l(4, "a", true, true),
        ];
        assert_eq!(
            copy_targets(all, Some(&0), "a"),
            vec![(1, "L1".to_string())]
        );
    }
}
