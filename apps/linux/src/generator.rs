//! The Generate page: pick a character, shape it with four dials, and
//! deal six sounds (the driver's `generate`, `vary` and `breed`, dealt by
//! `oscyl_core::generate::hand`). Picking a sound plays it on
//! the synth; one can be kept over the program selected in the list (one
//! undo step), varied, bred with another, or morphed into another. The
//! last hands stay reachable with Back and Forward. A family row filters
//! the characters; Surprise picks a character and dials by itself; a dealt
//! sound's recipe code can be copied, and a code typed in deals its sound
//! again.
//!
//! On a synth that runs Oscyl's oscillator unit (minilogue xd,
//! prologue), New Oscillators deals six oscillators of its own
//! (`oscyl_app::oscillators`), varied and bred like sounds; Keep puts one
//! on the shelf and loads it into the synth. Deal with a New Oscillator
//! does that with a new one and, once the synth lists it, deals sounds
//! that use it.
//!
//! The page owns no documents: it tells the app what to do through
//! [`Ask`], and the app tells it the target slot and the connection.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::glib;
use oscyl_app::generator::Hands;
use oscyl_app::oscillators::{Oscillators, family_title};
use oscyl_driver::{Driver, Family, Macros, Recipe};
use oscyl_logue::osc::{self, Patch};

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
    /// The selected sound's recipe code went to the clipboard.
    Copied,
    /// Put this oscillator on the shelf and load it into the synth's
    /// first free oscillator slot (its synth's driver comes along).
    KeepOscillator(&'static dyn Driver, Patch),
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
    surprise: gtk::Button,
    driver: Cell<&'static dyn Driver>,
    /// "All" and one chip per family the synth has characters of.
    families: adw::WrapBox,
    family_chips: RefCell<Vec<(Option<Family>, gtk::ToggleButton)>>,
    /// The family whose characters show (`None`: all). Kept for the
    /// session, across synths; until one is picked, the first character's,
    /// so the page opens on a handful of tiles, not on all of them.
    family: Cell<Option<Family>>,
    family_picked: Cell<bool>,
    characters: gtk::FlowBox,
    characters_plate: Plated,
    dials: Vec<Dial>,
    /// One chip per editor page: keep that page from the selected program.
    chips: adw::WrapBox,
    keep_chips: RefCell<Vec<(usize, gtk::ToggleButton)>>,
    keep_caption: gtk::Label,
    /// "My units": one chip per user unit the synth holds, on unless the
    /// player excludes it; hidden while it holds none.
    units_legend: gtk::Label,
    units: adw::WrapBox,
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
    morph_list: gtk::ListBox,
    morph: gtk::MenuButton,
    /// The selected sound's recipe code and its Copy; hidden without one.
    code_row: gtk::Box,
    code: gtk::Label,
    /// "Enter a Code…": its field and what is wrong with what was typed.
    enter: gtk::MenuButton,
    code_entry: gtk::Entry,
    code_error: gtk::Label,
    lamps: RefCell<Vec<gtk::Widget>>,
    /// New oscillators: the section (shown when the synth runs Oscyl's
    /// oscillator unit), its tiles, actions and the hands behind them.
    osc_section: gtk::Widget,
    osc_tiles: gtk::FlowBox,
    osc_plate: Plated,
    osc_empty: gtk::Label,
    osc_actions: gtk::Widget,
    osc_keep: gtk::Button,
    osc_deal_with: gtk::Button,
    osc_status: gtk::Label,
    osc_breed: gtk::MenuButton,
    osc_breed_list: gtk::ListBox,
    oscillators: RefCell<Oscillators>,
    /// The oscillator Deal with a New Oscillator loaded: its sounds are
    /// dealt once the synth lists it.
    pending: RefCell<Option<String>>,
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

/// A choice that can be held with others (a stage, a unit): a checkbox
/// chip, an empty box before its word, the lifted plate with a check when
/// held (the same width either way), as on the phone.
fn check_chip(text: &str) -> gtk::ToggleButton {
    let mark = gtk::Image::from_icon_name("object-select-symbolic");
    mark.add_css_class("check");
    mark.set_valign(gtk::Align::Center);
    mark.set_accessible_role(gtk::AccessibleRole::Presentation);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.append(&mark);
    row.append(&gtk::Label::new(Some(text)));
    gtk::ToggleButton::builder()
        .child(&row)
        .css_classes(["check-chip"])
        .build()
}

/// A family in its row, as on the phone: its word alone in the legend
/// colour; the chosen one in ink at 600 over an ink index line. Both
/// weights keep their room, so choosing moves nothing.
fn family_tab(text: &str) -> gtk::ToggleButton {
    let room = gtk::Label::builder()
        .label(text)
        .css_classes(["family-room"])
        .accessible_role(gtk::AccessibleRole::Presentation)
        .build();
    room.set_opacity(0.0);
    let word = gtk::Label::new(Some(text));
    let over = gtk::Overlay::builder().child(&room).build();
    over.add_overlay(&word);
    let b = gtk::ToggleButton::builder()
        .child(&over)
        .css_classes(["flat", "family-tab"])
        .build();
    b.update_property(&[gtk::accessible::Property::Label(text)]);
    b
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

/// A dealt sound's tile, as on the phone: the hand's telling `page` drawn
/// (the one page whose pictures tell the hand apart), its name with a lamp
/// while it plays, then its category, the user units it relies on and
/// where it was kept ("Lead · with waves · kept in 004").
fn sound_tile(
    d: &dyn Driver,
    p: &[u8],
    page: usize,
    kept: Option<&str>,
    units: &[String],
) -> (gtk::FlowBoxChild, gtk::Widget) {
    let name = d.name(p).unwrap_or_default();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.add_css_class("gen-tile");
    let picture = mini_graph();
    picture.set_data(graph_of(d, p, page), None);
    picture.set_accessible_role(gtk::AccessibleRole::Presentation);
    body.append(&picture);
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
    // The user units it relies on and where it was kept, after its
    // category, in sentence case.
    let with =
        (!units.is_empty()).then(|| tr("with {units}").replace("{units}", &units.join(", ")));
    let kept = kept.map(|slot| tr("kept in {slot}").replace("{slot}", slot));
    let caption = [d.category(p).map(category_name), with, kept]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    let legend = gtk::Label::builder()
        .label(&caption)
        .xalign(0.0)
        .ellipsize(pango::EllipsizeMode::End)
        .css_classes(["tile-caption"])
        .build();
    body.append(&head);
    body.append(&legend);
    let child = gtk::FlowBoxChild::builder().child(&body).build();
    child.update_property(&[gtk::accessible::Property::Label(
        &tr("{name}, {category}")
            .replace("{name}", &name)
            .replace("{category}", &caption),
    )]);
    (child, lamp.upcast())
}

/// A new oscillator's tile, as on the phone: one cycle of its wave, its
/// name, then its family and its parameters' names.
fn oscillator_tile(p: &Patch) -> gtk::FlowBoxChild {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.add_css_class("gen-tile");
    let wave = mini_graph();
    wave.set_min_width_em(12.0);
    wave.set_data(
        crate::widgets::GraphData::Wave(osc::render::picture(p, 128)),
        None,
    );
    wave.set_accessible_role(gtk::AccessibleRole::Presentation);
    let title = gtk::Label::builder()
        .label(&p.name)
        .xalign(0.0)
        .ellipsize(pango::EllipsizeMode::End)
        .css_classes(["tile-title"])
        .build();
    let family = family_title(p.family);
    let caption = std::iter::once(family.as_str())
        .chain(p.params.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" · ");
    let hint = gtk::Label::builder()
        .label(&caption)
        .xalign(0.0)
        .wrap(true)
        .lines(2)
        .ellipsize(pango::EllipsizeMode::End)
        .css_classes(["tile-caption"])
        .build();
    body.append(&wave);
    body.append(&title);
    body.append(&hint);
    let child = gtk::FlowBoxChild::builder().child(&body).build();
    child.update_property(&[
        gtk::accessible::Property::Label(
            &tr("{name}, {category}")
                .replace("{name}", &p.name)
                .replace("{category}", &family),
        ),
        gtk::accessible::Property::Description(
            &tr("Parameters: {names}").replace("{names}", &p.params.join(", ")),
        ),
    ]);
    child
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
        let families = adw::WrapBox::builder()
            .child_spacing(6)
            .line_spacing(6)
            .css_classes(["gen-families"])
            .build();
        let row = families.upcast_ref::<gtk::Widget>();
        row.set_accessible_role(gtk::AccessibleRole::Group);
        row.update_property(&[gtk::accessible::Property::Label(&tr("Family"))]);
        page.append(&families);
        let characters = flow(&tr("Character"), 6);
        let (characters_plate, frame) = framed(&characters);
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
            d.set_revertible(false);
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
        let surprise = gtk::Button::builder()
            .child(
                &adw::ButtonContent::builder()
                    .icon_name("oscyl-dice-symbolic")
                    .label(tr("Surprise Me"))
                    .build(),
            )
            .tooltip_text(tr("Pick a character and dials for me, and deal"))
            .build();
        // A code typed or pasted deals its sound again.
        let code_entry = gtk::Entry::builder()
            .placeholder_text(tr("Recipe code"))
            .width_chars(26)
            .input_purpose(gtk::InputPurpose::FreeForm)
            .build();
        code_entry.update_property(&[gtk::accessible::Property::Label(&tr("Recipe code"))]);
        let code_deal = gtk::Button::builder().label(tr("Deal")).build();
        let code_field = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        code_field.append(&code_entry);
        code_field.append(&code_deal);
        let code_error = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .max_width_chars(30)
            .visible(false)
            .css_classes(["caption", "gen-code-error"])
            .build();
        // Said at once when it appears.
        code_error.set_accessible_role(gtk::AccessibleRole::Alert);
        let code_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        code_box.add_css_class("gen-code-box");
        code_box.append(&code_field);
        code_box.append(&code_error);
        let enter = gtk::MenuButton::builder()
            .label(tr("Enter a Code…"))
            .tooltip_text(tr("Deal the sound a recipe code names"))
            .popover(&gtk::Popover::builder().child(&code_box).build())
            .css_classes(["outlined"])
            .build();
        go.append(&generate);
        go.append(&surprise);
        go.append(&enter);
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

        // The synth's own units, as sources.
        let units_legend = legend(&tr("My units"));
        units_legend.set_tooltip_text(Some(&tr(
            "User oscillators and effects the synth holds. About one sound in three uses one.",
        )));
        units_legend.set_visible(false);
        page.append(&units_legend);
        let units = adw::WrapBox::builder()
            .child_spacing(6)
            .line_spacing(6)
            .visible(false)
            .build();
        page.append(&units);

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
        let morph_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["gen-breed-list"])
            .build();
        morph_list.update_property(&[gtk::accessible::Property::Label(&tr("Morph to"))]);
        let morph = gtk::MenuButton::builder()
            .label(tr("Morph To…"))
            .tooltip_text(tr("Deal six steps from this sound to another"))
            .popover(&gtk::Popover::builder().child(&morph_list).build())
            .build();
        // The selected sound's recipe code, small, with its Copy.
        let code_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        code_row.add_css_class("gen-code");
        let code_legend = gtk::Label::builder()
            .label(tr("Recipe code"))
            .css_classes(["dim-label", "caption"])
            .build();
        let code = gtk::Label::builder()
            .selectable(true)
            .ellipsize(pango::EllipsizeMode::Middle)
            .css_classes(["caption", "numeric", "gen-code-text"])
            .build();
        code.update_relation(&[gtk::accessible::Relation::LabelledBy(&[
            code_legend.upcast_ref()
        ])]);
        let copy = gtk::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text(tr("Copy the recipe code"))
            .css_classes(["flat"])
            .build();
        copy.update_property(&[gtk::accessible::Property::Label(&tr(
            "Copy the recipe code",
        ))]);
        code_row.append(&code_legend);
        code_row.append(&code);
        code_row.append(&copy);
        code_row.set_visible(false);
        let keeps = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        keeps.append(&keep);
        keeps.append(&keep_library);
        actions.append(&keeps);
        actions.append(&open);
        actions.append(&vary_group);
        actions.append(&breed);
        actions.append(&morph);
        actions.set_visible(false);
        page.append(&actions);
        page.append(&code_row);

        // New oscillators, for a synth that runs Oscyl's oscillator unit.
        let osc_section = gtk::Box::new(gtk::Orientation::Vertical, 0);
        osc_section.add_css_class("gen-osc");
        osc_section.append(&legend(&tr("New Oscillators")));
        let osc_intro = gtk::Label::builder()
            .label(tr(
                "Oscillators of your own for the synth's user slots. Keep one to load it: Generate then uses it.",
            ))
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label", "gen-intro"])
            .build();
        osc_section.append(&osc_intro);
        let osc_buttons = adw::WrapBox::builder()
            .child_spacing(8)
            .line_spacing(8)
            .css_classes(["gen-osc-buttons"])
            .build();
        let osc_new = gtk::Button::builder()
            .label(tr("New Oscillators"))
            .tooltip_text(tr("Make six new oscillators"))
            .build();
        let osc_deal_with = gtk::Button::builder()
            .label(tr("Deal with a New Oscillator"))
            .build();
        osc_buttons.append(&osc_new);
        osc_buttons.append(&osc_deal_with);
        osc_section.append(&osc_buttons);
        let osc_status = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .visible(false)
            .css_classes(["dim-label", "caption", "gen-osc-status"])
            .build();
        osc_status.set_accessible_role(gtk::AccessibleRole::Status);
        osc_section.append(&osc_status);
        let osc_tiles = flow(&tr("New Oscillators"), 3);
        let (osc_plate, osc_frame) = framed(&osc_tiles);
        let osc_empty = gtk::Label::builder()
            .label(tr("Press New Oscillators to make six."))
            .css_classes(["dim-label", "gen-empty"])
            .build();
        osc_frame.append(&osc_empty);
        osc_section.append(&osc_frame);
        let osc_actions = adw::WrapBox::builder()
            .child_spacing(8)
            .line_spacing(8)
            .css_classes(["gen-actions"])
            .visible(false)
            .build();
        let osc_keep = gtk::Button::builder()
            .label(tr("Keep Oscillator"))
            .css_classes(["gen-keep"])
            .build();
        let osc_vary = gtk::Button::builder()
            .label(tr("Vary"))
            .tooltip_text(tr("Make six variations of this oscillator"))
            .build();
        let osc_breed_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["gen-breed-list"])
            .build();
        osc_breed_list.update_property(&[gtk::accessible::Property::Label(&tr("Breed with"))]);
        let osc_breed = gtk::MenuButton::builder()
            .label(tr("Breed With…"))
            .tooltip_text(tr("Make six children of this oscillator and another"))
            .popover(&gtk::Popover::builder().child(&osc_breed_list).build())
            .build();
        osc_actions.append(&osc_keep);
        osc_actions.append(&osc_vary);
        osc_actions.append(&osc_breed);
        osc_section.append(&osc_actions);
        page.append(&osc_section);

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
            columns(&osc_tiles, &bp, sound_cols);
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
            surprise,
            driver: Cell::new(driver),
            families,
            family_chips: RefCell::default(),
            family: Cell::new(None),
            family_picked: Cell::new(false),
            characters,
            characters_plate,
            dials,
            chips,
            keep_chips: RefCell::default(),
            keep_caption,
            units_legend,
            units,
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
            morph_list,
            morph,
            code_row,
            code,
            enter,
            code_entry,
            code_error,
            lamps: RefCell::default(),
            osc_section: osc_section.upcast(),
            osc_tiles,
            osc_plate,
            osc_empty,
            osc_actions: osc_actions.upcast(),
            osc_keep,
            osc_deal_with,
            osc_status,
            osc_breed,
            osc_breed_list,
            oscillators: RefCell::new(Oscillators::new(
                u64::from(glib::random_int()) << 32 | u64::from(glib::random_int()),
            )),
            pending: RefCell::default(),
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
        let me = this.weak(|me, ()| me.copy_code());
        copy.connect_clicked(move |_| me(()));
        let me = this.weak(|me, ()| me.deal_code());
        code_deal.connect_clicked(move |_| me(()));
        let me = this.weak(|me, ()| me.deal_oscillators());
        osc_new.connect_clicked(move |_| me(()));
        let me = this.weak(|me, ()| me.vary_oscillator());
        osc_vary.connect_clicked(move |_| me(()));
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
        self.code_row.set_visible(false);
        self.characters.remove_all();
        for i in 0..d.characters().len() {
            self.characters.append(&character_tile(d, i));
        }
        self.build_families(d);
        let runs = oscyl_midi::logue::platform(d).is_some_and(|p| osc::supports(&p));
        self.osc_section.set_visible(runs);
        self.set_pending(None);
        self.refresh_units();
        self.generate.set_sensitive(!d.characters().is_empty());
        self.surprise.set_sensitive(!d.characters().is_empty());
        self.enter.set_sensitive(!d.characters().is_empty());
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
            let b = check_chip(&tr(page.title));
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

    /// "My units": the user units the synth of this page's driver held
    /// when last listed, each a chip that is on unless the player excluded
    /// it (kept per synth for the session by the app's palettes).
    pub fn refresh_units(&self) {
        let d = self.driver.get();
        while let Some(c) = self.units.first_child() {
            self.units.remove(&c);
        }
        let entries = oscyl_app::palette::shared().units(d);
        for e in &entries {
            let b = check_chip(&e.unit.name);
            b.set_active(e.on);
            b.set_tooltip_text(Some(&format!("{} {}", e.section, e.unit.slot + 1)));
            b.update_property(&[gtk::accessible::Property::Label(
                &tr("Use {name} ({section})")
                    .replace("{name}", &e.unit.name)
                    .replace("{section}", &e.section),
            )]);
            let (kind, slot) = (e.unit.kind.name, e.unit.slot);
            let me = self.me.clone();
            b.connect_toggled(move |b| {
                if let Some(me) = me.upgrade() {
                    oscyl_app::palette::shared().set_on(me.driver.get(), kind, slot, b.is_active());
                }
            });
            self.units.append(&b);
        }
        self.units_legend.set_visible(!entries.is_empty());
        self.units.set_visible(!entries.is_empty());
        // The new oscillator is in a slot and its parameters known: deal
        // sounds that use it.
        let loaded = self.pending.borrow().as_ref().and_then(|name| {
            entries
                .iter()
                .find(|e| &e.unit.name == name && e.unit.params.is_some())
                .map(|e| e.unit.clone())
        });
        if let Some(unit) = loaded {
            self.set_pending(None);
            self.deal_with(unit);
        }
    }

    /// The synth could not take the new oscillator (its slots are full,
    /// the upload failed, it went): stop waiting for it. The app says why.
    pub fn oscillator_failed(&self) {
        if self.pending.borrow().is_some() {
            self.set_pending(None);
        }
    }

    /// Say `text` where the section's status goes.
    fn say(&self, text: &str) {
        self.osc_status.set_label(text);
        self.osc_status.set_visible(true);
    }

    /// Wait for oscillator `name` to be listed (or no longer, `None`),
    /// saying so under the section's buttons.
    fn set_pending(&self, name: Option<String>) {
        if let Some(n) = &name {
            self.osc_status.set_label(
                &tr("Loading “{name}”. Its sounds follow once the synth lists it.")
                    .replace("{name}", n),
            );
        }
        self.osc_status.set_visible(name.is_some());
        *self.pending.borrow_mut() = name;
    }

    /// The hands deal with the units that are on.
    fn use_units(&self) {
        let palette = oscyl_app::palette::shared().palette(self.driver.get());
        self.hands.borrow_mut().set_palette(palette);
    }

    /// The family row: "All", then each family `d` has characters of, as
    /// chips of which one is on. The family chosen before stays chosen
    /// when the synth has it.
    fn build_families(&self, d: &'static dyn Driver) {
        while let Some(c) = self.families.first_child() {
            self.families.remove(&c);
        }
        let has = |f: Family| d.characters().iter().any(|c| c.family == f);
        if !self.family_picked.get() {
            self.family.set(d.characters().first().map(|c| c.family));
        } else if self.family.get().is_some_and(|f| !has(f)) {
            self.family.set(None);
        }
        let mut chips: Vec<(Option<Family>, gtk::ToggleButton)> = vec![];
        let all =
            std::iter::once(None).chain(Family::ALL.into_iter().filter(|f| has(*f)).map(Some));
        for family in all {
            let b = family_tab(&family.map_or_else(|| tr("All"), |f| tr(f.title())));
            b.set_active(family == self.family.get());
            // One of the row is on, as radio buttons are.
            if let Some((_, first)) = chips.first() {
                b.set_group(Some(first));
            }
            let pick = self.weak(move |me, ()| {
                me.family_picked.set(true);
                me.set_family(family);
            });
            b.connect_toggled(move |b| {
                if b.is_active() {
                    pick(());
                }
            });
            self.families.append(&b);
            chips.push((family, b));
        }
        // A synth with one family (or none) needs no row.
        self.families.set_visible(chips.len() > 2);
        *self.family_chips.borrow_mut() = chips;
        self.characters.invalidate_filter();
    }

    /// Show only `family`'s characters (`None`: all). The chosen character
    /// stays when it is one of them; else the family's first is chosen.
    fn set_family(&self, family: Option<Family>) {
        self.family.set(family);
        for (f, b) in self.family_chips.borrow().iter() {
            if *f == family && !b.is_active() {
                b.set_active(true);
            }
        }
        self.characters.invalidate_filter();
        let characters = self.driver.get().characters();
        let shown =
            |i: usize| family.is_none_or(|f| characters.get(i).is_some_and(|c| c.family == f));
        if !shown(self.character.get())
            && let Some(first) = (0..characters.len()).find(|&i| shown(i))
        {
            self.select_character(first);
        }
        self.characters_plate.snap();
    }

    /// Surprise: the model picks a character and dials and deals; the
    /// page shows what it picked.
    fn surprise_me(&self) {
        self.use_units();
        let picked = self.hands.borrow_mut().surprise(self.driver.get());
        if let Some((character, macros)) = picked {
            self.show_recipe(character, macros);
            self.show_hand(true, true);
        }
    }

    /// Move the character tiles and the dials to a recipe the model chose
    /// (a surprise, a code). A character the family row hides shows again.
    fn show_recipe(&self, character: usize, macros: Macros) {
        let family = self
            .driver
            .get()
            .characters()
            .get(character)
            .map(|c| c.family);
        if self.family.get().is_some_and(|f| Some(f) != family) {
            self.set_family(None);
        }
        self.select_character(character);
        self.macros.set(macros);
        let values = [macros.bright, macros.movement, macros.grit, macros.space];
        for (dial, v) in self.dials.iter().zip(values) {
            let v = (v * 100.0).round() as u16;
            dial.show(v, v);
        }
    }

    /// Deal the hand the typed code names, or say in place what is wrong
    /// with it.
    fn deal_code(&self) {
        let dealt = self
            .hands
            .borrow_mut()
            .deal_code(self.driver.get(), &self.code_entry.text());
        match dealt {
            Ok((character, macros)) => {
                self.code_error.set_visible(false);
                self.code_entry.remove_css_class("error");
                self.code_entry.set_text("");
                self.enter.popdown();
                self.show_recipe(character, macros);
                self.show_hand(true, true);
            }
            Err(why) => {
                self.code_error.set_label(&why);
                self.code_error.set_visible(true);
                self.code_entry.add_css_class("error");
                self.code_entry.grab_focus();
            }
        }
    }

    /// The selected sound's recipe code to the clipboard.
    fn copy_code(&self) {
        if let Some(code) = self.hands.borrow().code() {
            self.root.clipboard().set_text(&code);
            self.on_ask.emit(Ask::Copied);
        }
    }

    /// Morph the selected sound into the `row`-th of the others.
    fn morph_selected(&self, row: i32) {
        let other = usize::try_from(row)
            .ok()
            .and_then(|r| self.hands.borrow().others().get(r).copied());
        let Some(other) = other else { return };
        let dealt = self.hands.borrow_mut().morph(self.driver.get(), other);
        if dealt {
            self.show_hand(true, true);
        }
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
        // Only the chosen family's tiles show.
        let me = self.me.clone();
        self.characters.set_filter_func(move |child| {
            let Some(me) = me.upgrade() else { return true };
            let Some(family) = me.family.get() else {
                return true;
            };
            usize::try_from(child.index())
                .ok()
                .and_then(|i| me.driver.get().characters().get(i))
                .is_none_or(|c| c.family == family)
        });
        let deal = self.weak(|me, ()| me.deal());
        self.generate.connect_clicked(move |_| deal(()));
        let surprise = self.weak(|me, ()| me.surprise_me());
        self.surprise.connect_clicked(move |_| surprise(()));
        let typed = self.weak(|me, ()| me.deal_code());
        self.code_entry.connect_activate(move |_| typed(()));
        // Typing again takes the complaint away.
        let again = self.weak(|me, ()| {
            me.code_error.set_visible(false);
            me.code_entry.remove_css_class("error");
        });
        self.code_entry.connect_changed(move |_| again(()));
        let morph = self.weak(|me, row: i32| me.morph_selected(row));
        let morph_button = self.morph.clone();
        self.morph_list.connect_row_activated(move |_, row| {
            morph_button.popdown();
            morph(row.index());
        });
        let picked = self.weak(|me, ()| me.on_sound_selected());
        self.sounds
            .connect_selected_children_changed(move |_| picked(()));
        let picked = self.weak(|me, ()| me.on_oscillator_selected());
        self.osc_tiles
            .connect_selected_children_changed(move |_| picked(()));
        let keep = self.weak(|me, ()| me.keep_oscillator());
        self.osc_keep.connect_clicked(move |_| keep(()));
        let new = self.weak(|me, ()| me.deal_with_new_oscillator());
        self.osc_deal_with.connect_clicked(move |_| new(()));
        let breed = self.weak(|me, row: i32| me.breed_oscillator(row));
        let breed_button = self.osc_breed.clone();
        self.osc_breed_list.connect_row_activated(move |_, row| {
            breed_button.popdown();
            breed(row.index());
        });
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
        *self.oscillators.borrow_mut() = Oscillators::new(seed);
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
        if !on {
            self.oscillator_failed();
        }
        self.offline.set_visible(!on);
        self.update_lamps();
        self.osc_deal_with.set_sensitive(on);
        self.osc_deal_with.set_tooltip_text(Some(&if on {
            tr("Make an oscillator, load it into the synth, and deal sounds that use it")
        } else {
            tr("Connect the synth to load a new oscillator")
        }));
        self.osc_keep.set_tooltip_text(Some(&if on {
            tr("Put it on the shelf and load it into the synth's first free oscillator slot")
        } else {
            tr("Put it on the shelf, to load once the synth is connected")
        }));
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
        self.use_units();
        let dealt = self
            .hands
            .borrow_mut()
            .deal(d, c, self.macros.get(), &keep, base.as_deref());
        if dealt {
            self.show_hand(true, true);
        }
    }

    /// Six sounds that all use `unit` (a new oscillator just loaded): of
    /// the chosen character when it can take the unit, else of one that
    /// can, which the page then shows chosen.
    fn deal_with(&self, unit: oscyl_driver::PaletteUnit) {
        let macros = self.macros.get();
        let dealt = self.hands.borrow_mut().deal_with_unit(
            self.driver.get(),
            self.character.get(),
            macros,
            unit,
        );
        match dealt {
            Ok(c) => {
                self.show_recipe(c, macros);
                self.show_hand(true, true);
            }
            Err(why) => self.say(&why),
        }
    }

    /// New Oscillators: six of every family in turn.
    pub fn deal_oscillators(&self) {
        self.oscillators.borrow_mut().deal(None);
        self.show_oscillators(true);
    }

    fn vary_oscillator(&self) {
        let amount = oscyl_app::generator::AMOUNTS[(self.amount.active() as usize).min(2)];
        if self.oscillators.borrow_mut().vary(amount) {
            self.show_oscillators(true);
        }
    }

    /// Breed the selected oscillator with the `row`-th of the others.
    fn breed_oscillator(&self, row: i32) {
        let selected = self.oscillators.borrow().selected_index();
        let Ok(row) = usize::try_from(row) else {
            return;
        };
        let other = if row >= selected { row + 1 } else { row };
        if self.oscillators.borrow_mut().breed(other) {
            self.show_oscillators(true);
        }
    }

    /// Keep the selected oscillator: the app shelves it and loads it.
    pub fn keep_oscillator(&self) {
        let p = self.oscillators.borrow().selected().cloned();
        if let Some(p) = p {
            self.on_ask.emit(Ask::KeepOscillator(self.driver.get(), p));
        }
    }

    /// Deal with a New Oscillator: make one, keep it, and deal sounds that
    /// use it once the synth lists it.
    fn deal_with_new_oscillator(&self) {
        let seed = u64::from(glib::random_int()) << 32 | u64::from(glib::random_int());
        let family = osc::Family::ALL[(seed % osc::Family::ALL.len() as u64) as usize];
        let p = osc::generate(family, seed);
        self.set_pending(Some(p.name.clone()));
        self.on_ask.emit(Ask::KeepOscillator(self.driver.get(), p));
    }

    /// Rebuild the oscillator tiles; `dealt` plays the deal-in.
    fn show_oscillators(&self, dealt: bool) {
        let o = self.oscillators.borrow();
        self.rebuilding.set(true);
        self.osc_tiles.remove_all();
        for (i, p) in o.hand().iter().enumerate() {
            let child = oscillator_tile(p);
            if dealt {
                child.add_css_class("deal");
                child.add_css_class(&format!("deal-{i}"));
            }
            self.osc_tiles.append(&child);
        }
        if let Some(child) = self.osc_tiles.child_at_index(o.selected_index() as i32) {
            self.osc_tiles.select_child(&child);
        }
        self.osc_plate.snap();
        self.rebuilding.set(false);
        let any = !o.hand().is_empty();
        self.osc_empty.set_visible(!any);
        self.osc_actions.set_visible(any);
        drop(o);
        self.on_oscillator_selected();
        if dealt {
            reveal(self.osc_section.clone(), self.osc_actions.clone());
        }
    }

    /// An oscillator was picked: the others are offered to breed with.
    fn on_oscillator_selected(&self) {
        if self.rebuilding.get() {
            return;
        }
        if let Some(i) = index_of(&self.osc_tiles) {
            self.oscillators.borrow_mut().select(i);
        }
        let o = self.oscillators.borrow();
        self.osc_breed_list.remove_all();
        for (j, p) in o.hand().iter().enumerate() {
            if j != o.selected_index() {
                let row = gtk::Label::builder().label(&p.name).xalign(0.0).build();
                self.osc_breed_list.append(&row);
            }
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
        let d = self.driver.get();
        let signature = d
            .characters()
            .get(h.character)
            .and_then(|c| c.signature_page)
            .unwrap_or(0);
        let page = oscyl_app::editor::telling_page(d, &h.programs, signature, 128);
        for (i, p) in h.programs.iter().enumerate() {
            let kept = h
                .kept
                .as_ref()
                .filter(|(k, _)| *k == i)
                .map(|(_, s)| s.as_str());
            let units = h.units.get(i).map_or(&[][..], Vec::as_slice);
            let (child, lamp) = sound_tile(d, p, page, kept, units);
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
        reveal(self.sounds_head.clone(), self.actions.clone());
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
            // The others of the hand, to breed with or morph to.
            for list in [&self.breed_list, &self.morph_list] {
                list.remove_all();
                for (j, p) in h.programs.iter().enumerate() {
                    if j != i {
                        let name = self.driver.get().name(p).unwrap_or_default();
                        let row = gtk::Label::builder().label(&name).xalign(0.0).build();
                        list.append(&row);
                    }
                }
            }
            let code = h.codes.get(i).cloned().flatten();
            self.code.set_label(code.as_deref().unwrap_or_default());
            self.code_row.set_visible(code.is_some());
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

/// Scroll so `head` to `bottom` are in view, or their top when they are
/// taller than the view; once they are laid out, frame by frame.
fn reveal(head: gtk::Widget, actions: gtk::Widget) {
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
        let (Some(top), Some(bottom)) = (head.compute_bounds(&page), actions.compute_bounds(&page))
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

#[cfg(test)]
mod tests {
    use super::*;
    use oscyl_app::generator::{HAND, HISTORY};
    use oscyl_core::generate::hand;

    const XD: &dyn Driver = &oscyl_korg_minilogue_xd::DRIVER;

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
        use oscyl_korg_minilogue_xd::Program as XdProgram;
        use oscyl_korg_minilogue_xd::generate as old;
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
                        units: vec![],
                    })
                });
                let now = hand(XD, c, seed, HAND, |s| {
                    XD.generate(
                        &Recipe {
                            character: c,
                            macros,
                            seed: s,
                            keep: vec![],
                            palette: Default::default(),
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

            // A dealt sound shows its code; Copy puts it on the clipboard.
            let code = g.hands.borrow().code().expect("a dealt sound has a code");
            assert!(g.code_row.is_visible());
            assert_eq!(g.code.label(), code);
            g.copy_code();
            assert!(matches!(asks.borrow().last(), Some(Ask::Copied)));

            // Morph: six steps, a legend naming both ends, no code.
            g.morph_selected(1);
            let h = g.hands.borrow().current().unwrap().clone();
            assert_eq!(h.programs.len(), HAND);
            assert!(h.origin.starts_with("From"), "{}", h.origin);
            assert!(!g.code_row.is_visible());

            // A family shows only its characters and takes the choice along.
            g.set_family(Some(Family::Rhythm));
            assert_eq!(XD.characters()[g.character.get()].family, Family::Rhythm);

            // A typed code deals its sound first and moves the page to its
            // recipe, out of the family that hid it.
            g.code_entry.set_text(&code);
            g.deal_code();
            let r = oscyl_app::generator::parse_code(XD, &code).unwrap();
            assert_eq!(g.character.get(), r.character);
            assert_eq!(g.family.get(), None);
            assert_eq!(g.hands.borrow().code(), Some(code));
            // A bad one says why, in place, and deals nothing.
            let at = g.hands.borrow().position();
            g.code_entry.set_text("pad-nonsense");
            g.deal_code();
            assert!(g.code_error.get_visible() && !g.code_error.label().is_empty());
            assert_eq!(g.hands.borrow().position(), at);

            // Surprise deals and shows what it picked.
            g.surprise_me();
            let h = g.hands.borrow().current().unwrap().clone();
            assert_eq!(h.character, g.character.get());
            let m = g.macros.get();
            assert_eq!(g.dials[0].value(), (m.bright * 100.0).round() as u16);

            // The synth's units: a chip each, on; a sound that uses one
            // says so on its tile; an excluded unit is not used.
            let waves = oscyl_logue::unit::UnitInfo {
                module: oscyl_midi::logue::Module::Osc,
                platform: 2,
                name: "waves".into(),
                dev_id: 0,
                prg_id: 0,
                version: 0x010000,
                params: Some(vec![]),
            };
            let listing = |unit: Option<oscyl_logue::unit::UnitInfo>| {
                vec![oscyl_app::synth::SlotInfo {
                    module: oscyl_midi::logue::Module::Osc,
                    slot: 2,
                    name: unit.as_ref().map(|u| u.name.clone()),
                    info: unit,
                }]
            };
            assert!(!g.units.is_visible());
            oscyl_app::palette::shared().listed(XD, &listing(Some(waves)));
            g.refresh_units();
            assert!(g.units.is_visible() && g.units_legend.is_visible());
            let chip = g
                .units
                .first_child()
                .and_downcast::<gtk::ToggleButton>()
                .unwrap();
            assert!(chip.is_active());
            g.set_family(None);
            g.select_character(XD.characters().iter().position(|c| c.id == "pad").unwrap());
            g.deal();
            let h = g.hands.borrow().current().unwrap().clone();
            assert!(h.units.iter().any(|u| u == &["waves"]), "{:?}", h.units);
            chip.set_active(false);
            g.deal();
            let h = g.hands.borrow().current().unwrap().clone();
            assert!(h.units.iter().all(Vec::is_empty));
            // The slot cleared: the row goes.
            oscyl_app::palette::shared().listed(XD, &listing(None));
            g.refresh_units();
            assert!(!g.units.is_visible());
        });
    }

    /// New oscillators: dealt, varied, bred and kept; Deal with a New
    /// Oscillator deals sounds that use it once the synth lists it. Needs
    /// a display; skipped without.
    #[test]
    fn new_oscillators_deal_keep_and_feed_the_sounds() {
        crate::on_gtk_thread(|| {
            let g = Generator::new(XD);
            let asks = Rc::new(RefCell::new(Vec::new()));
            let a = asks.clone();
            g.connect_ask(move |ask| a.borrow_mut().push(ask));
            g.set_seed(7);
            assert!(
                g.osc_section.is_visible(),
                "the xd runs the oscillator unit"
            );
            assert!(g.osc_empty.is_visible() && !g.osc_actions.is_visible());
            g.deal_oscillators();
            assert_eq!(g.osc_tiles.observe_children().n_items() as usize, HAND);
            assert!(g.osc_actions.is_visible());
            let first = g.oscillators.borrow().hand().to_vec();
            g.osc_tiles
                .select_child(&g.osc_tiles.child_at_index(2).unwrap());
            assert_eq!(g.oscillators.borrow().selected_index(), 2);
            assert_eq!(
                g.osc_breed_list.observe_children().n_items() as usize,
                HAND - 1
            );
            g.vary_oscillator();
            assert!(
                g.oscillators
                    .borrow()
                    .hand()
                    .iter()
                    .all(|p| !first.contains(p))
            );
            g.breed_oscillator(0);
            assert_eq!(g.oscillators.borrow().hand().len(), HAND);
            g.keep_oscillator();
            let kept = match asks.borrow().last() {
                Some(Ask::KeepOscillator(_, p)) => p.clone(),
                _ => panic!("Keep asks the app to keep it"),
            };
            assert_eq!(Some(&kept), g.oscillators.borrow().selected());

            // Offline, Deal with a New Oscillator waits for a synth.
            g.set_connected(false);
            assert!(!g.osc_deal_with.is_sensitive());
            g.set_connected(true);
            g.deal_with_new_oscillator();
            assert!(matches!(
                asks.borrow().last(),
                Some(Ask::KeepOscillator(..))
            ));
            assert!(g.osc_status.is_visible());
            // The synth could not take it (slots full, a failed upload):
            // nothing waits on it any more.
            g.oscillator_failed();
            assert!(!g.osc_status.is_visible() && g.pending.borrow().is_none());
            g.deal_with_new_oscillator();
            let new = match asks.borrow().last() {
                Some(Ask::KeepOscillator(_, p)) => p.clone(),
                _ => panic!("it is kept"),
            };
            // Listed, its parameters known: six sounds that use it are
            // dealt, of a character that takes it when the chosen one does
            // not (the page moves to it).
            let info = oscyl_logue::osc::build(&oscyl_midi::logue::platform(XD).unwrap(), &new)
                .unwrap()
                .info();
            let unit = oscyl_driver::PaletteUnit {
                kind: *XD
                    .capabilities()
                    .units
                    .iter()
                    .find(|k| k.name == "osc")
                    .unwrap(),
                slot: 5,
                name: new.name.clone(),
                params: Some(vec![]),
            };
            let takes_none = (0..XD.characters().len())
                .find(|&c| {
                    Hands::new(1).deal_with_unit(XD, c, Macros::default(), unit.clone()) != Ok(c)
                })
                .expect("a character that takes no unit");
            g.set_family(None);
            g.select_character(takes_none);
            oscyl_app::palette::shared().listed(
                XD,
                &[oscyl_app::synth::SlotInfo {
                    module: oscyl_midi::logue::Module::Osc,
                    slot: 5,
                    name: Some(new.name.clone()),
                    info: Some(info),
                }],
            );
            g.refresh_units();
            assert!(!g.osc_status.is_visible());
            let h = g.hands.borrow().current().unwrap().clone();
            assert!(
                h.units.iter().all(|u| u == std::slice::from_ref(&new.name)),
                "{:?}",
                h.units
            );
            assert_ne!(h.character, takes_none);
            assert_eq!(g.character.get(), h.character, "the page shows it");
            oscyl_app::palette::shared().listed(XD, &[]);
        });
    }
}
