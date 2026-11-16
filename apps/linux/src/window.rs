//! The main window: sources sidebar, program list and editor in libadwaita's
//! three-pane pattern, with breakpoints.

use std::rc::Rc;

use adw::prelude::*;
use gtk::gio;

use crate::i18n::tr;
use crate::settings::Settings;

/// Collapse the program list into navigation below this width.
const BP_LIST: &str = "max-width: 1100sp";
/// Below this width the editor header is too tight for the Live and
/// Compare words: their glyphs remain.
const BP_TIGHT: &str = "max-width: 960sp";
/// Turn the sources sidebar into an overlay below this width.
const BP_SIDEBAR: &str = "max-width: 720sp";

/// The header's primary action (Sync or Save): the count of edited
/// programs behind the red edit tick, the label and, when clean, a check;
/// and the audition toggle beside it.
pub struct Primary {
    pub button: gtk::Button,
    pub icon: gtk::Image,
    pub label: gtk::Label,
    pub count: gtk::Label,
    pub audition: gtk::ToggleButton,
    /// Hold to hear and see the saved program; drag sideways to blend.
    pub compare: gtk::Button,
    /// The blend's line along the Compare button's foot, while held.
    pub blend: gtk::ProgressBar,
    pub undo: gtk::Button,
    pub redo: gtk::Button,
    /// The History popover's content, filled as it opens.
    pub history: gtk::MenuButton,
    pub history_body: gtk::Box,
    /// The audition lamp's activity blink.
    blink: adw::TimedAnimation,
}

impl Primary {
    /// Blink the audition lamp: something just went to the synth. While
    /// changes stream, the lamp flickers at its own pace, like a MIDI
    /// activity light, rather than restarting on every message.
    pub fn blink(&self) {
        if self.audition.is_active() && self.blink.state() != adw::AnimationState::Playing {
            self.blink.play();
        }
    }
}

pub struct OscylWindow {
    pub window: adw::ApplicationWindow,
    pub outer: adw::OverlaySplitView,
    pub inner: adw::NavigationSplitView,
    pub sidebar_page: adw::ToolbarView,
    pub list_page: adw::NavigationPage,
    /// The list pane's title; its subtitle counts the programs.
    pub list_title: adw::WindowTitle,
    /// Applies below `BP_SIDEBAR` (phones).
    pub narrow: adw::Breakpoint,
    /// Whether `narrow` applies now.
    pub is_narrow: Rc<std::cell::Cell<bool>>,
    /// Between the list's and the sidebar's collapse: a tight header.
    tight: adw::Breakpoint,
    pub editor_page: adw::NavigationPage,
    pub toasts: adw::ToastOverlay,
    pub settings: std::cell::RefCell<Settings>,
    /// The interface-size toast while it shows.
    zoom_toast: Rc<std::cell::RefCell<Option<adw::Toast>>>,
    /// The toast showing (or about to), dismissed by the next one.
    shown_toast: std::cell::RefCell<Option<adw::Toast>>,
}

impl std::ops::Deref for OscylWindow {
    type Target = adw::ApplicationWindow;
    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl OscylWindow {
    pub fn new(app: &adw::Application, settings: Settings) -> Rc<OscylWindow> {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Oscyl")
            .default_width(1440)
            .default_height(900)
            .width_request(360)
            .height_request(480)
            .build();

        let sidebar_page = adw::ToolbarView::new();
        let list_page = adw::NavigationPage::builder()
            .title(tr("Programs"))
            .tag("programs")
            .build();
        let editor_page = adw::NavigationPage::builder()
            .title(tr("Editor"))
            .tag("editor")
            .build();

        let inner = adw::NavigationSplitView::builder()
            .sidebar(&list_page)
            .content(&editor_page)
            .min_sidebar_width(280.0)
            .max_sidebar_width(320.0)
            .sidebar_width_fraction(0.24)
            .build();
        let outer = adw::OverlaySplitView::builder()
            .sidebar(&sidebar_page)
            .content(&inner)
            .min_sidebar_width(220.0)
            .max_sidebar_width(260.0)
            .build();
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&outer));
        window.set_content(Some(&toasts));

        // Only one window breakpoint applies at a time (the last matching),
        // so the narrow one repeats the wider one's setters.
        let collapse_list = adw::Breakpoint::new(
            adw::BreakpointCondition::parse(BP_LIST).expect("valid breakpoint"),
        );
        collapse_list.add_setter(&inner, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(collapse_list);
        let tight = adw::Breakpoint::new(
            adw::BreakpointCondition::parse(BP_TIGHT).expect("valid breakpoint"),
        );
        tight.add_setter(&inner, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(tight.clone());
        let collapse_all = adw::Breakpoint::new(
            adw::BreakpointCondition::parse(BP_SIDEBAR).expect("valid breakpoint"),
        );
        collapse_all.add_setter(&inner, "collapsed", Some(&true.to_value()));
        collapse_all.add_setter(&outer, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(collapse_all.clone());
        let list_title = adw::WindowTitle::new("", "");
        list_page
            .bind_property("title", &list_title, "title")
            .sync_create()
            .build();

        let is_narrow: Rc<std::cell::Cell<bool>> = Rc::default();
        {
            let n = is_narrow.clone();
            collapse_all.connect_apply(move |_| n.set(true));
            let n = is_narrow.clone();
            collapse_all.connect_unapply(move |_| n.set(false));
        }
        Rc::new(OscylWindow {
            window,
            outer,
            inner,
            sidebar_page,
            list_page,
            list_title,
            narrow: collapse_all,
            is_narrow,
            tight,
            editor_page,
            toasts,
            zoom_toast: Rc::default(),
            shown_toast: Default::default(),
            settings: std::cell::RefCell::new(settings),
        })
    }

    /// Fill the three panes: header bars, the sources sidebar, the program
    /// list and the editor (with `editor_title` in its bar). Returns the
    /// primary (Sync/Save) action's parts.
    pub fn set_panes(
        &self,
        sidebar: &impl IsA<gtk::Widget>,
        list: &impl IsA<gtk::Widget>,
        editor: &impl IsA<gtk::Widget>,
        editor_title: &impl IsA<gtk::Widget>,
    ) -> Primary {
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&adw::WindowTitle::new("Oscyl", "")));
        let menu = gio::Menu::new();
        let section = gio::Menu::new();
        section.append(Some(&tr("Open Library…")), Some("win.open"));
        section.append(Some(&tr("Explore Demo Library")), Some("win.open-demo"));
        section.append(Some(&tr("Back Up Synth…")), Some("win.backup"));
        section.append(Some(&tr("Save Playing Sound")), Some("win.save-playing"));
        menu.append_section(None, &section);
        let section = gio::Menu::new();
        section.append(Some(&tr("Preferences")), Some("win.preferences"));
        section.append(Some(&tr("Keyboard Shortcuts")), Some("win.shortcuts"));
        section.append(Some(&tr("Guide")), Some("win.guide"));
        section.append(Some(&tr("About Oscyl")), Some("win.about"));
        menu.append_section(None, &section);
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu)
            .primary(true)
            .tooltip_text(tr("Main menu"))
            .build();
        header.pack_end(&menu_button);
        self.sidebar_page.add_top_bar(&header);
        self.sidebar_page.set_content(Some(sidebar));

        let list_view = adw::ToolbarView::new();
        let list_header = adw::HeaderBar::new();
        let toggle = gtk::ToggleButton::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text(tr("Show sources (F9)"))
            .build();
        self.outer
            .bind_property("show-sidebar", &toggle, "active")
            .bidirectional()
            .sync_create()
            .build();
        self.outer
            .bind_property("collapsed", &toggle, "visible")
            .sync_create()
            .build();
        list_header.pack_start(&toggle);
        list_header.set_title_widget(Some(&self.list_title));
        // New Program, in one of your libraries: shown only where the
        // action applies (the synth's 500 slots are fixed).
        let add = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .action_name("win.new-program")
            .tooltip_text(tr("New Program (Ctrl+N)"))
            .build();
        add.update_property(&[
            gtk::accessible::Property::Label(&tr("New Program")),
            gtk::accessible::Property::KeyShortcuts("Control+N"),
        ]);
        add.connect_sensitive_notify(|b| b.set_visible(b.is_sensitive()));
        add.set_visible(add.is_sensitive());
        list_header.pack_end(&add);
        list_view.add_top_bar(&list_header);
        list_view.set_content(Some(list));
        self.list_page.set_child(Some(&list_view));

        let editor_view = adw::ToolbarView::new();
        let editor_header = adw::HeaderBar::new();
        // The program name is the page's big title; the bar carries the slot.
        editor_header.set_title_widget(Some(editor_title));
        let undo = gtk::Button::builder()
            .icon_name("edit-undo-symbolic")
            .action_name("win.undo")
            .tooltip_text(tr("Undo (Ctrl+Z)"))
            .build();
        let redo = gtk::Button::builder()
            .icon_name("edit-redo-symbolic")
            .action_name("win.redo")
            .tooltip_text(tr("Redo (Ctrl+Shift+Z)"))
            .build();
        editor_header.pack_start(&undo);
        editor_header.pack_start(&redo);
        // The history: snapshots and every step, newest first (Ctrl+H).
        let history_body = gtk::Box::new(gtk::Orientation::Vertical, 8);
        history_body.add_css_class("history-body");
        let history = gtk::MenuButton::builder()
            .icon_name("document-open-recent-symbolic")
            .popover(&gtk::Popover::builder().child(&history_body).build())
            .tooltip_text(tr("History (Ctrl+H)"))
            .build();
        history.update_property(&[
            gtk::accessible::Property::Label(&tr("History")),
            gtk::accessible::Property::KeyShortcuts("Control+H"),
        ]);
        editor_header.pack_start(&history);
        // Phones: undo and redo move into a menu to leave the bar room.
        let edit_menu = gio::Menu::new();
        edit_menu.append(Some(&tr("Undo")), Some("win.undo"));
        edit_menu.append(Some(&tr("Redo")), Some("win.redo"));
        let more = gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .menu_model(&edit_menu)
            .tooltip_text(tr("Edit"))
            .visible(false)
            .build();
        more.update_property(&[gtk::accessible::Property::Label(&tr("Edit"))]);
        editor_header.pack_end(&more);
        for (w, narrow) in [
            (undo.upcast_ref::<gtk::Widget>(), false),
            (redo.upcast_ref(), false),
            (history.upcast_ref(), false),
            (more.upcast_ref(), true),
        ] {
            self.narrow
                .add_setter(w, "visible", Some(&narrow.to_value()));
        }
        let button = gtk::Button::builder().action_name("win.primary").build();
        // The memory card, the synth's own storage: to save, to sync, or
        // with a check once nothing is left to write.
        let icon = gtk::Image::from_icon_name("oscyl-saved-symbolic");
        let label = gtk::Label::new(Some(&tr("Saved")));
        // The card, the verb, then the count in the action colour.
        let count = gtk::Label::builder()
            .css_classes(["edit-count", "numeric"])
            .visible(false)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        content.append(&icon);
        content.append(&label);
        content.append(&count);
        button.set_child(Some(&content));
        button.set_widget_name("primary-action");
        editor_header.pack_end(&button);
        // Outlined: headphones, the word, and the function lamp, a ring
        // while off and lit green while auditioning; it blinks as changes
        // go to the synth.
        let lamp = gtk::Box::builder()
            .css_classes(["lamp"])
            .valign(gtk::Align::Center)
            .build();
        let audition_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        audition_content.append(&gtk::Image::from_icon_name("audio-headphones-symbolic"));
        let live_label = gtk::Label::new(Some(&tr("Live")));
        audition_content.append(&live_label);
        audition_content.append(&lamp);
        let audition = gtk::ToggleButton::builder()
            .child(&audition_content)
            .css_classes(["outlined", "audition"])
            .action_name("win.audition")
            .tooltip_text(tr(
                "Audition live: play every change on the synth without storing it (Ctrl+Shift+A)",
            ))
            .build();
        audition.update_property(&[
            gtk::accessible::Property::Label(&tr("Audition live")),
            gtk::accessible::Property::KeyShortcuts("Control+Shift+A"),
        ]);
        let blink = adw::TimedAnimation::builder()
            .widget(&lamp)
            .target(&adw::PropertyAnimationTarget::new(&lamp, "opacity"))
            .value_from(0.25)
            .value_to(1.0)
            .duration(240)
            .easing(adw::Easing::EaseOutCubic)
            .build();
        // Hold to compare: while pressed, the editor and the synth show
        // the saved program (backslash does the same).
        let compare_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        compare_content.append(&gtk::Image::from_icon_name("oscyl-compare-symbolic"));
        let compare_label = gtk::Label::new(Some(&tr("Hold to compare")));
        compare_content.append(&compare_label);
        // In a tight header or on a phone the two keep their glyphs and
        // lose their words.
        for l in [&live_label, &compare_label] {
            self.tight.add_setter(l, "visible", Some(&false.to_value()));
            self.narrow
                .add_setter(l, "visible", Some(&false.to_value()));
        }
        // While held, a line along the foot says how far toward the edit
        // the blend stands (empty: the saved program).
        let blend = gtk::ProgressBar::builder()
            .css_classes(["blend"])
            .valign(gtk::Align::End)
            .can_target(false)
            .visible(false)
            .build();
        blend.update_property(&[gtk::accessible::Property::Label(&tr(
            "Blend toward the edit",
        ))]);
        let compare_over = gtk::Overlay::builder().child(&compare_content).build();
        compare_over.add_overlay(&blend);
        let compare = gtk::Button::builder()
            .child(&compare_over)
            .css_classes(["outlined", "compare"])
            .tooltip_text(tr(
                "Hold to hear and see the saved program; drag sideways to blend it with your edit (or hold \\, with Left and Right)",
            ))
            .build();
        compare.update_property(&[
            gtk::accessible::Property::Label(&tr("Compare with the saved program")),
            gtk::accessible::Property::KeyShortcuts("backslash"),
        ]);
        editor_header.pack_end(&compare);
        editor_header.pack_end(&audition);
        editor_view.add_top_bar(&editor_header);
        editor_view.set_content(Some(editor));
        self.editor_page.set_child(Some(&editor_view));
        Primary {
            button,
            icon,
            label,
            count,
            audition,
            compare,
            blend,
            undo,
            redo,
            history,
            history_body,
            blink,
        }
    }

    pub fn toast(&self, text: &str) {
        self.show_toast(adw::Toast::new(text));
    }

    /// Show `toast` now. Toasts never queue: a new one dismisses the one
    /// showing, so actions in a row (rolls, say) leave one toast, the last.
    pub fn show_toast(&self, toast: adw::Toast) {
        if let Some(old) = self.shown_toast.replace(Some(toast.clone()))
            && old != toast
        {
            old.dismiss();
        }
        self.toasts.add_toast(toast);
    }

    /// Say the interface size after a change, in one toast that updates
    /// while it shows (no queue of them), with Reset when not at 100 %.
    pub fn zoom_toast(&self, zoom: f64) {
        let title =
            tr("Interface size {percent} %").replace("{percent}", &format!("{:.0}", zoom * 100.0));
        let reset = (zoom - 1.0).abs() > 1e-6;
        if let Some(t) = self.zoom_toast.borrow().as_ref() {
            t.set_title(&title);
            t.set_button_label(reset.then(|| tr("Reset")).as_deref());
            return;
        }
        let t = adw::Toast::builder().title(&title).build();
        if reset {
            t.set_button_label(Some(&tr("Reset")));
        }
        t.set_action_name(Some("win.zoom-reset"));
        let slot = self.zoom_toast.clone();
        t.connect_dismissed(move |_| {
            slot.replace(None);
        });
        self.zoom_toast.replace(Some(t.clone()));
        self.show_toast(t);
    }
}
