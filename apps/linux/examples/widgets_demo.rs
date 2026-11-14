//! Scratch harness for the custom widgets: renders them to a PNG.
//!
//! `cargo run -p oscyl-gtk --example widgets_demo -- OUT.png [dark] [zoom] [narrow|mid]`

#[allow(dead_code)]
#[path = "../src/callback.rs"]
mod callback;
#[allow(dead_code)] // the widgets use only `tr`
#[path = "../src/i18n.rs"]
mod i18n;
#[allow(dead_code)]
#[path = "../src/shot.rs"]
mod shot;
#[allow(dead_code)]
#[path = "../src/widgets/mod.rs"]
mod widgets;

use adw::prelude::*;
use gtk::{gdk, gio, glib};
use oscyl_driver::Driver;
use widgets::{Dial, Graph, GraphData, HandlePoint, StageStrip};

fn load_styles(dark: bool) {
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let display = gdk::Display::default().expect("a display");
    // Like the app: dark tokens are a layer loaded while dark.
    let sheets: &[&str] = if dark {
        &["style.css", "dark.css", "widgets.css"]
    } else {
        &["style.css", "widgets.css"]
    };
    for sheet in sheets {
        let css = gtk::CssProvider::new();
        css.load_from_path(data.join(sheet));
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    gtk::IconTheme::for_display(&display).add_search_path(data.join("icons/scalable/actions"));
    for w in ["Bold", "ExtraBold"] {
        let file = data.join(format!("fonts/Manrope-{w}.ttf"));
        if let Err(e) = pangocairo::FontMap::default().add_font_file(&file) {
            eprintln!("font {}: {e}", file.display());
        }
    }
}

fn dial(label: &str, v: u16, original: u16) -> Dial {
    let d = Dial::new(label);
    d.set_range(0, 1023);
    d.show(v, original);
    d
}

fn main() -> glib::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = args
        .first()
        .cloned()
        .unwrap_or_else(|| "widgets.png".into());
    let dark = args.iter().any(|a| a == "dark");
    let zoom = args.iter().any(|a| a == "zoom");
    let narrow = args.iter().any(|a| a == "narrow");
    let app = adw::Application::builder()
        .application_id("io.github.oscyl.WidgetsDemo")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_startup(move |_| {
        if let (true, Some(s)) = (zoom, gtk::Settings::default()) {
            let dpi = if s.gtk_xft_dpi() > 0 {
                s.gtk_xft_dpi()
            } else {
                96 * 1024
            };
            s.set_gtk_xft_dpi(dpi * 3 / 2);
        }
    });
    app.connect_activate(move |app| {
        if dark {
            adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
        } else {
            adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceLight);
        }
        load_styles(dark);
        // The minilogue xd's pages, as the editor shows them.
        let d: &dyn Driver = &oscyl_minilogue_xd::DRIVER;
        let saved = d.init_program();
        let mut p = saved.clone();
        let field = |name: &str| d.fields().iter().position(|f| f.name == name).unwrap();
        for (name, v) in [("Cutoff", 618), ("Resonance", 400), ("VCO 2 Pitch", 600)] {
            let _ = d.set(&mut p, field(name), v);
        }
        let pages = d.curation().pages;
        let strip = StageStrip::new();
        strip.set_pages(
            &pages
                .iter()
                .map(|p| p.title.to_string())
                .collect::<Vec<_>>(),
        );
        let shape = |q: &[u8], page: usize, points: usize| {
            GraphData::from(d.graphs(q, page, points).remove(0))
        };
        for page in 0..pages.len() {
            let text = d.page_summary_in(&p, page, false, &|s| s.to_string());
            let short = d.page_summary_in(&p, page, true, &|s| s.to_string());
            strip.set_summary(page, &text, &short);
            let edited = shape(&p, page, 128) != shape(&saved, page, 128);
            strip.set_graph(
                page,
                shape(&p, page, 128),
                edited.then(|| shape(&saved, page, 128)),
                false,
            );
            strip.set_edits(page, usize::from(edited));
        }
        let filter = pages
            .iter()
            .position(|p| p.icon == Some("oscyl-filter-symbolic"))
            .unwrap_or(0);
        strip.select(filter);
        strip.connect_stage_selected(|s| eprintln!("selected page {s}"));

        let dials = gtk::Box::builder().spacing(8).homogeneous(true).build();
        let cutoff = dial("Cutoff", 618, 560);
        cutoff.connect_value_changed(|_, v| eprintln!("cutoff {v}"));
        dials.append(&cutoff);
        dials.append(&dial("Resonance", 210, 210));
        let eg = dial("Filter EG Int", 690, 690);
        eg.set_formatter(|v| format!("{:+.0}%", (f32::from(v) - 512.0) / 5.12));
        dials.append(&eg);
        let vel = Dial::new("Velocity");
        vel.set_range(0, 127);
        vel.show(64, 64);
        dials.append(&vel);
        cutoff.grab_focus();

        let big = Graph::with_handles();
        big.set_data(shape(&p, filter, 200), Some(shape(&saved, filter, 200)));
        big.set_description("Filter response");
        big.set_handles(vec![HandlePoint {
            at: (0.6, 0.8),
            axes: (true, true),
            name: "Cutoff and resonance".into(),
            value: "Cutoff 618 · Resonance 400".into(),
        }]);
        big.connect_handle_moved(|(i, to)| eprintln!("handle {i} to {to:?}"));
        big.set_size_request(-1, 160);

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();
        content.append(&strip);
        content.append(&big);
        content.append(&dials);
        let view = adw::ToolbarView::new();
        view.add_top_bar(&adw::HeaderBar::new());
        view.set_content(Some(&content));
        let win = adw::ApplicationWindow::builder()
            .application(app)
            .default_width(if narrow {
                390
            } else if args.iter().any(|a| a == "mid") {
                700
            } else {
                1100
            })
            .default_height(if narrow { 844 } else { 720 })
            .content(&view)
            .build();
        shot::capture_and_quit(&win, out.clone().into());
    });
    app.run_with_args(&[String::new()])
}
