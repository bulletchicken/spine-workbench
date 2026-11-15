# look-and-feel

What everything in the app is drawn with: the design system's values and
parts, what the app adds beside them, the type, and every sign the app
draws. How things move is next door, in `choreography/`.

The design system is called Clear. `DESIGN.md` at the root of the
repository says what each value and each part is for, how it behaves and
why. Its two stylesheets live here, **`tokens.css`** (every value, by name)
and **`components.css`** (every part, as plain classes). Both are the
design system's own files, kept here exactly as it writes them: they are
never edited in the app. A change to one follows "Keeping this current" in
`DESIGN.md`. A test holds `components.css` to the system's own search: no
colour, length or time written as a number. The files also hold the app’s
`data-motion="reduced"` answers, the compact note and the narrow-window
values; the screens set none of those again.

| File | Holds |
|---|---|
| `tokens.css` | the design system's values, by name, one value each. Never edited here |
| `components.css` | the design system's parts, as plain classes: the page and its reset, the focus ring, type, glass, buttons, the step control, the bar, the badge and note, the frame and stage, places, tiles, marks, names, the dock, the meter, the notice, and the parts of the 3D moment. Never edited here |
| `base.css` | the app's one stylesheet: it imports the files here and the four shared keyframes, in order, then adds the reset lines the system lacks and `.spoken-only` |
| `earlier-tokens.css` | the earlier look's tokens, kept for the parts that are no longer on the page. Nothing new reads them |
| `named-timings.css` | the earlier parts' durations and gaps that are not tokens |
| `fonts.css` | Geist and Geist Mono, from the installed font packages |
| `confidence.css` | the earlier parts' one place `data-conf` becomes colour and line |
| `page-state.ts` | `usePageState`: the one way the app writes the five attributes `components.css` reads on `<html>` (`data-moment`, `data-films`, `data-ready`, `data-drag`, `data-leaving`) |
| `data-attributes.ts` | the value sets of the data attributes that carry state to CSS |
| `ConfidenceGlyph.tsx` | the five confidence glyphs, for the earlier parts |
| `StateIcon.tsx` | the sign for the state of a row or a chip |
| `icons/` | `Icon`, the names it takes, and the icons drawn for this app |
| `figures/` | the four view figures, and `figure-for-view.ts`, which finds one by film kind |
| `BrandMark.tsx` | the app's mark: three bars, the middle one in the melon blend. It carries its own gradient, with an id of its own for each mark drawn, and its two stops read `--melon-1` and `--melon-2` through CSS |

## How a part is drawn

A part's markup wears the system's class (`.bar`, `.primary`, `.quiet`,
`.tile`, `.mark`) and takes its rules from `components.css`. A part keeps
a stylesheet of its own only for what `components.css` does not cover,
and every rule in it uses the system's names and writes no colour, size
or timing of its own (`names-only.test.ts` holds this, and lets by the
stylesheets of the earlier look while they are still in the tree). Every
such rule is something the system has not yet taken in.

Script reads times, curves and plain numbers through
`choreography/timing.ts`, from the same custom properties CSS uses.

## The earlier tokens

The parts the app no longer mounts still compile, keep their tests and
their gallery pages, and read the earlier look's names (`--room-*`,
`--ink-1`, `--lumen-*`, the confidence colours, `--t-*` and the rest).
Those live in `earlier-tokens.css`, with the values they had, so they
never mix with the design system's. When a part is restyled its tokens
come out of that file. Nothing new reads anything in it.

Two of the earlier names are the design system's on the page: `--ink-2`
and `--ink-3`. The earlier look's film ground (`data-ground="film"`, which
the film viewer still says) hands its own light inks over under those
two names, so an earlier part drawn on a film still reads. For the same
reason nothing the design system draws on a film reads `--ink-2` or
`--ink-3`.

## Type

| Family | Token | For |
|---|---|---|
| Geist | `--font-sans` | everything read or pressed; the body's own font |
| Geist Mono | `--font-mono` | numbers and vertebra names only |

Each role has a ready-made `font` value: `--type-display`,
`--type-heading`, `--type-body`, `--type-body-medium`, `--type-small`,
`--type-small-medium`, `--type-row`, `--type-caption`. A number inside a
sentence takes the sentence's size and adds the class `num`, which gives
it the mono face with every figure one width. Weights are 400, 500 and
600, and nothing heavier. No serif, and no capital-letter labels.

Every face is `font-display: block`: text is first seen in its own face,
never in a fallback that is then swapped away.

`.num` and the type roles (`.t-display` and the rest) are the design
system's. `spoken-only`, in `base.css`, is the app's own helper class:
words that are spoken and not shown, such as the engine's version in the
notice.

### Keys

The shipped faces have `↑` and `↓`. They have no `←` or `→`, and none of
the signs printed on a keyboard's held keys (`⌘`, `⇧`, `⌥`) or on Return.
A character a face lacks is drawn by whatever the system has, in another
size and weight, and differently on each system. So a key hint never
writes one of those as a character:

- an arrow key is an icon, all four of them, so they are one family:
  `arrow-left`, `arrow-right`, `arrow-up`, `arrow-down`;
- the Command, Shift and Option keys are the icons `command`, `shift`
  and `option`;
- every other key is its letter or its word in the mono face: `Z`,
  `Enter`, `Esc`, `Ctrl`, `Shift`, `Alt`.

## Icons and signs

```tsx
<Icon name="boundary-wand" />     // 20 px, a 1.5 line, the colour of the text around it
<StateIcon state="warn" />        // 18 px, in the state's own colour
```

- Icons are lines with round caps and joins and no fill, drawn on a
  20 grid. `--icon-stroke` is 1.5, in the icon's own units, so the line
  thins and thickens with the icon. A part sets an icon's size in its own
  CSS with `--icon-size` (`--icon-small` is 18, `--icon-medium` 20).
  Nothing about an icon is set from script.
- `icon-name.ts` lists every name. The drawn set is one file each; the
  rest are Lucide's, named for what they mean here.
- Every shape states its own fill and stroke. The two inherit, so a
  shape that left one unsaid would take it from the part around it.
- Every icon, glyph, state icon and figure is decoration and is hidden
  from assistive technology. The control or row it sits in says the
  same thing in words.
- Every sign in a family is its own shape. `told-apart.test.tsx` fails
  if two states ever draw the same shapes.

## Confidence, in the earlier parts

The earlier parts put `data-conf` on a vertebra, a rung or a chip, and
read the properties `confidence.css` sets, so no part writes a
`[data-conf]` rule of its own. Clear does not use it: an outline is
found, needs a look, or is pointed at, and `components.css` says how
each is drawn.

## Focus

Every focusable thing gets one ring from `components.css`, with no rule
of its own: `--ring-width` of `--accent`, `--ring-gap` outside the
control, following its own corners. Keyboard focus only.

A part may replace the ring with a focus of its own that shows as
plainly, and then turns the outline off itself. No part turns the ring
off and draws nothing in its place.

## Gallery pages

Each part has a page (`*.gallery.tsx`) in the development-only gallery
at `?gallery`. Their surroundings are in `look-and-feel.gallery.css`,
which only gallery pages import.
