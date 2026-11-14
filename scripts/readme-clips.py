#!/usr/bin/env python3
"""Builds the README's feature clips: short looping animated WebP tiles, one
idea each, that step through real states of the apps. Nothing is drawn here
but the move from one picture to the next.

The phone pictures are the Android screenshot goldens. The desktop ones are
docs/images/desktop/*.png, which the Linux app draws itself, with an empty
profile so that none of your own libraries show:

    XDG_CONFIG_HOME=$(mktemp -d) XDG_DATA_HOME=$(mktemp -d) cargo run -p oscyl-gtk -- \\
        --demo demo:filter:edited:blend --theme dark --size 1360x850 \\
        --screenshot docs/images/desktop/filter-blend.png

Run it again when they change. Needs Pillow with WebP.

    scripts/readme-clips.py [name ...]
"""

import glob
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
SNAP = ROOT / "apps/mobile/androidApp/src/test/snapshots"
OUT = ROOT / "docs/images"
W, H, FPS = 600, 400, 30
STEP = 1000 // FPS


def golden(name):
    """A phone golden: `screen` (the dark theme) or `screen@suffix`."""
    screen, _, suffix = name.partition("@")
    found = glob.glob(str(SNAP / f"io.github.oscyl.ui.{screen}_WIDTH_360DP*{suffix or '_dark'}.png"))
    if not found:
        sys.exit(f"no golden for {name}")
    return Image.open(found[0]).convert("RGB")


def desktop(name):
    return Image.open(OUT / "desktop" / f"{name}.png").convert("RGB")


def fill(img, box):
    """The part of a picture that fills the tile."""
    return img.resize((W, H), Image.LANCZOS, box=box)


def phone(name, top):
    return fill(golden(name), (0, top, 1080, top + 1080 * H // W))


def desk(name, top=128):
    """The editor's pane of the desktop window."""
    return fill(desktop(name), (548, top, 1352, top + 804 * H // W))


def island(strip, top, height):
    """One state of the island, at the foot of the library it floats over."""
    screen = golden("library.LibraryScreenKt.library_programs.themes")
    ground = screen.getpixel((4, 4))
    band = golden(strip).crop((0, top, 1080, top + height)).resize((W, height * W // 1080), Image.LANCZOS)
    y = H - band.height - 4
    tile = Image.new("RGB", (W, H), ground)
    tile.paste(screen.resize((W, y), Image.LANCZOS, box=(0, 1570 - y * 1080 // W, 1080, 1570)))
    # The list fades out under the island, as it does in the app.
    fade = Image.linear_gradient("L").resize((W, 110))
    tile.paste(ground, (0, y - 110, W, y), fade)
    tile.paste(band, (0, y))
    return tile


def rail(name):
    """The stage rail, as it sits at the right edge of the editor."""
    img = golden(name)
    tile = Image.new("RGB", (W, H), img.getpixel((4, 4)))
    part = img.crop((300, 0, 1080, 960)).resize((780 * H // 960, H), Image.LANCZOS)
    tile.paste(part, (W - part.width - 40, 0))
    return tile


ISLAND = "shell.IslandKt.island_states.themes"
RAIL = "parts.StageRailKt.stageRail"
# A clip: (how one state gives way to the next, [(state, seconds it holds)]).
CLIPS = {
    "clip-island": ("fade", lambda: [(island(ISLAND, 228 * i + 12, 240), s) for i, s in enumerate([0.9, 0.7, 0.9, 0.9, 1.1])]),
    "clip-search": ("fade", lambda: [
        (island("shell.IslandKt.island_search.themes", 0, 264), 1.0),
        (island("shell.IslandKt.island_searching.themes", 0, 264), 1.2),
        (island("shell.IslandKt.island_search.themes", 0, 264), 0.5),
        (island("shell.IslandNoticeKt.islandNotice.themes", 0, 276), 1.4),
    ]),
    "clip-rail": ("fade", lambda: [(rail(f"{RAIL}_resting.themes"), 1.0), (rail(f"{RAIL}_pressed.themes"), 0.25), (rail(f"{RAIL}_held_scrubbing.themes"), 1.5)]),
    "clip-compare": ("fade", lambda: [(phone("editor.EditorScreenKt.editor.themes", 200), 1.2), (phone("editor.EditorScreenKt.editor_comparing.themes", 200), 1.2)]),
    "clip-generate": ("fade", lambda: [(phone("generate.GenerateScreenKt.generate_empty.themes", 1150), 1.0), (phone("generate.GenerateScreenKt.generate.themes", 1150), 1.8)]),
    "clip-sync": ("fade", lambda: [(phone("review.ReviewSheetKt.review_save.themes", 1640), 1.3), (phone("review.ReviewSheetKt.review_syncing.themes", 1640), 1.5)]),
    "clip-desktop-editor": ("fade", lambda: [(desk("filter"), 0.9), (desk("filter-blend"), 0.12), (desk("filter-edited"), 1.5), (desk("filter-compare"), 1.1)]),
    "clip-desktop-generate": ("fade", lambda: [(desk("generate-empty", 62), 1.0), (desk("generate", 314), 1.4), (desk("generate-keep", 314), 1.6)]),
    "clip-desktop-themes": ("wipe", lambda: [(desk("filter-edited-light"), 1.1), (desk("filter-edited"), 1.1), (desk("filter-edited-amoled"), 1.1)]),
}


def ease(t):
    return t * t * (3 - 2 * t)


def fade(a, b, t):
    return Image.blend(a, b, ease(t))


def wipe(a, b, t):
    """The next state sweeps in from the left, behind a soft edge."""
    edge = int(-80 + (W + 160) * ease(t))
    mask = Image.new("L", (W, H), 0)
    draw = ImageDraw.Draw(mask)
    for i in range(80):
        draw.line((edge - i, 0, edge - i, H), fill=int(255 * (i + 1) / 80))
    if edge > 80:
        draw.rectangle((0, 0, edge - 80, H), fill=255)
    return Image.composite(b, a, mask)


CORNERS = Image.new("L", (W * 2, H * 2), 0)
ImageDraw.Draw(CORNERS).rounded_rectangle((0, 0, W * 2 - 1, H * 2 - 1), 44, fill=255)
CORNERS = CORNERS.resize((W, H), Image.LANCZOS)


def build(name):
    how, states = CLIPS[name]
    states = states()
    move = {"fade": (fade, 0.3), "wipe": (wipe, 0.7)}[how]
    frames, times = [], []
    for i, (img, hold) in enumerate(states):
        frames.append(img)
        times.append(int(hold * 1000))
        nxt = states[(i + 1) % len(states)][0]
        n = int(move[1] * FPS)
        for k in range(1, n):
            frames.append(move[0](img, nxt, k / n))
            times.append(STEP)
    frames = [f.convert("RGBA") for f in frames]
    for f in frames:
        f.putalpha(CORNERS)
    out = OUT / f"{name}.webp"
    frames[0].save(out, save_all=True, append_images=frames[1:], duration=times, loop=0, quality=86, method=6)
    print(f"{out.relative_to(ROOT)}: {sum(times) / 1000:.1f} s, {out.stat().st_size // 1024} KB")
    return [s[0] for s in states]


if __name__ == "__main__":
    for clip in sys.argv[1:] or CLIPS:
        build(clip)
