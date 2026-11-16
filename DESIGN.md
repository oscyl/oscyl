---
name: Oscyl
description: Ulm Hi-Fi. Program librarian and editor for hardware synths (minilogue xd, prologue, Prophet Rev2), drawn like 1960s Ulm-school audio equipment; native on Linux (libadwaita) and on mobile (Android and iOS, Compose Multiplatform) with the same identity.
colors:
  ground: "#e7e6e2"
  sidebar: "#dedcd7"
  list: "#ecebe7"
  raised: "#f6f5f2"
  plate: "#2a2b2c"
  on-plate: "#ecebe7"
  plate-legend: "#a9a8a3"
  rule: "#cbc9c3"
  rule-strong: "#8a8882"
  ink: "#1d1e1f"
  legend: "#62615c"
  on: "#2c8a45"
  act: "#a54c08"
  on-act: "#ffffff"
  graph-faint: "rgb(29 30 31 / 30%)"
  hover: "rgb(29 30 31 / 6%)"
  press: "rgb(29 30 31 / 11%)"
  dark-ground: "#202122"
  dark-sidebar: "#1a1b1c"
  dark-list: "#232425"
  dark-raised: "#2b2c2d"
  dark-plate: "#d9d8d3"
  dark-on-plate: "#202122"
  dark-plate-legend: "#56554f"
  dark-rule: "#353637"
  dark-rule-strong: "#6b6c6d"
  dark-ink: "#ecebe7"
  dark-legend: "#9d9c97"
  dark-on: "#3fae5a"
  dark-act: "#f08a24"
  dark-on-act: "#202122"
  dark-graph-faint: "rgb(236 235 231 / 34%)"
  floating-rim: "#cbc9c3"
  dark-floating-rim: "#5a5b5c"
typography:
  display:
    fontFamily: "Albert Sans, sans-serif"
    fontSize: "2.3em"
    fontWeight: 700
    letterSpacing: "-0.02em"
  title:
    fontFamily: "system-ui"
    fontSize: "1.1em"
    fontWeight: 600
  body:
    fontFamily: "system-ui"
    fontSize: "1em"
    fontWeight: 400
  value:
    fontFamily: "system-ui"
    fontSize: "1.1em"
    fontWeight: 600
    fontFeature: "\"tnum\""
  label:
    fontFamily: "system-ui"
    fontSize: "0.846em"
    fontWeight: 500
    letterSpacing: "0.08em"
rounded:
  inner: "4px"
  control: "6px"
  panel: "10px"
  window: "8px"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "6": "24px"
  "8": "32px"
components:
  button-outlined:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
  button-primary:
    backgroundColor: "{colors.plate}"
    textColor: "{colors.on-plate}"
    rounded: "{rounded.control}"
    padding: "0 6px 0 14px"
  edit-count:
    backgroundColor: "{colors.act}"
    textColor: "{colors.on-act}"
    rounded: "{rounded.control}"
    padding: "0 4px"
    height: "18px"
  stage-count:
    backgroundColor: "{colors.act}"
    textColor: "{colors.on-act}"
    rounded: "{rounded.inner}"
    padding: "0 4px"
    height: "16px"
  field:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
  segment:
    backgroundColor: "transparent"
    textColor: "{colors.legend}"
    rounded: "{rounded.inner}"
    padding: "4px 11px"
  segment-checked:
    backgroundColor: "{colors.plate}"
    textColor: "{colors.on-plate}"
    rounded: "{rounded.inner}"
    padding: "4px 11px"
  stage-tab:
    backgroundColor: "transparent"
    textColor: "{colors.legend}"
    rounded: "{rounded.control}"
  stage-tab-selected:
    backgroundColor: "{colors.plate}"
    textColor: "{colors.plate-legend}"
    rounded: "{rounded.control}"
  section-panel:
    backgroundColor: "transparent"
    rounded: "{rounded.panel}"
  list-row-selected:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
  compare-tag:
    backgroundColor: "{colors.plate}"
    textColor: "{colors.on-plate}"
    rounded: "{rounded.control}"
    padding: "2px 9px"
  review-bar:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.ink}"
    height: "48px"
    padding: "0 12px 0 24px"
  tooltip:
    backgroundColor: "{colors.plate}"
    textColor: "{colors.on-plate}"
    rounded: "{rounded.control}"
    padding: "2px 4px"
---

# Design System: Oscyl

## Overview

**Creative North Star: "Ulm Hi-Fi"**

Less, but every widget exact. Oscyl is drawn like 1960s Ulm-school audio equipment: light-grey or anthracite enamel, fine printed legends, 1 px rules, small soft corners, and colour that only ever means a function. The window is three columns of enamel (sidebar, program list, editor) on one ground. The editor has no cards. It is a plain ground divided by rules, with a framed row of six stage tabs, a framed graph and framed section panels of dials with printed scales.

The editor is a compare bench. Every control keeps the saved sound on its face, so an edit reads as a distance: a dial shows a dotted ghost index and an orange arc from saved to current, a segment group notches its saved option, each stage tab counts its edits. Holding Compare glides the whole page, and the synth, back to the saved program; letting go glides forward. A review bar at the editor's foot counts the changes and hinges up into a sheet of them. Plates are lifted enamel parts: a faint top-to-bottom sweep and a soft 1 px drop shadow.

The source of truth for every colour, shape, spacing and motion value is **`design/tokens.json`**. The Linux build (`apps/linux/build.rs`) generates `tokens-light.css` (base), `tokens-dark.css` and `tokens-amoled.css` from it as `--oscyl-<role>` custom properties. The Android app will generate its colour schemes, type and motion from the same file. The frontmatter above mirrors the light and dark columns so tools can read them. If it ever disagrees with `tokens.json`, `tokens.json` wins: change the tokens there and never in a stylesheet. The AMOLED column (true black ground, `#151515` raised, no shadow) lives only in `tokens.json`.

The Linux stylesheet maps the tokens onto libadwaita's named colours (`--accent-bg-color` is the plate, `--window-bg-color` the ground, `--success-color` the green, `--warning-color` the orange, and so on). Every stock widget therefore follows the world before any component CSS is written. The world rejects the category's neon-on-black DAW skin and the generic rounded-card app.

**Key Characteristics:**
- Achromatic enamel. Only two hues exist, and each one is a function.
- Selection is the plate: a solid light-grey (dark theme) or anthracite (light theme) panel with its own text colour.
- Every control is outlined with a 1 px rule and has 6 px corners; panels and frames 10 px; a selected plate sits inset with 4 px corners. Nothing is filled grey just to look like a button.
- The saved value is always on the control: ghost index, notch, count. One held Compare shows it everywhere at once.
- Moving plates and dial discs are lifted enamel; everything else stays flat.
- Small uppercase legends at +0.08em, tabular figures for anything that changes.
- Albert Sans Bold for the program title only. The platform UI font everywhere else.
- Short, firm motion on one easing curve.

## Colors

Grey enamel in three tones plus a plate, and two function lamps.

### Primary
- **Control Plate** (`plate`, with `on-plate` text and `plate-legend` for secondary text on it): the plate is anthracite in light and light grey in dark/AMOLED. It is the one sign of selection and of the primary action. It is used for the chosen segment of a toggle group, the selected stage tab, the selected generator tile, the "Edited only" chip when on, the primary header action (Sync/Save), the "Saved version" tag while comparing, tooltips and text selection. Notices (toasts) are floating `raised` parts, not the plate (see Floating).

### Secondary
- **Lamp Green** (`on`): connected, synced, audio live. It appears as the synth status lamp, the lamp inside the audition button while auditioning, and "saved" text. Nothing else.

### Tertiary
- **Action Orange** (`act`, with `on-act` text): needs action. It appears as the edit dot (6 px) on a list row (after the name, before the picture), a library row, after a selector's legend and a dial's label, in the review bar and on a review row; as a dial's arc from saved to current and its end dot; as a segment group's saved-option notch (12 x 3 px); as the "was N" revert line; as the dashed cutoff marker in an edited filter graph; and as the count box in the primary action and on each stage tab. Nothing else, and never as "danger".

### Neutral
- **Ground** (`ground`): the window, header bars and editor.
- **Sidebar** (`sidebar`): the sources column, one step darker than the ground.
- **List** (`list`): the program-list column.
- **Raised** (`raised`): the selected list row, the selected sidebar row, the dial disc, the review bar and sheet, boxed lists, popovers.
- **Rule** (`rule`): 1 px dividers, the graph frame, section rules, row rules (the program list's rules at 70%).
- **Rule Strong** (`rule-strong`): outlines of buttons, fields, toggle groups and the stage-tab frame and its separators. Also the dial rim and the scrollbar. Under `prefers-contrast: more`, it replaces `rule`.
- **Ink** (`ink`): text, the dial index line, graph lines, the focus ring.
- **Legend** (`legend`): printed legends, slot numbers, captions, dial ticks, graph axis legends, unselected segment and stage text, the off lamp. Under high contrast it becomes `ink`.
- **Graph Faint** (`graph-faint`): the saved shape, drawn dashed behind an edited graph.
- **Hover / Press** (`hover`, `press`): ink at 6% and 11% over any surface. A held Compare button sits in `press`.
- **Shadow** (`shadow`): the dial disc's and the review bar's shadow. Zero in AMOLED.
- **Floating Rim** (`floating-rim`): the 1 px rim of a floating part (the phone's island, rail and sheets; both apps' notices): `rule` on light, 70% of the way to `rule-strong` on dark, `rule-strong` on AMOLED, where it is the lift. The phone's values follow its own `rule-strong` (`mobile.color`).

### Named Rules
**The Function Colour Rule.** Colour only ever means a function. Green is connected, synced or live. Orange is edited or waiting to be written. Everything else is achromatic. A new state that is neither gets a shape, a word or the plate, never a third hue. Every coloured mark is paired with a shape and a word (the accessible description says "edited", the lamp sits beside "Connected").

**The Plate Rule.** Selection and the primary action are the plate, a solid panel in the opposite tone of the ground. Selection is never shown by a coloured fill, a tint or an underline.

**The Two Lamps Rule.** Destructive actions are outlined in ink, not orange or red. Orange means "needs action", and that meaning must stay clean.

## Typography

**Display Font:** Albert Sans Bold 700 (bundled static instance, with sans-serif fallback)
**Body Font:** the platform UI font (Adwaita Sans on GNOME, Roboto on Android)
**Label Font:** the platform UI font, small uppercase

**Character:** a single geometric grotesque title in the voice of an equipment nameplate. Everything you read or operate is in the desktop's own font, which is drawn for small sizes.

### Hierarchy
- **Display** (700, 2.3em, 1.6em in the narrow editor, -0.02em): the program title in the editor. Nothing else.
- **Title** (600, 1.1em): the selected stage heading ("Filter") and stage-tab titles at body size.
- **Value** (600, 1.1em, tabular): a dial's value under the disc.
- **Body** (400–500, 1em): row names (500, 600 when selected), control labels (a dial's or selector's name, sentence case, `legend`), review rows; the review count at 600.
- **Label** (500, 0.846em, +0.08em, uppercase): section legends ("SYNTH", "LIBRARIES", "VCO 1", "MY UNITS"), stage group legends in the review sheet, category run headers in the list. Never a control's own name. Stage summaries and captions use the same 0.846em size in sentence case.
- **Scale numerals** (500, 0.6em, `legend`): the dial's printed 0, 5 and 10.

### Named Rules
**The One Title Rule.** Albert Sans is for the program title only. Running text, buttons, legends and values stay in the platform font.

**The Crisp Text Rule.** On Linux, text is rendered with manual font rendering (`gtk-font-rendering: manual`, so the desktop's own antialias and "slight" hinting apply) and unhinted metrics (`gtk-hint-font-metrics: false`). Automatic rendering drops hinting at fractional scales, which blurs horizontal strokes. Never ship a running-text face that needs hinting off to look right. Drawn widgets place text on whole pixels.

**The Tabular Rule.** Values, slot numbers, counts and anything that updates while you watch use tabular figures.

## Layout

Three columns on one ground, separated by 1 px rules: sidebar 260, program list 320, editor flexible. Each column has its own header bar. Spacing is on a 4 px unit (4, 8, 12, 16, 24, 32). Sizes in drawn widgets and the editor are in em, so zoom scales them. Program-list rows are about 40 px (2.6em) with a 1 px rule between them. Sidebar rows are 2.4em. The editor pads 1.5em 2em 2em (1em in narrow mode). It is one full-width column of title, stage tabs, stage heading, framed graph and section panels; there is no side column. In a section panel the dials lead and the selectors stack in a column beside them; when the panel is too narrow for both, the selectors wrap under. The editor keeps 4.5em of foot padding so the review bar never covers the last panel; the review sheet opens over the page and never reflows it. Breakpoints: stacked under 660sp (tighter panels), narrow under 500sp (phones: 1em padding, 1.6em title, header buttons lose their words, the review bar drops its stage chips).

## Elevation & Depth

Flat ground, lifted parts. Surfaces, grouping and hover are tone (sidebar darker than ground, ground darker than raised) and 1 px rules. A small set of physical parts is lifted, like the knobs and slides of a front panel: the moving plate of a segment group, the stage tabs and the generator tiles; the dial disc; the review bar; popovers. The program-list and sidebar selection plates (`raised`) stay flat.

### Shadow Vocabulary
- **Lifted plate** (outset shadow 0 1px blur 3px in `shadow`, none on AMOLED; fill swept from 6% lighter at the top to 5% darker at the bottom): the sliding plate in segment groups, stage tabs and generator tiles.
- **Dial disc** (outset shadow 0 1.5px blur 4px in `shadow`; fill swept from 7% lighter to 10% darker; 1 px `rule-strong` rim).
- **Review bar** (`box-shadow: 0 -1px 0 var(--oscyl-rule-strong), 0 -8px 28px var(--oscyl-shadow)`): the one upward shadow, for the bar and sheet at the editor's foot.
- **Docked scope** (`box-shadow: 0 1px 0 var(--oscyl-rule-strong), 0 8px 24px var(--oscyl-shadow)`): the strip hanging from the editor's top edge.
- **Popover** (`box-shadow: 0 0 0 1px var(--oscyl-rule), 0 4px 12px var(--oscyl-shadow)`): floats over content.
- **Floating** (`mobile.floating` in `design/tokens.json`; on the desktop the notice (toast) only, `0 0 0 1px floating-rim, 0 1px 3px shadow, 0 10px 28px shadow` with panel corners, its action outlined, standing 12 px above the review bar while the bar shows, as the phone's stands above its island): two layers in `shadow`, a close one (blur 3 dp, 1 dp down) and a soft one (blur 28 dp, 10 dp down), on `raised` with a 1 dp `rule`. On dark the rim sits near `rule-strong` (70% of the way from `rule`): a flat `raised` fill and a crisp outline (a lighter fill erased the `rule` lines inside sheets), so the part reads against a ground of nearly its own tone. AMOLED casts no shadow, so its floating parts carry a 1 dp `rule-strong` rim instead. No inner highlight. Under the bottom island the page fades into `ground` (from 16 dp above it, where the rail and the keys stand clear of it), as Claude's and Telegram's composers do: rows never read through beside the island. Floating parts: the island, the stage rail, sheets, menus and the lifted top bar. While the page scrolls by hand, the island, its fade and the foot's keys dim to 40% (draw only), so dense rows read through them; the fling's end or the next touch brings them back.
- **Section panel sweep** (`linear-gradient(to bottom, ink 3%, transparent 45%)`): a faint enamel sheen on the panel's top, no shadow. The panel's frame is a 1 px `rule` with 10 px corners, and the section's legend (the uppercase label style) is engraved into its top rule: the rule breaks around it (6 px each side), the legend starts 18 px from the start edge, and a long name ends in an ellipsis inside the frame. A section named like its stage has no legend and an unbroken rule. Both apps (desktop: the `sectionframe` widget; phone: `SectionPanel`).

### Named Rules
**The Enamel Rule.** Grounds, rows, fields, buttons and hover are tone and rules; they never lift, glow or carry a shadow. Only the parts above lift, and only by the soft values above. AMOLED shadow is zero; there a floating part's rim is its lift.

## Shapes

Soft precision: `radius` 6 px on controls, rows, count boxes and tooltips; `radius-panel` 10 px on frames (stage tabs, section panels, the stage graph, boxed lists); `radius-inner` 4 px on a group's inset segments, so the chosen plate floats inside its outline with 2 to 3 px of air. 1 px rules on whole pixels. Lamps, edit dots, dial discs and switches are round. The review bar has 14 px top corners and square bottom corners, docked to the editor's foot. A segment group's saved-option notch is a 12 x 3 px bar with 1.5 px ends, 3 px below the frame. (Revised 2026-11-12 from square 2 px corners at the owner's request: square read as dated; the identity stays in the enamel palette, printed dials and function colour.) The window frame (8 px) and floating dialog sheets (6 px) keep libadwaita's platform frame. They are window chrome, not a component radius.

## Components

### Buttons
- **Shape:** 6 px corners, 1 px `rule-strong` outline drawn as an inset ring, no fill. Hover fills with `hover`, press with `press`, both over 90 ms.
- **Primary (plate):** the header's Sync/Save is the plate with `on-plate` text at 600: the memory card (`oscyl-save-symbolic` for a library, `oscyl-sync-symbolic` for a synth's slots, the mobile Save key's drawings), the verb, then the count in an orange box (`act`, `on-act`, 18 px, 6 px corners, 700). When clean, it shows a quiet flat "Synced"/"Saved" with the card carrying its check (`oscyl-saved-symbolic`) in `legend` and no count. A library's Save writes at once and says "Saved 3 programs"; a sync first lists the slots it will change. Hover and press mix 12% and 22% ink into the plate.
- **Audition:** an outlined button: headphones, the word "Live" and a 7 px lamp. Off, the lamp is a 1 px `legend` ring; live, it fills `on` (150 ms) and the button holds `press`. Each change sent to the synth blinks the lamp (opacity 25% to 100%, 240 ms ease-out); a blink is never restarted mid-way, so a stream of changes flickers like a MIDI activity light.
- **Hold to compare:** outlined, the compare glyph (a half-filled circle, `oscyl-compare-symbolic`) and "Hold to compare" at 500; icon only on phones. It acts while pressed; held, it fills with `press`. Holding backslash in the editor (not while typing) does the same. Held and dragged sideways (either way, 160 px for the whole way, the phone's fader distance) it blends the saved program into the edit (the core's `blend`): a 2 px ink line along its foot runs out to the blend; with backslash held, Left and Right step it by tenths. Letting go glides to the edit.
- **Destructive:** outlined in `ink`, never orange.
- **Words:** both apps use the same words for the same action; capitalisation follows the platform (header capitalisation on the desktop, as GNOME asks: "Surprise Me", "Keep in Library"; sentence case on phones: "Surprise me"). Legends and captions are sentence case (or the uppercase label style) on both.
- **Header icon buttons** (undo, redo, history, menu) are flat, with no outline. Undo and redo say the step they take in their tooltips, in the history's words ("Undo: Filter · Cutoff: 431 to 618").
- **History** (the editor header's clock glyph, beside redo, Ctrl+H): a popover, the desktop's form of the phone's history sheet. SNAPSHOTS first, for a program open, read and writable: A, B and C, each a 40 x 32 plate (empty a `rule` outline, stored the lifted plate), its state, and outlined Store and Recall; they live in memory until another program opens, and a recall is one undo step ("Recalled snapshot A"). Then STEPS: every step of the document, newest first, in the core's words beside its slot, down to "As saved"; an ink index line marks where the document stands, steps undone wait in `legend`, and a click goes there (so many undos or redos as one move).

### Inputs / Fields
- **Style:** search and text fields are outlined (1 px `rule-strong`) with 6 px corners and no fill.
- **Focus:** the outline turns `ink`. Keyboard focus everywhere is a visible 2 px ring in `focus` (ink), never coloured.

### Selector revert
A selector's legend is its name in sentence case at body size in `legend` ("Wave", "Drive"), never the uppercase label style (that is for section legends). While the selector (segments, menu) is edited, a 6 px orange dot follows the legend and a "↶ was Saw" line stands at the legend row's end: an undo glyph and the saved value in `act`, 0.85em at 500, crossfading in over 150 ms. Clicking it restores the saved value. The row always keeps the line's and the dot's space, and the control keeps its ticks' and notch's space under it, so editing never moves the layout. On phones at large text the line takes its own row under the legend (always kept). Dials draw the same line themselves. Both apps.

### Chips
- **Dropdown and toggle chips** ("All", "Edited only") are outlined buttons. When checked, a chip becomes the plate.
- **Checkbox chips** (Generate's stages to hold and My units; both apps): several can be on, so each reads as a checkbox: an empty 12 px box (1 px `rule-strong`, 2 px corners) before its word; on, the lifted plate with a 16 px check in the box's place, one width either way.
- **Group legends** (the units page's Shelf and sections; both apps): the uppercase label style, with the group's action at the legend row's end.

### Segments (toggle groups)
- **Style:** one 1 px `rule-strong` frame with 6 px corners and 2 px padding; segments sit inset with 4 px corners and no separators; the chosen one is the plate. 4 px 11 px padding, `legend` text at 500.
- **Layout:** a selector fills its column and its options take equal shares of it (on phones a column at most 360 dp wide; options too long for one row stack, the marks moving inside each option). Option height is the platform's: compact on the desktop, a 48 dp target on phones.
- **State:** the chosen segment is the lifted plate with `on-plate` text at 600; the plate slides between options (`base`). A program value prints a tick under each option, just outside the frame: `legend` 1 x 4 px, the chosen one `ink` 1.5 x 6 px. While another option is chosen, the saved one carries an orange 12 x 3 px notch 3 px under the frame in place of its tick. Settings segments (not a program value) print no marks. Booleans are an **Off / On** segment pair, not a switch, so a parameter reads like every other choice. Stock switches elsewhere stay achromatic (ink when on).

### Navigation
- **Sidebar:** the status row (lamp, "minilogue xd" in 600, state caption, outlined Connect/Disconnect) over a rule. Uppercase legends "Synth" and "Libraries". Rows with stock symbolic glyphs. The selected row is a flat `raised` plate (6 px) that slides from row to row. A library with unsynced edits trails an orange dot. Libraries sit in a tree of folders: a folder row is a `legend` chevron (turning a quarter on the slow curve as it opens) and its name; each level inside is indented 18 px by a 1 px `rule` guide under the chevron above. Dragging a library or folder marks the landing place with a 2 px ink line indented to its level, or outlines the folder (1 px ink, inset) it would go into.
- **Library rows** (mobile): libraries and folders carry the program row's ⋯ at their end (in `legend`, ink when selected), opening their menu; a long press still does. New library and New folder ask for the name at once ("Name the new library", the given name selected, Done).
- **Narrow island** (mobile): the destinations keep their words while all three fit one row beside the search and Save keys; otherwise they are their drawings alone (each said by name), so the island keeps one row and a word never breaks. On the narrowest island the Save key's word goes too, its drawing standing alone.
- **Type** (mobile): running text (`body*`) is untracked; only uppercase legends are tracked out. Back headers keep one line with an ellipsis.
- **Menus** (mobile): the page's own items first, then a rule, then Preferences, Guide, About.
- **No libraries** (mobile): the empty state carries New library and Import file itself; the row of actions shows once there is a library.
- **Guide** (mobile): each chapter is a few steps, each its own paragraph, each starting with the verb; parts are named by what they look like ("the synth button, top right"). The Library's role question shows what each answer does under it.
- **Notices** (mobile): 12 dp above the island on every destination, 8 dp above the page's foot keys, and ending 8 dp before the editor's rail and Compare (starting past the stage strip in the stage view), so the part a hint names stays in sight: the shell owns the foot (`ShellFrame`), and the parts standing there say so while in sight. A new program's and a kept sound's notice carry Open (the program in the editor) before Undo; actions 8 dp apart, Dismiss (48 dp) in the end corner, 16 dp of rim before the words and 8 dp after Dismiss. The words and actions share one line when they fit; otherwise the actions stand under the words at the end, Dismiss staying top end. A notice arrives as the parts at the foot do (from 16 dp lower and clear, over `base`) and leaves over `fast`; a new one waits for the old one to go. It goes by itself like a snackbar, each time stretched by the platform's accessibility timeout: 4 s when it only tells (a failure included), a hint as long as its words take to read (60 ms a character, 4 to 10 s), 10 s with Undo or Open, never for a failed save's Retry, and never for one with an action while a screen reader runs (one that only tells gets 10 s there). The time counts only while the app is in front and no finger rests on the notice. A tap elsewhere on the page puts it away (a scroll or a drag does not), and so does a swipe to either side past 40% of its width.
- **Off controls** (mobile): a control that cannot change (a factory slot, a slot not read) is drawn at 60%, so it never looks turnable; an action that cannot act now (a menu row, an icon button) is drawn in `legend` at 60% (`OscylColors.disabled`), the one off ink.
- **Transient parts** (mobile): every part that comes and goes (a notice, the stepper, a select's options, the ⋮ menu, a foot key, the search filters and Done) arrives over `base` and leaves over `fast` on the one easing, by alpha with a 0.96 scale (from 16 dp lower at the foot), on its layer only; at once with motion off (`parts/Transient.kt`). A part that would move others when it comes keeps its place laid out (Done) or moves them by placement only (Generate stepping aside for Keep).
- **Restoring** (mobile): every "was" restores the same way: a dial's "↶ was N" in its scale's foot (or on its own line at large text) restores on a tap, as the segments' line does; a double tap on the knob still does.
- **Felt** (mobile): an action that took (added, kept, saved, pasted, removed, copied, exported) is a confirm, fired with its notice; Generate's choices and chips, the search key and the Save key tick; opening the stepper is a tick, not a confirm.
- **Counts and digits** (mobile): counts use Latin digits, as slots and values do, in every locale (one digit system on a page); the rail's count box is 16 dp with 11 sp figures in its cell's corner.
- **Settings segments** (mobile): a segment group that is not a program value (Preferences, Live, the picker's tabs, Vary's amount, search filters) prints no ticks and keeps no room for a notch.
- **Program list:** outlined search, then chips, over a rule. Rows show the slot number in `legend` (tabular), the name and an orange dot when edited. Category runs are headed by an uppercase legend aligned with the names. The selected row is a `raised` plate that slides between rows and moves with the list, without gliding, when it scrolls. Where programs can be reordered (one of Oscyl's libraries, or the synth once read; never while searched, filtered or grouped) a `legend` drag handle shows on hover at the row's start. Dragged between rows, the rows part around a gap holding a 2 px ink line; dragged onto a row (its middle half), that row is outlined in ink to swap. The dragged row carries a legend under it: "Move to 003 · 3 slots change". On mobile the program list's bar carries only Back, the title (with its count), the compact chip and ⋮; undo and redo head the ⋮ menu in their step's words ("Undo: Dust Lead removed"). A row held lifts (a confirm; the floating make) and follows the finger; a 2 dp ink line shows where it lands, above or under a row, and letting go moves it there as one undo step (never while filtered, read or on a factory slot). Held and let go without moving, it opens its menu. Search lives in the island: while a program list with programs is in sight the island carries a search key (48 dp, the island's make) at its start. Tapped, the island turns into the field, the whole width, standing on the keyboard: the glyph, the field (focused), a trailing ✕ that clears the text or, with none, closes the search, and while the keyboard is up "Done" before it, which puts it away and keeps the filter (its place is kept when it is hidden, so the field never widens). Over the field while typing, the filters as segments on a floating plate of their own, as tall as the field (64 dp), with no frame inside it (the plate's rim is the one edge, as on the field) and the chosen option's plate concentric with the part's corner (8 dp inside 14 dp, 6 dp in): All · Edited, and User · Factory on a synth's slots. The filters and Done follow where the keyboard is heading, not where it stands, so they leave as it starts to close; the island, the field and the empty foot while another field types crossfade (`base` in, `fast` out). Typing filters by name or slot (case and common accents folded) from the top of the results; the matched letters are underlined in the names; the caption says how many match ("3 of 60 programs"); reordering is off while filtering; "Nothing here matches" offers Clear search; Back closes the search before it leaves; when it closes the list returns where it stood. Another source starts a fresh search; opening a program and coming back keeps it. Beside a side island the list carries the key and the field itself, floating at its foot's end. New program is a floating key at the list's foot end in a library (the + glyph and "New", 56 dp, the island's make; it fades out over `fast` while searching), heads the list's ⋮ menu, and stands in the empty state only where the key does not: it adds after the selected program (else at the end), takes the list to it and says "New program at 013" with Undo. In a list of more than 40 slots, a slot scrubber floats on the end edge while the list scrolls: a 52 dp scale with five slots printed down it and the list's place as an ink line. Held, the list follows the thumb, the slot and name stand in a plate beside it, and every bank of ten ticks; while the list scrolls it stands at 60%, so the rows' ⋯ column reads through it, and whole once held; it fades a moment after the list stops, and faded it takes no touch. When any page is scrolled by hand a long way (900 dp of travel since the scroll began: a long list, a licence, the editor flung far), the island, its fade and the foot's keys ease to 65% (over `slow`, both ways) and back when it stops or is touched; a short page cannot travel that far, so over it they stay whole. With the keyboard up (and no search) the island steps away. Each read program's row also carries its first picture (the first page that has one, as the core draws it; `legend` ink): last in the row after the edit dot (3.6em wide on the desktop, then nothing; 56 x 28 dp on mobile, before the menu's place), so pictures stand in one column (a row without one keeps the place). While a synth is read, a slot to come is its slot and a printed rule where the name will be; read, its name and picture fade in once; the list never scrolls by itself, and the read's tuning scale and Stop (the word, outlined; ✕ only ever closes) stand in the heading's own row. The first time a list can play to a synth (once per install), its first row rests 24 dp open on "Play" until the first touch on the list. A swipe toward the start slides a row up to 96 dp over "Play" and a lamp; past 64 dp a tick, the lamp lights and the word turns ink, and letting go plays the program on its synth without opening it (screen readers: Audition).

### Stage tabs (signature)
Six equal segments inset in one 1 px `rule-strong` frame with 10 px corners and 3 px padding; each tab has 6 px corners and no separators. Each tab holds its title (600, `ink`, never truncated), an orange count box at the top right with that stage's number of edits (16 px, 4 px corners, 0.75em at 700; its space is always kept and it fades in), its own mini line drawing of that stage (wave, mix bars, filter curve, envelope), and a one-line summary at 0.846em. The selected tab is the lifted plate, sliding between tabs: title in `on-plate`, drawing in `on-plate`, summary in `plate-legend`. Each drawing is its own picture. No line runs between tabs. The row behaves as a single-selection tab list (arrow keys, Home/End, Alt+1…6). Narrower than its tabs, it scrolls inside a whole frame; where tabs run past an edge they fade into the ground under an arrow that scrolls to them, and the plate rides the scroll.

### Stage graph (signature)
The selected stage drawn large in a 1 px `rule` frame with 10 px corners: an ink line with a soft 8% fill, frequency/time legends in `legend`, and dashed `rule` grid lines. When edited, the saved shape is drawn dashed in `graph-faint` behind, and the filter cutoff is marked with a dashed orange line. Handles are `ground` discs with an ink ring. During a compare, shapes glide between edited and saved (220 ms); shapes that cannot interpolate (effects, voices) crossfade over the same time. The tab drawings do the same. The heading above it is the stage's title (Title, 600) with its live summary beside it on one line (0.846em, `legend`, tabular, ending in an ellipsis rather than wrapping: "Filter  LP 2-pole · 618 · res 34%"), as on the phone. The enlarge action (⤢, flat) sits inside the frame's top end corner on both apps. The heading's end carries the dice (`oscyl-dice-symbolic`, flat): a click rolls that stage's values only (the core's `roll_page`), as one undo step the history calls "Rolled Filter", with a toast offering Undo; off on a factory slot. A click, not the phone's hold: a pointer does not cross it on its way elsewhere.

### Dial (signature)
A control from a hi-fi front panel.
- **Scale:** 21 printed ticks over 270° (7:30 to 4:30) in `legend`, 1 px, every fifth long; 0 and 10 printed below the end ticks and 5 above the top.
- **Disc:** an enamel disc in `raised`, swept lighter at the top and darker at the bottom, with a 1 px `rule-strong` rim and a soft shadow (see Elevation). Hover and drag darken the disc 18% toward ink and the rim to `legend`, so it reads as something to turn.
- **Index:** an ink line with round caps running in from the rim. It glides on the frame clock (exponential, tau 30 ms; 66 ms while comparing) and the value rolls with it. It snaps when animations are off.
- **Below:** while edited, the "↶ was 431" revert line just under the disc (its room always kept, so the value never moves), then the value (600, 1.1em, tabular), then the label (sentence case, `legend`). Both apps draw this order.
- **Edited:** a dotted ghost index marks the saved value; an orange arc runs on the outer track from it to the current value and ends in a dot; another dot follows the label, as a selector's dot follows its legend. Clicking "was 431" restores it. The accessible description says "edited, was 431".
- **Focus:** a 2 px ink ring, offset 2 px.

### Hold to compare (signature)
While Compare is held, every control, plate, value and graph shows the saved program and glides there slowly (dials at tau 66 ms, graphs 220 ms), and the synth auditions it. Dragged to a blend, they show and play the program between the two, each step drawn in the frame it lands (no glide), the tag still saying "Saved version". Edits are ignored until release; letting go glides forward. A "Saved version" tag (the plate, 600, 6 px corners, 2 px 9 px) fades in beside the title, its space always kept. There is no colour for the moment: the tag and the pressed button say it.

### Review bar (signature)
Replaces any fixed changes column. A `raised` bar docked at the editor's foot (48 px, 14 px top corners, upward shadow) slides up on the first edit: orange dot, "N changes in this program" (600), flat per-stage chips ("Filter 2", `legend`, ink on hover) that show that stage, outlined Revert All, and a flat chevron. The chevron hinges a sheet up over the page (the page never reflows): changes grouped by stage under uppercase legends, in up to three columns, each row (44 px, ruled) with orange dot, "Filter · Cutoff", "431 to 618" and its own revert. Escape folds it from anywhere in the editor. Phones drop the chips. Bar and sheet are laid out once at full height and moved by a transform on a critically damped spring (no bounce, about 250 ms), so they never relay out the page, can be interrupted, and reverse with their velocity; only the visible part takes input.

### Docked scope (signature)
While most of the stage graph has scrolled above the top, a `raised` strip hangs from the editor's top edge (14 px bottom corners, `0 8px 24px` shadow, the page's side margins): the stage name (600), its summary in `legend`, and the same live shape compact, saved shape dashed, gliding with Compare. On the right, the six stages as a selector of their live drawings (`legend`, ink on hover, `on-plate` on the lifted plate), an orange dot on each edited stage; picking one shows that stage and glides back to its graph. Phones drop the selector. It slides down and away on the same spring as the review bar. Clicking it glides the page back to the stage heading (320 ms).

### Demo chooser (desktop)
Explore Demo Library with no synth shown asks which synth's demo, in a dialog of nameplates (the phone picker's Try a demo): two or three to a row, each an outlined 10 px tile with the synth's abstract panel drawn small from the core (`oscyl_app::panel`: its frame, the name display, a knob per field column with its index at noon, in shades of ink), the model at 600 and "KORG · 4 voices" under it in `legend`. With a synth shown its demo opens at once.

### Perform page (desktop)
The editor's title row ends with an outlined "Perform" (the pad glyph, `oscyl-pad-symbolic`, and the word; Ctrl+P). It opens a page in the editor's place (the header says "Perform"; a back chevron beside the program's name, Escape or Ctrl+P return): an XY pad (`XyPad`: a 1 px `rule-strong` frame with 10 px corners, a grid printed at tenths in `rule`, every fifth `rule-strong`; the point a graph handle; while edited the saved point a dotted `legend` ring joined to it by an orange line; held, dashed guides and a plate with both values; the axes' names in the corners they grow toward) with its Across and Up choosers under it, and four large knobs two by two beside it (under it when the pane is narrow), each under its "KNOB n" legend with a chooser below. Choosers are searchable drop-downs over the program's dials named by section ("VCO 1 · Shape"), "None" first; two knobs of one name say whose ("VCO 1 Shape"). The layout is kept per synth model in the settings (`play`); the core proposes the first (`play_defaults`). A drag or an arrow key (Shift: a tenth) moves both fields as one undo step; a double click takes both back to saved. On a factory slot the values show with the controls off and the read-only note. "Play" only ever means hearing a program.

### Motion
`fast` 90 ms for indicators that follow input (hover, press). The dial index follows on the frame clock instead (tau 30 ms, 66 ms while comparing). `base` 150 ms for selection and state (segments, stage tabs, count and tag fades, controls rising 6 px into place on a stage change). `slow` 220 ms for panels and pages: the page stack crossfades and the new page settles 10 px into place; the review bar slides and its sheet hinges; compare graph glides and crossfades. Every selection (segments, stage tabs, generator tiles, sidebar and program list) is one sliding plate, the app's one authored motion, moved by a critically damped spring (no bounce, about 270 ms) that carries its speed into a new target, so a held arrow key flows from row to row. The changes bar and sheet and the docked scope move on the same kind of spring. So do list rows: a reordered row slides past its neighbours (by drag, Alt+Up/Down, Ctrl+Alt+Up/Down), rows part around a drag's gap and glide back when it is cancelled, a new program fades in while the rows below glide down, a removed one's neighbours glide up, and a sidebar folder folds its rows open and shut (the rows below follow, nothing jumps) while rows that change places glide there. A mouse wheel glides the page, lists and chip row (200 ms); touchpads keep GTK's kinetic scrolling. One easing: `cubic-bezier(0.2, 0, 0, 1)`. A setting turns all of it off on top of the system preference.

### Mobile (Compose Multiplatform, as shipped)

The Android and iOS apps (`apps/mobile`) generate their tokens from `design/tokens.json` at build time, as the Linux build does. Three `ColorScheme`s (light, dark, AMOLED) are built from the matching column, and dynamic colour stays off. Material 3 sits underneath only for behaviour (semantics, back, text). Every visible part is Oscyl's own, drawn on Compose foundation.

| M3 role | Token (each theme's column) | Why |
|---|---|---|
| primary / onPrimary | plate / on-plate | The filled primary action (Sync/Save) is the plate. |
| primaryContainer / onPrimaryContainer | plate / on-plate | Selection: checked segments, the selected stage tab, a checked filter chip. |
| secondaryContainer / onSecondaryContainer | raised / ink | The selected list row and the navigation indicator. |
| secondary, tertiary, tertiaryContainer (and on-) | ink, ink, raised (on: ground, ground, ink) | Kept achromatic so no stock component invents a hue. |
| background, surface | ground | Window and editor. |
| surfaceContainerLow | sidebar | Navigation drawer or rail. |
| surfaceContainer, surfaceVariant | list | Program list pane. |
| surfaceContainerHigh, surfaceContainerHighest, surfaceContainerLowest | raised | Menus, sheets, the dial disc. |
| onSurface, onBackground | ink | Text, index lines, graph lines. |
| onSurfaceVariant | legend | Legends, captions, ticks. |
| outline | rule-strong | Outlined buttons, fields, segment and tab frames. |
| outlineVariant | rule | Dividers, graph frame. |
| inverseSurface / inverseOnSurface | plate / on-plate | Tooltips are the plate. Notices are floating `raised` parts on both apps. |
| error / onError | ink / ground | Errors are said in words and outlined in ink. Orange is not borrowed for errors. |
| scrim | shadow | AMOLED: none. |
| extended `act` / `onAct` | act / on-act | The single attention colour: edit dots, the dial's saved-to-current arc, the segment notch, "was N" lines, the edited cutoff marker, the stage counts and the Sync count. Only those. |
| extended `on` | on | Success, connected, audio live: the status lamp and the audition lamp. |
| extended `graphFaint`, `hover`, `press` | graph-faint, hover, press | Drawn widgets and state layers. |

On the phone `rule-strong` is a step stronger than the shared value (light #7d7b75, dark #77787a, AMOLED #6c6c6c; `mobile.color` in the tokens, the Linux app keeps the shared ones), so outlines reach 3:1 on the surfaces they stand on in daylight. Expose the extended colours through a `LocalOscylColors` composition local. Override M3 state layers with `hover`/`press` so pressed states stay achromatic.

- **Shape:** `extraSmall` = `RoundedCornerShape(4.dp)` (inset plates), `small` and `medium` = 6 dp (controls, chips, fields), `large` = 10 dp (panels, frames), `extraLarge` = platform default for dialogs and sheets. Lamps and edit dots are `CircleShape` 6 dp. Rules are 1 dp.
- **Type:** the program title uses one style (`headlineLarge`, or `displaySmall` on tablets) in Albert Sans Bold (700, -0.02em). Every other style keeps `FontFamily.Default` (Roboto). Legends use `labelMedium` at 500, `letterSpacing = 0.08.em`, uppercase. Values and counts use `fontFeatureSettings = "tnum"`.
- **Motion:** `tween(90 / 150 / 220, easing = CubicBezierEasing(0.2f, 0f, 0f, 1f))` for fast/base/slow. The knob's index is where the value is, at once, for a step: under the finger, tracking the synth's own knob however fast it turns. A jump past `mobile.knob.glide_from` (12 %) of the range (a restore, another program) glides through a critically damped spring (`spring_stiffness`, ω² = 1100/s², about 30 ms), and compare swings it at any size (230/s², about 66 ms): weight, with no overshoot. Honour the system animator scale and the app's own off switch.
- **Knob** (phones; `mobile.knob`): a 64 dp disc in a 96 dp target, the printed scale just inside it and no 0/5/10 numerals (the value under it says it). A touch on the disc or its ring (76 dp) grabs: until the finger lifts the knob owns the gesture and the page holds still; the value and label below do not grab, so the page scrolls between knobs. Up turns it up; 220 dp of drag runs the range, and a finger slower than 0.15 dp/ms turns at 0.35×. A double tap restores the saved value. While edited, "was N" follows the value inline in orange, so panels keep no empty line for it (the "keep the line's space" rule holds from medium width up, where the dial and its "was" line stay as on the desktop). An ink arc runs from zero to the value at the orange arc's radius (the orange arc from saved to the value stays over it). A long press on the value (or the screen reader's "Set exactly") opens the stepper under it: a floating plate of minus, the value and plus; a press steps one, a hold repeats after the long-press timeout from 8 to 40 steps a second over 1.5 s, stopping at the ends; each press or hold is one undo step. Where the core can read typed text the stepper's value is a field: Done sets it, and text no value is written as says "No such value" under it.
- **Components (own, not stock M3):**
  - The plate action (Save, Sync, Generate, Get started) is a lifted plate with the 6 dp orange count box.
  - Outlined actions are 1 dp `rule-strong` frames with 6 dp corners.
  - Off/On and every other choice is a plate segment group with the saved-option notch.
  - The stage rail, dials and graphs are drawn on `Canvas` to the specs above.
  - Lifted plates use a 1 dp shadow and a vertical gradient, never M3 tonal elevation.
  - There are no `Card`s, no pills and no `Badge`.
- **Floating parts** (the floating shadow above):
  - **Island:** on phones, a floating island at the foot carries Library, Editor and Generate on a sliding plate (14 dp outer and 10 dp inner corners); 8 dp off its end, the Save key: a floating part as tall as the island, its end on the 12 dp margin under the rail, in the island's make and a tab's anatomy (a drawing over its word). It counts and keeps one source: in the Library the list's, elsewhere the editor's program's (the list is taken there first). A tap on it keeps a library's edits at once (the notice says "Saved 2 programs"; a failure opens the review with its reason); a hold, and any tap for a synth's slots, opens the review first. Its drawing is a memory card, the synth's own storage (`ic_save`: the card with its label and contacts; `ic_sync`: the card with an arrow, for a synth's slots; `ic_saved`: the card with a check on it). While there is something to write: the card and the verb ("Save", "Sync"; "Salva", "Invia") in `ink`, the orange count on the drawing's end corner as the rail's stages carry theirs (16 dp tall, at least 16 dp wide, 11 sp tabular figures, "99+" above 99; it widens toward the start, over the drawing, and stays inside the key; mirrored right to left). Clean: the card with its check and "Saved" in `legend`, disabled; in `ink` while the "Saved" notice stands. The key carries no lifted plate: the plate is the selection's, and the count is the one bright mark at the foot. Both faces are laid out, one over the other, and all four words (Save, Sync, Saved, Synced) hold its width unseen, so the key never changes size and the tabs never move, between states or between a library and a synth's slots. While a save or a sync runs, the printed scale (`ProgressScale`) runs along the key's foot and the face waits at 55%. Its words are short ones of their own ("Saved", "Synced"; "Salvato", "Inviato"; a screen reader hears the state in full). The key keeps its word while the tabs have room for theirs beside it (large text included); on the narrowest island the word goes and the drawing stands alone, as the tabs' do. The chrome's text grows with the system font size up to 130% and no further (`ChromeScale`: the island, side island and search field, the foot keys, the stage rail, the stage view's and the graph page's headers, the top bars and Back headers, a picture's axis words): past that the island's tabs keep their drawings alone, titles cut to a few letters and axis words leave the picture; a screen reader still says every label, and the content grows in full. The faces crossfade on `base`, the count pops on `fast`. There is no Synth destination on any size: the synths live behind the chip.
  - **Side island:** in medium and expanded windows, the island stands on end at the start edge, as wide as its widest label, the Save key under it at the same width, built as the tabs above it. The content beside it does not pad again for the insets the island takes (the end cutout and bar on a phone on its side), and floating panels (sheets, menus) centre in the room beside it.
  - **Stage rail:** an index of the stages, answered on touch-down. The moment a finger lands: a tick, the row dims (`press`), the lifted plate comes to it and a **flag** beside the rail names the stage (its name and summary on a small floating part with 10 dp corners, 8 dp from the rail; in over `fast` with an 8 dp slide from the rail, out over `base`; no ripple). Lifted there it is a tap: the page glides to the stage and the flag stays until it lands. Moved along the rail it is a glide, with no long press to wait for: the rail opens toward the panel into the labelled list on the plate spring, the plate rides the finger with no spring between them, each stage passed ticks and the page follows; on release the plate settles on the plate spring and the rail closes. On a phone the drawings stay at the end edge under the thumb, the words open toward the start, the orange count stays on the drawing, and rows keep 48 dp open or closed, so nothing moves under a finger. The rail's own size is one of two (the rest column, or the whole list while it rests open), never between: what opens is drawn, clipped and translated. Each row is drawn in the rail's inks and again in the plate's, clipped to where the plate stands, so a riding plate never leaves light words on it. On the first look at the editor (once per install, kept in the settings' `taught` hints) it rests open with every stage's name; a touch on it keeps it open and it closes once it has taken the page somewhere, or on the first touch on the page itself. In the stage view it stands at the start edge and opens toward the end. In a wide window it stands open for good, its rows 56 dp, reading drawing, words, count. It floats at the editor's end edge, anchored low: its foot stands 16 dp (`mobile.rail_anchor_dp`) above the island, in thumb reach, with Hold to compare under it as a floating 48 dp part in the rail's own make: the glyph alone, on the rail's lifted plate while held (the top row's Compare is gone). Panels end at its edge. In a wide window the panels keep a reading width (840 dp at most), centred in the room beside it. The slot legend keeps one line in every language (cut short beside a long Perform), so the title block keeps one height.
  - **Review sheet:** the one docked sheet (see "Synth sheet"), from the Save key: the edited programs with their changes and reverts, the action at its foot. Asked for from the synth sheet, that one lands first and this one rises after: never two veils.
  - **Dialogs** (program menu, names, confirmations) are one floating panel at a reading width, with Cancel then the action as outlined buttons. They arrive as a destination does (from 0.96 scale and clear over `base`, on their layer) and, put away by Back, a tap outside or Cancel, leave the same way over `fast`. Menu rows share one metric and one make: 48 dp, 16 dp from the panel's edge, a 20 dp glyph, then `bodyLarge` at 500; every action of the program, library and ⋮ menus has its authored glyph. Their outlined buttons stand on the 16 dp the title and words do. A dialog's field rises with the keyboard by placement (the panel never resizes per frame). The ⋮ menu is 200 to 320 dp wide; a long row wraps to two lines.
  - **Clearance:** a page's scrolling content keeps clear of the island (`LocalIslandClearance`). No page scrolls under the status bar: the insets pad the viewport, not the content.
  - **Insets:** every part pads by the safe drawing insets (system bars, the display cutout on any edge, the IME), read through one `safeInsets` so previews and tests can stand a fake cutout in (`CutoutFrame`: punch-hole, corner, tall notch, double, and both landscape rotations).
- **Top bar** (every destination; collapsing on the Library, the program list and Generate, the Editor next):
  - **Leading:** navigation only: Back where the page has a parent, else nothing.
  - **Trailing:** the page's own actions (undo and redo on the program list and the Editor), the **synth chip**, then the menu (⋮: Preferences, Guide, About Oscyl, each a glyph and its word, on a floating panel that grows from its corner: scale 0.9 to 1 and alpha on its layer, `base` in, `fast` out). The chip is one part in two sizes. **Full:** a 1 dp `rule-strong` frame, 34 dp tall: the lamp (lit while a synth is connected), the synth's model, "+1" for each other synth and "Live" in `legend` while edits play; a read draws a slim line along its foot; with no synth, the words "No synth" in `legend` and no lamp. **Compact** (where the bar carries more actions, and at large text): no frame, the synth drawn small in the line hand (its body and keys, 24 dp), the lamp standing in the body's open top end corner as a panel's own lamp, "+1" beside it and a read's line under it; with no synth the body is whole, in `legend`, with no lamp. The lamp blinks per change sent and breathes during a read at both sizes; its words stay spoken.
  - **Empty bar:** the sources pane beside the programs (whose bar carries the chip and ⋮) has nothing for a bar to hold: its bar keeps only its title's room, no empty 56 dp row.
  - **At rest** the bar is the page: no fill, and the page's large title (`headlineSmall`, semibold) waits in the content. **On scroll** the title rises into the bar (`titleMedium`), with the subtitle (the source, the synth), and the bar lifts: `raised`, the floating shadow, a 1 dp rule at its foot (`rule-strong` on AMOLED). It is fully scroll-linked with no snap, read in draw and layout only. The bar's title is the page's one heading, wherever it shows.
- **Editor top bar:** the slot legend and Albert Sans title rise into the bar as they leave, with the stage in view as its subtitle; undo, redo, the chip (its lamp only) and the menu trailing. Live is on the synth's card in the sheet.
- **Compare and morph:** the Compare part at the rail's foot carries the compare mark (the saved sound, a dotted ring, half behind the sound now, a solid ring) and three printed ridges on the side it slides toward; it is off (`legend`, half strength) when there is nothing to compare. Held, it takes the `press` fill and the fader comes out beside it, in its own fixed place (nothing moves): a floating plate, 48 dp tall, "Edited", the tuning scale from the part outward with its index at the blend, "Saved" by the part, and "Slide to blend" over the scale until the slide starts. Sliding 160 dp away from the edge blends saved and edited (every value and picture follows, the synth plays the blend), a tick each tenth, a detent at both ends; letting go glides to the edit. The bar's subtitle says "Saved version" while comparing. The first edit ever says how the part works above the island ("Hold Compare, under the stages, to hear the saved sound…", once per install); a tap too short to hear anything says it again. The part is kept off the system's Back swipe. In the landscape stage view it stands at the start edge and slides toward the end. Each stage heading ends with a dice glyph that rolls that stage's values only on a hold, as one undo step ("Rolled Filter"); a tap says "Hold the dice to roll Filter." (a stray tap in the thumb's path never rewrites a stage); none on a factory slot or a slot not read yet. While comparing, "SAVED VERSION" in ink takes the slot legend's place at the top of the page (crossfaded, nothing moves).
- **Perform page** (the pad glyph ▣ and the word "Perform" in the editor's title row and ⋮; "Play" is only ever hearing a program): a page above the editor, as Preferences is, under the same Back header with the chip and ⋮. A large XY pad (`XyPad`: a ruled frame, a grid printed at tenths, every fifth `rule-strong`; the point a graph handle's ring; the saved point a dotted ghost joined by an orange line while edited; dashed guides and a value plate while held; a tick per tenth, a bump at an edge) on two parameters, then four macro knobs, each with its "Knob n" as an outlined action under it, a chevron after the word as a select field has (it opens the choice of what the knob turns; said as a drop-down); two knobs of one name say whose ("VCO 1 Shape"). Portrait: the pad at the page's width (at most 60% of its height), the knobs in a row (two by two on a phone's width, under 400 dp, and at large text); landscape or wide: the pad at the start, the knobs two by two at the end. The axes' buttons and the knobs' legends open the docked sheet of the program's dials by stage, "None" first, the current one on the plate. The layout is kept per synth model; the core proposes the first one (the pad on the first two-axis handle, the large dials first). A field the program does not show is unassigned there and kept in the layout. The pad moves both fields as one undo step, and a double tap on it takes both back to saved (one step, as a knob's double tap); a factory slot shows the values with the controls off and the read-only note.
- **Full-screen graph:** a tap on a stage's picture, or its ⤢ corner, grows it to the whole page by container transform (shared bounds; Back, partway or whole, drives it home, and the list scrolls the picture back into place first). A header with the close chevron, the stage, the program and its place ("3 of 4"), and Compare (its glyph and the word; the hint teaches the hold); the picture fills the page with 64 dp handles and the value on a plate beside the finger; a pager across the stages that have a picture; the stage's controls on a floating sheet docked at the foot that peeks a row. The sheet rises to its content, never past it: a drag (or a tap on its grip) raises it until its last row shows, at most to most of the page, where it scrolls. A stage whose controls all show in the peek (less than a control's 48 dp below it) has nowhere to rise: no grip is drawn, a drag moves nothing, a screen reader hears no action, and its knobs stand where every other stage's do. A swipe down on the header closes it.
- **Stage view** (a phone on its side, height under 480 dp): one stage at a time. The strip at the start edge with Hold to compare at its foot; the stage's title, program and chip above its picture, large; its controls in a scrolling column beside it; the island on end at the end edge. Every edge part pads for a cutout on its side, in both rotations.
- **Synth sheet** (from the chip), the one docked sheet (`DockedSheet`, shared by the synths, the review and the edit history; title, Close and grip alike): a floating sheet that rises from the foot over a veiled page, island and all, edge to edge: the full width on a phone (640 dp at most, centred, in a wide window, and in the room beside a side island, never over it), its foot under the navigation bar and its content scrolling under it too (a scroll pads its own end by `LocalSheetFoot`), 14 dp top corners and square lower ones, its rim only along its head (no frame along the display's edges), a grip (drawn in 28 dp, a 48 dp target that reaches under the title row) as its one Close (no ✕), as tall as its content up to 92% of the window. It moves by placement only on `slow`; a drag down, a flick, a tap on the veil or the grip, Close or Back put it away, and predictive Back sinks it with the finger. Its list scrolls only once the sheet is fully up. It is composed a moment after the app opens and kept, silent, below the foot. Inside, a rack of nameplate cards on `ground` in a 1 dp rule, one per synth, real or demo: the panel drawn small from `PanelDrawing`, the model, the lamp and its line ("Connected, firmware 2.10", "Simulated, no sound" with a "Demo" legend), a read's progress (a slim line and its words), Live (Off/Live, with "Live: every edit plays on the synth as you make it." under it; a screen reader hears a read at its start, each quarter and its end) on the card of the synth that plays the open program, and its jobs as tiles (a glyph over a word, three to a row, two at large text): Read all again or Stop, Save playing sound, User units · n, Review edits with the orange count, Backups · n, and last Disconnect (its glyph and word, as every job); a synth that went is a card with Connect; a port awaiting access is a row with the panel at rest drawn small (which synth it is is not known yet), its unlit lamp and Allow. Units and Backups are sub-pages on the shared axis X. A dashed plate with a + at the end adds a synth (the picker). The chip's lamp blinks per change sent.
- **Synth picker:** a full page with a Back header, a search over models and makers (its glyph leading, as the program search's), and Plugged in (ports to allow, synths to reconnect, else "Waiting for USB") or Try a demo: nameplates, two columns on a phone, each a lifted tile with an abstract panel drawn from the driver's own pages and fields (knobs per field column, the name display), the model, the maker and its voices, all from the core's registry (the driver's tier is not the player's business). The last tile asks for a driver (the new-synth request form): a dashed plate like the sheet's Add a synth, its + over its words, centred.
- **First run:** one screen. The oscillator line draws itself, "Oscyl" in Albert Sans, one line of purpose, a listening card ("Plug in your synth over USB", its lamp's ring pulsing) that lifts onto the floating shadow with its lamp lit when a real synth answers and offers Read its programs; "No synth here? Try one" (outlined) opens the picker (a demo and its library start the Library); "Continue without a synth" on the plate, the page's one way on without a synth (as Read its programs is with one); on a phone on its side the name and purpose stand at the start and the card and the ways on beside them, so an action is in sight; a privacy line. It shows once and starts in the Library; the role question left it (Preferences keeps the setting, and a saved one is still honoured).
- **Empty states:** one designed state (`EmptyState`) everywhere a list or page can be empty: a line drawing in the panel's hand (the panel at rest, a plug before its socket, a shelf with a dotted gap; `rule-strong` outlines, `legend` parts, no colour), the title (`titleMedium`), one line of guidance (`bodyLarge`) and the next step as one outlined action. A source still loading shows skeleton rows (rule-coloured bars at the rows' own height), never a blank page.
- **Pages above a destination:** shared axis X: the page leaves 30 dp toward the start fading over `fast`, the new one arrives from 30 dp toward the end fading in over `base` after it (mirrored right to left); predictive Back drives the pop. About opens only once its notices are read (off the main thread), so it is whole on its first frame.
- **Felt feedback:** a tick as the island's plate moves and on a chip tap; the chip's lamp breathes (full to 45% on `slow` × 4) while a read runs; a long press on the chip adds a synth; a flick down on the sheet puts it away.
- **Drawing for speed:** a knob's printed scale and disc, and a stage picture's saved shape and curve, sit on cached layers (`CompositingStrategy.Offscreen`), so a turn or a scroll redraws only what moves; shared-element modifiers apply only while a transition runs.
- **Destination switch:** fade-through. The old destination fades out over `fast`, then the new one fades in over `base` from 0.96 scale, on the ground: no frame mixes two pages, and none is blank. The motion starts once the arriving page's first frame is out (two frame callbacks), so a first visit plays whole instead of jumping past the frames its composition took. Destinations stay composed once visited, their scroll and panes kept; the hidden ones are silent and take no touch. Back from the Editor or Generate returns to the Library.
- **Pages:**
  - **Pages above a destination** (Preferences, Guide, About, a licence, the synth picker) open with one Back header, in step with the top bars: a 56 dp row, the back arrow 4 dp from the edge, then the page title as its heading in the page titles' one size (`headlineSmall`, semibold, the size of a destination's large title). The header brings its own inset; content stands on the 16 dp page margin. Where the header stays while the page scrolls under it (Perform; the picker's header, search and tabs over its grid), it lifts as a destination's bar does: `raised`, the floating shadow, a 1 dp rule at its foot.
  - **Generate:** a row of families over the character choices (All, Keys, Bass, Lead, Pad, Rhythm, Effect, only those the synth has: words alone in `legend`, the chosen one in `ink` at 600 over a 2 dp ink index line, scrolling sideways when it must; the page opens on the first character's family and the choice holds for the session; both apps), the character choices of that family, four shape dials (two by two on phones, with no "was" line, because a shape has no saved value), the stages to hold ("Hold from slot 004"; several can be held, so each reads as a checkbox: an empty box before its word, a check on the plate when held, one width either way), and the set of six tiles (two columns, three from 600 dp). Its actions are foot keys: Generate (on the plate) floats at the foot's end above the island, the page scrolling under it; once a set is dealt, Keep (the page's point) arrives at the end and takes the plate, and Generate steps aside for it by placement and floats beside it (its plate crossfading on `base`). **Foot keys** (`parts/FootKey.kt`) are every page's own actions: 56 dp tall with the island's corner, an icon and a word, end-aligned 12 dp above the island, the page's point on the plate (like Save), others floating like the island. No page docks a solid bar of its own (the Library's New is one too). Keep with a slot open asks, in a menu rising from the foot, "In slot 004, replacing it" or "In a library"; with none, it keeps in a library at once. With no synth the intro says to connect one to hear each sound. Each sound tile (both apps) is one picture, drawn from the page whose pictures differ most across the hand (the core's `telling_page`), then the name at 600, then a sentence-case caption in `legend` (`plate-legend` when picked): category, the units it uses and where it was kept ("Lead · with waves · kept in 004"). An oscillator tile is its wave, its name, then its family and its parameters' names. On the desktop the tiles sit inset in one framed grid (10 px frame, 3 px padding) and the picked one is the lifted plate sliding between them, like the stage tabs; on phones each tile is its own lifted `raised` part and the picked one fills with the plate. The desktop's character choices stay framed tiles with a typical result's picture and the character's hint (it has the room); the phone's are outlined choices with the hint as their state. When the synth holds user units, a "My units" row of checkbox chips (as the stages to hold: on unless left out) follows the dials' actions, and a tile names the unit its sound uses in its caption ("Lead · with waves"). The synth sheet's User units page opens with the Shelf: its legend with Add… at the row's end, then each kept unit (name over its section) with Load and a bin; a unit that declares parameters shows them under its name on a tap, each with its range. Under the dials, two outlined actions (beside the dials on the desktop, under Generate): Surprise me with the dice glyph (the core picks a character and dials; the dials glide there, the choice moves, a set is dealt) and Enter a code… (a dialog with one field; a bad code keeps the field, its ring turned 2 dp `ink`, with the core's reason in a line under it that is always kept). With a sound picked: Vary, Breed with… and Morph to… (the last two ask which other sound in one row of names; Morph deals six steps between the two, the legend "From “A” to “B”"), and the sound's recipe code small in monospace with Share (the system's sheet; Copy on the desktop), absent for a sound that has none.
- **Reading progress:** one part, the tuning scale (`ProgressScale`): printed `rule-strong` ticks along a 6 dp strip (every fifth taller), an ink line run out to the value and an index at its end that glides to each new value on `base`. On the synth's card, the Library's synth row and the review sheet's write; its strip is always kept, and the read's words take the connection line's place, so a read starting or ending moves nothing. The chip keeps its slim line.
- **Press:** `press` is there the moment the finger lands and lets go over `base` (at once with motion off): the quickest tap still shows.
- **Rows that select** (programs, libraries) keep one weight and keep the menu's 48 dp place, so selecting moves neither a name's wrap nor the edit dots' column. Names keep their own direction but align to the layout's start.
- **Edit history:** two runs under their legends, SNAPSHOTS (A, B, C) then STEPS; the one docked sheet, from a long press on undo and from the editor's menu: every step of the document shown, newest first, in the core's words ("Filter · Cutoff: 431 to 618", "Dust Lead removed") beside its program's slot, down to "As saved". The step the document stands at carries an ink index line at its start; steps undone wait above it in `legend`. A tap goes there: so many undos or redos as one move. Above the steps, for a program open, read and writable, three snapshot plates A, B, C (40 x 32 dp in 48 dp targets) keep whole states in memory until another program opens: a hold stores, a tap recalls (one undo step, "Recalled snapshot A"); empty is a `rule` outline, stored a lifted plate.
- **Line drawings** (`LineArt`): one hand for the empty states and the Guide's chapters (panel, cable, shelf, knob, trace, tiles, sync, unit): `rule-strong` outlines, `legend` parts, dotted `rule` for what is missing, 120 by 64 dp, no colour.
- **Handle drag:** the dot, the curve and the value tag move in the same frame as the finger, from the first step to the last; the curve draws the value's own picture while dragged and never glides after a release. A knob being turned and the Compare fader being slid do the same (`PanelState.following`): their pictures are drawn in the frame they land, so the line never trails its own handles; a choice, a revert, undo and a Compare tap still glide. The stages the full-screen pager shows count as on screen with the list's.
- **First-run hints:** two forms only, and never an overlay, a scrim, a badge or a dot. *The part shows itself*: idle motion in place, once, until the first touch on its surface (the first program row opens on "Play"; the open rail's plate nods to the next stage and back; the search key's glass nods beside the first long list). *A line replies*: a notice at the foot, only in answer to something the person just did (a tap on Compare or on a dice says to hold; the first plain tap on the rail says "Slide along the stages to move through them."; the first Save ends "Hold Save to see the changes first."; the first menu a hold opens on a row that could move says "Keep holding and move to reorder." under its heading). Each is kept once per install in the settings' `taught` set (`Teaching`, `Hints`), and a hint about a gesture is retired by doing the gesture, shown or not. With animations off the first form does not play; the lines still do.
- **Lamps:** the lamp's fill and ring fade together on the `base` tween, at once with the app's animations off.
- **Haptics:** a tick on each printed dial step and each stage passing under the finger; a detent at the saved value and a knob's centre; a bump at a knob's end stop; never more than one per frame (the firmest wins); a confirmation when an action takes. A Preferences switch turns them off, and the system's setting also applies.
## Do's and Don'ts

### Do:
- **Do** change colour, shape, spacing and motion in `design/tokens.json` only, and let the build generate `--oscyl-*` (and later the Compose theme).
- **Do** map tokens onto libadwaita's named colours first, so stock widgets follow before any component CSS.
- **Do** show selection and the primary action as the plate.
- **Do** outline buttons, fields, chips and segment groups with a 1 px `rule-strong` inset ring and 6 px corners (10 px for panels).
- **Do** keep the saved value on every control while edited (dial ghost and arc, segment notch, "was N" line, stage count), and reserve its space so editing never moves the layout.
- **Do** mark an edit with the 6 px orange dot and say "edited" in the accessible description. Put edits into words only in the "was N" lines and the review sheet.
- **Do** let anything that shows a program value glide or crossfade with Compare.
- **Do** put the verb first in the primary action and the count in the orange box. When clean, show "Synced"/"Saved" with no count.
- **Do** use Off/On segments for boolean parameters.
- **Do** keep text crisp: manual font rendering on Linux, and the platform UI font for running text.
- **Do** use tabular figures for values, slots and counts, and em sizes in drawn widgets.
- **Do** check new work in light, dark, AMOLED and high contrast.

### Don't:
- **Don't** introduce a third hue, or use green or orange as decoration or for any other meaning.
- **Don't** use orange or red for destructive actions or errors. Outline in ink and say it in words.
- **Don't** fill a standalone button grey, or show selection with a tint, underline or colour.
- **Don't** put cards in the editor or stack a card on a card. Sections are framed panels and rules; the review bar is the editor's one raised part.
- **Don't** bring back a fixed changes column; changes live in the review bar and sheet.
- **Don't** use Albert Sans for anything but the program title.
- **Don't** add shadows, glows or lifts beyond the Shadow Vocabulary: lifted plates, the dial disc, the review bar and popovers. List and sidebar selection stay flat.
- **Don't** make pills of buttons or segment groups (switches and lamps are the only round shapes besides dials).
- **Don't** join the stage tabs' drawings into one running line. Each tab owns its picture.
- **Don't** use neon-on-black DAW styling, KORG logos or trade dress, or the official Librarian's look.
