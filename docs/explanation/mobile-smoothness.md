# Mobile smoothness

The mobile app must feel smooth on a phone: a drag follows the finger, and
a page change never stutters. This page records what was measured, what was
slow, and what changed, in order. To measure, follow
[Judge smoothness](../how-to/mobile.md#judge-smoothness).

## The rules that came out of it

- Judge smoothness only on the release-like build. A debug build runs
  Compose without optimisation and is several times slower.
- Never use `Modifier.dropShadow` or `innerShadow`: they blur on the CPU
  for each new size. Use `softShadow`.
- Nothing that animates changes a layout size in each frame. Move or scale
  a layer instead.
- What a transition shows stays composed under it, so a transition composes
  nothing new.
- A control that becomes edited moves nothing: it keeps its space, or draws
  in place.
- A fade of a large part fades stroke by stroke
  (`CompositingStrategy.ModulateAlpha`), not through an offscreen copy.
- A class that every item of a list gets must compare by value.

## Editor drag performance (2026-11-13)

The plan's target was an editor drag at p90 16 ms or less, on a release
build. The measurement used:

- `dumpsys gfxinfo`;
- a release build (`-PreleaseBuild=true`) on the Pixel_10_b emulator
  (gfxstream, host GPU);
- a drag of Cutoff in its middle range, so the filter graph drew again in
  every frame.

Three runs of about 220 frames each gave:

- p50 21 to 22 ms, p90 27 ms, p95 27 to 28 ms;
- GPU p90 23 ms;
- 0 to 2 slow frames on the UI thread in each run.

The UI thread's work was done. The graph keeps its paths, dashes and
strokes from frame to frame, and an edit asks the core once.

## Smoothness run (2026-11-13)

A release build on the same emulator, with the frame statistics of `dumpsys
gfxinfo` for each gesture, warm (after the first pass):

| Gesture | p50 | p90 | p95 | Slow UI-thread frames | GPU p90 |
|---|---|---|---|---|---|
| 500-slot list, flings both ways | 16 ms | 16 ms | 16 ms | 0 of 118 to 294 | 14 ms |
| Editor drag (Cutoff, middle range) | 22 to 24 ms | 27 to 28 ms | 27 to 30 ms | 0 to 1 of about 220 | 23 ms |
| Stage-rail scrub (hold, then drag through every stage and back) | 21 ms | 25 to 31 ms | 32 ms | 3 to 4 of about 410 | 19 to 21 ms |

- **The list** was smooth. Its first, cold pass composed new rows and was
  slower: 20 slow UI frames of 75. So each row's words are now looked up
  once for the list, not once for each row.
- **The editor's** drag and scrub were bound by the GPU. The p90 was above
  the target because the GPU took the rest of the frame. This was the
  emulator's GPU, so the numbers do not stand for a phone. The owner
  decides on any new target, after a measurement on a real device.

## The redesign, on real phones (2026-11-13)

The owner measured the phone-first redesign on a Pixel 7 Pro (120 Hz) and a
Pixel 2 XL (60 Hz). The tool was `dumpsys gfxinfo` over adb, for each
journey.
These were **debug builds**: debuggable, no R8 and no Baseline Profile. They
were installed beside the owner's own Oscyl, as `io.github.oscyl.redesign`.
Debug Compose runs several times slower than release, so these numbers are
an upper bound.

| Journey | Pixel 7 Pro p50 / p90 / p95 | Pixel 2 XL p50 / p90 / p95 |
|---|---|---|
| Editor scroll (6 flings) | 10 / 22 / 26 ms | 12 / 36 / 109 ms |
| Knob drag (slow and fast, up and down) | 9 / 13 / 14 ms | 24 / 34 / 40 ms |
| Destination switch (5 switches) | 6 / 11 / 17 ms | 8 / 21 / 30 ms |
| Graph grow and Back (3 times) | 10 / 25 / 40 ms | 32 / 53 / 73 ms |

The design's targets are p95 8.3 ms on the 7 Pro and 16.7 ms on the 2 XL.
They are for a release build with R8 full mode and Baseline Profiles. On the debug build,
the slowest spots were the knob drag on the 2 XL and the graph's first
growth. During a knob drag, each value composed the field and its section
again. At the graph's first growth, the full-screen page composed its pager
and its controls sheet.

A screen recording of the destination switch (`adb shell screenrecord`),
read frame by frame, showed this. The old page fades out, the ground shows
for a few frames, and the new page fades in. No frame mixes the two pages
or flashes.

## The redesign, release-like (2026-11-13)

The benchmark build (R8, with the shipped Baseline Profile) installs as
`io.github.oscyl.bench`. The `:baselineprofile` module holds the profile
generator and the frame benchmarks. FrameTimingMetric,
`frameDurationCpuMs` p50 / p90 / p95 in ms, 5 iterations; the 2 XL's knob,
graph and scroll rows are later runs of 2 iterations:

| Journey | Pixel 7 Pro (120 Hz, target p95 8.3) | Pixel 2 XL (60 Hz, target p95 16.7) |
|---|---|---|
| Editor scroll | 4.4 / 6.0 / 6.7 | 7.7 / 27.6 / 28.2 |
| Program list scroll | – | 4.4 / 8.8 / 9.5 |
| Knob drag | 6.2 / 9.9 / 10.7 | 12.7 / 27.3 / 27.8 |
| Graph grow and back | 7.3 / 14.4 / 15.4 | 26.0 / 62.4 / 73.3 |
| Destination switch | 2.6 / 6.7 / 11.2 | 7.9 / 17.1 / 20.7 |
| Synth sheet | 4.9 / 6.2 / 6.9 | 11.0 / 12.5 / 14.0 |

On the 7 Pro, the frame overrun's p95 was negative (on time) for the
scroll, the knob and the sheet. On the 2 XL the render thread set the pace.
It spent 9 to 13 ms of GPU command flush on each full-screen frame of its
QHD+ panel. Still above the target: the graph's growth and return, the 2 XL's
knob and scroll, and the destination switch.

## Smoothness pass

`atrace` and `simpleperf record --app` on the 2 XL found what was slow:

- **Blurred shadows on the UI thread.** `Modifier.dropShadow` blurs a mask
  on the CPU again for every new size. A growing rail, a dragged sheet or a
  sliding plate cost 35 to 57 ms a frame. Every shadow is now `softShadow`
  (`ui/parts/SoftShadow.kt`): gradients that the GPU shades at any size.
- **Panels composed inside a frame.** A fling, a rail jump or the way back
  from the full-screen graph composed whole sections of knobs, for 15 to
  190 ms. The editor's list now keeps the program composed around the page
  (`LazyLayoutCacheWindow`), and fills it in idle time. The list stays
  composed under the full-screen graph, and the graph's page stays composed
  (hidden) under the list.
- **The graph's growth** measured and drew the picture again at every size.
  It is now a transform of the page's picture on its layer (`Portrait` in
  `EditorScreen.kt`). The page's ground and header fade, and its sheet
  rises.
- **The controls sheet** changed its height in each frame. It now slides by
  translation. It rises under a drag anywhere on it, before its list
  scrolls. It lowers from the list's top, on a tap on the picture, or on
  Back.
- **Each knob step** asked the core for the head and for five written
  values. Now the head is asked once for each gesture, and a turning knob's
  values are written once for each turn.
- **Layout jumps.** "was N" is now in the foot of the knob's scale. A
  selector's "was X" is at the end of its legend's row. Nothing moves when a
  control becomes edited, and no empty line waits under a selector.

Refreshes missed, out of frames shown, on the Pixel 2 XL (60 Hz), on the
same journeys, before and after. The graph journey opens and closes the
full-screen graph three times.

| Journey | Before | After |
|---|---|---|
| Graph grow and back | 30 / 102 | 1 to 4 / 99 |
| Editor scroll | 2 to 7 / 127 | 1 to 3 / 127 |
| Knob drag | 0 to 5 / 127 | 0 to 2 / 127 |
| Stage rail, held and scrubbed | 4 to 6 / 127 | 0 to 1 / 127 |

On the Pixel 7 Pro (120 Hz), the graph journey had frames of up to 83 ms
before. After, it missed 1 or 2 refreshes of 58. The knob and the rail missed none.
Its scroll figure does not compare: the panel changes its refresh rate
while the page moves.

## Second pass (2026-11-14)

This pass used `simpleperf record --app ... -g --trace-offcpu --clockid
monotonic`, with the samples cut to the slow frames of `dumpsys gfxinfo
framestats`, so blocked time counts too.

- **The profile was never compiled in.** `cmd package compile -m
  speed-profile` only uses a profile that the app's installer wrote. Just
  after `adb install` there is none, so the app ran interpreted.
  `scripts/mobile-phone.sh` now broadcasts
  `androidx.profileinstaller.action.INSTALL_PROFILE` first, and fails if the
  app refuses it. Numbers from before this date are of an interpreted app.
- **The Baseline Profile is older than the redesign.** It has no rule for
  `ScrollPeek`, `DockedSheet`, `SoftShadow` or `SheetLayer`, so that code
  runs interpreted until the JIT compiles it. Generate the profile again
  before a release.
- **A fade through an offscreen copy.** `graphicsLayer { alpha }` below 1
  makes a copy as large as what fades, in the first frame of the motion.
  That cost the render thread 30 ms or more in three places. One was the
  start of a scroll (the island's dim). One was each destination switch (two
  whole screens). One was the graph page's entry. Those layers now fade stroke by stroke
  (`CompositingStrategy.ModulateAlpha`).
- **A synth's read in one go.** The demo synth answers 500 slots at once,
  and the model took them all in one turn of the main loop. It now takes
  them in slices of 4 ms.
- **Work for a list that nobody looked at.** Each knob release changed the
  document's revision. Then every row of the program list, composed under
  the editor, asked the core for its picture again. The list's menu asked
  for the whole history. Now the pictures of untouched rows stay
  across an editor change, and the menu reads the history when it opens.
- **`RowWords` had no `equals`.** So every recomposition of the program
  list composed every row again.
- **The graph page's sheet** now composes its neighbours' knobs one section
  for each frame while the pager rests, and keeps them measured. A swipe's
  change at the halfway point composes nothing.

Refreshes missed, out of frames shown, on the Pixel 2 XL, with the profile
compiled in both runs and the app warm:

| Journey | Before | After |
|---|---|---|
| Program list fling | 4 to 7 / 127 | 0 to 3 / 127 |
| Editor scroll | 3 to 5 / 127 | 0 to 1 / 127 (one pass of 4 and 5 just after an install) |
| Knob drag | 4 to 6 / 127 | 0 to 3 / 127 |
| Destination switch | 2 / 121 warm, 5 to 12 from a fresh start | 2 / 120 warm, 4 to 5 from a fresh start |
| Graph grow and back | 5 / 68 | 4 to 5 / 93 (the render thread's first frame of each growth) |

Passes differ by a few refreshes on this phone, but the slow frames named
above are gone in every pass. Two remain. One render-thread frame of 20 to
30 ms comes as the graph's page first draws. Two frames of 20 to 40 ms come
as a destination comes back, because its tree is placed and drawn again. The
motion waits for them, so nothing stutters, but the tap answers late.
