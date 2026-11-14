#!/usr/bin/env bash
# Builds the README collages and the social preview from pictures the apps
# drew themselves: the Android screenshot goldens and docs/images/*.png.
# Run it again when the goldens change. Needs ImageMagick 7 with WebP.
set -euo pipefail
cd "$(dirname "$0")/.."

snap=apps/mobile/androidApp/src/test/snapshots
out=docs/images
font=apps/linux/data/fonts/AlbertSans-Bold.ttf
bg='#1a1b1c' ink='#ecebe7' legend='#9d9c97' rule='#4a4b4c'
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# golden <screen> <theme>: the path of one phone golden (360x800 dp).
golden() {
    local f
    f=$(find "$snap" -name "io.github.oscyl.ui.$1.themes_*_$2.png" | head -n 1)
    [ -n "$f" ] || { echo "no golden for $1 ($2)" >&2; exit 1; }
    echo "$f"
}

# round <in> <out> <width> <height> <radius>: resize, round the corners, add a rim.
round() {
    local w=$3 h=$4 r=$5
    magick "$1" -resize "${w}x${h}!" \
        \( -size "${w}x${h}" xc:none -fill white -draw "roundrectangle 0,0 $((w - 1)),$((h - 1)) $r,$r" \) \
        -alpha set -compose DstIn -composite \
        \( -size "${w}x${h}" xc:none -fill none -stroke "$rule" -strokewidth 2 \
           -draw "roundrectangle 1,1 $((w - 2)),$((h - 2)) $r,$r" \) \
        -compose Over -composite "$2"
}

# card <in> <out> <title> <line>: a rounded phone with a caption under it.
card() {
    round "$1" "$tmp/r.png" 330 733 26
    magick "$tmp/r.png" -background none -gravity north -extent 330x815 \
        -font "$font" -gravity north \
        -fill "$ink" -pointsize 23 -annotate +0+752 "$3" \
        -fill "$legend" -pointsize 16 -annotate +0+785 "$4" "$2"
}

# row <out> <card>...: cards side by side on the ground colour.
row() {
    local o=$1
    shift
    magick -background "$bg" "$@" +smush 30 -bordercolor "$bg" -border 44x40 \
        -quality 88 -define webp:method=6 "$o"
}

card "$(golden library.LibraryScreenKt.library_programs dark)" "$tmp/1.png" Library "Every program, with its picture"
card "$(golden editor.EditorScreenKt.editor dark)" "$tmp/2.png" Editor "Stage by stage, along the signal"
card "$(golden editor.EditorScreenKt.editor_fullScreenGraph dark)" "$tmp/3.png" Graphs "Saved curve behind the edited one"
card "$(golden generate.GenerateScreenKt.generate dark)" "$tmp/4.png" Generate "Six new sounds from four dials"
card "$(golden play.PlayScreenKt.play dark)" "$tmp/5.png" Perform "An XY pad and your own knobs"
row "$out/tour.webp" "$tmp"/{1,2,3,4,5}.png

# The hero: the Linux app beside the phone.
round "$out/linux-editor-dark.png" "$tmp/desk.png" 1200 750 14
round "$out/android-phone-editor.png" "$tmp/phone.png" 334 750 26
row "$out/hero.webp" "$tmp/desk.png" "$tmp/phone.png"

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

ls -l "$out"/*.webp "$out/social-preview.png"
