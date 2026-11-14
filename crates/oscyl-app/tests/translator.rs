use oscyl_app::i18n::{set_translator, tr};

#[test]
fn first_translator_wins() {
    set_translator(|s| s.to_uppercase());
    assert_eq!(tr("a"), "A");
    set_translator(|s| format!("x{s}"));
    assert_eq!(tr("a"), "A");
    assert_eq!(tr(""), "");
}
