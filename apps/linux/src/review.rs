//! The review bar at the editor's foot: how many changes the shown program
//! has, per stage, and Revert All. It slides in with the first edit and
//! hinges up into a sheet listing every change by stage, each with its
//! revert, over the page (the page never reflows to make room).
//!
//! Motion: the bar and sheet never change size while they move. The card
//! is laid out once at full height and slid by a transform, driven by
//! critically damped springs, so a move costs no relayout, can be
//! interrupted at any point, and reverses with the velocity it had.

use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, graphene, gsk};
use oscyl_driver::{Curation, Driver};

use crate::callback::Slot;
use crate::editor::change_text;
use crate::i18n::{tr, trn};
use oscyl_app::review::changes;

pub struct Review {
    /// The whole bar and sheet; shown while there are changes.
    pub root: Drawer,
    toggle: gtk::ToggleButton,
    count: gtk::Label,
    /// The per-stage chips; a phone hides them.
    pub chips: gtk::ScrolledWindow,
    chip_row: gtk::Box,
    groups: gtk::FlowBox,
    /// One row per changed field, in schema order.
    rows: RefCell<Vec<(usize, adw::ActionRow)>>,
    /// A page index.
    on_stage: Slot<usize>,
    on_revert: Slot<Option<usize>>,
    me: Weak<Review>,
}

impl Review {
    pub fn new() -> Rc<Review> {
        let count = gtk::Label::builder().xalign(0.0).build();
        count.add_css_class("review-count");
        // The per-stage chips scroll sideways when they outrun the bar
        // (wheel, touch or keys), with faded edges saying there is more;
        // the bar's actions always keep their place.
        let chip_row = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        let chips = gtk::ScrolledWindow::builder()
            .child(&chip_row)
            .hscrollbar_policy(gtk::PolicyType::External)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_width(true)
            .hexpand(true)
            .css_classes(["review-chips"])
            .build();
        crate::widgets::smooth_wheel_sideways(&chips);
        let revert_all = gtk::Button::builder()
            .label(tr("Revert All"))
            .action_name("win.revert-program")
            .valign(gtk::Align::Center)
            .build();
        let toggle = gtk::ToggleButton::builder()
            .icon_name("pan-up-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text(tr("Show every change"))
            .css_classes(["flat", "review-toggle"])
            .build();
        toggle.update_property(&[gtk::accessible::Property::Label(&tr("Show every change"))]);
        let dot = gtk::Box::builder()
            .css_classes(["edit-tick"])
            .valign(gtk::Align::Center)
            .build();
        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        bar.add_css_class("review-bar");
        bar.append(&dot);
        bar.append(&count);
        bar.append(&chips);
        bar.append(&revert_all);
        bar.append(&toggle);

        let groups = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .homogeneous(true)
            .min_children_per_line(1)
            .max_children_per_line(3)
            .column_spacing(28)
            .row_spacing(12)
            .valign(gtk::Align::Start)
            .build();
        groups.add_css_class("review-groups");
        let scroll = gtk::ScrolledWindow::builder()
            .child(&groups)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(320)
            .build();
        crate::widgets::smooth_wheel(&scroll);
        let sheet = scroll;
        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("review");
        card.append(&bar);
        card.append(&sheet);
        card.update_property(&[gtk::accessible::Property::Label(&tr(
            "Changes to this program",
        ))]);

        let root = Drawer::new(&card);
        {
            let root = root.downgrade();
            toggle.connect_toggled(move |t| {
                if let Some(r) = root.upgrade() {
                    r.set_open(t.is_active());
                }
            });
        }
        Rc::new_cyclic(|me| Review {
            root,
            toggle,
            count,
            chips,
            chip_row,
            groups,
            rows: RefCell::default(),
            on_stage: Slot::default(),
            on_revert: Slot::default(),
            me: me.clone(),
        })
    }

    /// Activating a change or a stage chip shows that stage.
    pub fn connect_stage(&self, f: impl Fn(usize) + 'static) {
        self.on_stage.connect(f);
    }

    /// A change's revert (`None`: the whole program).
    pub fn connect_revert(&self, f: impl Fn(Option<usize>) + 'static) {
        self.on_revert.connect(f);
    }

    /// Rebuild from the shown program and its saved version.
    pub fn set(&self, d: &dyn Driver, c: &Curation, p: &[u8], base: &[u8]) {
        while let Some(c) = self.groups.first_child() {
            self.groups.remove(&c);
        }
        while let Some(c) = self.chip_row.first_child() {
            self.chip_row.remove(&c);
        }
        let all = changes(d, c, p, base);
        let n = all.len();
        self.count.set_label(&trn(
            "%d change in this program",
            "%d changes in this program",
            n as u32,
        ));
        self.root.set_shown(n > 0);
        if n == 0 {
            self.toggle.set_active(false);
        }

        let mut lists: Vec<(Option<usize>, gtk::ListBox)> = Vec::new();
        let mut list_for = |stage: Option<usize>, title: &str| -> gtk::ListBox {
            if let Some((_, l)) = lists.iter().find(|(s, _)| *s == stage) {
                return l.clone();
            }
            let group = gtk::Box::new(gtk::Orientation::Vertical, 2);
            let legend = gtk::Label::builder().label(title).xalign(0.0).build();
            legend.add_css_class("legend");
            legend.add_css_class("review-legend");
            let list = gtk::ListBox::new();
            list.add_css_class("review-list");
            list.set_selection_mode(gtk::SelectionMode::None);
            list.update_property(&[gtk::accessible::Property::Label(title)]);
            group.append(&legend);
            group.append(&list);
            self.groups.append(&group);
            lists.push((stage, list.clone()));
            list
        };
        let mut rows = Vec::with_capacity(n);
        let mut per_stage: Vec<(usize, usize)> = Vec::new();
        for change in all {
            let Some(field) = change.field else {
                list_for(None, &tr("Program")).append(&self.row(
                    &tr("Name"),
                    &tr("renamed"),
                    None,
                    None,
                ));
                continue;
            };
            let group = change
                .stage
                .map_or_else(|| tr("Program"), |s| tr(c.pages[s].title));
            let text = change_text(&change.before, &change.after);
            let row = self.row(&change.title, &text, Some(field), change.stage);
            list_for(change.stage, &group).append(&row);
            rows.push((field, row));
            if let Some(s) = change.stage {
                match per_stage.iter_mut().find(|(t, _)| *t == s) {
                    Some((_, k)) => *k += 1,
                    None => per_stage.push((s, 1)),
                }
            }
        }
        *self.rows.borrow_mut() = rows;
        per_stage.sort_by_key(|(s, _)| *s);
        for (stage, k) in per_stage {
            let chip = gtk::Button::builder()
                .label(format!("{} {k}", tr(c.pages[stage].title)))
                .css_classes(["flat", "review-chip"])
                .tooltip_text(tr("Show in the editor"))
                .build();
            let me = self.me.clone();
            chip.connect_clicked(move |_| {
                if let Some(me) = me.upgrade() {
                    me.on_stage.emit(stage);
                }
            });
            self.chip_row.append(&chip);
        }
    }

    /// Keep in step with one field: update its row in place, rebuilding
    /// only when the field starts or stops being changed.
    pub fn update(&self, d: &dyn Driver, c: &Curation, field: usize, p: &[u8], base: &[u8]) {
        let edited = d.get(p, field) != d.get(base, field);
        let rows = self.rows.borrow();
        match rows.iter().find(|(f, _)| *f == field) {
            Some((_, row)) if edited => {
                let (before, after) = oscyl_app::editor::before_after(d, field, base, p);
                row.set_subtitle(&change_text(&before, &after));
            }
            None if !edited => {}
            _ => {
                drop(rows);
                self.set(d, c, p, base);
            }
        }
    }

    /// Fold the sheet back into the bar; false when it was not open.
    pub fn fold(&self) -> bool {
        let open = self.toggle.is_active();
        self.toggle.set_active(false);
        open
    }

    /// Open or fold the sheet of changes.
    pub fn open(&self, open: bool) {
        self.toggle.set_active(open);
    }

    /// Rows listed now (one per changed field).
    #[cfg(test)]
    pub fn rows(&self) -> usize {
        self.rows.borrow().len()
    }

    fn row(
        &self,
        title: &str,
        subtitle: &str,
        field: Option<usize>,
        stage: Option<usize>,
    ) -> adw::ActionRow {
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(subtitle)
            .build();
        row.add_prefix(
            &gtk::Box::builder()
                .css_classes(["edit-tick"])
                .valign(gtk::Align::Center)
                .build(),
        );
        row.upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Description(&tr("edited"))]);
        if let Some(stage) = stage {
            row.set_activatable(true);
            row.set_tooltip_text(Some(&tr("Show in the editor")));
            let me = self.me.clone();
            row.connect_activated(move |_| {
                if let Some(me) = me.upgrade() {
                    me.on_stage.emit(stage);
                }
            });
        }
        if let Some(field) = field {
            let b = gtk::Button::from_icon_name("edit-undo-symbolic");
            b.add_css_class("flat");
            b.set_valign(gtk::Align::Center);
            let revert = tr("Revert {name}").replace("{name}", title);
            b.set_tooltip_text(Some(&revert));
            b.update_property(&[gtk::accessible::Property::Label(&revert)]);
            let me = self.me.clone();
            b.connect_clicked(move |_| {
                if let Some(me) = me.upgrade() {
                    me.on_revert.emit(Some(field));
                }
            });
            row.add_suffix(&b);
        }
        row
    }
}

/// How a drawer move feels: critically damped (no bounce), settling in
/// about a quarter of a second.
fn spring() -> adw::SpringParams {
    adw::SpringParams::new(1.0, 1.0, 420.0)
}

mod drawer_imp {
    use super::*;

    #[derive(Default)]
    pub struct Drawer {
        pub(super) child: OnceCell<gtk::Widget>,
        /// How far the bar is in (0 hidden, 1 shown) and the sheet open.
        pub(super) shown: Cell<f64>,
        pub(super) open: Cell<f64>,
        pub(super) shown_anim: RefCell<Option<adw::SpringAnimation>>,
        pub(super) open_anim: RefCell<Option<adw::SpringAnimation>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Drawer {
        const NAME: &'static str = "OscylDrawer";
        type Type = super::Drawer;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("drawer");
        }
    }

    impl ObjectImpl for Drawer {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Drawer {
        fn measure(&self, o: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            match self.child.get() {
                Some(c) => {
                    let (min, nat, _, _) = c.measure(o, for_size);
                    (min, nat, -1, -1)
                }
                None => (0, 0, -1, -1),
            }
        }

        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            if let Some(c) = self.child.get() {
                let y = self.obj().offset(height);
                let t = gsk::Transform::new().translate(&graphene::Point::new(0.0, y));
                c.allocate(width, height, -1, Some(t));
            }
        }

        // Only the card's visible part takes input; the transparent room
        // above a folded sheet lets clicks through to the page.
        fn contains(&self, _x: f64, y: f64) -> bool {
            let obj = self.obj();
            self.shown.get() > 0.0 && y >= f64::from(obj.offset(obj.height()))
        }
    }
}

glib::wrapper! {
    /// A bar that slides up from the bottom and opens into a sheet, moved
    /// by springs and a transform (see the module docs).
    pub struct Drawer(ObjectSubclass<drawer_imp::Drawer>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Drawer {
    /// `card`'s first child is the bar, its last the sheet.
    fn new(card: &gtk::Box) -> Self {
        let this: Self = glib::Object::new();
        this.set_valign(gtk::Align::End);
        card.set_parent(&this);
        let _ = this.imp().child.set(card.clone().upcast());
        if let Some(sheet) = card.last_child() {
            sheet.set_visible(false);
        }
        this.set_visible(false);
        this
    }

    fn bar_height(&self, width: i32) -> f32 {
        self.imp()
            .child
            .get()
            .and_then(|c| c.first_child())
            .map_or(0.0, |b| {
                b.measure(gtk::Orientation::Vertical, width).1 as f32
            })
    }

    /// How far down the card is slid inside a drawer of `height`.
    fn offset(&self, height: i32) -> f32 {
        let imp = self.imp();
        let bar = self.bar_height(self.width());
        let sheet = (height as f32 - bar).max(0.0);
        // The bar slides fully out of view, its shadow included.
        let hidden = bar + 32.0;
        (1.0 - imp.open.get() as f32) * sheet + (1.0 - imp.shown.get() as f32) * hidden
    }

    /// Slide the bar in or out.
    pub fn set_shown(&self, shown: bool) {
        if shown {
            self.set_visible(true);
        }
        let weak = self.downgrade();
        self.spring_to(true, if shown { 1.0 } else { 0.0 }, move |_| {
            if let Some(d) = weak.upgrade()
                && d.imp().shown.get() == 0.0
            {
                d.set_visible(false);
            }
        });
    }

    /// Open the sheet over the page, or fold it into the bar.
    pub fn set_open(&self, open: bool) {
        let Some(sheet) = self.imp().child.get().and_then(|c| c.last_child()) else {
            return;
        };
        if open {
            sheet.set_visible(true);
        }
        let weak = self.downgrade();
        self.spring_to(false, if open { 1.0 } else { 0.0 }, move |_| {
            if let Some(d) = weak.upgrade()
                && d.imp().open.get() == 0.0
                && let Some(sheet) = d.imp().child.get().and_then(|c| c.last_child())
            {
                // A folded sheet leaves the focus chain and the screen
                // reader's view.
                sheet.set_visible(false);
            }
        });
    }

    /// Spring `shown` (or `open`) to `to`, from wherever it is now and
    /// with the velocity it has, then run `done`.
    fn spring_to(&self, shown: bool, to: f64, done: impl Fn(&adw::SpringAnimation) + 'static) {
        let imp = self.imp();
        let (value, slot) = if shown {
            (&imp.shown, &imp.shown_anim)
        } else {
            (&imp.open, &imp.open_anim)
        };
        let velocity = slot.borrow().as_ref().map_or(0.0, |a| {
            if a.state() == adw::AnimationState::Playing {
                a.velocity()
            } else {
                0.0
            }
        });
        if let Some(old) = slot.borrow_mut().take() {
            old.pause();
        }
        let weak = self.downgrade();
        let target = adw::CallbackAnimationTarget::new(move |v| {
            if let Some(d) = weak.upgrade() {
                let imp = d.imp();
                if shown {
                    imp.shown.set(v);
                } else {
                    imp.open.set(v);
                }
                d.queue_allocate();
            }
        });
        let anim = adw::SpringAnimation::builder()
            .widget(self)
            .value_from(value.get())
            .value_to(to)
            .spring_params(&spring())
            .initial_velocity(velocity)
            .clamp(true)
            .target(&target)
            .build();
        anim.connect_done(done);
        anim.play();
        slot.replace(Some(anim));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bar shows with the first change and the sheet folds out of
    /// the focus chain when closed. Off screen, springs finish at once.
    #[test]
    fn bar_follows_changes_and_sheet_folds_away() {
        crate::on_gtk_thread(|| {
            let d: &dyn Driver = &oscyl_minilogue_xd::DRIVER;
            let c = d.curation();
            let review = Review::new();
            let base = d.init_program();
            let mut p = base.clone();
            review.set(d, &c, &p, &base);
            assert!(!review.root.is_visible());
            let cutoff = d.fields().iter().position(|f| f.name == "Cutoff").unwrap();
            d.set(&mut p, cutoff, 700).unwrap();
            review.set(d, &c, &p, &base);
            assert!(review.root.is_visible());
            assert_eq!(review.root.imp().shown.get(), 1.0);
            let sheet = review
                .root
                .imp()
                .child
                .get()
                .and_then(|c| c.last_child())
                .unwrap();
            assert!(!sheet.is_visible());
            review.open(true);
            assert!(sheet.is_visible());
            assert_eq!(review.root.imp().open.get(), 1.0);
            assert!(review.fold());
            assert!(!sheet.is_visible());
            assert!(!review.fold(), "folding twice reports nothing to fold");
        });
    }
}
