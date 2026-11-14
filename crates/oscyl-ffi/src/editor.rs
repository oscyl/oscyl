//! One program open in the editor, and a document's edits under review:
//! pages, fields and their controls, summaries, pictures and handles,
//! edits with what they play, the review and the sync plan. The decisions
//! are `oscyl-app`'s (`editor`, `review`); this carries them across.
//! Every method is cheap and synchronous unless it says it blocks.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use oscyl_app::editor::{self as ed, Audition as Play, Legend, MarkKind, Scale, Widget};
use oscyl_app::i18n::tr;
use oscyl_app::review;
use oscyl_app::state::Doc;
use oscyl_driver::{Curation, Driver};

use crate::document::Document;
use crate::synth::{FfiError, SlotProgram};

/// One section of a page.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SectionInfo {
    /// Its heading, or `None` when named like its page.
    pub title: Option<String>,
    /// Field indices, in display order.
    pub fields: Vec<u32>,
}

/// One editor page (a stage).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PageInfo {
    pub title: String,
    /// The Linux app's symbolic icon id (`oscyl-filter-symbolic`), if any.
    pub icon: Option<String>,
    /// An introduction under the heading, or empty.
    pub intro: String,
    pub sections: Vec<SectionInfo>,
}

/// How a field is operated.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum ControlKind {
    /// A range of raw values; `large` for a section's main controls.
    Dial { min: u32, max: u32, large: bool },
    /// A few named options side by side.
    Segments { options: Vec<String> },
    /// More named options than segments hold.
    Menu { options: Vec<String> },
    /// Off / On.
    Toggle,
}

/// A field as the program shows it now. Query again after an edit: the
/// control and the label can change with other fields.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct FieldState {
    pub field: u32,
    pub label: String,
    pub control: ControlKind,
    pub raw: u32,
    pub saved_raw: u32,
    /// The option shown (not for a dial).
    pub option: Option<u32>,
    pub saved_option: Option<u32>,
    pub text: String,
    pub saved_text: String,
    /// Whether it does anything under the program's other settings.
    pub relevant: bool,
    pub edited: bool,
    /// What it does, and a tip, or empty.
    pub help: String,
}

/// A stage at a glance.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StageState {
    pub summary: String,
    /// The shortest summary that still reads (rails, narrow columns).
    pub short_summary: String,
    pub edits: u32,
    /// Per section, whether anything in it applies.
    pub sections_shown: Vec<bool>,
}

/// How a picture's mark is drawn (see `oscyl_app::editor::MarkKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MarkStyle {
    Line,
    Area,
    Bar,
    Tap,
    Wash,
    Sweep,
    Note,
}

/// A word a graph's scale prints; the app translates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum AxisWord {
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

/// One mark, points as x0, y0, x1, y1, … in graph coordinates (x 0..1
/// left to right, y 0..1 bottom to top).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GraphMark {
    pub style: MarkStyle,
    pub points: Vec<f32>,
}

/// A tick under the graph, and its word centred on `label_x` when it
/// fits in `room` graph widths.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GraphTick {
    pub x: f32,
    pub word: Option<AxisWord>,
    pub label_x: f32,
    pub room: f32,
}

/// A level legend beside the graph.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GraphLevel {
    pub y: f32,
    pub word: AxisWord,
}

/// A page's picture as marks, rules and legends. Two pictures of one
/// `kind` with the same point counts glide point by point; others
/// crossfade.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GraphShape {
    pub kind: u8,
    pub marks: Vec<GraphMark>,
    /// Solid rules across, at these heights.
    pub rules: Vec<f32>,
    pub cutoff: Option<f32>,
    /// Ticks below (true) or level legends beside (false); neither for a
    /// wave (no ticks, no levels).
    pub below: bool,
    pub ticks: Vec<GraphTick>,
    pub levels: Vec<GraphLevel>,
    /// The time axis' word at its end, the pitch axis' at its top.
    pub time: bool,
    pub pitch: bool,
}

/// A stage's picture now and, when it differs, as saved.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StageGraph {
    pub now: Option<GraphShape>,
    pub saved: Option<GraphShape>,
}

/// What changed after an edit, in one call: every stage at a glance, and
/// the fields, pictures and handles of `pages` (the stages on screen), in
/// that order.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PanelUpdate {
    pub stages: Vec<StageState>,
    pub fields: Vec<FieldState>,
    pub graphs: Vec<StageGraph>,
    pub handles: Vec<Vec<HandleInfo>>,
}

/// The editor's title and top row, in one call.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct EditorFacts {
    pub name: String,
    pub slot: String,
    pub read_only: bool,
    pub loaded: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub comparing: bool,
    /// Edited programs in the document.
    pub edited_count: u32,
}

/// A draggable point on a stage's picture.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct HandleInfo {
    /// What it moves ("Cutoff and resonance").
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub moves_x: bool,
    pub moves_y: bool,
    /// Its values, for the label beside it and screen readers.
    pub value: String,
}

/// A field and the raw value a handle drag gives it.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct FieldChange {
    pub field: u32,
    pub raw: u32,
}

/// What an edit plays on the synth: the field's live messages (send with
/// `Synth.live`) or the whole program (send with `Synth.audition`).
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum Audition {
    Live { messages: Vec<Vec<u8>> },
    Program { data: Vec<u8> },
}

impl From<Play> for Audition {
    fn from(p: Play) -> Audition {
        match p {
            Play::Live(messages) => Audition::Live { messages },
            Play::Program(data) => Audition::Program { data },
        }
    }
}

/// One change in the review.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ReviewChange {
    /// The field, or `None` for the name.
    pub field: Option<u32>,
    /// The stage's title, if the editor shows the field.
    pub stage: Option<String>,
    pub stage_index: Option<u32>,
    /// "Filter · Cutoff", or "Name".
    pub title: String,
    pub before: String,
    pub after: String,
}

/// An edited program with its changes.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ReviewProgram {
    pub index: u32,
    pub slot: String,
    pub name: String,
    pub changes: Vec<ReviewChange>,
}

/// What a sync writes: the edited programs' positions and slot programs.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SyncPlan {
    pub indices: Vec<u32>,
    pub programs: Vec<SlotProgram>,
}

/// A connected synth, for [`audition_target`].
#[derive(Debug, Clone, uniffi::Record)]
pub struct SynthRef {
    pub id: u64,
    pub driver_id: String,
}

/// The synth that plays a program of `driver_id`: `active` when it is that
/// model, else the first of that model.
#[uniffi::export]
pub fn audition_target(
    synths: Vec<SynthRef>,
    active: Option<u64>,
    driver_id: String,
) -> Option<u64> {
    ed::target_of(
        synths.iter().map(|s| (s.id, s.driver_id.as_str())),
        active,
        &driver_id,
    )
}

/// Back up the saved versions of `doc`'s programs `indices` (a synth's
/// slots, before a sync writes them) as a new file in folder `dir`;
/// returns its path. Blocks (writes a file).
#[uniffi::export]
pub fn write_backup(dir: String, doc: &Document, indices: Vec<u32>) -> Result<String, FfiError> {
    let (d, lib) = doc.with(|d| {
        let at: Vec<usize> = indices.iter().map(|&i| i as usize).collect();
        (d.driver, d.backup_of(&at))
    });
    review::write_backup(std::path::Path::new(&dir), &review::utc_stamp(), d, &lib)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(FfiError::failed)
}

fn u(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

pub(crate) fn shape(g: &oscyl_driver::Graph) -> GraphShape {
    let pic = ed::picture(g);
    let word = |l: Legend| match l {
        Legend::Hz100 => AxisWord::Hz100,
        Legend::Khz1 => AxisWord::Khz1,
        Legend::Khz10 => AxisWord::Khz10,
        Legend::Attack => AxisWord::Attack,
        Legend::Decay => AxisWord::Decay,
        Legend::Sustain => AxisWord::Sustain,
        Legend::Release => AxisWord::Release,
        Legend::Note => AxisWord::Note,
        Legend::Time => AxisWord::Time,
        Legend::Pitch => AxisWord::Pitch,
        Legend::Percent0 => AxisWord::Percent0,
        Legend::Percent50 => AxisWord::Percent50,
        Legend::Percent100 => AxisWord::Percent100,
    };
    GraphShape {
        kind: pic.kind,
        marks: pic
            .marks
            .into_iter()
            .map(|m| GraphMark {
                style: match m.kind {
                    MarkKind::Line => MarkStyle::Line,
                    MarkKind::Area => MarkStyle::Area,
                    MarkKind::Bar => MarkStyle::Bar,
                    MarkKind::Tap => MarkStyle::Tap,
                    MarkKind::Wash => MarkStyle::Wash,
                    MarkKind::Sweep => MarkStyle::Sweep,
                    MarkKind::Note => MarkStyle::Note,
                },
                points: m.points.into_iter().flat_map(|(x, y)| [x, y]).collect(),
            })
            .collect(),
        rules: pic.rules,
        cutoff: pic.cutoff,
        below: pic.scale == Scale::Below,
        ticks: pic
            .ticks
            .into_iter()
            .map(|t| GraphTick {
                x: t.x,
                word: t.legend.map(word),
                label_x: t.label_x,
                room: t.room,
            })
            .collect(),
        levels: pic
            .levels
            .into_iter()
            .map(|(y, l)| GraphLevel { y, word: word(l) })
            .collect(),
        time: pic.time,
        pitch: pic.pitch,
    }
}

/// Program `index` of a [`Document`], open in the editor. Edits go
/// through the document (change tracking, undo). Made by
/// [`Document::editor`]; a document that reorders or removes programs
/// needs a new one.
#[derive(uniffi::Object)]
pub struct ProgramEditor {
    doc: Arc<Document>,
    index: usize,
    driver: &'static dyn Driver,
    curation: Curation,
    /// Where the program shown stands between saved (0) and the edit (1),
    /// as f32 bits: below 1 it is compared or morphed, everything reads the
    /// blend and edits are ignored.
    blend: AtomicU32,
}

impl ProgramEditor {
    /// The program shown and its saved version: both saved while comparing.
    fn pair(&self) -> (Vec<u8>, Vec<u8>) {
        let t = self.blend();
        self.doc.with(|d| {
            let base = d.base(self.index).map(<[u8]>::to_vec).unwrap_or_default();
            let work = d.work(self.index).map(<[u8]>::to_vec).unwrap_or_default();
            let shown = if t < 1.0 {
                ed::blend(self.driver, &base, &work, t)
            } else {
                work
            };
            (shown, base)
        })
    }

    fn state(&self, field: usize, p: &[u8], base: &[u8]) -> FieldState {
        let d = self.driver;
        let spec = d.fields().get(field).copied();
        let (min, max) = spec.map_or((0, 0), |f| (f.min, f.max));
        let control = d.control(p, field);
        let raw = d.get(p, field).unwrap_or(min);
        let saved_raw = d.get(base, field).unwrap_or(min);
        let words = |names: Vec<&'static str>| names.into_iter().map(tr).collect();
        let h = d.help(field);
        FieldState {
            field: u(field),
            label: tr(&d.label_in(p, field)),
            control: match ed::widget(control) {
                Widget::Dial => ControlKind::Dial {
                    min: min.into(),
                    max: max.into(),
                    large: d.emphasized(field),
                },
                Widget::Segments(n) => ControlKind::Segments { options: words(n) },
                Widget::Menu(n) => ControlKind::Menu { options: words(n) },
                Widget::Toggle => ControlKind::Toggle,
            },
            raw: raw.into(),
            saved_raw: saved_raw.into(),
            option: ed::option_of(control, min, raw).map(u),
            // The saved value reads as the saved program's own control does.
            saved_option: ed::option_of(d.control(base, field), min, saved_raw).map(u),
            text: tr(&d.format_value(p, field)),
            saved_text: tr(&d.format_value(base, field)),
            relevant: d.relevant(p, field),
            edited: raw != saved_raw,
            help: [h.summary, h.detail]
                .into_iter()
                .filter(|t| !t.is_empty())
                .map(tr)
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    /// Change the program (one undo step per field until [`Self::seal`]),
    /// unless comparing, read-only or not read yet.
    fn apply(&self, field: usize, raw: u16) -> Result<Option<Audition>, FfiError> {
        self.apply_all(&[(field, raw)])
    }

    /// Set `changes` (one gesture, so one undo step with what came before
    /// it until a seal); returns what to play: every field's live messages
    /// when the synth takes them all live, else the whole program.
    fn apply_all(&self, changes: &[(usize, u16)]) -> Result<Option<Audition>, FfiError> {
        if self.comparing() || changes.is_empty() {
            return Ok(None);
        }
        self.doc
            .edit_loaded(self.index, |d| {
                changes
                    .iter()
                    .try_for_each(|&(f, raw)| d.set_field(self.index, f, raw))
            })
            .ok_or_else(|| FfiError::Refused {
                reason: tr("Wait until all programs have been read from the synth."),
            })?
            .map_err(|e| FfiError::Refused { reason: e.0 })?;
        let (p, _) = self.pair();
        let mut live = Vec::new();
        for &(f, _) in changes {
            match ed::audition_of(self.driver, &p, f) {
                Play::Live(m) => live.extend(m),
                Play::Program(data) => return Ok(Some(Audition::Program { data })),
            }
        }
        Ok(Some(Audition::Live { messages: live }))
    }
}

#[uniffi::export]
impl Document {
    /// Program `index` in the editor.
    pub fn editor(self: Arc<Self>, index: u32) -> Arc<ProgramEditor> {
        let driver = self.with(|d| d.driver);
        Arc::new(ProgramEditor {
            doc: self,
            index: index as usize,
            driver,
            curation: driver.curation(),
            blend: AtomicU32::new(1.0f32.to_bits()),
        })
    }

    /// Every edited program with its changes, in document order.
    pub fn review(&self) -> Vec<ReviewProgram> {
        self.with(|d| {
            let c = d.driver.curation();
            d.edited_indices()
                .into_iter()
                .map(|i| ReviewProgram {
                    index: u(i),
                    slot: oscyl_app::slots::slot_label(d.driver, i),
                    name: d.name(i),
                    changes: match (d.work(i), d.base(i)) {
                        (Some(p), Some(base)) => review::changes(d.driver, &c, p, base)
                            .into_iter()
                            .map(|ch| ReviewChange {
                                field: ch.field.map(u),
                                stage: ch.stage.map(|s| tr(c.pages[s].title)),
                                stage_index: ch.stage.map(u),
                                title: ch.title,
                                before: ch.before,
                                after: ch.after,
                            })
                            .collect(),
                        _ => vec![],
                    },
                })
                .collect()
        })
    }

    /// What syncing a synth's slots writes; refused until every edited
    /// program has been read.
    pub fn sync_plan(&self) -> Result<SyncPlan, FfiError> {
        let loaded: Vec<bool> = (0..self.len()).map(|i| self.is_loaded(i)).collect();
        self.with(|d| review::sync_plan(d, |i| loaded.get(i).copied().unwrap_or(false)))
            .map(|p| SyncPlan {
                indices: p.indices.into_iter().map(u).collect(),
                programs: p
                    .batch
                    .into_iter()
                    .map(|(slot, data)| SlotProgram { slot, data })
                    .collect(),
            })
            .map_err(|reason| FfiError::Refused { reason })
    }

    /// Every program is kept as it is now: nothing shows as edited.
    pub fn mark_saved(&self) {
        self.with_mut(Doc::mark_saved);
    }
}

#[uniffi::export]
impl ProgramEditor {
    pub fn index(&self) -> u32 {
        u(self.index)
    }

    /// The program's name and slot as shown.
    pub fn name(&self) -> String {
        self.doc.with(|d| d.name(self.index))
    }

    pub fn slot(&self) -> String {
        self.doc.slot_label(u(self.index))
    }

    /// A factory slot: shown, never edited.
    pub fn read_only(&self) -> bool {
        self.doc.is_read_only(u(self.index))
    }

    /// Read from the synth (always in a library).
    pub fn loaded(&self) -> bool {
        self.doc.is_loaded(u(self.index))
    }

    /// The program has edits.
    pub fn edited(&self) -> bool {
        self.doc.is_edited(u(self.index))
    }

    pub fn pages(&self) -> Vec<PageInfo> {
        self.curation
            .pages
            .iter()
            .map(|page| PageInfo {
                title: tr(page.title),
                icon: page.icon.map(str::to_string),
                intro: tr(page.help),
                sections: (0..page.sections.len())
                    .map(|s| SectionInfo {
                        title: ed::section_heading(page, s).map(tr),
                        fields: page.sections[s].fields.iter().map(|&f| u(f)).collect(),
                    })
                    .collect(),
            })
            .collect()
    }

    /// Every stage, and the fields, pictures (curves about `points` long)
    /// and handles of `pages`: what one edit can change, in one call.
    pub fn update(&self, pages: Vec<u32>, points: u32) -> PanelUpdate {
        PanelUpdate {
            stages: self.stages(),
            fields: pages.iter().flat_map(|&p| self.fields(p)).collect(),
            graphs: pages.iter().map(|&p| self.graphs(p, points)).collect(),
            handles: pages.iter().map(|&p| self.handles(p)).collect(),
        }
    }

    /// The title's and top row's facts.
    pub fn facts(&self) -> EditorFacts {
        EditorFacts {
            name: self.name(),
            slot: self.slot(),
            read_only: self.read_only(),
            loaded: self.loaded(),
            can_undo: self.doc.can_undo(),
            can_redo: self.doc.can_redo(),
            comparing: self.comparing(),
            edited_count: self.doc.edited_count(),
        }
    }

    /// The page the editor opens on.
    pub fn first_page(&self) -> u32 {
        u(ed::first_page(&self.curation))
    }

    /// The page to open at for someone who mainly does `role` (the
    /// welcome's answer; none: [`Self::first_page`]).
    pub fn start_page(&self, role: Option<crate::settings::Role>) -> u32 {
        u(ed::start_page(&self.curation, role.map(Into::into)))
    }

    /// Field `field` now.
    pub fn field(&self, field: u32) -> FieldState {
        let (p, base) = self.pair();
        self.state(field as usize, &p, &base)
    }

    /// Every field of page `page` now, in section order.
    pub fn fields(&self, page: u32) -> Vec<FieldState> {
        let (p, base) = self.pair();
        self.curation
            .pages
            .get(page as usize)
            .map(|pg| {
                pg.sections
                    .iter()
                    .flat_map(|s| &s.fields)
                    .map(|&f| self.state(f, &p, &base))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Every stage at a glance.
    pub fn stages(&self) -> Vec<StageState> {
        let (p, base) = self.pair();
        let d = self.driver;
        let t = |s: &str| tr(s);
        self.curation
            .pages
            .iter()
            .enumerate()
            .map(|(i, page)| StageState {
                summary: d.page_summary_in(&p, i, false, &t),
                short_summary: d.page_summary_in(&p, i, true, &t),
                edits: u(ed::stage_edits(d, page, &p, &base)),
                sections_shown: page
                    .sections
                    .iter()
                    .map(|s| ed::section_shown(d, &s.fields, &p))
                    .collect(),
            })
            .collect()
    }

    /// Page `page`'s picture now (curves about `points` long) and, when it
    /// differs, as saved.
    pub fn graphs(&self, page: u32, points: u32) -> StageGraph {
        let (p, base) = self.pair();
        let (page, points) = (page as usize, points as usize);
        let now = ed::graph(self.driver, &p, page, points);
        let saved = ed::graph(self.driver, &base, page, points).filter(|b| Some(b) != now.as_ref());
        StageGraph {
            now: now.as_ref().map(shape),
            saved: saved.as_ref().map(shape),
        }
    }

    /// The handles on page `page`'s picture.
    pub fn handles(&self, page: u32) -> Vec<HandleInfo> {
        let (p, _) = self.pair();
        self.driver
            .handles(&p, page as usize)
            .into_iter()
            .map(|h| HandleInfo {
                name: tr(h.name),
                x: h.at.0,
                y: h.at.1,
                moves_x: h.x.is_some(),
                moves_y: h.y.is_some(),
                value: ed::handle_value(self.driver, &p, &h),
            })
            .collect()
    }

    /// The fields handle `handle` of page `page` sets when dragged to
    /// (`x`, `y`), only those that change; apply each with [`Self::set`].
    pub fn drag(&self, page: u32, handle: u32, x: f32, y: f32) -> Vec<FieldChange> {
        let (p, _) = self.pair();
        let at = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
        self.driver
            .drag(&p, page as usize, handle as usize, at)
            .into_iter()
            .map(|(f, raw)| FieldChange {
                field: u(f),
                raw: raw.into(),
            })
            .collect()
    }

    /// Drag handle `handle` of page `page` to (`x`, `y`): sets the fields
    /// it moves as one step (merged with the rest of the drag until
    /// [`Self::seal`]) and returns what to play, or `None` when nothing
    /// changed or while comparing.
    pub fn drag_to(
        &self,
        page: u32,
        handle: u32,
        x: f32,
        y: f32,
    ) -> Result<Option<Audition>, FfiError> {
        let changes: Vec<(usize, u16)> = self
            .drag(page, handle, x, y)
            .into_iter()
            .map(|c| (c.field as usize, u16::try_from(c.raw).unwrap_or(u16::MAX)))
            .collect();
        // Both of a handle's fields belong to the one drag.
        self.doc.with_mut(Doc::gesture);
        self.apply_all(&changes)
    }

    /// Set several fields together (the play page's pad): one step with the
    /// rest of the gesture until [`Self::seal`], as a handle's drag. What to
    /// play, `None` while comparing.
    pub fn set_all(&self, changes: Vec<FieldChange>) -> Result<Option<Audition>, FfiError> {
        let changes: Vec<(usize, u16)> = changes
            .into_iter()
            .map(|c| (c.field as usize, u16::try_from(c.raw).unwrap_or(u16::MAX)))
            .collect();
        self.doc.with_mut(Doc::gesture);
        self.apply_all(&changes)
    }

    /// Set a field to `raw`; returns what to play (`None` while comparing).
    /// Refused on a factory slot or one not read yet.
    pub fn set(&self, field: u32, raw: u32) -> Result<Option<Audition>, FfiError> {
        let raw = u16::try_from(raw).unwrap_or(u16::MAX);
        self.apply(field as usize, raw)
    }

    /// Pick option `option` of a segments, menu or Off/On field.
    pub fn choose(&self, field: u32, option: u32) -> Result<Option<Audition>, FfiError> {
        let (p, _) = self.pair();
        let f = field as usize;
        let Some(spec) = self.driver.fields().get(f).copied() else {
            return Ok(None);
        };
        match ed::raw_of(
            self.driver.control(&p, f),
            spec.min,
            spec.max,
            option as usize,
        ) {
            Some(raw) => self.apply(f, raw),
            None => Ok(None),
        }
    }

    /// Put a field back as saved (one undo step).
    pub fn revert_field(&self, field: u32) -> Result<Option<Audition>, FfiError> {
        let (_, base) = self.pair();
        let f = field as usize;
        let Ok(raw) = self.driver.get(&base, f) else {
            return Ok(None);
        };
        self.doc.seal();
        let played = self.apply(f, raw);
        self.doc.seal();
        played
    }

    /// End the current undo step (a gesture ended).
    pub fn seal(&self) {
        self.doc.seal();
    }

    /// `raw` for `field`, as the program shown reads it.
    /// The value of `field` that typed `text` names (its written form, or
    /// the nearest written number), in this program's context; `None` when
    /// it names none.
    pub fn parse(&self, field: u32, text: String) -> Option<u32> {
        let (p, _) = self.pair();
        ed::parse_in(self.driver, &p, field as usize, &text).map(u32::from)
    }

    pub fn format(&self, field: u32, raw: u32) -> String {
        let (p, _) = self.pair();
        ed::format_in(
            self.driver,
            &p,
            field as usize,
            u16::try_from(raw).unwrap_or(u16::MAX),
        )
    }

    /// Roll stage `page` from `seed`: its fields only, one undo step the
    /// history calls "Rolled {stage}". Refused while comparing, on a
    /// factory slot or before the program is read. What to play.
    pub fn roll(&self, page: u32, seed: u64) -> Result<Option<Audition>, FfiError> {
        if self.comparing() {
            return Ok(None);
        }
        if self.read_only() {
            return Err(FfiError::Refused {
                reason: tr("This slot is read-only (a factory bank)."),
            });
        }
        let page = page as usize;
        let title = self
            .curation
            .pages
            .get(page)
            .map(|p| tr(p.title))
            .unwrap_or_default();
        let (p, _) = self.pair();
        let rolled = oscyl_app::generator::roll_page(self.driver, &p, page, seed)
            .map_err(|e| FfiError::Refused { reason: e.0 })?;
        let label = oscyl_app::generator::rolled_label(&title);
        self.doc
            .edit_loaded(self.index, |d| {
                d.set_program_named(self.index, rolled, &label)
            })
            .ok_or_else(|| FfiError::Refused {
                reason: tr("Wait until all programs have been read from the synth."),
            })?;
        Ok(Some(Audition::Program {
            data: self.pair().0,
        }))
    }

    /// Put `data` (a snapshot of this program) back: one undo step the
    /// history calls "Recalled snapshot {letter}". Refused as [`Self::roll`]
    /// is, and for bytes the synth would not take. What to play.
    pub fn recall(&self, data: Vec<u8>, letter: String) -> Result<Option<Audition>, FfiError> {
        if self.comparing() {
            return Ok(None);
        }
        if self.read_only() {
            return Err(FfiError::Refused {
                reason: tr("This slot is read-only (a factory bank)."),
            });
        }
        self.driver
            .check_program(&data)
            .map_err(|e| FfiError::Refused { reason: e.0 })?;
        let label = oscyl_app::review::recalled_label(&letter);
        self.doc
            .edit_loaded(self.index, |d| {
                d.set_program_named(self.index, data, &label)
            })
            .ok_or_else(|| FfiError::Refused {
                reason: tr("Wait until all programs have been read from the synth."),
            })?;
        Ok(Some(Audition::Program {
            data: self.pair().0,
        }))
    }

    /// The play page's layout this model starts with (until one is chosen).
    pub fn play_defaults(&self) -> crate::settings::PlayLayout {
        ed::play_defaults(&self.curation, self.driver).into()
    }

    /// Hold to compare: show (and play) the saved program, ignore edits.
    pub fn set_comparing(&self, on: bool) {
        self.set_blend(if on { 0.0 } else { 1.0 });
    }

    /// Show the program between saved (0) and the edit (1): every value,
    /// picture and the program to audition follow; edits wait until 1.
    pub fn set_blend(&self, t: f32) {
        let t = if t.is_nan() { 1.0 } else { t.clamp(0.0, 1.0) };
        self.blend.store(t.to_bits(), Ordering::Relaxed);
    }

    pub fn blend(&self) -> f32 {
        f32::from_bits(self.blend.load(Ordering::Relaxed))
    }

    pub fn comparing(&self) -> bool {
        self.blend() < 1.0
    }

    /// The program as shown, to audition (the saved one while comparing).
    pub fn program(&self) -> Vec<u8> {
        self.pair().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo(id: &str) -> Arc<Document> {
        let d = crate::synth::driver(id).unwrap();
        Document::new(Doc::from_data(d, d.demo_library(), None, "Demo").unwrap())
    }

    const DRIVERS: [&str; 3] = [
        "korg-minilogue-xd",
        "korg-prologue",
        "sequential-prophet-rev2",
    ];

    fn a_dial(e: &ProgramEditor) -> (u32, FieldState) {
        e.pages()
            .iter()
            .enumerate()
            .find_map(|(page, _)| {
                e.fields(u(page)).into_iter().find(|f| {
                    f.relevant
                        && matches!(f.control, ControlKind::Dial { min, max, .. } if max > min + 2)
                })
            })
            .map(|f| (f.field, f))
            .expect("a dial")
    }

    #[test]
    fn every_synth_edits_through_its_document() {
        for id in DRIVERS {
            let doc = demo(id);
            let e = doc.clone().editor(0);
            assert!(!e.pages().is_empty(), "{id}");
            assert_eq!(e.stages().len(), e.pages().len());
            let (field, f) = a_dial(&e);
            let ControlKind::Dial { min, max, .. } = f.control else {
                unreachable!()
            };
            let to = if f.raw == max { min } else { max };
            let played = e.set(field, to).unwrap();
            assert!(played.is_some(), "{id}");
            let now = e.field(field);
            assert!(now.edited && now.raw == to, "{id}");
            assert!(doc.is_edited(0));
            let review = doc.review();
            assert_eq!(review.len(), 1);
            assert_eq!(review[0].changes[0].field, Some(field));
            e.seal();
            doc.undo();
            assert!(!e.field(field).edited && !doc.is_edited(0), "{id}");
        }
    }

    #[test]
    fn handles_drag_the_right_fields_and_compare_shows_saved() {
        let doc = demo("korg-minilogue-xd");
        let e = doc.clone().editor(0);
        let (page, hs) = (0..e.pages().len() as u32)
            .map(|p| (p, e.handles(p)))
            .find(|(_, h)| h.len() == 1 && h[0].moves_x && h[0].moves_y)
            .expect("a page with one two-axis handle");
        let changes = e.drag(page, 0, (hs[0].x + 0.3).min(1.0), 0.95);
        assert!(!changes.is_empty());
        let sectioned: Vec<u32> = e.pages()[page as usize]
            .sections
            .iter()
            .flat_map(|s| s.fields.clone())
            .collect();
        for c in &changes {
            assert!(sectioned.contains(&c.field));
            e.set(c.field, c.raw).unwrap();
        }
        assert!(
            e.graphs(page, 64).saved.is_some(),
            "edited: the saved shape shows"
        );
        e.set_comparing(true);
        assert!(!e.field(changes[0].field).edited, "comparing shows saved");
        assert_eq!(e.set(changes[0].field, 0).unwrap(), None, "edits ignored");
        assert!(e.graphs(page, 64).saved.is_none());
        e.set_comparing(false);
        assert!(e.field(changes[0].field).edited);
    }

    #[test]
    fn roll_is_one_undo_step_with_a_description() {
        let doc = demo("korg-minilogue-xd");
        let e = doc.clone().editor(0);
        assert!(
            (0..e.pages().len() as u32).any(|p| e.roll(p, 7).unwrap().is_some() && doc.can_undo()),
            "a page rolls"
        );
        let h = doc.history();
        assert_eq!(h.len(), 1);
        assert!(h[0].title.starts_with("Rolled "), "{}", h[0].title);
        assert!(doc.undo().is_some() && !doc.can_undo(), "one step");
    }

    #[test]
    fn recall_is_one_undo_step_with_a_description() {
        let doc = demo("korg-minilogue-xd");
        let e = doc.clone().editor(0);
        let snapshot = e.program();
        let (field, f) = a_dial(&e);
        let to = match f.control {
            ControlKind::Dial { min, max, .. } => {
                if f.raw == max {
                    min
                } else {
                    max
                }
            }
            _ => unreachable!(),
        };
        e.set(field, to).unwrap();
        e.seal();
        assert!(e.recall(snapshot.clone(), "A".into()).unwrap().is_some());
        assert_eq!(e.program(), snapshot);
        let h = doc.history();
        assert_eq!(h.last().unwrap().title, "Recalled snapshot A");
        assert!(doc.undo().is_some());
        assert_eq!(e.field(field).raw, to, "undo takes the recall back");
    }

    #[test]
    fn roll_refused_on_read_only() {
        let doc = Document::synth("sequential-prophet-rev2".into(), "Rev2".into()).unwrap();
        let factory = doc.rows().iter().position(|r| r.read_only).unwrap() as u32;
        let d = crate::synth::driver("sequential-prophet-rev2").unwrap();
        doc.receive(
            oscyl_app::slots::slots(d)[factory as usize],
            d.init_program(),
        );
        let e = doc.clone().editor(factory);
        assert!(matches!(e.roll(0, 1), Err(FfiError::Refused { .. })));
        // Not read yet: refused too.
        let e = doc.clone().editor(0);
        assert!(matches!(e.roll(0, 1), Err(FfiError::Refused { .. })));
    }

    #[test]
    fn set_all_is_one_undo_step() {
        let doc = demo("korg-minilogue-xd");
        let e = doc.clone().editor(0);
        let dials: Vec<(u32, u32)> = e
            .pages()
            .iter()
            .enumerate()
            .flat_map(|(p, _)| e.fields(p as u32))
            .filter_map(|f| match f.control {
                ControlKind::Dial { min, max, .. } => {
                    Some((f.field, if f.raw == max { min } else { max }))
                }
                _ => None,
            })
            .take(2)
            .collect();
        for step in 0..3 {
            let changes = dials
                .iter()
                .map(|&(field, raw)| FieldChange {
                    field,
                    raw: raw.saturating_sub(step),
                })
                .collect();
            e.set_all(changes).unwrap();
        }
        e.seal();
        assert_eq!(
            doc.history().len(),
            1,
            "two fields over three moves, one step"
        );
    }

    #[test]
    fn set_comparing_is_blend_zero() {
        let doc = demo("korg-minilogue-xd");
        let e = doc.clone().editor(0);
        let (field, f) = a_dial(&e);
        let (min, max) = match f.control {
            ControlKind::Dial { min, max, .. } => (min, max),
            _ => unreachable!(),
        };
        let to = if f.raw == max { min } else { max };
        e.set(field, to).unwrap();
        e.seal();
        e.set_comparing(true);
        assert_eq!(e.blend(), 0.0);
        assert_eq!(e.field(field).raw, f.raw, "compare is the saved program");
        e.set_blend(0.5);
        let mid = e.field(field).raw;
        assert!(
            mid.min(to) <= mid && (mid > f.raw.min(to) && mid < f.raw.max(to)),
            "half way: {mid} between {} and {to}",
            f.raw
        );
        assert!(e.comparing(), "a blend short of the edit refuses edits");
        assert_eq!(e.set(field, f.raw).unwrap(), None);
        e.set_comparing(false);
        assert_eq!((e.blend(), e.field(field).raw), (1.0, to));
    }

    #[test]
    fn factory_slots_refuse_and_syncs_wait_for_the_read() {
        let doc = Document::synth("sequential-prophet-rev2".into(), "Rev2".into()).unwrap();
        let factory = doc.rows().iter().position(|r| r.read_only).unwrap() as u32;
        let d = crate::synth::driver("sequential-prophet-rev2").unwrap();
        let slot = oscyl_app::slots::slots(d)[factory as usize];
        doc.receive(slot, d.init_program());
        let e = doc.clone().editor(factory);
        let (field, f) = a_dial(&e);
        assert!(matches!(
            e.set(field, f.raw + 1),
            Err(FfiError::Refused { .. })
        ));
        let e = doc.clone().editor(0);
        assert!(
            matches!(e.set(field, 1), Err(FfiError::Refused { .. })),
            "not read yet"
        );
        doc.receive(oscyl_app::slots::slots(d)[0], d.init_program());
        e.set(field, f.raw + 1).unwrap();
        let plan = doc.sync_plan().unwrap();
        assert_eq!(plan.indices, [0]);
        // Stored: what was written becomes the slot's saved version.
        let written = &plan.programs[0];
        doc.receive(written.slot, written.data.clone());
        assert!(!doc.is_edited(0));
    }

    #[test]
    fn choices_pick_options_and_backups_are_written() {
        let doc = demo("korg-minilogue-xd");
        let e = doc.clone().editor(0);
        let seg = (0..e.pages().len() as u32)
            .flat_map(|p| e.fields(p))
            .find(|f| {
                matches!(&f.control, ControlKind::Segments { options } if options.len() > 1)
                    && f.relevant
            })
            .unwrap();
        let other = if seg.option == Some(0) { 1 } else { 0 };
        e.choose(seg.field, other).unwrap();
        assert_eq!(e.field(seg.field).option, Some(other));
        assert_eq!(e.field(seg.field).saved_option, seg.option);
        let dir = std::env::temp_dir().join(format!("oscyl-ffi-backup-{}", std::process::id()));
        let path = write_backup(dir.to_string_lossy().into(), &doc, vec![0]).unwrap();
        assert!(std::path::Path::new(&path).exists());
        let _ = std::fs::remove_dir_all(dir);
        doc.mark_saved();
        assert_eq!(doc.edited_count(), 0);
        let synths = vec![SynthRef {
            id: 7,
            driver_id: "korg-minilogue-xd".into(),
        }];
        assert_eq!(
            audition_target(synths, None, "korg-minilogue-xd".into()),
            Some(7)
        );
    }
}
