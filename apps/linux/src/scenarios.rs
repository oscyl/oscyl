//! A synth leaving and coming back, through the real window: the worker's
//! events fed straight to [`App::on_synth`], the state that shows checked.
//! Needs a display; without one each test is skipped.

use std::rc::Rc;
use std::sync::Once;

use adw::prelude::*;

use crate::app::App;
use crate::sidebar::Place;
use crate::synth::Ev;

const XD: &str = "korg-minilogue-xd";
const PORT_A: &str = "minilogue xd:minilogue xd _ SOUND 28:1";
const PORT_B: &str = "minilogue xd:minilogue xd _ SOUND 32:1";

/// Runs `test` on GTK's test thread with a fresh window, its libraries
/// and settings in a scratch folder.
fn with_app(test: impl FnOnce(Rc<App>) + Send + std::panic::UnwindSafe + 'static) {
    let dir = std::env::temp_dir().join("oscyl-scenarios");
    static HOME: Once = Once::new();
    HOME.call_once(|| {
        let _ = std::fs::remove_dir_all(&dir);
        // SAFETY: set once, before GLib reads them, while no other
        // thread of this test binary reads the environment.
        unsafe {
            std::env::set_var("XDG_DATA_HOME", dir.join("data"));
            std::env::set_var("XDG_CONFIG_HOME", dir.join("config"));
        }
    });
    crate::on_gtk_thread(move || {
        // GLib keeps the folder it read first: never touch the real
        // libraries if another test read it before the override.
        if !gtk::glib::user_data_dir().starts_with(&dir) {
            eprintln!("GLib already uses the real data folder: scenario skipped");
            return;
        }
        thread_local! {
            // One per process: every widget test runs on this thread.
            static APP: adw::Application = {
                let app = adw::Application::builder()
                    .application_id("io.github.oscyl.Oscyl.Tests")
                    .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
                    .build();
                app.register(None::<&gtk::gio::Cancellable>).expect("register");
                app
            };
        }
        let win = APP.with(|app| crate::window::OscylWindow::new(app, Default::default()));
        test(App::new(win));
    });
}

fn xd() -> &'static dyn oscyl_driver::Driver {
    oscyl_midi::driver(XD).expect("xd")
}

fn connected(id: u64, port: &str) -> Ev {
    Ev::Connected {
        id,
        driver: xd(),
        port: port.into(),
        firmware: Some("2.10".into()),
        simulated: false,
    }
}

/// Slot `n`'s program as the synth holds it; `turn` picks another one.
fn program(n: u16, turn: usize) -> Vec<u8> {
    let demo = xd().demo_library().programs;
    demo[(n as usize + turn) % demo.len()].data.clone()
}

fn read(app: &App, id: u64, slots: std::ops::Range<u16>) {
    for n in slots {
        app.on_synth(Ev::Program(id, n, program(n, 0)));
    }
}

fn read_all(app: &App, id: u64) {
    read(app, id, 0..500);
    app.on_synth(Ev::ReadDone {
        id,
        cancelled: false,
    });
}

fn cutoff() -> usize {
    xd().fields()
        .iter()
        .position(|f| f.name == "Cutoff")
        .expect("cutoff")
}

fn entry(app: &App, port: &str) -> (u64, bool) {
    let synths = app.synths.borrow();
    let s = synths.iter().find(|s| s.port == port).expect("entry");
    (s.id, s.online)
}

/// Off after everything was read, then on: the same entry comes back and
/// the page shows its programs, not "No synth connected".
#[test]
fn reconnect_after_full_read_shows_the_synth_again() {
    with_app(|app| {
        app.on_synth(connected(1, PORT_A));
        read_all(&app, 1);
        app.show_program(3);
        let libs = app.libs.borrow().len();
        app.on_synth(Ev::Disconnected(1));
        assert_eq!(app.list.len(), 500);
        assert!(app.is_loaded(499));
        assert_eq!(entry(&app, PORT_A), (1, false));
        assert_eq!(app.libs.borrow().len(), libs, "no library made of it");
        assert_eq!(
            app.audition_target(),
            None,
            "an offline synth plays nothing"
        );
        app.on_synth(connected(2, PORT_A));
        assert_eq!(app.synths.borrow().len(), 1);
        assert_eq!(entry(&app, PORT_A), (2, true));
        assert_eq!(app.active.get(), Some(2));
        assert_eq!(*app.place.borrow(), Place::Synth);
        assert_eq!(app.list.len(), 500);
        assert_eq!(app.current.get(), Some(3));
        assert_eq!(app.audition_target(), Some(2));
    });
}

/// The first synth connects while the Synth page is already chosen (it
/// showed "No synth connected"): the page shows it.
#[test]
fn a_first_synth_shows_on_a_page_already_chosen() {
    with_app(|app| {
        app.sidebar.select(&Place::Synth);
        app.go(Place::Synth);
        assert_eq!(app.list.len(), 0);
        app.on_synth(connected(1, PORT_A));
        assert_eq!(app.list.len(), 500);
    });
}

/// Off mid-read: the rows not read say so; back on, they fill in.
#[test]
fn off_mid_read_then_back() {
    with_app(|app| {
        app.on_synth(connected(1, PORT_A));
        read(&app, 1, 0..100);
        app.on_synth(Ev::ReadDone {
            id: 1,
            cancelled: true,
        });
        app.on_synth(Ev::Disconnected(1));
        assert!(app.is_loaded(99) && !app.is_loaded(100));
        assert_eq!(crate::programs::RowSource::name(&*app, 100), "Not read");
        app.on_synth(connected(2, PORT_A));
        read_all(&app, 2);
        assert!(app.is_loaded(499));
    });
}

/// An edit made while it was gone survives its return; a slot changed on
/// the synth meanwhile shows the synth's new program.
#[test]
fn reattach_keeps_edits_and_refreshes_unedited() {
    with_app(|app| {
        app.on_synth(connected(1, PORT_A));
        read_all(&app, 1);
        app.on_synth(Ev::Disconnected(1));
        app.show_program(3);
        app.edit(cutoff(), 600);
        app.on_synth(connected(2, PORT_A));
        app.on_synth(Ev::Program(2, 3, program(3, 1)));
        app.on_synth(Ev::Program(2, 4, program(4, 1)));
        let (edited, value, slot4) = app
            .with_synth(|s| {
                let v = xd().get(s.doc.work(3).unwrap(), cutoff()).unwrap();
                (s.doc.is_edited(3), v, s.doc.work(4).unwrap().to_vec())
            })
            .unwrap();
        assert!(edited);
        assert_eq!(value, 600);
        assert_eq!(slot4, program(4, 1));
    });
}

/// Off mid-sync: the edits stay edited and can be synced once it is back.
#[test]
fn offline_mid_sync_keeps_edits() {
    with_app(|app| {
        app.on_synth(connected(1, PORT_A));
        read_all(&app, 1);
        for slot in [3, 7] {
            app.show_program(slot);
            app.edit(cutoff(), 600);
        }
        app.on_synth(Ev::WriteFailed(1, "no reply".into()));
        app.on_synth(Ev::Disconnected(1));
        let primary = || {
            app.win
                .lookup_action("primary")
                .and_downcast::<gtk::gio::SimpleAction>()
                .is_some_and(|a| a.is_enabled())
        };
        assert_eq!(app.with_synth(|s| s.doc.edited_count()), Some(2));
        assert!(!primary(), "nothing to sync to while it is gone");
        app.on_synth(connected(2, PORT_A));
        read_all(&app, 2);
        assert_eq!(app.with_synth(|s| s.doc.edited_count()), Some(2));
        assert!(primary());
    });
}

/// Two of one model: only the one that cycles goes offline, and it comes
/// back to its own entry.
#[test]
fn two_xds_only_the_cycled_one_goes_offline() {
    with_app(|app| {
        app.on_synth(connected(1, PORT_A));
        app.on_synth(connected(2, PORT_B));
        app.on_synth(Ev::Disconnected(2));
        assert_eq!(entry(&app, PORT_A), (1, true));
        assert_eq!(entry(&app, PORT_B), (2, false));
        app.on_synth(connected(3, PORT_B));
        assert_eq!(entry(&app, PORT_B), (3, true));
        assert_eq!(app.synths.borrow().len(), 2);
    });
}

/// A simulated synth still leaves when disconnected.
#[test]
fn simulated_synths_still_leave() {
    with_app(|app| {
        app.on_synth(Ev::Connected {
            id: 1,
            driver: xd(),
            port: "simulated".into(),
            firmware: None,
            simulated: true,
        });
        app.on_synth(Ev::Disconnected(1));
        assert!(app.synths.borrow().is_empty());
    });
}
