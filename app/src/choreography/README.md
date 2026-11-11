# choreography

How things move. Light arrives, cameras move, nothing bounces for fun:
things appear by brightening, and exactly one thing loops, the lit next
action. Every duration and curve is a custom property, from
`look-and-feel/tokens.css` or `look-and-feel/named-timings.css`; no
component's CSS holds a literal time.

| File | Holds |
|---|---|
| `breathe.css`, `settle.css`, `roll.css`, `flash.css` | the four keyframes more than one area uses; `base.css` imports them |
| `one-loop.ts` | the single animation loop every script-driven motion joins |
| `timing.ts` | reads a duration or a curve where a part stands, for script-driven motion |
| `curve.ts` | a CSS cubic-bezier as a function |
| `stagger.ts` | each level's place in a cascade, and the style that carries it to CSS |
| `reduced-motion.ts` | the system's setting and the app's, joined into `full` or `reduced` |
| `use-system-reduced-motion.ts` | the system's setting, followed as it changes |

A keyframe that one component alone uses lives in that component's CSS.

## The four shared keyframes

Keyframe names are global, so a component uses these by name.

```css
/* settle: a line or a card arrives. --i is its place among its siblings. */
.guide-line {
  animation: settle var(--t-calm) var(--ease-out) both;
  animation-delay: calc(var(--i, 0) * var(--stagger-line));
}

/* roll: a changed name. --i is its distance from the edit. */
.level-name {
  animation: roll var(--t-base) var(--ease-out) both;
  animation-delay: calc(var(--i, 0) * var(--stagger));
}

/* flash: a new outline or a new rung, lit and then fading to its own colours. */
.outline.is-new,
.rung-frame.is-new {
  animation: flash var(--t-slow) var(--ease-out);
}
```

`settle`, `roll` and `flash` say only where a thing starts, so each ends
on the part's own colours and place. `roll` rises 8 px unless the part
sets `--roll`; a name drawn on a film sets it in film pixels.

`flash` goes on the one thing that is new and draws itself: the
outline's own path, which reads the stroke and fill, or the rung's own
frame, which reads the background. It never goes on a group, or on a box
with drawings inside it. Fill, stroke and stroke-width inherit, so
anything inside that leaves one unsaid would be drawn white and 6 px
wide while the flash runs. Every sign in `look-and-feel/` states its own
fill and stroke, so a glyph beside a flashing frame keeps its shape; a
shape a part draws for itself should do the same.

`breathe` is different. It moves one number, `--breath`, from 0 to 1 and
back, and each lit part turns that number into its own light. A part
takes the whole animation, never the keyframes' name:

```css
.lit-button {
  box-shadow: 0 0 calc(18px + 16px * var(--breath)) rgb(var(--breath-light) / calc(0.3 + 0.25 * var(--breath)));
  animation: var(--breathing);
}
```

- `--breath-light` is the colour of the breathing light, set once in
  `breathe.css`: the blue of `--lumen-glow` as bare channels, so a part
  gives it a strength that follows the breath. No part writes that blue
  as a literal.
- `--breath` does not inherit. A box that takes the animation breathes
  alone; what is inside it stays at rest. So each thing that glows takes
  the animation itself: each card of a pair of choices, not the pair.
- `--breathing` is `none` under reduced motion, and a part that is not
  breathing rests at `--breath: 0.5`. So no part needs a rule of its own
  to stop the loop.

## A cascade

A delay is never set from script. A part writes each item's place as
`--i` and its CSS multiplies by a gap that is a token:

```tsx
placesFromEdit(levels.length, editedAt).map((place, level) => <text style={placeStyle(place)}>…</text>);
```

## Motion driven by script

Join the one loop; do not call `requestAnimationFrame`.

```ts
const duration = durationOf(element, '--t-refit'); // 280, or 1 under reduced motion
const curve = curveOf(element, '--ease-out');
let elapsed = 0;
const leave = joinLoop((dt) => {
  elapsed += dt;
  place(curve(Math.min(1, elapsed / duration)));
  if (elapsed >= duration) leave();
});
```

- `dt` is the milliseconds since the frame before. It is 0 on a joiner's
  first frame, and never more than 100, so a tab that was hidden does
  not make a motion leap.
- The loop starts with its first joiner and stops with its last. Leave
  when the motion is done.
- Read a duration or a curve once, when a motion starts, from an element
  inside the app root: that is where `data-motion="reduced"` shortens
  them.
- A camera or a bay does not glide under reduced motion, not even for
  the 160 ms `--t-slow` shrinks to: it lands at once. Whatever moves one
  checks the motion mode first, and under `reduced` sets the place
  without joining the loop.

## Reduced motion

`data-motion` on the app root is `motionMode(system, app)`: reduced when
the system asks for it or the app does. Under it:

| | Becomes |
|---|---|
| the tokens | `--t-quick` and `--t-base` 1 ms, `--t-calm` 120 ms, `--t-slow` and `--t-cinema` 160 ms, `--stagger` 0 |
| every named timing | 1 ms, or 0 for a gap; how long a toast stays is not motion and keeps its 4.6 s |
| `breathe` | does not run; lit parts hold half a breath |
| `settle` | brightens without rising |
| bays and cameras | land at once, with no glide; the shrunken tokens are for what brightens or fades |

The system's own setting has the same effect through a media query, with
no attribute at all. A component that loops something of its own (a busy
ring, a sheen) stops it under both.
