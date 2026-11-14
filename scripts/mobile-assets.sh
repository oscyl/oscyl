#!/usr/bin/env bash
# Generate the mobile apps' icons (iOS's, and Android's adaptive launcher icon), the
# iOS launch colour and the Kotlin build phase's input list from the one source of
# each, never by hand: the app icon from the Linux app's SVG (its chart card filling the square,
# which iOS masks itself) and the launch screen's ground from design/tokens.json.
# Needs rsvg-convert and ImageMagick (`--inputs` writes only the list, and needs
# neither). `--check` validates what is committed instead
# (plists parse, asset catalogs are valid JSON, the icon is 1024 px square without
# alpha, the colours match the tokens, the input list is current), so Linux CI proves
# the files are well formed.
set -euo pipefail
cd "$(dirname "$0")/.."

app=apps/mobile/iosApp/iosApp
assets=$app/Assets.xcassets
icon=$assets/AppIcon.appiconset/AppIcon.png
source_svg=apps/linux/data/icons/hicolor/scalable/apps/io.github.oscyl.Oscyl.svg

colours() {
    # The launch colour set, light and dark grounds from the tokens.
    python3 -I - design/tokens.json <<'PY'
import json, sys
tokens = json.load(open(sys.argv[1]))["color"]
def comps(hexa):
    h = hexa.lstrip("#")
    return {"red": f"0x{h[0:2].upper()}", "green": f"0x{h[2:4].upper()}", "blue": f"0x{h[4:6].upper()}", "alpha": "1.000"}
def colour(hexa, dark=False):
    c = {"color": {"color-space": "srgb", "components": comps(hexa)}, "idiom": "universal"}
    if dark:
        c["appearances"] = [{"appearance": "luminosity", "value": "dark"}]
    return c
print(json.dumps({
    "colors": [colour(tokens["light"]["ground"]), colour(tokens["dark"]["ground"], dark=True)],
    "info": {"author": "xcode", "version": 1},
}, indent=2))
PY
}

inputs() {
    # What the Xcode "Compile Kotlin" phase reads, so Xcode skips the phase (and
    # Gradle) when none of it changed: every tracked source of the Kotlin and Rust
    # builds, as paths from the Xcode project.
    git ls-files -- \
        'apps/mobile/shared/src' 'apps/mobile/iosPlatform/src' \
        'apps/mobile/*.gradle.kts' 'apps/mobile/*/build.gradle.kts' \
        'apps/mobile/gradle.properties' 'apps/mobile/gradle/libs.versions.toml' \
        'crates/*.rs' 'crates/*Cargo.toml' 'Cargo.toml' 'Cargo.lock' 'po/*.po' \
        'design/tokens.json' 'apps/linux/data/fonts' 'LICENSE-MIT' 'LICENSE-APACHE' \
        | sed 's|^|$(SRCROOT)/../../../|'
}
xcfilelist=apps/mobile/iosApp/Configuration/KotlinInputs.xcfilelist

if [ "${1:-}" = "--inputs" ]; then
    # Only the input list (a source file was added or removed): no image tools needed.
    inputs > "$xcfilelist"
    exit 0
fi

if [ "${1:-}" = "--check" ]; then
    python3 -I - "$app" "$assets" "$icon" <<'PY'
import glob, json, plistlib, struct, sys
app, assets, icon = sys.argv[1:4]
for p in glob.glob(f"{app}/**/*.plist", recursive=True) + glob.glob(f"{app}/../**/*.entitlements", recursive=True):
    with open(p, "rb") as f:
        plistlib.load(f)
# The privacy manifest: no tracking, and a reason for each required-reason API the
# binary uses (see docs/mobile/README.md).
with open(f"{app}/PrivacyInfo.xcprivacy", "rb") as f:
    privacy = plistlib.load(f)
assert privacy["NSPrivacyTracking"] is False, "PrivacyInfo: tracking must be false"
reasons = {t["NSPrivacyAccessedAPIType"]: t["NSPrivacyAccessedAPITypeReasons"] for t in privacy["NSPrivacyAccessedAPITypes"]}
assert reasons.get("NSPrivacyAccessedAPICategoryFileTimestamp") == ["C617.1"], "PrivacyInfo: file timestamps need C617.1"
assert reasons.get("NSPrivacyAccessedAPICategorySystemBootTime") == ["35F9.1"], "PrivacyInfo: boot time needs 35F9.1"
for p in glob.glob(f"{assets}/**/Contents.json", recursive=True):
    json.load(open(p))
with open(icon, "rb") as f:
    head = f.read(26)
assert head[:8] == b"\x89PNG\r\n\x1a\n", "the icon is not a PNG"
w, h, depth, kind = struct.unpack(">IIBB", head[16:26])
assert (w, h) == (1024, 1024), f"the icon is {w}x{h}, not 1024x1024"
assert kind == 2, "the icon has alpha (or is not RGB): the App Store refuses it"
# Android's adaptive icon: 108 dp layers at every density, the foreground transparent.
for name, px in [("mdpi", 108), ("hdpi", 162), ("xhdpi", 216), ("xxhdpi", 324), ("xxxhdpi", 432)]:
    for layer, alpha in [("background", False), ("foreground", True)]:
        with open(f"{app}/../../androidApp/src/main/res/mipmap-{name}/ic_launcher_{layer}.png", "rb") as f:
            w, h, depth, kind = struct.unpack(">IIBB", f.read(26)[16:26])
        assert (w, h) == (px, px), f"mipmap-{name} {layer}: {w}x{h}, not {px}"
        # The foreground is the themed icon's mask too: it needs transparency.
        assert (kind == 6) == alpha, f"mipmap-{name} {layer}: colour type {kind}"
icon_xml = open(f"{app}/../../androidApp/src/main/res/mipmap-anydpi/ic_launcher.xml").read()
assert "<monochrome" in icon_xml, "the adaptive icon needs its monochrome layer"
# The project file is an old-style plist: no parser here, but its brackets, quotes
# and comments must balance, which catches a broken hand edit.
text = open(f"{app}/../iosApp.xcodeproj/project.pbxproj").read()
depth, i = [], 0
while i < len(text):
    c = text[i]
    if text.startswith("/*", i):
        end = text.index("*/", i + 2); i = end + 2; continue
    if text.startswith("//", i):
        i = text.index("\n", i); continue
    if c == '"':
        i += 1
        while text[i] != '"':
            i += 2 if text[i] == "\\" else 1
    elif c in "{(":
        depth.append(c)
    elif c in "})":
        assert depth and depth.pop() == {"}": "{", ")": "("}[c], f"project.pbxproj: unbalanced {c} at {i}"
    i += 1
assert not depth, "project.pbxproj: unclosed brackets"
print("iOS plists, privacy manifest, project file, asset catalogs and icon are valid")
PY
    if ! diff <(colours) "$assets/LaunchBackground.colorset/Contents.json" >/dev/null; then
        echo "$assets/LaunchBackground.colorset is not what design/tokens.json gives: run scripts/mobile-assets.sh" >&2
        exit 1
    fi
    if ! diff <(inputs) "$xcfilelist" >/dev/null; then
        echo "$xcfilelist is not the current list of sources: run scripts/mobile-assets.sh --inputs" >&2
        exit 1
    fi
    exit 0
fi

# The card's face filling the square: no shadow rect, no rounding (iOS masks the
# icon itself).
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
python3 -I - "$source_svg" "$tmp/icon.svg" <<'PY'
import re, sys
svg = open(sys.argv[1]).read()
svg = re.sub(r'\s*<rect [^>]*/>', "", svg)
svg = re.sub(r'\s*<clipPath .*?</clipPath>', "", svg)
svg = svg.replace(' clip-path="url(#face)"', "")
# The chart's rules run edge to edge; the wave sits in iOS's safe area, centred.
svg = re.sub(r'd="M(\d+) 14V106"', r'd="M\1 0V128"', svg)
svg = re.sub(r'd="M16 (\d+)H112"', r'd="M0 \1H128"', svg)
svg = svg.replace('viewBox="0 0 128 128"', 'viewBox="8 2 112 112"').replace('width="128" height="128"', 'width="1024" height="1024"')
svg = svg.replace("</defs>", '</defs>\n  <rect x="0" y="0" width="128" height="128" fill="url(#g)"/>', 1)
open(sys.argv[2], "w").write(svg)
PY
rsvg-convert -w 1024 -h 1024 "$tmp/icon.svg" -o "$tmp/icon.png"
magick "$tmp/icon.png" -background white -alpha remove -alpha off -strip "PNG24:$icon"

# Android's adaptive icon in layers: the chart card (its fill and rules, edge to edge)
# behind, and the wave with its nib in front on transparency, which is also the
# themed (monochrome) icon's mask. The wave sits in the 66 dp the launcher's mask
# always shows of the 108 dp layers.
python3 -I - "$source_svg" "$tmp/back.svg" "$tmp/front.svg" <<'PY'
import re, sys
svg = open(sys.argv[1]).read()
view = 'viewBox="-21 -27 170 170"'
head = re.sub(r'viewBox="[^"]*"', view, svg.split("<defs>")[0]).replace('width="128" height="128"', 'width="1024" height="1024"')
head = re.sub(r"\s*<!--.*?-->", "", head, flags=re.S)
defs = re.search(r"<defs>.*?</defs>", svg, re.S).group(0)
defs = re.sub(r"\s*<clipPath .*?</clipPath>", "", defs)
rules = re.search(r'<g clip-path="url\(#face\)"(.*?)</g>', svg, re.S).group(1)
rules = re.sub(r'd="M(\d+) 14V106"', r'd="M\1 -60V200"', rules)
rules = re.sub(r'd="M16 (\d+)H112"', r'd="M-60 \1H200"', rules)
mark = "".join(re.findall(r'\s*<(?:path d="M28[^>]*|circle [^>]*)/>', svg))
assert rules.count("-60V200") >= 5 and rules.count("H200") >= 5 and mark, "the icon's source changed shape"
open(sys.argv[2], "w").write(f'{head}{defs}\n  <rect x="-60" y="-60" width="260" height="260" fill="url(#g)"/>\n  <g{rules}</g>\n</svg>\n')
open(sys.argv[3], "w").write(f"{head}{mark}\n</svg>\n")
PY
res=apps/mobile/androidApp/src/main/res
for density in mdpi:1 hdpi:1.5 xhdpi:2 xxhdpi:3 xxxhdpi:4; do
    name=${density%%:*}
    scale=${density##*:}
    px=$(python3 -c "print(round(108 * $scale))")
    mkdir -p "$res/mipmap-$name"
    rsvg-convert -w "$px" -h "$px" "$tmp/back.svg" -o "$tmp/back.png"
    magick "$tmp/back.png" -alpha off -strip "PNG24:$res/mipmap-$name/ic_launcher_background.png"
    rsvg-convert -w "$px" -h "$px" "$tmp/front.svg" -o "$tmp/front.png"
    magick "$tmp/front.png" -strip "PNG32:$res/mipmap-$name/ic_launcher_foreground.png"
done
mkdir -p "$res/mipmap-anydpi"
cat > "$res/mipmap-anydpi/ic_launcher.xml" <<'XML'
<?xml version="1.0" encoding="utf-8"?>
<!-- Generated by scripts/mobile-assets.sh from the Linux app's icon. -->
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@mipmap/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
    <monochrome android:drawable="@mipmap/ic_launcher_foreground" />
</adaptive-icon>
XML
mkdir -p "$assets/LaunchBackground.colorset"
colours > "$assets/LaunchBackground.colorset/Contents.json"
inputs > "$xcfilelist"
echo "wrote $icon, the Android launcher icon, the launch colour and $xcfilelist"
