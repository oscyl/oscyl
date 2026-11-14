#!/usr/bin/env python3
"""Frame times of one journey on an attached phone, in seconds, not minutes.

    scripts/mobile-frames.py [-s SERIAL] [-p PACKAGE] [JOURNEY ...]

Drives the installed app with adb (taps and swipes on test tags) and reads
`dumpsys gfxinfo framestats`: no Gradle, no instrumentation, no reinstall. Use it
while tuning; the Macrobenchmark in :baselineprofile stays the number of record.
Journeys: scroll knob graph switch sheet programs rail play (default: all).
Prints p50/p90/p95/p99 of the whole frame, and of its UI-thread and render-thread
parts, with the share of frames over the display's budget ("late": on Android 11
one slow frame makes every later one of the gesture late while the screen still gets
one per refresh) and, the number the eye sees, the refreshes the display missed out
of the frames it showed (the last 128 of the journey).
"""
import argparse
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

ap = argparse.ArgumentParser()
ap.add_argument("-s", "--serial")
ap.add_argument("-p", "--package", default="io.github.oscyl.bench")
ap.add_argument("journeys", nargs="*")
args = ap.parse_args()
ADB = ["adb"] + (["-s", args.serial] if args.serial else [])
PKG = args.package


def sh(*cmd):
    return subprocess.run(ADB + ["shell", *cmd], capture_output=True, text=True, check=True).stdout


def nodes():
    """Every node with a resource id (a test tag): id -> list of (cx, cy, bounds)."""
    xml = subprocess.run(ADB + ["exec-out", "uiautomator", "dump", "/dev/tty"], capture_output=True, text=True).stdout
    xml = xml[: xml.rfind(">") + 1]
    found = {}
    for n in ET.fromstring(xml).iter("node"):
        rid = n.get("resource-id")
        if not rid:
            continue
        l, t, r, b = map(int, re.findall(r"-?\d+", n.get("bounds")))
        found.setdefault(rid, []).append(((l + r) // 2, (t + b) // 2, (l, t, r, b)))
    return found


def find(tag, wait=10.0):
    """The first node tagged [tag]; a uiautomator dump is itself the wait for idle."""
    end = time.monotonic() + wait
    while True:
        hit = nodes().get(tag)
        if hit or time.monotonic() > end:
            return hit
        # The dump returns when the UI is idle; ask again until the tag is there.


def must(tag):
    hit = find(tag)
    if not hit:
        sys.exit(f"no '{tag}' on screen (is {PKG} installed and unlocked?)")
    return hit


def tap(tag, index=0):
    hit = must(tag)
    x, y, _ = hit[min(index, len(hit) - 1)]
    sh("input", "tap", str(x), str(y))


def swipe(x0, y0, x1, y1, ms):
    sh("input", "swipe", *map(str, (x0, y0, x1, y1, ms)))


def open_editor():
    sh("am", "start", "-W", "-n", f"{PKG}/io.github.oscyl.android.MainActivity")
    seen = nodes()
    if "first-try" in seen:
        tap("first-try")
        tap("plate-korg-minilogue-xd")
    if "editor-list" in seen:
        return
    tap("tab-Library")
    if not find("program-row", 2.0):
        tap("library-row")
    tap("program-row", 10)
    must("editor-list")


def fling(tag):
    _, _, (l, t, r, b) = must(tag)[0]
    # Along the start edge, in the page's margin: a knob under the finger would turn instead.
    x, top, bottom = l + 20, t + (b - t) // 5, b - (b - t) // 5
    for _ in range(3):
        swipe(x, bottom, x, top, 80)
        swipe(x, top, x, bottom, 80)


def scroll():
    fling("editor-list")


def programs():
    tap("tab-Library")
    if not find("program-row", 2.0):
        tap("library-row")
    fling("program-list")
    tap("program-row", 10)


def knob():
    x, y, (l, t, r, b) = must("knob")[0]
    travel = 300
    for _ in range(3):
        swipe(x, y, x, y - travel, 900)
        swipe(x, y, x, y + travel, 250)


def graph():
    if not find("graph-expand", 0):
        # The second stage (the oscillators) has a picture: jump there from the rail.
        x, _, (l, t, r, b) = must("stage-rail")[0]
        sh("input", "tap", str(x), str(t + (b - t) // 4))
    for _ in range(3):
        tap("graph-expand")
        tap("graph-close")
    must("editor-list")


def switch():
    for tab in ("tab-Library", "tab-Generate", "tab-Editor", "tab-Library", "tab-Editor"):
        tap(tab)
    must("editor-list")


def sheet():
    for _ in range(3):
        tap("synth-chip")
        tap("sheet-grip")
    must("editor-list")


def play():
    """Open the play page, drag the pad's point round a circle for about 3 s, close it."""
    import math
    tap("play-open")
    x, y, (l, t, r, b) = must("xy-pad")[0]
    radius = (r - l) // 4
    sh("input", "motionevent", "DOWN", str(x + radius), str(y))
    # Each move is one adb round trip: the circle paces itself (72 steps, about 3 s).
    for i in range(1, 73):
        a = 2 * math.pi * i / 36
        sh("input", "motionevent", "MOVE", str(int(x + radius * math.cos(a))), str(int(y + radius * math.sin(a))))
    sh("input", "motionevent", "UP", str(x + radius), str(y))
    sh("input", "keyevent", "BACK")
    must("editor-list")


def rail():
    """Hold the stage rail until it grows, scrub across the stages, let go."""
    x, _, (l, t, r, b) = must("stage-rail")[0]
    top, bottom = t + (b - t) // 10, b - (b - t) // 10
    for _ in range(2):
        sh("input", "motionevent", "DOWN", str(x), str(top))
        time.sleep(0.7)  # A finger held still: the long press.
        steps = 24
        for i in list(range(steps + 1)) + list(range(steps, -1, -1)):
            sh("input", "motionevent", "MOVE", str(x), str(top + (bottom - top) * i // steps))
        sh("input", "motionevent", "UP", str(x), str(top))
    must("editor-list")


def pct(sorted_values, p):
    return sorted_values[min(len(sorted_values) - 1, int(len(sorted_values) * p / 100))]


def frames():
    """(total, ui, render, gpu) ms per frame since the last reset, from the app's window."""
    out = sh("dumpsys", "gfxinfo", PKG, "framestats")
    rows, cols = [], None
    for line in out.splitlines():
        if line.startswith("Flags,"):
            cols = line.rstrip(",").split(",")
        elif cols and re.match(r"^\d+,", line):
            v = dict(zip(cols, map(int, line.rstrip(",").split(","))))
            if v["Flags"] != 0 or v["FrameCompleted"] == 0:
                continue  # A first or skipped frame, or one still in flight.
            rows.append((
                (v["FrameCompleted"] - v["IntendedVsync"]) / 1e6,
                (v["SyncStart"] - v["Vsync"]) / 1e6,
                (v["FrameCompleted"] - v["SyncStart"]) / 1e6,
                # The GPU's own time, where the phone reports it (Android 11 and later).
                (v["GpuCompleted"] - v["SwapBuffers"]) / 1e6 if v.get("GpuCompleted") else 0.0,
            ))
        elif line.startswith("---PROFILEDATA---") and rows:
            cols = None
    return rows


def surface():
    """The app window's buffer layer in SurfaceFlinger: `VRI-<window>#n` on newer
    Android, `<window>#n` before."""
    listing = sh("dumpsys", "SurfaceFlinger", "--list")
    window = re.escape(PKG) + r"/[\w.]+#\d+"
    hit = re.search("VRI-" + window, listing) or re.search("^" + window, listing, re.M)
    if not hit:
        sys.exit(f"no window of {PKG} in SurfaceFlinger")
    return hit.group(0)


def missed(layer, vsync_ms):
    """(shown, missed): frames the display showed, and refreshes that passed with none
    while the journey was moving (a gap of over 150 ms is the pause between gestures)."""
    lines = sh("dumpsys", "SurfaceFlinger", "--latency", layer).splitlines()[1:]
    shown = sorted(int(l.split()[1]) / 1e6 for l in lines if len(l.split()) == 3 and 0 < int(l.split()[1]) < 2**62)
    gaps = [b - a for a, b in zip(shown, shown[1:])]
    return len(shown), sum(round(g / vsync_ms) - 1 for g in gaps if 1.5 * vsync_ms < g < 150)


def budget_ms(layer):
    """The display's refresh period now, as SurfaceFlinger reports it for the window."""
    return int(sh("dumpsys", "SurfaceFlinger", "--latency", layer).split()[0]) / 1e6


JOURNEYS = {"scroll": scroll, "knob": knob, "graph": graph, "switch": switch, "sheet": sheet, "programs": programs, "rail": rail, "play": play}
todo = args.journeys or list(JOURNEYS)
open_editor()
budget = budget_ms(surface())
print(f"{PKG}, budget {budget:.1f} ms; ms p50/p90/p95/p99")
print(f"{'journey':10} {'frames':>6} {'total':>24} {'ui thread':>24} {'render thread':>24} {'gpu':>24} {'late':>6} {'missed':>9}")
for name in todo:
    sh("dumpsys", "gfxinfo", PKG, "reset")
    layer = surface()
    sh("dumpsys", "SurfaceFlinger", "--latency-clear", layer)
    JOURNEYS[name]()
    rows = frames()
    shown, gone = missed(layer, budget)
    if not rows:
        print(f"{name:10} no frames")
        continue
    cells = []
    for i in range(4):
        col = sorted(r[i] for r in rows)
        cells.append(" / ".join(f"{pct(col, p):.1f}" for p in (50, 90, 95, 99)))
    late = sum(1 for r in rows if r[0] > budget) * 100 // len(rows)
    print(f"{name:10} {len(rows):6} {cells[0]:>24} {cells[1]:>24} {cells[2]:>24} {cells[3]:>24} {late:5}% {gone:4}/{shown:<4}")
