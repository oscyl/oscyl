//! Conformance suite for Oscyl synth drivers.
//!
//! Every driver calls [`run`] from its own `tests/conformance.rs`, against
//! its own simulator. A failure panics with `"<driver id>: <check>: <what>"`.

pub mod pump;

use oscyl_driver::{
    Control, Curation, Driver, DriverError, DriverSession, LibProgram, LibraryData, Mutability,
    Page, Recipe, SessionEvent, SessionOp, Simulator,
};

use oscyl_driver::sysex::DEVICE_INQUIRY;
use pump::{run_until_idle, settle};

/// Every check, in order.
pub fn run(driver: &dyn Driver) {
    init_program(driver);
    fields(driver);
    names(driver);
    wrong_length_programs(driver);
    rejects_bad_targets(driver);
    file_formats_round_trip(driver);
    write_then_read_back(driver);
    audition(driver);
    progress(driver);
    faults_never_report_false_success(driver);
    lost_replies_fail(driver);
    curation_covers_every_field(driver);
    formats_every_value(driver);
    generation_is_deterministic_and_valid(driver);
    live_messages_are_well_formed(driver);
    presentation_is_consistent(driver);
    recognizes_its_simulator(driver);
    app_hooks(driver);
}

macro_rules! check {
    ($d:expr, $check:literal, $cond:expr, $($what:tt)+) => {
        assert!($cond, "{}: {}: {}", $d.info().id, $check, format!($($what)+))
    };
}

pub fn init_program(d: &dyn Driver) {
    let p = d.init_program();
    check!(
        d,
        "init_program",
        d.check_program(&p).is_ok(),
        "init program is invalid"
    );
    let name = d.name(&p).unwrap_or_default();
    let allowed = d.name_rules().allowed;
    check!(
        d,
        "init_program",
        !name.is_empty() && name.chars().all(allowed),
        "init name {name:?} empty or not allowed"
    );
}

pub fn fields(d: &dyn Driver) {
    let base = d.init_program();
    for (i, f) in d.fields().iter().enumerate() {
        for v in [f.min, f.max] {
            let mut p = base.clone();
            let set = d.set(&mut p, i, v);
            check!(d, "fields", set.is_ok(), "{} = {v}: {set:?}", f.name);
            check!(
                d,
                "fields",
                d.get(&p, i) == Ok(v),
                "{} reads back wrong",
                f.name
            );
            for (j, g) in d.fields().iter().enumerate().filter(|(j, _)| *j != i) {
                check!(
                    d,
                    "fields",
                    d.get(&p, j) == d.get(&base, j),
                    "setting {} changed {}",
                    f.name,
                    g.name
                );
            }
        }
        let mut p = base.clone();
        if f.max < u16::MAX {
            check!(
                d,
                "fields",
                d.set(&mut p, i, f.max + 1).is_err(),
                "{} accepts max + 1",
                f.name
            );
        }
        if f.min > 0 {
            check!(
                d,
                "fields",
                d.set(&mut p, i, f.min - 1).is_err(),
                "{} accepts min - 1",
                f.name
            );
        }
    }
}

/// A name of `len` allowed characters.
fn name_of(d: &dyn Driver, len: usize) -> String {
    let allowed = d.name_rules().allowed;
    let pool: Vec<char> = ('A'..='Z')
        .chain('0'..='9')
        .filter(|c| allowed(*c))
        .collect();
    assert!(
        !pool.is_empty(),
        "{}: names: no letters or digits allowed",
        d.info().id
    );
    (0..len).map(|i| pool[i % pool.len()]).collect()
}

pub fn names(d: &dyn Driver) {
    let max = d.name_rules().max_len;
    let mut p = d.init_program();
    let name = name_of(d, max);
    check!(
        d,
        "names",
        d.set_name(&mut p, &name).is_ok(),
        "{max}-char name rejected"
    );
    check!(
        d,
        "names",
        d.name(&p).as_deref() == Ok(name.as_str()),
        "name does not read back"
    );
    check!(
        d,
        "names",
        d.set_name(&mut p, &name_of(d, max + 1)).is_err(),
        "{}-char name accepted",
        max + 1
    );
}

/// First, second and last slot of every writable bank, each with a
/// distinct program named after its slot.
fn test_programs(d: &dyn Driver) -> Vec<(u16, Vec<u8>)> {
    let max = d.name_rules().max_len;
    let mut slots: Vec<u16> = d
        .capabilities()
        .banks
        .iter()
        .filter(|b| b.writable && b.len > 0)
        .flat_map(|b| [b.first, b.first + 1.min(b.len - 1), b.first + b.len - 1])
        .collect();
    slots.dedup();
    slots
        .into_iter()
        .map(|slot| {
            let mut p = d.init_program();
            let name: String = format!("C{slot}").chars().take(max).collect();
            d.set_name(&mut p, &name)
                .unwrap_or_else(|e| panic!("{}: test name {name:?}: {e}", d.info().id));
            (slot, p)
        })
        .collect()
}

fn submit(d: &dyn Driver, s: &mut dyn DriverSession, op: SessionOp, check: &str) {
    if let Err(e) = s.submit(op) {
        panic!("{}: {check}: submit failed: {e}", d.info().id);
    }
}

pub fn write_then_read_back(d: &dyn Driver) {
    let (mut s, mut sim) = (d.session(0), d.simulator());
    let programs = test_programs(d);
    submit(
        d,
        &mut *s,
        SessionOp::WritePrograms(programs.clone()),
        "write_then_read_back",
    );
    let ev = run_until_idle(&mut *s, &mut *sim, d.info().id);
    check!(
        d,
        "write_then_read_back",
        ev.last() == Some(&SessionEvent::Done),
        "write ended {:?}",
        ev.last()
    );
    for (slot, p) in &programs {
        check!(
            d,
            "write_then_read_back",
            sim.program(*slot).as_ref() == Some(p),
            "slot {slot} holds wrong data"
        );
    }
    check!(
        d,
        "write_then_read_back",
        sim.is_settled(),
        "simulator not settled after the write"
    );
    let slots: Vec<u16> = programs.iter().map(|(n, _)| *n).collect();
    submit(
        d,
        &mut *s,
        SessionOp::ReadPrograms(slots),
        "write_then_read_back",
    );
    let ev = run_until_idle(&mut *s, &mut *sim, d.info().id);
    for (slot, p) in &programs {
        check!(
            d,
            "write_then_read_back",
            ev.contains(&SessionEvent::Program(*slot, p.clone())),
            "slot {slot} did not read back"
        );
    }
    check!(
        d,
        "write_then_read_back",
        ev.last() == Some(&SessionEvent::Done),
        "read ended {:?}",
        ev.last()
    );
}

pub fn audition(d: &dyn Driver) {
    if !d.capabilities().edit_buffer {
        return;
    }
    let (mut s, mut sim) = (d.session(0), d.simulator());
    let (_, p) = test_programs(d).swap_remove(0);
    submit(d, &mut *s, SessionOp::Audition(p.clone()), "audition");
    let ev = run_until_idle(&mut *s, &mut *sim, d.info().id);
    check!(
        d,
        "audition",
        ev.last() == Some(&SessionEvent::Done),
        "audition ended {:?}",
        ev.last()
    );
    check!(
        d,
        "audition",
        sim.edit_buffer() == p,
        "edit buffer does not hold the program"
    );
    submit(d, &mut *s, SessionOp::ReadCurrent, "audition");
    let ev = run_until_idle(&mut *s, &mut *sim, d.info().id);
    check!(
        d,
        "audition",
        ev.contains(&SessionEvent::Current(p)),
        "current program does not read back"
    );
    settle(&mut *sim, d.info().id);
}

pub fn progress(d: &dyn Driver) {
    let (mut s, mut sim) = (d.session(0), d.simulator());
    let programs = test_programs(d);
    let slots = programs.iter().map(|(n, _)| *n).collect();
    for op in [
        SessionOp::WritePrograms(programs),
        SessionOp::ReadPrograms(slots),
    ] {
        submit(d, &mut *s, op, "progress");
        let ev = run_until_idle(&mut *s, &mut *sim, d.info().id);
        let steps: Vec<(usize, usize)> = ev
            .iter()
            .filter_map(|e| match e {
                SessionEvent::Progress { done, total } => Some((*done, *total)),
                _ => None,
            })
            .collect();
        let ok = !steps.is_empty()
            && steps.windows(2).all(|w| w[0].0 < w[1].0)
            && steps.iter().all(|(_, t)| *t == steps[0].1)
            && steps.last().is_some_and(|(done, total)| done == total);
        check!(d, "progress", ok, "progress {steps:?}");
        let last_progress = ev
            .iter()
            .rposition(|e| matches!(e, SessionEvent::Progress { .. }));
        let done = ev.iter().position(|e| *e == SessionEvent::Done);
        check!(
            d,
            "progress",
            done > last_progress,
            "Done before the last progress"
        );
    }
}

/// Did an op end in `Done` (true) or `Failed` (false)? Anything else fails
/// the check.
fn ended_done(d: &dyn Driver, ev: &[SessionEvent], fault: &str) -> bool {
    match ev.last() {
        Some(SessionEvent::Done) => true,
        Some(SessionEvent::Failed(_)) => false,
        other => panic!("{}: faults: {fault}: op ended {other:?}", d.info().id),
    }
}

pub fn faults_never_report_false_success(d: &dyn Driver) {
    for fault in d.simulator().faults() {
        let (mut s, mut sim): (Box<dyn DriverSession>, Box<dyn Simulator>) =
            (d.session(0), d.simulator());
        sim.inject(fault);
        let programs = test_programs(d);
        submit(
            d,
            &mut *s,
            SessionOp::WritePrograms(programs.clone()),
            "faults",
        );
        let ev = run_until_idle(&mut *s, &mut *sim, d.info().id);
        if ended_done(d, &ev, fault) {
            for (slot, p) in &programs {
                check!(
                    d,
                    "faults",
                    sim.program(*slot).as_ref() == Some(p),
                    "{fault}: write reported done but slot {slot} differs"
                );
            }
        }
        let slots = programs.iter().map(|(n, _)| *n).collect();
        submit(d, &mut *s, SessionOp::ReadPrograms(slots), "faults");
        let ev = run_until_idle(&mut *s, &mut *sim, d.info().id);
        ended_done(d, &ev, fault);
        for e in &ev {
            if let SessionEvent::Program(slot, bytes) = e {
                check!(
                    d,
                    "faults",
                    sim.program(*slot).as_ref() == Some(bytes),
                    "{fault}: read of slot {slot} returned data the synth does not hold"
                );
            }
        }
    }
}

pub fn wrong_length_programs(d: &dyn Driver) {
    let p = d.init_program();
    for bad in [&p[..p.len() - 1], &[p.clone(), vec![0]].concat()[..]] {
        let mut b = bad.to_vec();
        let n = b.len();
        check!(
            d,
            "wrong_length_programs",
            d.check_program(&b).is_err(),
            "{n} bytes pass check_program"
        );
        check!(
            d,
            "wrong_length_programs",
            d.get(&b, 0).is_err(),
            "get accepts {n} bytes"
        );
        check!(
            d,
            "wrong_length_programs",
            d.set(&mut b, 0, d.fields()[0].min).is_err(),
            "set accepts {n} bytes"
        );
        check!(
            d,
            "wrong_length_programs",
            d.name(&b).is_err(),
            "name accepts {n} bytes"
        );
        check!(
            d,
            "wrong_length_programs",
            d.set_name(&mut b, "A").is_err(),
            "set_name accepts {n} bytes"
        );
    }
}

/// Writes to read-only banks or past the last bank, and reads past the
/// last bank, are refused at `submit` and send nothing.
pub fn rejects_bad_targets(d: &dyn Driver) {
    let banks = d.capabilities().banks;
    let past = banks.iter().map(|b| b.first + b.len).max().unwrap_or(0);
    let mut writes: Vec<u16> = banks
        .iter()
        .filter(|b| !b.writable && b.len > 0)
        .flat_map(|b| [b.first, b.first + b.len - 1])
        .collect();
    writes.push(past);
    let p = d.init_program();
    for slot in writes {
        let mut s = d.session(0);
        let r = s.submit(SessionOp::WritePrograms(vec![(slot, p.clone())]));
        check!(
            d,
            "rejects_bad_targets",
            r.is_err(),
            "write to slot {slot} accepted"
        );
        check!(
            d,
            "rejects_bad_targets",
            s.poll_transmit(std::time::Instant::now()).is_none(),
            "write to slot {slot} sent bytes"
        );
    }
    let mut s = d.session(0);
    let r = s.submit(SessionOp::ReadPrograms(vec![past]));
    check!(
        d,
        "rejects_bad_targets",
        r.is_err(),
        "read of slot {past} accepted"
    );
    check!(
        d,
        "rejects_bad_targets",
        s.poll_transmit(std::time::Instant::now()).is_none(),
        "read of slot {past} sent bytes"
    );
}

/// Every simulator offers `lose_reply` (every reply lost). Under it every
/// op must end `Failed`: a lost reply is never success.
pub fn lost_replies_fail(d: &dyn Driver) {
    check!(
        d,
        "lost_replies_fail",
        d.simulator().faults().contains(&"lose_reply"),
        "simulator has no lose_reply fault"
    );
    let (mut s, mut sim) = (d.session(0), d.simulator());
    sim.inject("lose_reply");
    let programs = test_programs(d);
    let slots = programs.iter().map(|(n, _)| *n).collect();
    let (_, one) = programs[0].clone();
    let mut ops = vec![
        SessionOp::WritePrograms(programs),
        SessionOp::ReadPrograms(slots),
        SessionOp::ReadCurrent,
    ];
    if d.capabilities().edit_buffer {
        ops.push(SessionOp::Audition(one));
    }
    for op in ops {
        let what = format!("{op:?}").chars().take(24).collect::<String>();
        submit(d, &mut *s, op, "lost_replies_fail");
        let ev = run_until_idle(&mut *s, &mut *sim, d.info().id);
        check!(
            d,
            "lost_replies_fail",
            matches!(ev.last(), Some(SessionEvent::Failed(_))),
            "{what}… ended {:?}",
            ev.last()
        );
    }
}

/// The driver names its ports and recognises its simulator's answer to
/// the device inquiry, but not the inquiry itself (a port looping our
/// output back).
pub fn recognizes_its_simulator(d: &dyn Driver) {
    let hints = d.port_hint();
    check!(
        d,
        "recognizes_its_simulator",
        !hints.is_empty() && hints.iter().all(|h| !h.is_empty()),
        "port hints {hints:?} empty"
    );
    let replies = d.simulator().handle(&DEVICE_INQUIRY);
    check!(
        d,
        "recognizes_its_simulator",
        replies.iter().any(|r| d.recognize(r)),
        "no recognised reply among {replies:02X?}"
    );
    check!(
        d,
        "recognizes_its_simulator",
        !d.recognize(&DEVICE_INQUIRY),
        "the inquiry itself is recognised"
    );
    // Byte 2 is the device's channel or 7F; any of them is still the synth.
    if let Some(reply) = replies.iter().find(|r| d.recognize(r)) {
        for ch in (0..16).chain([0x7F]) {
            let mut r = reply.clone();
            r[2] = ch;
            check!(
                d,
                "recognizes_its_simulator",
                d.recognize(&r),
                "reply on channel {ch:#04x} not recognised"
            );
        }
    }
    // A reply from another vendor (7D: non-commercial id).
    let foreign = [
        0xF0, 0x7E, 0x00, 0x06, 0x02, 0x7D, 0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00, 0xF7,
    ];
    check!(
        d,
        "recognizes_its_simulator",
        !d.recognize(&foreign),
        "a foreign identity reply is recognised"
    );
}

/// Every vendor format gives back the program bytes it was given, and the
/// author and comment unless it holds none; unknown extensions
/// are refused.
pub fn file_formats_round_trip(d: &dyn Driver) {
    let lib = LibraryData {
        programs: test_programs(d)
            .into_iter()
            .map(|(_, data)| LibProgram {
                data,
                author: "A".into(),
                comment: "C".into(),
            })
            .collect(),
        extras: Default::default(),
    };
    for f in d.file_formats() {
        let bytes = d.export(f.ext, &lib).unwrap_or_else(|e| {
            panic!(
                "{}: file_formats_round_trip: export .{}: {e}",
                d.info().id,
                f.ext
            )
        });
        let back = d.import(f.ext, &bytes).unwrap_or_else(|e| {
            panic!(
                "{}: file_formats_round_trip: import .{}: {e}",
                d.info().id,
                f.ext
            )
        });
        // A format declares whether it keeps author and comment: if it
        // does, they come back exactly; if not, they come back empty.
        let same = back.programs.len() == lib.programs.len()
            && back.programs.iter().zip(&lib.programs).all(|(b, p)| {
                let meta = if f.keeps_metadata {
                    (b.author.as_str(), b.comment.as_str())
                        == (p.author.as_str(), p.comment.as_str())
                } else {
                    b.author.is_empty() && b.comment.is_empty()
                };
                b.data == p.data && meta
            });
        check!(
            d,
            "file_formats_round_trip",
            same,
            ".{} changed the programs",
            f.ext
        );
        let again = d
            .export(f.ext, &back)
            .ok()
            .and_then(|b| d.import(f.ext, &b).ok());
        check!(
            d,
            "file_formats_round_trip",
            again.as_ref() == Some(&back),
            ".{} is not stable over a second round trip",
            f.ext
        );
    }
    check!(
        d,
        "file_formats_round_trip",
        d.import("oscyl-no-such-format", &[]).is_err(),
        "unknown format accepted"
    );
}

/// Every field is in exactly one section or excluded, and what the editor
/// shows is complete: titles, labels, and controls that fit the field.
pub fn curation_covers_every_field(d: &dyn Driver) {
    let c = d.curation();
    let shape = curation_shape(&c, d.fields().len());
    check!(d, "curation", shape.is_ok(), "{}", shape.unwrap_err());
    // Controls may depend on the program: check init and generated ones.
    let mut programs = vec![d.init_program()];
    for character in 0..d.characters().len() {
        let r = Recipe {
            character,
            seed: 11,
            ..Recipe::default()
        };
        programs.extend(d.generate(&r, None));
    }
    for i in c
        .pages
        .iter()
        .flat_map(|p| &p.sections)
        .flat_map(|s| &s.fields)
    {
        let (i, f) = (*i, &d.fields()[*i]);
        check!(
            d,
            "curation",
            !d.short_label(i).is_empty(),
            "{} has no short label",
            f.name
        );
        for p in &programs {
            let ok = match d.control(p, i) {
                Control::Toggle => f.max - f.min == 1,
                Control::Choice(names) => !names.is_empty(),
                Control::Bands { names, width } => !names.is_empty() && width > 0,
                Control::Continuous => true,
            };
            check!(d, "curation", ok, "{} has an unfit control", f.name);
        }
    }
}

/// Whether `c` lays out `n` fields: at least one page, not every field
/// excluded (a raw dump or an empty editor is not curation), every index
/// below `n` and in exactly one section or the excluded list, titles and
/// sections non-empty.
pub fn curation_shape(c: &Curation, n: usize) -> Result<(), String> {
    if c.pages.is_empty() {
        return Err("no pages".into());
    }
    let shown: Vec<usize> = c
        .pages
        .iter()
        .flat_map(|p| &p.sections)
        .flat_map(|s| s.fields.iter().copied())
        .collect();
    if shown.is_empty() {
        return Err("every field is excluded".into());
    }
    let mut seen = vec![0usize; n];
    for &i in shown.iter().chain(&c.excluded) {
        *seen
            .get_mut(i)
            .ok_or_else(|| format!("field index {i} out of range"))? += 1;
    }
    if let Some((i, times)) = seen.iter().enumerate().find(|(_, t)| **t != 1) {
        return Err(format!("field {i} appears {times} times"));
    }
    for p in &c.pages {
        if p.title.is_empty() || p.sections.is_empty() {
            return Err(format!("page {:?} is untitled or empty", p.title));
        }
        if let Some(s) = p
            .sections
            .iter()
            .find(|s| s.title.is_empty() || s.fields.is_empty())
        {
            return Err(format!(
                "page {}: section {:?} untitled or empty",
                p.title, s.title
            ));
        }
    }
    Ok(())
}

/// Every field formats its minimum and maximum as non-empty text.
pub fn formats_every_value(d: &dyn Driver) {
    let base = d.init_program();
    for (i, f) in d.fields().iter().enumerate() {
        for v in [f.min, f.max] {
            let mut p = base.clone();
            if let Err(e) = d.set(&mut p, i, v) {
                panic!("{}: formats: {} = {v}: {e}", d.info().id, f.name);
            }
            check!(
                d,
                "formats",
                !d.format_value(&p, i).is_empty() && !d.format_raw(i, v).is_empty(),
                "{} = {v} formats empty",
                f.name
            );
        }
    }
}

/// The editor's other data agrees with the curation and the schema:
/// emphasized fields are shown, labels exist, categories list what
/// `category` says, signature pages exist, and graphs, handles and drags
/// stay within the pages and fields.
pub fn presentation_is_consistent(d: &dyn Driver) {
    let c = d.curation();
    let n = d.fields().len();
    let shown: Vec<usize> = c
        .pages
        .iter()
        .flat_map(|p| &p.sections)
        .flat_map(|s| s.fields.iter().copied())
        .collect();
    let mut programs = vec![d.init_program()];
    for character in 0..d.characters().len() {
        let r = Recipe {
            character,
            seed: 13,
            ..Recipe::default()
        };
        programs.extend(d.generate(&r, None));
    }
    for i in 0..n {
        let name = d.fields()[i].name;
        check!(
            d,
            "presentation",
            !d.emphasized(i) || shown.contains(&i),
            "{name} is emphasized but not shown"
        );
        for p in &programs {
            check!(
                d,
                "presentation",
                !d.label_in(p, i).is_empty() || !shown.contains(&i),
                "{name} has no label"
            );
        }
    }
    for p in &programs {
        if let Some(cat) = d.category(p) {
            check!(
                d,
                "presentation",
                d.categories().contains(&cat),
                "category {cat:?} is not in categories()"
            );
        }
        for page in 0..c.pages.len() {
            d.page_summary(p, page);
            d.graphs(p, page, 64);
            for (h, handle) in d.handles(p, page).iter().enumerate() {
                let ok = [handle.x, handle.y].iter().flatten().all(|&i| i < n)
                    && !handle.name.is_empty();
                check!(d, "presentation", ok, "page {page}: bad handle {h}");
                for (i, v) in d.drag(p, page, h, (0.5, 0.5)) {
                    let f = d.fields().get(i);
                    check!(
                        d,
                        "presentation",
                        f.is_some_and(|f| (f.min..=f.max).contains(&v)),
                        "page {page}: handle {h} drags field {i} to {v}"
                    );
                }
            }
        }
    }
    for ch in d.characters() {
        check!(
            d,
            "presentation",
            ch.signature_page.is_none_or(|s| s < c.pages.len()),
            "{}: no signature page {:?}",
            ch.id,
            ch.signature_page
        );
    }
    check!(
        d,
        "presentation",
        d.macro_info().iter().all(|m| !m.title.is_empty()),
        "a macro has no title"
    );
}

/// A generated, varied or bred program, checked valid with every field in
/// range.
fn valid_generated(d: &dyn Driver, what: &str, p: Result<Vec<u8>, DriverError>) -> Vec<u8> {
    let p = p.unwrap_or_else(|e| panic!("{}: generation: {what}: {e}", d.info().id));
    check!(
        d,
        "generation",
        d.check_program(&p).is_ok(),
        "{what} is invalid"
    );
    for (i, f) in d.fields().iter().enumerate() {
        let v = d.get(&p, i);
        check!(
            d,
            "generation",
            v.as_ref().is_ok_and(|v| (f.min..=f.max).contains(v)),
            "{what}: {} = {v:?}",
            f.name
        );
    }
    p
}

/// Same recipe, same bytes; every program valid; restricted and fixed
/// fields keep the base; kept pages come from the base; vary and breed
/// are deterministic and leave fixed fields alone.
pub fn generation_is_deterministic_and_valid(d: &dyn Driver) {
    let init = d.init_program();
    let pages = d.curation().pages;
    let not_free: Vec<usize> = (0..d.fields().len())
        .filter(|&i| d.mutability(i) != Mutability::Free)
        .collect();
    let same =
        |a: &[u8], b: &[u8], fields: &[usize]| fields.iter().all(|&i| d.get(a, i) == d.get(b, i));
    for character in 0..d.characters().len() {
        let mut programs = vec![];
        for seed in [0, 1, 0xDEAD_BEEF] {
            let r = Recipe {
                character,
                seed,
                ..Recipe::default()
            };
            let what = format!("character {character} seed {seed}");
            let p = valid_generated(d, &what, d.generate(&r, None));
            check!(
                d,
                "generation",
                d.generate(&r, None).as_ref() == Ok(&p),
                "{what} is not deterministic"
            );
            check!(
                d,
                "generation",
                same(&p, &init, &not_free),
                "{what} changed a restricted or fixed field"
            );
            programs.push(p);
        }
        let (a, b) = (&programs[0], &programs[2]);
        for (what, made, again) in [
            ("vary", d.vary(a, 0.7, 3), d.vary(a, 0.7, 3)),
            ("breed", d.breed(a, b, 0.5, 3), d.breed(a, b, 0.5, 3)),
        ] {
            let p = valid_generated(d, what, made);
            check!(
                d,
                "generation",
                again.as_ref() == Ok(&p),
                "{what} is not deterministic"
            );
        }
        generation_keeps_the_base(d, character, &pages, b);
    }
}

/// With a base, every non-free field comes from it and kept pages come
/// back from it whole; vary keeps every non-free field; breed takes
/// non-free fields from its first parent. The base has every field at
/// its maximum, so nothing matches init by accident.
fn generation_keeps_the_base(d: &dyn Driver, character: usize, pages: &[Page], other: &[u8]) {
    let mut base = d.init_program();
    for (i, f) in d.fields().iter().enumerate() {
        d.set(&mut base, i, f.max).expect("max is in range");
    }
    let not_free: Vec<usize> = (0..d.fields().len())
        .filter(|&i| d.mutability(i) != Mutability::Free)
        .collect();
    let differs = |p: &[u8], fields: &[usize]| {
        fields
            .iter()
            .find(|&&i| d.get(p, i) != d.get(&base, i))
            .map(|&i| d.fields()[i].name)
    };
    let keep_page = |keep: Vec<usize>| {
        let r = Recipe {
            character,
            seed: 2,
            keep,
            ..Recipe::default()
        };
        valid_generated(d, "generate with a base", d.generate(&r, Some(&base)))
    };
    let p = keep_page(vec![]);
    let bad = differs(&p, &not_free);
    check!(
        d,
        "generation",
        bad.is_none(),
        "generate with a base changed non-free {bad:?}"
    );
    if let Some(page) = pages.first() {
        let mut fields: Vec<usize> = page
            .sections
            .iter()
            .flat_map(|s| s.fields.iter().copied())
            .collect();
        fields.extend(&not_free);
        let bad = differs(&keep_page(vec![0]), &fields);
        check!(
            d,
            "generation",
            bad.is_none(),
            "page {} not kept: {bad:?}",
            page.title
        );
    }
    for (what, made) in [
        ("vary", d.vary(&base, 0.9, 4)),
        ("breed", d.breed(&base, other, 0.9, 4)),
    ] {
        let p = valid_generated(d, what, made);
        let bad = differs(&p, &not_free);
        check!(
            d,
            "generation",
            bad.is_none(),
            "{what} changed non-free {bad:?}"
        );
    }
}

/// Exactly one complete channel-voice message on `channel`: a status byte
/// and its data bytes (1 for program change and channel pressure, else 2),
/// each below 0x80.
pub fn well_formed(msg: &[u8], channel: u8) -> bool {
    let Some((&status, data)) = msg.split_first() else {
        return false;
    };
    let n = match status & 0xF0 {
        0xC0 | 0xD0 => 1,
        _ => 2,
    };
    (0x80..0xF0).contains(&status)
        && status & 0x0F == channel
        && data.len() == n
        && data.iter().all(|b| *b < 0x80)
}

/// Live messages exist exactly when the synth takes live changes, and
/// each is complete channel-voice MIDI on the channel asked for.
pub fn live_messages_are_well_formed(d: &dyn Driver) {
    const CH: u8 = 5;
    let base = d.init_program();
    let mut any = false;
    for (i, f) in d.fields().iter().enumerate() {
        for v in [f.min, f.max] {
            let mut p = base.clone();
            if let Err(e) = d.set(&mut p, i, v) {
                panic!("{}: live_messages: {} = {v}: {e}", d.info().id, f.name);
            }
            let Some(bytes) = d.live_messages(&p, i, CH) else {
                continue;
            };
            any = true;
            check!(
                d,
                "live_messages",
                d.capabilities().live_changes,
                "{} has live messages but live_changes is false",
                f.name
            );
            check!(
                d,
                "live_messages",
                !bytes.is_empty(),
                "{} = {v}: empty",
                f.name
            );
            for msg in &bytes {
                check!(
                    d,
                    "live_messages",
                    well_formed(msg, CH),
                    "{} = {v}: not one channel message {msg:02X?}",
                    f.name
                );
            }
        }
    }
    check!(
        d,
        "live_messages",
        any || !d.capabilities().live_changes,
        "live_changes is true but no field has a live message"
    );
}

/// The hooks apps show around the editor: the demo library holds valid
/// programs that fit the writable slots, the demo simulator answers the
/// device inquiry as the driver's synth, the list facet is an in-range
/// choice field, and firmware text never panics on the simulator's reply.
pub fn app_hooks(d: &dyn Driver) {
    let id = d.info().id;
    let demo = d.demo_library();
    let writable: usize = d
        .capabilities()
        .banks
        .iter()
        .filter(|b| b.writable)
        .map(|b| usize::from(b.len))
        .sum();
    assert!(
        demo.programs.len() <= writable,
        "{id}: demo library too big"
    );
    for p in &demo.programs {
        d.check_program(&p.data)
            .unwrap_or_else(|e| panic!("{id}: demo program: {e}"));
    }
    let replies = d
        .demo_simulator()
        .handle(&oscyl_driver::sysex::DEVICE_INQUIRY);
    let reply = replies
        .iter()
        .find(|r| d.recognize(r))
        .unwrap_or_else(|| panic!("{id}: demo simulator is not recognised"));
    let _ = d.firmware(reply);
    if let Some(f) = d.list_facet() {
        assert!(f < d.fields().len(), "{id}: list facet out of range");
        assert!(
            matches!(d.control(&d.init_program(), f), Control::Choice(_)),
            "{id}: list facet is not a choice"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{curation_shape, well_formed};
    use oscyl_driver::{Curation, Page, Section};

    fn page(fields: Vec<usize>) -> Page {
        Page {
            title: "P",
            icon: None,
            help: "",
            sections: vec![Section { title: "S", fields }],
        }
    }

    #[test]
    fn a_raw_dump_or_empty_editor_is_refused() {
        let ok = Curation {
            pages: vec![page(vec![0])],
            excluded: vec![1],
        };
        assert_eq!(curation_shape(&ok, 2), Ok(()));
        let none = Curation {
            pages: vec![],
            excluded: vec![0, 1],
        };
        assert!(curation_shape(&none, 2).is_err());
        let all_excluded = Curation {
            pages: vec![Page {
                sections: vec![],
                ..page(vec![])
            }],
            excluded: vec![0, 1],
        };
        assert!(curation_shape(&all_excluded, 2).is_err());
        let twice = Curation {
            pages: vec![page(vec![0, 1])],
            excluded: vec![1],
        };
        assert!(curation_shape(&twice, 2).is_err());
        let missing = Curation {
            pages: vec![page(vec![0])],
            excluded: vec![],
        };
        assert!(curation_shape(&missing, 2).is_err());
        let out_of_range = Curation {
            pages: vec![page(vec![0, 1, 2])],
            excluded: vec![],
        };
        assert!(curation_shape(&out_of_range, 2).is_err());
    }

    #[test]
    fn one_message_per_vec() {
        assert!(well_formed(&[0xB5, 99, 0], 5));
        assert!(well_formed(&[0xC5, 3], 5));
        assert!(!well_formed(&[0xB5, 99, 0, 0xB5, 98, 1], 5));
        assert!(!well_formed(&[0xB4, 99, 0], 5));
        assert!(!well_formed(&[0xB5, 0x80, 0], 5));
        assert!(!well_formed(&[0xB5, 99], 5));
        assert!(!well_formed(&[], 5));
    }
}
