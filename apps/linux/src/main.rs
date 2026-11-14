//! Oscyl for GNOME.

mod actions;
mod app;
mod appearance;
mod callback;
mod dialogs;
mod editor;
mod files;
mod generator;
mod guide;
mod history;
mod i18n;
use oscyl_app::libraries;
mod library_menu;
mod onboarding;
mod perform;
mod pot;
mod program_menu;
mod programs;
mod review;
use oscyl_app::settings;
mod shot;
mod sidebar;
use oscyl_app::slots;
use oscyl_app::state;
use oscyl_app::synth;
mod synth_events;
mod units;
mod widgets;
mod window;

use adw::prelude::*;
use gtk::{gio, glib};

pub const APP_ID: &str = "io.github.oscyl.Oscyl";

/// Developer options parsed before GTK sees the command line.
#[derive(Debug, Default, Clone)]
pub struct Options {
    /// Render the main window to this PNG and quit (for design reviews).
    pub screenshot: Option<std::path::PathBuf>,
    pub size: Option<(i32, i32)>,
    pub theme: Option<settings::Theme>,
    pub zoom: Option<f64>,
    /// Force the simulated synth (also `OSCYL_SIMULATOR=1`).
    pub simulator: bool,
    /// Scripted state for screenshots; see [`app::App::run_demo`].
    pub demo: Option<String>,
    /// Write the translation template and exit.
    pub write_pot: Option<std::path::PathBuf>,
}

fn parse_options() -> Result<Options, String> {
    let mut o = Options {
        simulator: std::env::var_os("OSCYL_SIMULATOR").is_some_and(|v| v == "1"),
        ..Options::default()
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "--screenshot" => o.screenshot = Some(value("--screenshot")?.into()),
            "--size" => {
                let v = value("--size")?;
                let (w, h) = v.split_once('x').ok_or("--size is WIDTHxHEIGHT")?;
                o.size = Some((
                    w.parse().map_err(|_| "bad width")?,
                    h.parse().map_err(|_| "bad height")?,
                ));
            }
            "--theme" => {
                o.theme = Some(match value("--theme")?.as_str() {
                    "light" => settings::Theme::Light,
                    "dark" => settings::Theme::Dark,
                    "amoled" => settings::Theme::Amoled,
                    "system" => settings::Theme::System,
                    t => return Err(format!("unknown theme {t:?}")),
                })
            }
            "--zoom" => {
                let z: f64 = value("--zoom")?.parse().map_err(|_| "bad zoom")?;
                o.zoom = Some(settings::Settings::clamp_zoom(z));
            }
            "--simulator" => o.simulator = true,
            "--demo" => o.demo = Some(value("--demo")?),
            "--write-pot" => o.write_pot = Some(value("--write-pot")?.into()),
            other => return Err(format!("unknown option {other:?}")),
        }
    }
    Ok(o)
}

fn main() -> glib::ExitCode {
    let options = match parse_options() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("oscyl-gtk: {e}");
            return glib::ExitCode::FAILURE;
        }
    };
    if let Some(path) = &options.write_pot {
        return match pot::write(path) {
            Ok(()) => glib::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("oscyl-gtk: {e}");
                glib::ExitCode::FAILURE
            }
        };
    }
    gio::resources_register_include!("oscyl.gresource").expect("bundled resources");
    i18n::init();

    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(if options.screenshot.is_some() {
            gio::ApplicationFlags::NON_UNIQUE
        } else {
            gio::ApplicationFlags::empty()
        })
        .build();
    app.connect_startup(|_| appearance::init());
    app.connect_activate(move |app| {
        let mut s = settings::Settings::load(&appearance::settings_path());
        s.overrides = settings::Overrides {
            theme: options.theme,
            zoom: options.zoom,
            simulator: options.simulator,
        };
        appearance::apply(&s);
        let win = window::OscylWindow::new(app, s);
        if let Some((w, h)) = options.size {
            win.set_default_size(w, h);
        }
        let controller = app::App::new(win.clone());
        // Scripted design states stay on their own data, even with a
        // synth plugged in.
        if let Some(demo) = &options.demo {
            controller.run_demo(demo);
        } else {
            controller.watch_hotplug();
        }
        if options.demo.is_none() && options.screenshot.is_none() {
            controller.first_run();
        }
        // The controller lives as long as its window.
        win.window
            .connect_close_request(move |_| controller.close_request());
        match &options.screenshot {
            Some(path) => shot::capture_and_quit(&win.window, path.clone()),
            None => win.present(),
        }
    });
    // GTK must not see our developer options.
    app.run_with_args(&[std::env::args().next().unwrap_or_default()])
}

/// Run a widget test on the one thread that owns GTK. Test threads run in
/// parallel, and GTK belongs to the thread that initialised it, so every
/// widget test goes through this thread. Without a display the test is
/// skipped.
#[cfg(test)]
pub fn on_gtk_thread(test: impl FnOnce() + Send + std::panic::UnwindSafe + 'static) {
    use std::sync::{Mutex, OnceLock, mpsc};
    type Job = Box<dyn FnOnce() + Send>;
    static WORKER: OnceLock<Mutex<mpsc::Sender<Job>>> = OnceLock::new();
    let worker = WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Job>();
        std::thread::spawn(move || {
            let ready = adw::init().is_ok();
            // Without a display each job is dropped unrun: a skip.
            rx.into_iter().filter(|_| ready).for_each(|job| job());
        });
        Mutex::new(tx)
    });
    let (done, result) = mpsc::channel();
    let job: Job = Box::new(move || {
        let _ = done.send(std::panic::catch_unwind(test));
    });
    worker
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .send(job)
        .expect("the GTK test thread is running");
    if let Ok(Err(panic)) = result.recv() {
        std::panic::resume_unwind(panic);
    }
}
