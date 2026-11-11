# look-and-feel

What everything in the app is drawn with: the tokens, the base styles,
the type, and every sign the app draws. How things move is next door, in
`choreography/`. The design these follow is in `docs/design/`.

| File | Holds |
|---|---|
| `base.css` | the app's one stylesheet: it imports the files below and the four shared keyframes, in order, then sets the reset, the body and the focus ring |
| `tokens.css` | the design's tokens, unchanged; never edited here. Each token that depends on its ground has a value for paper and one for a film |
| `named-timings.css` | the durations and gaps the design names that are not tokens |
| `fonts.css` | Instrument Serif, Geist and Geist Mono, from the installed font packages |
| `confidence.css` | the one place `data-conf` becomes colour and line |
| `data-attributes.ts` | the value sets of the data attributes that carry state to CSS |
| `ConfidenceGlyph.tsx` | the five confidence glyphs |
| `StateIcon.tsx` | the sign for the state of a row or a chip |
| `icons/` | `Icon`, the names it takes, and the eight icons drawn for this app |
| `figures/` | the four view figures, and `figure-for-view.ts`, which finds one by film kind |
| `BrandMark.tsx` | the app's mark |

## Paper and film

The page is white and ruled: regions are divided by one-pixel hairlines
(`--rule`) and by space, never by a box with a fill and a shadow. A film
is still read on black, so its well is `--well` and is never themed.

That makes two grounds, and they need opposite inks. A token that
depends on its ground has a value for each (`--ink-1-paper`,
`--ink-1-film`), and component CSS uses the plain name (`--ink-1`),
which is the one for the ground the part stands on:

- paper, unless an ancestor says `data-ground="film"`;
- film inside `data-ground="film"`: the film viewer says it, so every
  layer and HUD in it is drawn in on-film values with no rule of its own;
- paper again inside `data-ground="paper"`, for a paper label laid on a
  film.

The inks, the lines, lumen, the focus ring and the confidence colours
follow the ground. `tokens.test.ts` holds the two grounds to the same
names, and `contrast.test.ts` holds every ink and every confidence
colour to its contrast on its own ground.

What floats (a popover, a sheet, a toast, a tooltip, a label on a film)
is paper with a `--keyline` round it. Nothing has a shadow, nothing is
glass and nothing glows: `--lift-*`, `--glass-blur` and `--glow-lumen`
are kept by name and say `none`.

## Colour and type

Ink means go, colour means data. Lumen (`--lumen-*`) is only for the
next action: on paper it is ink, and the lit button is the one solid ink
thing on a page; on a film it is white. It has no hue. The only hues are
the confidence colours, and each comes with a line pattern, a glyph and
a word.

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
its own.

Each state's colour has an on-paper value and an on-film value, and
`--c` is worked out where `data-conf` stands. So put `data-conf` inside
the ground the mark is drawn on: on the vertebra's group inside the
film, on the rung on paper. One `data-conf` on a box that holds both a
film and a paper part would colour both for paper.

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

/* A rung is a ruled row on paper. Its rule is --rule whatever the state. */
.rung-bar {
  width: var(--conf-bar-w);
  background: var(--conf-bar);
  opacity: var(--conf-rung-fade);
}
.rung-name {
  opacity: var(--conf-rung-fade);
}

.legend-chip-line {
  border-top: var(--conf-edge-w) var(--conf-edge) var(--c);
}
```

What a part adds for itself is what does not depend on the state: the
5 px casing under an outline, a rung's size and its rule, how a selected
rung shows.

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

Every focusable thing gets a 2 px ring with a 2 px gap from `base.css`,
with no rule of its own: ink with a paper gap on paper, white with a
black gap on a film. The ring is an outline and a shadow in the same
place.

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
