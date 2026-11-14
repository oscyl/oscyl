//! The locale switch changes the process-wide translator, so it runs in a
//! test binary of its own, away from tests that read English.

use oscyl_app::catalog::set_locale;
use oscyl_app::i18n::{tr, trn};

#[test]
fn the_locale_switches_core_strings() {
    set_locale("it-IT");
    assert_eq!(tr("Light"), "Chiaro");
    assert_eq!(trn("%d program", "%d programs", 2), "2 programmi");
    assert_eq!(trn("%d program", "%d programs", 1), "1 programma");
    set_locale("en");
    assert_eq!(tr("Light"), "Light");
    assert_eq!(trn("%d program", "%d programs", 1), "1 program");
}
