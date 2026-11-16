#!/usr/bin/env bash
# The data the GTK app needs beside its binary on Windows and macOS: GTK's
# settings schemas, the Adwaita icons, Oscyl's app icon and the translations
# (Oscyl's, GTK's and libadwaita's).
# Usage: bundle-share.sh PREFIX SHARE
#   PREFIX  where GTK is installed (MSYS2's /ucrt64, or $(brew --prefix))
#   SHARE   the bundle's share directory, created if missing
set -euo pipefail

prefix=$1 share=$2
root=$(cd "$(dirname "$0")/../.." && pwd)

schemas=$share/glib-2.0/schemas
mkdir -p "$schemas" "$share/icons/hicolor/scalable/apps"
cp "$prefix"/share/glib-2.0/schemas/org.gtk.gtk4.*.gschema.xml "$schemas/"
glib-compile-schemas "$schemas"

cp -R "$prefix/share/icons/Adwaita" "$share/icons/"
cp "$prefix/share/icons/hicolor/index.theme" "$share/icons/hicolor/"
cp "$root/apps/linux/data/icons/hicolor/scalable/apps/io.github.oscyl.Oscyl.svg" \
    "$share/icons/hicolor/scalable/apps/"
cache=$(command -v gtk4-update-icon-cache || command -v gtk-update-icon-cache)
for theme in "$share"/icons/*/; do
    "$cache" -q -t -f "$theme"
done

while read -r lang; do
    [[ -z $lang || $lang == \#* ]] && continue
    dir=$share/locale/$lang/LC_MESSAGES
    mkdir -p "$dir"
    msgfmt "$root/po/$lang.po" -o "$dir/oscyl.mo"
    for domain in gtk40 libadwaita; do
        if [[ -e $prefix/share/locale/$lang/LC_MESSAGES/$domain.mo ]]; then
            cp "$prefix/share/locale/$lang/LC_MESSAGES/$domain.mo" "$dir/"
        fi
    done
done < "$root/po/LINGUAS"
