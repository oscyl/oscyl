# Translate Oscyl

Oscyl keeps its words in two places:

- **gettext catalogs** in `po/`, for the desktop app, the drivers' labels,
  value names and help, and the core's messages. The mobile apps show the
  core's messages too.
- **Compose resources**, for the mobile app's own words, in
  `apps/mobile/shared/src/commonMain/composeResources/values*/strings.xml`.

English is the source. Italian is the second language, and every new string
needs it. If you are not sure of a translation, ask someone who reads the
language to review it, and say so in the pull request.

## Translate the desktop app and the core

The domain is `oscyl`. In `po/`:

- `oscyl.pot` is the template;
- `<lang>.po` is one language, for example `it.po`;
- `LINGUAS` lists the languages that ship.

1. Regenerate the template after a change to the words:

   ```sh
   cargo run -p oscyl-gtk -- --write-pot po/oscyl.pot
   ```

   A test fails when the template is out of date.

2. Start a new language, or update one:

   ```sh
   msginit -i po/oscyl.pot -l <lang> -o po/<lang>.po   # a new language
   msgmerge -U po/<lang>.po po/oscyl.pot               # an existing one
   ```

3. For a new language, add it to `po/LINGUAS`.
4. Translate the strings in `po/<lang>.po`. Keep labels short: the label
   under a dial has room for about 12 characters.
5. Try the translation from a source checkout:

   ```sh
   mkdir -p /tmp/oscyl-locale/it/LC_MESSAGES
   msgfmt po/it.po -o /tmp/oscyl-locale/it/LC_MESSAGES/oscyl.mo
   OSCYL_LOCALEDIR=/tmp/oscyl-locale LANGUAGE=it cargo run -p oscyl-gtk
   ```

The strings come from the app and from the drivers that have translations.
Today that is the minilogue xd: its parameter names, value names and help.
The other drivers stay in English. To translate a driver's words, see
[Add a driver for a new synth](add-a-synth.md#6-translate-its-words-optional).

## Translate the mobile app

1. Add each English string to `values/strings.xml`.
2. Add the same key to `values-<lang>/strings.xml` for every language.
   `StringsCompleteTest` fails when a language misses a key.
3. Use positional arguments (`%1$s`) and plurals for counts.
4. Put a comment above a string whose context is not clear.

Only the launcher label is in
`apps/mobile/androidApp/src/main/res/values/strings.xml`. It is the brand
name and has no translation. The MIDI errors that Android and iOS share are
Compose resources too.

Debug and test builds carry a generated pseudo-locale, `en-XA`. Its text is
accented, about 40% longer, and in brackets, so clipping and joined strings
show. Pick "English (XA)" in the phone's developer options, or look at the
`pseudoLocale_xa` screenshot goldens. Release builds (`-PreleaseBuild=true`)
leave it out.

Never translate the names and texts of the licences on the About page.
