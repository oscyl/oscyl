//! The Generate page: pick a character, shape it with four dials, and
//! deal six sounds (the driver's `generate`, `vary` and `breed`, dealt by
//! `oscyl_core::generate::hand`). Picking a sound plays it on
//! the synth; one can be kept over the program selected in the list (one
//! undo step), varied, or bred with another. The last hands stay
//! reachable with Back and Forward.
//!
//! The page owns no documents: it tells the app what to do through
//! [`Ask`], and the app tells it the target slot and the connection.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::glib;
use oscyl_app::generator::Hands;
use oscyl_driver::{Driver, Macros, Recipe};

use crate::callback::Slot;
use crate::editor::graph_of;
use crate::i18n::tr;
use crate::programs::category_name;
use crate::state::Program;
use crate::widgets::{Dial, Graph, Plated, SegmentTrack};

/// What the page asks the app to do.
pub enum Ask {
    /// Play this program on the synth (the whole program: a new sound).
    Audition(Program),
    /// Put it over the selected slot, as one undo step.
    /// Its synth's driver comes along: a document of another synth
    /// refuses it.
    Keep(&'static dyn Driver, Program),
    /// Show the kept program in the editor.
    Open,
    /// Keep was pressed with no slot selected.
    PickSlot,
}

/// The selected slot: where Keep puts a sound, and the base for kept
/// stages (`None` while it is still being read).
pub struct Target {
    pub slot: String,
    pub name: String,
    pub base: Option<Program>,
}

pub struct Generator {
    pub root: gtk::Widget,
    generate: gtk::Button,
    driver: Cell<&'static dyn Driver>,
    characters: gtk::FlowBox,
    dials: Vec<Dial>,
    /// One chip per editor page: keep that page from the selected program.
    chips: adw::WrapBox,
    keep_chips: RefCell<Vec<(usize, gtk::ToggleButton)>>,
    keep_caption: gtk::Label,
    /// "Hold from Slot 004": the stages' legend names the slot they come from.
    hold_legend: gtk::Label,
    offline: gtk::Label,
    sounds: gtk::FlowBox,
    sounds_plate: Plated,
    sounds_head: gtk::Widget,
    origin: gtk::Label,
    position: gtk::Label,
    back: gtk::Button,
    forward: gtk::Button,
    empty: gtk::Label,
    actions: gtk::Widget,
    keep: gtk::Button,
    /// Keep the selected sound at the end of one of Oscyl's libraries.
    keep_library: gtk::MenuButton,
    open: gtk::Button,
    amount: adw::ToggleGroup,
    breed_list: gtk::ListBox,
    breed: gtk::MenuButton,
    lamps: RefCell<Vec<gtk::Widget>>,
    character: Cell<usize>,
    macros: Cell<Macros>,
    target: RefCell<Option<Target>>,
    connected: Cell<bool>,
    /// The hands dealt, and where the page is among them (the shared model).
    hands: RefCell<Hands>,
    /// Set while tiles are rebuilt: selection signals are ignored.
    rebuilding: Cell<bool>,
    /// Set when the selected sound should not be played.
    quiet: Cell<bool>,
    on_ask: Slot<Ask>,
    me: Weak<Generator>,
}

/// A legend over a section ("CHARACTER").
fn legend(text: &str) -> gtk::Label {
    let l = gtk::Label::builder().label(text).xalign(0.0).build();
    l.add_css_class("section-heading");
    l.set_accessible_role(gtk::AccessibleRole::Heading);
    l
}

fn mini_graph() -> Graph {
    let g = Graph::new();
    g.add_css_class("mini");
    g.set_compact();
    g
}

/// Character `i`'s tile: its name, a typical result's picture, a hint.
fn character_tile(d: &dyn Driver, i: usize) -> gtk::FlowBoxChild {
    let c = d.characters()[i];
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.add_css_class("gen-tile");
    let title = gtk::Label::builder()
        .label(tr(c.title))
        .xalign(0.0)
        .css_classes(["tile-title"])
        .build();
    let picture = mini_graph();
    let recipe = Recipe {
        character: i,
        seed: 1,
        ..Recipe::default()
    };
    if let Ok(sample) = d.generate(&recipe, None) {
        picture.set_data(graph_of(d, &sample, c.signature_page.unwrap_or(0)), None);
    }
    picture.set_accessible_role(gtk::AccessibleRole::Presentation);
    let hint = gtk::Label::builder()
        .label(tr(c.hint))
        .xalign(0.0)
        .wrap(true)
        .lines(2)
        .max_width_chars(11)
        .ellipsize(pango::EllipsizeMode::End)
        .css_classes(["tile-hint"])
        .build();
    body.append(&title);
    body.append(&picture);
    body.append(&hint);
    let child = gtk::FlowBoxChild::builder().child(&body).build();
    child.update_property(&[
        gtk::accessible::Property::Label(&tr(c.title)),
        gtk::accessible::Property::Description(&tr(c.hint)),
    ]);
    child
}

/// A dealt sound's tile: name, a lamp while it plays, its category (and
/// where it was kept), and the six stages drawn small.
fn sound_tile(d: &dyn Driver, p: &[u8], kept: Option<&str>) -> (gtk::FlowBoxChild, gtk::Widget) {
    let name = d.name(p).unwrap_or_default();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.add_css_class("gen-tile");
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let title = gtk::Label::builder()
        .label(&name)
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(pango::EllipsizeMode::End)
        .css_classes(["tile-title"])
        .build();
    let lamp = gtk::Box::builder()
        .css_classes(["status-dot", "on"])
        .valign(gtk::Align::Center)
        .tooltip_text(tr("Playing on the synth"))
        .visible(false)
        .build();
    head.append(&title);
    head.append(&lamp);
    let category = d.category(p).map(category_name).unwrap_or_default();
    let caption = match kept {
        Some(slot) => tr("{category} · kept in slot {slot}")
            .replace("{category}", &category)
            .replace("{slot}", slot),
        None => category,
    };
    let legend = gtk::Label::builder()
        .label(&caption)
        .xalign(0.0)
        .ellipsize(pango::EllipsizeMode::End)
        .css_classes(["tile-legend"])
        .build();
    let band = gtk::Box::builder()
        .homogeneous(true)
        .spacing(4)
        .css_classes(["gen-band"])
        .build();
    // The band keeps the width six pages take, however many there are.
    let pages = d.curation().pages.len();
    for page in 0..pages {
        let g = mini_graph();
        g.set_min_width_em(12.0 / pages.max(6) as f32);
        g.set_data(graph_of(d, p, page), None);
        g.set_accessible_role(gtk::AccessibleRole::Presentation);
        band.append(&g);
    }
    body.append(&head);
    body.append(&legend);
    body.append(&band);
    let child = gtk::FlowBoxChild::builder().child(&body).build();
    child.update_property(&[gtk::accessible::Property::Label(
        &tr("{name}, {category}")
            .replace("{name}", &name)
            .replace("{category}", &caption),
    )]);
    (child, lamp.upcast())
}

/// A grid of `columns` tiles a row (breakpoints change it).
fn flow(label: &str, columns: u32) -> gtk::FlowBox {
    let f = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .activate_on_single_click(true)
        .homogeneous(true)
        .min_children_per_line(columns)
        .max_children_per_line(columns)
        .column_spacing(3)
        .row_spacing(3)
        .build();
    f.update_property(&[gtk::accessible::Property::Label(label)]);
    f
}

/// A framed grid of tiles whose selection is the sliding plate.
fn framed(f: &gtk::FlowBox) -> (Plated, gtk::Box) {
    let plate = Plated::flow(f);
    let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
    frame.add_css_class("gen-frame");
    frame.append(&plate);
    (plate, frame)
}

fn index_of(f: &gtk::FlowBox) -> Option<usize> {
    f.selected_children()
        .first()
        .and_then(|c| usize::try_from(c.index()).ok())
}

impl Generator {
    pub fn new(driver: &'static dyn Driver) -> Rc<Generator> {
        let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
        page.add_css_class("editor");
        page.add_css_class("generator");

        // Head.
        let title = gtk::Label::builder()
            .label(tr("Generate"))
            .xalign(0.0)
            .css_classes(["gen-title"])
            .build();
        title.set_accessible_role(gtk::AccessibleRole::Heading);
        let intro = gtk::Label::builder()
            .label(tr(
                "Pick a character, shape it, and press Generate. Every sound plays on the synth as you pick it.",
            ))
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label", "gen-intro"])
            .build();
        page.append(&title);
        page.append(&intro);

        // Character.
        page.append(&legend(&tr("Character")));
        let characters = flow(&tr("Character"), 6);
        let (_, frame) = framed(&characters);
        page.append(&frame);

        // Shape, and the action.
        page.append(&legend(&tr("Shape")));
        let shape_row = gtk::Box::new(gtk::Orientation::Horizontal, 24);
        shape_row.add_css_class("gen-shape");
        // Two pairs of dials, side by side; stacked on phones.
        let dial_panel = gtk::Box::builder()
            .homogeneous(true)
            .spacing(8)
            .hexpand(true)
            .css_classes(["section-panel", "gen-dials"])
            .build();
        let dial_pairs = [
            gtk::Box::builder().homogeneous(true).spacing(8).build(),
            gtk::Box::builder().homogeneous(true).spacing(8).build(),
        ];
        for pair in &dial_pairs {
            dial_panel.append(pair);
        }
        // Titles and help come from the driver (`set_driver`).
        let mut dials = Vec::new();
        for _ in 0..4 {
            let d = Dial::new("");
            d.set_range(0, 100);
            d.set_formatter(|v| format!("{v} %"));
            d.set_step_hint(5, 25);
            d.show(50, 50);
            dial_pairs[dials.len() / 2].append(&d);
            dials.push(d);
        }
        shape_row.append(&dial_panel);
        let go = gtk::Box::new(gtk::Orientation::Vertical, 8);
        go.set_valign(gtk::Align::Center);
        go.add_css_class("gen-go");
        let generate = gtk::Button::builder()
            .label(tr("Generate"))
            .tooltip_text(tr("Deal six new sounds (Space)"))
            .css_classes(["suggested-action", "gen-generate"])
            .build();
        let offline = gtk::Label::builder()
            .label(tr(
                "Offline. Connect the synth to hear each sound as you pick it.",
            ))
            .wrap(true)
            .max_width_chars(28)
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build();
        go.append(&generate);
        go.append(&offline);
        shape_row.append(&go);
        page.append(&shape_row);

        // Keep stages of the selected program.
        let hold_legend = legend(&tr("Hold from the selected program"));
        page.append(&hold_legend);
        let chips = adw::WrapBox::builder()
            .child_spacing(6)
            .line_spacing(6)
            .build();
        page.append(&chips);
        let keep_caption = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label", "caption", "gen-keep-caption"])
            .build();
        page.append(&keep_caption);

        // Sounds.
        let sounds_head = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        sounds_head.add_css_class("gen-sounds-head");
        let origin = legend(&tr("Sounds"));
        origin.set_hexpand(true);
        origin.set_ellipsize(pango::EllipsizeMode::End);
        let position = gtk::Label::builder()
            .css_classes(["dim-label", "caption", "numeric"])
            .build();
        let back = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text(tr("Previous set (Alt+Left)"))
            .css_classes(["flat"])
            .sensitive(false)
            .build();
        back.update_property(&[gtk::accessible::Property::Label(&tr("Previous set"))]);
        let forward = gtk::Button::builder()
            .icon_name("go-next-symbolic")
            .tooltip_text(tr("Next set (Alt+Right)"))
            .css_classes(["flat"])
            .sensitive(false)
            .build();
        forward.update_property(&[gtk::accessible::Property::Label(&tr("Next set"))]);
        sounds_head.append(&origin);
        sounds_head.append(&position);
        sounds_head.append(&back);
        sounds_head.append(&forward);
        page.append(&sounds_head);

        let sounds = flow(&tr("Sounds"), 3);
        let (sounds_plate, sounds_frame) = framed(&sounds);
        let empty = gtk::Label::builder()
            .label(tr("Press Generate to deal six sounds."))
            .css_classes(["dim-label", "gen-empty"])
            .build();
        sounds_frame.append(&empty);
        page.append(&sounds_frame);

        // What to do with the selected sound.
        let actions = adw::WrapBox::builder()
            .child_spacing(8)
            .line_spacing(8)
            .css_classes(["gen-actions"])
            .build();
        let keep = gtk::Button::builder()
            .label(tr("Keep in Slot…"))
            .css_classes(["gen-keep"])
            .build();
        let keep_library = gtk::MenuButton::builder()
            .label(tr("Keep in Library"))
            .tooltip_text(tr("Add this sound to the end of a library"))
            .build();
        let open = gtk::Button::builder()
            .label(tr("Open in Editor"))
            .css_classes(["flat"])
            .visible(false)
            .build();
        let amount = adw::ToggleGroup::new();
        for (i, n) in [tr("Subtle"), tr("Medium"), tr("Wild")]
            .into_iter()
            .enumerate()
        {
            amount.add(adw::Toggle::builder().label(n).name(i.to_string()).build());
        }
        amount.set_active(1);
        amount
            .upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Label(&tr("How different"))]);
        amount.set_tooltip_text(Some(&tr("How different the variations are")));
        let vary = gtk::Button::builder()
            .label(tr("Vary"))
            .tooltip_text(tr("Deal six variations of this sound"))
            .build();
        let vary_group = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        vary_group.append(&SegmentTrack::new(&amount));
        vary_group.append(&vary);
        let breed_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["gen-breed-list"])
            .build();
        breed_list.update_property(&[gtk::accessible::Property::Label(&tr("Breed with"))]);
        let popover = gtk::Popover::builder().child(&breed_list).build();
        let breed = gtk::MenuButton::builder()
            .label(tr("Breed With…"))
            .tooltip_text(tr("Deal six children of this sound and another"))
            .popover(&popover)
            .build();
        let keeps = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        keeps.append(&keep);
        keeps.append(&keep_library);
        actions.append(&keeps);
        actions.append(&open);
        actions.append(&vary_group);
        actions.append(&breed);
        actions.set_visible(false);
        page.append(&actions);

        let scroll = gtk::ScrolledWindow::builder()
            .child(&page)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        crate::widgets::smooth_wheel(&scroll);
        let bin = adw::BreakpointBin::builder()
            .child(&scroll)
            .width_request(320)
            .height_request(320)
            .build();
        // Columns follow the page's width (a BreakpointBin applies one
        // breakpoint at a time, so each carries all its rules): characters
        // 6, 4, 3 or 2 a row; sounds 3, 2 or 1 (their drawings need room); the dials and Generate
        // stack when narrow.
        let columns = |f: &gtk::FlowBox, bp: &adw::Breakpoint, n: u32| {
            bp.add_setter(f, "min-children-per-line", Some(&n.to_value()));
            bp.add_setter(f, "max-children-per-line", Some(&n.to_value()));
        };
        for (width, chars, sound_cols, narrow) in [
            (760, 4, 2, false),
            (560, 3, 2, true),
            (480, 3, 1, true),
            (400, 2, 1, true),
        ] {
            let condition = format!("max-width: {width}sp");
            let bp = adw::Breakpoint::new(
                adw::BreakpointCondition::parse(&condition).expect("valid breakpoint"),
            );
            columns(&characters, &bp, chars);
            columns(&sounds, &bp, sound_cols);
            if narrow {
                bp.add_setter(
                    &shape_row,
                    "orientation",
                    Some(&gtk::Orientation::Vertical.to_value()),
                );
                bp.add_setter(&go, "halign", Some(&gtk::Align::Start.to_value()));
                if width <= 400 {
                    bp.add_setter(
                        &dial_panel,
                        "orientation",
                        Some(&gtk::Orientation::Vertical.to_value()),
                    );
                }
                let p = page.clone();
                bp.connect_apply(move |_| p.add_css_class("narrow"));
                let p = page.clone();
                bp.connect_unapply(move |_| p.remove_css_class("narrow"));
            }
            bin.add_breakpoint(bp);
        }

        let this = Rc::new_cyclic(|me| Generator {
            root: bin.upcast(),
            generate,
            driver: Cell::new(driver),
            characters,
            dials,
            chips,
            keep_chips: RefCell::default(),
            keep_caption,
            hold_legend,
            offline,
            sounds,
            sounds_plate,
            sounds_head: sounds_head.upcast(),
            origin,
            position,
            back,
            forward,
            empty,
            actions: actions.upcast(),
            keep,
            keep_library,
            open,
            amount,
            breed_list,
            breed,
            lamps: RefCell::default(),
            character: Cell::new(0),
            macros: Cell::new(Macros::default()),
            target: RefCell::default(),
            connected: Cell::new(false),
            hands: RefCell::new(Hands::new(
                u64::from(glib::random_int()) << 32 | u64::from(glib::random_int()),
            )),
            rebuilding: Cell::new(false),
            quiet: Cell::new(false),
            on_ask: Slot::default(),
            me: me.clone(),
        });
        this.wire(&page, &vary);
        this.build(driver);
        this.set_target(None);
        this.set_connected(false);
        this
    }

    /// Generate for `driver`'s synth from now on: its characters, macro
    /// names and pages; the hands dealt for another synth go.
    pub fn set_driver(&self, driver: &'static dyn Driver) {
        if self.driver.get().info().id != driver.info().id {
            self.build(driver);
        }
    }

    fn build(&self, d: &'static dyn Driver) {
        self.driver.set(d);
        self.hands.borrow_mut().clear();
        self.rebuilding.set(true);
        self.sounds.remove_all();
        self.rebuilding.set(false);
        self.empty.set_visible(true);
        self.actions.set_visible(false);
        self.sounds_head.set_visible(true);
        self.position.set_visible(false);
        self.origin.set_label(&tr("Sounds"));
        self.characters.remove_all();
        for i in 0..d.characters().len() {
            self.characters.append(&character_tile(d, i));
        }
        self.generate.set_sensitive(!d.characters().is_empty());
        for (dial, info) in self.dials.iter().zip(d.macro_info()) {
            let title = tr(info.title);
            dial.set_label(&title);
            dial.set_tooltip_text(Some(&tr(info.help)));
            dial.update_property(&[gtk::accessible::Property::Label(&title)]);
        }
        while let Some(c) = self.chips.first_child() {
            self.chips.remove(&c);
        }
        let mut chips = Vec::new();
        for (i, page) in d.curation().pages.iter().enumerate() {
            let b = gtk::ToggleButton::builder()
                .label(tr(page.title))
                .css_classes(["chip"])
                .build();
            b.update_property(&[gtk::accessible::Property::Label(
                &tr("Hold {stage}").replace("{stage}", &tr(page.title)),
            )]);
            self.chips.append(&b);
            chips.push((i, b));
        }
        *self.keep_chips.borrow_mut() = chips;
        self.select_character(0);
        let target = self.target.borrow_mut().take();
        self.set_target(target);
    }

    /// Run `f` on the page while it is alive.
    fn weak<A>(&self, f: impl Fn(&Generator, A) + 'static) -> impl Fn(A) + 'static {
        let me = self.me.clone();
        move |a| {
            if let Some(me) = me.upgrade() {
                f(&me, a);
            }
        }
    }

    fn wire(&self, page: &gtk::Box, vary: &gtk::Button) {
        let pick = self.weak(|me, ()| {
            if let Some(i) = index_of(&me.characters) {
                me.character.set(i);
            }
        });
        self.characters
            .connect_selected_children_changed(move |_| pick(()));
        for (i, d) in self.dials.iter().enumerate() {
            let set = self.weak(move |me, v: u16| {
                let mut m = me.macros.get();
                let t = f32::from(v) / 100.0;
                match i {
                    0 => m.bright = t,
                    1 => m.movement = t,
                    2 => m.grit = t,
                    _ => m.space = t,
                }
                me.macros.set(m);
            });
            // A macro is a setting, not an edit: no "was" mark.
            d.connect_value_changed(move |d, v| {
                d.show(v, v);
                set(v);
            });
        }
        let deal = self.weak(|me, ()| me.deal());
        self.generate.connect_clicked(move |_| deal(()));
        let picked = self.weak(|me, ()| me.on_sound_selected());
        self.sounds
            .connect_selected_children_changed(move |_| picked(()));
        let back = self.weak(|me, ()| me.step(-1));
        self.back.connect_clicked(move |_| back(()));
        let forward = self.weak(|me, ()| me.step(1));
        self.forward.connect_clicked(move |_| forward(()));
        let keep = self.weak(|me, ()| me.keep_selected());
        self.keep.connect_clicked(move |_| keep(()));
        let open = self.weak(|me, ()| me.on_ask.emit(Ask::Open));
        self.open.connect_clicked(move |_| open(()));
        let vary_now = self.weak(|me, ()| me.vary_selected());
        vary.connect_clicked(move |_| vary_now(()));
        let breed = self.weak(|me, row: i32| me.breed_selected(row));
        let breed_button = self.breed.clone();
        self.breed_list.connect_row_activated(move |_, row| {
            breed_button.popdown();
            breed(row.index());
        });

        // Space deals (unless a button or field has the key); Alt+Left
        // and Alt+Right walk the hands.
        let keys = gtk::ShortcutController::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let space = self.weak(|me, ()| me.deal());
        let root = page.clone();
        keys.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("space"),
            Some(gtk::CallbackAction::new(move |_, _| {
                let busy = root.root().and_then(|r| r.focus()).is_some_and(|f| {
                    f.is::<gtk::Editable>()
                        || matches!(
                            f.accessible_role(),
                            gtk::AccessibleRole::Button
                                | gtk::AccessibleRole::ToggleButton
                                | gtk::AccessibleRole::Radio
                                | gtk::AccessibleRole::Checkbox
                        )
                });
                if busy {
                    return glib::Propagation::Proceed;
                }
                space(());
                glib::Propagation::Stop
            })),
        ));
        for (trigger, delta) in [("<Alt>Left", -1), ("<Alt>Right", 1)] {
            let step = self.weak(move |me, ()| me.step(delta));
            keys.add_shortcut(gtk::Shortcut::new(
                gtk::ShortcutTrigger::parse_string(trigger),
                Some(gtk::CallbackAction::new(move |_, _| {
                    step(());
                    glib::Propagation::Stop
                })),
            ));
        }
        page.add_controller(keys);
    }

    pub fn connect_ask(&self, f: impl Fn(Ask) + 'static) {
        self.on_ask.connect(f);
    }

    /// Start dealing from `seed` (screenshots use a fixed one).
    pub fn set_seed(&self, seed: u64) {
        self.hands.borrow_mut().set_seed(seed);
    }

    /// Put keyboard focus on Generate, so Space deals at once.
    pub fn focus(&self) {
        self.generate.grab_focus();
    }

    fn select_character(&self, c: usize) {
        if let Some(child) = self.characters.child_at_index(c as i32) {
            self.characters.select_child(&child);
        }
        self.character.set(c);
    }

    pub fn set_connected(&self, on: bool) {
        self.connected.set(on);
        self.offline.set_visible(!on);
        self.update_lamps();
    }

    pub fn set_target(&self, target: Option<Target>) {
        let (caption, can_keep_stages) = match &target {
            None => (tr("Select a program in the list to build on it."), false),
            Some(Target {
                slot, base: None, ..
            }) => (
                tr("Slot {slot} is still being read.").replace("{slot}", slot),
                false,
            ),
            Some(Target { slot, name, .. }) => (
                tr("Held stages are copied as they are from slot {slot}, “{name}”.")
                    .replace("{slot}", slot)
                    .replace("{name}", name),
                true,
            ),
        };
        self.keep_caption.set_label(&caption);
        self.hold_legend.set_label(&match &target {
            Some(t) => tr("Hold from Slot {slot}").replace("{slot}", &t.slot),
            None => tr("Hold from the selected program"),
        });
        for (_, b) in self.keep_chips.borrow().iter() {
            b.set_sensitive(can_keep_stages);
        }
        let keep_label = match &target {
            Some(t) => tr("Keep in Slot {slot}").replace("{slot}", &t.slot),
            None => tr("Keep in Slot…"),
        };
        self.keep.set_label(&keep_label);
        let reading = matches!(&target, Some(Target { base: None, .. }));
        self.keep.set_sensitive(!reading);
        self.keep.set_tooltip_text(Some(&match &target {
            Some(Target {
                base: None, slot, ..
            }) => tr("Slot {slot} is still being read.").replace("{slot}", slot),
            Some(t) => {
                tr("Replace “{name}” with this sound (you can undo it)").replace("{name}", &t.name)
            }
            None => tr("Select a slot in the program list first."),
        }));
        *self.target.borrow_mut() = target;
    }

    /// Generate pressed: six new sounds of the chosen character.
    pub fn deal(&self) {
        let d = self.driver.get();
        let c = self.character.get();
        let keep: Vec<usize> = self
            .keep_chips
            .borrow()
            .iter()
            .filter(|(_, b)| b.is_sensitive() && b.is_active())
            .map(|(s, _)| *s)
            .collect();
        let base = self.target.borrow().as_ref().and_then(|t| t.base.clone());
        let dealt = self
            .hands
            .borrow_mut()
            .deal(d, c, self.macros.get(), &keep, base.as_deref());
        if dealt {
            self.show_hand(true, true);
        }
    }

    /// The sound selected in the hand shown, with the synth it is for.
    pub fn selected_program(&self) -> Option<(&'static dyn Driver, Program)> {
        self.selected().map(|(p, _)| (self.driver.get(), p))
    }

    /// The libraries Keep in Library offers (index and name).
    pub fn set_libraries(&self, libraries: &[(usize, String)]) {
        let menu = crate::program_menu::library_targets(libraries, "win.keep-in-library", |j| {
            j.to_variant()
        });
        self.keep_library.set_menu_model(Some(&menu));
    }

    fn selected(&self) -> Option<(Program, usize)> {
        self.hands.borrow().selected()
    }

    fn vary_selected(&self) {
        let amount = (self.amount.active() as usize).min(2);
        let dealt = self.hands.borrow_mut().vary(self.driver.get(), amount);
        if dealt {
            self.show_hand(true, true);
        }
    }

    /// Breed the selected sound with the `row`-th of the others.
    fn breed_selected(&self, row: i32) {
        let other = usize::try_from(row)
            .ok()
            .and_then(|r| self.hands.borrow().others().get(r).copied());
        let Some(other) = other else { return };
        let dealt = self.hands.borrow_mut().breed(self.driver.get(), other);
        if dealt {
            self.show_hand(true, true);
        }
    }

    fn step(&self, delta: isize) {
        let moved = self.hands.borrow_mut().step(delta);
        if moved {
            self.show_hand(false, true);
        }
    }

    /// Rebuild the tiles for the current hand; `dealt` plays the deal-in,
    /// `play` sends the selected sound to the synth.
    fn show_hand(&self, dealt: bool, play: bool) {
        let hands = self.hands.borrow();
        let Some(h) = hands.current() else {
            return;
        };
        let (at, n) = hands.position();
        self.rebuilding.set(true);
        self.sounds.remove_all();
        let mut lamps = Vec::with_capacity(h.programs.len());
        for (i, p) in h.programs.iter().enumerate() {
            let kept = h
                .kept
                .as_ref()
                .filter(|(k, _)| *k == i)
                .map(|(_, s)| s.as_str());
            let (child, lamp) = sound_tile(self.driver.get(), p, kept);
            if dealt {
                child.add_css_class("deal");
                child.add_css_class(&format!("deal-{i}"));
            }
            self.sounds.append(&child);
            lamps.push(lamp);
        }
        *self.lamps.borrow_mut() = lamps;
        if let Some(child) = self.sounds.child_at_index(h.selected as i32) {
            self.sounds.select_child(&child);
        }
        self.sounds_plate.snap();
        self.rebuilding.set(false);
        self.origin.set_label(&h.origin);
        self.position.set_label(
            &tr("Set {n} of {total}")
                .replace("{n}", &(at + 1).to_string())
                .replace("{total}", &n.to_string()),
        );
        self.position.set_visible(n > 1);
        self.back.set_sensitive(at > 0);
        self.forward.set_sensitive(at + 1 < n);
        self.empty.set_visible(false);
        self.actions.set_visible(true);
        drop(hands);
        self.quiet.set(!play);
        self.on_sound_selected();
        self.quiet.set(false);
        if dealt {
            self.reveal_sounds();
        }
    }

    /// Scroll so the sounds (legend, tiles and actions) are in view, or
    /// their top when they are taller than the view; once they are laid
    /// out, frame by frame.
    fn reveal_sounds(&self) {
        let (head, actions) = (self.sounds_head.clone(), self.actions.clone());
        head.clone().add_tick_callback(move |_, _| {
            let Some(scroll) = head
                .ancestor(gtk::ScrolledWindow::static_type())
                .and_downcast::<gtk::ScrolledWindow>()
            else {
                return glib::ControlFlow::Break;
            };
            let Some(page) = scroll.child() else {
                return glib::ControlFlow::Break;
            };
            let (Some(top), Some(bottom)) =
                (head.compute_bounds(&page), actions.compute_bounds(&page))
            else {
                return glib::ControlFlow::Break;
            };
            if actions.height() == 0 {
                return glib::ControlFlow::Continue;
            }
            let adj = scroll.vadjustment();
            let (view, at) = (adj.page_size(), adj.value());
            let top = f64::from(top.y()) - 16.0;
            let bottom = f64::from(bottom.y() + bottom.height()) + 16.0;
            let want = if bottom - top > view || top < at {
                top
            } else if bottom > at + view {
                bottom - view
            } else {
                at
            };
            adj.set_value(want);
            glib::ControlFlow::Break
        });
    }

    /// A sound was picked (click, arrow keys, a new hand): play it and
    /// offer its actions.
    fn on_sound_selected(&self) {
        if self.rebuilding.get() {
            return;
        }
        let Some(i) = index_of(&self.sounds) else {
            return;
        };
        let program = {
            let mut hands = self.hands.borrow_mut();
            hands.select(i);
            let Some(h) = hands.current() else {
                return;
            };
            self.open
                .set_visible(h.kept.as_ref().is_some_and(|(k, _)| *k == i));
            self.breed_list.remove_all();
            for (j, p) in h.programs.iter().enumerate() {
                if j != i {
                    let name = self.driver.get().name(p).unwrap_or_default();
                    let row = gtk::Label::builder().label(&name).xalign(0.0).build();
                    self.breed_list.append(&row);
                }
            }
            h.programs[i].clone()
        };
        self.update_lamps();
        // Playing is the page's promise: on a new hand too, so the first
        // sound is heard at once.
        if !self.quiet.get() {
            self.on_ask.emit(Ask::Audition(program));
        }
    }

    fn update_lamps(&self) {
        let sel = index_of(&self.sounds);
        for (i, lamp) in self.lamps.borrow().iter().enumerate() {
            lamp.set_visible(self.connected.get() && sel == Some(i));
        }
    }

    pub fn keep_selected(&self) {
        let target = self.target.borrow().as_ref().map(|t| t.slot.clone());
        let Some(slot) = target else {
            self.on_ask.emit(Ask::PickSlot);
            return;
        };
        let Some((p, _)) = self.selected() else {
            return;
        };
        self.on_ask.emit(Ask::Keep(self.driver.get(), p));
        self.hands.borrow_mut().mark_kept(slot);
        // Redraw the kept tile's legend without playing it again.
        self.show_hand(false, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oscyl_app::generator::{HAND, HISTORY};
    use oscyl_core::generate::hand;

    const XD: &dyn Driver = &oscyl_minilogue_xd::DRIVER;

    fn names(g: &Generator) -> Vec<String> {
        g.hands
            .borrow()
            .current()
            .unwrap()
            .programs
            .iter()
            .map(|p| XD.name(p).unwrap())
            .collect()
    }

    /// The contract's dealer gives the xd the hands the app dealt with the
    /// xd's own generator: generate (no base), vary, and breed of
    /// generated sounds.
    #[test]
    fn same_hands_as_before_for_the_xd() {
        use oscyl_minilogue_xd::Program as XdProgram;
        use oscyl_minilogue_xd::generate as old;
        let bytes = |v: Vec<XdProgram>| -> Vec<Vec<u8>> {
            v.into_iter().map(|p| p.as_bytes().to_vec()).collect()
        };
        for (c, character) in old::Character::ALL.into_iter().enumerate() {
            for seed in [1, 7, 99] {
                let macros = Macros {
                    bright: 0.3,
                    movement: 0.8,
                    grit: 0.1,
                    space: 0.6,
                };
                let was = old::hand(character, seed, |s| {
                    old::generate(&old::Recipe {
                        character,
                        macros: old::Macros {
                            bright: 0.3,
                            movement: 0.8,
                            grit: 0.1,
                            space: 0.6,
                        },
                        seed: s,
                        keep: vec![],
                    })
                });
                let now = hand(XD, c, seed, HAND, |s| {
                    XD.generate(
                        &Recipe {
                            character: c,
                            macros,
                            seed: s,
                            keep: vec![],
                        },
                        None,
                    )
                })
                .unwrap();
                assert_eq!(now, bytes(was.clone()), "generate {c} {seed}");
                let p = was[0].clone();
                let varied = old::hand(character, seed, |s| old::vary(&p, 0.35, s));
                let now = hand(XD, c, seed, HAND, |s| XD.vary(p.as_bytes(), 0.35, s)).unwrap();
                assert_eq!(now, bytes(varied), "vary {c} {seed}");
                let q = was[1].clone();
                let bred = old::hand(character, seed, |s| {
                    old::breed(&p, &q, 0.2 + 0.6 * (s % 1000) as f32 / 1000.0, s)
                });
                let now = hand(XD, c, seed, HAND, |s| {
                    XD.breed(
                        p.as_bytes(),
                        q.as_bytes(),
                        0.2 + 0.6 * (s % 1000) as f32 / 1000.0,
                        s,
                    )
                })
                .unwrap();
                assert_eq!(now, bytes(bred), "breed {c} {seed}");
            }
        }
    }

    /// Deal, vary, breed, walk back and forward, keep. Needs a display;
    /// skipped without.
    #[test]
    fn hands_deal_vary_breed_and_keep() {
        crate::on_gtk_thread(|| {
            let g = Generator::new(XD);
            let asks = Rc::new(RefCell::new(Vec::new()));
            let a = asks.clone();
            g.connect_ask(move |ask| a.borrow_mut().push(ask));
            g.set_seed(7);

            g.deal();
            assert_eq!(g.sounds.observe_children().n_items() as usize, HAND);
            let first = names(&g);
            // The first sound of a new hand plays at once.
            assert!(
                matches!(asks.borrow().last(), Some(Ask::Audition(p)) if XD.name(p).unwrap() == first[0])
            );

            g.vary_selected();
            g.breed_selected(0);
            assert_eq!(g.hands.borrow().position(), (2, 3));
            g.step(-1);
            assert!(
                g.hands
                    .borrow()
                    .current()
                    .unwrap()
                    .origin
                    .contains(&first[0])
            );
            g.step(1);
            g.step(-1);
            g.step(-1);
            assert_eq!(names(&g), first, "Back returns to the first hand");
            g.step(1);
            assert_eq!(g.hands.borrow().position().0, 1);

            // No slot: Keep asks for one.
            g.keep_selected();
            assert!(matches!(asks.borrow().last(), Some(Ask::PickSlot)));
            g.set_target(Some(Target {
                slot: "004".into(),
                name: "Turning Tide".into(),
                base: Some(XD.init_program()),
            }));
            let n = asks.borrow().len();
            g.keep_selected();
            assert!(matches!(asks.borrow().last(), Some(Ask::Keep(..))));
            assert_eq!(asks.borrow().len(), n + 1, "keeping does not play again");
            assert!(g.open.is_visible());

            // History is bounded.
            for _ in 0..6 {
                g.deal();
            }
            assert_eq!(g.hands.borrow().position().1, HISTORY);
        });
    }
}
