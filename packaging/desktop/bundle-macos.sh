#!/usr/bin/env bash
# Oscyl.app: the GTK app with every Homebrew dylib it loads copied into
# Contents/Frameworks (dylibbundler rewrites their paths), share/ under
# Contents/Resources (see bundle-share.sh; the app points GLib at it), the
# licence texts, an icon, and an ad-hoc signature (not notarised).
# Usage: bundle-macos.sh BINARY VERSION OUT
#   BINARY   the built oscyl-gtk
#   VERSION  the app version, for Info.plist
#   OUT      the directory to create Oscyl.app in
# Needs Homebrew's gtk4, libadwaita, gettext, dylibbundler and librsvg.
set -euo pipefail

binary=$1 version=$2 out=$3
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
prefix=$(brew --prefix)
app=$out/Oscyl.app
contents=$app/Contents

mkdir -p "$contents/MacOS" "$contents/Resources/licences"
cp "$binary" "$contents/MacOS/oscyl-gtk"
dylibbundler -cd -of -b -x "$contents/MacOS/oscyl-gtk" \
    -d "$contents/Frameworks" -p @executable_path/../Frameworks/
if otool -L "$contents/MacOS/oscyl-gtk" "$contents"/Frameworks/*.dylib | grep -F "$prefix/"; then
    echo "bundle-macos.sh: the libraries above still point into Homebrew" >&2
    exit 1
fi

"$here/bundle-share.sh" "$prefix" "$contents/Resources/share"

# Homebrew's bottles need the macOS they were built for, or newer.
minimum=$(sw_vers -productVersion | cut -d. -f1)
cat > "$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>io.github.oscyl.Oscyl</string>
  <key>CFBundleName</key><string>Oscyl</string>
  <key>CFBundleDisplayName</key><string>Oscyl</string>
  <key>CFBundleExecutable</key><string>oscyl-gtk</string>
  <key>CFBundleIconFile</key><string>Oscyl</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>$minimum.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

iconset=$(mktemp -d)/Oscyl.iconset
mkdir -p "$iconset"
svg=$root/apps/linux/data/icons/hicolor/scalable/apps/io.github.oscyl.Oscyl.svg
for size in 16 32 128 256 512; do
    rsvg-convert -w "$size" -h "$size" "$svg" -o "$iconset/icon_${size}x${size}.png"
    rsvg-convert -w $((size * 2)) -h $((size * 2)) "$svg" -o "$iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$iconset" -o "$contents/Resources/Oscyl.icns"

# The licence texts of the formulae the dylibs came from, and their versions
# (Homebrew links each formula's current version at opt/<formula>).
licences=$contents/Resources/licences
cp "$root/LICENSE-MIT" "$root/LICENSE-APACHE" "$here/THIRD-PARTY.txt" "$licences/"
{
    for lib in "$contents"/Frameworks/*.dylib; do
        find -L "$prefix/opt" -maxdepth 3 -path "*/lib/${lib##*/}" -print -quit
    done | sed -n "s|^$prefix/opt/\([^/]*\)/lib/.*|\1|p"
    # The icons are a formula with no dylib.
    echo adwaita-icon-theme
} | sort -u | while read -r formula; do
    brew list --versions "$formula" >> "$licences/PACKAGES.txt"
    mkdir -p "$licences/$formula"
    find -L "$prefix/opt/$formula/" -maxdepth 1 -type f \
        \( -iname 'COPYING*' -o -iname 'LICEN[CS]E*' -o -iname 'COPYRIGHT*' \) \
        -exec cp {} "$licences/$formula/" \;
done

# Every Mach-O must be signed on Apple silicon; ad hoc, since the app is not
# notarised.
codesign --force --sign - "$contents"/Frameworks/*.dylib
codesign --force --sign - "$app"
