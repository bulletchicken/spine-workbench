# look-and-feel

What everything in the app is drawn with: the tokens, the base styles,
the type, and every sign the app draws. How things move is next door, in
`choreography/`. The design these follow is in `docs/design/`.

| File | Holds |
|---|---|
| `base.css` | the app's one stylesheet: it imports the files below and the four shared keyframes, in order, then sets the reset, the body and the focus ring |
| `tokens.css` | the design's tokens, unchanged; never edited here |
| `named-timings.css` | the durations and gaps the design names that are not tokens |
| `fonts.css` | Instrument Serif, Geist and Geist Mono, from the installed font packages |
| `confidence.css` | the one place `data-conf` becomes colour and line |
| `data-attributes.ts` | the value sets of the data attributes that carry state to CSS |
| `ConfidenceGlyph.tsx` | the five confidence glyphs |
| `StateIcon.tsx` | the sign for the state of a row or a chip |
| `icons/` | `Icon`, the names it takes, and the eight icons drawn for this app |
| `figures/` | the four view figures, and `figure-for-view.ts`, which finds one by film kind |
| `BrandMark.tsx` | the app's mark |

## Colour and type

Light means go, colour means data. Lumen (`--lumen-*`) is only for the
next action. The only hues are the confidence colours, and each comes
with a line pattern, a glyph and a word.

| Family | Token | For |
|---|---|---|
| Instrument Serif | `--font-voice` | the guide's one sentence, sheet and popover titles, the L and R letters |
| Geist | `--font-ui` | everything read as a sentence or pressed; the body's own font |
| Geist Mono | `--font-data` | anything measured, counted or named as a level |

Geist Mono's figures are one width and its zero is slashed as drawn.
Geist's figures are proportional: a column of numbers set in Geist needs
`font-variant-numeric: tabular-nums`.

Every face is `font-display: block`: text is first seen in its own face,
never in a fallback that is then swapped away.

### Keys

The shipped faces have `↑` and `↓`. They have no `←` or `→`, and none of
the signs printed on a keyboard's held keys (`⌘`, `⇧`, `⌥`) or on Return.
A character a face lacks is drawn by whatever the system has, in another
size and weight, and differently on each system. So a key hint never
writes one of those as a character:

- an arrow key is an icon, all four of them, so they are one family:
  `arrow-left`, `arrow-right`, `arrow-up`, `arrow-down`;
- Command, Shift and Option on an Apple keyboard are the icons
  `command`, `shift` and `option`;
- every other key is its letter or its word in the data face: `Z`,
  `Enter`, `Esc`, `Ctrl`, `Shift`, `Alt`.

An icon in a hint is as tall as the letters beside it, with a line that
weighs what theirs does: set `--icon-size` to the hint's font size and
`--icon-stroke` to `2px`.

## Confidence

Put `data-conf` on a vertebra, a rung or a chip, and read the
properties `confidence.css` sets. Everything that differs from one state
to the next is one of them, so no part writes a `[data-conf]` rule of
its own:

```css
.outline-casing {
  stroke: var(--casing);
  opacity: var(--conf-casing-fade);
}
.outline {
  fill: var(--conf-wash);
  stroke: var(--c);
  stroke-width: var(--conf-line);
  stroke-dasharray: var(--conf-dash);
  stroke-linecap: var(--conf-cap);
  opacity: var(--conf-fade);
}

.rung {
  opacity: var(--conf-rung-fade);
}
.rung-bar {
  width: var(--conf-bar-w);
  background: var(--conf-bar);
}
.rung-frame {
  border: var(--conf-frame-w) var(--conf-edge) var(--c);
  background: var(--conf-wash);
}

.legend-chip-line {
  border-top: var(--conf-edge-w) var(--conf-edge) var(--c);
}
```

What a part adds for itself is what does not depend on the state: the
5 px casing under an outline, a rung's size, how a selected rung shows.

`<ConfidenceGlyph conf="look" />` is the glyph as a 12 px picture.
Inside another SVG, draw `<ConfidenceGlyphShapes conf="look" cased />` in
a group that places it: the shapes are 12 units across, centred on the
origin, and take their colour from the nearest `data-conf`. `cased` lays
the dark casing under a glyph drawn on a film: a line round a solid
glyph, a whole disc under one with a hollow, so no bone shows through.

## Icons and signs

```tsx
<Icon name="boundary-wand" />     // 20 px, a 1.6 px line, the colour of the text around it
<StateIcon state="warn" />        // 18 px, in the state's own colour
```

- A part sets an icon's size in its own CSS with `--icon-size`, and may
  set `--icon-stroke`, the line's width when the icon is 20 px. The line
  thins and thickens with the icon. Nothing about an icon is set from
  script.
- `icon-name.ts` lists every name. The drawn set is one file each; the
  rest are Lucide's, named for what they mean here. A new Lucide icon is
  one line in `icon-name.ts` and one in `Icon.tsx`.
- `FIGURE_FOR_VIEW[kind]` is the figure for a film kind.
- Every shape states its own fill and stroke. The two inherit, so a
  shape that left one unsaid would take it from the part around it, as
  when `flash` runs there.
- Every icon, glyph, state icon and figure is decoration and is hidden
  from assistive technology. The control or row it sits in says the
  same thing in words.
- Every sign in a family is its own shape. `told-apart.test.tsx` fails
  if two states ever draw the same shapes.

## Focus

Every focusable thing gets a 2 px lumen ring with a 2 px dark gap from
`base.css`, with no rule of its own. The ring is an outline and a shadow
in the same place.

A part may replace the ring with a focus of its own that shows as
plainly, and then turns both off itself (`outline: none; box-shadow:
none`): a corner pin on a film, where a ring would be a rectangle round
a dot, takes a brighter, wider line; a rung of the ladder shows as the
selected rung. No part turns the ring off and draws nothing in its
place.

## Gallery pages

Each part has a page (`*.gallery.tsx`) in the dev-only gallery at
`?gallery`. Their surroundings are in `look-and-feel.gallery.css`, which
only gallery pages import.
