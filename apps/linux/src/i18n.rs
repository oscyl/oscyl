//! Translations (gettext, domain "oscyl").
//!
//! Every user-visible string goes through [`tr`]. Strings that live in
//! `oscyl-core` (parameter names, help, value names) are plain English there
//! and translated here at the UI edge; `oscyl-gtk --write-pot` extracts them
//! together with the app's own strings into `po/oscyl.pot`.

use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};

pub const DOMAIN: &str = "oscyl";

/// Where compiled catalogues live: set `OSCYL_LOCALEDIR` at build time for
/// packaged installs; defaults to the standard system location, or on
/// Windows and macOS to the bundle's own `share/locale`.
fn localedir() -> std::path::PathBuf {
    if let Some(d) = option_env!("OSCYL_LOCALEDIR") {
        return d.into();
    }
    #[cfg(not(target_os = "linux"))]
    if let Some(share) = crate::bundle_share() {
        return share.join("locale");
    }
    "/usr/share/locale".into()
}

/// Bind the message catalogue. GTK sets the process locale from the
/// environment when it initialises, so this only binds the domain.
pub fn init() {
    // OSCYL_LOCALEDIR at run time overrides the built-in location (handy
    // for testing translations from a source checkout). Failures only mean
    // "no translation available": English is shown.
    let dir = std::env::var_os("OSCYL_LOCALEDIR").map_or_else(localedir, Into::into);
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
