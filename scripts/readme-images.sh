#!/usr/bin/env bash
# Builds every picture the README and the website show, from the apps
# themselves: whole windows of the Linux app (drawn in its demo, with an
# empty profile so none of your own libraries show) and whole screens of the
# Android app (its screenshot goldens). Nothing is cropped or animated: each
# picture is one real state of one app.
#
# Run it again when the UI changes. Needs a display (or a Wayland/X session),
# ImageMagick 7 and the Linux app built: cargo build -p oscyl-gtk.
set -euo pipefail
cd "$(dirname "$0")/.."
snap=apps/mobile/androidApp/src/test/snapshots
out=docs/images
font=apps/linux/data/fonts/AlbertSans-Bold.ttf
bg='#1a1b1c' ink='#ecebe7' legend='#9d9c97' rule='#4a4b4c'
gtk=${CARGO_TARGET_DIR:-target}/debug/oscyl-gtk
[ -x "$gtk" ] || { echo "build the Linux app first: cargo build -p oscyl-gtk" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/config" "$tmp/data"

# desktop <demo> <theme> <out>: the Linux app's window. Drawn at 1600x1000 so
# no header is clipped, then scaled to 1200x750.
desktop() {
    XDG_CONFIG_HOME=$tmp/config XDG_DATA_HOME=$tmp/data \
        "$gtk" --demo "$1" --theme "$2" --size 1600x1000 --screenshot "$tmp/shot.png" >/dev/null 2>&1
    magick "$tmp/shot.png" -filter Lanczos -resize 1200x750 -strip "$3"
}

# golden <screen> <theme>: the path of one phone golden (360x800 dp).
golden() {
    local f
    f=$(find "$snap" -name "io.github.oscyl.ui.$1.themes_WIDTH_360DP_*_$2.png" | head -n 1)
    [ -n "$f" ] || { echo "no golden for $1 ($2)" >&2; exit 1; }
    echo "$f"
}

# round <in> <out> <width> <height> <radius>: resize, round the corners, add a rim.
round() {
    local w=$3 h=$4 r=$5
    magick "$1" -filter Lanczos -resize "${w}x${h}!" \
        \( -size "${w}x${h}" xc:none -fill white -draw "roundrectangle 0,0 $((w - 1)),$((h - 1)) $r,$r" \) \
        -alpha set -compose DstIn -composite \
        \( -size "${w}x${h}" xc:none -fill none -stroke "$rule" -strokewidth 2 \
           -draw "roundrectangle 1,1 $((w - 2)),$((h - 2)) $r,$r" \) \
        -compose Over -composite -strip "$2"
}

# phone <screen> <out>: one phone screen, rounded, 400x889.
phone() { round "$(golden "$1" dark)" "$2" 400 889 30; }

# The desktop app.
desktop demo:filter:edited dark "$out/desktop-editor.png"
desktop demo:filter:edited light "$out/desktop-editor-light.png"
desktop demo:filter:edited:compare dark "$out/desktop-compare.png"
desktop demo:generate dark "$out/desktop-generate.png"
desktop demo:generate:new-oscillators dark "$out/desktop-oscillators.png"
desktop demo:filter:edited:review dark "$out/desktop-sync.png"
desktop demo:units dark "$out/desktop-units.png"

# The phone app.
phone library.LibraryScreenKt.library_programs "$out/phone-library.png"
phone editor.EditorScreenKt.editor "$out/phone-editor.png"
phone generate.GenerateScreenKt.generate "$out/phone-generate.png"
phone play.PlayScreenKt.play "$out/phone-perform.png"

# The hero: the desktop editor beside the phone editor, on the ground colour.
round "$out/desktop-editor.png" "$tmp/desk.png" 1200 750 14
round "$(golden editor.EditorScreenKt.editor dark)" "$tmp/phone.png" 338 750 26
magick -background "$bg" "$tmp/desk.png" "$tmp/phone.png" +smush 30 \
    -bordercolor "$bg" -border 40x36 -strip "$out/hero.png"

# The social preview (Settings → Social preview): 1280x640 PNG.
round "$(golden editor.EditorScreenKt.editor dark)" "$tmp/a.png" 250 556 22
round "$(golden editor.EditorScreenKt.editor_fullScreenGraph dark)" "$tmp/b.png" 250 556 22
round "$(golden generate.GenerateScreenKt.generate dark)" "$tmp/c.png" 250 556 22
magick -size 1280x640 "xc:$bg" -font "$font" -gravity northwest \
    -fill "$ink" -pointsize 104 -annotate +64+170 Oscyl \
    -pointsize 31 -annotate +68+308 "Librarian and sound editor" \
    -annotate +68+350 "for hardware synths" \
    -fill "$legend" -pointsize 22 -annotate +68+424 "Linux · Android · iOS · CLI" \
    -annotate +68+456 "Open source, one Rust core" \
    "$tmp/a.png" -geometry +500+110 -composite \
    "$tmp/b.png" -geometry +766+42 -composite \
    "$tmp/c.png" -geometry +1032+110 -composite \
    -strip "$out/social-preview.png"

ls -l "$out"/*.png
