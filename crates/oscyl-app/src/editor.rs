//! The editor's decisions, shared by every app: which widget a field
//! gets and what an option means in raw values, how values read in the
//! context of the program, where a field sits, how many edits a stage
//! has, what a handle says, and a page's picture as plain marks to draw.
//! Each app draws; none decides these again.

use oscyl_driver::{Control, Curation, Driver, Graph, Handle, Page};

use crate::i18n::tr;

/// The widget a field's [`Control`] is drawn as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Widget {
    /// A numeric range.
    Dial,
    /// A few named values side by side.
    Segments(Vec<&'static str>),
    /// Named values, too many for segments: a wrapped selector or menu.
    Menu(Vec<&'static str>),
    /// Off / On, as two segments.
    Toggle,
}

/// Choices up to this many are segments; more go in a menu.
const SEGMENTS_MAX: usize = 4;

/// The widget for `control`.
pub fn widget(control: Control) -> Widget {
    match control {
        Control::Continuous => Widget::Dial,
        Control::Toggle => Widget::Toggle,
        Control::Choice(names) if names.len() <= SEGMENTS_MAX => Widget::Segments(names.to_vec()),
        Control::Choice(names) => Widget::Menu(names.to_vec()),
        Control::Bands { names, .. } => Widget::Menu(names.to_vec()),
    }
}

/// Which option of `control` raw value `raw` shows (`None` for a dial),
/// for a field whose range starts at `min`.
pub fn option_of(control: Control, min: u16, raw: u16) -> Option<usize> {
    match control {
        Control::Continuous => None,
        Control::Toggle => Some(usize::from(raw != min)),
        Control::Choice(_) => Some(usize::from(raw.saturating_sub(min))),
        Control::Bands { names, width } => {
            Some(usize::from(raw / width.max(1)).min(names.len().saturating_sub(1)))
        }
    }
}

/// The raw value that picks option `option` of `control`, for a field of
/// range `min..=max` (`None` for a dial).
pub fn raw_of(control: Control, min: u16, max: u16, option: usize) -> Option<u16> {
    let option = u16::try_from(option).ok()?;
    match control {
        Control::Continuous => None,
        Control::Toggle => Some(if option == 0 { min } else { min + 1 }),
        Control::Choice(_) => Some(min.saturating_add(option).min(max)),
        Control::Bands { width, .. } => Some(
            option
                .saturating_mul(width)
                .saturating_add(width / 2)
                .min(max),
        ),
    }
}

/// `raw` for `field`, read in the context of program `p` (a voice mode
/// depth names a chord or an arpeggio depending on the mode), translated.
/// When `field` is, in `p`, a parameter of a user unit the synth's
/// palette knows ([`crate::palette`]): the unit's own name for it, and the
/// lowest and highest value a program stores for it (the value itself;
/// the value plus 100 when it runs either side of zero).
pub fn unit_param(d: &dyn Driver, p: &[u8], field: usize) -> Option<(String, u16, u16)> {
    let (kind, slot, n) = d.unit_param(p, field)?;
    let palette = crate::palette::shared().all(d);
    let unit = palette
        .units
        .iter()
        .find(|u| u.kind == kind && u.slot == slot)?;
    let param = unit.params.as_ref()?.get(n)?;
    let f = d.fields().get(field)?;
    let stored = |v: i16| {
        let v = match param.kind {
            oscyl_driver::UnitParamKind::Bipolar => v + 100,
            _ => v,
        };
        (v.max(0) as u16).clamp(f.min, f.max)
    };
    let (lo, hi) = (stored(param.min), stored(param.max));
    Some((param.name.clone(), lo.min(hi), hi.max(lo)))
}

/// The label for `field` in `p`, in the app's language: the unit's own
/// word where the field is a known user unit's parameter (never
/// translated), else the driver's.
pub fn label_in(d: &dyn Driver, p: &[u8], field: usize) -> String {
    match unit_param(d, p, field) {
        Some((name, ..)) if !name.is_empty() => name,
        _ => crate::i18n::tr(&d.label_in(p, field)),
    }
}

/// The lowest and highest value `field` takes in `p`: a known user unit's
/// parameter keeps to what the unit declares, any other field to its own
/// range.
pub fn range_in(d: &dyn Driver, p: &[u8], field: usize) -> (u16, u16) {
    match unit_param(d, p, field) {
        Some((_, lo, hi)) => (lo, hi),
        None => d.fields().get(field).map_or((0, 0), |f| (f.min, f.max)),
    }
}

pub fn format_in(d: &dyn Driver, p: &[u8], field: usize, raw: u16) -> String {
    let mut q = p.to_vec();
    match d.set(&mut q, field, raw) {
        Ok(()) => tr(&d.format_value(&q, field)),
        Err(_) => tr(&d.format_raw(field, raw)),
    }
}

/// A program between `saved` (t = 0) and `edited` (t = 1): every dial at
/// `round(saved + (edited - saved) * t)`, every choice the edit's from
/// `t >= 0.5` (the saved one before). `t` is clamped to 0..1; the ends are
/// the two programs byte for byte.
pub fn blend(d: &dyn Driver, saved: &[u8], edited: &[u8], t: f32) -> Vec<u8> {
    let t = if t.is_nan() { 1.0 } else { t.clamp(0.0, 1.0) };
    if t <= 0.0 {
        return saved.to_vec();
    }
    if t >= 1.0 {
        return edited.to_vec();
    }
    let mut out = if t >= 0.5 {
        edited.to_vec()
    } else {
        saved.to_vec()
    };
    for f in 0..d.fields().len() {
        if widget(d.control(edited, f)) != Widget::Dial {
            continue;
        }
        let (Ok(a), Ok(b)) = (d.get(saved, f), d.get(edited, f)) else {
            continue;
        };
        let v = f32::from(a) + (f32::from(b) - f32::from(a)) * t;
        // A value the program refuses in this context keeps the one it has.
        let _ = d.set(&mut out, f, v.round() as u16);
    }
    out
}

/// The play page's layout a model starts with: the XY pad on the first
/// picture handle that moves on both axes (none: unassigned), and four
/// macro dials not on the pad: the large dials in page order first, then
/// the other dials (a model with fewer leaves the rest unassigned).
pub fn play_defaults(c: &Curation, d: &dyn Driver) -> crate::settings::PlayLayout {
    let p = d.init_program();
    let pad = (0..c.pages.len())
        .flat_map(|page| d.handles(&p, page))
        .find_map(|h| Some((h.x?, h.y?)));
    let (x, y) = (pad.map(|(x, _)| x), pad.map(|(_, y)| y));
    let mut dials: Vec<usize> = Vec::new();
    for f in c
        .pages
        .iter()
        .flat_map(|page| page.sections.iter().flat_map(|s| s.fields.iter().copied()))
    {
        if widget(d.control(&p, f)) == Widget::Dial
            && Some(f) != x
            && Some(f) != y
            && !dials.contains(&f)
        {
            dials.push(f);
        }
    }
    // Large first, each kind in page order.
    dials.sort_by_key(|&f| !d.emphasized(f));
    let mut macros: Vec<Option<usize>> = dials.into_iter().take(4).map(Some).collect();
    macros.resize(4, None);
    crate::settings::PlayLayout { x, y, macros }
}

/// Text as typed or as written, folded for comparing: trimmed, lower case,
/// no spaces, no leading "+", a decimal comma as a point.
fn fold_value(text: &str) -> String {
    let t: String = text
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c == ',' { '.' } else { c })
        .collect();
    t.strip_prefix('+').map(str::to_string).unwrap_or(t)
}

/// The number a folded text starts with ("-12c" is -12, "2.4hz" is 2.4).
fn leading_number(folded: &str) -> Option<f64> {
    let end = folded
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || c == '.' || (i == 0 && c == '-')))
        .map_or(folded.len(), |(i, _)| i);
    folded[..end].parse().ok()
}

/// The raw value of `field` that typed `text` names, in the context of
/// program `p`: the value whose written form it is (folded: case, spaces,
/// "+" and a decimal comma do not matter), the nearest to the field's
/// current value when several are written alike; else, when `text` starts
/// with a number, the value whose written number is nearest to it; else
/// `None`.
pub fn parse_in(d: &dyn Driver, p: &[u8], field: usize, text: &str) -> Option<u16> {
    let info = d.fields().get(field)?;
    let now = d.get(p, field).unwrap_or(info.min);
    parse_among(&written(d, p, field)?, now, text)
}

/// Every value of `field` with its written form, folded, in program `p`'s context.
fn written(d: &dyn Driver, p: &[u8], field: usize) -> Option<Vec<(u16, String)>> {
    let info = d.fields().get(field)?;
    Some(
        (info.min..=info.max)
            .map(|raw| (raw, fold_value(&format_in(d, p, field, raw))))
            .collect(),
    )
}

/// [`parse_in`]'s choice among a field's `written` values, `now` its current one.
fn parse_among(written: &[(u16, String)], now: u16, text: &str) -> Option<u16> {
    let typed = fold_value(text);
    if typed.is_empty() {
        return None;
    }
    let off = |raw: u16| (i32::from(raw) - i32::from(now)).abs();
    if let Some((raw, _)) = written
        .iter()
        .filter(|(_, w)| *w == typed)
        .min_by_key(|(r, _)| off(*r))
    {
        return Some(*raw);
    }
    let want = leading_number(&typed)?;
    written
        .iter()
        .filter_map(|(raw, w)| leading_number(w).map(|n| (*raw, (n - want).abs())))
        .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| off(a.0).cmp(&off(b.0))))
        .map(|(raw, _)| raw)
}

/// A changed field's saved and current values, translated, without a
/// program's context (before/after text).
pub fn before_after(d: &dyn Driver, field: usize, base: &[u8], p: &[u8]) -> (String, String) {
    let raw = |q: &[u8]| tr(&d.format_raw(field, d.get(q, field).unwrap_or(0)));
    (raw(base), raw(p))
}

/// The page and section a field is shown in.
pub fn section_of(c: &Curation, field: usize) -> Option<(usize, &'static str)> {
    c.pages.iter().enumerate().find_map(|(i, page)| {
        page.sections
            .iter()
            .find(|s| s.fields.contains(&field))
            .map(|s| (i, s.title))
    })
}

/// A section's heading, or `None` for one named like its page (no second
/// heading).
pub fn section_heading(page: &Page, section: usize) -> Option<&'static str> {
    page.sections
        .get(section)
        .map(|s| s.title)
        .filter(|t| *t != page.title)
}

/// The page an editor opens on: the oscillators, or the first.
pub fn first_page(c: &Curation) -> usize {
    page_with_icon(c, "oscyl-oscillator-symbolic").unwrap_or(0)
}

/// The page a program opens at for someone who mainly does `role` (the
/// welcome's answer): its start page when the synth has it, else
/// [`first_page`].
pub fn start_page(c: &Curation, role: Option<crate::settings::Role>) -> usize {
    role.and_then(|r| page_with_icon(c, r.start_page_icon()))
        .unwrap_or_else(|| first_page(c))
}

fn page_with_icon(c: &Curation, icon: &str) -> Option<usize> {
    c.pages.iter().position(|p| p.icon == Some(icon))
}

/// Whether `field` differs between `p` and `base`.
pub fn is_edited(d: &dyn Driver, field: usize, p: &[u8], base: &[u8]) -> bool {
    d.get(p, field) != d.get(base, field)
}

/// How many fields of `page` differ between `p` and `base`.
pub fn stage_edits(d: &dyn Driver, page: &Page, p: &[u8], base: &[u8]) -> usize {
    page.sections
        .iter()
        .flat_map(|s| &s.fields)
        .filter(|&&f| is_edited(d, f, p, base))
        .count()
}

/// Whether a section has anything that applies under `p` (layer B of a
/// one-layer program has nothing): editors hide it otherwise.
pub fn section_shown(d: &dyn Driver, fields: &[usize], p: &[u8]) -> bool {
    fields.iter().any(|&f| d.relevant(p, f))
}

/// Page `page`'s picture of `p`, its curves about `points` long.
pub fn graph(d: &dyn Driver, p: &[u8], page: usize, points: usize) -> Option<Graph> {
    d.graphs(p, page, points).into_iter().next()
}

/// The page whose pictures tell `programs` apart most (a dealt hand's
/// tiles each show this one page), `signature` (the character's own) when
/// none differs more. Pictures are about `points` long. Both apps draw a
/// hand's tiles with it.
pub fn telling_page<P: AsRef<[u8]>>(
    d: &dyn Driver,
    programs: &[P],
    signature: usize,
    points: usize,
) -> usize {
    let score = |page: usize| {
        let pictures: Vec<Option<Picture>> = programs
            .iter()
            .map(|p| graph(d, p.as_ref(), page, points).as_ref().map(picture))
            .collect();
        spread(&pictures)
    };
    let mut best = (score(signature), signature);
    for page in (0..d.curation().pages.len()).filter(|p| *p != signature) {
        let s = score(page);
        if s > best.0 {
            best = (s, page);
        }
    }
    best.1
}

/// How much the pictures differ: the summed squared distance of their
/// points from the mean picture. Zero when one is missing or they are not
/// drawn alike (marks of other lengths), so such a page never wins.
pub fn spread(pictures: &[Option<Picture>]) -> f64 {
    let flat: Option<Vec<Vec<f32>>> = pictures
        .iter()
        .map(|g| {
            g.as_ref().map(|g| {
                g.marks
                    .iter()
                    .flat_map(|m| m.points.iter().flat_map(|&(x, y)| [x, y]))
                    .collect()
            })
        })
        .collect();
    let Some(flat) = flat else { return 0.0 };
    let len = flat.first().map_or(0, Vec::len);
    if flat.len() < 2 || len == 0 || flat.iter().any(|f| f.len() != len) {
        return 0.0;
    }
    let n = flat.len() as f64;
    (0..len)
        .map(|i| {
            let mean = flat.iter().map(|f| f64::from(f[i])).sum::<f64>() / n;
            flat.iter()
                .map(|f| (f64::from(f[i]) - mean).powi(2))
                .sum::<f64>()
        })
        .sum()
}

/// What handle `h` moves, with its values: "Cutoff 618 · Resonance 255".
pub fn handle_value(d: &dyn Driver, p: &[u8], h: &Handle) -> String {
    [h.x, h.y]
        .into_iter()
        .flatten()
        .map(|f| format!("{} {}", tr(d.short_label(f)), tr(&d.format_value(p, f))))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The connected synth that plays a program of `driver`: the active one
/// when it is that model, else the first of that model.
pub fn target_of<'a, Id: Copy + PartialEq + 'a>(
    synths: impl IntoIterator<Item = (Id, &'a str)> + Clone,
    active: Option<Id>,
    driver: &str,
) -> Option<Id> {
    synths
        .clone()
        .into_iter()
        .find(|(id, d)| Some(*id) == active && *d == driver)
        .or_else(|| synths.into_iter().find(|(_, d)| *d == driver))
        .map(|(id, _)| id)
}

/// What an edit should play: the field's live messages when the synth
/// takes it while notes sound, else the whole program in the edit buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audition {
    Live(Vec<Vec<u8>>),
    Program(Vec<u8>),
}

/// What to send once `field` of `p` changed (channel 0, as the desktop).
pub fn audition_of(d: &dyn Driver, p: &[u8], field: usize) -> Audition {
    match d.live_messages(p, field, 0) {
        Some(m) => Audition::Live(m),
        None => Audition::Program(p.to_vec()),
    }
}

/// How a mark of a [`Picture`] is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkKind {
    /// The pen line.
    Line,
    /// A soft fill under the line, then the line (a filter response).
    Area,
    /// A level bar: corners (x0, top) and (x1, 0); filled faint, top edge
    /// in pen.
    Bar,
    /// A hit or echo: a bar stroke from (x, 0) up to the point.
    Tap,
    /// A decaying wash, filled faint (a reverb tail).
    Wash,
    /// A thinner line (a modulation sweep).
    Sweep,
    /// A note block: corners (x0, y) and (x1, y), drawn as a solid block.
    Note,
}

/// One mark: its kind and points in graph coordinates (x 0..1 left to
/// right, y 0..1 bottom to top).
#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    pub kind: MarkKind,
    pub points: Vec<(f32, f32)>,
}

/// The words a graph's scale prints, for the app to translate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Legend {
    Hz100,
    Khz1,
    Khz10,
    Attack,
    Decay,
    Sustain,
    Release,
    Note,
    Time,
    Pitch,
    Percent0,
    Percent50,
    Percent100,
}

/// A tick under the graph at `x`, with an optional legend centred on
/// `label_x` that the app prints when `room` (graph widths) allows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    pub x: f32,
    pub legend: Option<Legend>,
    pub label_x: f32,
    /// The width the legend may take, in graph widths (1: any).
    pub room: f32,
}

/// Where a graph's scale goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    /// Nothing (a wave).
    None,
    /// Ticks and legends under the graph.
    Below,
    /// Level legends beside it, with dashed rules.
    Side,
}

/// A page's picture as plain marks, rules and legends: what every app
/// draws, gliding between two pictures of one kind point by point.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    /// Which driver graph it is, so apps only glide between pictures of
    /// one kind (and crossfade otherwise).
    pub kind: u8,
    pub marks: Vec<Mark>,
    /// Solid rules across, at these heights (the baseline, unity gain).
    pub rules: Vec<f32>,
    /// The filter's cutoff, marked dashed (orange while edited).
    pub cutoff: Option<f32>,
    pub scale: Scale,
    pub ticks: Vec<Tick>,
    /// Legends beside the graph at heights (levels), with dashed rules
    /// above zero.
    pub levels: Vec<(f32, Legend)>,
    /// Legends at the end of the time axis and the top of the pitch axis.
    pub time: bool,
    pub pitch: bool,
}

fn tick(x: f32, legend: Option<Legend>) -> Tick {
    Tick {
        x,
        legend,
        label_x: x,
        room: 1.0,
    }
}

/// `g` as a [`Picture`].
pub fn picture(g: &Graph) -> Picture {
    let mut pic = Picture {
        kind: 0,
        marks: vec![],
        rules: vec![0.0],
        cutoff: None,
        scale: Scale::Below,
        ticks: vec![],
        levels: vec![],
        time: false,
        pitch: false,
    };
    match g {
        Graph::Wave(samples) => {
            let last = samples.len().saturating_sub(1).max(1) as f32;
            pic.marks.push(Mark {
                kind: MarkKind::Line,
                points: samples
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (i as f32 / last, (v.clamp(-1.0, 1.0) + 1.0) / 2.0))
                    .collect(),
            });
            pic.rules = vec![0.5];
            pic.scale = Scale::None;
        }
        Graph::Response {
            points,
            cutoff,
            unity,
        } => {
            pic.kind = 1;
            pic.marks.push(Mark {
                kind: MarkKind::Area,
                points: points.clone(),
            });
            pic.rules.push(*unity);
            pic.cutoff = Some(*cutoff);
            // x is log frequency over 20 Hz to 20 kHz.
            pic.ticks = [
                (100.0f32, Legend::Hz100),
                (1e3, Legend::Khz1),
                (1e4, Legend::Khz10),
            ]
            .into_iter()
            .map(|(hz, l)| tick((hz / 20.0).log10() / 3.0, Some(l)))
            .collect();
        }
        Graph::Envelope(pts) => {
            pic.kind = 2;
            pic.marks.push(Mark {
                kind: MarkKind::Line,
                points: pts.to_vec(),
            });
            let names = [
                Legend::Attack,
                Legend::Decay,
                Legend::Sustain,
                Legend::Release,
            ];
            pic.ticks = names
                .iter()
                .enumerate()
                .map(|(i, &l)| Tick {
                    x: pts[i].0,
                    legend: Some(l),
                    label_x: (pts[i].0 + pts[i + 1].0) / 2.0,
                    room: pts[i + 1].0 - pts[i].0,
                })
                .chain(std::iter::once(tick(pts[4].0, None)))
                .collect();
        }
        Graph::Levels(levels) => {
            pic.kind = 3;
            let slot = 1.0 / levels.len().max(1) as f32;
            pic.marks = levels
                .iter()
                .enumerate()
                .map(|(i, &l)| {
                    let x0 = slot * i as f32 + slot * 0.2;
                    Mark {
                        kind: MarkKind::Bar,
                        points: vec![(x0, l.clamp(0.0, 1.0)), (x0 + slot * 0.6, 0.0)],
                    }
                })
                .collect();
            pic.rules = vec![];
            pic.scale = Scale::Side;
            pic.levels = vec![
                (0.0, Legend::Percent0),
                (0.5, Legend::Percent50),
                (1.0, Legend::Percent100),
            ];
        }
        Graph::Effects { taps, tail, sweep } => {
            pic.kind = 4;
            if let Some((len, level)) = tail {
                let start = taps.first().map_or(0.03, |t| t.0);
                let mut points = vec![(start, 0.0)];
                points.extend((0..=40).map(|k| {
                    let t = k as f32 / 40.0;
                    (start + t * len, level * (-4.0 * t).exp())
                }));
                points.push((start + len, 0.0));
                pic.marks.push(Mark {
                    kind: MarkKind::Wash,
                    points,
                });
            }
            if let Some((cycles, depth)) = sweep {
                pic.marks.push(Mark {
                    kind: MarkKind::Sweep,
                    points: (0..=120)
                        .map(|k| {
                            let t = k as f32 / 120.0;
                            (
                                t,
                                0.88 + 0.07 * depth * (t * cycles * std::f32::consts::TAU).sin(),
                            )
                        })
                        .collect(),
                });
            }
            pic.marks.extend(taps.iter().map(|&(t, a)| Mark {
                kind: MarkKind::Tap,
                points: vec![(t, (a * 0.75).clamp(0.0, 1.0))],
            }));
            if let Some(&(t, _)) = taps.first() {
                pic.ticks.push(tick(t, Some(Legend::Note)));
            }
            pic.time = true;
        }
        Graph::Voices(notes) => {
            pic.kind = 5;
            pic.marks = notes
                .iter()
                .map(|n| Mark {
                    kind: MarkKind::Note,
                    points: vec![(n.start, n.pitch), (n.end, n.pitch)],
                })
                .collect();
            // Steps of the arpeggiator clock as ticks.
            pic.ticks = (1..8).map(|k| tick(k as f32 / 8.0, None)).collect();
            pic.time = true;
            pic.pitch = true;
        }
    }
    pic
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xd() -> &'static dyn Driver {
        oscyl_midi::driver("korg-minilogue-xd").unwrap()
    }

    fn field(d: &dyn Driver, name: &str) -> usize {
        d.fields().iter().position(|f| f.name == name).unwrap()
    }

    /// The spike for typed values: a value's written form typed back lands
    /// on that very value (several values written alike, a band's say, go
    /// to the one the field stands at). Run over every value of the three
    /// drivers once (1 010 621 values, all back), it is sampled here.
    #[test]
    fn parse_round_trips_every_value_of_every_field() {
        for id in [
            "korg-minilogue-xd",
            "korg-prologue",
            "sequential-prophet-rev2",
        ] {
            let d = oscyl_midi::driver(id).unwrap();
            let p = d.init_program();
            for (f, info) in d.fields().iter().enumerate() {
                let all = written(d, &p, f).unwrap();
                let sample = all.iter().step_by(97).chain(all.last());
                for (raw, _) in sample {
                    let text = format_in(d, &p, f, *raw);
                    assert_eq!(
                        parse_among(&all, *raw, &text),
                        Some(*raw),
                        "{id} {}: {text:?}",
                        info.name
                    );
                }
            }
        }
    }

    /// Two programs of the xd: the init program, and it with every dial
    /// at its top and every choice at its last option.
    fn saved_and_edited(d: &dyn Driver) -> (Vec<u8>, Vec<u8>) {
        let saved = d.init_program();
        let mut edited = saved.clone();
        for (f, info) in d.fields().iter().enumerate() {
            let _ = d.set(&mut edited, f, info.max);
        }
        (saved, edited)
    }

    #[test]
    fn blend_ends_are_saved_and_edited() {
        let d = xd();
        let (saved, edited) = saved_and_edited(d);
        assert_eq!(blend(d, &saved, &edited, 0.0), saved);
        assert_eq!(blend(d, &saved, &edited, 1.0), edited);
        assert_eq!(blend(d, &saved, &edited, -3.0), saved, "clamped");
        assert_eq!(blend(d, &saved, &edited, 7.0), edited, "clamped");
    }

    #[test]
    fn blend_midpoint_rounds_dials_and_switches_choices_at_half() {
        let d = xd();
        let (saved, edited) = saved_and_edited(d);
        let cutoff = field(d, "Cutoff");
        let (a, b) = (
            d.get(&saved, cutoff).unwrap(),
            d.get(&edited, cutoff).unwrap(),
        );
        let mid = blend(d, &saved, &edited, 0.5);
        assert_eq!(
            d.get(&mid, cutoff).unwrap(),
            (f32::from(a) + (f32::from(b) - f32::from(a)) * 0.5).round() as u16
        );
        let choice = (0..d.fields().len())
            .find(|&f| {
                widget(d.control(&edited, f)) != Widget::Dial
                    && d.get(&saved, f) != d.get(&edited, f)
            })
            .expect("a choice the edit changed");
        assert_eq!(
            d.get(&blend(d, &saved, &edited, 0.49), choice),
            d.get(&saved, choice)
        );
        assert_eq!(
            d.get(&blend(d, &saved, &edited, 0.5), choice),
            d.get(&edited, choice)
        );
    }

    #[test]
    fn xd_defaults_are_cutoff_resonance_and_four_dials() {
        let d = xd();
        let l = play_defaults(&d.curation(), d);
        assert_eq!(
            (l.x, l.y),
            (Some(field(d, "Cutoff")), Some(field(d, "Resonance")))
        );
        assert_eq!(l.macros.len(), 4);
        assert!(
            l.macros
                .iter()
                .all(|m| m.is_some_and(|f| Some(f) != l.x && Some(f) != l.y)),
            "{:?}",
            l.macros
        );
        // The large dials lead.
        let large = l
            .macros
            .iter()
            .take_while(|m| m.is_some_and(|f| d.emphasized(f)))
            .count();
        assert!(
            large >= 1 && l.macros[large..].iter().all(|m| !d.emphasized(m.unwrap())),
            "{:?}",
            l.macros
        );
    }

    #[test]
    fn defaults_without_a_two_axis_handle_leave_the_pad_unassigned() {
        let d = xd();
        let none = oscyl_driver::Curation {
            pages: vec![],
            excluded: vec![],
        };
        let l = play_defaults(&none, d);
        assert_eq!((l.x, l.y, l.macros), (None, None, vec![None; 4]));
    }

    #[test]
    fn parse_accepts_bare_numbers() {
        let d = xd();
        let p = d.init_program();
        let f = (0..d.fields().len())
            .find(|&f| format_in(d, &p, f, d.fields()[f].max).ends_with('c'))
            .expect("a field in cents");
        let raw = (d.fields()[f].min..=d.fields()[f].max)
            .find(|&r| format_in(d, &p, f, r).starts_with("+14"))
            .expect("+14 c is a value");
        assert_eq!(parse_in(d, &p, f, "14"), Some(raw));
        assert_eq!(
            parse_in(d, &p, f, " +14 C "),
            Some(raw),
            "case, spaces and the sign do not matter"
        );
    }

    #[test]
    fn parse_takes_the_nearest() {
        let d = xd();
        let p = d.init_program();
        let f = field(d, "Cutoff");
        let raw = parse_in(d, &p, f, "615.2").unwrap();
        let n = leading_number(&fold_value(&format_in(d, &p, f, raw))).unwrap();
        for r in d.fields()[f].min..=d.fields()[f].max {
            let m = leading_number(&fold_value(&format_in(d, &p, f, r))).unwrap();
            assert!((m - 615.2).abs() >= (n - 615.2).abs(), "{r} is nearer");
        }
    }

    #[test]
    fn parse_refuses_words() {
        let d = xd();
        let p = d.init_program();
        assert_eq!(parse_in(d, &p, field(d, "Cutoff"), "abc"), None);
        assert_eq!(parse_in(d, &p, field(d, "Cutoff"), ""), None);
    }

    #[test]
    fn options_and_raw_values_agree() {
        let bands = Control::Bands {
            names: &["a", "b", "c"],
            width: 10,
        };
        assert_eq!(raw_of(bands, 0, 29, 1), Some(15));
        assert_eq!(option_of(bands, 0, 15), Some(1));
        assert_eq!(option_of(bands, 0, 29), Some(2), "the last band runs on");
        assert_eq!(raw_of(Control::Toggle, 3, 4, 1), Some(4));
        assert_eq!(option_of(Control::Toggle, 3, 4), Some(1));
        let choice = Control::Choice(&["x", "y", "z", "w", "v"]);
        assert_eq!(raw_of(choice, 1, 5, 2), Some(3));
        assert_eq!(option_of(choice, 1, 3), Some(2));
        assert_eq!(widget(choice), Widget::Menu(vec!["x", "y", "z", "w", "v"]));
        assert_eq!(
            widget(Control::Choice(&["x", "y"])),
            Widget::Segments(vec!["x", "y"])
        );
        assert_eq!(raw_of(Control::Continuous, 0, 9, 1), None);
    }

    #[test]
    fn stages_count_their_edits_and_the_editor_opens_on_the_oscillators() {
        let d = xd();
        let c = d.curation();
        let base = d.init_program();
        let mut p = base.clone();
        let cutoff = field(d, "Cutoff");
        d.set(&mut p, cutoff, 700).unwrap();
        let (page, _) = section_of(&c, cutoff).unwrap();
        assert_eq!(stage_edits(d, &c.pages[page], &p, &base), 1);
        assert_eq!(stage_edits(d, &c.pages[0], &p, &base), 0);
        assert_eq!(
            c.pages[first_page(&c)].icon,
            Some("oscyl-oscillator-symbolic")
        );
        // A performer starts at the voice settings; a sound designer at the oscillators.
        use crate::settings::Role;
        assert_eq!(
            c.pages[start_page(&c, Some(Role::Perform))].icon,
            Some("oscyl-voice-symbolic")
        );
        assert_eq!(start_page(&c, Some(Role::SoundDesign)), first_page(&c));
        assert_eq!(start_page(&c, None), first_page(&c));
        let (before, after) = before_after(d, cutoff, &base, &p);
        assert_ne!(before, after);
    }

    #[test]
    fn every_graph_becomes_a_picture() {
        for d in oscyl_midi::DRIVERS {
            let p = d.init_program();
            for page in 0..d.curation().pages.len() {
                if let Some(g) = graph(*d, &p, page, 64) {
                    let pic = picture(&g);
                    assert!(
                        !pic.marks.is_empty()
                            || matches!(g, Graph::Voices(_) | Graph::Effects { .. })
                    );
                    for m in &pic.marks {
                        assert!(!m.points.is_empty());
                    }
                }
            }
        }
    }

    #[test]
    fn the_active_synth_plays_first() {
        let synths = [(1u64, "a"), (2, "b"), (3, "a")];
        assert_eq!(target_of(synths, Some(3), "a"), Some(3));
        assert_eq!(target_of(synths, Some(2), "a"), Some(1));
        assert_eq!(target_of(synths, None, "c"), None);
    }
}
