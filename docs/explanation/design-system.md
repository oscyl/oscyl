# The design system

Oscyl's apps share one look, which the design system calls "Ulm Hi-Fi".
The apps look like 1960s audio equipment from the Ulm school of design. The
full design system is
[`DESIGN.md`](https://github.com/oscyl/oscyl/blob/main/DESIGN.md), at the
root of the repository. This page explains its main ideas, and where its
values come from.

## The ideas

- **Less, but every widget exact.** Grey or anthracite enamel, fine printed
  legends, 1 px rules and small soft corners. Colour only ever means a
  function.
- **Two function colours.** Green means connected, synced or live. Orange
  means edited, or waiting to be written. Everything else is grey, and every
  coloured mark also has a shape and a word.
- **Selection is a plate.** A selected segment, stage tab or tile becomes a
  solid plate, in the opposite tone of the ground, with its own text colour.
- **The editor is a compare bench.** Every control keeps the saved sound on
  its face, so an edit reads as a distance from the saved value. Hold to
  compare plays the saved program, on the screen and on the synth.
- **One design language.** The desktop app and the phone apps use the same
  parts for the same jobs: the same dials, stage tabs, graphs and review of
  changes.

## Where the values come from

Every colour, shape, spacing and motion value is in one file,
[`design/tokens.json`](https://github.com/oscyl/oscyl/blob/main/design/tokens.json).

- The desktop app's build (`apps/linux/build.rs`) makes its light, dark and
  AMOLED stylesheets from the tokens, as `--oscyl-<role>` custom
  properties.
- The mobile apps generate their colours, shapes and motion from the same
  file, as `OscylTokens.kt`.
- The iOS launch colours come from it too, through
  `scripts/mobile-assets.sh`.

When `DESIGN.md` and `tokens.json` disagree, `tokens.json` wins. Change a
value in the tokens, never in a stylesheet.

## Why one system

A user moves between the phone and the computer. When the same control
looks and acts the same on both, nothing must be learned twice. And a
contributor who changes a part on one app knows what the other app must
match.
