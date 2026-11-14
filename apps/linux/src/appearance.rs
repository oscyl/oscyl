//! Theme, zoom, motion and the bundled font.

use std::cell::{Cell, RefCell};

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::settings::{Settings, Theme};

/// Where the settings live: `~/.config/oscyl/settings.json`.
pub fn settings_path() -> std::path::PathBuf {
    gtk::glib::user_config_dir()
        .join("oscyl")
        .join("settings.json")
}

const RESOURCE: &str = "/io/github/oscyl/Oscyl";
/// Albert Sans is the display face (program titles only); running text uses
/// the desktop's UI font, which is drawn for small sizes. A static Bold
/// instance made from the variable font with fontTools' instancer.
const FONT_FILES: [&str; 1] = ["AlbertSans-Bold.ttf"];

thread_local! {
    static DARK: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
    static AMOLED: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
    static BASE_DPI: Cell<Option<i32>> = const { Cell::new(None) };
    static ICON_SCALE: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

/// Register resources, the stylesheet, icons and the bundled font. Call
/// once after GTK is initialised.
pub fn init() {
    let display = gdk::Display::default().expect("a display");
    let theme = gtk::IconTheme::for_display(&display);
    theme.add_resource_path(&format!("{RESOURCE}/icons"));
    // The app's own icon (hicolor layout), for About and anything else that names it.
    theme.add_resource_path(&format!("{RESOURCE}/icons/hicolor"));

    // Tokens first (generated from design/tokens.json; light is the base),
    // then the component sheets that use them.
    for sheet in [
        "tokens-light.css",
        "style.css",
        "widgets.css",
        "app.css",
        "editor.css",
    ] {
        let css = gtk::CssProvider::new();
        css.load_from_resource(&format!("{RESOURCE}/{sheet}"));
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    register_font();
    crisp_text();

    // Dark tokens follow libadwaita's resolved dark state, which covers both
    // the system preference and a forced scheme.
    let manager = adw::StyleManager::default();
    set_layer(&DARK, "tokens-dark.css", manager.is_dark(), 1);
    manager.connect_dark_notify(|m| set_layer(&DARK, "tokens-dark.css", m.is_dark(), 1));
}

/// Add or remove a token stylesheet layered above the base tokens.
fn set_layer(
    slot: &'static std::thread::LocalKey<RefCell<Option<gtk::CssProvider>>>,
    file: &str,
    on: bool,
    rank: u32,
) {
    let display = gdk::Display::default().expect("a display");
    slot.with(|cell| {
        let mut cell = cell.borrow_mut();
        match (on, cell.as_ref()) {
            (true, None) => {
                let p = gtk::CssProvider::new();
                p.load_from_resource(&format!("{RESOURCE}/{file}"));
                gtk::style_context_add_provider_for_display(
                    &display,
                    &p,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + rank,
                );
                *cell = Some(p);
            }
            (false, Some(p)) => {
                gtk::style_context_remove_provider_for_display(&display, p);
                *cell = None;
            }
            _ => {}
        }
    });
}

/// GTK's automatic font rendering turns hinting off at fractional scales
/// (125 % on a 4K screen): horizontal strokes then straddle two pixel rows
/// and read as blurred or cut. Manual rendering applies the desktop's own
/// antialiasing and hinting choices (GNOME's "slight" by default), which
/// keeps stems on the pixel grid. Glyph advances stay unhinted so letter
/// spacing does not jump at fractional scales.
fn crisp_text() {
    if let Some(s) = gtk::Settings::default() {
        s.set_gtk_font_rendering(gtk::FontRendering::Manual);
        s.set_gtk_hint_font_metrics(false);
    }
}

/// Pango can only load font files from disk, so the bundled face is
/// copied to the cache directory (when it changed) and registered.
fn register_font() {
    let dir = glib::user_cache_dir().join("oscyl").join("fonts");
    let map = pangocairo::FontMap::default();
    for file in FONT_FILES {
        let Ok(bytes) = gio::resources_lookup_data(
            &format!("{RESOURCE}/fonts/{file}"),
            gio::ResourceLookupFlags::NONE,
        ) else {
            glib::g_warning!("oscyl", "bundled font {file} missing from resources");
            continue;
        };
        let path = dir.join(file);
        let current = std::fs::read(&path)
            .map(|b| b == bytes.as_ref())
            .unwrap_or(false);
        if !current
            && let Err(e) =
                std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, &bytes))
        {
            glib::g_warning!("oscyl", "cannot cache bundled font {file}: {e}");
            continue;
        }
        if let Err(e) = map.add_font_file(&path) {
            glib::g_warning!("oscyl", "cannot register bundled font {file}: {e}");
        }
    }
}

/// Apply theme, zoom and motion preferences.
pub fn apply(settings: &Settings) {
    let manager = adw::StyleManager::default();
    manager.set_color_scheme(match settings.theme() {
        Theme::System => adw::ColorScheme::Default,
        Theme::Light => adw::ColorScheme::ForceLight,
        Theme::Dark | Theme::Amoled => adw::ColorScheme::ForceDark,
    });
    set_amoled(settings.theme() == Theme::Amoled);
    set_zoom(settings.zoom());
    if let Some(gtk_settings) = gtk::Settings::default() {
        if settings.reduce_motion {
            gtk_settings.set_gtk_enable_animations(false);
        } else {
            gtk_settings.reset_property("gtk-enable-animations");
        }
    }
}

fn set_amoled(on: bool) {
    set_layer(&AMOLED, "tokens-amoled.css", on, 2);
}

/// Zoom scales all text through GTK's own text-scaling setting (the same
/// mechanism as the desktop "Large Text" option); Oscyl's custom widgets
/// size themselves in ems, so graphs and dials scale with it.
fn set_zoom(zoom: f64) {
    let Some(gtk_settings) = gtk::Settings::default() else {
        return;
    };
    let base = BASE_DPI.with(|b| {
        b.get().unwrap_or_else(|| {
            let dpi = gtk_settings.gtk_xft_dpi();
            let dpi = if dpi > 0 { dpi } else { 96 * 1024 };
            b.set(Some(dpi));
            dpi
        })
    });
    let zoom = Settings::clamp_zoom(zoom);
    gtk_settings.set_gtk_xft_dpi((f64::from(base) * zoom).round() as i32);
    set_icon_scale(zoom);
}

/// Text scaling does not resize icons, so interface icons (in buttons,
/// rows, the strip and pills) follow the zoom through their CSS icon size.
fn set_icon_scale(zoom: f64) {
    let display = gdk::Display::default().expect("a display");
    let px = (16.0 * zoom).round();
    let css = format!(
        "button image, row image, stagestrip image, \
         .program-list image, .sources image:not(.synth-icon) {{ -gtk-icon-size: {px}px; }}"
    );
    ICON_SCALE.with(|cell| {
        let mut cell = cell.borrow_mut();
        let provider = cell.get_or_insert_with(|| {
            let p = gtk::CssProvider::new();
            gtk::style_context_add_provider_for_display(
                &display,
                &p,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 3,
            );
            p
        });
        provider.load_from_string(&css);
    });
}
