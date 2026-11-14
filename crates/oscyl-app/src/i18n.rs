//! Translation hook. Strings here are plain English; each app installs its
//! translator once at start ([`set_translator`]): gettext on Linux, the
//! embedded catalogues on mobile. Without one, [`tr`] returns the msgid.

use std::sync::OnceLock;

static TRANSLATOR: OnceLock<fn(&str) -> String> = OnceLock::new();

/// Install the app's translator. Only the first call counts.
pub fn set_translator(f: fn(&str) -> String) {
    let _ = TRANSLATOR.set(f);
}

static PLURAL: OnceLock<fn(&str, &str, u32) -> String> = OnceLock::new();

/// Install the app's plural translator. Only the first call counts.
pub fn set_plural_translator(f: fn(&str, &str, u32) -> String) {
    let _ = PLURAL.set(f);
}

/// Translate a message with a plural form for `n`, with `%d` replaced by
/// `n`. Without a translator, English rules: `singular` for one.
pub fn trn(singular: &str, plural: &str, n: u32) -> String {
    let text = match PLURAL.get() {
        Some(f) => f(singular, plural, n),
        None if n == 1 => singular.to_string(),
        None => plural.to_string(),
    };
    text.replace("%d", &n.to_string())
}

/// Translate a message.
pub fn tr(msgid: &str) -> String {
    if msgid.is_empty() {
        return String::new();
    }
    match TRANSLATOR.get() {
        Some(f) => f(msgid),
        None => msgid.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untranslated_returns_msgid() {
        assert_eq!(tr("Light"), "Light");
    }

    #[test]
    fn empty_stays_empty() {
        assert_eq!(tr(""), "");
    }
}
