# Translations

Oscyl uses gettext (domain `oscyl`).

- `oscyl.pot`: the template. Regenerate it with
  `cargo run -p oscyl-gtk -- --write-pot po/oscyl.pot`.
- `<lang>.po`: one file per language (for example `it.po`). Start a new
  language with `msginit -i po/oscyl.pot -l <lang> -o po/<lang>.po`, and
  update an existing one with `msgmerge -U po/<lang>.po po/oscyl.pot`.
- `LINGUAS`: the languages that ship.

To try a translation from a source checkout:

```sh
msgfmt po/it.po -o /tmp/oscyl-locale/it/LC_MESSAGES/oscyl.mo  # mkdir -p first
OSCYL_LOCALEDIR=/tmp/oscyl-locale LANGUAGE=it cargo run -p oscyl-gtk
```

Strings come from the app and from the drivers with translations (today the
minilogue xd: parameter names, value names, help); other drivers stay English.
Keep translations short enough for the controls: labels under
dials are about 12 characters wide.
