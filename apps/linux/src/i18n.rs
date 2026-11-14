//! Translations (gettext, domain "oscyl").
//!
//! Every user-visible string goes through [`tr`]. Strings that live in
//! `oscyl-core` (parameter names, help, value names) are plain English there
//! and translated here at the UI edge; `oscyl-gtk --write-pot` extracts them
//! together with the app's own strings into `po/oscyl.pot`.

use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};

pub const DOMAIN: &str = "oscyl";

/// Where compiled catalogues live: set `OSCYL_LOCALEDIR` at build time for
/// packaged installs; defaults to the standard system location.
const LOCALEDIR: &str = match option_env!("OSCYL_LOCALEDIR") {
    Some(d) => d,
    None => "/usr/share/locale",
};

/// Bind the message catalogue. GTK sets the process locale from the
/// environment when it initialises, so this only binds the domain.
pub fn init() {
    // OSCYL_LOCALEDIR at run time overrides the built-in location (handy
    // for testing translations from a source checkout). Failures only mean
    // "no translation available": English is shown.
    let dir = std::env::var("OSCYL_LOCALEDIR").unwrap_or_else(|_| LOCALEDIR.to_string());
    let _ = bindtextdomain(DOMAIN, dir);
    let _ = bind_textdomain_codeset(DOMAIN, "UTF-8");
    let _ = textdomain(DOMAIN);
    oscyl_app::i18n::set_translator(tr);
}

/// Translate a message.
pub fn tr(msgid: &str) -> String {
    // gettext("") returns the catalogue header, never what callers want.
    if msgid.is_empty() {
        return String::new();
    }
    gettextrs::gettext(msgid)
}

/// Translate a message with a plural form, with `%d` replaced by `n`.
pub fn trn(singular: &str, plural: &str, n: u32) -> String {
    gettextrs::ngettext(singular, plural, n).replace("%d", &n.to_string())
}
